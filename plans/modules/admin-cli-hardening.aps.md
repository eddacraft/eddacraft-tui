<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Admin CLI Hardening

| Scope     | Owner  | Priority | Status   |
| --------- | ------ | -------- | -------- |
| ADMINCLIH | @aneki | medium   | Proposed |

## Purpose

Close the structural gaps surfaced by the [council review on PR #944](https://github.com/eddacraft/anvil-001/pull/944).
ADMINCLI delivered the operator CLI; this module makes the two most operationally
risky flows (`send-migration` and audit attribution) safe against realistic
failure modes.

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
  shared key during transition

## Out of Scope (v1)

- Web admin panel / OAuth-based admin flow — separate initiative
- Full RBAC on admin endpoints — all admin operators remain equivalent
- Per-operator rate limits or per-operator audit queries
- CLI-side session caching or dotfile-based config

## Interfaces

**Depends on:**

- `apps/anvil-api/src/routes/admin.ts`, `apps/anvil-api/src/routes/admin-schemas.ts`
- `apps/anvil-api/src/middleware/adminAuth.ts`
- `apps/anvil-api/src/db/queries.ts` (audit writer, admin-key lookup)
- `apps/admin-cli/src/client.ts`, `apps/admin-cli/src/commands/send-migration.ts`

**Exposes:**

- Stable `previewToken` field on the `POST /admin/send-migration` dry-run
  response, and an optional `previewToken` field on the real-send request
- `admin_keys` table (or equivalent) keyed on hashed key with
  `actor_email`, `created_at`, `revoked_at`
- CLI-side response parser/validator module

## Boundary Rules

- Snapshot tokens are opaque to the CLI and must be short-lived (≤10 min)
- Existing `ADMIN_KEY` continues to work during rollout; new keys can be
  added additively and the legacy path removed in a follow-up once all
  operators are on per-operator keys
- CLI response validation raises a distinct exit code (reuse `2` — server
  bug) with a message that includes the expected vs actual shape summary
- No breaking changes to the `--json` output format

## Acceptance Criteria

- [ ] `send-migration` preview response includes a `previewToken`; real send
      accepts it and the server rejects with 409 if the cohort would differ
      from the snapshot
- [ ] Operators can provision a per-operator admin key; when present, the
      server records the authenticated actor from the key row (not from the
      `X-Admin-Actor` header)
- [ ] The shared `ADMIN_KEY` continues to function; when used, behaviour is
      unchanged (actor still self-reported)
- [ ] CLI rejects malformed admin-API responses with a clear message before
      rendering; new test covers missing/mistyped fields on
      `DryRunResponse` and `SendResponse`
- [ ] Runbook updated with a "per-operator keys" section and a short
      explainer on the preview-token contract

## Risks & Mitigations

| Risk | Mitigation |
| ---- | ---------- |
| Snapshot storage bloats the DB during a migration burst | Ephemeral cache (TTL ≤10 min); fall back to recomputing if missing |
| Per-operator key rollout leaves a window where some ops use shared key and audit rows are mixed | Annotate audit rows with `auth_method` set to either `"shared"` or `"per_operator"` so post-hoc queries can distinguish |
| CLI response validator becomes brittle as server evolves | Keep schemas in the shared `admin-schemas.ts` module that server + CLI both import |
| Breaking shared-key users mid-rollout | Feature-flag per-operator lookup server-side; keep legacy path until telemetry shows shared key unused |

## Tasks

### Phase A: Snapshot the send-migration cohort

#### ADMINCLIH-001: Snapshot token for send-migration

- **Intent:** Make `send-migration` atomic across preview and send by
  anchoring both calls to a server-side recipient snapshot
- **Expected Outcome:** Dry-run response includes an opaque `previewToken`;
  passing it on the real send replays the snapshot recipients; mismatch
  returns 409 with a diff summary; CLI surfaces the diff in its error
- **Scope:** `apps/anvil-api/src/routes/`, `apps/anvil-api/src/db/`,
  `apps/admin-cli/src/commands/send-migration.ts`,
  `apps/admin-cli/src/__tests__/send-migration.test.ts`
- **Non-scope:** Generalising the snapshot pattern to other endpoints
- **Files:**
  - `apps/anvil-api/src/routes/admin-schemas.ts`
  - `apps/anvil-api/src/routes/admin.ts`
  - `apps/anvil-api/src/db/queries.ts` (snapshot storage)
  - `apps/anvil-api/src/db/migrations/NNNN-admin-send-migration-snapshots.sql`
  - `apps/admin-cli/src/commands/send-migration.ts`
  - `apps/admin-cli/src/__tests__/send-migration.test.ts`
  - `docs/runbooks/admin-cli.md`
- **Dependencies:** ADMINCLI-012 (merged)
- **Validation:** `pnpm -F @eddacraft/anvil-api test -- send-migration` and
  `pnpm -F @eddacraft/admin-cli test -- send-migration`; manual run against
  preview env with a mid-flow import to confirm 409 fires
- **Confidence:** medium (depends on snapshot-storage choice)
- **Status:** Proposed

### Phase B: Authenticated operator identity

#### ADMINCLIH-002: Per-operator admin keys

- **Intent:** Let each operator carry their own admin key so audit
  attribution is authenticated rather than self-reported via
  `X-Admin-Actor`
- **Expected Outcome:** New `admin_keys` table with `hashed_key`,
  `actor_email`, `created_at`, `revoked_at`; `adminAuth` middleware looks
  up the caller's key and injects the authenticated actor into the
  request context; audit writer uses the authenticated actor when
  available; shared `ADMIN_KEY` still works and marks audit rows with
  `auth_method: "shared"`; CLI unchanged
- **Scope:** `apps/anvil-api/src/middleware/`, `apps/anvil-api/src/db/`,
  `apps/anvil-api/src/db/migrations/`, `apps/anvil-api/src/__tests__/`
- **Non-scope:** CLI-side provisioning flow (keys are provisioned via
  Pulumi/IaC for v1), removing the shared `ADMIN_KEY` (deferred to a
  follow-up once adoption ≥100%)
- **Files:**
  - `apps/anvil-api/src/db/migrations/NNNN-admin-keys.sql`
  - `apps/anvil-api/src/db/queries.ts`
  - `apps/anvil-api/src/middleware/adminAuth.ts`
  - `apps/anvil-api/src/__tests__/admin.test.ts`
  - `docs/runbooks/admin-cli.md` (add "per-operator keys" section)
- **Dependencies:** —
- **Validation:**
  `pnpm -F @eddacraft/anvil-api test -- --testNamePattern="adminAuth"`;
  manual: provision a per-op key, run `anvil-admin audit`, confirm
  `actor` column shows the key's mapped email regardless of
  `ANVIL_ADMIN_ACTOR`
- **Confidence:** medium
- **Status:** Proposed

### Phase C: Defensive CLI parsing

#### ADMINCLIH-003: Runtime validation of admin-API responses

- **Intent:** Surface malformed admin responses as a clean CLI error
  rather than an undefined-access crash inside the renderer
- **Expected Outcome:** A shared parser (Zod or manual) validates
  `DryRunResponse`, `SendResponse`, `ListResponse`, `ShowResponse`,
  `AuditResponse` at the CLI boundary; validation failures throw
  `AdminError` with a non-zero exit code and a message naming the
  offending field; covered by tests that inject malformed responses
- **Scope:** `apps/admin-cli/src/`, shared schemas in
  `apps/anvil-api/src/routes/admin-schemas.ts`
- **Non-scope:** Server-side response shape changes, retries on
  malformed responses
- **Files:**
  - `apps/admin-cli/src/client.ts` (hook in validation)
  - `apps/admin-cli/src/commands/*.ts` (use validated types)
  - `apps/admin-cli/src/__tests__/` (new fixtures with broken shapes)
- **Dependencies:** —
- **Validation:** `pnpm -F @eddacraft/admin-cli test` — new tests pass;
  `tsc --noEmit` clean
- **Confidence:** high
- **Status:** Proposed
