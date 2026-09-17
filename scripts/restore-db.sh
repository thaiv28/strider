#!/usr/bin/env bash
# Restore the database from the backup restore point (drops + recreates objects).
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$DIR/backups/backpack.sql"
[ -f "$SRC" ] || { echo "No backup at $SRC"; exit 1; }
docker exec -i backpack-db psql -U backpack -d backpack < "$SRC"
echo "Restored from: $SRC"
