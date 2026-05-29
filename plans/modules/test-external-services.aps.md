# Test External Services

| ID   | Owner      | Status | Progress |
| ---- | ---------- | ------ | -------- |
| TEXT | @eddacraft | Draft  | 0/14     |

**Last reviewed:** 2026-04-26 — TFIX archived Complete (CI infrastructure
foundation in place). All `apps/anvil-api` and `infra/` paths still current on
`dev`.

## Purpose

The platform integrates with external services — Resend for transactional
email, Neon Postgres for persistence, GitHub OAuth for docs auth gating,
Azure for infrastructure state, and Vercel for deployment and hosting.
These external boundaries are currently untested: no contract tests verify
API compatibility, no recorded fixtures guard against upstream schema
changes, and failures in these services would surface only in production.
OPA is used as a local policy engine binary; its invocation is covered in
TCOV and is therefore out of scope for TEXT.

This module establishes contract and integration tests for external service
boundaries. It uses record/replay patterns to keep tests fast, deterministic,
and runnable without live credentials in most cases, while periodic live runs
validate the recordings are still accurate.

## External Service Inventory

| Service              | SDK / Client                               | Consuming Code                          | APIs Used                                                                |
| -------------------- | ------------------------------------------ | --------------------------------------- | ------------------------------------------------------------------------ |
| **Neon Postgres**    | `@neondatabase/serverless` ^1.0.0          | `apps/anvil-api/src/db/`                | SQL over HTTP (waitlist, users, auth tokens, audit log)                  |
| **Resend**           | `resend` ^6.10.0                           | `apps/anvil-api/src/routes/`            | Send email, template rendering (OTP, beta invite, waitlist)              |
| **GitHub OAuth API** | native `fetch`                             | `apps/anvil-api/src/routes/auth-github` | `POST /login/oauth/access_token`, `GET /user`, `GET /user/emails`       |
| **Azure Key Vault**  | `@azure/keyvault-secrets` + `@azure/identity` | `infra/src/keyvault.ts`                 | `getSecret` (deploy-time secrets for Vercel envs)                        |
| **Azure DNS**        | `@pulumi/azure-native` (Pulumi)            | `infra/src/dns/`                        | DNS record management (A, CNAME, TXT)                                    |
| **Vercel**           | `@pulumiverse/vercel` (Pulumi)             | `infra/src/vercel.ts`                   | Project creation, env vars, domains, build config (6 managed projects)   |

## In Scope

- Neon Postgres contract tests (connection, query, error handling, schema
  compatibility)
- Resend email API contract tests (send, template rendering, error handling)
- GitHub OAuth API contract tests (token exchange, user info, email retrieval)
- Azure resource management contract tests (Key Vault secrets, DNS)
- Vercel API contract tests (deployment, environment variables, project
  configuration)
- Record/replay test infrastructure (HTTP fixture recording, schema validation
  against recorded responses)
- Periodic live validation CI job (scheduled, not on every PR)
- Secret management patterns for CI (GitHub Actions secrets, environment
  scoping)

## Out of Scope

- Internal subprocess boundaries (TINT)
- Unit test coverage (TCOV)
- CI infrastructure (TFIX)
- OPA binary invocation (covered in TCOV — OPA is a local binary, not a remote
  service)
- Performance or load testing of external services

## Interfaces

**Depends on:**

- TFIX — CI infrastructure foundation
- `packages/transactional/` — Resend email templates
- `apps/anvil-api/` — API routes that call external services
- `infra/src/` — Pulumi infrastructure definitions (Azure, Vercel)

**Exposes:**

- Record/replay test pattern (reusable for future service integrations)
- HTTP fixture library for all integrated services
- Scheduled CI job for live contract validation

## Constraints

- Live tests must not incur costs beyond free-tier usage
- Recorded fixtures must be refreshable without code changes
- Tests must pass without credentials (using recordings); live mode opt-in via
  environment variable
- No production service accounts used in CI — dedicated test/sandbox accounts
  only

## Risks

| Risk                                          | Impact | Mitigation                                               |
| --------------------------------------------- | ------ | -------------------------------------------------------- |
| Recorded fixtures go stale without detection  | high   | Scheduled weekly live run; hash-check response schemas   |
| Secrets leak into test fixtures or logs       | high   | Sanitisation pass on recordings; CI secret masking       |
| External service rate limits block CI         | medium | Record/replay by default; live runs scheduled off-peak   |
| Service APIs change without notice            | medium | Schema validation catches structural drift in recordings |

