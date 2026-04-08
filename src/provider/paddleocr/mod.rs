mod api;
mod models;

use api::*;
use models::*;

use crate::error::{anyhow, Result};
use crate::provider::traits::*;
use crate::utils;

use std::collections::HashMap;
use std::path::Path;
use tempfile::tempdir;
use tokio::time::sleep;

pub struct PaddleOcrProvider {
    api_key: String,
    client: reqwest::Client,
}

impl PaddleOcrProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self { api_key, client }
    }
}

#[derive(Debug, Clone)]
pub struct PaddleOcrConfig {
    pub page_ranges: Option<Vec<(u32, u32)>>,
    pub use_doc_orientation_classify: Option<bool>,
    pub use_doc_unwarping: Option<bool>,
    pub use_layout_detection: Option<bool>,
    pub use_chart_recognition: Option<bool>,
    pub layout_nms: Option<bool>,
    pub layout_merge_bboxes_mode: Option<String>,
    pub show_formula_number: Option<bool>,
    pub restructure_pages: Option<bool>,
    pub prettify_markdown: Option<bool>,
}

impl Default for PaddleOcrConfig {
    fn default() -> Self {
        Self {
            page_ranges: None,
            use_doc_orientation_classify: Some(false),
            use_doc_unwarping: Some(false),
            use_layout_detection: Some(true),
            use_chart_recognition: Some(false),
            layout_nms: Some(true),
            layout_merge_bboxes_mode: Some("union".to_string()),
            show_formula_number: Some(true),
            restructure_pages: Some(true),
            prettify_markdown: Some(true),
        }
    }
}

impl ProviderConfig for PaddleOcrConfig {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait::async_trait]
impl DocumentProvider for PaddleOcrProvider {
    fn name(&self) -> &'static str {
        "PaddleOCR"
    }

    async fn parse_document(
        &self,
        file_path: &Path,
        config: &dyn ProviderConfig,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ParseResult> {
        let config = config
            .as_any()
            .downcast_ref::<PaddleOcrConfig>()
            .ok_or_else(|| anyhow!("Invalid config type for PaddleOcrProvider"))?;

        // 处理页面范围
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

        // 构建可选参数
        let optional_payload = OptionalPayload {
            use_doc_orientation_classify: config.use_doc_orientation_classify,
            use_doc_unwarping: config.use_doc_unwarping,
            use_layout_detection: config.use_layout_detection,
            use_chart_recognition: config.use_chart_recognition,
            layout_nms: config.layout_nms,
            layout_merge_bboxes_mode: config.layout_merge_bboxes_mode.clone(),
            show_formula_number: config.show_formula_number,
            restructure_pages: config.restructure_pages,
            prettify_markdown: config.prettify_markdown,
        };

        // 提交任务
        progress_cb(ProgressUpdate::new("Submitting OCR job...".to_string()));

        let job_id =
            submit_job(&self.client, &self.api_key, &input_path, &optional_payload).await?;

        progress_cb(ProgressUpdate::new(format!("Job submitted, job ID: {}", job_id)));

        // 轮询任务状态
        let mut json_url: Option<String> = None;

        loop {
            let status = get_job_status(&self.client, &self.api_key, &job_id).await?;

            match status.state.as_str() {
                "pending" => {
                    progress_cb(ProgressUpdate::new("Job pending...".to_string()));
                }
                "running" => {
                    if let Some(ref progress) = status.extract_progress {
                        if let (Some(total), Some(extracted)) =
                            (progress.total_pages, progress.extracted_pages)
                        {
                            progress_cb(ProgressUpdate::new(format!(
                                "Processing: {}/{} pages",
                                extracted, total
                            )));
                        } else {
                            progress_cb(ProgressUpdate::new("Processing...".to_string()));
                        }
                    } else {
                        progress_cb(ProgressUpdate::new("Processing...".to_string()));
                    }
                }
                "done" => {
                    if let Some(ref progress) = status.extract_progress {
                        if let Some(extracted) = progress.extracted_pages {
                            progress_cb(ProgressUpdate::new(format!(
                                "Completed! Extracted {} pages",
                                extracted
                            )));
                        }
                    }

                    if let Some(ref result_url) = status.result_url {
                        json_url = Some(result_url.json_url.clone());
                    }

                    break;
                }
                "failed" => {
                    return Err(anyhow!(
                        "Job failed: {}",
                        status.error_msg.unwrap_or("Unknown error".to_string())
                    ));
                }
                other => {
                    return Err(anyhow!("Unknown job state: {}", other));
                }
            }

            sleep(std::time::Duration::from_secs(5)).await;
        }

        let json_url = json_url.ok_or_else(|| anyhow!("No result URL found"))?;

        // 下载并解析 JSONL
        progress_cb(ProgressUpdate::new("Downloading results...".to_string()));

        let jsonl_content = download_jsonl(&self.client, &json_url).await?;
        let lines: Vec<&str> = jsonl_content.lines().filter(|l| !l.trim().is_empty()).collect();

        let mut markdown_texts = Vec::new();
        let mut images = HashMap::new();

        let output_temp_dir = tempdir()?;

        for (line_idx, line) in lines.iter().enumerate() {
            let line_result: JsonlLine = serde_json::from_str(line)?;

            for (layout_idx, layout) in line_result.result.layout_parsing_results.iter().enumerate()
            {
                // 保存 Markdown 文本，替换图片引用
                let mut md_text = layout.markdown.text.clone();

                // 下载并保存图片
                for (img_path, img_url) in &layout.markdown.images {
                    let img_bytes = download_image(&self.client, img_url).await?;

                    // 把路径里的 / 替换成 _，避免创建子目录
                    let safe_img_name = img_path.replace("/", "_");
                    let img_filename =
                        format!("image_{}_{}_{}", line_idx, layout_idx, safe_img_name);
                    let img_path_on_disk = output_temp_dir.path().join(&img_filename);
                    tokio::fs::write(&img_path_on_disk, img_bytes).await?;

                    // 替换 Markdown 里的引用
                    md_text = md_text.replace(img_path, &img_filename);

                    images.insert(img_filename.clone(), img_path_on_disk);
                }

                if let Some(ref output_imgs) = layout.output_images {
                    for (img_name, img_url) in output_imgs {
                        let img_bytes = download_image(&self.client, img_url).await?;

                        // 把路径里的 / 替换成 _，避免创建子目录
                        let safe_img_name = img_name.replace("/", "_");
                        let img_filename =
                            format!("output_{}_{}_{}", line_idx, layout_idx, safe_img_name);
                        let img_path_on_disk = output_temp_dir.path().join(&img_filename);
                        tokio::fs::write(&img_path_on_disk, img_bytes).await?;

                        images.insert(img_filename.clone(), img_path_on_disk);
                    }
                }

                markdown_texts.push(md_text);
            }
        }

        // 合并所有 Markdown 文本
        let combined_markdown = markdown_texts.join("\n\n---\n\n");

        Ok(ParseResult { markdown: combined_markdown, images, temp_dir: Some(output_temp_dir) })
    }
}
