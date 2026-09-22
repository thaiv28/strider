# backpack-app — Architecture

Strider is a multi-user backpacking trip planner and gear/weight tracker. Auth.js
uses verified Google identity for public, self-service sign-in and JWT sessions
carry the owning database user id. Private reads and mutations validate that id
against every user-owned parent resource before accessing nested records.

The application is designed for both human and agent-driven changes. Favor
small composable server actions, keep the schema as the data-model source of
truth, and keep operational behavior in version-controlled workflows/scripts.

## Stack

- **Next.js 15** (App Router) · **React 19** · **TypeScript 5.7** · Auth.js 5 · ESM (`"type":"module"`).
- **Postgres 16**. Local development uses Docker on host port **5433** with the
  `backpack-pgdata` volume. Production uses a dedicated container and retained
  host storage at `/srv/backpack/postgres`.
- **Drizzle ORM 0.36** + **drizzle-kit 0.28**. Schema at `src/db/schema.ts` is
  the source of truth. Committed migrations live in `drizzle/`; production runs
  `npm run db:migrate` before replacing the app container.
- **Tailwind v4** (`@tailwindcss/postcss`), custom palette tokens (see CONVENTIONS).
- **Leaflet 1.9 / react-leaflet 5** for maps; **@dnd-kit** for drag-reorder;
  **marked** for markdown; **node-ical** for calendar feeds; **pg** driver.
- Node **24** (`.nvmrc`), though anything ≥20 should work.

## Runtime model

- Server Components by default; pages that read the DB set
  `export const dynamic = "force-dynamic"`.
- Mutations are **server actions** in `app/<area>/actions.ts` (`"use server"`),
  which call Drizzle and `revalidatePath()` the affected routes.
- Binary responses (permit files) use a **GET route handler**
  (`app/trips/[id]/permit/route.ts`) streaming `bytea` from the DB.
- Auth.js middleware protects every route except the public landing page, login, health, icons, Next.js
  static assets, and tokenized `/share/trips/[token]` views. Shared trip URLs are
  persistent bearer links stored per trip; they are read-only, omit private trip
  report notes, and can be revoked by clearing the token. A shared link does not
  create a user session or grant access to any other Trip.
- `/api/health` verifies that the app can query PostgreSQL; it does not verify
  every third-party map, food, weather, or calendar dependency.

## Production topology

Traffic resolves through Route 53 and CloudFront to an Nginx container on one
Graviton EC2 instance. Nginx rejects requests without CloudFront's private
origin-verification header and proxies accepted traffic to the Next.js
container. PostgreSQL runs in a separate container on the private Docker
network. Images are stored in ECR, runtime secrets in Secrets Manager, and
deployment/backup objects in private S3 buckets. GitHub Actions assumes its AWS
deployment role through OIDC and invokes the instance through Systems Manager;
the instance does not require public SSH for publication.

See `docs/handoff/OPERATIONS.md` and `docs/adr/0001-single-instance-aws-runtime.md`.

## Routes (`app/`)

| Route | Purpose |
|---|---|
| `/` | public Strider landing page |
| `/basecamp` | **Basecamp** — authenticated dashboard: next objective, base-weight bar, quick stats |
| `/trips` · `/trips/[id]` | trip list + full trip view (route map, days, gear, food, permit) |
| `/trips/[id]/print` | printable trip sheet (USGS topo map, per-day distance/gain, permit) |
| `/trips/[id]/permit` | GET handler returning the stored permit blob inline |
| `/share/trips/[token]` | unauthenticated live, view-only trip plan (private notes omitted) |
| `/share/trips/[token]/permit` | token-validated shared permit response |
| `/gear` | **Gear Library** — inventory, loadouts, kits, and the Wishlist tab |
| `/food` | meals, ingredients/pantry, nutrition |
| `/calendar` | planning calendar over iCal feeds |
| `/settings` | report template, master packing list, energy/BMR params |

## Data model (highlights — full detail in `src/db/schema.ts`)

- **Canonical unit is grams (integer)**; oz/lb derived in the UI (`lib/util.ts`).
- Gear: `gear_category` (tree, `sort_order`) → `gear_item` (weight_g per-unit,
  `price_cents`, `quantity`, `default_weight_class` base/worn/consumable,
  `status` current/retired/wishlist, `is_kit`). Kit contents live in
  `gear_component` (a flat list; parent weight = Σ components).
