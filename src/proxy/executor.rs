//! HTTP request execution with detailed timing and TLS information.
//!
//! This module handles the core proxy request execution logic, supporting both
//! HTTP and HTTPS requests with redirect following, compression handling, and
//! detailed timing metrics.

use super::response_builder::{build_response, ResponseBuildParams};
use super::types::*;
use crate::infra::dns::resolve_dns;
use crate::infra::tls::create_tls_config;
use crate::shared::cert_parser::extract_cert_info;
use crate::shared::{CapturedCertInfo, DetailedTiming};
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, header::HeaderName, Method, Request, Version};
use hyper_util::rt::TokioIo;
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::{net::TcpStream, time::timeout};
use tokio_rustls::TlsConnector;

/// Maximum number of redirects to follow.
const MAX_REDIRECTS: usize = 20;

/// Default request timeout in milliseconds.
const DEFAULT_TIMEOUT_MS: u64 = 30000;

/// Context for tracking request state during redirect chain.
struct RequestContext {
    url: String,
    host: String,
    port: u16,
    path: String,
    is_https: bool,
}

impl RequestContext {
    fn from_url(url: &str) -> Result<Self, ProxyResponse> {
        let parsed_url = match url::Url::parse(url) {
            Ok(u) => u,
            Err(e) => {
                return Err(ProxyResponse::error(
                    format!("Invalid URL: {}", e),
                    "INVALID_URL".to_string(),
                ))
            }
        };

        let host = match parsed_url.host_str() {
            Some(h) => h.to_string(),
            None => {
                return Err(ProxyResponse::error(
                    "URL has no host".to_string(),
                    "INVALID_URL".to_string(),
                ))
            }
        };

        let is_https = parsed_url.scheme() == "https";
        let port = parsed_url.port().unwrap_or(if is_https { 443 } else { 80 });
        let path = if parsed_url.query().is_some() {
            format!("{}?{}", parsed_url.path(), parsed_url.query().unwrap())
        } else {
            parsed_url.path().to_string()
        };
        let path = if path.is_empty() { "/".to_string() } else { path };

        Ok(Self {
            url: url.to_string(),
            host,
            port,
            path,
            is_https,
        })
    }

    fn update_from_redirect(&mut self, location: &str) -> String {
        if location.starts_with("http://") || location.starts_with("https://") {
            // Absolute URL
            if let Ok(parsed) = url::Url::parse(location) {
                let new_is_https = parsed.scheme() == "https";
                let new_host = parsed.host_str().unwrap_or(&self.host).to_string();
                let explicit_port = parsed.port();
                let default_port = if new_is_https { 443 } else { 80 };

                // Smart port handling
                let new_port = if let Some(p) = explicit_port {
                    p
                } else if new_host == self.host && self.port != default_port {
                    self.port
                } else {
                    default_port
                };

                self.is_https = new_is_https;
                self.host = new_host;
                self.port = new_port;
                self.path = if parsed.query().is_some() {
                    format!("{}?{}", parsed.path(), parsed.query().unwrap())
                } else {
                    parsed.path().to_string()
                };

                // Rebuild URL with correct port
                let scheme = if self.is_https { "https" } else { "http" };
                let host_with_port = if self.port == default_port {
                    self.host.clone()
                } else {
                    format!("{}:{}", self.host, self.port)
                };
                let next_url = format!("{}://{}{}", scheme, host_with_port, self.path);
                self.url = next_url.clone();
                next_url
            } else {
                self.url = location.to_string();
                location.to_string()
            }
        } else {
            // Relative URL
            let scheme = if self.is_https { "https" } else { "http" };
            let default_port = if self.is_https { 443 } else { 80 };
            let host_with_port = if self.port == default_port {
                self.host.clone()
            } else {
                format!("{}:{}", self.host, self.port)
            };

            if location.starts_with('/') {
                self.path = location.to_string();
                let next_url = format!("{}://{}{}", scheme, host_with_port, location);
                self.url = next_url.clone();
                next_url
            } else {
                self.path = format!("/{}", location);
                let next_url = format!("{}://{}/{}", scheme, host_with_port, location);
                self.url = next_url.clone();
                next_url
            }
        }
    }
}

/// Builds an HTTP request with the given parameters.
fn build_http_request(
    method: &str,
    path: &str,
    host: &str,
    headers: &HashMap<String, String>,
    body: Option<&String>,
) -> Result<Request<Full<Bytes>>, ProxyResponse> {
    let method = match Method::from_str(&method.to_uppercase()) {
        Ok(m) => m,
        Err(_) => {
            return Err(ProxyResponse::error(
                format!("Invalid method: {}", method),
                "INVALID_METHOD".to_string(),
            ))
        }
    };

    let mut req_builder = Request::builder()
        .method(method)
        .uri(path)
        .header("Host", host);

    // Add headers
    for (key, value) in headers {
        if let Ok(name) = HeaderName::from_str(key) {
            req_builder = req_builder.header(name, value);
        }
    }

    // Add accept-encoding for compression
    if !headers.contains_key("accept-encoding") {
        req_builder = req_builder.header("Accept-Encoding", "gzip, deflate, br");
    }

    let body_content = body.cloned().unwrap_or_default();
    req_builder.body(Full::new(Bytes::from(body_content))).map_err(|e| {
        ProxyResponse::error(
            format!("Failed to build request: {}", e),
            "REQUEST_BUILD_ERROR".to_string(),
        )
    })
}

