# Beta Access — Module Specification

## Scope

Lightweight invite-only beta access system. Admin generates tokens, distributes
them to users. CLI validates tokens against a Vercel-hosted API backed by Neon
Postgres. Unauthenticated users are blocked from all commands except `login`,
`logout`, `whoami`, `beta`, `start`, `--help`, `--version`.

## Architecture

```
┌─────────────┐     POST /api/v1/auth/verify     ┌──────────────┐     ┌────────────┐
│  anvil CLI   │ ──────────────────────────────▶  │  anvil-api   │ ──▶ │   Neon DB   │
│  (user)      │ ◀──────────────────────────────  │  (Hono)      │ ◀── │  (Postgres) │
└─────────────┘     {valid, user, scopes}         └──────────────┘     └────────────┘
                                                        ▲
┌─────────────┐     POST /api/v1/admin/invite           │
│  anvil CLI   │ ───────────────────────────────────────┘
│  (admin)     │     Authorization: Bearer <ADMIN_KEY>
└─────────────┘
```

## Components

### API (`apps/anvil-api/`)

- **Framework:** Hono on Vercel
- **Database:** Neon Postgres via `@neondatabase/serverless`
- **Validation:** Zod schemas via `@hono/zod-validator`

### CLI Auth (`apps/anvil-cli/`)

- **Auth store:** `~/.anvil/auth.json` (mode 0o600)
- **Auth gate:** Commander.js `preAction` hook
- **Admin commands:** `anvil beta invite/revoke`

## Endpoints

| Method | Path                    | Auth   | Description              |
| ------ | ----------------------- | ------ | ------------------------ |
| GET    | `/api/v1/health`        | None   | Health check             |
| POST   | `/api/v1/auth/verify`   | None   | Validate beta token      |
| POST   | `/api/v1/admin/invite`  | Admin  | Create user + token      |
| POST   | `/api/v1/admin/revoke`  | Admin  | Revoke token(s)          |
| GET    | `/api/v1/admin/user/:email` | Admin | Lookup user + tokens |

## Database Schema

### `beta_users`

| Column     | Type                     | Notes                     |
| ---------- | ------------------------ | ------------------------- |
| id         | uuid (PK)                | gen_random_uuid()         |
| email      | citext UNIQUE NOT NULL   | Normalised email          |
| name       | text                     | Optional display name     |
| status     | text DEFAULT 'active'    | active, suspended, banned |
| notes      | text                     | Internal notes            |
| created_at | timestamptz              | DEFAULT now()             |
| updated_at | timestamptz              | Trigger-maintained        |

### `access_tokens`

| Column     | Type                     | Notes                     |
| ---------- | ------------------------ | ------------------------- |
| id         | uuid (PK)                | gen_random_uuid()         |
| user_id    | uuid FK → beta_users     | ON DELETE CASCADE         |
| token_hash | text UNIQUE NOT NULL     | SHA-256(pepper + raw)     |
| scopes     | text[] DEFAULT '{beta}'  | Permission scopes         |
| expires_at | timestamptz NOT NULL     | Token expiration          |
| revoked_at | timestamptz              | NULL = active             |
| created_at | timestamptz              | DEFAULT now()             |

### `audit_log`

| Column     | Type                     | Notes                     |
| ---------- | ------------------------ | ------------------------- |
| id         | uuid (PK)                | gen_random_uuid()         |
| action     | text NOT NULL            | e.g. token.created        |
| actor      | text NOT NULL            | admin, system, user email |
| metadata   | jsonb DEFAULT '{}'       | Action-specific data      |
| created_at | timestamptz              | DEFAULT now()             |

## Token Format

- Prefix: `anvil_beta_`
- Payload: `base64url(randomBytes(32))`
- Storage: SHA-256 hash only (raw returned once on invite)

## CLI Commands

| Command                          | Description                    |
| -------------------------------- | ------------------------------ |
| `anvil login [--token <tok>]`    | Authenticate with beta token   |
| `anvil logout`                   | Clear stored credentials       |
| `anvil whoami`                   | Display current session info   |
| `anvil beta invite --email ...`  | Create user + generate token   |
| `anvil beta revoke --email ...`  | Revoke all tokens for user     |

## Secrets

| Secret          | Location          | Purpose                          |
| --------------- | ----------------- | -------------------------------- |
| DATABASE_URL    | Vercel env        | Neon Postgres connection string  |
| ADMIN_KEY       | Vercel env + local | Bearer token for admin endpoints |
| TOKEN_PEPPER    | Vercel env        | Mixed into token hashing         |
| ANVIL_API_URL   | CLI env (optional) | Override API URL for dev         |
| ANVIL_ADMIN_KEY | Local env         | Used by `anvil beta` commands    |

## Tasks

| Task     | Description                          | Status      |
| -------- | ------------------------------------ | ----------- |
| BETA-001 | Scaffold anvil-api as Hono app       | Not Started |
| BETA-002 | Database schema + query layer        | Not Started |
| BETA-003 | Token generation + hashing utilities | Not Started |
| BETA-004 | Admin auth middleware                | Not Started |
| BETA-005 | POST /api/v1/auth/verify endpoint    | Not Started |
| BETA-006 | Admin endpoints (invite, revoke)     | Not Started |
| BETA-007 | Wire routes into app entry           | Not Started |
| BETA-008 | Auth store + API client (CLI)        | Not Started |
| BETA-009 | login, logout, whoami commands       | Not Started |
| BETA-010 | Auth gate (preAction hook)           | Not Started |
| BETA-011 | Admin CLI commands (beta invite/revoke) | Not Started |
| BETA-012 | CI + root config updates             | Not Started |
| BETA-013 | Neon DB setup + Vercel deployment    | Not Started |
