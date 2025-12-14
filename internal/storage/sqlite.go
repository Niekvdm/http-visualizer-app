// Package storage provides SQLite-based persistent storage for the desktop app.
package storage

import (
	"database/sql"
	"fmt"
	"sync"

	_ "github.com/mattn/go-sqlite3"
)

// Database wraps a SQLite connection with thread-safe access.
type Database struct {
	db *sql.DB
	mu sync.Mutex
}

// New creates a new database connection at the specified path.
func New(path string) (*Database, error) {
	db, err := sql.Open("sqlite3", path)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	// Enable WAL mode for better concurrency
	if _, err := db.Exec("PRAGMA journal_mode=WAL;"); err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to enable WAL mode: %w", err)
	}

	// Create storage table if not exists
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS storage (
			store TEXT NOT NULL,
			key TEXT NOT NULL,
			value TEXT NOT NULL,
			updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
			PRIMARY KEY (store, key)
		)
	`)
	if err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to create storage table: %w", err)
	}

	// Create index for faster store lookups
	_, err = db.Exec("CREATE INDEX IF NOT EXISTS idx_storage_store ON storage(store)")
	if err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to create index: %w", err)
	}

	return &Database{db: db}, nil
}

// Close closes the database connection.
func (d *Database) Close() error {
	return d.db.Close()
}

// Get retrieves a value from storage.
func (d *Database) Get(store, key string) (*string, error) {
	d.mu.Lock()
	defer d.mu.Unlock()

	var value string
	err := d.db.QueryRow(
		"SELECT value FROM storage WHERE store = ? AND key = ?",
		store, key,
	).Scan(&value)

	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, fmt.Errorf("query error: %w", err)
	}

	return &value, nil
}

// Set stores a value in storage.
func (d *Database) Set(store, key, value string) error {
	d.mu.Lock()
	defer d.mu.Unlock()

	_, err := d.db.Exec(`
		INSERT INTO storage (store, key, value, updated_at)
		VALUES (?, ?, ?, strftime('%s', 'now'))
		ON CONFLICT(store, key) DO UPDATE SET
			value = excluded.value,
			updated_at = strftime('%s', 'now')
	`, store, key, value)

	if err != nil {
		return fmt.Errorf("insert error: %w", err)
	}

	return nil
}

// Remove deletes a value from storage.
func (d *Database) Remove(store, key string) error {
	d.mu.Lock()
	defer d.mu.Unlock()

	_, err := d.db.Exec(
		"DELETE FROM storage WHERE store = ? AND key = ?",
		store, key,
	)

	if err != nil {
		return fmt.Errorf("delete error: %w", err)
	}

	return nil
}

// Has checks if a key exists in storage.
func (d *Database) Has(store, key string) (bool, error) {
	d.mu.Lock()
	defer d.mu.Unlock()

	var exists int
	err := d.db.QueryRow(
		"SELECT 1 FROM storage WHERE store = ? AND key = ?",
		store, key,
	).Scan(&exists)

	if err == sql.ErrNoRows {
		return false, nil
	}
	if err != nil {
		return false, fmt.Errorf("query error: %w", err)
	}

	return true, nil
}

// Clear removes all values in a store.
func (d *Database) Clear(store string) error {
	d.mu.Lock()
	defer d.mu.Unlock()

	_, err := d.db.Exec("DELETE FROM storage WHERE store = ?", store)
	if err != nil {
		return fmt.Errorf("delete error: %w", err)
	}

	return nil
}

// Keys returns all keys in a store.
func (d *Database) Keys(store string) ([]string, error) {
	d.mu.Lock()
	defer d.mu.Unlock()

	rows, err := d.db.Query("SELECT key FROM storage WHERE store = ?", store)
	if err != nil {
		return nil, fmt.Errorf("query error: %w", err)
	}
	defer rows.Close()

	var keys []string
	for rows.Next() {
		var key string
		if err := rows.Scan(&key); err != nil {
			return nil, fmt.Errorf("scan error: %w", err)
		}
		keys = append(keys, key)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("rows error: %w", err)
	}

	return keys, nil
}