## Ready Checklist

Change status to **Ready** when:

- [x] External service inventory complete (which services, which APIs, which
  packages call them) — see table above
- [ ] Record/replay tooling selected (nock, msw, polly.js, or custom)
- [ ] Sandbox/test accounts provisioned for each service
- [ ] Secret management pattern agreed (GitHub Actions secrets, environment
  scoping)
- [ ] At least one service fully scoped with tasks

## Work Items

> Tasks below are directional. Full task definitions will be written once the
> Ready Checklist is complete and the service inventory is done.

### Phase 1 — Infrastructure

#### TEXT-001: select and configure record/replay tooling

- **Status:** Proposed
- **Intent:** Choose an HTTP recording library (nock, msw, polly.js) and set
  up the fixture directory structure.
- **Expected Outcome:** A single record/replay library is selected, documented
  (with the rationale for the choice over the other candidates), and wired
  into the test harness. The fixture directory structure exists and is ready
  for per-service recordings.
- **Validation:** The chosen library is present in the relevant
  `package.json` and a smoke test records then replays one fixture without
  live credentials (`pnpm test` for the harness passes in replay mode).
- **Confidence:** high

#### TEXT-002: secret management for CI

- **Status:** Proposed
- **Intent:** Establish the pattern for provisioning service credentials in
  GitHub Actions — which secrets, which scopes, how to rotate.
- **Expected Outcome:** A documented secret-management pattern that lists each
  required secret, its scope (per the Constraints: dedicated test/sandbox
  accounts only, no production accounts), and a rotation procedure. Live
  tests are opt-in via an environment variable and CI masks the secrets.
- **Validation:** Manual review of the documented pattern against the
  Constraints (no production accounts, free-tier only, credentials gated
  behind the opt-in env var) and confirmation that the secrets are masked in
  CI logs.
- **Confidence:** high

#### TEXT-003: scheduled live validation CI job

- **Status:** Proposed
- **Intent:** Create a scheduled (weekly) CI workflow that runs all external
  contract tests against live services.
- **Expected Outcome:** A scheduled GitHub Actions workflow runs the external
  contract tests in live mode on a weekly cadence (off-peak, per the rate-limit
  mitigation), separate from the per-PR replay-mode runs.
- **Validation:** The workflow file exists with a `schedule` (cron) trigger,
  and a manual dispatch of the workflow completes a full live run green.
- **Confidence:** high

### Phase 2 — Neon Postgres

#### TEXT-004: Neon Postgres connection contract test

- **Status:** Proposed
- **Intent:** Test database connection via `@neondatabase/serverless` — success,
  connection errors, timeout handling. Verify `setClient()` injection works for
  test isolation.
- **Expected Outcome:** Contract tests cover the success path plus connection
  error and timeout handling for `@neondatabase/serverless`, and assert that
  `setClient()` injection isolates the client under test.
- **Validation:** `pnpm test` (or the harness's targeted test command for
  `apps/anvil-api/src/db/`) passes in replay mode with the connection tests
  green.
- **Confidence:** high

#### TEXT-005: Neon Postgres query contract tests

- **Status:** Proposed
- **Intent:** Test core query patterns (waitlist insert, user lookup, auth
  token CRUD, audit log append) against recorded responses. Verify schema
  compatibility — column types, nullability, constraints.
- **Expected Outcome:** Contract tests exercise each core query pattern
  (waitlist insert, user lookup, auth-token CRUD, audit-log append) against
  recorded responses and assert schema compatibility (column types,
  nullability, constraints).
- **Validation:** `pnpm test` for `apps/anvil-api/src/db/` passes in replay
  mode with the query contract tests green.
- **Confidence:** high

### Phase 3 — Resend

#### TEXT-006: Resend send API contract test

- **Status:** Proposed
- **Intent:** Test email send via Resend API — success, rate limit, invalid
  address, template rendering.
- **Expected Outcome:** Contract tests cover the Resend send path for success,
  rate-limit, and invalid-address responses against recorded fixtures.
- **Validation:** `pnpm test` for `apps/anvil-api/src/routes/` passes in replay
  mode with the Resend send tests green.
- **Confidence:** high

#### TEXT-007: Resend template rendering tests

