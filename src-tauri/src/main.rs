// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::TcpListener;
use tauri::Manager;

/// Find an available port
fn find_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to port")
        .local_addr()
        .expect("Failed to get local address")
        .port()
}

fn main() {
    // Find an available port for the backend server
    let port = find_available_port();

    // Set the port environment variable
    std::env::set_var("PORT", port.to_string());

    // Start the backend server in a separate thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            // Import and run the server from the parent crate
            // Note: This requires exposing a run function from the parent crate
            println!("Starting HTTP Visualizer backend on port {}", port);

            // For now, we'll use the embedded server approach
            // The server runs on the found port and serves both API and static files
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

            use axum::{routing::get, Router};
            use tower_http::cors::{Any, CorsLayer};

            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);

            let app = Router::new()
                .route("/api/health", get(|| async {
                    axum::Json(serde_json::json!({
                        "status": "ok",
                        "version": env!("CARGO_PKG_VERSION"),
                        "backend": "tauri-embedded"
                    }))
                }))
                .route("/api/proxy", axum::routing::post(proxy_handler))
                .layer(cors);

            axum::serve(listener, app).await.unwrap();
        });
    });

    // Give the server a moment to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Build and run the Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            // Navigate to the backend server URL
            let url = format!("http://127.0.0.1:{}", port);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval(&format!("window.location.href = '{}'", url));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Proxy handler for the embedded server
async fn proxy_handler(
    axum::Json(request): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    use reqwest::Method;
    use std::str::FromStr;
    use std::time::Duration;

    let method = request["method"].as_str().unwrap_or("GET");
    let url = request["url"].as_str().unwrap_or("");
    let timeout = request["timeout"].as_u64().unwrap_or(30000);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return axum::Json(serde_json::json!({
                "success": false,
                "error": {
                    "message": e.to_string(),
                    "code": "CLIENT_ERROR"
                }
            }));
        }
    };

    let method = Method::from_str(method).unwrap_or(Method::GET);
    let mut req_builder = client.request(method, url);

    // Add headers
    if let Some(headers) = request["headers"].as_object() {
        for (key, value) in headers {
            if let Some(v) = value.as_str() {
                req_builder = req_builder.header(key, v);
            }
        }
    }

    // Add body
    if let Some(body) = request["body"].as_str() {
        req_builder = req_builder.body(body.to_string());
    }

    let start = std::time::Instant::now();

    match req_builder.send().await {
        Ok(response) => {
            let duration = start.elapsed().as_millis() as u64;
            let status = response.status().as_u16();
            let headers: std::collections::HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let final_url = response.url().to_string();

            match response.text().await {
                Ok(body) => {
                    axum::Json(serde_json::json!({
                        "success": true,
                        "data": {
                            "status": status,
                            "statusText": status_text(status),
                            "headers": headers,
                            "body": body,
                            "isBinary": false,
                            "size": body.len(),
                            "timing": { "total": duration },
                            "url": final_url,
                            "redirected": false
                        }
                    }))
                }
                Err(e) => {
                    axum::Json(serde_json::json!({
                        "success": false,
                        "error": {
                            "message": e.to_string(),
                            "code": "BODY_READ_ERROR"
                        }
                    }))
                }
            }
        }
        Err(e) => {
            let code = if e.is_timeout() {
                "TIMEOUT"
            } else if e.is_connect() {
                "CONNECTION_FAILED"
            } else {
                "REQUEST_FAILED"
            };
            axum::Json(serde_json::json!({
                "success": false,
                "error": {
                    "message": e.to_string(),
                    "code": code
                }
            }))
        }
    }
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}
