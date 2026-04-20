<!-- APS: Design spec for FLAGM module -->

# Feature Flag Migration Design

Date: 2026-04-20
Module: `FLAGM`
Status: Ready
Supersedes: Nothing (sequel to
`plans/specs/2026-04-09-feature-flagging-design.md`)

## Goal

Retire every control classified **migrate** in
`docs/guides/feature-flag-inventory.md` and replace it with a
shared-manifest flag resolved through the FLAGS model. Each cutover must be
behaviour-preserving and provable via a parity test.

## Context

FLAGS shipped the shared model and two exemplar wirings: CLI
`cli.licence-gate` and docs `docs.access`. Both exemplars still sit
alongside the original ad-hoc checks — the inventory entry for each
control is now a two-source-of-truth arrangement. The inventory also lists
two migrate targets with no flag-backed path yet: `ANVIL_DEV=1` and
`ALLOWED_SCOPES` in the API. The CLI `requires_auth()` command table is
treated here as part of the broader `cli.licence-gate` migration rather
than as a separate inventory control.

This spec fixes the approach for all four migrate controls, so the
remaining tasks in FLAGM can be executed without re-litigating the
strategy.

## Migration pattern

Every migrate control moves through the same four phases. The phase labels
are used in the per-control tables below.

1. **Dual-evaluate.** Keep the ad-hoc check in place and add a parallel
   `resolveFlag` call. Run both on the request path and compare outcomes in
   a parity test. At this phase the flag result is advisory only — the
   legacy decision still wins.
2. **Cut over.** Switch the runtime decision to the flag result. The
   legacy check remains in-tree behind a dual-evaluate assertion (dev and
   test only) for one release so production regressions surface as test
   failures in the next CI run.
3. **Retire.** Delete the legacy check, the dual-evaluate assertion, and
   the parity-test harness for that control. Update the inventory entry to
   reflect retirement.
4. **Close.** When every migrate control is retired, close FLAGM via
   FLAGM-006 and update the inventory summary table.

## Parity-test shape

Parity tests live alongside the control they replace, not in a shared
module. Each test covers three canonical cases for the control:

- **enabled:** an input that the legacy check allows today
- **disabled:** an input that the legacy check denies today
- **default:** an input that exercises the default-variant path (missing
  claim, missing env var, missing tier)

The test asserts that the legacy decision and the flag decision agree for
every case. The test fails loudly on any divergence — divergence is a
migration bug, not flag drift.

Test shape (sketched in pseudo-code):

```
for each case in [enabled, disabled, default]:
    legacy = legacy_check(case.input)
    flag   = resolveFlag(FLAG_KEY, case.context).variant == "enabled"
    assert legacy == flag, "parity failure for {case.name}"
```

Tests use the in-process resolver and evaluation context builder — no
snapshot download or network call. Reason: parity is about evaluation
equivalence, not distribution.

## Rollback path

Every cutover commit must be independently revertible. Rules:

- A single commit contains the cutover for exactly one control
- The legacy check remains in-tree for one release after cutover
- The dual-evaluate assertion runs in dev and test, never on the production
  hot path
- If the parity test fails post-cutover, revert the cutover commit; the
  legacy check resumes authority until the flag or its targeting is fixed

FLAGM-006 is the only task allowed to delete a legacy check. No earlier
task may delete one, even if parity tests have been green for a release.

## Per-control migration plan

### `cli.licence-gate` (CLI licence-gated actions)

- **Current state (post FLAGM-002):** the flag definition in
  `crates/anvil-cli/src/feature_flags.rs` is wrapped by a CLI-local
  `CliGateFlag` that carries a `gated_commands` metadata list
  (`CLI_GATED_COMMANDS`). `main::requires_auth` delegates to
  `feature_flags::command_needs_licence_gate(command_canonical_name(cmd))`.
  The hard-coded match survives as a test-only `requires_auth_legacy`
  retained solely for parity assertions; it is scheduled for deletion in
  FLAGM-006.
- **Metadata representation decision (FLAGM-002):** option 1 — flag
  attribute, implemented as a CLI-local wrapper. The per-command list is
  a property of the CLI host, not a property of the shared flag contract,
  so it did not warrant extending the shared Rust/Zod schemas. Options 2
  (companion manifest) and 3 (targeting predicate) were rejected because
  they either duplicated the manifest for CLI-only data or overloaded the
  resolver's variant semantics to express reachability.
