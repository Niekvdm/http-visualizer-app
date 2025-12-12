pub mod config;
pub mod error;
pub mod infra;
pub mod proxy;
pub mod routes;
pub mod shared;

use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};

pub use config::Config;
pub use infra::{decompress_body, DnsResolver, HickoryDnsResolver, TlsProvider};
pub use proxy::{execute_request, HttpProxyService, ProxyRequest, ProxyResponse, ProxyService};
pub use shared::{status_text, CapturedCertInfo, DetailedTiming};

/// Builder for creating configured Axum routers.
///
/// Provides a fluent API for building the HTTP Visualizer application router
/// with optional features like CORS, static file serving, and tracing.
///
/// # Example
///
/// ```ignore
/// use http_visualizer_app::AppBuilder;
///
/// let app = AppBuilder::new()
///     .with_cors()
///     .with_static_files()
///     .build();
/// ```
pub struct AppBuilder {
    cors: bool,
    static_files: bool,
    backend_name: &'static str,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    /// Creates a new `AppBuilder` with default settings.
    pub fn new() -> Self {
        Self {
            cors: false,
            static_files: false,
            backend_name: "rust-axum",
        }
    }

    /// Enables CORS middleware with permissive settings.
    pub fn with_cors(mut self) -> Self {
        self.cors = true;
        self
    }

    /// Enables static file serving from embedded frontend assets.
    pub fn with_static_files(mut self) -> Self {
        self.static_files = true;
        self
    }

    /// Sets the backend name reported in health checks.
    pub fn with_backend_name(mut self, name: &'static str) -> Self {
        self.backend_name = name;
        self
    }

    /// Builds the configured Axum Router.
    pub fn build(self) -> Router {
        let mut app = Router::new()
            .route("/api/health", get(routes::health::health_check))
            .route("/api/proxy", axum::routing::post(routes::proxy::proxy_request));

        if self.static_files {
            app = app.fallback(routes::static_files::serve_static);
        }

        if self.cors {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);
            app = app.layer(cors);
        }

        app
    }

    /// Builds the router with all features enabled.
    ///
    /// This is equivalent to calling:
    /// ```ignore
    /// AppBuilder::new()
    ///     .with_cors()
    ///     .with_static_files()
    ///     .build()
    /// ```
    pub fn build_full() -> Router {
        Self::new()
            .with_cors()
            .with_static_files()
            .build()
    }

    /// Builds a minimal API router without static files.
    ///
    /// Useful for Tauri or other embedded scenarios where
    /// the frontend is served separately.
    pub fn build_api_only() -> Router {
        Self::new()
            .with_cors()
            .build()
    }
}
