# Use Google identity with private per-user workspaces

Strider uses Auth.js with verified Google identity as its public, self-service sign-in method and resolves every private read and mutation through the session's database user id. Existing owner data is claimed in place only when `AUTH_OWNER_EMAIL` matches the first Google sign-in, while unauthenticated sharing remains a separate, read-only, revocable bearer-link capability so sharing one Trip never grants access to its owner's workspace.

## Consequences

Calendar authorization will be requested later as a separate incremental Google consent rather than as part of sign-in. Production workflow tests use a hidden password-protected E2E identity with no access to the owner's data, and every parent or child id supplied by a client must be checked against the session user before it is read or changed.
