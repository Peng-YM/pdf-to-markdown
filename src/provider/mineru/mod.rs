mod api;
mod models;

use api::*;

use crate::error::{anyhow, Result};
use crate::provider::traits::*;
use crate::utils;

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use tempfile::tempdir;
use tokio::time::sleep;

/// MinerU model variant.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MinerUModel {
    /// Precision API, visual language model (recommended, default)
    #[default]
    Vlm,
    /// Precision API, traditional pipeline
    Pipeline,
    /// Agent lightweight API, no auth needed
    Agent,
}

impl MinerUModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            MinerUModel::Vlm => "vlm",
            MinerUModel::Pipeline => "pipeline",
            MinerUModel::Agent => "agent",
        }
    }
}


pub struct MinerUProvider {
    /// Only needed for Precision API (Vlm / Pipeline). Agent API uses no auth.
    api_key: String,
    client: reqwest::Client,
}

impl MinerUProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .expect("Failed to create HTTP client");

        Self { api_key, client }
    }
}

#[derive(Debug, Clone)]
pub struct MinerUConfig {
    pub model: MinerUModel,
    pub page_ranges: Option<Vec<(u32, u32)>>,
    pub is_ocr: Option<bool>,
    pub enable_formula: Option<bool>,
    pub enable_table: Option<bool>,
    pub language: Option<String>,
}

impl Default for MinerUConfig {
    fn default() -> Self {
        Self {
            model: MinerUModel::default(),
            page_ranges: None,
            is_ocr: Some(true),
            enable_formula: Some(true),
            enable_table: Some(true),
            language: Some("en".to_string()),
        }
    }
}

impl ProviderConfig for MinerUConfig {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait::async_trait]
impl DocumentProvider for MinerUProvider {
    fn name(&self) -> &'static str {
        "MinerU"
    }

    async fn parse_document(
        &self,
        file_path: &Path,
        config: &dyn ProviderConfig,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ParseResult> {
        let config = config
            .as_any()
            .downcast_ref::<MinerUConfig>()
            .ok_or_else(|| anyhow!("Invalid config type for MinerUProvider"))?;

        // If page ranges specified, split PDF first (consistent with PaddleOCR)
        let temp_dir = if let Some(ref ranges) = config.page_ranges {
            let tmp_dir = tempdir()?;
            let split_pdf_path = tmp_dir.path().join("split.pdf");
            progress_cb(ProgressUpdate::new(format!("Splitting PDF to pages {:?}", ranges)));
            utils::split_pdf(file_path, &split_pdf_path, ranges)?;
            Some(tmp_dir)
        } else {
            None
        };

        let input_path = if let Some(ref tmp_dir) = temp_dir {
            tmp_dir.path().join("split.pdf")
        } else {
            file_path.to_path_buf()
        };

        let file_name = input_path.file_name().and_then(|n| n.to_str()).unwrap_or("document.pdf");

        match config.model {
            MinerUModel::Agent => {
                self.parse_via_agent(&input_path, file_name, config, &mut progress_cb).await
            }
            MinerUModel::Vlm | MinerUModel::Pipeline => {
                let model_version =
                    if config.model == MinerUModel::Vlm { "vlm" } else { "pipeline" };
                self.parse_via_precision(
                    &input_path,
                    file_name,
                    model_version,
                    config,
                    &mut progress_cb,
                )
                .await
            }
        }
    }
}

