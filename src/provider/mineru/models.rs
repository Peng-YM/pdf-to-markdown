use serde::{Deserialize, Serialize};

// ============================================
// Precision API — file-urls/batch (upload local file)
// ============================================

#[derive(Debug, Serialize)]
pub struct FileUrlsBatchRequest {
    pub files: Vec<FileItem>,
    pub model_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_ocr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_formula: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_cache: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct FileItem {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FileUrlsBatchResponse {
    pub code: i32,
    pub msg: Option<String>,
    pub data: Option<FileUrlsBatchData>,
}

#[derive(Debug, Deserialize)]
pub struct FileUrlsBatchData {
    pub batch_id: String,
    pub file_urls: Vec<String>,
}

// ============================================
// Precision API — extract-results/batch (poll)
// ============================================

#[derive(Debug, Deserialize)]
pub struct BatchResultsResponse {
    pub code: i32,
    pub msg: Option<String>,
    pub data: Option<BatchResultsData>,
}

#[derive(Debug, Deserialize)]
pub struct BatchResultsData {
    #[allow(dead_code)]
    pub batch_id: String,
    pub extract_result: Vec<ExtractResultItem>,
}

#[derive(Debug, Deserialize)]
pub struct ExtractResultItem {
    #[allow(dead_code)]
    pub file_name: String,
    pub state: String,
    pub full_zip_url: Option<String>,
    pub extract_progress: Option<ExtractProgress>,
    pub err_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExtractProgress {
    pub total_pages: Option<i32>,
    #[allow(dead_code)]
    pub extracted_pages: Option<i32>,
}

// ============================================
// Agent API — file upload
// ============================================

#[derive(Debug, Serialize)]
pub struct AgentFileRequest {
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_ocr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_formula: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AgentFileResponse {
    pub code: i32,
    pub data: Option<AgentFileData>,
}

#[derive(Debug, Deserialize)]
pub struct AgentFileData {
    pub task_id: String,
    pub file_url: String,
}

// ============================================
// Agent API — poll status
// ============================================

#[derive(Debug, Deserialize)]
pub struct AgentStatusResponse {
    pub code: i32,
    pub data: Option<AgentStatusData>,
}

#[derive(Debug, Deserialize)]
pub struct AgentStatusData {
    pub state: String,
    pub markdown_url: Option<String>,
    pub extract_progress: Option<ExtractProgress>,
    pub err_code: Option<i32>,
    pub err_msg: Option<String>,
}
