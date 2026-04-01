# Test External Services

| ID   | Owner      | Status |
| ---- | ---------- | ------ |
| TEXT | @eddacraft | Draft  |

## Purpose

The platform integrates with external services — Resend for transactional
email, Azure for infrastructure state, and Vercel for deployment and hosting.
These external boundaries are currently untested: no contract tests verify
API compatibility, no recorded fixtures guard against upstream schema
changes, and failures in these services would surface only in production.
OPA is used as a local policy engine binary; its invocation is covered in
TCOV and is therefore out of scope for TEXT.

This module establishes contract and integration tests for external service
boundaries. It uses record/replay patterns to keep tests fast, deterministic,
and runnable without live credentials in most cases, while periodic live runs
validate the recordings are still accurate.

## In Scope

- Resend email API contract tests (send, template rendering, error handling)
- Azure resource management contract tests (blob storage, DNS, any
  Pulumi-managed resources)
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

- [ ] External service inventory complete (which services, which APIs, which
  packages call them)
- [ ] Record/replay tooling selected (nock, msw, polly.js, or custom)
- [ ] Sandbox/test accounts provisioned for each service
- [ ] Secret management pattern agreed (GitHub Actions secrets, environment
  scoping)
- [ ] At least one service fully scoped with tasks

## Preliminary Task Sketch

> Tasks below are directional. Full task definitions will be written once the
> Ready Checklist is complete and the service inventory is done.

### Phase 1 — Infrastructure

#### TEXT-001: select and configure record/replay tooling

- **Intent:** Choose an HTTP recording library (nock, msw, polly.js) and set
  up the fixture directory structure.
- **Confidence:** high

#### TEXT-002: secret management for CI

- **Intent:** Establish the pattern for provisioning service credentials in
  GitHub Actions — which secrets, which scopes, how to rotate.
- **Confidence:** high

#### TEXT-003: scheduled live validation CI job

- **Intent:** Create a scheduled (weekly) CI workflow that runs all external
  contract tests against live services.
- **Confidence:** high

### Phase 2 — Resend

#### TEXT-004: Resend send API contract test

- **Intent:** Test email send via Resend API — success, rate limit, invalid
  address, template rendering.
- **Confidence:** high

#### TEXT-005: Resend template rendering tests

- **Intent:** Verify transactional email templates (`beta-invite.tsx`,
  `otp-code.tsx`) render correctly via Resend's API.
- **Confidence:** high

### Phase 3 — Azure

#### TEXT-006: Azure blob storage contract tests

- **Intent:** Test blob upload, download, list, and delete operations against
  the Azure storage API (or emulator).
- **Confidence:** medium

#### TEXT-007: Azure DNS contract tests

- **Intent:** If DNS is managed via Azure, test record creation and lookup
  against the API.
- **Confidence:** medium

### Phase 4 — Vercel

#### TEXT-008: Vercel deployment API contract tests

- **Intent:** Test deployment creation, status polling, and promotion via the
  Vercel REST API.
- **Confidence:** medium

#### TEXT-009: Vercel environment variable API tests

- **Intent:** Test env var read/write/delete operations against the Vercel
  project API.
- **Confidence:** medium

### Phase 5 — Validation

#### TEXT-010: recording staleness detection

- **Intent:** Add schema validation to recorded fixtures — if the live response
  structure diverges from the recording, flag it.
- **Confidence:** medium
