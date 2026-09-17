# Going full-stack & multi-user — roadmap

Turning the app from a local single-user tool into a hosted app other people can use.

**Headline:** the app is architecturally close — this is a finishing job, not a rewrite. Every table already has `userId` and read queries filter by it; migrations are set up properly (`drizzle-kit generate`/`migrate` + a real `drizzle/` folder). The real blockers are **auth** and **authorization**.

---

## 1. Real authentication (the #1 blocker)
Today `getCurrentUserId()` (`lib/gear.ts:96`) just returns the **first row in the users table** — one user, no login.

- Add an auth library — **Auth.js (NextAuth v5)** is the standard fit for App Router; **Clerk** / **WorkOS** if you'd rather not run auth yourself.
- Build a sign-up / login flow (email+password, or Google/GitHub OAuth — easiest).
- Rewrite `getCurrentUserId()` to read the **session**, map it to a `users` row (create on first login), return that id. Everything already calls this function, so the change flows through the whole app.

## 2. Authorization on every mutation (the #1 security bug)
Read functions scope by `userId`, but **server actions** (`app/trips/actions.ts`, etc.) take ids from the client — e.g. `updateTrip(tripId, patch)`, `removeGpx(tripId)`, `deleteTrip(tripId)`.

Once multiple users exist, user A can pass user B's `tripId` and edit/delete it. Every action must verify the row belongs to the session user before writing:
- a `WHERE id = ? AND userId = ?` guard, or
- a shared `assertOwnsTrip(tripId, userId)` helper called at the top of each action.

Non-negotiable for multi-user.

## 3. Managed database
Swap local Docker Postgres for hosted:
- **Neon** or **Supabase** (serverless Postgres, good free tiers, pair well with Vercel), or **RDS**.
- Set `DATABASE_URL` as a host secret.
- Run `npm run db:migrate` on deploy — **not** `db:push --force` (can drop data).
- Enable automated backups.

## 4. Hosting
- **Vercel** — least resistance for Next.js: push to GitHub, connect repo, set env vars. HTTPS + domain built in.
  - **Caveat:** the `/trips/[id]/print` map compositing (pngjs/jpeg-js + tile fetches) runs server-side and is slow/memory-hungry — watch function timeout/memory limits.
- **Alternatives** (fewer platform limits): container on **Railway** / **Render** / **Fly.io** (`next start` in Docker), colocated with the DB.

## 5. File uploads → blob storage
Permit/GPX files are stored as `bytea` **inside Postgres** (`schema.ts:267`). Works at small scale but bloats the DB and every backup.

Before real traffic: move uploads to object storage (**S3**, **Vercel Blob**, or **Supabase Storage**); keep only the URL/key in the row.

## 6. Hardening before sharing the URL
- **Input validation** on all server-action inputs — add **Zod** schemas.
- **Rate limiting** on AI/plan endpoints and external fetches (Overpass, weather, USGS tiles, RainViewer) — a public user could run up cost or get the IP banned. **Cache** tiles/weather.
- **Secrets**: move any API keys out of the repo into host env vars; confirm nothing sensitive is committed.
- **Abuse limits**: cap upload size, trips per user, etc.

## 7. Operational basics
- Error monitoring (**Sentry**) + basic logging.
- Analytics / privacy note if collecting emails.
- CI: run `tsc` + build on PRs before deploy.

---

## Suggested order
1. Auth (Auth.js)
2. Authorization guards on every server action
3. Managed DB + `db:migrate` on deploy
4. Deploy to Vercel
5. Blob storage for uploads
6. Validation + rate limiting
7. Monitoring

## Rough effort
- **Auth + authz guards** — the real work; a few focused days if every action is touched carefully.
- **DB + Vercel deploy** — an afternoon.
- **Rest** — incremental.

## First milestone
Wire in Auth.js (#1) + add ownership guards to every server action (#2). Unblocks everything else.
Optional pre-step: audit that lists every server action and whether it currently enforces ownership, to know the exact blast radius.
