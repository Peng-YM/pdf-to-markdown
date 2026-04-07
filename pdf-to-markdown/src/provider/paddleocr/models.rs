use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================
// Job Submission Response
// ============================================

#[derive(Debug, Deserialize)]
pub struct JobSubmissionResponse {
    pub data: JobSubmissionData,
}

#[derive(Debug, Deserialize)]
pub struct JobSubmissionData {
    #[serde(rename = "jobId")]
    pub job_id: String,
}

// ============================================
// Job Status Polling Response
// ============================================

#[derive(Debug, Deserialize)]
pub struct JobStatusResponse {
    pub data: JobStatusData,
}

#[derive(Debug, Deserialize)]
pub struct JobStatusData {
    pub state: String, // "pending", "running", "done", "failed"
    #[serde(rename = "errorMsg")]
    pub error_msg: Option<String>,
    #[serde(rename = "extractProgress")]
    pub extract_progress: Option<ExtractProgress>,
    #[serde(rename = "resultUrl")]
    pub result_url: Option<ResultUrl>,
}

#[derive(Debug, Deserialize)]
pub struct ExtractProgress {
    #[serde(rename = "totalPages")]
    pub total_pages: Option<i32>,
    #[serde(rename = "extractedPages")]
    pub extracted_pages: Option<i32>,
    #[serde(rename = "startTime")]
    #[allow(dead_code)]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    #[allow(dead_code)]
    pub end_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResultUrl {
    #[serde(rename = "jsonUrl")]
    pub json_url: String,
}

// ============================================
// JSONL Result (per line)
// ============================================

#[derive(Debug, Deserialize)]
pub struct JsonlLine {
    pub result: JsonlResult,
}

#[derive(Debug, Deserialize)]
pub struct JsonlResult {
    #[serde(rename = "layoutParsingResults")]
    pub layout_parsing_results: Vec<LayoutParsingResult>,
}

#[derive(Debug, Deserialize)]
pub struct LayoutParsingResult {
    pub markdown: MarkdownData,
    #[serde(rename = "outputImages")]
    pub output_images: Option<HashMap<String, String>>, // img_name -> url
}

#[derive(Debug, Deserialize)]
pub struct MarkdownData {
    pub text: String,
    pub images: HashMap<String, String>, // img_path -> url
}

// ============================================
// Optional Payload for Job Submission
// ============================================

#[derive(Debug, Serialize)]
pub struct OptionalPayload {
    #[serde(rename = "useDocOrientationClassify")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_doc_orientation_classify: Option<bool>,
    #[serde(rename = "useDocUnwarping")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_doc_unwarping: Option<bool>,
    #[serde(rename = "useLayoutDetection")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_layout_detection: Option<bool>,
    #[serde(rename = "useChartRecognition")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_chart_recognition: Option<bool>,
}

impl Default for OptionalPayload {
    fn default() -> Self {
        Self {
            use_doc_orientation_classify: Some(false),
            use_doc_unwarping: Some(false),
            use_layout_detection: Some(false),
            use_chart_recognition: Some(false),
        }
    }
}
