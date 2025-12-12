use axum::Json;

use crate::proxy::{execute_request, ProxyRequest, ProxyResponse};

pub async fn proxy_request(Json(request): Json<ProxyRequest>) -> Json<ProxyResponse> {
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
