package static

import (
	"io/fs"
	"mime"
	"net/http"
	"path"
	"path/filepath"
	"strings"
)

// Handler creates an HTTP handler for serving static files with SPA support.
func Handler() http.Handler {
	// Get the frontend subdirectory
	fsys, err := fs.Sub(Frontend, "frontend")
	if err != nil {
		// Fallback to the root if frontend subdirectory doesn't exist
		fsys = Frontend
	}

	return &staticHandler{fs: fsys}
}

type staticHandler struct {
	fs fs.FS
}

func (h *staticHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	urlPath := strings.TrimPrefix(r.URL.Path, "/")
	if urlPath == "" {
		urlPath = "index.html"
	}

	// Try to serve the exact path first
	if content, err := fs.ReadFile(h.fs, urlPath); err == nil {
		h.serveContent(w, urlPath, content)
		return
	}

	// For non-file paths (no extension), serve index.html (SPA support)
	if !strings.Contains(path.Base(urlPath), ".") {
		if content, err := fs.ReadFile(h.fs, "index.html"); err == nil {
			h.serveContent(w, "index.html", content)
			return
		}
	}

	// Try with .html extension
	htmlPath := urlPath + ".html"
	if content, err := fs.ReadFile(h.fs, htmlPath); err == nil {
		h.serveContent(w, htmlPath, content)
		return
	}

	// Try index.html in directory
	indexPath := path.Join(urlPath, "index.html")
	if content, err := fs.ReadFile(h.fs, indexPath); err == nil {
		h.serveContent(w, indexPath, content)
		return
	}

	// Fallback to index.html for SPA routing
	if content, err := fs.ReadFile(h.fs, "index.html"); err == nil {
		h.serveContent(w, "index.html", content)
		return
	}

	// 404 if nothing found
	http.NotFound(w, r)
}

func (h *staticHandler) serveContent(w http.ResponseWriter, filePath string, content []byte) {
	// Determine MIME type
	ext := filepath.Ext(filePath)
	mimeType := mime.TypeByExtension(ext)
	if mimeType == "" {
		mimeType = "application/octet-stream"
	}

	w.Header().Set("Content-Type", mimeType)
	w.Header().Set("Cache-Control", "public, max-age=31536000, immutable")
	w.WriteHeader(http.StatusOK)
	w.Write(content)
}
