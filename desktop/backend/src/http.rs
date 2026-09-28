use crate::Result;
use serde_json::Value;
use std::{io::Read, time::Duration};
pub fn bytes(
    url: &str,
    limit: usize,
    timeout_ms: u64,
    headers: &[(&str, &str)],
) -> Result<Vec<u8>> {
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" {
        return Err("HTTPS 주소만 다운로드할 수 있습니다".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .user_agent("mabinogi-rem-booster")
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client.get(parsed);
    for (k, v) in headers {
        request = request.header(*k, *v)
    }
    let response = request
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err("다운로드 허용 크기를 초과했습니다".into());
    }
    let mut bytes = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("다운로드 허용 크기를 초과했습니다".into());
    }
    Ok(bytes)
}
pub fn json(url: &str) -> Result<Value> {
    serde_json::from_slice(&bytes(
        url,
        8 * 1024 * 1024,
        15000,
        &[
            ("Accept", "application/vnd.github+json"),
            ("X-GitHub-Api-Version", "2022-11-28"),
        ],
    )?)
    .map_err(|e| e.to_string())
}
pub fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
