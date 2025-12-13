# ArgoCD Setup Guide

Complete guide for setting up ArgoCD with automatic image updates for HTTP Visualizer on k3s.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Architecture Overview](#architecture-overview)
- [1. Install ArgoCD](#1-install-argocd)
- [2. Expose ArgoCD UI](#2-expose-argocd-ui)
- [3. Install ArgoCD Image Updater](#3-install-argocd-image-updater)
- [4. Configure GitHub Container Registry](#4-configure-github-container-registry)
- [5. Configure Repository Access](#5-configure-repository-access)
- [6. Deploy the Application](#6-deploy-the-application)
- [7. Verify the Setup](#7-verify-the-setup)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

- k3s cluster running and accessible via `kubectl`
- `kubectl` configured to communicate with your cluster
- GitHub account with access to the repositories
- (Optional) MetalLB installed for LoadBalancer services

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CI/CD Pipeline                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐  │
│   │   GitHub     │    │   GitHub     │    │   GitHub Container   │  │
│   │   Push       │───▶│   Actions    │───▶│   Registry (ghcr.io) │  │
│   │              │    │   Build      │    │                      │  │
│   └──────────────┘    └──────────────┘    └──────────┬───────────┘  │
│                                                       │              │
│                                                       ▼              │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                        k3s Cluster                            │  │
│   │  ┌─────────────────┐    ┌─────────────────────────────────┐  │  │
│   │  │ ArgoCD Image    │    │ ArgoCD                          │  │  │
│   │  │ Updater         │───▶│ - Detects Git changes           │  │  │
│   │  │ - Watches ghcr  │    │ - Syncs to cluster              │  │  │
│   │  │ - Updates Git   │    │                                 │  │  │
│   │  └─────────────────┘    └───────────────┬─────────────────┘  │  │
│   │                                          │                    │  │
│   │                                          ▼                    │  │
│   │                         ┌─────────────────────────────────┐  │  │
│   │                         │ HTTP Visualizer Deployment      │  │  │
│   │                         │ - Service                       │  │  │
│   │                         │ - Ingress (SSL via cert-manager)│  │  │
│   │                         └─────────────────────────────────┘  │  │
│   └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 1. Install ArgoCD

### Create the ArgoCD namespace

```bash
kubectl create namespace argocd
```

### Install ArgoCD

```bash
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
```

### Wait for pods to be ready

```bash
kubectl wait --for=condition=Ready pods --all -n argocd --timeout=300s
```

Or watch them come up:

```bash
kubectl get pods -n argocd -w
```

Expected output (all pods should be Running):

```
NAME                                                READY   STATUS    RESTARTS   AGE
argocd-application-controller-0                     1/1     Running   0          2m
argocd-applicationset-controller-xxx                1/1     Running   0          2m
argocd-dex-server-xxx                               1/1     Running   0          2m
argocd-notifications-controller-xxx                 1/1     Running   0          2m
argocd-redis-xxx                                    1/1     Running   0          2m
argocd-repo-server-xxx                              1/1     Running   0          2m
argocd-server-xxx                                   1/1     Running   0          2m
```

### Get the initial admin password

```bash
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d && echo
```

Save this password. Username is `admin`.

---

## 2. Expose ArgoCD UI

### Option A: MetalLB LoadBalancer (Recommended for internal access)

Patch the ArgoCD server service to use LoadBalancer with a specific IP:

```bash
kubectl patch svc argocd-server -n argocd -p '{
  "spec": {
    "type": "LoadBalancer",
    "loadBalancerIP": "192.168.2.225"
  }
}'
```

Access ArgoCD at: `https://192.168.2.225`

### Option B: Port Forward (Quick testing)

```bash
kubectl port-forward svc/argocd-server -n argocd 8080:443
```

Access ArgoCD at: `https://localhost:8080`

### Option C: Ingress with TLS

Create an ingress for ArgoCD:

```yaml
# argocd-ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: argocd-ingress
  namespace: argocd
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-production"
    nginx.ingress.kubernetes.io/ssl-passthrough: "true"
    nginx.ingress.kubernetes.io/backend-protocol: "HTTPS"
spec:
  rules:
    - host: argocd.your-domain.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: argocd-server
                port:
                  number: 443
  tls:
    - hosts:
        - argocd.your-domain.com
      secretName: argocd-tls
```

```bash
kubectl apply -f argocd-ingress.yaml
```

---

## 3. Install ArgoCD Image Updater

### Install Image Updater

```bash
kubectl apply -f https://raw.githubusercontent.com/argoproj-labs/argocd-image-updater/stable/config/install.yaml
```

### Verify installation

```bash
kubectl get pods -n argocd -l app.kubernetes.io/name=argocd-image-updater
```

Expected output:

```
NAME                                       READY   STATUS    RESTARTS   AGE
argocd-image-updater-xxx                   1/1     Running   0          1m
```

### Check logs

```bash
kubectl logs -n argocd -l app.kubernetes.io/name=argocd-image-updater -f
```

---

## 4. Configure GitHub Container Registry

### Create a GitHub Personal Access Token

1. Go to GitHub → **Settings** → **Developer settings** → **Personal access tokens** → **Fine-grained tokens**
2. Click **Generate new token**
3. Configure:
   - **Token name**: `argocd-image-updater`
   - **Expiration**: Choose appropriate duration
   - **Repository access**: Select `http-visualizer-app`
   - **Permissions**:
     - `Contents`: Read and write (for git write-back)
     - `Packages`: Read (for reading container images)
4. Click **Generate token**
5. **Copy the token immediately** (you won't see it again)

### Create registry secret for Image Updater

This allows Image Updater to check ghcr.io for new images:

```bash
kubectl -n argocd create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=YOUR_GITHUB_USERNAME \
  --docker-password=YOUR_GITHUB_TOKEN
```

### Configure Image Updater to use the registry secret

Edit the Image Updater config:

```bash
kubectl -n argocd edit configmap argocd-image-updater-config
```

Add the registries configuration:

```yaml
data:
  registries.conf: |
    registries:
      - name: GitHub Container Registry
        prefix: ghcr.io
        api_url: https://ghcr.io
        credentials: pullsecret:argocd/ghcr-secret
        default: true
```

Alternatively, create the configmap from scratch:

```bash
kubectl -n argocd create configmap argocd-image-updater-config \
  --from-literal=registries.conf='
registries:
  - name: GitHub Container Registry
    prefix: ghcr.io
    api_url: https://ghcr.io
    credentials: pullsecret:argocd/ghcr-secret
    default: true
' --dry-run=client -o yaml | kubectl apply -f -
```

### Restart Image Updater to pick up changes

```bash
kubectl -n argocd rollout restart deployment argocd-image-updater
```

---

## 5. Configure Repository Access

ArgoCD and Image Updater need access to your Git repository for:
- ArgoCD: Reading k8s manifests
- Image Updater: Writing back updated image tags

### Create repository secret

```bash
kubectl -n argocd create secret generic repo-http-visualizer-app \
  --from-literal=type=git \
  --from-literal=url=https://github.com/Niekvdm/http-visualizer-app.git \
  --from-literal=username=YOUR_GITHUB_USERNAME \
  --from-literal=password=YOUR_GITHUB_TOKEN
```

### Label the secret for ArgoCD

```bash
kubectl -n argocd label secret repo-http-visualizer-app \
  argocd.argoproj.io/secret-type=repository
```

### Verify repository is connected

```bash
kubectl -n argocd get secrets -l argocd.argoproj.io/secret-type=repository
```

Or check in ArgoCD UI: **Settings** → **Repositories**

---

## 6. Deploy the Application

### Ensure k8s manifests are in your repository

Your repository should have this structure:

```
http-visualizer-app/
├── .github/
│   └── workflows/
│       └── build-push.yaml
├── k8s/
│   ├── kustomization.yaml
│   ├── namespace.yaml
│   ├── service.yaml
│   └── ingress.yaml
├── src/
├── Cargo.toml
├── Dockerfile
└── ...
```

### Apply the ArgoCD Application

```bash
kubectl apply -f k8s/argocd-application.yaml
```

Or apply directly:

```yaml
# k8s/argocd-application.yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: http-visualizer
  namespace: argocd
  annotations:
    # ArgoCD Image Updater configuration
    argocd-image-updater.argoproj.io/image-list: app=ghcr.io/niekvdm/http-visualizer
    argocd-image-updater.argoproj.io/app.update-strategy: latest
    argocd-image-updater.argoproj.io/write-back-method: git
spec:
  project: default
  source:
    repoURL: https://github.com/Niekvdm/http-visualizer-app.git
    targetRevision: HEAD
    path: k8s
  destination:
    server: https://kubernetes.default.svc
    namespace: dzone-dev
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
      - CreateNamespace=true
      - PruneLast=true
```

```bash
kubectl apply -f - <<EOF
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: http-visualizer
  namespace: argocd
  annotations:
    argocd-image-updater.argoproj.io/image-list: app=ghcr.io/niekvdm/http-visualizer
    argocd-image-updater.argoproj.io/app.update-strategy: latest
    argocd-image-updater.argoproj.io/write-back-method: git
spec:
  project: default
  source:
    repoURL: https://github.com/Niekvdm/http-visualizer-app.git
    targetRevision: HEAD
    path: k8s
  destination:
    server: https://kubernetes.default.svc
    namespace: dzone-dev
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
      - CreateNamespace=true
      - PruneLast=true
EOF
```

---

## 7. Verify the Setup

### Check ArgoCD Application status

```bash
kubectl get applications -n argocd
```

Expected output:

```
NAME               SYNC STATUS   HEALTH STATUS
http-visualizer    Synced        Healthy
```

### Check Image Updater logs

```bash
kubectl logs -n argocd -l app.kubernetes.io/name=argocd-image-updater --tail=50
```

### Check application pods

```bash
kubectl get pods -n dzone-dev
```

### View in ArgoCD UI

1. Open ArgoCD UI (https://192.168.2.225 or your configured URL)
2. Login with `admin` and the password from step 1
3. Click on `http-visualizer` application
4. View sync status, health, and resource tree

---

## Troubleshooting

### ArgoCD can't connect to repository

**Symptoms**: Application shows "ComparisonError" or "Unable to fetch"

**Solution**:
```bash
# Check repository secret
kubectl -n argocd get secret repo-http-visualizer-app -o yaml

# Verify credentials work
git ls-remote https://YOUR_TOKEN@github.com/Niekvdm/http-visualizer-app.git
```

### Image Updater not detecting new images

**Symptoms**: New images pushed but no updates

**Solution**:
```bash
# Check Image Updater logs
kubectl logs -n argocd -l app.kubernetes.io/name=argocd-image-updater -f

# Verify registry secret
kubectl -n argocd get secret ghcr-secret -o yaml

# Force Image Updater to check
kubectl -n argocd rollout restart deployment argocd-image-updater
```

### Application not syncing

**Symptoms**: Changes in Git not reflected in cluster

**Solution**:
```bash
# Check sync status
kubectl -n argocd get application http-visualizer -o yaml

# Force sync via CLI
kubectl -n argocd patch application http-visualizer --type merge -p '{"operation": {"sync": {}}}'

# Or use ArgoCD CLI
argocd app sync http-visualizer
```

### Image pull errors

**Symptoms**: Pods stuck in `ImagePullBackOff`

**Solution**:
```bash
# Check pod events
kubectl -n dzone-dev describe pod <pod-name>

# Verify image exists
docker pull ghcr.io/niekvdm/http-visualizer:latest

# Check if imagePullSecrets needed for private packages
kubectl -n dzone-dev get serviceaccount default -o yaml
```

### SSL certificate issues

**Symptoms**: Ingress not serving HTTPS

**Solution**:
```bash
# Check cert-manager logs
kubectl logs -n cert-manager -l app=cert-manager

# Check certificate status
kubectl -n dzone-dev get certificate
kubectl -n dzone-dev describe certificate tommie-tls
```

---

## Useful Commands Reference

```bash
# ArgoCD CLI login (if installed)
argocd login 192.168.2.225 --username admin --password <password> --insecure

# List applications
argocd app list

# Sync application
argocd app sync http-visualizer

# Get application details
argocd app get http-visualizer

# View Image Updater annotations
kubectl -n argocd get application http-visualizer -o jsonpath='{.metadata.annotations}' | jq

# Check which images Image Updater is tracking
kubectl logs -n argocd -l app.kubernetes.io/name=argocd-image-updater | grep -i "image"
```

---

## Next Steps

1. **Change admin password**: Settings → User Info → Update Password
2. **Set up SSO**: Configure OIDC/SAML for team access
3. **Add more applications**: Create additional ArgoCD Applications for other services
4. **Configure notifications**: Set up Slack/email notifications for sync events
