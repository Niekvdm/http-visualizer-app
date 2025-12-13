# Build stage - Frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /frontend

# Copy frontend package files
COPY http-visualizer/package.json http-visualizer/yarn.lock ./
COPY http-visualizer/.yarnrc.yml ./.yarnrc.yml

# Install dependencies (corepack provides yarn)
RUN corepack enable && yarn install

# Copy source and build
COPY http-visualizer/ ./
RUN yarn build

# Build stage - Backend
FROM rust:1.83-alpine AS backend-builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# Copy manifests first (better layer caching)
COPY http-visualizer-app/Cargo.toml http-visualizer-app/Cargo.lock* ./
COPY http-visualizer-app/build.rs ./

# Create dummy src to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs

# Build dependencies only (cached if Cargo.toml unchanged)
RUN cargo build --release && rm -rf src target/release/deps/http_visualizer*

# Copy actual source
COPY http-visualizer-app/src ./src

# Copy frontend build
COPY --from=frontend-builder /frontend/dist ./frontend

# Build the application
RUN cargo build --release

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy the binary from builder
COPY --from=backend-builder /app/target/release/http-visualizer-app ./

# Create non-root user
RUN addgroup -g 1000 app && adduser -u 1000 -G app -s /bin/sh -D app
USER app

# Expose port
EXPOSE 3000

# Set environment variables
ENV PORT=3000
ENV RUST_LOG=http_visualizer_app=info

# Run the application
CMD ["./http-visualizer-app"]
