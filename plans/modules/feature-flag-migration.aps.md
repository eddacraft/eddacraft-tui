<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Feature Flag Migration

| Scope  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| FLAGM  | —     | medium   | Draft  |

## Purpose

Retire the ad-hoc feature-flag-like controls catalogued as **migrate** in
`docs/guides/feature-flag-inventory.md` and replace them with shared-manifest
flags resolved through the FLAGS model. FLAGS-008 proved the exemplar path on
CLI licence gating and docs access; this module finishes the job across the
remaining ad-hoc switches so no new surface keeps inventing its own gating
mechanism.

**Problem:** The inventory lists five existing ad-hoc controls classified as
migrate. Two (CLI licence gating, docs access) now have exemplar flag wiring
from FLAGS-008 but still rely on the original ad-hoc checks alongside the
flag. Three (`ANVIL_DEV` bypass, beta access scopes, and the hard-coded
`requires_auth()` command list) have no flag-backed path yet. Leaving the
ad-hoc checks in place keeps two sources of truth for the same decision.

## In Scope

- Replace the hard-coded `requires_auth()` command list in the Rust CLI with
  flag-driven per-command entitlement decisions
- Replace the `ANVIL_DEV=1` raw env-var check with a local operator override
  on `cli.licence-gate`
- Move docs-site `/anvil` gating to evaluate through the shared resolver (not
  the current inline edge evaluator) once a docs-side snapshot loader exists
- Migrate `ALLOWED_SCOPES` in `apps/anvil-api/` to per-scope entitlement
  flags evaluated via the shared resolver
- Parity tests proving the old and new behaviour are equivalent for the
  migration window

## Out of Scope

- Items classified **defer** in the inventory (`ADMIN_KEY`, policy profiles,
  per-policy toggles, OPA agent orchestration rollout)
- New capabilities classified **adopt** (dashboard, AI builder, tier-based
  product capabilities) — those consume the shared model directly when built
- Changing the evaluation semantics for existing controls; migration must be
  behaviour-preserving on day one

## Interfaces

**Depends on:**

- `feature-flagging` (FLAGS, Complete) — shared manifest, resolver, telemetry
- `beta-auth-streamline` (BAUTH, Complete) — `plan`/`tier` claims on sessions
- `docs-auth-gating` (DOCSAUTH, Complete) — existing middleware surface
- `rust-cli` (RCLI) — host for `requires_auth()` and `ANVIL_DEV` handling

**Exposes:**

- `cli.licence-gate` — entitlement flag now authoritative for CLI per-command
  gating (was advisory in FLAGS-008)
- `cli.dev-bypass` — local-override contract that subsumes `ANVIL_DEV=1`
- `api.scope.<name>` — per-scope entitlement flags replacing `ALLOWED_SCOPES`
- `docs.access` — resolver-backed evaluation (replaces inline edge stub)

## Acceptance Criteria

- [ ] `requires_auth()` is removed or reduced to a flag lookup; the command
      table is defined as flag metadata or derived from the manifest
- [ ] `ANVIL_DEV=1` is documented as a local override shortcut and resolves
      through the shared resolver's local-override precedence
- [ ] Docs middleware resolves `docs.access` through the shared resolver once
      a docs-side snapshot distribution lands; inline stub is removed
- [ ] `ALLOWED_SCOPES` in the API is derived from the flag manifest, not a
      hard-coded constant
- [ ] Parity tests cover the old-vs-new decision for at least one enabled,
      one disabled, and one default case per migrated control
- [ ] Telemetry for migrated flags is visible in the observability pipeline
      (minimal OTEL first-use metrics, per the FLAGS contract)

## Constraints

- Migration must be behaviour-preserving at rollout — same inputs must produce
  the same allow/deny outcome, verified by parity tests
- No new ad-hoc env-var or constant-list gating added during migration
- Local overrides keep precedence over targeting rules (FLAGS resolver
  precedence model already guarantees this)
- Docs middleware must keep working inside the Vercel edge runtime — the
  snapshot loader must either ship as an edge-compatible module or the flag
  must remain inline until such a loader exists

## Design Spec

_None yet — see FLAGS design spec at
`plans/specs/2026-04-09-feature-flagging-design.md` and the inventory at
`docs/guides/feature-flag-inventory.md` for background. A short migration
design note will land with FLAGM-001 if the approach diverges from FLAGS-008._

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Inventory of migrate targets is explicit
- [ ] Per-control migration approach is agreed (FLAGM-001)
- [ ] Parity-test approach for each control is agreed (FLAGM-001)
- [ ] Docs-side snapshot loader path is decided (blocks FLAGM-004)

## Risks & Mitigations

