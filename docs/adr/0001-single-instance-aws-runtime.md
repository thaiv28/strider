# Use a protected single-instance AWS runtime

Strider will initially run as Dockerized Next.js and PostgreSQL on one Graviton
EC2 instance, with CloudFront and Route 53 at the edge and PostgreSQL data on a
separate encrypted, retained EBS volume. This preserves the existing Postgres,
server-action, binary-upload, and map-rendering behavior while keeping an
idle personal deployment within the platform's small cost envelope; a
serverless rewrite or always-on managed database would add substantial migration
work or baseline cost. The application remains single-user and is protected by
an application login until its ownership checks are ready for multi-user use.

The authentication and single-user portion of this decision is superseded by
ADR 0003; the AWS runtime decision remains current.

## Consequences

The instance is a deliberate availability trade-off: automated encrypted
database backups and a retained data volume provide recovery, but there is no
multi-AZ failover. The app and database can later be split without changing the
domain model or repository boundary.
