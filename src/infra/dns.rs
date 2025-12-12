//! DNS resolution infrastructure.
//!
//! Provides a trait-based abstraction for DNS resolution, allowing for
//! dependency injection and easier testing.

use hickory_resolver::{config::*, TokioAsyncResolver};
use std::{
    net::IpAddr,
    sync::Arc,
    time::Instant,
};
use tokio::sync::OnceCell;

/// DNS resolution result containing resolved IPs and timing information.
#[derive(Debug)]
pub struct DnsResult {
    /// List of resolved IP addresses.
    pub ips: Vec<IpAddr>,
    /// Time taken for DNS resolution in milliseconds.
    pub duration_ms: u64,
}

/// Trait for DNS resolution.
///
/// This abstraction allows for different DNS resolver implementations
/// and makes testing easier by allowing mock implementations.
#[allow(async_fn_in_trait)]
pub trait DnsResolver: Send + Sync {
    /// Resolves a hostname to a list of IP addresses.
    ///
    /// # Arguments
    ///
    /// * `host` - The hostname to resolve
    ///
    /// # Returns
    ///
    /// A `Result` containing `DnsResult` on success, or an error message on failure.
    async fn resolve(&self, host: &str) -> Result<DnsResult, String>;
}

/// Global DNS resolver instance for connection reuse.
static DNS_RESOLVER: OnceCell<Arc<TokioAsyncResolver>> = OnceCell::const_new();

/// Gets or initializes the global DNS resolver.
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

/// DNS resolver implementation using hickory-resolver (formerly trust-dns).
#[derive(Default)]
pub struct HickoryDnsResolver;

impl HickoryDnsResolver {
    /// Creates a new `HickoryDnsResolver` instance.
    pub fn new() -> Self {
        Self
    }
}

impl DnsResolver for HickoryDnsResolver {
    async fn resolve(&self, host: &str) -> Result<DnsResult, String> {
        let start = Instant::now();

        // Check if already an IP address
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Ok(DnsResult {
                ips: vec![ip],
                duration_ms: 0,
            });
        }

        let resolver = get_resolver().await;
        match resolver.lookup_ip(host).await {
            Ok(response) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let ips: Vec<IpAddr> = response.iter().collect();
                if ips.is_empty() {
                    Err("DNS lookup returned no addresses".to_string())
                } else {
                    Ok(DnsResult { ips, duration_ms })
                }
            }
            Err(e) => Err(format!("DNS lookup failed: {}", e)),
        }
    }
}

/// Convenience function for DNS resolution using the default resolver.
pub async fn resolve_dns(host: &str) -> Result<DnsResult, String> {
    HickoryDnsResolver::new().resolve(host).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_ip_address() {
        let resolver = HickoryDnsResolver::new();
        let result = resolver.resolve("127.0.0.1").await.unwrap();
        assert_eq!(result.ips.len(), 1);
        assert_eq!(result.ips[0].to_string(), "127.0.0.1");
        assert_eq!(result.duration_ms, 0);
    }

    #[tokio::test]
    async fn test_resolve_ipv6_address() {
        let resolver = HickoryDnsResolver::new();
        let result = resolver.resolve("::1").await.unwrap();
        assert_eq!(result.ips.len(), 1);
        assert_eq!(result.ips[0].to_string(), "::1");
    }
}
