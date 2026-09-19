# Strider current state

Last reviewed: 2026-09-19.

## Deployed baseline

- Repository: `thaiv28/backpack-app` (private)
- Production: `https://backpack.thaiv.dev`
- Health: `https://backpack.thaiv.dev/api/health`
- Release branch: `main`
- Publication: automatic GitHub Actions deployment on every push to `main`
- Runtime: ARM64 Next.js and PostgreSQL containers on one Graviton EC2 instance
- Access: shared password gate with a 30-day HTTP-only session cookie
- Data identity: one seeded user selected by `getCurrentUserId()`

The last reviewed deployment completed the public health check, 24 responsive
layout checks, and two authenticated workflow checks. GitHub Actions—not a
developer's local GitHub CLI session—is the durable publication path.

## Working capabilities

- Gear inventory, category ordering, kits, loadouts, wishlist replacements, and
  projected loadout weight.
- Meal/ingredient management, nutrition lookup, trip food planning, and calorie
  estimates.
- Trip creation/import, daily distance/elevation, gear/food snapshots,
  campsites, packing lists, GPX upload, permit upload, and printable trip sheets.
- Interactive route maps with topo/OSM layers and optional weather, snow, and
  air-quality overlays.
- Calendar feeds and configurable report/energy settings.
- Mobile navigation and responsive coverage at 360, 390, and 430 CSS pixels.
- PR validation with isolated PostgreSQL and production post-deploy browser
  verification.
- Encrypted initial seed plus encrypted daily database backups to private S3.

## Source-of-truth map

| Concern | Source of truth |
| --- | --- |
| Domain terms | `CONTEXT.md` |
| Database model | `src/db/schema.ts` and reviewed files in `drizzle/` |
| Application behavior | `app/`, `components/`, `lib/`, and `src/` |
| Build/runtime image | `Dockerfile` and `next.config.mjs` |
| Pull-request verification | `.github/workflows/ci.yml` |
| Production publication | `.github/workflows/deploy.yml` and `ops/deploy-ec2.sh` |
| Shared AWS/domain provisioning | `thaiv28/project-platform-infrastructure` |
| Architecture and conventions | `docs/handoff/ARCHITECTURE.md` and `CONVENTIONS.md` |
| Operations/recovery | `docs/handoff/OPERATIONS.md` and `docs/adr/` |

## Known limitations and risks

- The shared password is an application-wide access gate, not multi-user auth.
  Database identity still means “first users row.”
- The runtime is intentionally single-instance with no multi-AZ failover.
- PostgreSQL and uploaded permit/GPX data are coupled to the instance's retained
  storage and database backups.
- `/api/health` checks only application-to-database connectivity.
- Production tests cover layout and the core trip lifecycle, not every gear,
  food, calendar, external API, or recovery path.
- Workflow tests mutate production briefly. Unique naming and `finally` cleanup
  reduce residue risk but cannot guarantee cleanup during a total outage.
- Database migrations are forward-applied before the app container is replaced.
  Application rollback does not reverse them.
- Deployment validates the new container but does not automatically restore the
  previous image after a post-publication browser-test failure.
- Daily backups are created, but this repository does not automate retention or
  periodic restore drills.
- Every `main` push, including documentation-only changes, invokes the complete
  production deployment.

## Recommended next work

1. Add an automated, isolated backup restore drill and explicit retention policy.
2. Add safe application rollback around container replacement and document the
   migration compatibility window it requires.
3. Expand workflow coverage to representative gear, food, and calendar writes.
4. Add observability beyond the database health check (structured logs, disk and
   backup freshness alarms, and external dependency visibility).
5. If sharing with other people becomes a goal, replace the shared gate with
   real identity and add ownership checks to every mutation before onboarding
   anyone. Treat `docs/going-multiuser.md` as an input to re-evaluate, not an
   implementation-ready plan.

## Handoff rule

Update this document in the same change whenever the production URL, release
branch, runtime topology, authentication model, test gates, backup behavior,
major capability, known limitation, or recommended next step changes.
