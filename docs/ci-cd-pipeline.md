# CI/CD Pipeline Setup

Guide for setting up the GitHub Actions CI/CD pipeline for HTTP Visualizer.

## Table of Contents

- [Overview](#overview)
- [GitHub Actions Workflow](#github-actions-workflow)
- [GitHub Container Registry Setup](#github-container-registry-setup)
- [Dockerfile Configuration](#dockerfile-configuration)
- [Triggering Builds](#triggering-builds)
- [Monitoring Builds](#monitoring-builds)

---

## Overview

The CI/CD pipeline automatically builds and pushes Docker images when code is pushed to the main branch.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Git Push  │────▶│   GitHub    │────▶│   Docker    │────▶│   ghcr.io   │
│   (main)    │     │   Actions   │     │   Build     │     │   Registry  │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
```

---

## GitHub Actions Workflow

### Workflow File

Location: `.github/workflows/build-push.yaml`

```yaml
name: Build and Push Docker Image

on:
  push:
    branches: [main, master]
    paths-ignore:
      - 'k8s/**'
      - '*.md'
  workflow_dispatch:  # Allows manual triggering

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: niekvdm/http-visualizer

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - name: Create workspace
        run: mkdir -p workspace

      - name: Checkout http-visualizer-app (backend)
        uses: actions/checkout@v4
        with:
          path: workspace/http-visualizer-app

      - name: Checkout http-visualizer (frontend)
        uses: actions/checkout@v4
        with:
          repository: Niekvdm/http-visualizer
          path: workspace/http-visualizer

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata (tags, labels)
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=sha,prefix=
            type=raw,value=latest,enable={{is_default_branch}}

      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          context: workspace
          file: workspace/http-visualizer-app/Dockerfile
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

### Workflow Features

| Feature | Description |
|---------|-------------|
| **Multi-repo checkout** | Checks out both backend and frontend repos |
| **Docker Buildx** | Enables advanced build features and caching |
| **GitHub Actions cache** | Uses `type=gha` for layer caching between builds |
| **Automatic tagging** | Tags with commit SHA and `latest` for main branch |
| **Manual trigger** | Can be triggered manually via `workflow_dispatch` |

---

## GitHub Container Registry Setup

### Package Visibility

By default, packages inherit the repository visibility:
- **Public repo** → Public package (anyone can pull)
- **Private repo** → Private package (requires authentication)

### Viewing Packages

1. Go to your GitHub profile
2. Click **Packages** tab
3. Select `http-visualizer`

Or directly: `https://github.com/users/Niekvdm/packages/container/package/http-visualizer`

### Pulling Images

**Public packages:**
```bash
docker pull ghcr.io/niekvdm/http-visualizer:latest
```

**Private packages (requires login):**
```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
docker pull ghcr.io/niekvdm/http-visualizer:latest
```

### Image Tags

The workflow creates these tags:

| Tag | Description | Example |
|-----|-------------|---------|
| `latest` | Latest main branch build | `ghcr.io/niekvdm/http-visualizer:latest` |
| `<sha>` | Commit SHA (7 chars) | `ghcr.io/niekvdm/http-visualizer:a1b2c3d` |

---

## Dockerfile Configuration

### Multi-stage Build with Cargo Chef

The Dockerfile uses cargo-chef for optimal Rust dependency caching:

```dockerfile
# Build stage - Frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /frontend
COPY http-visualizer/package.json http-visualizer/yarn.lock http-visualizer/.yarnrc.yml ./
COPY http-visualizer/.yarn ./.yarn
RUN corepack enable && yarn install --immutable
COPY http-visualizer/ ./
RUN yarn build

# Cargo chef - prepare recipe (analyzes dependencies)
FROM rust:1.83-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static
RUN cargo install cargo-chef
WORKDIR /app

# Planner stage - create dependency recipe
FROM chef AS planner
COPY http-visualizer-app/Cargo.toml http-visualizer-app/Cargo.lock* ./
COPY http-visualizer-app/build.rs ./
COPY http-visualizer-app/src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - build dependencies first (cached), then app
FROM chef AS backend-builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY http-visualizer-app/Cargo.toml http-visualizer-app/Cargo.lock* ./
COPY http-visualizer-app/build.rs ./
COPY http-visualizer-app/src ./src
COPY --from=frontend-builder /frontend/dist ./frontend
RUN cargo build --release

# Runtime stage
FROM alpine:3.19
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=backend-builder /app/target/release/http-visualizer-app ./
RUN addgroup -g 1000 app && adduser -u 1000 -G app -s /bin/sh -D app
USER app
EXPOSE 3000
ENV PORT=3000
ENV RUST_LOG=http_visualizer_app=info
CMD ["./http-visualizer-app"]
```

### Build Caching

The cargo-chef approach provides:

1. **Dependency caching**: Dependencies are compiled once and cached
2. **Incremental builds**: Only application code is rebuilt when source changes
3. **Fast CI builds**: Subsequent builds are significantly faster

---

## Triggering Builds

### Automatic Triggers

Builds trigger automatically on:
- Push to `main` or `master` branch
- Excluding changes to `k8s/**` and `*.md` files

### Manual Trigger

1. Go to **Actions** tab in GitHub
2. Select **Build and Push Docker Image** workflow
3. Click **Run workflow**
4. Select branch and click **Run workflow**

### Skip CI

Add `[skip ci]` to commit message to skip the build:
```bash
git commit -m "Update README [skip ci]"
```

---

## Monitoring Builds

### GitHub Actions UI

1. Go to repository → **Actions** tab
2. Click on the workflow run
3. View build logs for each step

### Build Status Badge

Add to README:
```markdown
![Build Status](https://github.com/Niekvdm/http-visualizer-app/actions/workflows/build-push.yaml/badge.svg)
```

### Notifications

Configure notifications in repository settings:
- **Settings** → **Notifications** → **Actions**

---

## Troubleshooting

### Build fails with "no space left on device"

GitHub Actions runners have limited disk space. The workflow already uses Buildx which is more efficient, but if issues persist:

```yaml
- name: Free disk space
  run: |
    sudo rm -rf /usr/share/dotnet
    sudo rm -rf /opt/ghc
    sudo rm -rf "/usr/local/share/boost"
```

### Frontend checkout fails

Ensure the frontend repository is public, or add a PAT with repo access:

```yaml
- name: Checkout http-visualizer (frontend)
  uses: actions/checkout@v4
  with:
    repository: Niekvdm/http-visualizer
    path: workspace/http-visualizer
    token: ${{ secrets.REPO_ACCESS_TOKEN }}  # For private repos
```

### Cache not working

Verify GitHub Actions cache is enabled and not full:
- Check **Actions** → **Management** → **Caches**
- Delete old caches if necessary

### Push to ghcr.io fails

Ensure the workflow has `packages: write` permission:

```yaml
permissions:
  contents: read
  packages: write
```

---

## Local Development

### Build locally

```bash
# From parent directory containing both repos
docker build -f http-visualizer-app/Dockerfile -t http-visualizer:local .
```

### Run locally

```bash
docker run -p 3000:3000 http-visualizer:local
```

### Test the image

```bash
curl http://localhost:3000/api/health
```
