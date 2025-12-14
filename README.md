# Project Tommie - Backend & Desktop App (Go)

A Go-based backend server and Wails desktop application for Project Tommie. Provides CORS-bypassing proxy functionality with optional SQLite storage for the desktop version.

## Architecture

The project supports two deployment modes:

### Web Mode (Standalone Server)

```
┌─────────────────────────────────────────────────────────┐
│                     Browser                              │
│   ┌─────────────────────────────────────────────────┐   │
│   │            Vue Frontend (localStorage)           │   │
│   └─────────────────────────┬───────────────────────┘   │
└─────────────────────────────┼───────────────────────────┘
                              │ fetch(/api/proxy)
                              ▼
┌─────────────────────────────────────────────────────────┐
│                   Go Backend (net/http)                  │
│  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │  Static Files   │  │      /api/proxy             │   │
│  │  (embed.FS)     │  │   (HTTP client)             │   │
│  └─────────────────┘  └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Desktop Mode (Wails)

```
┌─────────────────────────────────────────────────────────┐
│                  Wails Desktop App                       │
│   ┌─────────────────────────────────────────────────┐   │
│   │              Vue Frontend (WebView)              │   │
│   └───────────────────────┬─────────────────────────┘   │
│                           │ Wails IPC                    │
│   ┌───────────────────────┴─────────────────────────┐   │
│   │                 Go Backend                       │   │
│   │  ┌─────────────────┐  ┌─────────────────────┐   │   │
│   │  │  SQLite Storage │  │   HTTP Proxy        │   │   │
│   │  │  (go-sqlite3)   │  │   (net/http)        │   │   │
│   │  └─────────────────┘  └─────────────────────┘   │   │
│   └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Features

### Core Features
- **CORS Bypass Proxy**: Execute HTTP requests to any URL without browser CORS restrictions
- **Static File Serving**: Embedded Vue frontend for single-binary distribution
- **Redirect Tracking**: Full redirect chain capture with timing per hop
- **Binary Response Support**: Base64 encoding for non-text responses
- **Detailed Timing**: DNS, TCP, TLS, TTFB, and download timing metrics
- **TLS Support**: HTTPS requests with certificate info capture
- **Compression**: Automatic decompression of gzip, deflate, and brotli responses

### Desktop-Only Features (Wails)
- **SQLite Storage**: Persistent storage for collections, settings, and auth configs
- **No Port Conflicts**: Uses Wails IPC instead of localhost server
- **Native Performance**: Direct Go execution without HTTP overhead

## Project Structure

```
tommie-app/
├── go.mod                     # Go module (zone.digit.tommie)
├── main.go                    # Wails desktop entry point
├── app.go                     # Desktop app bindings
├── frontend/                  # Vue build output (embedded)
├── cmd/
│   ├── server/
│   │   └── main.go           # Standalone server entry point
│   └── desktop/
│       ├── main.go           # Alternative desktop entry
│       └── app.go            # App bindings
├── internal/
│   ├── config/
│   │   └── config.go         # Environment configuration
│   ├── proxy/
│   │   ├── types.go          # ProxyRequest/ProxyResponse
│   │   ├── executor.go       # HTTP request execution
│   │   ├── response_builder.go # Response construction
│   │   └── timing.go         # Timing utilities
│   ├── storage/
│   │   └── sqlite.go         # SQLite database wrapper
│   ├── static/
│   │   ├── embed.go          # Embedded static files
│   │   └── static.go         # Static file handler
│   └── infra/
│       ├── dns.go            # DNS resolution with timing
│       ├── tls.go            # TLS certificate parsing
│       └── decompressor.go   # Response decompression
└── pkg/
    └── statustext/
        └── statustext.go     # HTTP status text mapping
```

## Wails IPC Bindings (Desktop Mode)

### Storage Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `StorageGet` | `store`, `key` | `*string, error` | Get value from SQLite |
| `StorageSet` | `store`, `key`, `value` | `error` | Set value in SQLite |
| `StorageRemove` | `store`, `key` | `error` | Remove value from SQLite |
| `StorageHas` | `store`, `key` | `bool, error` | Check if key exists |
| `StorageClear` | `store` | `error` | Clear all keys in store |
| `StorageKeys` | `store` | `[]string, error` | Get all keys in store |

Store names: `collections`, `theme`, `auth`, `tokens`, `environment`, `presentation`

