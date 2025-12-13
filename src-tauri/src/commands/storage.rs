use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// Database wrapper for thread-safe access
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create a new database connection at the specified path
    pub fn new(path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&path)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Create storage table if not exists
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS storage (
                store TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                PRIMARY KEY (store, key)
            )
            "#,
            [],
        )?;

        // Create index for faster store lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_storage_store ON storage(store)",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

/// Get a value from storage
#[tauri::command]
pub fn storage_get(
    app: AppHandle,
    store: String,
    key: String,
) -> Result<Option<String>, String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT value FROM storage WHERE store = ?1 AND key = ?2")
        .map_err(|e| format!("Prepare error: {}", e))?;

    let result: Result<String, _> = stmt.query_row([&store, &key], |row| row.get(0));

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Query error: {}", e)),
    }
}

/// Set a value in storage
#[tauri::command]
pub fn storage_set(
    app: AppHandle,
    store: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    conn.execute(
        r#"
        INSERT INTO storage (store, key, value, updated_at)
        VALUES (?1, ?2, ?3, strftime('%s', 'now'))
        ON CONFLICT(store, key) DO UPDATE SET
            value = excluded.value,
            updated_at = strftime('%s', 'now')
        "#,
        [&store, &key, &value],
    )
    .map_err(|e| format!("Insert error: {}", e))?;

    Ok(())
}

/// Remove a value from storage
#[tauri::command]
pub fn storage_remove(app: AppHandle, store: String, key: String) -> Result<(), String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    conn.execute(
        "DELETE FROM storage WHERE store = ?1 AND key = ?2",
        [&store, &key],
    )
    .map_err(|e| format!("Delete error: {}", e))?;

    Ok(())
}

/// Check if a key exists in storage
#[tauri::command]
pub fn storage_has(app: AppHandle, store: String, key: String) -> Result<bool, String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT 1 FROM storage WHERE store = ?1 AND key = ?2")
        .map_err(|e| format!("Prepare error: {}", e))?;

    let exists = stmt.exists([&store, &key]).map_err(|e| format!("Query error: {}", e))?;

    Ok(exists)
}

/// Clear all values in a store
#[tauri::command]
pub fn storage_clear(app: AppHandle, store: String) -> Result<(), String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    conn.execute("DELETE FROM storage WHERE store = ?1", [&store])
        .map_err(|e| format!("Delete error: {}", e))?;

    Ok(())
}

/// Get all keys in a store
#[tauri::command]
pub fn storage_keys(app: AppHandle, store: String) -> Result<Vec<String>, String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT key FROM storage WHERE store = ?1")
        .map_err(|e| format!("Prepare error: {}", e))?;

    let keys: Result<Vec<String>, _> = stmt
        .query_map([&store], |row| row.get(0))
        .map_err(|e| format!("Query error: {}", e))?
        .collect();

    keys.map_err(|e| format!("Collect error: {}", e))
}
