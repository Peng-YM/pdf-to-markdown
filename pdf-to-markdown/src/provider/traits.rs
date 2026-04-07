use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;

use crate::error::Result;
use tempfile::TempDir;

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub message: String,
    pub current: u64,
    pub total: Option<u64>,
}

impl ProgressUpdate {
    pub fn new(message: String) -> Self {
        Self {
            message,
            current: 0,
            total: None,
        }
    }
}

#[derive(Debug)]
pub struct ParseResult {
    pub markdown: String,
    pub images: HashMap<String, PathBuf>, // image_name -> temp_path
    pub temp_dir: Option<TempDir>, // holds the temp dir for images
}

pub trait ProviderConfig: Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone)]
pub struct ZhipuConfig {
    pub tool_type: String,
    pub max_retries: u32,
    pub poll_interval_secs: u64,
    pub page_ranges: Option<Vec<(u32, u32)>>,
}

impl Default for ZhipuConfig {
    fn default() -> Self {
        Self {
            tool_type: "prime".to_string(),
            max_retries: 60,
            poll_interval_secs: 3,
            page_ranges: None,
        }
    }
}

impl ProviderConfig for ZhipuConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait::async_trait]
pub trait DocumentProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn parse_document(
        &self,
        file_path: &Path,
        config: &dyn ProviderConfig,
        progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ParseResult>;
}
