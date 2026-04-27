use super::models::*;
use crate::error::{anyhow, Result};
use reqwest::Client;
use std::path::Path;

const PRECISION_BASE: &str = "https://mineru.net/api/v4";
const AGENT_BASE: &str = "https://mineru.net/api/v1";

// ============================================
// Precision API — local file upload + parse
// ============================================

/// Step 1: Request a signed upload URL via file-urls/batch.
/// Returns (batch_id, upload_url).
pub async fn precision_request_upload(
    client: &Client,
    api_key: &str,
    file_name: &str,
    model_version: &str,
    is_ocr: Option<bool>,
    enable_formula: Option<bool>,
    enable_table: Option<bool>,
    language: Option<String>,
    no_cache: Option<bool>,
) -> Result<(String, String)> {
    let request = FileUrlsBatchRequest {
        files: vec![FileItem { name: file_name.to_string() }],
        model_version: model_version.to_string(),
        is_ocr,
        enable_formula,
        enable_table,
        language,
        no_cache,
    };

    let response = client
        .post(format!("{}/file-urls/batch", PRECISION_BASE))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    let body: FileUrlsBatchResponse = response.json().await.map_err(|e| {
        anyhow!("Failed to parse file-urls/batch response (status {}): {}", status, e)
    })?;

    if body.code != 0 {
        return Err(anyhow!(
            "MinerU file-urls/batch failed: code={}, msg={:?}",
            body.code,
            body.msg
        ));
    }

    let data = body.data.ok_or_else(|| anyhow!("MinerU file-urls/batch returned no data"))?;
    let upload_url = data
        .file_urls
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("MinerU file-urls/batch returned empty file_urls"))?;

    Ok((data.batch_id, upload_url))
}

/// Step 2: Upload file bytes to the signed URL (PUT, no Content-Type).
pub async fn precision_upload_file(client: &Client, upload_url: &str, file_path: &Path) -> Result<()> {
    let file_bytes = tokio::fs::read(file_path).await?;

    let response = client
        .put(upload_url)
        .body(file_bytes)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("MinerU file upload failed: status {}, body: {}", status, text));
    }

    Ok(())
}

/// Step 3: Poll batch results until done.
pub async fn precision_poll_batch(
    client: &Client,
    api_key: &str,
    batch_id: &str,
) -> Result<BatchResultsData> {
    let url = format!("{}/extract-results/batch/{}", PRECISION_BASE, batch_id);

    loop {
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        let status = response.status();
        let body: BatchResultsResponse = response.json().await.map_err(|e| {
            anyhow!("Failed to parse batch results response (status {}): {}", status, e)
        })?;

        if body.code != 0 {
            return Err(anyhow!(
                "MinerU batch results query failed: code={}, msg={:?}",
                body.code,
                body.msg
            ));
        }

        let data = body.data.ok_or_else(|| anyhow!("MinerU batch results returned no data"))?;

        let all_done = data.extract_result.iter().all(|item| {
            item.state == "done" || item.state == "failed"
        });

        if all_done {
            return Ok(data);
        }

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

/// Download a ZIP file from a URL.
pub async fn download_zip(client: &Client, zip_url: &str) -> Result<Vec<u8>> {
    let response = client.get(zip_url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("ZIP download failed: status {}, body: {}", status, text));
    }
    Ok(response.bytes().await?.to_vec())
}

// ============================================
// Agent API — file upload + parse
// ============================================

/// Step 1: Initiate agent file parse, get task_id and upload URL.
pub async fn agent_request_upload(
    client: &Client,
    file_name: &str,
    language: Option<String>,
    page_range: Option<String>,
    enable_table: Option<bool>,
    is_ocr: Option<bool>,
    enable_formula: Option<bool>,
) -> Result<(String, String)> {
    let request = AgentFileRequest {
        file_name: file_name.to_string(),
        language,
        page_range,
        enable_table,
        is_ocr,
        enable_formula,
    };

    let response = client
        .post(format!("{}/agent/parse/file", AGENT_BASE))
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    if status == 429 {
        return Err(anyhow!("MinerU Agent API rate limited (HTTP 429). Try again later or use the Precision API."));
    }

    let body: AgentFileResponse = response.json().await.map_err(|e| {
        anyhow!("Failed to parse agent file response (status {}): {}", status, e)
    })?;

    if body.code != 0 {
        return Err(anyhow!("MinerU Agent API failed: code={}", body.code));
    }

    let data = body.data.ok_or_else(|| anyhow!("MinerU Agent API returned no data"))?;
    Ok((data.task_id, data.file_url))
}

/// Step 2: Upload file bytes to the agent signed URL.
pub async fn agent_upload_file(client: &Client, upload_url: &str, file_path: &Path) -> Result<()> {
    let file_bytes = tokio::fs::read(file_path).await?;

    let response = client
        .put(upload_url)
        .body(file_bytes)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Agent file upload failed: status {}, body: {}", status, text));
    }

    Ok(())
}

/// Step 3: Poll agent task status until done.
pub async fn agent_poll_status(
    client: &Client,
    task_id: &str,
) -> Result<AgentStatusData> {
    let url = format!("{}/agent/parse/{}", AGENT_BASE, task_id);

    loop {
        let response = client.get(&url).send().await?;

        let status = response.status();
        let body: AgentStatusResponse = response.json().await.map_err(|e| {
            anyhow!("Failed to parse agent status response (status {}): {}", status, e)
        })?;

        if body.code != 0 {
            return Err(anyhow!("MinerU Agent status query failed: code={}", body.code));
        }

        let data = body.data.ok_or_else(|| anyhow!("MinerU Agent status returned no data"))?;

        match data.state.as_str() {
            "done" => return Ok(data),
            "failed" => {
                return Err(anyhow!(
                    "Agent parse failed: code={}, msg={:?}",
                    data.err_code.unwrap_or(0),
                    data.err_msg
                ));
            }
            "waiting-file" | "uploading" | "pending" | "running" => {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            other => {
                return Err(anyhow!("Unknown agent task state: {}", other));
            }
        }
    }
}

/// Download markdown content from the agent CDN URL.
pub async fn download_markdown(client: &Client, url: &str) -> Result<String> {
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Markdown download failed: status {}, body: {}", status, text));
    }
    Ok(response.text().await?)
}
