//! Response building utilities for proxy responses.
//!
//! Handles the construction of `ProxyResponse` from raw HTTP response data,
//! including decompression, binary detection, and size calculations.

use super::types::*;
use crate::infra::decompress_body;
use crate::shared::{status_text, CapturedCertInfo, DetailedTiming};
use base64::Engine;
use hyper::Version;
use std::collections::HashMap;
use std::net::IpAddr;

/// Determines if response body is likely binary based on content-type.
///
/// # Arguments
///
/// * `content_type` - The Content-Type header value
///
/// # Returns
///
/// `true` if the content is likely binary, `false` otherwise.
pub fn is_binary_content(content_type: Option<&str>) -> bool {
    let ct = match content_type {
        Some(ct) => ct.to_lowercase(),
        None => return false,
    };

    let text_types = [
        "text/",
        "application/json",
        "application/xml",
        "application/javascript",
        "application/x-javascript",
        "application/ecmascript",
        "application/x-www-form-urlencoded",
        "+json",
        "+xml",
    ];

    !text_types.iter().any(|t| ct.contains(t))
}

/// Converts an HTTP version to its string representation.
pub fn version_to_string(version: Version) -> String {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/1.1",
    }
    .to_string()
}

/// Parameters for building a proxy response.
pub struct ResponseBuildParams {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body_bytes: Vec<u8>,
    pub timing: DetailedTiming,
    pub final_url: String,
    pub redirect_chain: Vec<RedirectHop>,
    pub tls_info: Option<CapturedCertInfo>,
    pub http_version: Version,
    pub server_ip: Option<IpAddr>,
    pub request_headers: HashMap<String, String>,
    pub request_body_size: Option<usize>,
}

/// Builds a `ProxyResponse` from raw response data.
///
/// Handles:
/// - Body decompression (gzip, deflate, brotli)
/// - Binary content detection and base64 encoding
/// - Size breakdown calculation
/// - TLS info conversion
///
/// # Arguments
///
/// * `params` - The response build parameters
///
/// # Returns
///
/// A `ProxyResponse` ready to be serialized and sent to the client.
pub fn build_response(params: ResponseBuildParams) -> ProxyResponse {
    let ResponseBuildParams {
        status,
        headers,
        body_bytes,
        timing,
        final_url,
        redirect_chain,
        tls_info,
        http_version,
        server_ip,
        request_headers,
        request_body_size,
    } = params;

    let content_type = headers.get("content-type").map(|s| s.as_str());
    let content_encoding = headers.get("content-encoding").map(|s| s.as_str());
    let is_binary = is_binary_content(content_type);

    // Decompress if needed
    let compressed_size = body_bytes.len();
    let decompressed: Vec<u8> = match decompress_body(&body_bytes, content_encoding) {
        Ok(d) => d,
        Err(e) => {
            return ProxyResponse::error(e, "DECOMPRESSION_ERROR".to_string());
        }
    };
    let body_size = decompressed.len();

    // Convert body
    let (body, body_base64) = if is_binary {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&decompressed);
        (String::new(), Some(b64))
    } else {
        (String::from_utf8_lossy(&decompressed).to_string(), None)
    };

    // Calculate sizes
    let status_line = format!(
        "{} {} {}",
        version_to_string(http_version),
        status,
        status_text(status)
    );
    let header_size: usize = headers
        .iter()
        .map(|(k, v)| k.len() + 2 + v.len() + 2)
        .sum::<usize>()
        + status_line.len()
        + 2;

    let compression_ratio = if content_encoding.is_some() && body_size > 0 {
        Some(compressed_size as f64 / body_size as f64)
    } else {
        None
    };

    let size_breakdown = SizeBreakdown {
        headers: header_size,
        body: body_size,
        total: header_size + body_size,
        compressed: if content_encoding.is_some() {
            Some(compressed_size)
        } else {
            None
        },
        uncompressed: if content_encoding.is_some() {
            Some(body_size)
        } else {
            None
        },
        encoding: content_encoding.map(|s| s.to_string()),
        compression_ratio,
    };

    // Build TLS info
    let tls = tls_info.map(|info| TlsInfo {
        protocol: Some(info.protocol),
        cipher: Some(info.cipher),
        issuer: info.issuer,
        subject: info.subject,
        valid_from: info.valid_from,
        valid_to: info.valid_to,
        valid: Some(true),
    });

    let server_software = headers.get("server").cloned();
    let connection = headers.get("connection").cloned();

    let data = ResponseData {
        status,
        status_text: status_text(status),
        headers,
        request_headers: Some(request_headers),
        body,
        body_base64,
        is_binary,
        size: body_size,
        timing: timing.to_timing_info(),
        url: final_url,
        redirected: !redirect_chain.is_empty(),
        redirect_chain: if redirect_chain.is_empty() {
            None
        } else {
            Some(redirect_chain)
        },
        tls,
        size_breakdown: Some(size_breakdown),
        server_ip: server_ip.map(|ip| ip.to_string()),
        protocol: Some(version_to_string(http_version)),
        from_cache: Some(false),
        resource_type: Some("fetch".to_string()),
        request_body_size,
        connection,
        server_software,
    };

    ProxyResponse::success(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary_content() {
        assert!(!is_binary_content(Some("text/html")));
        assert!(!is_binary_content(Some("application/json")));
        assert!(!is_binary_content(Some("application/xml")));
        assert!(!is_binary_content(Some("text/plain; charset=utf-8")));
        assert!(is_binary_content(Some("image/png")));
        assert!(is_binary_content(Some("application/octet-stream")));
        assert!(!is_binary_content(None));
    }

    #[test]
    fn test_version_to_string() {
        assert_eq!(version_to_string(Version::HTTP_11), "HTTP/1.1");
        assert_eq!(version_to_string(Version::HTTP_2), "HTTP/2");
        assert_eq!(version_to_string(Version::HTTP_10), "HTTP/1.0");
    }
}
