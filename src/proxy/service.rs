//! Proxy service abstraction layer.
//!
//! Provides a trait-based abstraction for proxy request execution,
//! enabling dependency injection and easier testing.

use super::executor::execute_request;
use super::types::{ProxyRequest, ProxyResponse};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Trait for proxy services that execute HTTP requests.
///
/// This abstraction allows for different proxy implementations
/// and makes testing easier by allowing mock implementations.
pub trait ProxyService: Send + Sync {
    /// Executes a proxy request and returns the response.
    ///
    /// # Arguments
    ///
    /// * `request` - The proxy request to execute
    ///
    /// # Returns
    ///
    /// A future that resolves to a `ProxyResponse`.
    fn execute(
        &self,
        request: ProxyRequest,
    ) -> Pin<Box<dyn Future<Output = ProxyResponse> + Send + '_>>;
}

/// Default HTTP proxy service implementation.
///
/// Uses the standard `execute_request` function for request execution.
#[derive(Default, Clone)]
pub struct HttpProxyService;

impl HttpProxyService {
    /// Creates a new `HttpProxyService` instance.
    pub fn new() -> Self {
        Self
    }

    /// Creates a new `HttpProxyService` wrapped in an `Arc`.
    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl ProxyService for HttpProxyService {
    fn execute(
        &self,
        request: ProxyRequest,
    ) -> Pin<Box<dyn Future<Output = ProxyResponse> + Send + '_>> {
        Box::pin(async move { execute_request(request).await })
    }
}

/// Extension trait for `ProxyService` that provides convenience methods.
pub trait ProxyServiceExt: ProxyService {
    /// Executes a GET request to the specified URL.
    fn get(&self, url: &str) -> Pin<Box<dyn Future<Output = ProxyResponse> + Send + '_>> {
        let request = ProxyRequest {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: Default::default(),
            body: None,
            timeout: None,
        };
        self.execute(request)
    }

    /// Executes a POST request to the specified URL with the given body.
    fn post(
        &self,
        url: &str,
        body: Option<String>,
    ) -> Pin<Box<dyn Future<Output = ProxyResponse> + Send + '_>> {
        let request = ProxyRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: Default::default(),
            body,
            timeout: None,
        };
        self.execute(request)
    }
}

// Implement ProxyServiceExt for all types that implement ProxyService
impl<T: ProxyService + ?Sized> ProxyServiceExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockProxyService {
        response: ProxyResponse,
    }

    impl ProxyService for MockProxyService {
        fn execute(
            &self,
            _request: ProxyRequest,
        ) -> Pin<Box<dyn Future<Output = ProxyResponse> + Send + '_>> {
            let response = self.response.clone();
            Box::pin(async move { response })
        }
    }

    impl Clone for ProxyResponse {
        fn clone(&self) -> Self {
            // For testing purposes only
            if self.success {
                ProxyResponse::success(self.data.as_ref().unwrap().clone())
            } else {
                ProxyResponse::error(
                    self.error.as_ref().unwrap().message.clone(),
                    self.error.as_ref().unwrap().code.clone(),
                )
            }
        }
    }

    impl Clone for super::super::types::ResponseData {
        fn clone(&self) -> Self {
            Self {
                status: self.status,
                status_text: self.status_text.clone(),
                headers: self.headers.clone(),
                request_headers: self.request_headers.clone(),
                body: self.body.clone(),
                body_base64: self.body_base64.clone(),
                is_binary: self.is_binary,
                size: self.size,
                timing: super::super::types::TimingInfo {
                    total: self.timing.total,
                    dns: self.timing.dns,
                    tcp: self.timing.tcp,
                    tls: self.timing.tls,
                    ttfb: self.timing.ttfb,
                    download: self.timing.download,
                    blocked: self.timing.blocked,
                },
                url: self.url.clone(),
                redirected: self.redirected,
                redirect_chain: self.redirect_chain.clone(),
                tls: self.tls.clone(),
                size_breakdown: self.size_breakdown.clone(),
                server_ip: self.server_ip.clone(),
                protocol: self.protocol.clone(),
                from_cache: self.from_cache,
                resource_type: self.resource_type.clone(),
                request_body_size: self.request_body_size,
                connection: self.connection.clone(),
                server_software: self.server_software.clone(),
            }
        }
    }

    #[tokio::test]
    async fn test_mock_proxy_service() {
        let mock_response = ProxyResponse::error("Test error".to_string(), "TEST".to_string());
        let service = MockProxyService {
            response: mock_response,
        };

        let request = ProxyRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout: None,
        };

        let response = service.execute(request).await;
        assert!(!response.success);
        assert_eq!(response.error.unwrap().code, "TEST");
    }
}
