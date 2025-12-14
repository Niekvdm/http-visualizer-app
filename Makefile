.PHONY: all server desktop dev clean deps frontend install-wails docker docker-build docker-push

# Use local temp dir to avoid execution restrictions on Windows
TMPDIR := $(CURDIR)/.tmp

# Default frontend source (compiled dist from http-visualizer)
FRONTEND_SRC ?= ../http-visualizer/dist

# Docker image settings
DOCKER_IMAGE ?= ghcr.io/niekvdm/tommie
DOCKER_TAG ?= latest

# Default target
all: server desktop

# Install dependencies
deps:
	go mod download
	go mod tidy

# Install Wails CLI
install-wails:
	go install github.com/wailsapp/wails/v2/cmd/wails@latest

# Build web server (static files only)
server: deps
	@echo "Building web server..."
	GOTMPDIR="$(TMPDIR)" TMP="$(TMPDIR)" TEMP="$(TMPDIR)" go build -o bin/server.exe ./cmd/server

# Build Wails desktop app
desktop: deps
	@mkdir -p $(TMPDIR)
	@echo "Building desktop app..."
	GOTMPDIR="$(TMPDIR)" TMP="$(TMPDIR)" TEMP="$(TMPDIR)" wails build

# Development mode for desktop
dev:
	@mkdir -p $(TMPDIR)
	GOTMPDIR="$(TMPDIR)" TMP="$(TMPDIR)" TEMP="$(TMPDIR)" wails dev

# Clean build artifacts
clean:
	rm -rf bin/
	rm -rf build/
	rm -rf .tmp/

# Copy frontend from dist folder
frontend:
	@echo "Copying frontend from $(FRONTEND_SRC)..."
	rm -rf frontend
	cp -r $(FRONTEND_SRC) frontend
	@echo "Frontend copied successfully."

# Run web server locally
run-server: server
	./bin/server.exe

# Generate Wails bindings
generate:
	wails generate module

# Build Docker image
docker-build: frontend
	@echo "Building Docker image..."
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .

# Push Docker image
docker-push:
	@echo "Pushing Docker image..."
	docker push $(DOCKER_IMAGE):$(DOCKER_TAG)

# Build and push Docker image
docker: docker-build docker-push

# Help
help:
	@echo "Available targets:"
	@echo "  all          - Build both server and desktop"
	@echo "  server       - Build web server (static files only)"
	@echo "  desktop      - Build Wails desktop app"
	@echo "  dev          - Run Wails in development mode"
	@echo "  clean        - Remove build artifacts"
	@echo "  deps         - Install/update dependencies"
	@echo "  frontend     - Copy frontend from ../http-visualizer/dist"
	@echo "  install-wails- Install Wails CLI"
	@echo "  run-server   - Build and run web server"
	@echo "  docker-build - Build Docker image"
	@echo "  docker-push  - Push Docker image to registry"
	@echo "  docker       - Build and push Docker image"
	@echo "  help         - Show this help message"
