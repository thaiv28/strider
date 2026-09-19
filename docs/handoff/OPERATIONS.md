# Strider operations

This is the runbook for the deployed instance at
`https://backpack.thaiv.dev`. It documents observable contracts and safe
recovery boundaries; secret values and account-specific identifiers remain in
GitHub/AWS configuration.

## Service topology

| Layer | Responsibility |
| --- | --- |
| Route 53 | Resolves `backpack.thaiv.dev` to CloudFront. |
| CloudFront | Terminates public HTTPS, supplies the private origin header, and caches safe edge responses. |
| Nginx on EC2 | Rejects requests without the origin header and proxies accepted traffic to `backpack-app:3000`. |
| Next.js container | Serves Strider, enforces the password cookie, and runs server actions. |
| PostgreSQL container | Stores all application data, including GPX and permit blobs. |
| ECR | Stores revision-addressed application images tagged with the Git revision. |
| Secrets Manager | Supplies database, login, session, origin, backup, and server API secrets. |
| Private S3 buckets | Store deployment assets, the encrypted seed, and encrypted daily backups. |
| Systems Manager | Runs the deployment script on the instance without an SSH publication step. |

The single-instance choice and its availability trade-off are recorded in
`docs/adr/0001-single-instance-aws-runtime.md`.

This repository owns the application image and its release procedure. Shared
domain, IAM, and platform provisioning belong to the private
`thaiv28/project-platform-infrastructure` repository. Check that repository
before changing or duplicating centrally managed AWS resources.

## Publication flow

Every push to `main` starts `.github/workflows/deploy.yml`:

1. Install dependencies, type-check, and build Next.js on the GitHub runner.
2. Assume the deployment role using GitHub OIDC.
3. Build and push an ARM64 runtime image tagged with `GITHUB_SHA`.
4. Write the current GitHub secret values into the project's Secrets Manager
   secret and upload the deployment script, Nginx template, and encrypted seed.
5. Invoke `ops/deploy-ec2.sh` through Systems Manager.
6. On the instance, pull the image, ensure PostgreSQL exists, restore the seed
   only when the `users` table is absent, and run committed migrations.
7. Replace the application and proxy containers, then poll the internal health
   endpoint with the origin header.
8. Invalidate CloudFront, verify the public health endpoint, and run all mobile
   and workflow browser tests against production.

Deployments are serialized and are not automatically cancelled. The workflow
has a 35-minute timeout. Documentation-only pushes also deploy because the
workflow currently has no path filter.

## Configuration contract

GitHub Actions expects these repository variables:

- `ECR_URI`
- `DEPLOY_BUCKET`
- `BACKUP_BUCKET`
- `INSTANCE_ID`
- `SECRET_ARN`
- `CLOUDFRONT_DISTRIBUTION_ID`

It expects these GitHub secrets:

- `AWS_ROLE_ARN`
- `APP_PASSWORD_HASH`
- `SESSION_SECRET`
- `DB_PASSWORD`
- `ORIGIN_SECRET`
- `BACKUP_KEY`
- `FDC_API_KEY`
- `GEOAPIFY_API_KEY`
- `NEXT_PUBLIC_WAQI_TOKEN`

The workflow fails before AWS mutation if a required deployment setting is
empty. API keys that represent optional application integrations may be empty.
Do not print values while diagnosing configuration.

## Health and verification

Public health check:

```bash
curl --fail https://backpack.thaiv.dev/api/health
```

A healthy result is `{"ok":true}` and proves the Next.js process can query
PostgreSQL. It does not test every external integration.

Useful repository checks:

```bash
npx tsc --noEmit
npm run build
npm run test:mobile
npm run test:workflow
```

Useful workflow inspection:

```bash
gh run list --repo thaiv28/backpack-app --limit 5
gh run view RUN_ID --repo thaiv28/backpack-app --log-failed
```

The mobile suite runs 24 checks across three phone widths. The workflow suite
runs two checks: unauthenticated redirect behavior and a complete authenticated
trip lifecycle. Its test record is deleted even when the main assertion path
fails, provided the browser can still reach the site.

## Backups and initial restore

The instance installs `/usr/local/bin/backpack-backup` and runs it daily at
03:17 UTC. The job:

1. Executes `pg_dump` inside the database container.
2. Compresses the SQL stream with gzip.
3. Encrypts it with AES-256-CBC/PBKDF2 and a salted payload.
4. Uploads it under `daily/` in the private backup bucket.

The repository contains only an encrypted initial seed at
`ops/seed/backpack.sql.enc`; its design is recorded in
`docs/adr/0002-encrypted-database-seed.md`. On a new instance, the deployment
script restores that seed only if the public `users` table does not exist.

Before any production restore:

- Identify the exact backup object and timestamp.
- Preserve the current `/srv/backpack/postgres` data or take a fresh backup.
- Prove decryption and restore against an isolated PostgreSQL instance first.
- Confirm whether migrations must run after the selected backup.
- Obtain explicit approval before replacing or modifying live data.

Backup creation is automated; restore testing and retention enforcement are not
currently performed by this repository.

## Failure triage

| Failed stage | First checks |
| --- | --- |
| Configuration | Confirm the named GitHub variables/secrets exist; never reveal their values. |
| Type-check/build | Reproduce with the pinned Node major and `npm ci`; inspect the first compiler error. |
| AWS credentials | Check OIDC trust, repository/branch conditions, role ARN, and region. |
| Image publication | Inspect Buildx cache/output and ECR permissions; the target must be `linux/arm64`. |
| SSM deployment | Inspect the command invocation output; check instance online status, disk, Docker, secret/S3/ECR access, and migration output. |
| Internal health | Inspect `docker logs --tail 100 backpack-app`, then database readiness and `runtime.env` presence/permissions without printing it. |
| Public health | Compare internal health with CloudFront origin/header behavior and invalidation state. |
| Browser tests | Download diagnostics, inspect the first failure, and check for an `E2E` trip residue before manually deleting only the exact test record. |

## Rollback boundary

There is no automatic application rollback in the deployment script. Images are
tagged by Git revision, but the normal recovery path is to revert the faulty
commit on `main`, which publishes a new revision through the same workflow.

A code rollback does not reverse:

- Drizzle migrations already committed to PostgreSQL.
- Records or uploaded files created by user activity or tests.
- Secret versions written during deployment.
- External/cache side effects.

Treat a migration failure or suspected data loss as an incident: stop repeated
deployments, preserve the volume and logs, validate a backup in isolation, and
choose a forward fix or approved restore deliberately.

## Security notes

- CloudFront-to-origin traffic is authorized by `X-Origin-Verify`; direct
  requests to Nginx without the value receive 403.
- The application password is checked using a timing-safe comparison of SHA-256
  bytes. The resulting session cookie is a bearer secret and lasts 30 days.
- The password gate protects one owner's application. It is not sufficient for
  multiple independent users or row-level authorization.
- GitHub OIDC drives deployment. Local GitHub CLI authorization is unrelated and
  may expire without affecting an already configured Actions deployment.
