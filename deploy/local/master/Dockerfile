FROM rust:1.88-slim-bookworm

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch --locked

COPY backend/migrations /migrations-seed
COPY backend/entrypoint-dev.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

WORKDIR /app
EXPOSE 8080
ENTRYPOINT ["/entrypoint.sh"]
