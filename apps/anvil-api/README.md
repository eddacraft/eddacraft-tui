# Anvil API

> **Status:** Beta access system (v1.0)

REST API for Anvil beta access management. Hono on Vercel with Neon Postgres.

## Endpoints

| Method | Path                           | Auth  | Description                        |
| ------ | ------------------------------ | ----- | ---------------------------------- |
| GET    | `/api/v1/health`               | None  | Health check                       |
| POST   | `/api/v1/waitlist`             | None  | Join waitlist + send email         |
| POST   | `/api/v1/waitlist/resend`      | Token | Force re-send confirmation         |
| POST   | `/api/v1/auth/verify`          | None  | Validate beta token                |
| POST   | `/api/v1/auth/device/start`    | None  | Start device code flow             |
| POST   | `/api/v1/auth/device/poll`     | None  | Poll for confirmation              |
| POST   | `/api/v1/auth/otp/request`     | None  | Request OTP email                  |
| POST   | `/api/v1/auth/otp/verify`      | None  | Verify OTP for JWT                 |
| POST   | `/api/v1/auth/session/refresh` | None  | Refresh JWT token                  |
| POST   | `/api/v1/admin/invite`         | Admin | Create user + token                |
| POST   | `/api/v1/admin/approve`        | Admin | Approve waitlist user              |
| POST   | `/api/v1/admin/revoke`         | Admin | Revoke token(s)                    |
| GET    | `/api/v1/admin/user/:email`    | Admin | Lookup user + tokens               |
| POST   | `/api/v1/admin/broadcast`      | Admin | Preview/send broadcast mail        |
| POST   | `/api/v1/admin/send-migration` | Admin | Migration-mail shim over broadcast |

> **Deprecation:** `/auth/verify` and `/auth/license/refresh` still work but new
> integrations should use the device code or OTP flows above.

### `POST /admin/broadcast`

Two-step, snapshot-backed mail-to-many. A dry-run resolves the audience, records
a single-use preview snapshot, and returns an opaque `previewToken`:

```jsonc
// dry-run — template + audience are required and seed the snapshot
{
  "template": "release-announcement",
  "audience": "beta:active",
  "dryRun": true,
}
```

The real-send consumes that snapshot atomically and treats it as the source of
truth for `template`, `templateProps`, `audience`, and `audienceParams`. Only
`previewToken` is required — request-time `template` / `audience` /
`templateProps` are **ignored** on the real-send leg (anti-bait-and-switch), so
a preview-token-only body is accepted:

```jsonc
// real-send — token-only; the snapshot drives template + audience
{ "dryRun": false, "previewToken": "<token from dry-run>" }
```

If the audience re-resolves to a different recipient set than the snapshot, the
send is rejected with `409 cohort_drift` and the operator must re-preview.

`POST /admin/send-migration` is a thin back-compat shim that maps a `source` to
the equivalent broadcast call (`template: waitlist-migration`,
`audience: waitlist:source`) and preserves the legacy response shape.

## Environment Variables

| Variable                      | Required | Description                                                                  |
| ----------------------------- | -------- | ---------------------------------------------------------------------------- |
| `DATABASE_URL`                | Yes      | Neon Postgres connection string                                              |
| `RESEND_API_KEY`              | Yes      | Resend API key for transactional emails                                      |
| `ADMIN_KEY`                   | Yes      | Shared admin bearer token (legacy fallback when per-operator mode is off)    |
| `ADMIN_PER_OPERATOR_KEYS`     | No       | Set to `1` to enable per-operator admin key resolution                       |
| `ADMIN_KEY_PEPPER`            | If above | Non-empty pepper string (recommended: 32-byte hex) for per-operator lookup   |
| `WAITLIST_RESEND_ADMIN_TOKEN` | Yes      | Token for waitlist resend endpoint                                           |
| `ANVIL_CORS_ORIGINS`          | Yes      | Comma-separated allowed origins                                              |
| `LICENSE_SIGNING_KEY`         | Yes      | ES256 private key (PKCS#8 PEM) for JWTs                                      |
| `TOKEN_PEPPER`                | No       | Extra secret mixed into token hashing                                        |
| `RESEND_WAITLIST_AUDIENCE_ID` | No       | Resend audience ID for waitlist                                              |
| `RESEND_BETA_AUDIENCE_ID`     | No       | Resend audience ID for beta users                                            |
| `CRON_SECRET`                 | Yes      | Bearer token for cron endpoint authentication                                |
| `ACTIVATE_URL`                | No       | Device code confirmation URL (default: `https://eddacraft.ai/auth/activate`) |

### Per-Operator Admin Keys

When `ADMIN_PER_OPERATOR_KEYS=1` is set together with a non-empty
`ADMIN_KEY_PEPPER`, the admin middleware authenticates each request via a
peppered-hash lookup of per-operator credentials provisioned outside the shared
admin key. If `ADMIN_PER_OPERATOR_KEYS=1` is set without `ADMIN_KEY_PEPPER`, the
middleware falls back to the legacy shared-key auth and logs an error
server-side; CLI requests will not see the misconfiguration directly. Provision
both via your secret manager (Pulumi handles this for the EddaCraft-managed
deployment).

### CORS and Vercel hardening (post-0.5.0-beta deploy)

The post-release Vercel/CORS hardening lowered the CORS preflight cache
lifetime, restored the Hono/Vercel entrypoint after the post-tag deploy break,
scoped the API tsconfig, controlled Nx framework detection, and added the
`svix>uuid` runtime override exception so production deploys do not trip on
dependency drift. Operators upgrading their `anvil-api` deployment for the
0.5.0-beta release should redeploy from the current `dev` or `main` branch
rather than cherry-picking individual fixes.

## Development

```bash
# Install dependencies
pnpm install

# Run tests
pnpm -F @eddacraft/anvil-api test

# Build
pnpm -F @eddacraft/anvil-api build

# Type check
pnpm -F @eddacraft/anvil-api typecheck
```

## Database Setup

Run `src/db/schema.sql` against your Neon Postgres database to create the
required tables (`beta_users`, `access_tokens`, `audit_log`, `device_codes`,
`otp_codes`, `refresh_tokens`).

### SQL Migration Runner (0.5.0-beta)

`anvil-api` ships a first-party SQL migration runner that drives schema changes
on every deploy. It supports:

- **Dry-run mode** — preview every statement that would run without applying
  anything, surfaced through the deploy workflow before Pulumi Up.
- **Drift detection** — compare the migrations table against the filesystem
  manifest and fail the deploy if a checked-in migration is missing from the
  database, or vice versa.
- **Manual runbook** — operator instructions for re-running the migration step
  if a deploy stops between Pulumi Up and the application rollout.

See the migration runbook under `docs/runbooks/db-migrations.md` for the
operator workflow. Migrations are idempotent and safe to re-run.

## Deployment

Deploy to Vercel with the required environment variables configured.
