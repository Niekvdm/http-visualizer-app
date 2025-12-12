//! TLS/SSL infrastructure.
//!
//! Provides trait-based abstractions for TLS configuration and connection handling.

use rustls::pki_types::ServerName;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsConnector};

/// Trait for TLS configuration providers.
///
/// This abstraction allows for different TLS configurations
/// and makes testing easier by allowing mock implementations.
pub trait TlsProvider: Send + Sync {
    /// Creates a new TLS client configuration.
    fn client_config(&self) -> Arc<rustls::ClientConfig>;

    /// Creates a TLS connector from this provider's configuration.
    fn connector(&self) -> TlsConnector {
        TlsConnector::from(self.client_config())
    }
}

/// Default TLS provider using rustls with system root certificates.
#[derive(Default)]
pub struct RustlsTlsProvider;

impl RustlsTlsProvider {
    /// Creates a new `RustlsTlsProvider` instance.
    pub fn new() -> Self {
        Self
    }
}

impl TlsProvider for RustlsTlsProvider {
    fn client_config(&self) -> Arc<rustls::ClientConfig> {
        create_tls_config()
    }
}

/// Creates a TLS client configuration with Mozilla's root certificates.
///
/// This configuration:
/// - Uses webpki-roots for trusted root certificates
/// - Does not use client authentication
/// - Supports TLS 1.2 and TLS 1.3
pub fn create_tls_config() -> Arc<rustls::ClientConfig> {
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Arc::new(config)
}

/// Establishes a TLS connection over an existing TCP stream.
///
/// # Arguments
///
/// * `provider` - The TLS provider to use for configuration
/// * `tcp_stream` - The established TCP connection
/// * `server_name` - The server name for SNI
///
/// # Returns
///
/// A `Result` containing the TLS stream on success, or an error on failure.
pub async fn connect_tls<P: TlsProvider>(
    provider: &P,
    tcp_stream: TcpStream,
    server_name: &str,
) -> Result<TlsStream<TcpStream>, String> {
    let connector = provider.connector();

    let server_name = ServerName::try_from(server_name.to_string())
        .map_err(|e| format!("Invalid server name: {}", e))?;

    connector
        .connect(server_name, tcp_stream)
        .await
        .map_err(|e| format!("TLS handshake failed: {}", e))
}

// Note: TLS tests are skipped because they require a crypto provider to be installed,
// which happens at runtime in the actual application but not in unit tests.
// The TLS functionality is tested through integration tests instead.
