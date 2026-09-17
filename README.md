# backpack-app

Backpacking trip planner + gear/weight tracker. Stores gear, meals, trips,
routes, and per-trip weight/calorie rollups; the frontend is layered on later.

The private hosted instance is published at `https://backpack.thaiv.dev` through
the `main` branch deployment workflow. See `docs/adr/` for the initial hosting
and data-protection decisions.

## Stack

- **PostgreSQL** (via Docker) · **Drizzle ORM** + drizzle-kit migrations · **TypeScript** (tsx)
- Frontend (later phases): Next.js + Tailwind + shadcn/ui.

Canonical weight unit is **grams**; oz/lb are derived in the UI.

## Weight model

Per trip, each selected gear item is `base` (packed) or `worn`; consumables are
computed from the meal plan (food) + per-day water/fuel rates.

```
base_weight       = Σ trip_gear[base]
worn_weight       = Σ trip_gear[worn]
consumable_weight = Σ trip_food + Σ trip_consumable
pack_weight       = base_weight + consumable_weight
skin_out_weight   = pack_weight + worn_weight
```

## Quick start

```bash
npm install
npm run db:up        # start Postgres in Docker (host port 5433)
npm run db:migrate   # apply migrations from drizzle/
npm run import       # load data from ~/backpack/*.html
npm run verify       # print row counts + weight totals
```

`npm run db:studio` opens Drizzle Studio. `npm run db:down` removes the container
(data persists in the `backpack-pgdata` volume).

## Layout

- `src/db/schema.ts` — the schema (source of truth)
- `drizzle/` — generated migrations
- `scripts/import-sheets.ts` — one-shot importer for the legacy Google Sheets exports
- `scripts/verify.ts` — sanity checks against the original sheet totals

## Import notes (Phase 1)

- Per-trip gear **selections** (the sheet's trip checkmark columns) were not
  machine-readable from the HTML export, so trips import without gear; selections
  are entered in the UI (Phase 3).
- Wishlist priority (color-coded in the sheet) is not captured; imported wishlist
  items have no priority yet.
- Calorie-model rows only import for trips whose names match the Hikes sheet.
