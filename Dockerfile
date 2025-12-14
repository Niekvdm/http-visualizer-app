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
FROM golang:1.23-alpine AS backend-builder

# Install build dependencies
RUN apk add --no-cache gcc musl-dev

WORKDIR /app

# Copy go mod files
COPY http-visualizer-app/go.mod http-visualizer-app/go.sum ./
RUN go mod download

# Copy source code
COPY http-visualizer-app/ ./

# Copy frontend build to embedding location
COPY --from=frontend-builder /frontend/dist ./internal/static/frontend

# Build the server binary (frontend is embedded at compile time)
RUN CGO_ENABLED=1 GOOS=linux go build -o server ./cmd/server

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary from builder (frontend is embedded in the binary)
COPY --from=backend-builder /app/server .

# Create non-root user
RUN addgroup -g 1000 app && adduser -u 1000 -G app -s /bin/sh -D app
USER app

# Expose port
EXPOSE 3000

# Set environment variables
ENV PORT=3000

# Run the server
CMD ["./server"]
