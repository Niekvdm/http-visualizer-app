# HTTP Visualizer Backend & Desktop App

A Rust-based backend server and Tauri desktop application for HTTP Visualizer. Provides CORS-bypassing proxy functionality with optional SQLite storage for the desktop version.

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
│                   Rust Backend (Axum)                    │
│  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │  Static Files   │  │      /api/proxy             │   │
│  │  (rust-embed)   │  │   (HTTP client)             │   │
│  └─────────────────┘  └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Desktop Mode (Tauri)

```
┌─────────────────────────────────────────────────────────┐
│                  Tauri Desktop App                       │
│   ┌─────────────────────────────────────────────────┐   │
│   │              Vue Frontend (WebView)              │   │
│   └───────────────────────┬─────────────────────────┘   │
│                           │ Tauri IPC                    │
│   ┌───────────────────────┴─────────────────────────┐   │
│   │                 Rust Backend                     │   │
│   │  ┌─────────────────┐  ┌─────────────────────┐   │   │
│   │  │  SQLite Storage │  │   HTTP Proxy        │   │   │
│   │  │  (rusqlite)     │  │   (hyper/rustls)    │   │   │
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
- **TLS Support**: HTTPS requests via rustls with certificate info capture

### Desktop-Only Features (Tauri)
- **SQLite Storage**: Persistent storage for collections, settings, and auth configs
- **No Port Conflicts**: Uses Tauri IPC instead of localhost server
- **Native Performance**: Direct Rust execution without HTTP overhead

## Project Structure

```
http-visualizer-app/
├── Cargo.toml                 # Rust dependencies (library)
├── build.rs                   # Build script
├── src/
│   ├── main.rs               # Standalone server entry point
│   ├── lib.rs                # Library exports (proxy logic)
│   ├── config.rs             # Environment configuration
│   ├── error.rs              # Error types
│   ├── routes/               # HTTP routes (standalone server)
│   │   ├── mod.rs            # Routes module
│   │   ├── health.rs         # GET /api/health
│   │   ├── proxy.rs          # POST /api/proxy
│   │   └── static_files.rs   # Embedded static files
│   ├── proxy/                # Core proxy logic (shared)
│   │   ├── mod.rs            # Proxy module
│   │   ├── types.rs          # ProxyRequest/ProxyResponse
│   │   ├── executor.rs       # HTTP request execution
│   │   ├── service.rs        # Proxy service interface
│   │   └── response_builder.rs # Response construction
│   ├── shared/               # Shared utilities
│   │   ├── mod.rs            # Shared module
│   │   ├── status_text.rs    # HTTP status text mapping
│   │   ├── timing.rs         # Timing utilities
│   │   └── cert_parser.rs    # TLS certificate parsing
│   └── infra/                # Infrastructure
│       ├── mod.rs            # Infra module
│       ├── dns.rs            # DNS resolution with timing
│       ├── tls.rs            # TLS configuration
│       └── decompressor.rs   # Response decompression
├── frontend/                  # Vue build output (gitignored)
└── src-tauri/                 # Tauri desktop app
    ├── Cargo.toml            # Tauri dependencies
    ├── build.rs              # Tauri build script
    ├── tauri.conf.json       # Tauri configuration
    ├── capabilities/
    │   └── default.json      # Tauri permissions
    ├── icons/                # App icons
    └── src/
        ├── main.rs           # Tauri entry point
        └── commands/         # IPC commands
            ├── mod.rs        # Commands module
            ├── storage.rs    # SQLite storage commands
            └── proxy.rs      # HTTP proxy command
```

## API Endpoints (Web Mode)

### `GET /api/health`

Health check endpoint.

```json
{
  "status": "ok",
  "version": "0.1.0",
  "backend": "rust-axum"
}
```

### `POST /api/proxy`

Proxy an HTTP request.

**Request:**
```json
{
  "method": "GET",
  "url": "https://api.example.com/data",
  "headers": { "Authorization": "Bearer token" },
  "body": null,
  "timeout": 30000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "status": 200,
    "statusText": "OK",
    "headers": { "content-type": "application/json" },
    "body": "{\"result\": \"data\"}",
    "isBinary": false,
    "size": 18,
    "timing": {
      "total": 245,
      "dns": 12,
      "tcp": 45,
      "tls": 89,
      "ttfb": 156,
      "download": 3
    },
    "url": "https://api.example.com/data",
    "redirected": false,
    "tls": {
      "protocol": "TLSv1.3",
      "cipher": "TLS_AES_256_GCM_SHA384"
    }
  }
}
```

## Tauri IPC Commands (Desktop Mode)

### Storage Commands

| Command | Parameters | Returns | Description |
|---------|------------|---------|-------------|
| `storage_get` | `store`, `key` | `Option<String>` | Get value from SQLite |
| `storage_set` | `store`, `key`, `value` | `()` | Set value in SQLite |
| `storage_remove` | `store`, `key` | `()` | Remove value from SQLite |
| `storage_has` | `store`, `key` | `bool` | Check if key exists |
| `storage_clear` | `store` | `()` | Clear all keys in store |
| `storage_keys` | `store` | `Vec<String>` | Get all keys in store |

Store names: `collections`, `theme`, `auth`, `tokens`, `environment`, `presentation`

### Proxy Command

| Command | Parameters | Returns | Description |
|---------|------------|---------|-------------|
| `proxy_request` | `request: ProxyRequest` | `ProxyResponse` | Execute HTTP request |

## Building

### Prerequisites

- Rust 1.70+ ([rustup](https://rustup.rs/))
- Node.js 18+ and Yarn (for frontend)

### Build Standalone Server

```bash
# Build frontend
cd ../http-visualizer
yarn install
yarn build

