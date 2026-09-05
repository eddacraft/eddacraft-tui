# anvil API architecture

| Type         | Authority     | Owner | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                    |
| ------------ | ------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architecture | Authoritative | APGOV | Live   | Last reviewed 2026-09-05 for SEC-013 shared licence-auth enforcement; authenticated-route boundary and Neon data flow unchanged. Prior review 2026-09-03 for CLAWOPEN-011's Neon integration harness and CLAWOPEN-007's generator atomic-output change; `apps/anvil-api` gained test files only and no production route, contract, or topology moved, so the diagrams stand. |

| Upstream                                                                                    | Downstream                                       |
| ------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `apps/anvil-api/src/**`, ADR-066, ADR-123, and BAUTH's `docs/architecture/auth-as-built.md` | API maintainers, CLI, docs shell, and operations |

This document is the live APGOV component authority. The former central
[API as-built](../../docs/architecture/api-as-built.md) is retained as a dated
compatibility and history record. BAUTH's
[auth as-built](../../docs/architecture/auth-as-built.md) remains authoritative
for authentication and authorisation.

## Scope and boundaries

The service owns HTTP composition, request validation, route orchestration, and
persistence calls for the hosted operator plane. [`index.ts`](src/index.ts)
applies logging, CORS, trace context, and rate limiting before dispatching to
[`routes/`](src/routes). Routes use [`db/client.ts`](src/db/client.ts) and
[`db/queries.ts`](src/db/queries.ts) for Neon persistence, plus bounded external
provider adapters for email and GitHub flows.

## Request-to-persistence flow

This diagram owns the API's request, trust, and persistence concern.

```mermaid
flowchart LR
    Request[HTTP request] --> Shared[logging, CORS, trace, rate limit]
    Shared --> Trust{route trust boundary}
    Trust -->|public| Public[public route]
    Trust -->|session or token| Auth[authenticated route]
    Trust -->|operator or cron secret| Privileged[privileged route]
    Public --> Handler[validated route handler]
    Auth --> Handler
    Privileged --> Handler
    Handler --> Store[Neon persistence]
    Handler --> Provider[email or GitHub provider]
    Store --> Response[structured response]
    Provider --> Response
```

Global middleware traces to [`index.ts`](src/index.ts) and
[`middleware/`](src/middleware). Trust branches trace to
[`routes/auth.ts`](src/routes/auth.ts),
[`routes/account-activity.ts`](src/routes/account-activity.ts),
[`routes/admin.ts`](src/routes/admin.ts), and
[`routes/cron.ts`](src/routes/cron.ts). Persistence traces to
[`db/client.ts`](src/db/client.ts) and [`db/queries.ts`](src/db/queries.ts). In
prose: every request crosses the shared middleware chain, then a route's public,
authenticated, operator, or cron boundary. The validated handler calls Neon or
an external provider and returns a structured result.

The licence branch uses
[`middleware/licence-auth.ts`](src/middleware/licence-auth.ts) for signature
verification and a fresh active-account lookup before returning an authenticated
identity.
[`auth-as-built.md`](../../docs/architecture/auth-as-built.md#online-licence-route-inventory-sec-013)
owns the route inventory and denial contracts. Activity payload validation
continues to reject before database access; authenticated persistence remains
best-effort. The diagram's existing authenticated-route boundary covers this
shared enforcement without adding a deployment or transport edge.

## Composition and source map

[`index.ts`](src/index.ts) mounts the `/api/v1` application. Its global
middleware order is logger, configured-origin CORS, trace context, then the
shared rate limiter; the health route and route modules execute behind that
chain. Boot probes signing and verifying keys, GitHub CLI OAuth credentials, and
Resend credentials without preventing the process from starting. The `/health`
response reports those states alongside database reachability.

Route ownership is split by trust and job:

- [`routes/auth.ts`](src/routes/auth.ts),
  [`routes/auth-device.ts`](src/routes/auth-device.ts),
  [`routes/auth-otp.ts`](src/routes/auth-otp.ts),
  [`routes/auth-session.ts`](src/routes/auth-session.ts),
  [`routes/auth-github.ts`](src/routes/auth-github.ts), and
  [`routes/auth-github-device.ts`](src/routes/auth-github-device.ts) implement
  the BAUTH-owned access flows.
- [`routes/admin.ts`](src/routes/admin.ts) and
  [`routes/admin-schemas.ts`](src/routes/admin-schemas.ts) own operator
  orchestration and validated request shapes; authentication and actor-scoped
  limiting live in [`middleware/admin-auth.ts`](src/middleware/admin-auth.ts)
  and [`middleware/admin-rate-limit.ts`](src/middleware/admin-rate-limit.ts).
- [`routes/waitlist.ts`](src/routes/waitlist.ts),
  [`routes/telemetry.ts`](src/routes/telemetry.ts),
  [`routes/account-activity.ts`](src/routes/account-activity.ts), and
  [`routes/cron.ts`](src/routes/cron.ts) own their bounded public,
  identity-bound, and scheduled surfaces.

[`db/client.ts`](src/db/client.ts) owns Neon client construction and test
replacement. [`db/queries.ts`](src/db/queries.ts) owns validated persistence
operations. [`db/migrate.ts`](src/db/migrate.ts) discovers ordered SQL files,
checks stored hashes for drift, serialises real runs with a PostgreSQL advisory
lock, and applies each migration transactionally with its tracking row. The
[database migration runbook](../../docs/runbooks/db-migrations.md) remains the
operator authority.

## Invariants, failure, and fallback

- CORS admits only configured origins; it is not an authentication mechanism.
- Trace context and rate limiting apply before every versioned route.
- Route-specific authentication must follow BAUTH's retained
  [auth authority](../../docs/architecture/auth-as-built.md).
- [`admin-auth.ts`](src/middleware/admin-auth.ts) prefers per-operator keys when
  configured. A database lookup failure deliberately falls back to the shared
  admin key. A revoked, malformed, or unknown credential is rejected; its audit
  write is attempted on a best-effort basis, and audit-write failure is logged
  without masking the rejection.
- Health reports database, signing-key, verifying-key, GitHub CLI credential,
  and Resend states explicitly. Database failure, unavailable signing or
  verifying keys, unavailable GitHub CLI credentials, and Resend
  `invalid`/`unconfigured` states gate overall health. A Resend or network probe
  result of `unverifiable` is explicit but non-gating, so it can coexist with
  overall `status: ok`.
- Authenticated account-activity ingest
  ([`account-activity.ts`](src/routes/account-activity.ts)) is fire-and-forget
  after a valid licence JWT, an allowlisted JSON payload, and an active
  `beta_users` subject, in that order. Payload 4xx never touch the database.
  Missing or non-active subjects then receive 401 and no row mutations
  (CIB-399). A user-lookup failure after a valid payload returns 503. A Neon
  connect or write failure after the subject is proven active is logged and
  still returns 202; it must not 500. Auth and payload errors remain 4xx. The
  Neon client prefers IPv4 DNS results to avoid Happy Eyeballs connect timeouts
  from Vercel.
- Persistence migrations remain governed by the
  [database migration runbook](../../docs/runbooks/db-migrations.md), not this
  component map.
