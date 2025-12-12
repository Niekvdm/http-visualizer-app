use super::types::*;
use base64::Engine;
use hickory_resolver::{config::*, TokioAsyncResolver};
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, header::HeaderName, Method, Request, Version};
use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use std::{
    collections::HashMap,
    io::Read,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{net::TcpStream, sync::OnceCell, time::timeout};
use tokio_rustls::TlsConnector;
use x509_parser::prelude::*;

// Global DNS resolver
static DNS_RESOLVER: OnceCell<Arc<TokioAsyncResolver>> = OnceCell::const_new();

async fn get_resolver() -> Arc<TokioAsyncResolver> {
    DNS_RESOLVER
        .get_or_init(|| async {
            Arc::new(TokioAsyncResolver::tokio(
                ResolverConfig::default(),
                ResolverOpts::default(),
            ))
        })
        .await
        .clone()
}

/// Captured TLS certificate information
#[derive(Debug, Clone)]
struct CapturedCertInfo {
    protocol: String,
    cipher: String,
    issuer: Option<String>,
    subject: Option<String>,
    valid_from: Option<u64>,
    valid_to: Option<u64>,
    san: Vec<String>,
}

/// Resolve DNS and return IPs with timing
async fn resolve_dns(host: &str) -> Result<(Vec<IpAddr>, u64), String> {
    let start = Instant::now();

    // Check if already an IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok((vec![ip], 0));
    }

    let resolver = get_resolver().await;
    match resolver.lookup_ip(host).await {
        Ok(response) => {
            let duration = start.elapsed().as_millis() as u64;
            let ips: Vec<IpAddr> = response.iter().collect();
            if ips.is_empty() {
                Err("DNS lookup returned no addresses".to_string())
            } else {
                Ok((ips, duration))
            }
        }
        Err(e) => Err(format!("DNS lookup failed: {}", e)),
    }
}

/// Create TLS config that captures certificate info
fn create_tls_config() -> Arc<rustls::ClientConfig> {
    let root_store = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Arc::new(config)
}

/// Extract certificate info from TLS connection
fn extract_cert_info(
    conn: &tokio_rustls::client::TlsStream<TcpStream>,
) -> Option<CapturedCertInfo> {
    let (_, client_conn) = conn.get_ref();

    // Get protocol version
    let protocol = match client_conn.protocol_version() {
        Some(rustls::ProtocolVersion::TLSv1_2) => "TLS 1.2".to_string(),
        Some(rustls::ProtocolVersion::TLSv1_3) => "TLS 1.3".to_string(),
        _ => "TLS".to_string(),
    };

    // Get cipher suite
    let cipher = client_conn
        .negotiated_cipher_suite()
        .map(|cs| format!("{:?}", cs.suite()))
        .unwrap_or_else(|| "Unknown".to_string());

    // Get peer certificates
    let certs = client_conn.peer_certificates()?;
    let cert = certs.first()?;

    // Parse the certificate using x509-parser would be ideal, but let's extract what we can
    // For now, we'll parse basic info from the DER-encoded certificate
    let cert_info = parse_x509_basic(cert.as_ref());

    Some(CapturedCertInfo {
        protocol,
        cipher,
        issuer: cert_info.issuer,
        subject: cert_info.subject,
        valid_from: cert_info.valid_from,
        valid_to: cert_info.valid_to,
        san: cert_info.san,
    })
}

/// Basic X.509 certificate parsing using x509-parser
struct BasicCertInfo {
    issuer: Option<String>,
    subject: Option<String>,
    valid_from: Option<u64>,
    valid_to: Option<u64>,
    san: Vec<String>,
}

