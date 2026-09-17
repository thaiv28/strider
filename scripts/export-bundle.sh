#!/usr/bin/env bash
# Bundle the entire project — backpack-app (code + git history + .env + a full DB
# dump), the gpx-routing MCP server, its Claude registration, and setup/steering
# docs — into a single portable zip.
#
# Usage:  bash scripts/export-bundle.sh
# Env overrides: GPX_MCP_DIR (default ~/gpx-routing-mcp), OUT_DIR (default ~).
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MCP_SRC="${GPX_MCP_DIR:-$HOME/gpx-routing-mcp}"
OUT_DIR="${OUT_DIR:-$HOME}"
STAMP="$(date +%Y%m%d-%H%M%S)"
WORK="$(mktemp -d)"
STAGE="$WORK/backpack-bundle"
OUT="$OUT_DIR/backpack-bundle-$STAMP.zip"
mkdir -p "$STAGE"

command -v rsync >/dev/null || { echo "rsync required" >&2; exit 1; }

# 1. Full database dump (schema + data, self-cleaning on restore).
echo "==> Dumping database"
if docker ps --format '{{.Names}}' | grep -qx backpack-db; then
  mkdir -p "$APP_DIR/backups"
  docker exec backpack-db pg_dump -U backpack -d backpack --clean --if-exists \
    > "$APP_DIR/backups/backpack.sql"
  echo "    backups/backpack.sql ($(wc -l < "$APP_DIR/backups/backpack.sql") lines)"
else
  echo "    WARNING: 'backpack-db' container not running — DB NOT included." >&2
  echo "             Start it (npm run db:up) and re-run to include data." >&2
fi

# 2. backpack-app: keep .git, .env, backups; drop regenerable dirs.
echo "==> Copying backpack-app (excluding node_modules/.next/build)"
rsync -a --exclude node_modules --exclude .next --exclude build \
  "$APP_DIR/" "$STAGE/backpack-app/"

# 3. gpx-routing MCP server code.
if [ -d "$MCP_SRC" ]; then
  echo "==> Copying gpx-routing-mcp (excluding node_modules)"
  rsync -a --exclude node_modules "$MCP_SRC/" "$STAGE/gpx-routing-mcp/"
else
  echo "    WARNING: $MCP_SRC not found — MCP server NOT included." >&2
fi

# 4. Extract the gpx-routing MCP registration from ~/.claude.json (any scope).
echo "==> Extracting MCP registration"
mkdir -p "$STAGE/mcp"
python3 - "$HOME/.claude.json" > "$STAGE/mcp/gpx-routing.json" <<'PY'
import json, sys
try:
    d = json.load(open(sys.argv[1]))
except Exception:
    print("{}"); sys.exit(0)
found = None
def walk(o):
    global found
    if isinstance(o, dict):
        srv = o.get("mcpServers")
        if isinstance(srv, dict) and "gpx-routing" in srv:
            found = srv["gpx-routing"]
        for v in o.values():
            walk(v)
walk(d)
print(json.dumps(found or {}, indent=2))
PY

# 5. Top-level agent setup guide.
cat > "$STAGE/SETUP.md" <<'MD'
# backpack-bundle — unpack & setup (read me first)

You are an agent (or human) restoring this project onto a fresh machine. This
bundle is self-contained: application code + git history + secrets + a full
database dump + the companion MCP server + its registration.

## Contents
```
backpack-bundle/
├── SETUP.md                     ← this file
├── backpack-app/                ← the Next.js app (full git repo, incl. .env)
│   ├── backups/backpack.sql     ← full Postgres dump (schema + data)
│   └── docs/handoff/            ← steering docs: ARCHITECTURE.md, CONVENTIONS.md
├── gpx-routing-mcp/             ← companion MCP server (waypoints → trail GPX)
└── mcp/gpx-routing.json         ← the Claude registration for that server
```
Read `backpack-app/docs/handoff/ARCHITECTURE.md` and `CONVENTIONS.md` before
making changes — they capture the non-obvious design and pitfalls.

## Prerequisites
- **Node ≥ 20** (developed on 24), **npm**, and **Docker** (for Postgres).
- **Claude Code** if you want the GPX routing MCP wired in.

## 1 — Application
```bash
cd backpack-app
npm install
npm run db:up          # Postgres 16 in Docker, host port 5433, volume backpack-pgdata
sleep 5                # let the DB accept connections
npm run db:restore     # loads backups/backpack.sql (schema + data)
npm run dev            # http://localhost:3000
```
- `.env` is included (real secrets: DATABASE_URL, FDC_API_KEY, GEOAPIFY_API_KEY,
  NEXT_PUBLIC_WAQI_TOKEN). Confirm `DATABASE_URL` points at `localhost:5433`.
  `NEXT_PUBLIC_*` vars are baked at boot — restart `dev` after changing them.
- **No data yet?** If the dump was skipped (container was down at export time),
  create just the schema with `npm run db:push`, then `npm run import` to reload
  from the legacy sheet exports (see the app README).
- `npm run db:down` removes the container but **keeps** the volume's data.
  `npm run db:studio` opens Drizzle Studio.

## 2 — GPX routing MCP (optional but recommended)
```bash
cd ../gpx-routing-mcp
npm install
npm run build          # compiles to dist/
```
Register it with Claude Code, pointing at the **new absolute path** and reusing
the ORS key from `mcp/gpx-routing.json` (field `env.ORS_API_KEY`):
```bash
claude mcp add gpx-routing \
  --env ORS_API_KEY='<paste from ../mcp/gpx-routing.json>' \
  -- node "$(pwd)/dist/index.js"
```
Use `-s user` to make it available in every project rather than just the current
directory. Get your own free key at https://openrouteservice.org/dev if the
included one is missing or rate-limited.

## Verify
- `curl -s -o /dev/null -w '%{http_code}' http://localhost:3000/` → `200`
- `cd backpack-app && npx tsc --noEmit` → clean
- In Claude Code, `/mcp` lists `gpx-routing` as connected.
MD

# 6. Zip it up.
echo "==> Writing $OUT"
( cd "$WORK" && zip -qr "$OUT" backpack-bundle )
rm -rf "$WORK"
echo "Done: $OUT"
du -h "$OUT" | cut -f1 | sed 's/^/    size: /'
