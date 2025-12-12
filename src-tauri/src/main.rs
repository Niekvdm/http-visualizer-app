// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use http_visualizer_app::AppBuilder;
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
            println!("Starting HTTP Visualizer backend on port {}", port);

            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

            // Use the shared AppBuilder for API-only router (no static files needed for Tauri)
            let app = AppBuilder::build_api_only();

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
