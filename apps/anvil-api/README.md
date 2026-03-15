# Anvil API

> **Status:** Beta access system (v1.0)

REST API for Anvil beta access management. Hono on Vercel with Neon Postgres.

## Endpoints

| Method | Path                        | Auth  | Description                |
| ------ | --------------------------- | ----- | -------------------------- |
| GET    | `/api/v1/health`            | None  | Health check               |
| POST   | `/api/v1/waitlist`          | None  | Join waitlist + send email |
| POST   | `/api/v1/waitlist/resend`   | Token | Force re-send confirmation |
| POST   | `/api/v1/auth/verify`       | None  | Validate beta token        |
| POST   | `/api/v1/auth/device/start` | None  | Start device code flow     |
| POST   | `/api/v1/auth/device/confirm` | None | Confirm device code        |
| POST   | `/api/v1/auth/device/poll`  | None  | Poll for confirmation      |
| POST   | `/api/v1/auth/otp/request`  | None  | Request OTP email          |
| POST   | `/api/v1/auth/otp/verify`   | None  | Verify OTP for JWT         |
| POST   | `/api/v1/auth/session/refresh` | None | Refresh JWT token       |
| POST   | `/api/v1/admin/invite`      | Admin | Create user + token        |
| POST   | `/api/v1/admin/approve`     | Admin | Approve waitlist user      |
| POST   | `/api/v1/admin/revoke`      | Admin | Revoke token(s)            |
| GET    | `/api/v1/admin/user/:email` | Admin | Lookup user + tokens       |

> **Deprecation:** `/auth/verify` and `/auth/license/refresh` still work but new
> integrations should use the device code or OTP flows above.

## Environment Variables

| Variable                      | Required | Description                                                             |
| ----------------------------- | -------- | ----------------------------------------------------------------------- |
| `DATABASE_URL`                | Yes      | Neon Postgres connection string                                         |
| `RESEND_API_KEY`              | Yes      | Resend API key for transactional emails                                 |
| `ADMIN_KEY`                   | Yes      | Bearer token for admin endpoints                                        |
| `WAITLIST_RESEND_ADMIN_TOKEN` | Yes      | Token for waitlist resend endpoint                                      |
| `ANVIL_CORS_ORIGINS`          | Yes      | Comma-separated allowed origins                                         |
| `LICENSE_SIGNING_KEY`         | Yes      | ES256 private key (PKCS#8 PEM) for JWTs                                 |
| `TOKEN_PEPPER`                | No       | Extra secret mixed into token hashing                                   |
| `RESEND_WAITLIST_AUDIENCE_ID` | No       | Resend audience ID for waitlist                                         |
| `RESEND_BETA_AUDIENCE_ID`    | No       | Resend audience ID for beta users                                       |
| `ACTIVATE_URL`                | No       | Device code confirmation URL (default: `https://eddacraft.ai/auth/activate`) |

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

## Deployment

Deploy to Vercel with the required environment variables configured.
