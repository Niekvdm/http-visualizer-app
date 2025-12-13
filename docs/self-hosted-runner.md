# Self-Hosted GitHub Actions Runner

Guide for setting up a self-hosted GitHub Actions runner on Raspberry Pi 5 for native ARM64 builds.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running as a Service](#running-as-a-service)
- [Docker Setup](#docker-setup)
- [Maintenance](#maintenance)
- [Troubleshooting](#troubleshooting)

---

## Overview

### Why Self-Hosted?

| Aspect | GitHub-Hosted | Self-Hosted (RPi5) |
|--------|---------------|---------------------|
| Architecture | x86_64 only | Native ARM64 |
| Build time | ~15-20 min (QEMU) | ~5-10 min (native) |
| Cost | Free (public) / 2000 min (private) | Free |
| Availability | Always available | Depends on RPi uptime |
| Maintenance | None | You manage updates |

### Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   GitHub Push   │────▶│  GitHub Actions │────▶│   RPi5 Runner   │
│                 │     │   (triggers)    │     │   (builds)      │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                         │
                                                         ▼
                                                ┌─────────────────┐
                                                │    ghcr.io      │
                                                │  (ARM64 image)  │
                                                └─────────────────┘
```

---

## Prerequisites

- Raspberry Pi 5 with Raspberry Pi OS (64-bit)
- At least 4GB RAM (8GB recommended for Rust compilation)
- 32GB+ SD card or SSD
- Stable internet connection
- GitHub account with repo admin access

### System Requirements

```bash
# Check architecture (should be aarch64)
uname -m

# Check available memory
free -h

# Check disk space
df -h
```

---

## Installation

### Step 1: Create Runner Directory

```bash
mkdir -p ~/actions-runner
cd ~/actions-runner
```

### Step 2: Download the Runner

Get the latest ARM64 version from [GitHub Releases](https://github.com/actions/runner/releases):

```bash
# Download (check for latest version)
curl -o actions-runner-linux-arm64-2.321.0.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.321.0/actions-runner-linux-arm64-2.321.0.tar.gz

# Verify checksum (optional but recommended)
echo "62f0c17e1eb4f7e2d0f727d39e6c4dde3d5b67a0e3eb3b04d46e738e7c4cf53f  actions-runner-linux-arm64-2.321.0.tar.gz" | sha256sum -c

# Extract
tar xzf ./actions-runner-linux-arm64-2.321.0.tar.gz
```

### Step 3: Get Registration Token

1. Go to your repository on GitHub
2. Navigate to **Settings** → **Actions** → **Runners**
3. Click **"New self-hosted runner"**
4. Select **Linux** and **ARM64**
5. Copy the token from the configuration command

Or use GitHub CLI:
```bash
gh api -X POST repos/Niekvdm/http-visualizer-app/actions/runners/registration-token | jq -r '.token'
```

### Step 4: Configure the Runner

```bash
./config.sh --url https://github.com/Niekvdm/http-visualizer-app --token YOUR_TOKEN
```

You'll be prompted for:
- **Runner group**: Press Enter for default
- **Runner name**: e.g., `rpi5-builder` (or press Enter for hostname)
- **Labels**: `self-hosted,Linux,ARM64` (default) - add `rpi5` for easier targeting
- **Work folder**: Press Enter for default `_work`

Example with all options:
```bash
./config.sh \
  --url https://github.com/Niekvdm/http-visualizer-app \
  --token YOUR_TOKEN \
  --name rpi5-builder \
  --labels self-hosted,Linux,ARM64,rpi5 \
  --work _work \
  --unattended
```

---

## Configuration

### Runner Labels

Labels help workflows target specific runners:

| Label | Description |
|-------|-------------|
| `self-hosted` | Required for self-hosted runners |
| `Linux` | Operating system |
| `ARM64` | Architecture |
| `rpi5` | Custom label for this specific runner |

### Adding Custom Labels

```bash
# Stop the runner first if running
./config.sh --labels self-hosted,Linux,ARM64,rpi5,docker
```

Or add via GitHub UI: Settings → Actions → Runners → [your runner] → Edit labels

---

## Running as a Service

### Install the Service

```bash
sudo ./svc.sh install
```

### Service Commands

```bash
# Start the service
sudo ./svc.sh start

# Stop the service
sudo ./svc.sh stop

# Check status
sudo ./svc.sh status

# Uninstall the service
sudo ./svc.sh uninstall
```

### Enable on Boot

The service is automatically enabled on boot after installation. Verify:

```bash
sudo systemctl is-enabled actions.runner.Niekvdm-http-visualizer-app.rpi5-builder.service
```

### View Logs

```bash
# Live logs
journalctl -u actions.runner.Niekvdm-http-visualizer-app.rpi5-builder.service -f

# Recent logs
journalctl -u actions.runner.Niekvdm-http-visualizer-app.rpi5-builder.service -n 100
```

---

## Docker Setup

The runner needs Docker to build container images.

### Install Docker

```bash
# Install Docker using the convenience script
curl -fsSL https://get.docker.com | sh

# Add your user to the docker group
sudo usermod -aG docker $USER

# Apply group changes (or log out and back in)
newgrp docker

# Verify installation
docker --version
docker run hello-world
```

### Configure Docker for the Runner

The runner service runs as root by default, which has Docker access. If running as a different user:

```bash
# Check which user runs the service
cat /etc/systemd/system/actions.runner.*.service | grep User

# Add that user to docker group
sudo usermod -aG docker <runner-user>

# Restart the runner
sudo ./svc.sh stop
sudo ./svc.sh start
```

### Docker BuildKit (Recommended)

Enable BuildKit for better build performance:

```bash
# Add to /etc/docker/daemon.json
sudo tee /etc/docker/daemon.json <<EOF
{
  "features": {
    "buildkit": true
  }
}
EOF

# Restart Docker
sudo systemctl restart docker
```

---

## Maintenance

### Updating the Runner

GitHub will notify you when updates are available.

```bash
cd ~/actions-runner

# Stop the service
sudo ./svc.sh stop

# Download new version
curl -o actions-runner-linux-arm64-X.XXX.X.tar.gz -L \
  https://github.com/actions/runner/releases/download/vX.XXX.X/actions-runner-linux-arm64-X.XXX.X.tar.gz

# Extract (overwrites existing files)
tar xzf ./actions-runner-linux-arm64-X.XXX.X.tar.gz

# Start the service
sudo ./svc.sh start
```

### Cleanup Old Docker Images

Build artifacts can consume disk space:

```bash
# Remove unused images
docker image prune -a

# Remove all build cache
docker builder prune -a

# Full cleanup (careful!)
docker system prune -a
```

Add to crontab for automatic cleanup:

```bash
# Edit crontab
crontab -e

# Add weekly cleanup (Sundays at 3 AM)
0 3 * * 0 docker system prune -af --volumes
```

### Monitor Disk Usage

```bash
# Check Docker disk usage
docker system df

# Check overall disk usage
df -h
```

---

## Troubleshooting

### Runner Not Appearing in GitHub

**Symptoms**: Runner shows as offline or doesn't appear

**Solutions**:
```bash
# Check if service is running
sudo ./svc.sh status

# Check logs for errors
journalctl -u actions.runner.*.service -n 50

# Re-register the runner
./config.sh remove
./config.sh --url https://github.com/Niekvdm/http-visualizer-app --token NEW_TOKEN
```

### Docker Permission Denied

**Symptoms**: `permission denied while trying to connect to the Docker daemon`

**Solutions**:
```bash
# Check docker group membership
groups

# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker

# Or run runner service as root (default)
sudo ./svc.sh stop
sudo ./svc.sh start
```

### Build Fails with Out of Memory

**Symptoms**: Rust compilation killed, exit code 137

**Solutions**:
```bash
# Check available memory during build
watch -n 1 free -h

# Add swap space
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Make permanent
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
```

### Slow Builds

**Tips for faster builds**:

1. Use SSD instead of SD card
2. Increase swap if RAM is limited
3. Enable Docker BuildKit
4. Use build cache in workflow

### Runner Stuck on Job

**Symptoms**: Job appears stuck, no progress

**Solutions**:
```bash
# Check runner logs
journalctl -u actions.runner.*.service -f

# Force stop current job (last resort)
sudo ./svc.sh stop
sudo ./svc.sh start

# Cancel job in GitHub UI
```

### Network Issues

**Symptoms**: Runner can't connect to GitHub

**Solutions**:
```bash
# Test connectivity
curl -I https://github.com
curl -I https://api.github.com

# Check DNS
nslookup github.com

# Check firewall
sudo iptables -L
```

---

## Security Considerations

### Runner Security

- Self-hosted runners should only be used with **trusted repositories**
- The runner has access to your RPi5 and Docker daemon
- Consider running in a VM or container for isolation

### Network Security

```bash
# Runner needs outbound access to:
# - github.com (443)
# - api.github.com (443)
# - codeload.github.com (443)
# - ghcr.io (443)
# - *.actions.githubusercontent.com (443)
```

### Secrets Handling

- GitHub Actions secrets are passed securely to the runner
- Secrets are masked in logs
- Clean up sensitive files after builds

---

## Useful Commands Reference

```bash
# Runner status
sudo ./svc.sh status

# View runner logs
journalctl -u actions.runner.*.service -f

# Docker status
docker info
docker system df

# System resources
htop
free -h
df -h

# Network check
curl -I https://github.com

# Restart everything
sudo ./svc.sh stop
sudo systemctl restart docker
sudo ./svc.sh start
```

---

## Removing the Runner

If you need to remove the runner completely:

```bash
cd ~/actions-runner

# Stop and uninstall service
sudo ./svc.sh stop
sudo ./svc.sh uninstall

# Remove from GitHub
./config.sh remove --token YOUR_TOKEN

# Delete files
cd ~
rm -rf actions-runner
```

Or remove via GitHub UI: Settings → Actions → Runners → [runner] → Remove