| Risk                                                      | Mitigation                                                                         |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Migration changes behaviour silently                      | Parity tests per control before cutting over; keep the old check + new flag dual-evaluating for one release |
| Edge runtime can't load the workspace runtime package     | Keep the inline evaluator until a docs-side snapshot loader exists; revisit in FLAGM-004 |
| Per-command flag lookup regresses CLI startup latency     | Resolve flags once at session start from the cached snapshot; avoid per-command I/O |
| API scope flags multiply manifest noise                   | Group under a single namespace (`api.scope.*`) and document retirement triggers    |
| `ANVIL_DEV` users lose the shortcut                       | Document the new local-override syntax and keep the env var as a compatibility shim for one release |

## Tasks

### FLAGM-001: Agree migration approach and parity-test pattern

- **Intent:** Before changing any runtime, agree per-control how the flag
  replaces the ad-hoc check and how parity is proven.
- **Expected Outcome:** A short design note documents, per migrate control,
  the flag key, evaluation context, dual-evaluation window, parity-test
  shape, and rollback path.
- **Scope:** `docs/guides/feature-flag-inventory.md`, `plans/specs/`
- **Non-scope:** Any code change
- **Validation:** `grep -q "parity test" docs/guides/feature-flag-inventory.md`
- **Confidence:** high

### FLAGM-002: Migrate `requires_auth()` to flag-driven command gating

- **Intent:** Remove the hard-coded per-command allow/deny table from
  `crates/anvil-cli/src/main.rs` and resolve it through `cli.licence-gate`
  plus per-command metadata on the flag or manifest.
- **Expected Outcome:** Each gated command resolves its access decision
  through the shared resolver; the hard-coded command list is gone or
  reduced to a flag lookup.
- **Scope:** `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/src/feature_flags.rs`, `packages/anvil/runtime/`
- **Non-scope:** Changing which commands are gated
- **Dependencies:** FLAGM-001
- **Validation:** `cargo test -p eddacraft-anvil --bin anvil feature_flags`
- **Confidence:** medium

### FLAGM-003: Replace `ANVIL_DEV=1` with local-override on `cli.licence-gate`

- **Intent:** Route developer bypass through the shared resolver's local-
  override path instead of a raw env-var short-circuit.
- **Expected Outcome:** `ANVIL_DEV=1` sets a local override that the resolver
  honours; the direct `check_auth()` env bypass is removed; telemetry records
  the override reason.
- **Scope:** `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/src/feature_flags.rs`
- **Non-scope:** New developer tooling UX
- **Dependencies:** FLAGM-002
- **Validation:** `cargo test -p eddacraft-anvil --bin anvil feature_flags`
- **Confidence:** high

### FLAGM-004: Move docs `/anvil` gate onto shared resolver

- **Intent:** Replace the inline edge evaluator in
  `apps/docs-site/lib/feature-flags.ts` with a resolver-backed evaluation
  once a docs-side snapshot loader exists.
- **Expected Outcome:** The docs middleware calls the shared resolver (edge-
  compatible entry point) with the session's evaluation context; the inline
  stub is deleted.
- **Scope:** `apps/docs-site/middleware.ts`, `apps/docs-site/lib/feature-flags.ts`, `packages/anvil/runtime/`
- **Non-scope:** Full snapshot distribution beyond what this flag needs
- **Dependencies:** FLAGM-001, FLAGS snapshot publication path
- **Validation:** `pnpm test -- --runInBand feature-flag-exemplars`
- **Confidence:** low

### FLAGM-005: Migrate `ALLOWED_SCOPES` to per-scope entitlement flags

- **Intent:** Stop hard-coding accepted API scope strings; derive them from
  the flag manifest via per-scope entitlement flags.
- **Expected Outcome:** `ALLOWED_SCOPES` is removed or derived from manifest
  metadata; each accepted scope has a flag key; admin routes resolve the
  flag per request instead of consulting the constant list.
- **Scope:** `apps/anvil-api/src/routes/admin.ts`, `packages/anvil/runtime/`, `packages/anvil/contracts/`
- **Non-scope:** Changing the `access_tokens` table schema
- **Dependencies:** FLAGM-001
- **Validation:** `pnpm test -- --runInBand feature-flag-scopes`
- **Confidence:** medium

### FLAGM-006: Retire dual-evaluation shims and close the migration

- **Intent:** After each migrated control has run dual-evaluated for one
  release, delete the legacy check and the parity-test scaffolding.
- **Expected Outcome:** Inventory `docs/guides/feature-flag-inventory.md` is
  updated to reflect that migrate-class controls are retired; the shared
  flag is the only source of truth per control.
- **Scope:** `docs/guides/feature-flag-inventory.md`, `crates/anvil-cli/`, `apps/docs-site/`, `apps/anvil-api/`
- **Non-scope:** New flags
- **Dependencies:** FLAGM-002, FLAGM-003, FLAGM-004, FLAGM-005
- **Validation:** `grep -Eq "retired|complete" docs/guides/feature-flag-inventory.md`
- **Confidence:** high
