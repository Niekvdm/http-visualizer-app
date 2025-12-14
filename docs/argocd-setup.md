# ArgoCD Setup

Setup ArgoCD with automatic image updates for Project Tommie.

## Architecture

```
GitHub Push → GitHub Actions → ghcr.io → ArgoCD Image Updater → ArgoCD → k3s Deployment
```

## 1. Install ArgoCD

```bash
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
kubectl wait --for=condition=Ready pods --all -n argocd --timeout=300s

# Get admin password
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d && echo
```

## 2. Expose ArgoCD UI

```bash
# MetalLB LoadBalancer
kubectl patch svc argocd-server -n argocd -p '{"spec":{"type":"LoadBalancer","loadBalancerIP":"192.168.2.225"}}'

# Or port forward
kubectl port-forward svc/argocd-server -n argocd 8080:443
```

## 3. Install Image Updater

```bash
kubectl apply -f https://raw.githubusercontent.com/argoproj-labs/argocd-image-updater/stable/config/install.yaml
```

## 4. Configure GitHub Container Registry

Create a GitHub PAT with `Contents: Read/Write` and `Packages: Read`.

```bash
# Registry secret
kubectl -n argocd create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=YOUR_USERNAME \
  --docker-password=YOUR_TOKEN

# Configure Image Updater
kubectl -n argocd create configmap argocd-image-updater-config \
  --from-literal=registries.conf='
registries:
  - name: GitHub Container Registry
    prefix: ghcr.io
    api_url: https://ghcr.io
    credentials: pullsecret:argocd/ghcr-secret
    default: true
' --dry-run=client -o yaml | kubectl apply -f -

kubectl -n argocd rollout restart deployment argocd-image-updater
```

## 5. Configure Repository Access

```bash
kubectl -n argocd create secret generic repo-http-visualizer-app \
  --from-literal=type=git \
  --from-literal=url=https://github.com/Niekvdm/http-visualizer-app.git \
  --from-literal=username=YOUR_USERNAME \
  --from-literal=password=YOUR_TOKEN

kubectl -n argocd label secret repo-http-visualizer-app argocd.argoproj.io/secret-type=repository
```

## 6. Deploy Application

```bash
kubectl apply -f k8s/argocd-application.yaml
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Can't connect to repo | Check secret: `kubectl -n argocd get secret repo-http-visualizer-app -o yaml` |
| Image not updating | Restart updater: `kubectl -n argocd rollout restart deployment argocd-image-updater` |
| Not syncing | Force sync: `kubectl -n argocd patch application tommie --type merge -p '{"operation":{"sync":{}}}'` |
| ImagePullBackOff | Verify image: `docker pull ghcr.io/niekvdm/tommie:latest` |
