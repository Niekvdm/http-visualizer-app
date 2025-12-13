# Deployment Quick Reference

Quick reference for common deployment operations.

---

## URLs & Access

| Service | URL | Credentials |
|---------|-----|-------------|
| ArgoCD UI | https://192.168.2.225 | admin / `kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" \| base64 -d` |
| HTTP Visualizer | https://tommie.digit.zone | N/A |
| GitHub Container Registry | ghcr.io/niekvdm/http-visualizer | GitHub token |

---

## Common Commands

### Check Deployment Status

```bash
# All in one
kubectl get pods -n dzone-dev && kubectl get application -n argocd http-visualizer
```

### Force Sync

```bash
kubectl -n argocd patch application http-visualizer --type merge -p '{"operation": {"sync": {}}}'
```

### Restart Application

```bash
kubectl -n dzone-dev rollout restart deployment tommie-deployment
```

### View Logs

```bash
# Application logs
kubectl -n dzone-dev logs -l app=tommie -f

# ArgoCD logs
kubectl -n argocd logs -l app.kubernetes.io/name=argocd-server -f

# Image Updater logs
kubectl -n argocd logs -l app.kubernetes.io/name=argocd-image-updater -f
```

### Check Image Version

```bash
kubectl -n dzone-dev get deployment tommie-deployment -o jsonpath='{.spec.template.spec.containers[0].image}'
```

---

## Deployment Flow

```
1. Push code to main branch
         ↓
2. GitHub Actions builds image (~5-10 min)
         ↓
3. Image pushed to ghcr.io
         ↓
4. ArgoCD Image Updater detects new image (~2 min)
         ↓
5. Image Updater commits new tag to Git
         ↓
6. ArgoCD syncs the change (~1 min)
         ↓
7. New pod deployed with new image
```

---

## Manual Operations

### Trigger GitHub Actions Build

```bash
gh workflow run build-push.yaml --repo Niekvdm/http-visualizer-app
```

Or via GitHub UI: Actions → Build and Push → Run workflow

### Pull Latest Image Manually

```bash
# On k3s node
sudo k3s ctr images pull ghcr.io/niekvdm/http-visualizer:latest
```

### Apply k8s Changes Directly (bypass ArgoCD)

```bash
kubectl apply -f k8s/
```

> **Warning**: ArgoCD will revert manual changes on next sync if `selfHeal: true`

---

## Troubleshooting Quick Fixes

| Problem | Quick Fix |
|---------|-----------|
| Pod stuck in ImagePullBackOff | Check image exists: `docker pull ghcr.io/niekvdm/http-visualizer:latest` |
| ArgoCD shows OutOfSync | Click Sync in UI or run force sync command |
| Image Updater not updating | Restart: `kubectl -n argocd rollout restart deployment argocd-image-updater` |
| SSL cert not working | Check cert-manager: `kubectl -n dzone-dev describe certificate` |
| Application unhealthy | Check logs: `kubectl -n dzone-dev logs -l app=tommie` |

---

## Important Files

```
http-visualizer-app/
├── .github/workflows/build-push.yaml  # CI pipeline
├── k8s/
│   ├── argocd-application.yaml        # ArgoCD app definition
│   ├── kustomization.yaml             # Kustomize config
│   ├── namespace.yaml                 # Namespace
│   ├── service.yaml                   # Deployment + Service
│   └── ingress.yaml                   # Ingress + TLS
├── Dockerfile                         # Container build
└── docs/
    ├── argocd-setup.md               # Full setup guide
    ├── ci-cd-pipeline.md             # CI/CD details
    └── deployment-quickref.md        # This file
```
