#!/bin/bash
set -e

DB_PATH="${DATABASE_URL#sqlite://}"
DB_PATH="${DB_PATH#//}"
DB_PATH="/${DB_PATH#/}"

mkdir -p "$(dirname "$DB_PATH")"

if [ ! -f "$DB_PATH" ]; then
    echo "[novabox-dev] Creating development database at $DB_PATH"
    sqlite3 "$DB_PATH" < /migrations-seed/001_init.sql
else
    echo "[novabox-dev] Database already exists at $DB_PATH"
fi

echo "[novabox-dev] Applying schema patches..."
sqlite3 "$DB_PATH" "ALTER TABLE servers ADD COLUMN online_mode INTEGER NOT NULL DEFAULT 1;" 2>/dev/null || true
sqlite3 "$DB_PATH" "ALTER TABLE servers ADD COLUMN auto_start INTEGER NOT NULL DEFAULT 0;" 2>/dev/null || true
sqlite3 "$DB_PATH" "ALTER TABLE servers ADD COLUMN auto_start_delay INTEGER NOT NULL DEFAULT 0;" 2>/dev/null || true
sqlite3 "$DB_PATH" "ALTER TABLE servers ADD COLUMN crash_detection INTEGER NOT NULL DEFAULT 1;" 2>/dev/null || true
sqlite3 "$DB_PATH" "ALTER TABLE servers ADD COLUMN shutdown_timeout INTEGER NOT NULL DEFAULT 30;" 2>/dev/null || true
sqlite3 "$DB_PATH" "ALTER TABLE servers ADD COLUMN show_on_status_page INTEGER NOT NULL DEFAULT 0;" 2>/dev/null || true

sqlite3 "$DB_PATH" "DELETE FROM _sqlx_migrations WHERE version = 1;" 2>/dev/null || true

exec cargo watch -x run -w src -w migrations
