// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{
    proxy_request, storage_clear, storage_get, storage_has, storage_keys, storage_remove,
    storage_set, Database,
};
use tauri::Manager;

/// Initialize the rustls crypto provider for TLS operations in proxy requests.
fn init_crypto_provider() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
}

fn main() {
    // Install rustls crypto provider before any TLS operations
    init_crypto_provider();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Get the app data directory for SQLite database
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            // Ensure the directory exists
            std::fs::create_dir_all(&app_data_dir)
                .expect("Failed to create app data directory");

            // Initialize SQLite database
            let db_path = app_data_dir.join("http-visualizer.db");
            println!("Initializing database at: {:?}", db_path);

            let database = Database::new(db_path)
                .expect("Failed to initialize database");

            // Register database as managed state
            app.manage(database);

            println!("HTTP Visualizer desktop app started (IPC mode)");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Storage commands
            storage_get,
            storage_set,
            storage_remove,
            storage_has,
            storage_clear,
            storage_keys,
            // Proxy command
            proxy_request,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
