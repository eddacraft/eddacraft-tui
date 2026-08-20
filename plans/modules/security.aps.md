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

**Last reviewed:** 2026-07-30

## Purpose

Maintain ongoing security posture beyond the initial 0.1.x hardening.
Covers dependency auditing, secret rotation, vulnerability response,
supply chain security, and security hardening as the project grows.

**Replaces:** `security-ci-pipeline` (archived), `security-review-backlog`
(archived). Both were 0.1.x scope; this module covers forward-looking
security concerns.

> **ID namespace note:** the archived `security-ci-pipeline` module historically
> reused **SEC-001..008** for CI-pipeline work items (workflow, Semgrep, audit,
> secret scan, licence check, custom rules, Scorecard, reporting). Those IDs
> are **not** the live backlog. This live SEC module is authoritative for
> forward work; do not renumber either set without an explicit APS decision.

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

## Work Items

| Item | Title | Status |
| ---- | ----- | ------ |
| SEC-001 | Reconcile + document the dependency-audit posture | Ready |
| SEC-002 | Secret rotation runbook | Ready |
| SEC-003 | Vulnerability response + coordinated disclosure policy | Ready |
| SEC-004 | Supply-chain policy doc (lockfile, registry, cargo-deny) | Ready |
| SEC-005 | HTTP security headers on `anvil-api` | Proposed — **needs APGOV boundary call** |
| SEC-006 | SBOM generation for release artefacts | **Deferred to SCA** — do not duplicate |
| SEC-007 | Atomic token-revocation hardening (GH #1672) | In Progress |
| SEC-008 | Named-pattern secret detection (GH #1800) | Merged |
| SEC-009 | Private docs entitlement gate (GH #1673) | Done |
| SEC-010 | Remediate brace-expansion denial-of-service alerts | Merged |
| SEC-011 | Remediate repository-wide JavaScript dependency advisories | Merged |

> **Cross-module overlaps flagged 2026-05-28 (do not duplicate scope):**
>
> - **SEC-006 (SBOM) ↔ SCA.** `supply-chain-attestation` (SCA, Proposed)
>   already owns SBOM generation — SCA-001 "Design the SBOM generation + merge
>   stage" uses the proper CycloneDX generators (`cargo-cyclonedx`,
>   `cyclonedx-npm`, `cyclonedx-gomod`, `cyclonedx-py`), feeds the merged SBOM
>   into Anvil's graph/witness layer (SCA-002), and gates release-time
>   attestation. SEC-006 is therefore **deferred to SCA**, not implemented
>   here — SEC keeps only a pointer so the posture doc (SEC-001) can link the
>   SBOM story without re-scoping it. SCA itself is gated on Anvil's graph
>   layer ingesting a dependency graph and stays Proposed.
> - **SEC-005 (security headers) ↔ APGOV.** API security headers "feed into API
>   governance" per this module's In-Scope note, but `api-governance` (APGOV)
>   does not currently list security headers in its scope (it governs
>   versioning, error contract, rate limiting, CORS, OpenAPI, deprecation,
>   health). SEC-005 stays **Proposed pending an owner call** on whether the
>   header config + policy lives in APGOV's API-surface governance or here in
>   SEC. It is not fleshable to Ready until that boundary is set.

### SEC-001: Reconcile and document the dependency-audit posture — Ready

- **Status:** Ready
- **Intent:** Make the already-shipped dependency-audit automation legible and
  close the one real Rust-update gap, rather than rebuilding it.
- **Reality on `main` (2026-05-28):** the original bullet ("pnpm + cargo audit
  in CI") is largely already shipped, but not via those literal tools:
  - `.github/workflows/security.yml` runs a `dependency-audit` job (Trivy
    `fs` scan over lockfiles, HIGH/CRITICAL, `exit-code: 1`), a `license-check`
    job (`scripts/license-check.sh`), a `secret-scan` job (TruffleHog
    `--only-verified`), and Semgrep SAST — path-gated per PR plus a weekly
    sweep. The comment at `security.yml:108` records *why* literal `pnpm audit`
    is not used (the npm v1 audit endpoint returns 410).
  - `.github/workflows/rust.yml` runs a `cargo-deny` job
    (`rust.yml:369`) whose `deny.toml` `[advisories]` section consumes the
    RUSTSEC advisory DB — i.e. the "cargo audit" intent already ships via
    cargo-deny.
  - `.github/dependabot.yml` updates `npm` and `github-actions` ecosystems
    weekly.
- **Expected Outcome:** A short `docs/guides/dependency-audit-posture.md` (new)
  documents the as-built audit surface (Trivy vuln scan, cargo-deny advisories,
  TruffleHog, dependabot, license-check) and its gating; and `.github/dependabot.yml`
  gains a `cargo` ecosystem entry so Rust dependency updates are automated the
  same way npm and actions already are (the one concrete gap, since cargo
  updates are not configured in Dependabot today and advisory gating already
  ships via cargo-deny).
- **Scope:** `.github/dependabot.yml`, `docs/guides/dependency-audit-posture.md`
  (new — does not exist yet).
- **Non-scope:** SBOM/attestation (SCA); changing the existing Trivy/cargo-deny
  thresholds; secret-detection rule content (SEC-008).
- **Dependencies:** —
- **Validation:** `grep -q "package-ecosystem: 'cargo'" .github/dependabot.yml`
  and the new posture doc passes `pnpm docs:check`.
- **Confidence:** high

### SEC-002: Secret rotation runbook — Ready

- **Status:** Ready
- **Intent:** Document how each long-lived secret is rotated, on what cadence,
  and through which channel (Pulumi ESC), so rotation is a runbook step rather
  than tribal knowledge.
- **Expected Outcome:** A `docs/runbooks/secret-rotation.md` (new) inventories
  the rotatable secrets (licence signing/verifying key PEMs, DB/Neon
  credentials, admin tokens, any third-party API keys), states a review/rotation
  cadence for each, and gives the rotation procedure including the Pulumi ESC
  path. Each secret has an owner and a "rotate by" review date.
- **Scope:** `docs/runbooks/secret-rotation.md` (new — does not exist yet);
  no code change.
- **Non-scope:** Implementing automated rotation; secret *detection* (that is
  the Rust scanner + SEC-008).
- **Dependencies:** —
- **Validation:** Manual review — an operator can follow the runbook to rotate
  the licence signing key end to end; `pnpm docs:check` passes.
- **Confidence:** medium — the secret inventory is grounded but the Pulumi ESC
  rotation path needs confirmation against the live infra.

### SEC-003: Vulnerability response and coordinated disclosure policy — Ready

- **Status:** Ready
- **Intent:** Define how externally-reported and internally-found
  vulnerabilities are received, triaged, and patched, and publish a disclosure
  contact — the root policy file does not exist today (`apps/anvil-api/` has a
  local `SECURITY.md`, but there is no repository-level `SECURITY.md`).
- **Expected Outcome:** A root `SECURITY.md` (new) states the disclosure
  channel, supported versions, and the response/patch SLA by severity; a
  companion `docs/runbooks/vulnerability-response.md` (new) gives the internal
  triage-to-patch procedure (intake, severity assessment, fix, release,
  advisory). The existing DeepSec finding flow (e.g. SEC-007's GH #1672) is
  referenced as the internal-discovery example.
- **Scope:** `SECURITY.md` (new — does not exist yet),
  `docs/runbooks/vulnerability-response.md` (new — does not exist yet).
- **Non-scope:** SOC 2 / compliance (compliance-reporting); pen-testing.
- **Dependencies:** —
- **Validation:** `test -f SECURITY.md` and `pnpm docs:check` passes; manual
  review that the SLA table maps severity → response time.
- **Confidence:** high

### SEC-004: Supply-chain policy documentation — Ready

- **Status:** Ready
- **Intent:** Write down the supply-chain policy that the `deny.toml` config
  already enforces, plus the lockfile/registry rules, so the enforcement is
  documented and changes to `deny.toml` have a referenced rationale.
- **Reality on `main`:** `deny.toml` already carries `[advisories]`,
  `[licenses]`, `[bans]`, and `[sources]` sections (enforced by the `rust.yml`
  `cargo-deny` job); pnpm uses `--frozen-lockfile` in CI; a private-registry
  evaluation has not been recorded.
- **Expected Outcome:** A `docs/guides/supply-chain-policy.md` (new) documents
  the lockfile policy (frozen lockfile, who may update), the `deny.toml`
  ban/source rules and how to amend them, the transitive-dependency risk
  posture, and the (current) decision on private registries. Cross-links SCA
  for the attestation/SBOM half of supply-chain security so the boundary is
  explicit.
- **Scope:** `docs/guides/supply-chain-policy.md` (new — does not exist yet);
  optionally an explanatory comment in `deny.toml`.
- **Non-scope:** SBOM/attestation (SCA); changing the actual ban/license rules.
- **Dependencies:** —
- **Validation:** `pnpm docs:check` passes; manual review that the policy doc
  matches the live `deny.toml` sections.
- **Confidence:** high

### SEC-005: HTTP security headers on `anvil-api` — Proposed (needs APGOV boundary call)

- **Status:** Proposed — **needs design**: the APGOV↔SEC ownership boundary
  must be set before this is fleshable to Ready (see the overlap callout
  above). Not promoted.
- **Intent:** Add HTTP security headers (CSP, HSTS, `X-Content-Type-Options`,
  `X-Frame-Options`, referrer policy) to the `anvil-api` Hono app, which
  currently sets none.
- **Expected Outcome:** Once the APGOV↔SEC ownership boundary is set, the
  `anvil-api` responds with the agreed security-header set on all routes, with
  CSP scoped to the API's actual needs (it serves JSON, not HTML, so the CSP can
  be strict).
- **Validation:** `curl -I` against a route shows the agreed headers and a route
  test asserts their presence (both deferred until ownership is set).
- **Reality on `main`:** `apps/anvil-api/src/index.ts` wires `cors`, `logger`,
  `traceContext`, and `rateLimiter` middleware but no `secureHeaders`
  equivalent — a grep for `secureHeaders`/`Content-Security-Policy`/`hsts`
  returns nothing in `apps/anvil-api/src`.
- **Blocks on:** an owner decision on whether the header config + policy is
  owned by `api-governance` (APGOV, which governs the API surface) or by SEC.
  Until that lands, scoping the work item risks duplicating APGOV scope.
- **Coordinates with:** APGOV (API-surface governance; CORS/rate-limit already
  live there).
- **Expected Outcome (deferred until ownership is set):** the API responds with
  the agreed security-header set on all routes, with CSP scoped to the API's
  actual needs (it serves JSON, not HTML, so the CSP can be strict).
- **Validation (deferred):** `curl -I` against a route shows the agreed headers;
  a route test asserts their presence.

### SEC-006: SBOM generation for release artefacts — Deferred to SCA

- **Status:** Deferred — **do not implement here.** SBOM generation is owned by
  `supply-chain-attestation` (SCA-001). See the overlap callout above.
- **Intent:** Keep SEC pointing at the single SBOM owner (SCA) rather than
  duplicating SBOM generation under SEC.
- **Expected Outcome:** SEC retains only a pointer to SCA-001/SCA-002 for the
  SBOM story; no SBOM generation is implemented in this module.
- **Validation:** Manual review — this item carries only the SCA pointer and the
  posture doc (SEC-001) links the SBOM story via SCA without re-scoping it.
- **Intent (historical):** Generate a software bill of materials for release
  artefacts.
- **Why deferred:** SCA-001 already scopes per-ecosystem CycloneDX SBOM
  generation + merge with the *proper* generators, dependency-graph ingestion
  (SCA-002), and release-time attestation. Duplicating it under SEC would split
  the SBOM story across two modules. SEC retains only this pointer; the work
  lives in SCA, which is gated on Anvil's graph layer ingesting a dependency
  graph and remains Proposed.
- **Pointer:** [`supply-chain-attestation`](./supply-chain-attestation.aps.md)
  SCA-001 / SCA-002.

### SEC-007: Atomic token-revocation hardening

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21). Merged
  2026-05-21 via PR [#1806](https://github.com/eddacraft/anvil-001/pull/1806)
  (`3c7b741d3`). Status reconciled 2026-08-19 — the item sat at In Progress for
  three months after the code landed.
**Owner:** Josh Boys
**Tracking:** GH issue #1672 (DeepSec `20260517012618-52306118d7d9df6a`)

- **Intent:** Make admin token revocation atomic across access and refresh
  tokens so a revoked user cannot re-mint a licence via `/session/refresh` or
  any login path until reactivated.
- **Expected Outcome:** `POST /admin/revoke` revokes refresh sessions atomically
  with access tokens, and account-level revocation (by email) suspends the user
  so the OAuth / OTP / device login paths cannot re-mint licences until an admin
  reactivates the account.
- **Validation:** `pnpm --filter @eddacraft/anvil-api test` — regression tests
  cover admin-revoke-by-email and admin-revoke-by-token both yielding 401 on
  `/session/refresh`, plus account-level revoke flipping status to `suspended`;
  `pnpm format:check && pnpm lint:check && pnpm typecheck`.

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

**Evidence:** `apps/anvil-api/src/routes/admin.ts` (`POST /admin/revoke`) runs
both branches as a Neon batch transaction — account-level revokes access
tokens, refresh tokens and flips `beta_users.status` to `suspended`;
grant-level revokes the hashed access token plus the owning user's refresh
family. Regression coverage lives in `apps/anvil-api/src/__tests__/admin.test.ts`
(`refreshSessionsRevoked` / `accountSuspended` assertions).

**Known follow-up (out of scope for SEC-007) — restated 2026-08-19:** the
original wording named `requireAuth()`, a helper that no longer exists, and
asserted that no licence path consults the database. That is no longer
accurate: `POST /auth/verify` re-reads the account and rejects any subject
whose `status` is not `active` (`apps/anvil-api/src/routes/auth.ts`). The
residual is narrower and now lives in **SEC-013**: licence-authenticated
routes added after this remediation verify the signature only, so an
unexpired licence held by a suspended account is still accepted there. The
broader `jti` deny-list / shorter-TTL question remains out of scope for both
items until a licensing-stream review opens the surface.

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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
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

### SEC-009: Private docs entitlement gate for signed licences

- **Status:** Done (shipped) **with a live residual — the gate as shipped does
  not discriminate.** Reconciled 2026-08-19. The entitlement *mechanism* landed
  as specified and is not being reopened; closing the residual is **SEC-012**.
- **Residual (2026-08-19):** `apps/docs-shell/lib/jwt.ts` accepts
  `tier ∈ {beta, pro, enterprise}`. Post-BACT-013 `signLicence` writes
  `tier: claims.plan` (`apps/anvil-api/src/lib/licence.ts`), and `plan` is
  `beta` for every account — the only value `beta_users.plan`'s CHECK admits
  today. Every validly-signed licence therefore satisfies the tier check, so
  the gate is structurally present but semantically vacuous. `verifyLicence`
  additionally defaults a claimless token to `'beta'`, which is inside the
  accepted set, so the API-side verifier fails **open** where
  `apps/docs-site/middleware.ts` fails **closed** on a missing claim. Do not
  cite SEC-009 as evidence that private docs are entitlement-gated until
  SEC-012 lands.
**Tracking:** GH issue [#1673](https://github.com/eddacraft/anvil-001/issues/1673)
**Identified From:** DeepSec run `20260517012618-52306118d7d9df6a`, finding
`acl-check`.

**Outcome:** `apps/docs-shell` must reject valid signed licence JWTs that do not
carry the private-docs entitlement represented by the docs access tier contract,
instead of treating any valid signature as sufficient for `/anvil` private docs.

**Why:** The docs shell currently gates `/anvil` by cookie presence and licence
signature only. A signed licence without a docs-access tier can therefore proxy
private Anvil documentation.

**Validation:**

- `pnpm --filter @eddacraft/docs-shell test` — regression covers a valid signed
  licence without docs entitlement redirecting to login and clearing the cookie.
- `pnpm --filter @eddacraft/docs-shell typecheck`

**Evidence:** 2026-05-28 — `pnpm --filter @eddacraft/docs-shell test` (47/47
passed) and `pnpm --filter @eddacraft/docs-shell typecheck` passed locally.

**changeType:** fix
**releaseIntent:** candidate
**releaseScope:** patch
**releaseNote:**

- **audience:** operator
- **type:** security
- **text:** The private docs shell now requires a docs-access entitlement in the
  signed licence before proxying `/anvil` documentation.

### SEC-010: Remediate brace-expansion denial-of-service alerts

- **Status:** Released/Shipped via v0.9.2-beta (22f6a9be · 2026-08-04). Merged 2026-08-03 via PR #3493
- **Pull Request:** [#3493](https://github.com/eddacraft/anvil-001/pull/3493)
  (merged into `main`).
- **Intent:** Clear Dependabot alerts
  [#236](https://github.com/eddacraft/anvil-001/security/dependabot/236) and
  [#237](https://github.com/eddacraft/anvil-001/security/dependabot/237) for
  CVE-2026-14257 by moving every curated development-tool consumer to the
  latest hardened patch in its existing major line.
- **Expected Outcome:** `tools/dev` resolves `brace-expansion` 1.x to `1.1.18`
  and 5.x to `5.0.9`; the root and DeepSec overrides resolve affected consumers
  to `5.0.9`; lockfiles and `ACKNOWLEDGEMENTS.md` agree with those versions.
- **In Scope:** Existing dependency overrides, their generated lockfiles, and
  generated attribution output.
- **Out of Scope:** Runtime feature changes, dependency replacement, audit
  scheduling, advisory dismissal, and unrelated dependency refreshes.
- **Dependencies:** None. The releases are compatible patch updates on the
  existing dependency paths.
- **Validation:** Clean frozen installs for the root, `tools/dev`, and
  `.deepsec`; dependency-tree assertions for `brace-expansion`; acknowledgements
  freshness; APS, documentation, formatting, lint, typecheck, and test gates;
  GitHub's Dependency Audit check on the pull request.
- **Evidence:** Council `council-c061eec1` passed after scoped re-review; local
  frozen installs, dependency-tree, acknowledgements, APS, documentation,
  formatting, lint, typecheck, JavaScript, and Rust workspace gates passed.
  All 30 applicable GitHub checks passed, merge commit `e2cba8f30` reached
  `main`, and Dependabot marked alerts #236 and #237 fixed at
  2026-08-03T07:05:28Z.
- **Confidence:** High. The affected package is used only by development
  tooling in this repository, and upstream published dedicated maintained-line
  fixes for both selected majors.

**changeType:** fix
**releaseIntent:** none
**releaseScope:** none

### SEC-011: Remediate repository-wide JavaScript dependency advisories

- **Status:** Merged 2026-08-04 via PR #3528
- **Pull Request:** [#3528](https://github.com/eddacraft/anvil-001/pull/3528)
  (rebase-merged into `main`).
- **Intent:** Clear Dependabot alerts
  [#238](https://github.com/eddacraft/anvil-001/security/dependabot/238)
  through
  [#248](https://github.com/eddacraft/anvil-001/security/dependabot/248),
  plus the repository-wide audit findings for `socket.io-parser` and the
  DeepSec `undici` consumers, in one CI-atomic remediation.
- **Expected Outcome:** Root and DeepSec dependency graphs resolve
  `fast-uri` to 3.1.5, `ip-address` to 10.4.0, and
  `socket.io-parser` to 4.2.7. Root `node-gyp` resolves `undici` to
  6.28.0; DeepSec's `@vercel/sandbox` and
  `@earendil-works/pi-coding-agent` consumers resolve it to 7.29.0 and
  8.9.0 respectively.
- **In Scope:** Consumer-specific root and DeepSec dependency overrides,
  generated lockfiles, and supersession of the three package-specific draft
  pull requests.
- **Out of Scope:** Runtime feature changes, dependency replacement, audit
  scheduling, advisory dismissal, unrelated dependency refreshes, merging,
  and release claims.
- **Dependencies:** None. Every selected release remains in the direct
  consumer's declared major line. The
  `@earendil-works/pi-coding-agent` exact `undici` 8.5.0 pin is
  intentionally overridden to the patched same-major 8.9.0 release.
- **Validation:** Clean frozen installs for the root and DeepSec;
  dependency-tree and lockfile assertions for all five vulnerable packages;
  APS, documentation, formatting, lint, typecheck, JavaScript, and Rust
  workspace gates; GitHub's repository-wide Dependency Audit check on the pull
  request. A task-owned, offline invocation of the installed pi 0.79.10 CLI
  reached `--help` successfully after configuring its imported `undici`
  dispatcher, covering the intentional exact-pin override without credentials
  or an external request.
- **Evidence:** Council `council-1cd25549` converged after scoped re-review,
  and independent verification passed with no blockers. All 30 applicable
  GitHub checks passed, including both repository-wide dependency audits, Node
  unit tests, and the Rust test suite. PR #3528 was rebase-merged into `main`
  as `3a46f2fab`; GitHub then marked Dependabot alerts #238 through #248 fixed
  between 2026-08-04T13:42:20Z and 2026-08-04T13:42:23Z.
- **Confidence:** High. The vulnerable paths are transitive development or
  security-tooling dependencies, and each override is scoped to its declaring
  consumer rather than forcing a repository-wide major.

**changeType:** fix
**releaseIntent:** none
**releaseScope:** none

### SEC-012: Make the licence entitlement claim authoritative and fail closed

- **Status:** In Progress — operator-authorised 2026-08-20. Filed 2026-08-19
  from the auth/authz plan review; promoted here from
  `beta-account-activity`'s Future-work section (reserved id BACT-014), which
  had no authorised home for it while BACT is Done 12/12.
- **Priority:** P1 access-control correctness.
- **Severity restated 2026-08-20 (implementation found the earlier framing
  overstated):** SEC-009's gate *does* discriminate. `docs.access` targets
  `accountTier in_set [plan-beta, plan-pro, plan-enterprise]` with
  `defaultVariant: disabled`, and `apps/docs-shell`'s local set matches that
  targeting exactly. The defect was never "the gate admits everyone" — it was
  that **the claim feeding the gate was fabricated when absent**:
  `verifyLicence` invented `'beta'` for a claimless token and promoted a stale
  `tier: 'pro'` into `plan`. Since every real account is `plan = beta`, actual
  exposure was tokens carrying no trustworthy plan claim, not the whole
  population.
- **Intent:** Finish the ADR-121 / OQ-C migration so a single entitlement claim
  governs private-docs access, no verifier fills in a permissive default, and
  the two edge verifiers agree.
- **Expected Outcome:** _(delivered 2026-08-20)_
  - Both **live** verifiers — `verifyLicence`
    (`apps/anvil-api/src/lib/licence.ts`) and `apps/docs-shell/lib/jwt.ts` —
    resolve the entitlement claim by one shared rule: `plan` verbatim when
    present; otherwise `beta` **only** for the exact legacy shape
    `tier === 'pro'`; otherwise `null`, which the gate denies.
  - `verifyLicence` fails **closed** — `LicenceClaims.plan` is
    `string | null` on the verify side and no permissive default is
    fabricated. `signLicence` throws rather than mint a licence with no plan.
- **`apps/docs-site` deliberately NOT touched — the app is retired.** Its
  `vercel.json` has carried `--always-skip` since 2026-07-08 (`847436623`,
  *"skip retired docs-site builds"*), which
  `tools/scripts/vercel-ignore-build.sh` documents as the flag "for retired
  Vercel projects". Its `middleware.ts` entitlement gate therefore never runs.
  This item's original Files list — inherited from BACT-014 — named that
  middleware as a fix target, and the BACT-013 advisory about docs-site's
  missing tests was really an advisory about dead code. **Corrected here:
  fixing or testing a retired app buys no security and costs maintenance.**
  See the residual note below for what should actually happen to it.
- **Live request path, for the record:** `apps/docs-shell` holds the
  entitlement gate (`verifyLicense`), then proxies to
  `apps/anvil-docs-private` with an `x-docs-upstream-secret` header;
  `anvil-docs-private/middleware.ts` checks only that shared secret and serves
  the static build. `docs-site` is not in this path.
- **Deliberately NOT in this item (see Non-scope):** `signLicence` still emits
  the `tier` compat alias. Removing it while pre-BACT-013 licences are live
  would break sessions; those expire ~2026-11-11 (90-day TTL), after which the
  alias and the `tier === 'pro'` branch in all three verifiers can be deleted
  outright. Each site carries a comment saying so.
- **Open Question — ANSWERED 2026-08-20** (design session; see
  `plans/specs/2026-08-20-entitlement-substrate-and-rbac-deferral.md`):
  `plan` is the entitlement axis and `scopes` the capability axis, both fail
  closed. Do not re-open this question in CIB-141 or CIB-143 — it was answered
  once, for all three.
- **Follow-up owed (not this item):** decide whether `apps/docs-site` is
  deleted or un-retired. It has been un-deployable for six weeks yet is still
  actively maintained — its `sidebars/anvil.ts` was updated alongside
  `anvil-docs-private`'s as recently as 2026-08-20 (`c6d81c10c`, DOCDEF-005) —
  and its README still claims **Status: Live**, reviewed 2026-08-03, a month
  *after* retirement. Either state is fine; the current one costs work on
  something that cannot ship and misleads anyone reading the README or this
  module's old Files list.
- **Known residual (not a security gap):** `apps/docs-shell` still expresses
  the entitled-plan set as a local constant rather than importing the
  catalogue, because it has no workspace dependencies (only `jose` + `next`)
  and adding `@eddacraft/anvil-runtime` to reach `resolveFlag` would widen what
  an edge bundle pulls. The set matches `docs.access` targeting today and both
  sides carry a comment pointing at the other. Sourcing it from the catalogue
  is drift-prevention, and belongs with the alias retirement.
- **Non-scope / do not:** do not widen `beta_users.plan`'s CHECK to invent
  commercial plans; do not remove the `tier` alias before outstanding
  dual-claim tokens have expired; do not redesign the flag catalogue.
- **Files:** `apps/anvil-api/src/lib/licence.ts`,
  `apps/docs-shell/lib/jwt.ts` (+ their tests). **Not**
  `apps/docs-site/**` — retired, see above.
- **Validation:** anvil-api licence/JWT tests for the new claim shape and the
  fail-closed path; `pnpm --filter @eddacraft/docs-shell test`; new docs-site
  middleware tests; a grep proving no production `payload.tier` reader remains.
- **Dependencies:** BACT-013 shipped (2026-08-13). Needs a deprecation window
  long enough that pre-BACT-013 tokens (which carry both claims) have expired.
- **Identified From:** BACT-013 independent verification advisories (PR #3839,
  2026-08-13), re-verified live 2026-08-19 during the auth/authz plan review.
- **Coordinates with:** SEC-009 (residual), CIB-141, CIB-143, ADR-121
  decision 6.
- **Evidence (2026-08-20):** `anvil-api` 760/760 pass (was 759 — the plan-claim
  block grew from 4 cases to 8, two of them inversions of the previous
  behaviour); `docs-shell` 54/54, including three new cases covering the
  claim rule and a stale fixture correction (`tier: 'beta'` with no `plan` is
  a shape `signLicence` has never minted). Typecheck clean via nx for both
  projects; `format:check` clean; `lint:check` 0 errors.
- **Confidence:** high — landed with guard coverage on every changed decision.

**changeType:** fix
**releaseIntent:** candidate
**releaseScope:** patch
**releaseNote:**

- **audience:** operator
- **type:** security
- **text:** Licence verification now reads the `plan` entitlement claim and
  never fabricates one: a token with no trustworthy claim is denied rather
  than defaulted, and a pre-BACT-013 `tier` is de-escalated to the plan the
  account actually holds instead of being trusted.

### SEC-013: Licence-authenticated routes must re-check account status

- **Status:** Draft — filed 2026-08-19 from the auth/authz plan review; **not
  self-authorised**.
- **Priority:** P2 revocation completeness — bounded by the licence TTL, not
  unbounded.
- **Intent:** Close the SEC-007 residual at the route layer. `POST /auth/verify`
  re-reads the account and rejects a subject whose `status` is not `active`;
  routes added after SEC-007 do not.
- **Expected Outcome:** every licence-authenticated route resolves the account
  and rejects non-`active` subjects, rather than treating a valid signature as
  sufficient. `apps/anvil-api/src/routes/account-activity.ts` is the known
  instance — it calls `verifyLicence` and proceeds on the claims alone, so a
  suspended account's unexpired licence still writes activity for the remainder
  of the licence TTL. The check belongs in shared licence-auth middleware so
  the next route added inherits it instead of repeating the omission.
- **Non-scope / do not:** do not build a `jti` deny-list, and do not shorten the
  licence TTL — both were considered and left out of SEC-007 for the same
  reason; they need a licensing-stream decision, not a route fix.
- **Files:** `apps/anvil-api/src/routes/account-activity.ts`, a shared
  licence-auth helper (new), `apps/anvil-api/src/routes/auth.ts` (reference
  implementation of the status check).
- **Validation:** a test proving a suspended account's still-valid licence is
  rejected by the account-activity ingest; a test proving an active account is
  unaffected; an inventory assertion (or grep) that no route calls
  `verifyLicence` without the status check.
- **Identified From:** SEC-007's restated known-follow-up; found live
  2026-08-19 during the auth/authz plan review.
- **Coordinates with:** SEC-007 (parent remediation), BACT-009 (added the
  affected route).
- **Confidence:** high — the gap is concrete and the fix pattern already exists
  in `/auth/verify`.

**changeType:** fix
**releaseIntent:** candidate
**releaseScope:** patch
