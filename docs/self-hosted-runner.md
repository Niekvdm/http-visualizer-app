# Self-Hosted Runner (RPi5)

GitHub Actions runner on Raspberry Pi 5 for native ARM64 builds.

## Why Self-Hosted?

| | GitHub-Hosted | Self-Hosted (RPi5) |
|-|---------------|---------------------|
| Architecture | x86_64 (QEMU for ARM) | Native ARM64 |
| Build time | ~15-20 min | ~3-5 min |
| Cost | Free (public) / limited (private) | Free |

## Installation

```bash
mkdir -p ~/actions-runner && cd ~/actions-runner

# Download ARM64 runner (check latest version)
curl -o actions-runner-linux-arm64-2.321.0.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.321.0/actions-runner-linux-arm64-2.321.0.tar.gz
tar xzf ./actions-runner-linux-arm64-2.321.0.tar.gz

# Get token: repo Settings → Actions → Runners → New self-hosted runner
./config.sh --url https://github.com/Niekvdm/http-visualizer-app --token YOUR_TOKEN \
  --name rpi5-builder --labels self-hosted,Linux,ARM64 --unattended
```

## Run as Service

```bash
sudo ./svc.sh install
sudo ./svc.sh start

# Logs
journalctl -u actions.runner.Niekvdm-http-visualizer-app.rpi5-builder.service -f
```

## Docker Setup

```bash
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# Enable BuildKit
sudo tee /etc/docker/daemon.json <<EOF
{"features":{"buildkit":true}}
EOF
sudo systemctl restart docker
```

## Maintenance

```bash
# Cleanup (weekly cron)
docker system prune -af --volumes

# Update runner
sudo ./svc.sh stop
curl -o actions-runner-linux-arm64-X.X.X.tar.gz -L https://github.com/actions/runner/releases/download/vX.X.X/actions-runner-linux-arm64-X.X.X.tar.gz
tar xzf ./actions-runner-linux-arm64-X.X.X.tar.gz
sudo ./svc.sh start
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Runner offline | `sudo ./svc.sh status`, check logs |
| Docker permission denied | `sudo usermod -aG docker $USER && newgrp docker` |
| Out of memory | Add swap: `sudo fallocate -l 4G /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile` |
| Slow builds | Use SSD, enable BuildKit, ensure build cache |

## Remove Runner

```bash
sudo ./svc.sh stop && sudo ./svc.sh uninstall
./config.sh remove --token YOUR_TOKEN
rm -rf ~/actions-runner
```