- **Status:** Proposed
- **Intent:** Verify transactional email templates (`beta-invite.tsx`,
  `otp-code.tsx`, `waitlist-confirmation.tsx`, `waitlist-migration.tsx`) render
  correctly via Resend's API.
- **Expected Outcome:** Each named transactional template
  (`beta-invite.tsx`, `otp-code.tsx`, `waitlist-confirmation.tsx`,
  `waitlist-migration.tsx`) has a test asserting it renders to the expected
  output.
- **Validation:** `pnpm test` for the `packages/transactional/` templates
  passes with a rendering assertion per template.
- **Confidence:** high

### Phase 4 — GitHub OAuth

#### TEXT-008: GitHub OAuth token exchange contract test

- **Status:** Proposed
- **Intent:** Test the `POST github.com/login/oauth/access_token` exchange —
  success, invalid code, expired code. Verify response schema.
- **Expected Outcome:** Contract tests cover the token-exchange success path
  plus invalid-code and expired-code responses, and assert the response schema
  matches what the consuming code expects.
- **Validation:** `pnpm test` for `apps/anvil-api/src/routes/auth-github`
  passes in replay mode with the token-exchange tests green.
- **Confidence:** high

#### TEXT-009: GitHub user and email API contract tests

- **Status:** Proposed
- **Intent:** Test `GET api.github.com/user` and `GET api.github.com/user/emails`
  — success, invalid token, scope-limited responses. Verify response schemas
  match what `auth-github.ts` expects.
- **Expected Outcome:** Contract tests cover the user and user-emails endpoints
  for success, invalid-token, and scope-limited responses, asserting the
  response schemas match what `auth-github.ts` consumes.
- **Validation:** `pnpm test` for `apps/anvil-api/src/routes/auth-github`
  passes in replay mode with the user/email API tests green.
- **Confidence:** high

### Phase 5 — Azure

#### TEXT-010: Azure Key Vault secret retrieval contract tests

- **Status:** Proposed
- **Intent:** Test `getSecret` calls against Azure Key Vault — success, not
  found, access denied. Verify `DefaultAzureCredential` fallback behaviour.
- **Expected Outcome:** Contract tests cover `getSecret` for success,
  not-found, and access-denied responses, and assert the
  `DefaultAzureCredential` fallback behaviour.
- **Validation:** `pnpm test` for `infra/src/keyvault.ts` passes in replay
  mode with the Key Vault tests green.
- **Confidence:** medium

#### TEXT-011: Azure DNS contract tests

- **Status:** Proposed
- **Intent:** Test DNS record creation and lookup against the Azure DNS API
  (A, CNAME, TXT records).
- **Expected Outcome:** Contract tests cover DNS record creation and lookup
  for A, CNAME, and TXT records against recorded Azure DNS responses.
- **Validation:** `pnpm test` for `infra/src/dns/` passes in replay mode with
  the DNS record tests green.
- **Confidence:** medium

### Phase 6 — Vercel

#### TEXT-012: Vercel deployment API contract tests

- **Status:** Proposed
- **Intent:** Test deployment creation, status polling, and promotion via the
  Vercel REST API.
- **Expected Outcome:** Contract tests cover deployment creation, status
  polling, and promotion against recorded Vercel REST API responses.
- **Validation:** `pnpm test` for `infra/src/vercel.ts` passes in replay mode
  with the deployment API tests green.
- **Confidence:** medium

#### TEXT-013: Vercel environment variable API tests

- **Status:** Proposed
- **Intent:** Test env var read/write/delete operations against the Vercel
  project API.
- **Expected Outcome:** Contract tests cover environment-variable read, write,
  and delete operations against recorded Vercel project API responses.
- **Validation:** `pnpm test` for `infra/src/vercel.ts` passes in replay mode
  with the env-var API tests green.
- **Confidence:** medium

### Phase 7 — Validation

#### TEXT-014: recording staleness detection

- **Status:** Proposed
- **Intent:** Add schema validation to recorded fixtures — if the live response
  structure diverges from the recording, flag it.
- **Expected Outcome:** The scheduled live run (TEXT-003) compares live
  response structure against the recorded fixtures and flags any divergence,
  so stale recordings are detected rather than silently passing.
- **Validation:** A test or check that, given a deliberately divergent live
  response, fails (flags the drift); the scheduled live run surfaces the flag.
- **Confidence:** medium
- **Dependencies:** TEXT-003 (scheduled live validation CI job)
