use http_visualizer_app::{execute_request, ProxyRequest, ProxyResponse};

/// Execute an HTTP proxy request
/// Reuses the proxy logic from the parent crate
#[tauri::command]
pub async fn proxy_request(request: ProxyRequest) -> ProxyResponse {
    // execute_request is the core function from http-visualizer-app
    // It handles all the HTTP request logic, timing, TLS info, etc.
    // Errors are returned as ProxyResponse with success: false
    execute_request(request).await
}
