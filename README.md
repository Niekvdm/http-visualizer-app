# HTTP Visualizer Backend

A Rust-based backend server for HTTP Visualizer that provides CORS-bypassing proxy functionality and serves the Vue frontend as a single deployable binary.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Client (Browser)                     │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Rust Backend (Axum)                    │
│  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │  Static Files   │  │      /api/proxy             │   │
│  │  (rust-embed)   │  │   (reqwest HTTP client)     │   │
│  └─────────────────┘  └─────────────────────────────┘   │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    External APIs                         │
│         (No CORS restrictions from server)               │
└─────────────────────────────────────────────────────────┘
```

## Features

- **CORS Bypass Proxy**: Execute HTTP requests to any URL without browser CORS restrictions
- **Static File Serving**: Embedded Vue frontend for single-binary distribution
- **Redirect Tracking**: Full redirect chain capture with timing per hop
- **Binary Response Support**: Base64 encoding for non-text responses
- **Detailed Timing**: Request duration metrics
- **TLS Support**: HTTPS requests via rustls

## API Endpoints

### `GET /api/health`

Health check endpoint for detecting backend availability.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "backend": "rust-axum"
}
```

### `POST /api/proxy`

Proxy an HTTP request to bypass CORS restrictions.

**Request:**
```json
{
  "method": "GET",
  "url": "https://api.example.com/data",
  "headers": {
    "Authorization": "Bearer token",
    "Accept": "application/json"
  },
  "body": null,
  "timeout": 30000
}
```

**Response (Success):**
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
    "timing": { "total": 245 },
    "url": "https://api.example.com/data",
    "redirected": false
  }
}
```

**Response (Error):**
```json
{
  "success": false,
  "error": {
    "message": "Connection refused",
    "code": "CONNECTION_FAILED"
  }
}
```

## Building

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Node.js 18+ and Yarn (for frontend build)

### Development

```bash
# Build frontend first
cd ../http-visualizer
yarn install
yarn build