### Proxy Method

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `ProxyRequest` | `request: ProxyRequest` | `ProxyResponse` | Execute HTTP request |

## Prerequisites

- Go 1.23+
- Node.js 18+ and Yarn (for frontend)
- Wails CLI v2 (for desktop builds)
- GCC (required for SQLite - CGO)

### Installing Wails CLI

```bash
go install github.com/wailsapp/wails/v2/cmd/wails@latest
```

### Windows: Installing GCC

SQLite requires CGO. Install a GCC compiler:

```bash
# Option 1: Using Scoop
scoop install mingw

# Option 2: Using Chocolatey
choco install mingw

# Option 3: Download from https://www.mingw-w64.org/
```

Verify installation:
```bash
gcc --version
```

## Building

### Prerequisites

```bash
# Install Wails CLI
make install-wails

# Install Go dependencies
make deps
```

### Build Frontend

First, build the Vue frontend in the `http-visualizer` project:

```bash
cd ../http-visualizer
yarn install
yarn build
cd ../http-visualizer-app
```

Then copy the frontend files:

```bash
make frontend
```

### Build Standalone Server

```bash
make server

# Run (available at http://localhost:3000)
make run-server
```

### Build Wails Desktop App

```bash
make desktop

# Executable at: build/bin/tommie.exe
```

### Build Everything

```bash
make all
```

### Development Mode

```bash
make dev
```

### Clean Build Artifacts

```bash
make clean
```

## Environment Variables (Server Mode)

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Server port |

## SQLite Schema

```sql
CREATE TABLE storage (
    store TEXT NOT NULL,      -- 'collections', 'theme', 'auth', etc.
    key TEXT NOT NULL,        -- storage key
    value TEXT NOT NULL,      -- JSON-serialized value
    updated_at INTEGER,       -- Unix timestamp
    PRIMARY KEY (store, key)
);
```

Database location: `%APPDATA%/tommie/storage.db` (Windows)

## Frontend Integration

The Vue frontend automatically detects the runtime environment:

```typescript
import { isWails } from '@/services/storage/platform'

if (isWails()) {
  // Use Wails IPC for storage and proxy
  await window.go.main.App.StorageSet('theme', 'current', 'matrix')
  const response = await window.go.main.App.ProxyRequest(request)
} else {
  // Use localStorage and fetch API
  localStorage.setItem('theme', 'matrix')
  const response = await fetch('/api/proxy', { method: 'POST', body: JSON.stringify(request) })
}
```

## Deployment Scenarios

| Scenario | Storage | Proxy | Notes |
|----------|---------|-------|-------|
| Browser + Extension | localStorage | Extension | Best for casual use |
| Browser + Server | localStorage | /api/proxy | Docker/server deployment |
| Wails Desktop | SQLite | IPC | Best for power users |

## Docker

### Build Docker Image

```bash
# Build frontend first
cd ../http-visualizer
yarn build
cd ../http-visualizer-app

# Copy frontend and build image
make docker-build

# Or manually
make frontend
docker build -t ghcr.io/niekvdm/tommie:latest .
```

### Run Docker Container

```bash
docker run -p 3000:3000 ghcr.io/niekvdm/tommie:latest
```

### Push to Registry

```bash
make docker-push

# Or with custom tag
make docker-push DOCKER_TAG=v1.0.0
```

## Kubernetes Deployment

The `k8s/` folder contains Kubernetes manifests for deployment:

```
k8s/
├── namespace.yaml          # dzone-dev namespace
├── service.yaml            # Service + Deployment
├── ingress.yaml            # Ingress with TLS
├── kustomization.yaml      # Kustomize configuration
└── argocd-application.yaml # ArgoCD Application
```

### Deploy with Kustomize

```bash
kubectl apply -k k8s/
```

### Deploy with ArgoCD

```bash
kubectl apply -f k8s/argocd-application.yaml
```

The ArgoCD application is configured with:
- Automatic sync and self-healing
- Image auto-update via ArgoCD Image Updater
- TLS termination at ingress

### Environment

- **Namespace**: `dzone-dev`
- **Host**: `tommie.digit.zone`
- **Port**: 3000 (internal), 80 (service)

## Dependencies

| Package | Purpose |
|---------|---------|
| `github.com/wailsapp/wails/v2` | Desktop framework |
| `github.com/mattn/go-sqlite3` | SQLite database |
| `github.com/andybalholm/brotli` | Brotli decompression |

## License

MIT
