FROM rust:1.88-slim-bookworm

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch --locked

WORKDIR /app
EXPOSE 8080
