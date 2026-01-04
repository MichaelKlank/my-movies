# Build stage for frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app

RUN corepack enable && corepack prepare pnpm@latest --activate

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml* ./
COPY apps/web/package.json apps/web/

RUN pnpm install --frozen-lockfile || pnpm install

COPY apps/web apps/web
RUN pnpm --filter @my-movies/web build

# Build stage for backend
# Rust 1.85+ required for Edition 2024
FROM rust:1.85-bookworm AS backend-builder
WORKDIR /app

# Install SQLx CLI for migrations
RUN cargo install sqlx-cli --no-default-features --features sqlite

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates crates

# Copy Tauri Cargo.toml (needed for workspace resolution, but we won't build it)
COPY apps/tauri/src-tauri/Cargo.toml apps/tauri/src-tauri/Cargo.toml
RUN mkdir -p apps/tauri/src-tauri/src && echo "fn main() {}" > apps/tauri/src-tauri/src/main.rs

# Build release binary (server only, not tauri)
RUN cargo build --release --package my-movies-server

# Runtime stage
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/target/release/my-movies-server /app/my-movies-server

# Copy frontend build
COPY --from=frontend-builder /app/apps/web/dist /app/static

# Create data directory with proper permissions
RUN mkdir -p /app/data && chmod 777 /app/data

# Set environment variables
ENV HOST=0.0.0.0
ENV PORT=3000
ENV DATABASE_URL=sqlite:/app/data/my-movies.db?mode=rwc
ENV STATIC_DIR=/app/static
ENV RUST_LOG=info

EXPOSE 3000

VOLUME ["/app/data"]

CMD ["/app/my-movies-server"]
