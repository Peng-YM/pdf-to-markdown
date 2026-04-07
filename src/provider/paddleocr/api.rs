use super::models::*;
use crate::error::{anyhow, Result};
use reqwest::multipart;
use reqwest::Client;
use std::path::Path;

const JOB_URL: &str = "https://paddleocr.aistudio-app.com/api/v2/ocr/jobs";
const MODEL: &str = "PaddleOCR-VL-1.5";

pub async fn submit_job(
    client: &Client,
    api_key: &str,
    file_path: &Path,
    optional_payload: &OptionalPayload,
) -> Result<String> {
    let headers = {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("bearer {}", api_key))?,
        );
        h
    };

    let _data = [
        ("model", MODEL.to_string()),
        ("optionalPayload", serde_json::to_string(optional_payload)?),
    ];

    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("document.pdf");

    let file_bytes = tokio::fs::read(file_path).await?;
    let part = multipart::Part::bytes(file_bytes)
        .file_name(file_name.to_string())
        .mime_str("application/pdf")?;

    let form = multipart::Form::new()
        .text("model", MODEL.to_string())
        .text("optionalPayload", serde_json::to_string(optional_payload)?)
        .part("file", part);

    let response = client.post(JOB_URL).headers(headers).multipart(form).send().await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await?;
        return Err(anyhow!("Job submission failed: status {}, response: {}", status, text));
    }

    let result: JobSubmissionResponse = response.json().await?;
    Ok(result.data.job_id)
}

pub async fn get_job_status(client: &Client, api_key: &str, job_id: &str) -> Result<JobStatusData> {
    let headers = {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("bearer {}", api_key))?,
        );
        h
    };

    let url = format!("{}/{}", JOB_URL, job_id);
    let response = client.get(&url).headers(headers).send().await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await?;
        return Err(anyhow!("Job status failed: status {}, response: {}", status, text));
    }

    let result: JobStatusResponse = response.json().await?;
    Ok(result.data)
}

pub async fn download_jsonl(client: &Client, json_url: &str) -> Result<String> {
    let response = client.get(json_url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await?;
        return Err(anyhow!("JSONL download failed: status {}, response: {}", status, text));
    }
    Ok(response.text().await?)
}

pub async fn download_image(client: &Client, image_url: &str) -> Result<Vec<u8>> {
    let response = client.get(image_url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await?;
        return Err(anyhow!("Image download failed: status {}, response: {}", status, text));
    }
    Ok(response.bytes().await?.to_vec())
}