fn parse_x509_basic(der: &[u8]) -> BasicCertInfo {
    let mut info = BasicCertInfo {
        issuer: None,
        subject: None,
        valid_from: None,
        valid_to: None,
        san: Vec::new(),
    };

    // Parse using x509-parser
    if let Ok((_, cert)) = X509Certificate::from_der(der) {
        // Extract subject CN or full subject
        info.subject = cert
            .subject()
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
            .map(|s| s.to_string())
            .or_else(|| Some(cert.subject().to_string()));

        // Extract issuer CN or full issuer
        info.issuer = cert
            .issuer()
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
            .map(|s| s.to_string())
            .or_else(|| Some(cert.issuer().to_string()));

        // Extract validity dates as Unix timestamps
        info.valid_from = Some(cert.validity().not_before.timestamp() as u64);
        info.valid_to = Some(cert.validity().not_after.timestamp() as u64);

        // Extract Subject Alternative Names
        if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
            for name in &san_ext.value.general_names {
                match name {
                    GeneralName::DNSName(dns) => {
                        info.san.push(dns.to_string());
                    }
                    GeneralName::IPAddress(ip) => {
                        if ip.len() == 4 {
                            info.san.push(format!(
                                "{}.{}.{}.{}",
                                ip[0], ip[1], ip[2], ip[3]
                            ));
                        } else if ip.len() == 16 {
                            // IPv6 - simplified representation
                            info.san.push(format!("IPv6:{:02x}{:02x}:...", ip[0], ip[1]));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    info
}

/// Determine if response body is likely binary based on content-type
fn is_binary_content(content_type: Option<&str>) -> bool {
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

fn status_text(status: u16) -> String {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
    .to_string()
}

fn version_to_string(version: Version) -> String {
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

/// Decompress body based on content-encoding
fn decompress_body(body: &[u8], encoding: Option<&str>) -> Result<Vec<u8>, String> {
    match encoding {
        Some("gzip") => {
            let mut decoder = flate2::read::GzDecoder::new(body);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| format!("Gzip decompression failed: {}", e))?;
            Ok(decompressed)
        }
        Some("deflate") => {
            let mut decoder = flate2::read::DeflateDecoder::new(body);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| format!("Deflate decompression failed: {}", e))?;
            Ok(decompressed)
        }
        Some("br") => {
            let mut decompressed = Vec::new();
            brotli::BrotliDecompress(&mut std::io::Cursor::new(body), &mut decompressed)
                .map_err(|e| format!("Brotli decompression failed: {}", e))?;
            Ok(decompressed)
        }
        _ => Ok(body.to_vec()),
    }
}

/// Detailed timing measurements
#[derive(Debug)]
struct DetailedTiming {
    dns_start: Option<Instant>,
    dns_end: Option<Instant>,
    tcp_start: Option<Instant>,
    tcp_end: Option<Instant>,
    tls_start: Option<Instant>,
    tls_end: Option<Instant>,
    request_start: Option<Instant>,
    ttfb: Option<Instant>,
    download_start: Option<Instant>,
    download_end: Option<Instant>,
    total_start: Instant,
}

impl DetailedTiming {
    fn new() -> Self {
        Self {
            dns_start: None,
            dns_end: None,
            tcp_start: None,
            tcp_end: None,
            tls_start: None,
            tls_end: None,
            request_start: None,
            ttfb: None,
            download_start: None,
            download_end: None,
            total_start: Instant::now(),
        }
    }

    fn to_timing_info(&self) -> TimingInfo {
        let total = self
            .download_end
            .unwrap_or_else(Instant::now)
            .duration_since(self.total_start)
            .as_millis() as u64;

        let dns = match (self.dns_start, self.dns_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let tcp = match (self.tcp_start, self.tcp_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let tls = match (self.tls_start, self.tls_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let ttfb = match (self.request_start, self.ttfb) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let download = match (self.download_start, self.download_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        TimingInfo {
            total,
            dns,
            tcp,
            tls,
            ttfb,
            download,
            blocked: Some(0),
        }
    }
}

/// Execute HTTP request with detailed timing
pub async fn execute_request(request: ProxyRequest) -> ProxyResponse {
    let mut timing = DetailedTiming::new();

    // Parse URL
    let parsed_url = match url::Url::parse(&request.url) {
        Ok(u) => u,
        Err(e) => {
            return ProxyResponse::error(format!("Invalid URL: {}", e), "INVALID_URL".to_string())
        }
    };

    let host = match parsed_url.host_str() {
        Some(h) => h.to_string(),
        None => {
            return ProxyResponse::error("URL has no host".to_string(), "INVALID_URL".to_string())
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

    let request_timeout = Duration::from_millis(request.timeout.unwrap_or(30000));

    // DNS Resolution
    timing.dns_start = Some(Instant::now());
    let (ips, _dns_time) = match resolve_dns(&host).await {
        Ok(r) => r,
        Err(e) => return ProxyResponse::error(e, "DNS_ERROR".to_string()),
    };
    timing.dns_end = Some(Instant::now());

    let server_ip = ips.first().copied();
    let addr = SocketAddr::new(server_ip.unwrap(), port);

    // TCP Connection
    timing.tcp_start = Some(Instant::now());
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
    timing.tcp_end = Some(Instant::now());

    // Get local address for connection info
    let _local_addr = tcp_stream.local_addr().ok();

    let request_headers = request.headers.clone();
    let request_body_size = request.body.as_ref().map(|b| b.len());

    // Track redirect chain
    let mut redirect_chain: Vec<RedirectHop> = Vec::new();
    let mut current_url = request.url.clone();
    let mut current_host = host.clone();
    let mut current_port = port;
    let mut current_path = path.clone();
    let mut current_is_https = is_https;
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
            tracing::debug!("Establishing new connection for redirect, current_url='{}'", current_url);
            let parsed = url::Url::parse(&current_url).unwrap();
            current_host = parsed.host_str().unwrap_or(&host).to_string();
            let redirect_port = parsed.port().unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });
            tracing::debug!("Parsed redirect URL: host='{}', port={}, parsed.port()={:?}",
                current_host, redirect_port, parsed.port());
            current_path = if parsed.query().is_some() {
                format!("{}?{}", parsed.path(), parsed.query().unwrap())
            } else {
                parsed.path().to_string()
            };
            if current_path.is_empty() {
                current_path = "/".to_string();
            }

            let (redirect_ips, _) = match resolve_dns(&current_host).await {
                Ok(r) => r,
                Err(e) => return ProxyResponse::error(e, "DNS_ERROR".to_string()),
            };

            let redirect_addr = SocketAddr::new(redirect_ips[0], redirect_port);
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

        // TLS Handshake (if HTTPS)
        if current_is_https {
            if is_first_request {
                timing.tls_start = Some(Instant::now());
            }

            let tls_config = create_tls_config();
            let connector = TlsConnector::from(tls_config);

            let server_name = match ServerName::try_from(current_host.clone()) {
                Ok(name) => name,
                Err(e) => {
                    return ProxyResponse::error(
                        format!("Invalid server name: {}", e),
                        "TLS_ERROR".to_string(),
                    )
                }
            };

            let tls_stream = match timeout(request_timeout, connector.connect(server_name, tcp_stream)).await
            {
                Ok(Ok(stream)) => stream,
                Ok(Err(e)) => {
                    return ProxyResponse::error(
                        format!("TLS handshake failed: {}", e),
                        "TLS_ERROR".to_string(),
                    )
                }
                Err(_) => {
                    return ProxyResponse::error(
                        "TLS handshake timed out".to_string(),
                        "TIMEOUT".to_string(),
                    )
                }
            };

            if is_first_request {
                timing.tls_end = Some(Instant::now());
                // Extract certificate info
                tls_info = extract_cert_info(&tls_stream);
            }

            // Send HTTP request over TLS
            let io = TokioIo::new(tls_stream);

            let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
                Ok(r) => r,
                Err(e) => {
                    return ProxyResponse::error(
                        format!("HTTP handshake failed: {}", e),
                        "HTTP_ERROR".to_string(),
                    )
                }
            };

            // Spawn connection handler
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::warn!("Connection error: {}", e);
                }
            });

            // Build request
            let method = match Method::from_str(&request.method.to_uppercase()) {
                Ok(m) => m,
                Err(_) => {
                    return ProxyResponse::error(
                        format!("Invalid method: {}", request.method),
                        "INVALID_METHOD".to_string(),
                    )
                }
            };

            let mut req_builder = Request::builder()
                .method(method)
                .uri(&current_path)
                .header("Host", &current_host);

            // Add headers
            for (key, value) in &request.headers {
                if let Ok(name) = HeaderName::from_str(key) {
                    req_builder = req_builder.header(name, value);
                }
            }

            // Add accept-encoding for compression
            if !request.headers.contains_key("accept-encoding") {
                req_builder = req_builder.header("Accept-Encoding", "gzip, deflate, br");
            }

            let body = request.body.clone().unwrap_or_default();
            let req = match req_builder.body(Full::new(Bytes::from(body))) {
                Ok(r) => r,
                Err(e) => {
                    return ProxyResponse::error(
                        format!("Failed to build request: {}", e),
                        "REQUEST_BUILD_ERROR".to_string(),
                    )
                }
            };

            if is_first_request {
                timing.request_start = Some(Instant::now());
            }

            // Send request
            let response = match timeout(request_timeout, sender.send_request(req)).await {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    return ProxyResponse::error(
                        format!("Request failed: {}", e),
                        "REQUEST_FAILED".to_string(),
                    )
                }
                Err(_) => {
                    return ProxyResponse::error("Request timed out".to_string(), "TIMEOUT".to_string())
                }
            };

            if is_first_request {
                timing.ttfb = Some(Instant::now());
            }

            http_version = response.version();
            let status = response.status().as_u16();
            let headers: HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            // Check for redirect
            if (300..400).contains(&status) {
                if let Some(location) = headers.get("location").cloned() {
                    let hop_duration = hop_start.elapsed().as_millis() as u64;

                    // Build the next URL, preserving port for redirects
                    let next_url = if location.starts_with("http://") || location.starts_with("https://") {
                        // Absolute URL - parse it to update current_* variables
                        if let Ok(parsed) = url::Url::parse(&location) {
                            let new_is_https = parsed.scheme() == "https";
                            let new_host = parsed.host_str().unwrap_or(&current_host).to_string();
                            let explicit_port = parsed.port();
                            let default_port = if new_is_https { 443 } else { 80 };

                            // Smart port handling: if redirecting to same host without explicit port,
                            // and we're on a non-standard port, preserve the original port
                            let new_port = if explicit_port.is_some() {
                                explicit_port.unwrap()
                            } else if new_host == current_host && current_port != default_port {
                                // Same host, no explicit port, we're on non-standard port - preserve it
                                current_port
                            } else {
                                default_port
                            };

                            current_is_https = new_is_https;
                            current_host = new_host;
                            current_port = new_port;
                            current_path = if parsed.query().is_some() {
                                format!("{}?{}", parsed.path(), parsed.query().unwrap())
                            } else {
                                parsed.path().to_string()
                            };

                            // Rebuild URL with correct port
                            let scheme = if current_is_https { "https" } else { "http" };
                            let host_with_port = if current_port == default_port {
                                current_host.clone()
                            } else {
                                format!("{}:{}", current_host, current_port)
                            };
                            format!("{}://{}{}", scheme, host_with_port, current_path)
                        } else {
                            location.clone()
                        }
                    } else {
                        // Relative URL - use current host and port
                        let scheme = if current_is_https { "https" } else { "http" };
                        let default_port = if current_is_https { 443 } else { 80 };
                        let host_with_port = if current_port == default_port {
                            current_host.clone()
                        } else {
                            format!("{}:{}", current_host, current_port)
                        };

                        if location.starts_with('/') {
                            current_path = location.clone();
                            format!("{}://{}{}", scheme, host_with_port, &location)
                        } else {
                            current_path = format!("/{}", location);
                            format!("{}://{}/{}", scheme, host_with_port, &location)
                        }
                    };

                    redirect_chain.push(RedirectHop {
                        url: current_url.clone(),
                        status,
                        duration: hop_duration,
                        headers: Some(headers),
                        opaque: None,
                        message: Some(format!("Redirect to: {}", next_url)),
                    });

                    if redirect_chain.len() >= 20 {
                        return ProxyResponse::error(
                            "Too many redirects".to_string(),
                            "TOO_MANY_REDIRECTS".to_string(),
                        );
                    }

                    current_url = next_url;
                    is_first_request = false;
                    continue;
                }
            }

            // Read body
            timing.download_start = Some(Instant::now());
            let body_bytes = match response.into_body().collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    return ProxyResponse::error(
                        format!("Failed to read body: {}", e),
                        "BODY_READ_ERROR".to_string(),
                    )
                }
            };
            timing.download_end = Some(Instant::now());

            return build_response(
                status,
                headers,
                body_bytes.to_vec(),
                timing,
                current_url,
                redirect_chain,
                tls_info,
                http_version,
                server_ip,
                request_headers,
                request_body_size,
            );
        } else {
            // HTTP (non-TLS)
            let io = TokioIo::new(tcp_stream);

            let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
                Ok(r) => r,
                Err(e) => {
                    return ProxyResponse::error(
                        format!("HTTP handshake failed: {}", e),
                        "HTTP_ERROR".to_string(),
                    )
                }
            };

            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    tracing::warn!("Connection error: {}", e);
                }
            });

            let method = match Method::from_str(&request.method.to_uppercase()) {
                Ok(m) => m,
                Err(_) => {
                    return ProxyResponse::error(
                        format!("Invalid method: {}", request.method),
                        "INVALID_METHOD".to_string(),
                    )
                }
            };

            let mut req_builder = Request::builder()
                .method(method)
                .uri(&current_path)
                .header("Host", &current_host);

            for (key, value) in &request.headers {
                if let Ok(name) = HeaderName::from_str(key) {
                    req_builder = req_builder.header(name, value);
                }
            }

            if !request.headers.contains_key("accept-encoding") {
                req_builder = req_builder.header("Accept-Encoding", "gzip, deflate, br");
            }

            let body = request.body.clone().unwrap_or_default();
            let req = match req_builder.body(Full::new(Bytes::from(body))) {
                Ok(r) => r,
                Err(e) => {
                    return ProxyResponse::error(
                        format!("Failed to build request: {}", e),
                        "REQUEST_BUILD_ERROR".to_string(),
                    )
                }
            };

            if is_first_request {
                timing.request_start = Some(Instant::now());
            }

            let response = match timeout(request_timeout, sender.send_request(req)).await {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    return ProxyResponse::error(
                        format!("Request failed: {}", e),
                        "REQUEST_FAILED".to_string(),
                    )
                }
                Err(_) => {
                    return ProxyResponse::error("Request timed out".to_string(), "TIMEOUT".to_string())
                }
            };

            if is_first_request {
                timing.ttfb = Some(Instant::now());
            }

            http_version = response.version();
            let status = response.status().as_u16();
            let headers: HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            // Check for redirect
            if (300..400).contains(&status) {
                if let Some(location) = headers.get("location").cloned() {
                    let hop_duration = hop_start.elapsed().as_millis() as u64;

                    // Build the next URL, preserving port for redirects
                    let next_url = if location.starts_with("http://") || location.starts_with("https://") {
                        // Absolute URL - parse it to update current_* variables
                        if let Ok(parsed) = url::Url::parse(&location) {
                            let new_is_https = parsed.scheme() == "https";
                            let new_host = parsed.host_str().unwrap_or(&current_host).to_string();
                            let explicit_port = parsed.port();
                            let default_port = if new_is_https { 443 } else { 80 };

                            // Smart port handling: if redirecting to same host without explicit port,
                            // and we're on a non-standard port, preserve the original port
                            let new_port = if explicit_port.is_some() {
                                explicit_port.unwrap()
                            } else if new_host == current_host && current_port != default_port {
                                // Same host, no explicit port, we're on non-standard port - preserve it
                                current_port
                            } else {
                                default_port
                            };

                            current_is_https = new_is_https;
                            current_host = new_host;
                            current_port = new_port;
                            current_path = if parsed.query().is_some() {
                                format!("{}?{}", parsed.path(), parsed.query().unwrap())
                            } else {
                                parsed.path().to_string()
                            };

                            // Rebuild URL with correct port
                            let scheme = if current_is_https { "https" } else { "http" };
                            let host_with_port = if current_port == default_port {
                                current_host.clone()
                            } else {
                                format!("{}:{}", current_host, current_port)
                            };
                            format!("{}://{}{}", scheme, host_with_port, current_path)
                        } else {
                            location.clone()
                        }
                    } else {
                        // Relative URL - use current host and port
                        let scheme = if current_is_https { "https" } else { "http" };
                        let default_port = if current_is_https { 443 } else { 80 };
                        let host_with_port = if current_port == default_port {
                            current_host.clone()
                        } else {
                            format!("{}:{}", current_host, current_port)
                        };

                        if location.starts_with('/') {
                            current_path = location.clone();
                            format!("{}://{}{}", scheme, host_with_port, &location)
                        } else {
                            current_path = format!("/{}", location);
                            format!("{}://{}/{}", scheme, host_with_port, &location)
                        }
                    };

                    redirect_chain.push(RedirectHop {
                        url: current_url.clone(),
                        status,
                        duration: hop_duration,
                        headers: Some(headers),
                        opaque: None,
                        message: Some(format!("Redirect to: {}", next_url)),
                    });

                    if redirect_chain.len() >= 20 {
                        return ProxyResponse::error(
                            "Too many redirects".to_string(),
                            "TOO_MANY_REDIRECTS".to_string(),
                        );
                    }

                    tracing::debug!("HTTP redirect: location='{}', next_url='{}', current_port={}",
                        location, next_url, current_port);

                    current_url = next_url;
                    is_first_request = false;
                    continue;
                }
            }

            // Read body
            timing.download_start = Some(Instant::now());
            let body_bytes = match response.into_body().collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    return ProxyResponse::error(
                        format!("Failed to read body: {}", e),
                        "BODY_READ_ERROR".to_string(),
                    )
                }
            };
            timing.download_end = Some(Instant::now());

            return build_response(
                status,
                headers,
                body_bytes.to_vec(),
                timing,
                current_url,
                redirect_chain,
                None,
                http_version,
                server_ip,
                request_headers,
                request_body_size,
            );
        }
    }
}

fn build_response(
    status: u16,
    headers: HashMap<String, String>,
    body_bytes: Vec<u8>,
    timing: DetailedTiming,
    final_url: String,
    redirect_chain: Vec<RedirectHop>,
    tls_info: Option<CapturedCertInfo>,
    http_version: Version,
    server_ip: Option<IpAddr>,
    request_headers: HashMap<String, String>,
    request_body_size: Option<usize>,
) -> ProxyResponse {
    let content_type = headers.get("content-type").map(|s| s.as_str());
    let content_encoding = headers.get("content-encoding").map(|s| s.as_str());
    let is_binary = is_binary_content(content_type);

    // Decompress if needed
    let compressed_size = body_bytes.len();
    let decompressed = match decompress_body(&body_bytes, content_encoding) {
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
    let status_line = format!("{} {} {}", version_to_string(http_version), status, status_text(status));
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
