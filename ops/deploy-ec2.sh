#!/usr/bin/env bash
set -euo pipefail

: "${ECR_URI:?ECR_URI is required}"
: "${DEPLOY_BUCKET:?DEPLOY_BUCKET is required}"
: "${BACKUP_BUCKET:?BACKUP_BUCKET is required}"
: "${SECRET_ARN:?SECRET_ARN is required}"
: "${IMAGE_TAG:?IMAGE_TAG is required}"

install -d -m 700 /srv/backpack
secret_json="$(aws secretsmanager get-secret-value --secret-id "$SECRET_ARN" --query SecretString --output text)"
db_password="$(jq -r .DB_PASSWORD <<<"$secret_json")"
session_secret="$(jq -r .SESSION_SECRET <<<"$secret_json")"
auth_google_id="$(jq -r .AUTH_GOOGLE_ID <<<"$secret_json")"
auth_google_secret="$(jq -r .AUTH_GOOGLE_SECRET <<<"$secret_json")"
auth_owner_email="$(jq -r .AUTH_OWNER_EMAIL <<<"$secret_json")"
auth_e2e_password_hash="$(jq -r .AUTH_E2E_PASSWORD_HASH <<<"$secret_json")"
origin_secret="$(jq -r .ORIGIN_SECRET <<<"$secret_json")"
backup_key="$(jq -r .BACKUP_KEY <<<"$secret_json")"
fdc_api_key="$(jq -r .FDC_API_KEY <<<"$secret_json")"
geoapify_api_key="$(jq -r .GEOAPIFY_API_KEY <<<"$secret_json")"

umask 077
cat > /srv/backpack/runtime.env <<EOF
DATABASE_URL=postgres://backpack:${db_password}@backpack-db:5432/backpack
SESSION_SECRET=${session_secret}
AUTH_GOOGLE_ID=${auth_google_id}
AUTH_GOOGLE_SECRET=${auth_google_secret}
AUTH_OWNER_EMAIL=${auth_owner_email}
AUTH_E2E_PASSWORD_HASH=${auth_e2e_password_hash}
AUTH_URL=https://backpack.thaiv.dev
FDC_API_KEY=${fdc_api_key}
GEOAPIFY_API_KEY=${geoapify_api_key}
EOF

registry="${ECR_URI%%/*}"
aws ecr get-login-password | docker login --username AWS --password-stdin "$registry" >/dev/null
image="${ECR_URI}:${IMAGE_TAG}"
# Keep the small single-instance host from accumulating every historical image.
# Running containers retain their image; only unreferenced layers are removed.
docker image prune --all --force >/dev/null
docker pull "$image"
docker network inspect backpack-net >/dev/null 2>&1 || docker network create backpack-net >/dev/null

if ! docker inspect backpack-db >/dev/null 2>&1; then
  docker run -d \
    --name backpack-db \
    --network backpack-net \
    --restart unless-stopped \
    --memory 256m \
    -e POSTGRES_USER=backpack \
    -e POSTGRES_PASSWORD="$db_password" \
    -e POSTGRES_DB=backpack \
    -v /srv/backpack/postgres:/var/lib/postgresql/data \
    postgres:16-alpine \
    -c shared_buffers=64MB \
    -c max_connections=20 >/dev/null
fi

for _ in $(seq 1 60); do
  docker exec backpack-db pg_isready -U backpack -d backpack >/dev/null 2>&1 && break
  sleep 2
done
docker exec backpack-db pg_isready -U backpack -d backpack >/dev/null

if ! docker exec backpack-db psql -U backpack -d backpack -Atqc "select 1 from pg_tables where schemaname='public' and tablename='users'" | grep -qx 1; then
  aws s3 cp "s3://${DEPLOY_BUCKET}/seed/backpack.sql.enc" - \
    | openssl enc -d -aes-256-cbc -pbkdf2 -pass "pass:${backup_key}" \
    | docker exec -i backpack-db psql -v ON_ERROR_STOP=1 -U backpack -d backpack
fi

# Every deployment gets an encrypted restore point immediately before migrations.
# This is especially important for identity migrations: linking an owner happens
# in place, but the pre-migration backup makes the database state recoverable.
pre_migration_stamp="$(date -u +%Y%m%dT%H%M%SZ)"
docker exec backpack-db pg_dump -U backpack -d backpack \
  | gzip \
  | openssl enc -aes-256-cbc -pbkdf2 -salt -pass "pass:${backup_key}" \
  | aws s3 cp - "s3://${BACKUP_BUCKET}/pre-migration/backpack-${pre_migration_stamp}.sql.gz.enc"

docker run --rm \
  --network backpack-net \
  --env-file /srv/backpack/runtime.env \
  "$image" npm run db:migrate

docker rm -f backpack-app >/dev/null 2>&1 || true
docker run -d \
  --name backpack-app \
  --network backpack-net \
  --restart unless-stopped \
  --memory 512m \
  --env-file /srv/backpack/runtime.env \
  "$image" >/dev/null

aws s3 cp "s3://${DEPLOY_BUCKET}/deploy/nginx.conf.template" /srv/backpack/nginx.conf.template
sed "s/__ORIGIN_SECRET__/${origin_secret}/g" /srv/backpack/nginx.conf.template > /srv/backpack/nginx.conf
docker rm -f backpack-proxy >/dev/null 2>&1 || true
docker run -d \
  --name backpack-proxy \
  --network backpack-net \
  --restart unless-stopped \
  --memory 64m \
  -p 80:80 \
  -v /srv/backpack/nginx.conf:/etc/nginx/nginx.conf:ro \
  nginx:1.27-alpine >/dev/null

cat > /usr/local/bin/backpack-backup <<EOF
#!/usr/bin/env bash
set -euo pipefail
stamp=\"\$(date -u +%Y%m%dT%H%M%SZ)\"
docker exec backpack-db pg_dump -U backpack -d backpack \\
  | gzip \\
  | openssl enc -aes-256-cbc -pbkdf2 -salt -pass 'pass:${backup_key}' \\
  | aws s3 cp - \"s3://${BACKUP_BUCKET}/daily/backpack-\${stamp}.sql.gz.enc\"
EOF
chmod 700 /usr/local/bin/backpack-backup
printf '17 3 * * * root /usr/local/bin/backpack-backup\n' > /etc/cron.d/backpack-backup
chmod 600 /etc/cron.d/backpack-backup
systemctl enable --now crond >/dev/null

for _ in $(seq 1 60); do
  if curl --fail --silent -H "X-Origin-Verify: ${origin_secret}" http://127.0.0.1/api/health >/dev/null; then
    exit 0
  fi
  sleep 2
done
docker logs --tail 100 backpack-app >&2
exit 1
