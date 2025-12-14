# CI/CD Pipeline

GitHub Actions workflow for building and pushing Docker images.

## Overview

```
Git Push (main) → GitHub Actions → Docker Build → ghcr.io
```

## Workflow

Location: `.github/workflows/build-push.yaml`

**Features:**
- Multi-repo checkout (backend + frontend)
- Docker Buildx with registry caching
- Automatic tagging (SHA + latest)
- Manual trigger via `workflow_dispatch`

## Image Tags

| Tag | Description |
|-----|-------------|
| `latest` | Latest main branch build |
| `<sha>` | Commit SHA (7 chars) |

## Dockerfile

Multi-stage build:
1. **frontend-builder**: Node 20 Alpine, builds Vue frontend
2. **backend-builder**: Go 1.23 Alpine, compiles Go server with embedded frontend
3. **runtime**: Alpine 3.19, minimal production image

## Triggers

- Push to `main`/`master` (excluding `k8s/**` and `*.md`)
- Manual: Actions → Build and Push → Run workflow

Skip builds: `git commit -m "message [skip ci]"`

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Frontend checkout fails | Add PAT for private repos |
| Push to ghcr.io fails | Ensure `packages: write` permission |
| Cache not working | Check Actions → Management → Caches |

## Local Build

```bash
# From parent dir containing both repos
docker build -f http-visualizer-app/Dockerfile -t tommie:local .
docker run -p 3000:3000 tommie:local
```
