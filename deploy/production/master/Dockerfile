# syntax=docker/dockerfile:1.7
FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo fetch --locked
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN mkdir -p /build/data \
    && sqlite3 /build/data/novabox.db < /build/migrations/001_init.sql \
    && for migration_file in /build/migrations/002*.sql; do \
         if [ -f "$migration_file" ]; then sqlite3 /build/data/novabox.db < "$migration_file"; fi; \
       done
ENV DATABASE_URL=sqlite:///build/data/novabox.db
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/novabox /usr/local/bin/novabox
EXPOSE 8080
CMD ["novabox"]
