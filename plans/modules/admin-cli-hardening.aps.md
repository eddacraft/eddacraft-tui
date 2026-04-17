<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Admin CLI Hardening

| Scope     | Owner  | Priority | Status   |
| --------- | ------ | -------- | -------- |
| ADMINCLIH | @aneki | medium   | Proposed |

## Purpose

Close the structural gaps surfaced by the [council review on PR #944](https://github.com/eddacraft/anvil-001/pull/944)
and the follow-up council review of this module (#946). ADMINCLI delivered the
operator CLI; this module hardens the two operationally risky flows
(`send-migration`, audit attribution) and adds one defensive quality
improvement (response validation) identified during close-out.

**Problem:** Three issues were deferred from the ADMINCLI close-out review
because they needed larger changes than fit the original module:

1. `send-migration` fetches a dry-run preview and the real-send payload in two
   separate API calls. The recipient cohort can drift between them (new
   imports, state changes, another operator working concurrently). The
   operator confirms against the preview list but may send to a different set.
   For an irreversible action this is the highest-severity operational risk.

2. `X-Admin-Actor` is fully client-controlled — any bearer-key holder can set
   it to any string. Audit log attribution is self-reported, not
   authenticated. Acceptable under the current "one shared admin key" threat
   model, but the audit log's evidentiary value is weaker than it appears.

3. API responses are consumed via unchecked TypeScript generic casts
   (`client.post<DryRunResponse>(…)`). A malformed server response crashes
   inside the table renderer rather than surfacing a clean error. Low
   frequency but high confusion when it happens.

**Solution:** Three focused work items. Server-side snapshot token for
send-migration (the big one). Per-operator admin keys so the server logs
actor from identity, not the `X-Admin-Actor` header. Zod (or equivalent
structural) validation on response payloads.

**Design Spec:** — (add one if the snapshot-token design grows beyond a small
API contract tweak)

## In Scope

- Server-side cohort snapshot for `POST /admin/send-migration` so the
  preview and send reference the same recipient set
- Per-operator admin keys (replacing the single shared `ADMIN_KEY`) with
  server-side mapping to an actor identity
- Runtime validation of admin-API responses on the CLI side
- Backwards-compatible rollout — existing operators can keep using the
  shared key during transition, with explicit cutover criteria

## Out of Scope (v1)

- Web admin panel / OAuth-based admin flow — separate initiative
- Full RBAC on admin endpoints — all admin operators remain equivalent
- Per-operator rate limits — tracked in a follow-up issue; not blocking
  but means per-key blast radius is bounded only by audit-log review
- CLI-side session caching or dotfile-based config
- Keychain-integrated local key storage — runbook covers env-var guidance;
  a loader is a follow-up

## Interfaces

**Depends on:**

- `apps/anvil-api/src/routes/admin.ts`, `apps/anvil-api/src/routes/admin-schemas.ts`
- `apps/anvil-api/src/middleware/admin-auth.ts`
- `apps/anvil-api/src/db/queries.ts` (audit writer, admin-key lookup)
- `apps/admin-cli/src/client.ts`, `apps/admin-cli/src/commands/send-migration.ts`

**Exposes:**

- Required `previewToken` field on the `POST /admin/send-migration` dry-run
  response and on the real-send request; single-use, TTL ≤10 min, opaque,
  bound to the creating key/actor
- `DriftDiffResponse` schema (added to `admin-schemas.ts`) describing the
  409 cohort-mismatch payload
- `admin_keys` table keyed on hashed key with columns `hashed_key` (UNIQUE),
  `actor_email`, `created_at`, `revoked_at`, `note`
- `admin_keys_audit` append-only table recording key inserts and revocations
  with `pulumi_commit_sha` and the actor who authorised the change
- `auth_method` column on `audit_log` (`"shared" | "per_operator"`)
- `send_migration_snapshots` table (or Redis/KV equivalent) holding the
  snapshot payload and `consumed_at`
- CLI-side response parser/validator module

## Boundary Rules

### Snapshot token (ADMINCLIH-001)

- Token is a server-generated random opaque ID (≥128 bits of entropy), not a
  signed payload. Stored server-side alongside the snapshot.
- Token is **required** on `POST /admin/send-migration` when `dryRun: false`.
  Requests without a token are rejected with 400 and a specific error code
  (`preview_token_required`). There is no permanent token-less fallback.
- Token is **single-use**. Consumed atomically on first successful real-send.
  Subsequent uses return 410 with `preview_token_consumed`.
- Token is **bound to the creator**: the snapshot row stores
  `created_by_actor` (and, when available, `created_by_key_id`). Real-send
  rejects with 403 `preview_token_actor_mismatch` if the caller differs.
- Token TTL ≤10 minutes. Expired tokens return **410 `preview_token_expired`**
  — distinct from 409 cohort drift — so the CLI can differentiate recovery
  paths.
- Cohort drift returns **409 `cohort_drift`** with a `DriftDiffResponse` body
  (`added: string[], removed: string[]`). CLI renders this before exiting
  non-zero.
- If the snapshot row is missing (cache/DB loss, eviction), real-send is
  rejected with 410 `preview_token_missing`. No silent recompute. The
  operator re-runs dry-run.

### Per-operator keys (ADMINCLIH-002)

- Admin keys are generated out-of-band (≥256 bits of entropy) and stored at
  rest as `hashed_key = HMAC-SHA-256(pepper, key)`, where the pepper is a
  server-side secret held in env/Vercel config (not the DB). Rationale:
  admin keys are high-entropy, so a keyed hash gives indexed-lookup
  performance without weakening against offline attack on a DB leak.
- `hashed_key` is UNIQUE. Multiple active (non-revoked) rows per
  `actor_email` are allowed (rotation) but the CLI must present exactly one
  key per invocation.
- `admin-auth` middleware hashes the presented bearer and performs a single
  parameterised `SELECT ... WHERE hashed_key = $1 AND revoked_at IS NULL`.
  Equality comparison uses constant-time primitives; no per-row iteration
  over the keys table.
- A request presenting a hashed key that matches a revoked row is rejected
  with 401 `admin_key_revoked`.
- Authentication failures (unknown key, revoked key, malformed bearer) are
  recorded in `audit_log` with `actor: null`, `outcome: "rejected_*"`, and
  the hashed bearer (never the plaintext bearer) so repeated attempts can be
  correlated.
- During dual-auth rollout, requests authenticated via the shared
  `ADMIN_KEY` **ignore `X-Admin-Actor` entirely**. The audit row records
  `actor: "shared-key@anvil"` (sentinel) and `auth_method: "shared"`. This
  eliminates the attribution-forgery path during the rollout window.
- Key provisioning is via reviewed Pulumi/IaC change (two-person rule).
  Every insert and every `revoked_at` update writes an `admin_keys_audit`
  row containing the Pulumi commit SHA. The `admin_keys` table is
  append-only from the app (no DELETEs).
- If the `admin_keys` lookup throws (table missing, DB error), the
  middleware falls back to shared-key comparison and stamps
  `auth_method: "shared"`. This prevents a DB hiccup from taking down the
  entire admin surface.
- Per-operator-key adoption is tracked by a metric derived from `auth_method`
  on every request. The shared-key path is removed in a follow-up module
  when the shared-key request rate is zero for ≥7 consecutive days. The
  feature flag guarding the dual path is named `admin.per_operator_keys`
  with an `expiryOrReviewDate` aligned to the cutover target.

### CLI response validation (ADMINCLIH-003)

- Response schemas (`DryRunResponse`, `SendResponse`, `ListResponse`,
  `ShowResponse`, `AuditResponse`, `DriftDiffResponse`) live in
  `admin-schemas.ts` and are imported by both server and CLI. Schema
  changes require both server and CLI tests to pass.
- CLI response validation failures raise `AdminError` with **exit code 5**
  (distinct from `2`, which is reserved for 5xx/network errors). The error
  message names the offending field and includes an expected-vs-actual
  shape summary.

### Other

- No breaking changes to the `--json` output format.

## Acceptance Criteria

### Snapshot token (ADMINCLIH-001)

- [ ] Dry-run response includes a `previewToken`; real-send with
      `dryRun: false` requires it and rejects without it (400
      `preview_token_required`)
- [ ] Server rejects with 409 `cohort_drift` and returns `DriftDiffResponse`
      when the cohort would differ; CLI renders the diff before exit
- [ ] Server rejects with 410 `preview_token_expired` after TTL;
      CLI surfaces a distinct message directing the operator to re-run
      `--dry-run`
- [ ] Server rejects with 410 `preview_token_consumed` on second use of the
      same token
- [ ] Server rejects with 403 `preview_token_actor_mismatch` when the caller
      differs from the snapshot creator; covered by a cross-operator test

### Per-operator keys (ADMINCLIH-002)

- [ ] Operators can be provisioned with per-operator keys via Pulumi; the
      server records the authenticated actor from the key row and ignores
      `X-Admin-Actor` on per-operator requests
- [ ] Requests authenticated via shared `ADMIN_KEY` ignore `X-Admin-Actor`
      and record `actor: "shared-key@anvil"`, `auth_method: "shared"`
- [ ] Revoked keys return 401 `admin_key_revoked` and the rejection is
      logged in `audit_log` with `outcome: "rejected_revoked"`
- [ ] Unknown/malformed bearers return 401 and are logged in `audit_log`
      with the hashed bearer
- [ ] Middleware falls back to shared-key comparison if the `admin_keys`
      lookup throws (DB error scenario has a test)
- [ ] `admin_keys` has a UNIQUE constraint on `hashed_key`; inserting a
      duplicate is rejected
- [ ] Every key insert/revoke writes an `admin_keys_audit` row with the
      Pulumi commit SHA

### CLI response validation (ADMINCLIH-003)

- [ ] Response schemas exist in `admin-schemas.ts` and are the single
      source of truth for both server and CLI
- [ ] CLI rejects malformed admin-API responses with **exit code 5** and a
      message naming the offending field; tests cover missing/mistyped
      fields on `DryRunResponse`, `SendResponse`, `ListResponse`,
      `ShowResponse`, `AuditResponse`

### Runbook

- [ ] Runbook covers: per-operator key provisioning, key revocation,
      shared `ADMIN_KEY` rotation, 409 cohort-drift recovery, 410
      token-expiry recovery, and env-var-based local key storage (with
      guidance against inline `export` in shell history)

## Risks & Mitigations

| Risk | Mitigation |
| ---- | ---------- |
| Snapshot storage bloats under migration burst | DB or KV with TTL ≤10 min; size-capped; missing snapshot is a hard error (no recompute) |
| Per-operator key rollout leaves a window where audit rows are mixed | `auth_method` column on `audit_log`; shared-key rows use sentinel actor; dashboards filter by `auth_method` |
| Attribution-forgery residual during dual-auth rollout | Shared-key path ignores `X-Admin-Actor` entirely, so no forgery during the window |
| Audit-email filters silently miss shared-key rows | Runbook documents the dual-path query pattern; optional DB view normalises both |
| CLI response validator becomes brittle as server evolves | Schemas live in `admin-schemas.ts`, imported by both server and CLI; schema change requires both test suites to pass |
| Shared-key users break mid-rollout | `admin.per_operator_keys` feature flag; lookup-failure falls back to shared-key; shared path removed only after 7 consecutive zero-shared days |
| Pulumi/IaC compromise during dual-auth window implies admin escalation | Two-person rule on IaC changes; `admin_keys_audit` table records every provisioning event with commit SHA; short rollout window |

## Tasks

### Phase A: Snapshot the send-migration cohort

#### ADMINCLIH-001: Snapshot token for send-migration

- **Intent:** Make `send-migration` atomic across preview and send by
  anchoring both calls to a server-side recipient snapshot that is
  required, single-use, TTL-bound, and bound to the creating actor
- **Expected Outcome:** Dry-run returns a required `previewToken`;
  real-send without the token returns 400, with a consumed/expired/missing
  token returns 410, with a cohort mismatch returns 409 and a
  `DriftDiffResponse`, with an actor mismatch returns 403. CLI surfaces
  each distinct error with a tailored recovery message.
- **Scope:** `apps/anvil-api/src/routes/`, `apps/anvil-api/src/db/`,
  `apps/admin-cli/src/commands/send-migration.ts`,
  `apps/admin-cli/src/__tests__/send-migration.test.ts`
- **Non-scope:** Generalising the snapshot pattern to other endpoints
- **Files:**
  - `apps/anvil-api/src/routes/admin-schemas.ts` (new `DriftDiffResponse`
    and updated request/response schemas)
  - `apps/anvil-api/src/routes/admin.ts`
  - `apps/anvil-api/src/db/queries.ts` (snapshot storage + consumed_at)
  - `apps/anvil-api/src/db/migrations/NNNN-admin-send-migration-snapshots.sql`
  - `apps/admin-cli/src/commands/send-migration.ts`
  - `apps/admin-cli/src/__tests__/send-migration.test.ts`
  - `docs/runbooks/admin-cli.md`
- **Dependencies:** ADMINCLI module merged (v1 CLI shipped on `dev`);
  benefits materially from ADMINCLIH-002 landing first so token binding
  can reference `created_by_key_id` rather than the self-reported actor —
  if landed first, tests cover the weaker binding case and are tightened
  post-002
- **Validation:** `pnpm -F @eddacraft/anvil-api test -- send-migration` and
  `pnpm -F @eddacraft/admin-cli test -- send-migration`; manual run against
  preview env with a mid-flow import to confirm 409 fires; manual run with
  a deliberate 11-minute pause to confirm 410 expiry fires
- **Confidence:** medium (depends on snapshot-storage choice — DB table vs
  KV)
- **Status:** Proposed

### Phase B: Authenticated operator identity

#### ADMINCLIH-002: Per-operator admin keys

- **Intent:** Let each operator carry their own admin key so audit
  attribution is authenticated rather than self-reported via
  `X-Admin-Actor`, with explicit hashing, revocation, provisioning, and
  cutover semantics
- **Expected Outcome:** New `admin_keys` table (hashed_key UNIQUE,
  actor_email, created_at, revoked_at, note); new `admin_keys_audit`
  append-only table; new `auth_method` column on `audit_log`;
  `admin-auth` middleware hashes bearers with HMAC-SHA-256 + server
  pepper, looks up via a single indexed SELECT, and rejects revoked keys
  with 401 `admin_key_revoked`; shared-key path ignores `X-Admin-Actor`
  entirely and writes sentinel `"shared-key@anvil"`; authentication
  failures are recorded in `audit_log`; feature flag
  `admin.per_operator_keys` gates the dual path; CLI unchanged for
  callers (same env vars, same header semantics on per-operator path)
- **Scope:** `apps/anvil-api/src/middleware/`, `apps/anvil-api/src/db/`,
  `apps/anvil-api/src/db/migrations/`, `apps/anvil-api/src/__tests__/`,
  Pulumi/IaC for key provisioning, `docs/runbooks/admin-cli.md`
- **Non-scope:** CLI-side provisioning flow (keys are provisioned via
  Pulumi/IaC for v1); removing the shared `ADMIN_KEY` (separate
  follow-up module gated on the 7-zero-day cutover criterion);
  per-operator rate limits
- **Files:**
  - `apps/anvil-api/src/db/migrations/NNNN-admin-keys.sql`
  - `apps/anvil-api/src/db/migrations/NNNN-admin-keys-audit.sql`
  - `apps/anvil-api/src/db/migrations/NNNN-audit-log-auth-method.sql`
  - `apps/anvil-api/src/db/queries.ts`
  - `apps/anvil-api/src/middleware/admin-auth.ts`
  - `apps/anvil-api/src/__tests__/admin.test.ts` (auth-method matrix,
    revoked-key test, DB-failure fallback test, sentinel actor test)
  - `apps/anvil-api/src/routes/admin-schemas.ts` (extend `AuditEntrySchema`
    with `auth_method`)
  - `docs/runbooks/admin-cli.md` (per-operator keys, revocation, rotation,
    local key storage guidance)
- **Dependencies:** —
- **Validation:**
  `pnpm -F @eddacraft/anvil-api test -- --testNamePattern="admin-auth"`;
  manual via curl: insert a fixture row into `admin_keys`, issue a request
  with the plaintext key, hit `GET /admin/audit` with shared-key auth and
  confirm the per-operator row shows the key's mapped email regardless of
  any `X-Admin-Actor` header
- **Confidence:** medium
- **Status:** Proposed

### Phase C: Defensive CLI parsing

#### ADMINCLIH-003: Runtime validation of admin-API responses

- **Intent:** Surface malformed admin responses as a clean CLI error
  rather than an undefined-access crash inside the renderer
- **Expected Outcome:** Response schemas added to `admin-schemas.ts` as
  the single source of truth; CLI `client` hooks validation into every
  admin response; validation failures throw `AdminError` with **exit
  code 5** and a message naming the offending field; tests inject
  malformed responses and assert the behaviour
- **Scope:** `apps/admin-cli/src/`, `apps/anvil-api/src/routes/admin-schemas.ts`
- **Non-scope:** Server-side response shape changes beyond adding
  response schemas, retries on malformed responses
- **Files:**
  - `apps/anvil-api/src/routes/admin-schemas.ts` (new `DryRunResponse`,
    `SendResponse`, `ListResponse`, `ShowResponse`, `AuditResponse`
    schemas — these do not exist today and must be created)
  - `apps/admin-cli/src/client.ts` (hook in validation, exit code 5)
  - `apps/admin-cli/src/commands/*.ts` (consume validated types)
  - `apps/admin-cli/src/__tests__/` (fixtures with broken shapes)
- **Dependencies:** ADMINCLIH-001 — -001 introduces `DriftDiffResponse`
  and mutates `DryRunResponse`/`SendResponse`; executing -003 first
  locks in stale schemas. Land after -001 or stage response-schema work
  behind -001's PR.
- **Validation:** `pnpm -F @eddacraft/admin-cli test` — new tests pass;
  `pnpm -F @eddacraft/anvil-api test` — server types unaffected;
  `tsc --noEmit` clean in both packages
- **Confidence:** high
- **Status:** Proposed
