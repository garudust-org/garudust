# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1.82-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies separately from source code.
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY bin/ bin/

RUN cargo build --release --bin garudust-server

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/garudust-server /usr/local/bin/garudust-server

ENV GARUDUST_PORT=3000
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/garudust-server"]
