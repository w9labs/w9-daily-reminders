FROM rust:1.83-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml ./
COPY server/Cargo.toml ./server/
COPY client/Cargo.toml ./client/
RUN mkdir -p server/src client/src && echo "fn main(){}" > server/src/main.rs && echo "" > client/src/lib.rs
RUN cargo fetch || true
COPY server/src ./server/src
COPY client/src ./client/src
RUN cd server && cargo build --release && cp target/release/w9-daily-reminders-server /app/app

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 wget && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash w9reminders
WORKDIR /app
COPY --from=builder /app/app /usr/local/bin/w9-daily-reminders-server
RUN chown -R w9reminders:w9reminders /app
USER w9reminders
EXPOSE 8084
HEALTHCHECK --interval=30s --timeout=10s --retries=3 CMD wget --quiet --tries=1 --spider http://localhost:8084/api/health || exit 1
ENV HOST=0.0.0.0 PORT=8084
CMD ["w9-daily-reminders-server"]
