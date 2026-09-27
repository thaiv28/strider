# Strider

Backpacking trip planner + gear/weight tracker. Stores gear, meals, trips,
routes, and per-trip weight/calorie rollups in a private workspace for each
verified Google account.

The hosted app is published at `https://strider.thaiv.dev` through
the `main` branch deployment workflow. See `docs/adr/` for the initial hosting
and data-protection decisions.

`https://backpack.thaiv.dev` remains an alias during the hostname transition.

Agents and new contributors should start with `AGENTS.md`, then read the current
handoff in `docs/handoff/CURRENT_STATE.md`. Architecture, conventions, and the
production runbook are maintained beside it in `docs/handoff/`.

## Stack

- **PostgreSQL** (via Docker) · **Drizzle ORM** + drizzle-kit migrations · **TypeScript** (tsx)
- **Next.js** · **Auth.js** with Google OAuth · **Tailwind CSS**

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

Copy `.env.example` to `.env.local`, configure a Google OAuth web client, and
set `AUTH_GOOGLE_ID`, `AUTH_GOOGLE_SECRET`, and `AUTH_SECRET`. For an existing
single-user database, set `AUTH_OWNER_EMAIL` to the exact verified Google email
that should claim the existing user row. The claim happens in place, preserving
the user id and all trip, gear, meal, and settings relationships.

`npm run db:studio` opens Drizzle Studio. `npm run db:down` removes the container
(data persists in the `backpack-pgdata` volume).

## Verification

```bash
npx tsc --noEmit       # type safety
npm run build          # production Next.js build
npm run test:mobile    # responsive layout checks at three phone widths
npm run test:workflow  # authenticated trip create/edit/upload/print/delete flow
```

Pull requests run these checks against an isolated Postgres service, including
cross-account isolation. Deployments repeat both browser suites against
`strider.thaiv.dev`; workflow-created records use isolated test identities,
unique names, and test cleanup.

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