- **Evaluation context:** `targetingKey` = stable session identifier
  (JWT `sub` when available, email today, `"cli-session"` as the
  backwards-compatible fallback); `audience.licencePlan` and
  `audience.accountTier` plumbed from `POST /api/v1/auth/verify`.
- **Dual-evaluation window:** one release — FLAGM-002 lands the flip to
  flag authority while the legacy match remains in-tree under
  `#[cfg(test)]` as `requires_auth_legacy`. FLAGM-006 retires it.
- **Parity-test cases:**
  - **enabled:** plan `"pro"`, command `audit` → both allow
  - **disabled:** plan `"free"`, command `admin` → both deny
  - **default:** missing plan claim, command `doctor` → both allow
  - **sweep:** every representative command in `PARITY_COMMAND_CASES`
    (gated + bypass + hidden aliases) agrees for both implementations
- **Rollback:** revert the FLAGM-002 commit; `requires_auth` reverts to
  the hard-coded match.
- **Test location:** `crates/anvil-cli/src/main.rs` tests module
  (`parity_*` tests) and `crates/anvil-cli/src/feature_flags.rs`
  (`command_needs_licence_gate_*`, `gate_metadata_*`, and
  `gated_commands_are_sorted_and_unique` tests).

### `cli.dev-bypass` (`ANVIL_DEV=1` env-var bypass)

- **Current state (post FLAGM-003):** the env-var read has moved into
  `feature_flags::local_overrides_from_env`, which emits a
  `FlagOverrides.local` entry mapping `cli.licence-gate → "enabled"`
  when `ANVIL_DEV=1`. `feature_flags::cli_dev_bypass_active` runs the
  resolver with those overrides and returns the resulting
  `ResolutionDetails` when `reason = LocalOverride`. `main::check_auth`
  consults that helper; the raw env-var branch is gone. A single
  literal `ANVIL_DEV=1` enables bypass — other values (`"true"`,
  `"0"`, empty) are intentional no-ops to match the legacy check
  byte-for-byte during dual-evaluation.
- **Target decision point:** resolver sees `ANVIL_DEV=1` as a local
  override that forces `cli.licence-gate` to `"enabled"`. The env-var
  read is an override-loader step, not a branch on the auth path.
- **Evaluation context:** same as `cli.licence-gate` — the override
  supplies the variant directly; no targeting is evaluated when an
  override is present.
- **Dual-evaluation window:** one release — parity assertions live in
  `crates/anvil-cli/src/feature_flags.rs` next to the new helpers; the
  inline legacy helper `legacy_dev_bypass_active` exists only under
  `#[cfg(test)]` and retires in FLAGM-006.
- **Parity-test cases:**
  - **enabled:** `ANVIL_DEV=1`, plan `"free"` → both allow (override
    wins over targeting)
  - **disabled:** `ANVIL_DEV` unset, plan `"free"` → both deny
  - **default:** `ANVIL_DEV` unset, missing plan → both agree there
    is no bypass; the flag's default variant still resolves to
    `"enabled"` (backwards compat), but that is the gate decision,
    not the bypass decision
- **Rollback:** revert the FLAGM-003 commit; the raw env-var branch
  resumes authority.
- **Test location:** `crates/anvil-cli/src/feature_flags.rs` tests
  module (`local_overrides_from_env_*`, `cli_dev_bypass_active_*`,
  `parity_*`).

### `docs.access` (docs `/anvil` gate)

- **Current state:** `apps/docs-site/lib/feature-flags.ts` ships an inline
  edge-compatible evaluator that mirrors the shared model. The docs-site
  cannot import the workspace runtime package because Vercel edge
  middleware does not resolve it.
- **Target decision point:** edge middleware calls the shared resolver via
  a docs-side snapshot loader. This is the only migrate control that is
  blocked on infrastructure — the loader must either ship as an
  edge-compatible module or the inline evaluator remains canonical.
- **Evaluation context:** `targetingKey` = authenticated session subject;
  `audience.accountTier` from the JWT `tier` claim; backwards-compat for
  tokens minted before the claim existed must be preserved during
  dual-evaluation and re-examined at cutover (see FLAGM-004).
- **Dual-evaluation window:** two releases — edge parity is hardest to
  verify, so keep the dual-evaluate assertion live longer.
