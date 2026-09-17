#!/usr/bin/env bash
# Dump the Postgres database to a single overwritten restore point.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$DIR/backups/backpack.sql"
mkdir -p "$DIR/backups"
docker exec backpack-db pg_dump -U backpack -d backpack --clean --if-exists > "$OUT"
echo "Backup written: $OUT ($(wc -l < "$OUT") lines)"
