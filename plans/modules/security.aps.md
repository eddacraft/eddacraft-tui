<!--
APS Module: Security
====================================
Ongoing security posture management. Replaces archived security modules.
See: plans/aps-rules.md
-->

# Security

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| SEC    | —     | In Progress |

**Last reviewed:** 2026-05-21

## Purpose

Maintain ongoing security posture beyond the initial 0.1.x hardening.
Covers dependency auditing, secret rotation, vulnerability response,
supply chain security, and security hardening as the project grows.

**Replaces:** `security-ci-pipeline` (archived), `security-review-backlog`
(archived). Both were 0.1.x scope; this module covers forward-looking
security concerns.

## In Scope

- **Dependency auditing:** Automated `pnpm audit`, `cargo audit`, GitHub
  Dependabot configuration, lockfile policies
- **Secret management:** Rotation strategy, Pulumi ESC integration,
  environment variable governance
- **Vulnerability response:** Incident process, patch SLA, disclosure policy
- **Supply chain security:** Lockfile policies, private registry evaluation,
  transitive dependency risk assessment
- **Security hardening:** HTTP security headers, CSP, rate limiting (feeds
  into API governance)
- **SBOM generation:** Software bill of materials for release artifacts

## Out of Scope

- Penetration testing (external)
- SOC 2 compliance preparation (covered by compliance-reporting)
- Secret detection in code (covered by the Rust secret scanner in
  `crates/anvil-checks/src/secret/` — per ADR-026 the TS `secret.check.ts`
  gate is retired)

## Interfaces

**Depends on:**

- CI pipeline — automated security scanning
- `crates/anvil-checks` — Rust gate checks (secret, dependency, command-safety)
- Pulumi — secret management via ESC
- API governance — rate limiting, security headers

**Exposes:**

- Security policy and incident response process
- Dependency audit automation
- Secret rotation schedule

## Estimated Scope

- **Effort:** 1-2 weeks

## Tasks