- **Parity-test cases:**
  - **enabled:** `tier=pro` → both allow
  - **disabled:** `tier=free` → both deny (inline + shared both deny)
  - **default:** missing `tier` claim → both allow today. FLAGM-004
    decides whether to flip to fail-closed at cutover and, if so, lands
    the change with the resolver swap.
- **Rollback:** revert the middleware cutover; inline evaluator resumes
  authority.
- **Test location:** `apps/docs-site/lib/feature-flags.test.ts` (new
  parity suite).

### `api.scope.*` (beta access scopes)

- **Current state (post FLAGM-005):** `apps/anvil-api/src/lib/feature-flags.ts`
  defines three entitlement flags (`api.scope.beta`, `api.scope.preview`,
  `api.scope.internal`) and exports `ALLOWED_API_SCOPES` derived from the
  manifest. `apps/anvil-api/src/routes/admin-schemas.ts` re-exports
  `ALLOWED_SCOPES` from that derived list — the hard-coded tuple is gone
  and `inviteSchema.scopes` now validates against the manifest-backed
  `API_SCOPE_NAMES`. The day-1 flag defaults resolve to `"enabled"` with
  no targeting, which is behaviour-preserving against the legacy
  constant-list check.
- **Target decision point:** for each scope string, the flag manifest is
  the single source of truth. Per-request evaluation via `resolveApiScope`
  / `isScopeAllowed` is in place for callers that need runtime entitlement
  decisions (admin routes, token-validation surface); the existing Zod
  schema still covers format validation up-front.
- **Evaluation context:** `targetingKey` defaults to `"api-anonymous"` via
  `defaultApiEvaluationContext`. Callers with a principal should build a
  richer context (token subject as `targetingKey`, plan record as
  `audience.accountTier`, role as `audience.userRole` where the scope is
  role-shaped — e.g. `internal`). Day-1 flags have no targeting rules, so
  the context does not change the decision until plan- or role-based
  rules are added post-FLAGM.
- **Dual-evaluation window:** one release — parity assertions live in
  `apps/anvil-api/src/lib/__tests__/feature-flags.test.ts` next to the
  helper. The legacy `['beta', 'preview', 'internal']` tuple exists only
  inside the test file (`LEGACY_ALLOWED_SCOPES`) and retires in FLAGM-006.
- **Parity-test cases:**
  - **enabled:** scope `"beta"` → legacy accepts (in list), flag resolves
    to `"enabled"` default → both allow
  - **disabled:** scope `"admin"` (unknown) → legacy rejects (not in
    list), flag lookup returns no backing flag → both deny
  - **default:** scope `"preview"` → legacy accepts, flag resolves to
    `"enabled"` default → both allow
  - **sweep:** every legacy scope name and a set of unknown scopes
    (`""`, `"admin"`, `"api.scope.beta"`, `"BETA"`) agree on both paths
- **Rollback:** revert the FLAGM-005 commit; `admin-schemas.ts` falls
  back to the inline constant tuple and the flag manifest goes away.
- **Test location:** `apps/anvil-api/src/lib/__tests__/feature-flags.test.ts`
  (parity + unit tests covering manifest shape, resolver behaviour, local
  overrides, and the three canonical parity cases).

## Telemetry expectations

Each migrated control emits the same first-use OTEL metric the FLAGS
contract already documents: `feature_flag.evaluated` with attributes
`flag_key`, `variant`, `reason`. Emissions should land on dual-evaluate
and stay on after retirement — retirement deletes the legacy check, not
the flag or its telemetry.

## What this spec does not decide

- The edge-compatible snapshot loader for docs-site (FLAGM-004 dep)
- Whether to flip `docs.access` from `enabled` to `disabled` for
  missing-tier tokens at cutover (FLAGM-004 will decide)
- Any re-scoping of **defer** controls in the inventory — those remain
  untouched by FLAGM

## Checklist

- [x] Per-control flag key, evaluation context, dual-evaluation window,
      parity-test cases, and rollback path are documented
- [x] Parity-test shape is defined once and referenced by each control
- [x] Inventory will reference this spec (update in the FLAGM-001 commit)
- [x] FLAGM-002 picks the per-command metadata representation
      (flag attribute via CLI-local `CliGateFlag` wrapper)
- [ ] FLAGM-004 picks the edge-compatible loader path