/// Sends an HTTP request and returns the response with headers.
async fn send_request<S>(
    sender: &mut hyper::client::conn::http1::SendRequest<Full<Bytes>>,
    req: Request<Full<Bytes>>,
    request_timeout: Duration,
) -> Result<hyper::Response<hyper::body::Incoming>, ProxyResponse>
where
    S: Send,
{
    match timeout(request_timeout, sender.send_request(req)).await {
        Ok(Ok(r)) => Ok(r),
        Ok(Err(e)) => Err(ProxyResponse::error(
            format!("Request failed: {}", e),
            "REQUEST_FAILED".to_string(),
        )),
        Err(_) => Err(ProxyResponse::error(
            "Request timed out".to_string(),
            "TIMEOUT".to_string(),
        )),
    }
}

/// Reads the response body with timeout.
async fn read_response_body(
    response: hyper::Response<hyper::body::Incoming>,
    request_timeout: Duration,
) -> Result<(u16, HashMap<String, String>, Version, Vec<u8>), ProxyResponse> {
    let version = response.version();
    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = match timeout(request_timeout, response.into_body().collect()).await {
        Ok(Ok(collected)) => collected.to_bytes().to_vec(),
        Ok(Err(e)) => {
            return Err(ProxyResponse::error(
                format!("Failed to read body: {}", e),
                "BODY_READ_ERROR".to_string(),
            ))
        }
        Err(_) => {
            return Err(ProxyResponse::error(
                "Body read timed out".to_string(),
                "TIMEOUT".to_string(),
            ))
        }
    };

    Ok((status, headers, version, body_bytes))
}

/// Execute HTTP request with detailed timing.
pub async fn execute_request(request: ProxyRequest) -> ProxyResponse {
    let mut timing = DetailedTiming::new();

    // Parse initial URL
    let mut ctx = match RequestContext::from_url(&request.url) {
        Ok(ctx) => ctx,
        Err(e) => return e,
    };

    let request_timeout = Duration::from_millis(request.timeout.unwrap_or(DEFAULT_TIMEOUT_MS));

    // DNS Resolution
    timing.start_dns();
    let dns_result = match resolve_dns(&ctx.host).await {
        Ok(r) => r,
        Err(e) => return ProxyResponse::error(e, "DNS_ERROR".to_string()),
    };
    timing.end_dns();

    let server_ip = dns_result.ips.first().copied();
    let addr = SocketAddr::new(server_ip.unwrap(), ctx.port);

    // TCP Connection
    timing.start_tcp();
    let tcp_stream = match timeout(request_timeout, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            return ProxyResponse::error(
                format!("TCP connection failed: {}", e),
                "CONNECTION_FAILED".to_string(),
            )
        }
        Err(_) => {
            return ProxyResponse::error(
                "TCP connection timed out".to_string(),
                "TIMEOUT".to_string(),
            )
        }
    };
    timing.end_tcp();

    let request_headers = request.headers.clone();
    let request_body_size = request.body.as_ref().map(|b| b.len());

    // Track redirect chain
    let mut redirect_chain: Vec<RedirectHop> = Vec::new();
    let mut tls_info: Option<CapturedCertInfo> = None;
    #[allow(unused_assignments)]
    let mut http_version = Version::HTTP_11;

    // For the first request, we already have the connection
    let mut maybe_tcp_stream = Some(tcp_stream);
    let mut is_first_request = true;

    loop {
        let hop_start = Instant::now();

        // Establish connection (reuse for first request)
        let tcp_stream = if let Some(stream) = maybe_tcp_stream.take() {
            stream
        } else {
            // New connection for redirect
            tracing::debug!(
                "Establishing new connection for redirect, current_url='{}'",
                ctx.url
            );

            let dns_result = match resolve_dns(&ctx.host).await {
                Ok(r) => r,
                Err(e) => return ProxyResponse::error(e, "DNS_ERROR".to_string()),
            };

            let redirect_addr = SocketAddr::new(dns_result.ips[0], ctx.port);
            tracing::debug!("Connecting to redirect address: {}", redirect_addr);

            match timeout(request_timeout, TcpStream::connect(redirect_addr)).await {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    return ProxyResponse::error(
                        format!("Redirect connection failed: {}", e),
                        "CONNECTION_FAILED".to_string(),
                    )
                }
                Err(_) => {
                    return ProxyResponse::error(
                        "Redirect connection timed out".to_string(),
                        "TIMEOUT".to_string(),
                    )
                }
            }
        };

        // Execute request based on protocol
        let (status, headers, version, body_bytes) = if ctx.is_https {
            match execute_https_request(
                tcp_stream,
                &ctx,
                &request,
                request_timeout,
                &mut timing,
                &mut tls_info,
                is_first_request,
            )
            .await
            {
                Ok(result) => result,
                Err(e) => return e,
            }
        } else {
            match execute_http_request(
                tcp_stream,
                &ctx,
                &request,
                request_timeout,
                &mut timing,
                is_first_request,
            )
            .await
            {
                Ok(result) => result,
                Err(e) => return e,
            }
        };

        http_version = version;

        // Check for redirect
        if (300..400).contains(&status) {
            if let Some(location) = headers.get("location").cloned() {
                let hop_duration = hop_start.elapsed().as_millis() as u64;
                let current_url = ctx.url.clone();
                let next_url = ctx.update_from_redirect(&location);

                redirect_chain.push(RedirectHop {
                    url: current_url,
                    status,
                    duration: hop_duration,
                    headers: Some(headers),
                    opaque: None,
                    message: Some(format!("Redirect to: {}", next_url)),
                });

                if redirect_chain.len() >= MAX_REDIRECTS {
                    return ProxyResponse::error(
                        "Too many redirects".to_string(),
                        "TOO_MANY_REDIRECTS".to_string(),
                    );
                }

                is_first_request = false;
                continue;
            }
        }

        // Build and return response
        return build_response(ResponseBuildParams {
            status,
            headers,
            body_bytes,
            timing,
            final_url: ctx.url,
            redirect_chain,
            tls_info,
            http_version,
            server_ip,
            request_headers,
            request_body_size,
        });
    }
}

