# Build stage - Frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /frontend

# Copy frontend source
COPY http-visualizer/package.json http-visualizer/yarn.lock http-visualizer/.yarnrc.yml ./
COPY http-visualizer/.yarn ./.yarn

# Install dependencies
RUN corepack enable && yarn install --immutable

# Copy source and build
COPY http-visualizer/ ./
RUN yarn build

# Build stage - Rust Backend
FROM rust:1.83-alpine AS backend-builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY http-visualizer-app/Cargo.toml http-visualizer-app/Cargo.lock* ./
COPY http-visualizer-app/build.rs ./

# Create src stub for dependency compilation
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY http-visualizer-app/src ./src

# Copy frontend build from previous stage
COPY --from=frontend-builder /frontend/dist ./frontend

# Build the application
RUN touch src/main.rs && cargo build --release

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
