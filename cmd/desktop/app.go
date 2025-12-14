package main

import (
	"context"
	"os"
	"path/filepath"

	"zone.digit.tommie/internal/proxy"
	"zone.digit.tommie/internal/storage"
)

// App struct holds the application state and provides IPC bindings.
type App struct {
	ctx context.Context
	db  *storage.Database
}

// NewApp creates a new App instance.
func NewApp() *App {
	return &App{}
}

// startup is called when the app starts. It initializes the database.
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx

	// Initialize database in app data directory
	dataDir, err := os.UserConfigDir()
	if err != nil {
		dataDir = "."
	}

	appDir := filepath.Join(dataDir, "tommie")
	if err := os.MkdirAll(appDir, 0755); err != nil {
		panic("failed to create app directory: " + err.Error())
	}

	dbPath := filepath.Join(appDir, "storage.db")
	db, err := storage.New(dbPath)
	if err != nil {
		panic("failed to initialize database: " + err.Error())
	}

	a.db = db
}

// shutdown is called when the app is closing.
func (a *App) shutdown(ctx context.Context) {
	if a.db != nil {
		a.db.Close()
	}
}

// ProxyRequest executes an HTTP request and returns the response.
// This is the main IPC binding for the proxy functionality.
func (a *App) ProxyRequest(request proxy.ProxyRequest) proxy.ProxyResponse {
	return proxy.ExecuteRequest(request)
}

// StorageGet retrieves a value from storage.
func (a *App) StorageGet(store, key string) (*string, error) {
	return a.db.Get(store, key)
}

// StorageSet stores a value in storage.
func (a *App) StorageSet(store, key, value string) error {
	return a.db.Set(store, key, value)
}

// StorageRemove deletes a value from storage.
func (a *App) StorageRemove(store, key string) error {
	return a.db.Remove(store, key)
}

// StorageHas checks if a key exists in storage.
func (a *App) StorageHas(store, key string) (bool, error) {
	return a.db.Has(store, key)
}

// StorageClear removes all values in a store.
func (a *App) StorageClear(store string) error {
	return a.db.Clear(store)
}

// StorageKeys returns all keys in a store.
func (a *App) StorageKeys(store string) ([]string, error) {
	return a.db.Keys(store)
}
