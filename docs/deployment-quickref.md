# Deployment Quick Reference

## URLs

| Service | URL |
|---------|-----|
| ArgoCD UI | https://192.168.2.225 |
| Tommie | https://tommie.digit.zone |
| Container Registry | ghcr.io/niekvdm/tommie |

## Commands

```bash
# Status
kubectl get pods -n dzone-dev && kubectl get application -n argocd tommie

# Force sync
kubectl -n argocd patch application tommie --type merge -p '{"operation":{"sync":{}}}'

# Restart
kubectl -n dzone-dev rollout restart deployment tommie-deployment

# Logs
kubectl -n dzone-dev logs -l app=tommie -f

# Current image
kubectl -n dzone-dev get deployment tommie-deployment -o jsonpath='{.spec.template.spec.containers[0].image}'
```

## Deployment Flow

```
Push to main → GitHub Actions (~3-5 min) → ghcr.io → Image Updater (~2 min) → ArgoCD sync (~1 min)
```

## Quick Fixes

| Problem | Fix |
|---------|-----|
| ImagePullBackOff | `docker pull ghcr.io/niekvdm/tommie:latest` |
| OutOfSync | Sync in ArgoCD UI |
| Image not updating | `kubectl -n argocd rollout restart deployment argocd-image-updater` |
| Unhealthy | `kubectl -n dzone-dev logs -l app=tommie` |
