use super::models::*;
use crate::error::{anyhow, Result};
use std::fs;
use std::path::Path;

pub(super) const API_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

pub(super) async fn create_task(
    client: &reqwest::Client,
    api_key: &str,
    file_path: &Path,
    tool_type: &str,
) -> Result<String> {
    let url = format!("{}/files/parser/create", API_BASE_URL);

    let file_content = fs::read(file_path)?;
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file.pdf");

    let part = reqwest::multipart::Part::bytes(file_content)
        .file_name(file_name.to_string())
        .mime_str("application/pdf")?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("tool_type", tool_type.to_string())
        .text("file_type", "PDF".to_string());

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        return Err(anyhow!("API error: {} - {}", status, body));
    }

    let create_response: CreateTaskResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow!("Failed to parse response: {} - {}", e, body))?;

    Ok(create_response.task_id)
}

pub(super) async fn query_result(
    client: &reqwest::Client,
    api_key: &str,
    task_id: &str,
) -> Result<QueryResultResponse> {
    let url = format!("{}/files/parser/result/{}/download_link", API_BASE_URL, task_id);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        return Err(anyhow!("API error: {} - {}", status, body));
    }

    let query_response: QueryResultResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow!("Failed to parse response: {} - {}", e, body))?;

    Ok(query_response)
}

pub(super) async fn download_file(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
) -> Result<()> {
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Download failed: {}", response.status()));
    }

    let bytes = response.bytes().await?;
    fs::write(path, bytes)?;

    Ok(())
}

pub(super) fn extract_zip(zip_path: &Path, extract_dir: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = extract_dir.join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}
