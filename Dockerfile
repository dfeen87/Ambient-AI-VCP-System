# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY crates ./crates

# Build the project
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/ambient-vcp /usr/local/bin/

# Create non-root user
RUN useradd -m -u 1000 ambient && \
    chown -R ambient:ambient /app

USER ambient

ENTRYPOINT ["ambient-vcp"]