- SEC-001: Dependency audit automation (pnpm + cargo audit in CI)
- SEC-002: Secret rotation strategy and documentation
- SEC-003: Vulnerability response process and SLA
- SEC-004: Supply chain security policy (lockfile, registry)
- SEC-005: HTTP security headers configuration
- SEC-006: SBOM generation for release artifacts
- SEC-007: Atomic token-revocation hardening (GH #1672)
- SEC-008: Named-pattern secret detection for AWS / GitHub PAT / Slack tokens (GH #1800)

### SEC-007: Atomic token-revocation hardening

**Status:** In Progress
**Owner:** Josh Boys
**Tracking:** GH issue #1672 (DeepSec `20260517012618-52306118d7d9df6a`)

**Outcome:** `POST /admin/revoke` revokes refresh sessions atomically with
access tokens, and account-level revocation (by email) lifts the user out of
`active` status so the OAuth / OTP / device login paths cannot re-mint
licences until an admin reactivates the account.

**Why:** DeepSec found five HIGH `other-token-revocation-bypass` findings in
`apps/anvil-api/`. Today `revoke {email}` only updates `access_tokens`, so
the user's `refresh_tokens` rows stay valid and `/session/refresh` mints a
fresh licence after revocation. Same shape for `revoke {token}` and the
unused `revokeTokensByEmail` / `revokeTokenByHash` / `revokeAccessTokensByUserId`
helpers. The fix defines account-level vs grant-level revocation semantics
and closes both in a single transaction.

**Semantics:**

- **Account-level (`revoke {email}`):** atomically revoke all
  `access_tokens` for the user, revoke all `refresh_tokens` for the user, and
  transition `beta_users.status` from `active` to `suspended`. The existing
  `status === 'active'` gates in `auth-otp.ts`, `auth-device.ts`,
  `auth-github.ts`, and `auth-session.ts` then block re-mint via every login
  path. `POST /admin/approve` continues to set status back to `active`, so
  reactivation works through the existing admin surface.
- **Grant-level (`revoke {token}`):** atomically revoke the specific
  `access_tokens` row by hash and revoke all `refresh_tokens` for the owning
  user, so the user cannot pivot through `/session/refresh` to mint a new
  access token. The user remains `active` and can re-authenticate to obtain a
  fresh grant via the normal login flow.

**Validation:**

- `pnpm --filter @eddacraft/anvil-api test` — new regression tests cover
  admin-revoke-by-email → `/session/refresh` returns 401, admin-revoke-by-token
  → `/session/refresh` returns 401, and account-level revoke flips status to
  `suspended`.
- `pnpm format:check && pnpm lint:check && pnpm typecheck`

**Known follow-up (out of scope for SEC-007):** Existing JWT licences issued
before revocation remain bearer-valid for up to 7 days because
`requireAuth()` validates the licence's signature and expiry in-process and
does not consult `access_tokens` or `beta_users.status`. Mitigating this
needs either a `jti` deny-list with a DB lookup on hot paths or a much
shorter licence TTL paired with mandatory `/session/refresh`, both of which
sit outside the "atomic revocation" remediation. Track separately under
SEC if/when the next licensing-stream review opens the surface.

**changeType:** fix
**releaseIntent:** candidate
**releaseScope:** patch
**releaseNote:**

- **audience:** operator
- **type:** security
- **text:** Admin token revocation now revokes refresh sessions atomically;
  revoking by email also suspends the account so OAuth/OTP/device login paths
  cannot re-mint licences until the user is reapproved.

Coordinates with: archived `beta-auth-streamline` (BAUTH) module, which
established the original revoke endpoints.

### SEC-008: Named-pattern secret detection for AWS / GitHub / Slack tokens

**Status:** Merged
**Tracking:** GH issue [#1800](https://github.com/eddacraft/anvil-001/issues/1800)
**Evidence:** Merged via PR [#1815](https://github.com/eddacraft/anvil-001/pull/1815)
(`fix(checks): flag textbook AWS access keys`). Root cause was post-match
filtering (`looks_like_code` + keyword allowlist) silently dropping
already-existing named patterns; fix splits the allowlist into shape vs
keyword tiers and marks structurally-unambiguous patterns
high-confidence. Adds `ASIA[0-9A-Z]{16}` (STS), `sk-ant-…` (Anthropic),
`sk-(?:proj|svcacct|admin)-…` (OpenAI) patterns. Pending release in the
next `v0.6.x-beta` tag.

**Intent:** Anvil's secret detector currently relies on a high-entropy
heuristic and consequently misses textbook AWS / GitHub PAT / Slack
token shapes when the literal token happens to fall below the entropy
threshold (the canonical AWS `EXAMPLE` keys are the loudest case).
Add named-pattern detection so the most-leaked credential shapes trip
the gate regardless of entropy.

**Outcome:** The secret detector trips on at least the industry-leaked
token shapes regardless of whether they cross the current high-entropy
threshold:

- `AKIA[0-9A-Z]{16}` — AWS access key ID
- `ASIA[0-9A-Z]{16}` — AWS STS temporary access key
- AWS secret key (40-char base64-alphabet, often near
  `aws_secret_access_key` / `secret`)
- GitHub PATs (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`)
- Slack tokens (`xox[apb]-…`)

The current high-entropy heuristic continues to run alongside the named
patterns, not as a replacement; the addition is named patterns layered on
top so high-recognition / lower-entropy tokens (the EXAMPLE-style AWS keys
in particular) stop sliding past the gate.

**Identified From:** [2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md)
finding #6. The canonical AWS example pair
(`AKIAIOSFODNN7EXAMPLE` + `wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY`) was
**allowed** by the MCP pre-write gate with 0 diagnostics, while a
random-base64-looking `sk-…` literal tripped the high-entropy rule on the
same surface.

**Test fixture (safe to commit):** The AWS example keys above are official
AWS-published documentation literals and are explicitly safe to commit
as test data.

**Validation:**

- Unit tests in `crates/anvil-checks/` (secret module) that fixture each
  named pattern and assert detection.
- MCP regression test that calls `anvil_validate_write` with each named
  pattern as `proposedContent` and asserts `decision: block`.

**Coordinates with:** MLP2-072 (MCP gate-shape) — the new named-pattern
hits need to flow through whichever decision shape MLP2-072 lands on so
they are not silently swallowed by an auth-gate response.

**changeType:** feature
**releaseIntent:** candidate
**releaseScope:** minor
**releaseNote:**

- **audience:** developer
- **type:** security
- **text:** Anvil's secret detector now catches AWS, GitHub PAT, and Slack
  token shapes by named pattern as well as high entropy.
