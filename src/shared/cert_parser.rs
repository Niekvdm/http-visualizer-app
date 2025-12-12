//! X.509 certificate parsing utilities.
//!
//! Provides functionality for extracting information from TLS certificates.

use tokio::net::TcpStream;
use x509_parser::prelude::*;

/// Captured TLS certificate information.
#[derive(Debug, Clone)]
pub struct CapturedCertInfo {
    pub protocol: String,
    pub cipher: String,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub valid_from: Option<u64>,
    pub valid_to: Option<u64>,
    pub san: Vec<String>,
}

/// Basic X.509 certificate information extracted from DER-encoded data.
#[derive(Debug)]
pub struct BasicCertInfo {
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub valid_from: Option<u64>,
    pub valid_to: Option<u64>,
    pub san: Vec<String>,
}

impl Default for BasicCertInfo {
    fn default() -> Self {
        Self {
            issuer: None,
            subject: None,
            valid_from: None,
            valid_to: None,
            san: Vec::new(),
        }
    }
}

/// Parses basic certificate information from DER-encoded X.509 data.
///
/// # Arguments
///
/// * `der` - The DER-encoded certificate data
///
/// # Returns
///
/// A `BasicCertInfo` struct containing the parsed certificate information.
/// Fields that cannot be parsed will be `None` or empty.
pub fn parse_x509_basic(der: &[u8]) -> BasicCertInfo {
    let mut info = BasicCertInfo::default();

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

/// Extracts certificate info from a TLS connection.
///
/// # Arguments
///
/// * `conn` - A reference to a TLS stream
///
/// # Returns
///
/// `Some(CapturedCertInfo)` if certificate information could be extracted,
/// `None` otherwise.
pub fn extract_cert_info(
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

    // Parse the certificate
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_invalid_der() {
        let info = parse_x509_basic(&[0, 1, 2, 3]);
        assert!(info.issuer.is_none());
        assert!(info.subject.is_none());
    }

    #[test]
    fn test_basic_cert_info_default() {
        let info = BasicCertInfo::default();
        assert!(info.issuer.is_none());
        assert!(info.subject.is_none());
        assert!(info.valid_from.is_none());
        assert!(info.valid_to.is_none());
        assert!(info.san.is_empty());
    }
}
