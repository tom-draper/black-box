# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:bookworm AS builder

WORKDIR /build

# Cache dependencies by building a stub first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/black_box* target/release/black-box

# Build the real binary
COPY src ./src
RUN cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/black-box /app/black-box

EXPOSE 8080

CMD ["/app/black-box"]
