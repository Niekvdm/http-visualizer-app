use axum::Json;
use std::env;

use crate::proxy::{execute_request, ProxyRequest, ProxyResponse};

/// Check if proxy is disabled via environment variable
fn is_proxy_disabled() -> bool {
    env::var("DISABLE_PROXY")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

pub async fn proxy_request(Json(request): Json<ProxyRequest>) -> Json<ProxyResponse> {
    // Check if proxy is disabled
    if is_proxy_disabled() {
        tracing::debug!("Proxy disabled, rejecting request");
        return Json(ProxyResponse::error(
            "Proxy is disabled. Please use the browser extension.".to_string(),
            "PROXY_DISABLED".to_string(),
        ));
    }

    tracing::debug!(
        method = %request.method,
        url = %request.url,
        "Proxying request"
    );

    let response = execute_request(request).await;

    if response.success {
        tracing::debug!("Request succeeded");
    } else if let Some(ref error) = response.error {
        tracing::warn!(code = %error.code, message = %error.message, "Request failed");
    }

    Json(response)
}
