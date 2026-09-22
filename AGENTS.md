# Strider agent guide

This repository deploys the private Strider backpacking planner at
`https://backpack.thaiv.dev`. Treat production data and infrastructure as real,
persistent user data.

## Read before changing code

1. `CONTEXT.md` for canonical domain language.
2. `docs/handoff/CURRENT_STATE.md` for the deployed baseline and known limits.
3. `docs/handoff/ARCHITECTURE.md` for system structure.
4. `docs/handoff/CONVENTIONS.md` for coding and verification rules.
5. `docs/handoff/OPERATIONS.md` for deployment, recovery, or AWS work.
6. Relevant records in `docs/adr/` before changing an architectural decision.

The schema, application code, deployment scripts, and GitHub workflows are the
operational source of truth. If they disagree with a handoff document, verify
the behavior and update the document in the same change. Keep `CONTEXT.md` a
domain glossary; implementation decisions belong in handoff docs or an ADR.
Platform-wide AWS provisioning and domain/IAM controls belong to the private
`thaiv28/project-platform-infrastructure` repository, not this application repo.

## Non-negotiable rules

- The app is multi-user. `getCurrentUserId()` resolves the Auth.js session, and
  every private read or mutation must enforce ownership for all supplied ids.
- `/share/trips/[token]` is the deliberate exception: a revocable bearer token
  grants read-only access to one Trip and never exposes private Trip Report notes.
- Grams and integer cents are canonical storage units. Convert only at UI edges.
- Never import server-only database modules as runtime values in client code.
- All writes use server actions and revalidate every affected route. Use a
  transaction for related multi-row writes.
- Change `src/db/schema.ts`, generate and review a migration, then apply it with
  `npm run db:migrate`. Never run `db:push` against production.
- Never commit `.env`, plaintext backups, permits, calendar URLs, API keys,
  OAuth credentials, password hashes, session values, origin secrets, or backup keys.
- Do not delete or recreate AWS resources, production volumes, backups, or
  production records without explicit user approval and a verified target.
- Do not add application-specific copies of centrally managed platform resources
  without first checking `thaiv28/project-platform-infrastructure`.
- Workflow tests create uniquely named production records and must retain their
  `finally` cleanup behavior.

## Required verification

For application changes, run:

```bash
npx tsc --noEmit
npm run build
npm run test:mobile
npm run test:workflow
```

The browser suites require a running app, PostgreSQL, a seeded user, and the
documented test environment variables. If those services are unavailable,
still run type checking/build/test discovery and state exactly what CI must
verify. Pull requests run the complete suite with isolated PostgreSQL. Every
push to `main` deploys and repeats mobile and workflow tests against production.

For documentation-only changes, at minimum run `git diff --check`, verify all
referenced paths and commands, and inspect the rendered Markdown structure.

## Change discipline

- Preserve unrelated user changes in a dirty worktree.
- Prefer small, reversible commits. Never rewrite `main` history.
- Do not log or paste secret values while debugging.
- A code rollback does not reverse database migrations or external side effects.
- Update `docs/handoff/CURRENT_STATE.md` when capabilities, constraints, URLs,
  deployment behavior, or the recommended next step changes.
