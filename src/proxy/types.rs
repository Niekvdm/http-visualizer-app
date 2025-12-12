use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Incoming proxy request from the frontend
#[derive(Debug, Deserialize)]
pub struct ProxyRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    /// Timeout in milliseconds
    pub timeout: Option<u64>,
}

/// Detailed timing information
#[derive(Debug, Serialize, Default)]
pub struct TimingInfo {
    /// Total request time in milliseconds
    pub total: u64,
    /// DNS lookup time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<u64>,
    /// TCP connection time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp: Option<u64>,
    /// TLS handshake time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<u64>,
    /// Time to first byte
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttfb: Option<u64>,
    /// Content download time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<u64>,
    /// Time blocked/queued
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<u64>,
}

/// Redirect hop information
#[derive(Debug, Serialize)]
pub struct RedirectHop {
    pub url: String,
    pub status: u16,
    pub duration: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opaque: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// TLS/SSL information
#[derive(Debug, Serialize)]
pub struct TlsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cipher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
}

/// Size breakdown information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeBreakdown {
    pub headers: usize,
    pub body: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncompressed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_ratio: Option<f64>,
}

/// Successful response data matching extension protocol
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<HashMap<String, String>>,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_base64: Option<String>,
    pub is_binary: bool,
    pub size: usize,
    pub timing: TimingInfo,
    pub url: String,
    pub redirected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_chain: Option<Vec<RedirectHop>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_breakdown: Option<SizeBreakdown>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_software: Option<String>,
}

/// Error data matching extension protocol
#[derive(Debug, Serialize)]
pub struct ErrorData {
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Full proxy response matching extension protocol
#[derive(Debug, Serialize)]
pub struct ProxyResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorData>,
}

impl ProxyResponse {
    pub fn success(data: ResponseData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String, code: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorData {
                message,
                code,
                name: None,
            }),
        }
    }
}