- Loadouts: `loadout` + `loadout_item` (per-loadout base/worn choice). A trip
  copies a loadout as a starting template.
- **Wishlist projection** (recent): `wishlist_replacement` (wishlist item →
  current items it would retire) and `wishlist_loadout` (explicit in/out override
  of a wishlist item's assumed loadout membership). The Wishlist tab projects a
  chosen loadout's base weight *if purchased*, netting replaced items.
- Trips: `trip` (+ per-trip overrides of a linked `trail`), `trip_day`,
  `trip_gear` (snapshotted name/weight/category so completed trips never shift),
  `trip_campsite`, `trip_gpx` (raw + derived `points`), `trip_permit`
  (blob via a `bytea` `customType`), `trip_meal`, `trip_consumable`.
- Food: `ingredient` (per-100g nutrition, density), `meal` + `meal_ingredient`
  (snapshotted nutrition).
- `calendar_feed` (secret iCal URLs), `report_settings` (singleton template +
  packing default), `energy_params` (BMR + calorie-burn tunables).

## `lib/` map

`gear.ts` (gear view/loadouts/wishlist — **server-only, imports pg**),
`categories.ts` (client-safe palette + the `Other` sentinel), `energy.ts`
(calorie model), `food*.ts` / `pantry.ts` (nutrition + USDA/Open Food Facts
search), `trip.ts`, `gpx.ts` (parse GPX → points/profile), `usgs-map.ts` +
`static-map.ts` (server-rendered USGS topo image + SVG overlay for print),
`overpass.ts` (campsite POIs, mirror-raced + localStorage cached), `weather.ts`
(radar/snow tile URLs), `calendar.ts`, `report.ts`, `plan-prompt.ts` (AI
trip-plan prompt), `palette*.ts`, `util.ts` (unit + currency formatters).

## Maps

Leaflet with a `LayersControl`: **USGS Topo** base (default; US-only,
`{z}/{y}/{x}`, maxNativeZoom ~16) + OSM; overlays for RainViewer radar, NASA GIBS
MODIS snow cover, and WAQI air quality (needs `NEXT_PUBLIC_WAQI_TOKEN`). The
print sheet uses the USGS ArcGIS `/export` endpoint to composite one static image
for the bbox, then overlays the track + numbered campsites as SVG.

## External services / secrets (`.env`)

- `DATABASE_URL` → `postgres://backpack:backpack@localhost:5433/backpack`
- `AUTH_URL` — canonical public origin used by Auth.js for authentication redirects
- `AUTH_GOOGLE_ID` / `AUTH_GOOGLE_SECRET` — Google web OAuth client
- `AUTH_OWNER_EMAIL` — verified Google email allowed to claim the legacy data user
- `AUTH_SECRET` or `SESSION_SECRET` — high-entropy Auth.js signing/encryption secret
- `AUTH_E2E_PASSWORD_HASH` — test-only credential for the isolated workflow identity
- `FDC_API_KEY` — USDA FoodData Central (ingredient search)
- `GEOAPIFY_API_KEY` — geocoding / static maps fallback
- `NEXT_PUBLIC_WAQI_TOKEN` — air-quality tiles (baked at build/boot; needs a dev
  restart to change)
- The **GPX routing MCP** (separate project) uses `ORS_API_KEY` (OpenRouteService),
  stored in the MCP registration, not in `.env`.

## Delivery and verification

- `.github/workflows/ci.yml` runs on pull requests and manual dispatch with an
  isolated PostgreSQL service. It migrates/seeds the database, type-checks,
  builds, and runs both Playwright suites.
- `.github/workflows/deploy.yml` runs on every push to `main`. It builds the
  Next.js app, assumes the AWS role, publishes an ARM64 image, updates runtime
  secrets/assets, deploys through SSM, invalidates CloudFront, and validates the
  live site.
- `tests/mobile.spec.ts` checks primary pages at 360, 390, and 430 CSS pixels,
  including overflow, the trip tab strip, dates, maps, and mobile navigation.
- `tests/workflows.spec.ts` checks authentication, cross-user isolation, and a full trip lifecycle:
  create, edit, add a day, print, upload GPX/permit, reload, and delete. Created
  records have unique names and are removed in `finally` cleanup.
