# ── builder stage ────────────────────────────────────────────────────────────
FROM rust:1.77-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    cmake \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    libsasl2-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release

# ── runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    libssl3 \
    libsasl2-2 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/qps-core /app/qps-core
COPY .env .env

EXPOSE 50051

ENTRYPOINT ["/app/qps-core"]
