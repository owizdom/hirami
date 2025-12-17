FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the application
RUN cargo build --release --bin mev-africa

FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/mev-africa /usr/local/bin/mev-africa

# Create data directory
RUN mkdir -p /data

# Set environment variables
ENV DATABASE_PATH=/data/mev_africa.db
ENV POLL_INTERVAL_SECONDS=12
ENV METRICS_BIND_ADDRESS=0.0.0.0:9090

# Expose metrics port
EXPOSE 9090

# Run the service
CMD ["mev-africa", "ingest"]



