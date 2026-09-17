# backpack-app — Architecture (agent steering doc)

A personal **backpacking trip planner + gear/weight tracker**. Single-user (no
auth): every query resolves the current user via `getCurrentUserId()`, which
returns the first `users` row. Written to be driven by an AI agent as much as a
human, so favor small, composable server actions and keep the schema the source
of truth.

## Stack

- **Next.js 15** (App Router) · **React 19** · **TypeScript 5.7** · ESM (`"type":"module"`).
- **Postgres 16** in Docker (host port **5433** → container 5432), data in the
  named volume `backpack-pgdata`.
- **Drizzle ORM 0.36** + **drizzle-kit 0.28**. Schema at `src/db/schema.ts` is
  the single source of truth; iterate with `npm run db:push` (schema is pushed,
  not migration-per-change, during active dev).
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

## Routes (`app/`)

| Route | Purpose |
|---|---|
| `/` | **Basecamp** — dashboard: next objective, base-weight bar, quick stats |
| `/trips` · `/trips/[id]` | trip list + full trip view (route map, days, gear, food, permit) |
| `/trips/[id]/print` | printable trip sheet (USGS topo map, per-day distance/gain, permit) |
| `/trips/[id]/permit` | GET handler returning the stored permit blob inline |
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
- `FDC_API_KEY` — USDA FoodData Central (ingredient search)
- `GEOAPIFY_API_KEY` — geocoding / static maps fallback
- `NEXT_PUBLIC_WAQI_TOKEN` — air-quality tiles (baked at build/boot; needs a dev
  restart to change)
- The **GPX routing MCP** (separate project) uses `ORS_API_KEY` (OpenRouteService),
  stored in the MCP registration, not in `.env`.
