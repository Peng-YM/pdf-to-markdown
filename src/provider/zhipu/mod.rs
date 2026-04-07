mod api;
mod models;

use api::*;

use crate::error::{anyhow, Result};
use crate::provider::traits::*;
use crate::utils;

use std::path::Path;
use tempfile::tempdir;
use tokio::time::sleep;

pub struct ZhipuProvider {
    api_key: String,
    client: reqwest::Client,
}

impl ZhipuProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { api_key, client }
    }
}

#[async_trait::async_trait]
impl DocumentProvider for ZhipuProvider {
    fn name(&self) -> &'static str {
        "Zhipu AI"
    }

    async fn parse_document(
        &self,
        file_path: &Path,
        config: &dyn ProviderConfig,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ParseResult> {
        let config = config
            .as_any()
            .downcast_ref::<ZhipuConfig>()
            .ok_or_else(|| anyhow!("Invalid config type for ZhipuProvider"))?;

        // 处理页面范围
        let temp_dir = if let Some(ref ranges) = config.page_ranges {
            let tmp_dir = tempdir()?;
            let split_pdf_path = tmp_dir.path().join("split.pdf");
            progress_cb(ProgressUpdate {
                message: format!("Splitting PDF ({} ranges)...", ranges.len()),
                current: 0,
                total: Some(config.max_retries as u64),
            });
            utils::split_pdf(file_path, &split_pdf_path, ranges)?;
            Some((tmp_dir, split_pdf_path))
        } else {
            None
        };

        let file_to_parse =
            if let Some((_, ref path)) = temp_dir { path.as_path() } else { file_path };

        progress_cb(ProgressUpdate {
            message: "Creating task...".to_string(),
            current: 0,
            total: Some(config.max_retries as u64),
        });

        let task_id =
            create_task(&self.client, &self.api_key, file_to_parse, &config.tool_type).await?;

        progress_cb(ProgressUpdate {
            message: format!("Task created: {}", task_id),
            current: 1,
            total: Some(config.max_retries as u64),
        });

        let mut markdown_content = String::new();
        let mut temp_dir_for_images = None;
        let mut images = std::collections::HashMap::new();

        for attempt in 1..=config.max_retries {
            progress_cb(ProgressUpdate {
                message: format!("Polling... (attempt {}/{})", attempt, config.max_retries),
                current: attempt as u64,
                total: Some(config.max_retries as u64),
            });

            let query_result = query_result(&self.client, &self.api_key, &task_id).await?;

            match query_result.status.as_str() {
                "succeeded" => {
                    progress_cb(ProgressUpdate {
                        message: "Parsing completed!".to_string(),
                        current: attempt as u64,
                        total: Some(config.max_retries as u64),
                    });

                    if let Some(content) = query_result.content {
                        markdown_content = content;
                    }

                    if let Some(download_url) = query_result.parsing_result_url {
                        progress_cb(ProgressUpdate {
                            message: "Downloading result package...".to_string(),
                            current: attempt as u64,
                            total: Some(config.max_retries as u64),
                        });

                        let temp_dir = tempdir()?;
                        let zip_path = temp_dir.path().join("result.zip");
                        download_file(&self.client, &download_url, &zip_path).await?;

                        let extract_dir = temp_dir.path().join("extracted");
                        std::fs::create_dir_all(&extract_dir)?;
                        extract_zip(&zip_path, &extract_dir)?;

                        let md_files: Vec<_> = walkdir::WalkDir::new(&extract_dir)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .map(|ext| ext.to_str() == Some("md"))
                                    .unwrap_or(false)
                            })
                            .collect();

                        if let Some(md_file) = md_files.first() {
                            if markdown_content.is_empty() {
                                markdown_content = std::fs::read_to_string(md_file.path())?;
                            }
                        }

                        // 收集所有图片
                        let img_files: Vec<_> = walkdir::WalkDir::new(&extract_dir)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .map(|ext| {
                                        ext.to_str() == Some("png")
                                            || ext.to_str() == Some("jpg")
                                            || ext.to_str() == Some("jpeg")
                                    })
                                    .unwrap_or(false)
                            })
                            .collect();

                        for img_file in img_files {
                            let filename =
                                img_file.file_name().to_str().unwrap_or("image.jpg").to_string();
                            images.insert(filename, img_file.path().to_path_buf());
                        }

                        temp_dir_for_images = Some(temp_dir);
                    }

                    break;
                }
                "processing" => {
                    sleep(std::time::Duration::from_secs(config.poll_interval_secs)).await;
                }
                _ => {
                    return Err(anyhow!("Parsing failed: {}", query_result.message));
                }
            }
        }

        if markdown_content.is_empty() {
            return Err(anyhow!("No content generated"));
        }

        Ok(ParseResult { markdown: markdown_content, images, temp_dir: temp_dir_for_images })
    }
}
