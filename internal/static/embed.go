// Package static provides embedded frontend assets and static file serving.
package static

import "embed"

// Frontend contains the embedded frontend files.
// The frontend directory should contain the built frontend assets.
//
//go:embed frontend/*
var Frontend embed.FS
