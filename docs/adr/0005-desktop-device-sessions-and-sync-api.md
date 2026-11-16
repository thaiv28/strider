# Use device sessions and a versioned row sync API for Strider Desktop

Strider Desktop uses device bearer tokens as a second authenticated entry point
into the same private workspace. The browser remains the sign-in authority:
`/desktop/authorize` uses the existing Auth.js session, explicit consent, an RFC
8252 loopback callback restricted to `127.0.0.1`, and PKCE S256. Authorization
codes expire after 60 seconds and are consumed once. Only SHA-256 hashes of
codes and device tokens are stored. Users can revoke individual devices from
Settings; device authentication never grants ownership of another workspace.

`/api/v1/*` requires a device token and a supported `X-Strider-App-Version`;
responses are never cached. The token exchange is the sole token-free API
operation. Device writes use the same ownership and active trip-edit grants as
web actions. Shared trip rows exclude private Trip Report notes and link tokens.
Permit bytes use a separate endpoint, with a SHA-256 digest in synced metadata.

Sync operates on rows with stable UUID `sync_id` identities and a global bigint
`server_seq`. PostgreSQL triggers allocate sequence values for web and device
writes alike. Pull merges visible rows and deletion tombstones by sequence;
newly visible shared trips include their full contents, and revoked visits
remove access on the next pull. Push applies up to 500 operations atomically,
records results by user and batch UUID for retries, adopts supported duplicate
memberships, and returns previous long-text values when overwriting a conflict.
The server remains authoritative for ownership, permissions, copied library
values, kit weights, recipe nutrition, and trip food totals.

## Consequences

Every synced insert, meaningful update, and recorded deletion takes the same
transaction-scoped PostgreSQL advisory lock before allocating its sequence.
This globally serializes synced writes across users until commit, so a committed
pull cursor cannot skip a lower sequence still awaiting commit. It is a known
throughput limit, and transactions should remain short. Sequence gaps after
rollback are harmless. Cascaded child deletes do not need tombstones when their
parent has disappeared: the device performs the same cascade locally.

Every new synced table requires UUID/sequence columns and indexes, both trigger
attachments, a sync registry entry, and explicit push authorization and
validation rules. SQL trigger bodies are reviewed additions to the generated
Drizzle migration; they are not represented by the Drizzle snapshot. Database
migration and API integration tests must pass against local PostgreSQL before
this branch is reviewed and deployed. This decision does not mark the desktop
sync API as deployed.