# Copy frontend assets
cp -r dist/* ../http-visualizer-app/frontend/

# Run backend in development mode
cd ../http-visualizer-app
cargo run
```

The server starts on `http://localhost:3000` by default.

### Production Build

```bash
# Build optimized release binary
cargo build --release

# Binary location: target/release/http-visualizer-app
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Server port |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `FRONTEND_PATH` | embedded | Path to frontend assets (optional) |

## Docker

### Build and Run

```bash
# Build from project root (contains both http-visualizer and http-visualizer-app)
docker-compose up --build

# Or build manually
docker build -t http-visualizer -f Dockerfile ..
docker run -p 3000:3000 http-visualizer
```

### Docker Compose

```yaml
version: '3.8'
services:
  http-visualizer:
    build:
      context: ..
      dockerfile: http-visualizer-app/Dockerfile
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=info
```

## Tauri Desktop App

The `src-tauri/` directory contains configuration for building a desktop application using Tauri.

### Prerequisites

- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)
- Tauri CLI: `cargo install tauri-cli`

### Build Desktop App

```bash
cd src-tauri

# Development
cargo tauri dev

# Production build
cargo tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Project Structure

```
http-visualizer-app/
├── Cargo.toml                 # Rust dependencies
├── build.rs                   # Build script (creates placeholder frontend)
├── Dockerfile                 # Multi-stage Docker build
├── docker-compose.yml         # Container orchestration
├── src/
│   ├── main.rs               # Entry point, server setup
│   ├── lib.rs                # Library exports
│   ├── config.rs             # Environment configuration
│   ├── error.rs              # Error types and handling
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── health.rs         # GET /api/health
│   │   ├── proxy.rs          # POST /api/proxy handler
│   │   └── static_files.rs   # Embedded static file serving
│   └── proxy/
│       ├── mod.rs
│       ├── types.rs          # Request/Response types
│       └── executor.rs       # HTTP request execution
├── frontend/                  # Vue build output (gitignored)
└── src-tauri/                 # Tauri desktop configuration
    ├── Cargo.toml
    ├── tauri.conf.json
    └── src/main.rs
```

## Frontend Integration

The Vue frontend seamlessly integrates with the Rust backend through the `useExtensionBridge` composable. The same code path that communicates with the browser extension also supports the proxy backend as a fallback.

### Detection Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        App Initialization                        │
└─────────────────────────────┬───────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│ checkExtensionAvailability() │     │ checkProxyBackendAvailability() │
│                         │     │                         │
│  Sends postMessage to   │     │  GET /api/health        │
│  browser extension      │     │                         │
└───────────┬─────────────┘     └───────────┬─────────────┘
            │                               │
            ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│ isExtensionAvailable    │     │ isProxyBackendAvailable │
│ = true/false            │     │ = true/false            │
└─────────────────────────┘     └─────────────────────────┘
```

Both checks run **in parallel** on startup. The first available method is used for requests.

### Request Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│              executeRequestViaExtension(options)                 │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │ Extension available? │
                    └──────────┬──────────┘
                               │
                 ┌─────────────┴─────────────┐
                 │ YES                       │ NO
                 ▼                           ▼
    ┌────────────────────┐      ┌─────────────────────┐
    │ executeViaExtension │      │ Proxy backend       │
    │                    │      │ available?          │
    │ postMessage to     │      └──────────┬──────────┘
    │ content script     │                 │
    └────────────────────┘       ┌─────────┴─────────┐
                                 │ YES               │ NO
                                 ▼                   ▼
                    ┌────────────────────┐  ┌───────────────┐
                    │ executeViaProxy    │  │ Throw Error   │
                    │                    │  │ "Neither      │
                    │ POST /api/proxy    │  │ available"    │
                    └────────────────────┘  └───────────────┘
```

### Modified File

**`http-visualizer/src/composables/useExtensionBridge.ts`**

#### Added State

```typescript
// Proxy backend detection state
let isProxyBackendAvailable = ref(false)
let proxyBackendVersion = ref<string | null>(null)
```

#### Added Functions

| Function | Description |
|----------|-------------|
| `checkProxyBackendAvailability()` | Fetches `GET /api/health` to detect if backend is running |
| `executeViaProxy(options)` | Sends request to `POST /api/proxy` |
| `isAnyBridgeAvailable()` | Returns `true` if extension OR proxy is available |
| `getCurrentBridgeType()` | Returns `'extension'`, `'proxy'`, or `null` |

#### Proxy Backend Detection

```typescript
async function checkProxyBackendAvailability(): Promise<boolean> {
  try {
    const response = await fetch('/api/health', {
      method: 'GET',
      headers: { Accept: 'application/json' },
    })
    if (response.ok) {
      const data = await response.json()
      isProxyBackendAvailable.value = true
      proxyBackendVersion.value = data.version || '1.0.0'
      return true
    }
  } catch {
    // Backend not available
  }
  isProxyBackendAvailable.value = false
  return false
}
```

#### Proxy Request Execution

```typescript
async function executeViaProxy(options: {
  method: string
  url: string
  headers?: Record<string, string>
  body?: string
  timeout?: number
}): Promise<ExtensionResponse> {
  const response = await fetch('/api/proxy', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      method: options.method,
      url: options.url,
      headers: options.headers || {},
      body: options.body,
      timeout: options.timeout || 30000,
    }),
  })
  return response.json()
}
```

#### Composable Exports

```typescript
export function useExtensionBridge() {
  return {
    // State
    isExtensionAvailable: readonly(isExtensionAvailable),
    extensionVersion: readonly(extensionVersion),
    isProxyBackendAvailable: readonly(isProxyBackendAvailable),    // NEW
    proxyBackendVersion: readonly(proxyBackendVersion),            // NEW

    // Methods
    checkExtensionAvailability,
    checkProxyBackendAvailability,  // NEW
    executeViaExtension,
    executeViaProxy,                // NEW
  }
}

// Utility exports
export function isAnyBridgeAvailable(): boolean              // NEW
export function getCurrentBridgeType(): 'extension' | 'proxy' | null  // NEW
```

### Response Format Compatibility

The Rust backend returns responses in the **exact same format** as the browser extension, ensuring seamless compatibility:

```typescript
interface ExtensionResponse {
  success: boolean
  data?: {
    status: number
    statusText: string
    headers: Record<string, string>
    requestHeaders?: Record<string, string>
    body: string
    bodyBase64?: string | null
    isBinary: boolean
    size: number
    timing: {
      total: number
      dns?: number
      tcp?: number
      tls?: number
      ttfb?: number
      download?: number
    }
    url: string
    redirected: boolean
    redirectChain?: RedirectHop[]
    tls?: TlsInfo
    sizeBreakdown?: SizeBreakdown
    serverIP?: string
    protocol?: string
    fromCache?: boolean
    connection?: string
    serverSoftware?: string
  }
  error?: {
    message: string
    code: string
    name?: string
  }
}
```

### Usage Example

```typescript
// In a Vue component
import { useExtensionBridge } from '@/composables/useExtensionBridge'

const {
  isExtensionAvailable,
  isProxyBackendAvailable,
  executeViaExtension,
  executeViaProxy
} = useExtensionBridge()

// Automatic fallback (recommended)
const response = await executeRequestViaExtension({
  method: 'GET',
  url: 'https://api.example.com/data',
  headers: { 'Accept': 'application/json' }
})

// Or explicitly use proxy
if (isProxyBackendAvailable.value) {
  const response = await executeViaProxy({
    method: 'POST',
    url: 'https://api.example.com/submit',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ data: 'value' })
  })
}
```

### Deployment Scenarios

| Scenario | Extension | Proxy Backend | Notes |
|----------|:---------:|:-------------:|-------|
| Browser + Extension | ✅ | - | Extension handles all requests |
| Browser + Rust Server | - | ✅ | Backend proxies requests |
| Browser + Both | ✅ | ✅ | Extension takes priority |
| Tauri Desktop App | - | ✅ | Embedded server in app |
| Docker Container | - | ✅ | Single container deployment |
| Static Hosting Only | - | - | Error: no bridge available |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `axum` | Web framework |
| `tokio` | Async runtime |
| `reqwest` | HTTP client with rustls |
| `serde` | Serialization |
| `rust-embed` | Embed static files in binary |
| `tower-http` | CORS, compression, tracing middleware |
| `tracing` | Structured logging |

## License

MIT
