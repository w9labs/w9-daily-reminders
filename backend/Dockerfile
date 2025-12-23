# Multi-stage build for w9-daily-reminders backend
# Stage 1: Build
FROM rust:1.83-slim AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/

# Build the binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (wget for healthcheck)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/w9-daily-reminders /app/w9-daily-reminders

# Create data directory
RUN mkdir -p /app/data && \
    chmod 755 /app/data

# Run as non-root user
RUN useradd -r -s /bin/false w9reminder && \
    chown -R w9reminder:w9reminder /app

USER w9reminder

EXPOSE 8787

ENTRYPOINT ["/app/w9-daily-reminders"]

