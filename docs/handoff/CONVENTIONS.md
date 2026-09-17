# backpack-app — Conventions & gotchas (agent steering doc)

Read alongside `ARCHITECTURE.md`. These are the non-obvious rules that keep
changes correct.

## Units & formatting
- **Grams are canonical** everywhere in the DB and in logic. Never store oz/lb.
- Convert/display only at the edge via `lib/util.ts`: `gToOz`, `ozToG`,
  `fmtWeight` (oz under a lb, else `lb + oz`), `fmtOz`, `fmtLbs`, `fmtUsd`
  (cents → `$`). Prices are stored as **integer cents** (`price_cents`).

## Server / client boundary (this has bitten us)
- `lib/gear.ts` and anything importing `src/db/index` pull in the **`pg`** driver
  and are **server-only**. A `"use client"` component that imports a *value* from
  such a module drags `pg` into the client bundle → `Module not found: 'fs'`.
- In client components import **types only** from `lib/gear` (`import type {...}`),
  and put shared runtime constants in a client-safe module. Example:
  `OTHER_CATEGORY_ID` / `OTHER_CATEGORY_NAME` live in `lib/categories.ts` and are
  re-exported from `lib/gear.ts` for server code.

## Mutations
- All writes are server actions in `app/<area>/actions.ts` (`"use server"`), and
  must `revalidatePath()` every route they affect (`/gear`, `/`, `/trips/[id]`…).
- Multi-row writes use `db.transaction`. The tx type is
  `Parameters<Parameters<typeof db.transaction>[0]>[0]`.
- Client UI updates optimistically (local state) inside a `useTransition`, then
  fires the action; the `revalidatePath` refresh reconciles.

## Schema changes
- Edit `src/db/schema.ts`, then `npm run db:push` (drizzle-kit push). Use
  `--force` for non-interactive column/enum drops. `db:generate`/`db:migrate`
  exist but active dev pushes directly.
- Removing a column/enum: delete it from the schema and push; verify with
  `db:studio` or `verify.ts`. Back up first with `npm run db:backup`.
- Blobs use a `customType<{data: Buffer}>` mapping to `bytea` (see `trip_permit`).

## Domain rules
- **Single user**: `getCurrentUserId()` returns the first `users` row; every
  user-scoped query filters on it. There is no auth.
- Uncategorized gear (`category_id IS NULL`) is surfaced as a synthetic **"Other"**
  group with id `-1`; `moveGear` maps `-1` back to a `null` category.
- `trip_gear`/`trip_meal` snapshot name+weight at plan time so a completed trip's
  totals never change when live gear is re-weighed or retired.
- Weight classes: gear item `default_weight_class` (base/worn/consumable); per
  loadout and per trip a selection is only base/worn (`pack_class`). Consumables
  come from the meal plan + water/fuel rates, not gear selection.

## UI palette (Tailwind v4 tokens)
Utility classes used across the app: `card`, `eyebrow`, `readout`,
`text-ink`/`text-muted`, `bg-panel`/`bg-panel2`, `accent`/`accentink`, and the
weight-class colors `text-cbase` / `text-ccons` / `text-cworn`. Category swatches
come from `categoryColor()` in `lib/categories.ts`.

## Working agreements
- **Iterate with `npm run dev` + `curl localhost:3000`; typecheck with
  `npx tsc --noEmit`.** Both should be clean before declaring done.
- `.env` is gitignored (holds secrets); never commit it. Do not commit unless
  explicitly asked.
- Don't cache weather tiles persistently (staleness); only static base tiles are
  safe to cache.
