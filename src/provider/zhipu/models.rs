use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTaskResponse {
    pub message: String,
    #[serde(rename = "task_id")]
    pub task_id: String,
    #[serde(default)]
    #[serde(rename = "success")]
    pub success: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryResultResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(rename = "task_id", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(rename = "parsing_result_url", skip_serializing_if = "Option::is_none")]
    pub parsing_result_url: Option<String>,
}