# Copy frontend to embedding location
cp -r dist/* ../http-visualizer-app/frontend/

# Build server
cd ../http-visualizer-app
cargo build --release

# Run (available at http://localhost:3000)
./target/release/http-visualizer-app
```

### Build Tauri Desktop App

```bash
# Build frontend
cd http-visualizer
yarn install
yarn build

# Copy frontend
cp -r dist/* ../http-visualizer-app/frontend/

# Build Tauri app
cd ../http-visualizer-app/src-tauri
cargo build --release

# Executable at: target/release/http-visualizer-desktop.exe
```

### Environment Variables (Server Mode)

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Server port |
| `RUST_LOG` | `info` | Log level |

## Docker (Server Mode)

```bash
# Build and run
docker-compose up --build

# Or manually
docker build -t http-visualizer -f Dockerfile ..
docker run -p 3000:3000 http-visualizer
```

## Frontend Integration

The Vue frontend automatically detects the runtime environment:

```typescript
import { isTauri } from '@/services/storage/platform'

if (isTauri()) {
  // Use Tauri IPC for storage and proxy
  await invoke('storage_set', { store: 'theme', key: 'current', value: 'matrix' })
  const response = await invoke('proxy_request', { request })
} else {
  // Use localStorage and fetch API
  localStorage.setItem('theme', 'matrix')
  const response = await fetch('/api/proxy', { method: 'POST', body: JSON.stringify(request) })
}
```

### Platform Detection

```typescript
// src/services/storage/platform.ts
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}
```

### Storage Abstraction

```typescript
// Automatically uses SQLite (Tauri) or localStorage (browser)
import { getStorage } from '@/services/storage'

const storage = getStorage('collections')
await storage.set('my-collection', collectionData)
const data = await storage.get('my-collection')
```

### API Client

```typescript
// Automatically uses IPC (Tauri) or fetch (browser)
import { proxyRequest } from '@/services/api'

const response = await proxyRequest({
  method: 'GET',
  url: 'https://api.example.com/data',
  headers: { 'Accept': 'application/json' },
  timeout: 30000
})
```

## SQLite Schema (Tauri)

```sql
CREATE TABLE storage (
    store TEXT NOT NULL,      -- 'collections', 'theme', 'auth', etc.
    key TEXT NOT NULL,        -- storage key
    value TEXT NOT NULL,      -- JSON-serialized value
    updated_at INTEGER,       -- Unix timestamp
    PRIMARY KEY (store, key)
);
```

Database location: `%APPDATA%/com.http-visualizer.app/http-visualizer.db` (Windows)

## Deployment Scenarios

| Scenario | Storage | Proxy | Notes |
|----------|---------|-------|-------|
| Browser + Extension | localStorage | Extension | Best for casual use |
| Browser + Server | localStorage | /api/proxy | Docker/server deployment |
| Tauri Desktop | SQLite | IPC | Best for power users |
| Browser + Both | localStorage | Extension (priority) | Fallback to server |

## Dependencies

### Core (Shared)
| Crate | Purpose |
|-------|---------|
| `hyper` | Low-level HTTP client |
| `tokio-rustls` | TLS with certificate access |
| `hickory-resolver` | DNS resolution with timing |
| `serde` | Serialization |

### Server Mode
| Crate | Purpose |
|-------|---------|
| `axum` | Web framework |
| `rust-embed` | Static file embedding |
| `tower-http` | CORS middleware |

### Tauri Mode
| Crate | Purpose |
|-------|---------|
| `tauri` | Desktop framework |
| `rusqlite` | SQLite database |

## License

MIT
