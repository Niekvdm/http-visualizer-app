// Package main provides the web server entry point.
// This server ONLY serves static frontend files - no proxy API.
// Proxy requests are handled by the browser extension when using the hosted version.
package main

import (
	"fmt"
	"log"
	"net/http"

	"zone.digit.tommie/internal/config"
	"zone.digit.tommie/internal/static"
)

func main() {
	cfg := config.Load()

	// Serve static files only - no API endpoints
	http.Handle("/", static.Handler())

	addr := fmt.Sprintf(":%d", cfg.Port)
	log.Printf("Project Tommie web server starting on http://localhost%s", addr)
	log.Printf("Note: This server only serves static files. Proxy requests are handled by the browser extension.")

	if err := http.ListenAndServe(addr, nil); err != nil {
		log.Fatalf("Server failed: %v", err)
	}
}
