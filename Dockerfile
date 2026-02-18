# Multi-stage build for Ambient AI VCP API Server
# Use slim image to reduce mirrored layer downloads on Render build workers.
FROM rust:1.88-slim-bookworm AS builder
WORKDIR /app
# Install WasmEdge native libraries
RUN apt-get update && \
    apt-get install -y curl libssl-dev pkg-config git python3 && \
    rm -rf /var/lib/apt/lists/* && \
    curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | \
    bash -s -- -p /usr/local
# Copy workspace files
COPY Cargo.toml ./
COPY crates ./crates
# Build the API server
RUN cargo build --release --bin api-server
# Runtime stage
FROM debian:bookworm-slim AS runtime
WORKDIR /app
# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*
# Copy the API server binary from builder
COPY --from=builder /app/target/release/api-server /app/api-server
# Copy WasmEdge shared libraries into runtime image
COPY --from=builder /usr/local/lib/libwasmedge.so* /usr/local/lib/
RUN ldconfig
# Copy migrations directory
COPY crates/api-server/migrations /app/migrations
# Create non-root user
RUN useradd -m -u 1000 ambient && \
    chown -R ambient:ambient /app
USER ambient
# Expose port (Render.com uses PORT env var)
EXPOSE 10000
# Run the API server
CMD ["/app/api-server"]