/// Executes an HTTPS request over a TLS connection.
async fn execute_https_request(
    tcp_stream: TcpStream,
    ctx: &RequestContext,
    request: &ProxyRequest,
    request_timeout: Duration,
    timing: &mut DetailedTiming,
    tls_info: &mut Option<CapturedCertInfo>,
    is_first_request: bool,
) -> Result<(u16, HashMap<String, String>, Version, Vec<u8>), ProxyResponse> {
    if is_first_request {
        timing.start_tls();
    }

    let tls_config = create_tls_config();
    let connector = TlsConnector::from(tls_config);

    let server_name = match rustls::pki_types::ServerName::try_from(ctx.host.clone()) {
        Ok(name) => name,
        Err(e) => {
            return Err(ProxyResponse::error(
                format!("Invalid server name: {}", e),
                "TLS_ERROR".to_string(),
            ))
        }
    };

    let tls_stream = match timeout(request_timeout, connector.connect(server_name, tcp_stream)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            return Err(ProxyResponse::error(
                format!("TLS handshake failed: {}", e),
                "TLS_ERROR".to_string(),
            ))
        }
        Err(_) => {
            return Err(ProxyResponse::error(
                "TLS handshake timed out".to_string(),
                "TIMEOUT".to_string(),
            ))
        }
    };

    if is_first_request {
        timing.end_tls();
        *tls_info = extract_cert_info(&tls_stream);
    }

    // Send HTTP request over TLS
    let io = TokioIo::new(tls_stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(r) => r,
        Err(e) => {
            return Err(ProxyResponse::error(
                format!("HTTP handshake failed: {}", e),
                "HTTP_ERROR".to_string(),
            ))
        }
    };

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::warn!("Connection error: {}", e);
        }
    });

    // Build and send request
    let req = build_http_request(
        &request.method,
        &ctx.path,
        &ctx.host,
        &request.headers,
        request.body.as_ref(),
    )?;

    if is_first_request {
        timing.start_request();
    }

    let response = send_request::<()>(&mut sender, req, request_timeout).await?;

    if is_first_request {
        timing.mark_ttfb();
    }

    // Read body
    timing.start_download();
    let (status, headers, version, body_bytes) = read_response_body(response, request_timeout).await?;
    timing.end_download();

    Ok((status, headers, version, body_bytes))
}

/// Executes an HTTP request (non-TLS).
async fn execute_http_request(
    tcp_stream: TcpStream,
    ctx: &RequestContext,
    request: &ProxyRequest,
    request_timeout: Duration,
    timing: &mut DetailedTiming,
    is_first_request: bool,
) -> Result<(u16, HashMap<String, String>, Version, Vec<u8>), ProxyResponse> {
    let io = TokioIo::new(tcp_stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(r) => r,
        Err(e) => {
            return Err(ProxyResponse::error(
                format!("HTTP handshake failed: {}", e),
                "HTTP_ERROR".to_string(),
            ))
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::warn!("Connection error: {}", e);
        }
    });

    // Build and send request
    let req = build_http_request(
        &request.method,
        &ctx.path,
        &ctx.host,
        &request.headers,
        request.body.as_ref(),
    )?;

    if is_first_request {
        timing.start_request();
    }

    let response = send_request::<()>(&mut sender, req, request_timeout).await?;

    if is_first_request {
        timing.mark_ttfb();
    }

    // Read body
    timing.start_download();
    let (status, headers, version, body_bytes) = read_response_body(response, request_timeout).await?;
    timing.end_download();

    Ok((status, headers, version, body_bytes))
}
