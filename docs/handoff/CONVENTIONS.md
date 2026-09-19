# backpack-app — Conventions and gotchas

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
- Edit `src/db/schema.ts`, run `npm run db:generate`, and review the generated
  SQL before committing it. Apply committed migrations with
  `npm run db:migrate`.
- `npm run db:push` is only for disposable local exploration. Never use it on
  production or as a substitute for a reviewed migration.
- Treat destructive schema changes as data migrations: take/verify a backup,
  preserve compatible application behavior during rollout, and document the
  irreversible part. Do not assume reverting application code reverses a
  migration.
- Blobs use a `customType<{data: Buffer}>` mapping to `bytea` (see `trip_permit`).

## Domain rules
- **Single data user**: `getCurrentUserId()` returns the first `users` row and
  user-scoped queries filter on it. The shared password/session middleware is
  an access gate, not per-user authentication or authorization.
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
- Iterate with `npm run dev` and the local health endpoint. Type-check with
  `npx tsc --noEmit` and create a production build with `npm run build`.
- Run `npm run test:mobile` for responsive changes and
  `npm run test:workflow` for authenticated/data-flow changes. Run both for
  shared layout, routing, deployment, database, or authentication changes.
- Browser tests need a running app, a migrated database, and a seeded user. Use
  the environment contract in `playwright.config.ts` and the GitHub workflows;
  never put a real production secret into source or shell history.
- Pull requests must pass isolated CI. A push to `main` publishes to production
  and then runs both browser suites against `https://backpack.thaiv.dev`.
- `.env` is gitignored (holds secrets); never commit it. Do not commit unless
  explicitly asked.
- Don't cache weather tiles persistently (staleness); only static base tiles are
  safe to cache.
- Keep production workflow records uniquely named and preserve cleanup in a
  `finally` block so tests do not pollute Thai's trip data.
- `docs/going-multiuser.md` is an exploratory roadmap, not the current runtime
  contract. Recheck it against the deployed architecture before implementing it.

## Authentication and security boundary

- `middleware.ts` is the route gate. Keep `/api/health` public for infrastructure
  health checks. `/share/trips/[token]` and its permit handler are deliberately
  public bearer-link routes: validate the token on every request, keep them
  read-only, and never include `trip_report` private notes.
- `APP_PASSWORD_HASH` is a SHA-256 digest, while `SESSION_SECRET` is the cookie
  value. Neither belongs in the repository or logs.
- Authentication tests may inject the session cookie from CI secrets. This is a
  test mechanism, not a second application authentication path.
- The origin-verification header protects direct EC2 access. Preserve it in
  CloudFront and `ops/nginx.conf.template` together.

## Deployment boundary

- GitHub OIDC credentials are used only by Actions. A developer's `gh` login is
  for repository operations and is not part of production authentication.
- Runtime deployment is defined by `.github/workflows/deploy.yml`, `Dockerfile`,
  and `ops/deploy-ec2.sh`; update `docs/handoff/OPERATIONS.md` when this flow
  changes.
- ARM64 is the runtime target. The Docker build stage runs on the native builder
  platform; only the production runtime layer is assembled for ARM64.
- The runtime image installs production dependencies only. Do not reintroduce a
  runtime dependency on TypeScript-only configuration or development packages.
