use bytes::Bytes;
use serde::Deserialize;

use super::zip::{decompress_entry, find_eocd, find_manifest_entry, parse_central_dir};

const TAIL_SIZE: u64 = 65536;

#[derive(Debug, thiserror::Error)]
pub enum JarParserError {
    #[error("http error: {0}")]
    Http(String),
    #[error("eocd not found")]
    EocdNotFound,
    #[error("invalid central directory: {0}")]
    InvalidCentralDir(String),
    #[error("manifest not found")]
    ManifestNotFound,
    #[error("decompression failed: {0}")]
    Decompression(String),
    #[error("json parse: {0}")]
    Json(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct FabricModJson {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

// ---------------------------------------------------------------------------
// Async I/O - HTTP Range Requests
// ---------------------------------------------------------------------------

/// HEAD → (content_length, supports_range)
pub async fn fetch_head(client: &reqwest::Client, url: &str) -> Result<(u64, bool), JarParserError> {
    let resp = client
        .head(url)
        .send()
        .await
        .map_err(|e| JarParserError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(JarParserError::Http(format!("HEAD {}", resp.status())));
    }
    let len = resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let supports_range = resp
        .headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("bytes"))
        .unwrap_or(false);

    Ok((len, supports_range))
}

async fn fetch_range(client: &reqwest::Client, url: &str, range: String) -> Result<Bytes, JarParserError> {
    let resp = client
        .get(url)
        .header(reqwest::header::RANGE, range)
        .send()
        .await
        .map_err(|e| JarParserError::Http(e.to_string()))?;

    if resp.status() != reqwest::StatusCode::PARTIAL_CONTENT && !resp.status().is_success() {
        return Err(JarParserError::Http(format!("Range {}", resp.status())));
    }
    resp.bytes()
        .await
        .map_err(|e| JarParserError::Http(e.to_string()))
}

/// Fetch last TAIL_SIZE bytes (or full file if smaller).
pub async fn fetch_tail(
    client: &reqwest::Client,
    url: &str,
    content_length: u64,
) -> Result<Bytes, JarParserError> {
    if content_length <= TAIL_SIZE {
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| JarParserError::Http(e.to_string()))?;
        return resp.bytes().await.map_err(|e| JarParserError::Http(e.to_string()));
    }
    let start = content_length - TAIL_SIZE;
    fetch_range(client, url, format!("bytes={start}-")).await
}

/// Fetch central directory slice.
pub async fn fetch_central_dir(
    client: &reqwest::Client,
    url: &str,
    eocd: &super::zip::Eocd,
) -> Result<Bytes, JarParserError> {
    let start = eocd.central_dir_offset as u64;
    let end = start + eocd.central_dir_size as u64 - 1;
    fetch_range(client, url, format!("bytes={start}-{end}")).await
}

/// Fetch raw file data for entry (parses local header to skip it).
pub async fn fetch_entry_raw(
    client: &reqwest::Client,
    url: &str,
    entry: &super::zip::CentralDirEntry,
) -> Result<Vec<u8>, JarParserError> {
    let start = entry.local_header_offset as u64;
    let end = start + 30 + 512 + entry.compressed_size as u64;
    let data = fetch_range(client, url, format!("bytes={start}-{end}")).await?;

    if data.len() < 30 {
        return Err(JarParserError::InvalidCentralDir("local header truncated".into()));
    }
    let file_name_len = u16::from_le_bytes(data[26..28].try_into().unwrap()) as usize;
    let extra_len = u16::from_le_bytes(data[28..30].try_into().unwrap()) as usize;
    let header_size = 30 + file_name_len + extra_len;
    if data.len() < header_size + entry.compressed_size as usize {
        return Err(JarParserError::InvalidCentralDir("entry truncated".into()));
    }
    let raw = &data[header_size..header_size + entry.compressed_size as usize];
    decompress_entry(raw, entry)
}

/// High-level: fetch and parse `fabric.mod.json` via Range Requests.
pub async fn fetch_fabric_mod_json(
    client: &reqwest::Client,
    url: &str,
) -> Result<FabricModJson, JarParserError> {
    let (content_length, supports_range) = fetch_head(client, url).await?;
    if !supports_range {
        return Err(JarParserError::Http("server does not support Range".into()));
    }

    let tail = fetch_tail(client, url, content_length).await?;
    let eocd = find_eocd(&tail)?;
    let cd_bytes = fetch_central_dir(client, url, &eocd).await?;
    let entries = parse_central_dir(&cd_bytes, &eocd)?;
    let entry = find_manifest_entry(&entries).ok_or(JarParserError::ManifestNotFound)?;
    let raw = fetch_entry_raw(client, url, entry).await?;

    if entry.file_name.ends_with(".toml") {
        return Err(JarParserError::Json("TOML manifest not in this path".into()));
    }

    serde_json::from_slice(&raw).map_err(|e| JarParserError::Json(e.to_string()))
}