impl MinerUProvider {
    async fn parse_via_precision(
        &self,
        file_path: &Path,
        file_name: &str,
        model_version: &str,
        config: &MinerUConfig,
        progress_cb: &mut Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ParseResult> {
        // Step 1: Get signed upload URL
        progress_cb(ProgressUpdate::new("Requesting upload URL from MinerU...".to_string()));

        let (batch_id, upload_url) =
            precision_request_upload(&self.client, &self.api_key, file_name, model_version, config)
                .await?;

        progress_cb(ProgressUpdate::new("Uploading file to MinerU...".to_string()));

        // Step 2: Upload file
        precision_upload_file(&self.client, &upload_url, file_path).await?;

        progress_cb(ProgressUpdate::new(
            "File uploaded. Waiting for parsing to complete...".to_string(),
        ));

        // Step 3: Poll until done
        sleep(std::time::Duration::from_secs(2)).await;

        let batch_result = precision_poll_batch(&self.client, &self.api_key, &batch_id).await?;

        // Step 4: Find our result item
        let result_item = batch_result
            .extract_result
            .iter()
            .find(|item| item.state == "done")
            .ok_or_else(|| {
                let failed = batch_result.extract_result.iter().find(|item| item.state == "failed");
                match failed {
                    Some(f) => anyhow!(
                        "MinerU parsing failed: {}",
                        f.err_msg.as_deref().unwrap_or("unknown error")
                    ),
                    None => anyhow!("MinerU parsing did not complete"),
                }
            })?;

        let total_pages = result_item.extract_progress.as_ref().and_then(|p| p.total_pages);
        progress_cb(ProgressUpdate::new(format!(
            "Parsing complete! Total pages: {}",
            total_pages.unwrap_or(0)
        )));

        // Step 5: Download ZIP
        let zip_url = result_item
            .full_zip_url
            .as_ref()
            .ok_or_else(|| anyhow!("No ZIP URL in MinerU result"))?;

        progress_cb(ProgressUpdate::new("Downloading result ZIP...".to_string()));

        let zip_bytes = download_zip(&self.client, zip_url).await?;

        // Step 6: Extract ZIP contents
        progress_cb(ProgressUpdate::new("Extracting results...".to_string()));

        extract_zip_contents(zip_bytes).await
    }

    async fn parse_via_agent(
        &self,
        file_path: &Path,
        file_name: &str,
        config: &MinerUConfig,
        progress_cb: &mut Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ParseResult> {
        // Step 1: Initiate agent file parse
        progress_cb(ProgressUpdate::new("Requesting agent upload URL from MinerU...".to_string()));

        let (task_id, upload_url) =
            agent_request_upload(&self.client, file_name, config).await?;

        progress_cb(ProgressUpdate::new("Uploading file to MinerU Agent...".to_string()));

        // Step 2: Upload file
        agent_upload_file(&self.client, &upload_url, file_path).await?;

        progress_cb(ProgressUpdate::new(
            "File uploaded. Waiting for agent parsing...".to_string(),
        ));

        // Step 3: Poll until done
        sleep(std::time::Duration::from_secs(2)).await;

        let status = agent_poll_status(&self.client, &task_id).await?;

        let total_pages = status.extract_progress.as_ref().and_then(|p| p.total_pages);
        progress_cb(ProgressUpdate::new(format!(
            "Agent parsing complete! Total pages: {}",
            total_pages.unwrap_or(0)
        )));

        // Step 4: Download markdown from CDN
        let markdown_url =
            status.markdown_url.ok_or_else(|| anyhow!("No markdown URL in Agent result"))?;

        progress_cb(ProgressUpdate::new("Downloading markdown...".to_string()));

        let markdown = download_markdown(&self.client, &markdown_url).await?;

        // Agent API returns only markdown text; no image directory in result.
        // Images in the markdown may reference CDN URLs — those are left as-is.
        let images = HashMap::new();
        let output_temp_dir = tempdir()?;

        Ok(ParseResult { markdown, images, temp_dir: Some(output_temp_dir) })
    }
}

/// Extract ZIP bytes into a ParseResult.
///
/// The MinerU ZIP typically contains:
///   - A root directory (named after the batch/task) with `full.md` and `images/`
///   - Or `full.md` and `images/` directly at the ZIP root
///
/// We find the first `.md` file and all image files under an `images/` prefix.
async fn extract_zip_contents(zip_bytes: Vec<u8>) -> Result<ParseResult> {
    let output_dir = tempdir()?;

    // Phase 1: Read all data from the ZIP (sync, no .await)
    let (markdown, image_data) = {
        let cursor = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| anyhow!("Failed to open MinerU result ZIP: {}", e))?;

        // Scan entries
        let mut md_candidates: Vec<(String, usize)> = Vec::new();
        let mut image_names: Vec<String> = Vec::new();

        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| anyhow!("ZIP entry error: {}", e))?;
            let name = entry.name().to_string();

            if entry.is_dir() {
                continue;
            }

            let name_lower = name.to_lowercase();
            if name_lower.contains("__macosx") || name.starts_with('.') {
                continue;
            }

            if name_lower.ends_with(".md") {
                let depth = name.chars().filter(|&c| c == '/').count();
                md_candidates.push((name.clone(), depth));
            }

            let is_image = name_lower.ends_with(".png")
                || name_lower.ends_with(".jpg")
                || name_lower.ends_with(".jpeg")
                || name_lower.ends_with(".gif")
                || name_lower.ends_with(".bmp")
                || name_lower.ends_with(".webp");

            if is_image {
                image_names.push(name);
            }
        }

        // Sort markdown by depth (prefer root-level)
        md_candidates.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        // Read markdown content
        let mut markdown_parts: Vec<String> = Vec::new();
        for (md_name, _depth) in &md_candidates {
            let mut entry = archive
                .by_name(md_name)
                .map_err(|e| anyhow!("Failed to read ZIP entry '{}': {}", md_name, e))?;
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| anyhow!("Failed to read markdown '{}': {}", md_name, e))?;
            markdown_parts.push(buf);
        }

        if markdown_parts.is_empty() {
            return Err(anyhow!("No markdown file found in MinerU result ZIP"));
        }

        let mut markdown = markdown_parts.join("\n\n");

        // Read image bytes (still in sync scope)
        let mut image_data: Vec<(String, String, Vec<u8>)> = Vec::new();
        for img_name in &image_names {
            let mut entry = archive
                .by_name(img_name)
                .map_err(|e| anyhow!("Failed to read image '{}': {}", img_name, e))?;
            let mut img_bytes = Vec::new();
            entry
                .read_to_end(&mut img_bytes)
                .map_err(|e| anyhow!("Failed to extract image '{}': {}", img_name, e))?;

            let file_stem = Path::new(img_name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(img_name)
                .to_string();

            // Update markdown references to use flat filenames
            markdown = markdown.replace(img_name, &file_stem);

            image_data.push((img_name.clone(), file_stem, img_bytes));
        }

        (markdown, image_data)
    }; // archive and all ZipFile borrows dropped here

    // Phase 2: Write image files (async, no zip borrows)
    let mut images = HashMap::new();
    for (_orig_name, file_stem, img_bytes) in image_data {
        let local_path = output_dir.path().join(&file_stem);
        tokio::fs::write(&local_path, img_bytes).await?;
        images.insert(file_stem, local_path);
    }

    Ok(ParseResult { markdown, images, temp_dir: Some(output_dir) })
}
