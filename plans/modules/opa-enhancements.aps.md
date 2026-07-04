# Policy Authoring and Runtime UX

| ID   | Owner | Priority | Status | Progress |
| ---- | ----- | -------- | ------ | -------- |
| OPAE | —     | high     | In Progress | 8/11      |

**Last reviewed:** 2026-07-04 (POLRESET-003: contracts reconciled with
ADR-098, the accepted POLRESET-001 design gate).

> **Reset note:** the old OPAE plan mixed a broad "delightful OPA" wishlist,
> retired TypeScript paths, natural-language generation, policy debugging,
> compliance reporting, remote bundles, and PR comments. That made OPAE look
> strategically important but blocked first policy value. This module is now the
> narrow product-contract home for **regorus-backed policy authoring and runtime
> UX**. Enterprise hierarchy, lifecycle, compliance reports, remote federation,
> AI governance signals, YAML/action taxonomy, and agent orchestration stay in
> their owning modules.

## Purpose

Make user-authored and bundled policies useful in the shipping Anvil product:
validate packs before load, evaluate through the ADR-040 regorus facade, explain
failures in remediation-first language, and provide the policy outcome contract
that save-time/pre-write enforcement can route to `warn`, `fence`, or
`interrupt`.

## In Scope

- User policy pack discovery and loading through `crates/anvil-policy`.
- Regorus-backed evaluation through `crates/anvil-policy-engine`. Go OPA
  survives only as CI reference/parity tooling (`opa test policies/fixtures/`,
  regal lint, and the poleng-parity bench); the Rust OPA-subprocess path is
  removed under ADR-098 AD-1's replace-then-delete sequence.
- Local policy library/install UX for starter packs.
- Remediation-first policy result and guidance contract.
- Deterministic save-time/pre-write policy input adapter over changed-code and
  graph context.
- Enforcement-routing contract that maps policy outcomes to Anvil's existing
  `warn`, `fence`, and `interrupt` vocabulary while preserving warnings-first
  defaults.
- User-facing docs and one starter example path.

## Out of Scope

- Natural-language policy generation.
- Interactive TUI policy debugger.
- Historical impact simulator.
- Remote bundle marketplace, federation, signing, or SSO.
- Broad PR auto-comments.
- Compliance reporting and legal framework coverage.
- Enterprise hierarchy/lifecycle/rollout management.
- Tool-call interception beyond existing save-time/pre-write write-validation
  surfaces.
- YAML/action taxonomy authoring; that remains ACTAX Phase 2 after the Rego
  path works end to end.

## Interfaces

**Depends on:**

- [POLRESET](./policy-value-enforcement-reset.aps.md) — reset sequence and
  enforcement design gate.
- [POLENG](../archive/modules/policy-engine.aps.md) / ADR-040 — regorus facade,
  `PolicyInput` v1, result post-processing, and `anvil policy eval` substrate.
- [POLVAL](./policy-pack-validation.aps.md) — pack metadata, manifests,
  validation, and test enforcement.
- [CPOL](./contextual-policy-assertions.aps.md) — deterministic context and
  guidance payloads.
- [IORISK](./io-risk-controls.aps.md) — shared risk vocabulary when starter packs
  need IO/prompt-risk dimensions.
- [EXCEPT](./git-native-exceptions.aps.md) — valid exception verification before
  fencing or interrupting.
- `crates/anvil-policy`, `crates/anvil-policy-engine`, `crates/anvil-cli`,
  `crates/anvil-intercept-rules`, and `crates/anvil-intercept-protocol`.

**Exposes:**

- User policy pack discovery contract.
- Policy library/install UX contract.
- Remediation-first policy guidance contract.
- Save-time/pre-write policy input adapter contract.
- Enforcement-routing contract for `warn`, `fence`, and `interrupt`.

## Acceptance Criteria

- [ ] User-authored Rego packs validate before evaluation.
- [ ] Valid packs evaluate through `anvil-policy-engine` / regorus, not a second
      production OPA runtime.
- [ ] Policy failures include rule id, source, rationale, changed-code context,
      and remediation or exception guidance.
- [ ] A starter policy pack can be installed locally and exercised from CLI and
      eval-regression fixtures.
- [ ] Save-time/pre-write policy results can route to `warn`, `fence`, or
      `interrupt` when explicitly configured.
- [ ] Default user posture stays warnings-first unless a policy or CI surface
      opts into stronger enforcement.
- [ ] Exceptions are checked for scope, expiry, attribution, and revocation before
      a policy result is suppressed.

## Work Items

### OPAE-001: Policy authoring reset ADR/spec

- **Status:** Done — satisfied by ADR-098 (PR #3121, the POLRESET-001 design
  gate): Rego-first path (AD-1/AD-2), pack admission fail-fast-before-eval
  (AD-4), save-time/pre-write boundary (AD-4), exception requirement (AD-6),
  and deferred surfaces (AD-7) are all pinned there. No separate OPAE ADR is
  needed.
- **Intent:** Pin the first-slice policy product contract and explicitly defer the
  old OPAE wishlist.
- **Expected Outcome:** ADR/spec records the Rego-first path, pack admission,
  save-time/pre-write boundary, exception requirement, and deferred surfaces.
- **Validation:** `pnpm adr:check` and `pnpm aps:active-lint`
- **Dependencies:** POLRESET-001
- **Confidence:** high

### OPAE-002: User policy pack discovery contract

- **Status:** Done — `crates/anvil-policy-engine/src/pack/discovery.rs` adds the
  canonical discovery contract. `discover_packs(workspace_root)` scans exactly
  one level of `<root>/.anvil/policies/`: an immediate subdirectory carrying a
  `pack.yaml` is a `PackRef { id, dir, manifest_path, has_provenance }` (the
  OPAE-004 install layout); loose `*.rego` files directly under the policies dir
  are reported as `loose_policies` (paths only, no evaluation) so callers can
  tell the pack-managed layout from the pre-pack flat layout the gate's
  `discover_policy_files` still evaluates. Deterministic (packs sorted by
  id/dir, loose and rejected by path) and workspace-scoped (canonicalise +
  `starts_with` containment on the policies dir AND every pack/loose entry,
  mirroring `resolve_member_path`). Fail-closed **per entry**: a symlink escaping
  the root lands in `rejected` while discovery continues, so one tampered entry
  cannot hide the rest — deliberately unlike the gate's all-or-nothing bundle
  posture (justified in the module doc comment). A missing `.anvil/policies/` is
  `Ok(empty)`, not an error. Discovery ≠ admission: a pack whose `pack.yaml`
  fails to load is still listed by `discover_packs`; the convenience
  `discover_and_load` carries each pack's `Result<PackManifest>` with no
  short-circuit. No new deps. Re-exported from `pack/mod.rs`. Validated by
  `cargo test -p eddacraft-anvil-policy-engine -- policy_pack_discovery`
  (10 tests) plus the full crate suite (178) and `cargo check --workspace`.
- **Intent:** Define where local user/bundled policy packs live and how Anvil
  discovers them.
- **Expected Outcome:** Policy pack discovery is deterministic, workspace-scoped,
  and compatible with POLVAL manifests.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_pack_discovery`
  (drift correction: the item previously cited the doomed `-p eddacraft-anvil-policy`
  OPA crate, removed under ADR-098 AD-1's replace-then-delete sequence; the
  discovery contract lives in the regorus engine crate)
- **Dependencies:** OPAE-001, POLVAL-001, POLVAL-002
- **Confidence:** high

### OPAE-003: Regorus-backed user policy load path

- **Status:** Done — PR-B repoint landed. `anvil gate`'s `run_check_policy` now
  discovers `.anvil/policies/*.rego` (excluding `*_test.rego`), evaluates through
  the `anvil-policy-engine` regorus facade, and reads violations/warnings from
  `data.anvil.policies`; `anvil policy list`/`explain` read the canonical
  anvil-checks AP registry instead of the hardcoded `builtin_policies()` mirror.
  Parity evidence: `cargo test -p eddacraft-anvil-policy` (153 passed —
  eval-regression harness and exceptions untouched) and
  `cargo test -p eddacraft-anvil -- policy` (74 passed), including new gate-check
  fixtures (violation fails, warning does not fail, no bundle skips, uncompilable
  `.rego` reported not skipped, `*_test.rego` excluded). Behaviour deltas vs the
  OPA path: the opa-binary-missing silent skip is removed; policy compile errors
  are now reported check failures (fail-fast); the gate policy input adopts the
  canonical `PolicyInput` v1 shape. PR-C (mechanical deletion of the OPA modules)
  follows this.
- **Intent:** Load validated user policies through the ADR-040 policy-engine
  facade.
- **Expected Outcome:** User-authored Rego reaches regorus through the same facade
  as bundled packs; Go OPA remains reference/parity only.
- **Note (ADR-098 AD-1):** OPAE-003 is PR-B of the OPA replace-then-delete
  sequence. Its scope includes repointing `anvil gate`'s `run_check_policy`
  (`gate.rs:1736`, today the OPA-subprocess path with no regorus backing) and
  `anvil policy list/explain` / `builtin_policies()` onto the facade, gated on
  eval-regression parity evidence. That repoint slice may start ahead of
  OPAE-002/POLVAL-004 per the gate's sequencing note; PR-C (mechanical deletion
  of the OPA modules) lands only after it.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- user_policy_eval`
- **Dependencies:** OPAE-002, POLVAL-004
- **Confidence:** high

### OPAE-004: Local policy library and install UX

- **Status:** Done — `anvil policy install <PACK-ID>` (plus `install --list` and
  `show`) installs the compile-time-embedded `anvil-baseline` starter pack
  (`change_scope` + `sensitive_paths`, shaped over `input.diff.changed_files`)
  into `<workspace>/.anvil/policies/<pack-id>/`. Install canonicalises the
  destination and refuses to write when `.anvil` resolves outside the workspace
  root (path-containment breach, fail-fast, nothing written), refuses to
  overwrite existing files without `--force` (fail-closed, naming them; this is
  also the recovery path for a crash-interrupted install, since the rollback
  journal is in-memory not crash-safe), runs the POLVAL admission stack
  (load_manifest → validate_pack → run_pack_tests → enforce_tests) over the
  installed copy, and rolls back completely on a failing or invalid pack so the
  live gate directory never holds a partial pack. A `provenance.yaml` records
  pack id, version, `installed_from: bundled:<version>`, and a sha256 per file
  (no timestamps — VCS records when).
- **Posture decision (advisory-first, slice 1):** the starter pack is advisory by
  design — both policies emit only `warning`-tier findings (no `violation`/`deny`
  rule) so the pack surfaces in the gate without ever failing it, and neither
  policy reads a `config` escape hatch (there is no per-workspace override on the
  current `PolicyInput` v1 contract; thresholds are fixed in-rego defaults).
  Blocking behaviour is deferred to Anvil's posture-driven enforcement routing (a
  later OPAE contract, per ADR-098 AD-5), not carried by Rego severity.
  `sensitive_paths` remediation points at review and the future
  `anvil exception grant <rule-id>` path, not at config keys. Verified: the gate
  discovers the installed `.rego` recursively (warning-class surfacing, no gate
  failure), excludes nested `*_test.rego`, and ignores `provenance.yaml`.
  Validation: `cargo test -p eddacraft-anvil -- policy_install` (13 passed) and
  `cargo test -p eddacraft-anvil -- policy` (98 passed).
- **Intent:** Provide a local install/list/show path for starter packs without a
  remote marketplace.
- **Expected Outcome:** `anvil policy install` can install bundled starter packs
  into the local policy set with clear provenance.
- **Validation:** `cargo test -p eddacraft-anvil -- policy_install`
- **Dependencies:** OPAE-003, POLVAL-005
- **Confidence:** medium

### OPAE-005: Remediation-first policy guidance contract

- **Status:** Done — `crates/anvil-policy-engine/src/guidance.rs` adds the unified
  `PolicyGuidance` output shape (rule id, closed `PolicySource` enum, rationale,
  changed-code `CodeContext`/`Span`, remediation, and static-but-parameterised
  exception guidance naming `anvil exception grant`). One vocabulary over all
  three producers via `From<&AssertionGuidance>`, `from_risk_guidance`, and
  `from_pack_finding`; deterministic context ordering, serde round-trip, UK
  spelling, no blocking flag (posture stays with OPAE-007), no exceptions-store
  wiring. Guidance also carries the producer's `message` (the "what failed"
  text) verbatim. Validated by `cargo test -p eddacraft-anvil-policy-engine --
  policy_guidance_contract` (12 tests) and the full crate suite.
- **Intent:** Standardise policy failure output so policy breaches are actionable.
- **Expected Outcome:** Results include rule id, policy source, rationale,
  changed-code context, remediation, and exception guidance.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_guidance_contract`
  (drift correction: the item previously cited the doomed `-p eddacraft-anvil-policy`
  OPA crate; the guidance contract lives in the regorus engine crate)
- **Dependencies:** OPAE-003, CPOL-003
- **Confidence:** high

### OPAE-006: Save-time/pre-write policy input adapter

- **Status:** Done — `crates/anvil-policy-engine/src/prewrite.rs` adds
  `PrewriteInput::from_parts` (deterministic, no clock/fs/env per ADR-040 D-2)
  assembling changed paths + kinds (reusing `context::ChangedPath`), workflow
  phase, config pairs, and a minimal additive-friendly `GraphFacts` (per-path
  boundaries + dependent counts). It projects into BOTH `PolicyInput`
  (`to_policy_input`) and `AssertionContext` (`to_assertion_context`) from one
  normalised source — the projections are proven to agree on changed paths. A
  `PrewriteBudget` carries the fail-open eval budget through to
  `EngineConfig::eval_timeout` (AD-5: timeout degrades to warn in the
  enforcement layer, not here). The projection is honest about the ADR-098 AD-4
  scope limit — `to_policy_input` leaves `repo_state`/edge fields partial/empty
  (no graph walk on the pre-write path) and `supports_edge_packs()` returns
  false, steering edge-based packs to `anvil gate`. Validated by
  `cargo test -p eddacraft-anvil-policy-engine -- policy_prewrite_input`
  (11 tests) and the full crate suite (167 passing).
- **Intent:** Build the deterministic policy input needed for changed-code policy
  evaluation at save-time/pre-write boundaries.
- **Expected Outcome:** Policy evaluation can consume changed paths, graph facts,
  config, and workflow context without whole-repo rescans on the hot path.
- **Note (ADR-098 AD-4/AD-5):** slice 1 runs policy on existing off-daemon
  surfaces only (MCP `anvil_validate_write`, `anvil gate`, CI); no `policy`
  check-family joins the daemon's `validate_paths` hot path, and
  `daemon_dep_boundary.rs` forbids `regorus`/`anvil-policy*` on the daemon.
  Pre-write evaluation carries a tight fail-open budget (timeout degrades to
  warn + log, never blocks the write).
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_prewrite_input`
  (drift correction: the item previously cited the doomed `-p eddacraft-anvil-policy`
  OPA crate; the adapter lives in the regorus engine crate)
- **Dependencies:** OPAE-003, CPOL-002, POLRESET-004
- **Confidence:** medium

### OPAE-007: Enforcement-routing contract

- **Status:** Merged 2026-07-04 via PR #3165 — `crates/anvil-intercept-rules/src/policy_routing.rs` adds the
  neutral, engine-free routing contract: `PolicyOutcome` (rule id + severity
  class `violation`|`warning`, mirroring the two Rego rule families) and
  `route_policy_outcome(outcome, posture: EnforcementMode) -> ControlDecision`.
  Warning-class routes to `warn` under every posture except `off` (→`allow`) and
  never vetoes; violation-class routes through `EnforcementMode::escalated_decision`
  (`off`→allow, `warn`→warn per ADR-002 warnings-first, `fence`→fence,
  `interrupt`→interrupt). The module depends **only** on `anvil-kernel-types` —
  the daemon gains no policy evaluation (ADR-098 AD-4), so a future daemon-side
  consumer can speak this vocabulary without linking an engine. The binary
  `RuleDecision` (`Allow`|`Interrupt`) is untouched; routing lives at the adapter
  layer (ADR-098 AD-5). Validated by
  `cargo test -p eddacraft-anvil-intercept-rules -- policy_routing` (6 tests: full
  posture×class matrix, `off`→`allow` never veto, warnings never veto under any
  posture, violation matches `escalated_decision`, no bare `block`) plus the full
  crate suite (97) and `cargo test -p eddacraft-anvil-intercept --test daemon_dep_boundary`
  (7, proving no engine crate leaked toward the daemon). Consumed by POLRESET-006
  on the MCP pre-write path, whose per-call discovery + compile is currently
  uncached (bounded by a pass deadline; warm-cache follow-up filed as OPAE-011).
- **Intent:** Map policy outcomes to Anvil's existing enforcement vocabulary.
- **Expected Outcome:** Explicit policy modes can route to `warn`, `fence`, or
  `interrupt`; default behaviour remains warnings-first.
- **Note (ADR-098 AD-3/AD-5/AD-6):** routing targets the unified kernel-types
  `ControlDecision` vocabulary (gains `Fence` + `#[serde(other)] Unknown`; one
  shared posture type replaces the daemon/MCP enum pair; veto projection is
  action-time, not parse-time). The rule trait stays binary Allow|Interrupt.
  Blocking modes require the `ANVIL_POLICY_ENFORCEMENT` out-of-band kill
  switch and are hard-gated on EXCEPT-006 verdict-aware exception wiring.
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules -- policy_routing`
- **Dependencies:** OPAE-005, OPAE-006, EXCEPT-006, POLRESET-006
- **Confidence:** medium

### OPAE-008: Starter pack end-to-end proof

- **Status:** Merged 2026-07-04 via PR #3167
- **Intent:** Prove one high-signal policy pack across install, validation,
  evaluation, guidance, and report-only regression.
- **Expected Outcome:** A starter pack demonstrates real policy value before broad
  compliance-pack expansion.
- **Validation:** `cargo test -p eddacraft-anvil -- starter_policy_pack` (the
  proof lands in the CLI crate, not the deleted `eddacraft-anvil-policy` crate
  the original line cited) and `cargo test -p eddacraft-anvil -- eval_regression_command`
  (already green on main).
- **Proof status:** Delivered jointly with POLRESET-007 as one proof module
  (`crates/anvil-cli/src/commands/policy/starter_proof.rs`). All five stages
  proven green — install + verified provenance, admission (regorus pack tests),
  advisory-first gate evaluation surfacing remediation-first guidance,
  pre-write projection with the warnings-never-veto-under-interrupt invariant,
  and frozen `anvil policy eval --json` v1 eval-harness exercisability. The
  proof surfaced and closed a policy extractor gap on **both** surfaces (the
  gate and the pre-write `mcp::policy_prewrite` path from PR #3165 both failed
  to recognise the documented `warning` rule set), fixed via a crate
  single-source rule-family vocabulary (`crate::policy_vocab`) — see
  POLRESET-007.
- **Dependencies:** OPAE-004, OPAE-007, POLRESET-007
- **Confidence:** medium

### OPAE-010: Pack configuration surface

- **Status:** Proposed
- **Intent:** Give policy packs a real, wired configuration surface — discovered
  during OPAE-004 review: `PolicyInput` (a frozen stability contract) carries no
  `config` field and no gate/pre-write caller populates one, so any pack rule
  reading `input.config` is permanently dead in production.
- **Expected Outcome:** A decided mechanism (additive `PolicyInput` field with a
  populated source, or a separate config document) lets packs read
  workspace-scoped configuration; the starter pack's thresholds become genuinely
  configurable; Rego pack tests can only exercise input shapes production
  supplies.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_input_config`
- **Dependencies:** OPAE-004, OPAE-007
- **Confidence:** medium

### OPAE-011: Compiled-policy cache for the pre-write path

- **Status:** Proposed
- **Intent:** Stop the MCP pre-write policy pass from repeating discovery,
  manifest parse, and `regorus` compile on **every** `anvil_validate_write` call.
  Filed from the POLRESET-006 review measurement: the pass is currently uncached
  and roughly linear in pack count (~450 µs/pack/call for trivial packs,
  unbounded for large real packs), capped today only by the pre-write pass
  deadline (which *truncates* rather than *speeds up* a large pack set).
- **Expected Outcome:** An mtime-keyed cache of loaded manifests and compiled
  engines keyed on `(pack dir, manifest mtime, member mtimes)` so a **warm**
  pre-write policy pass costs eval-only; the cold pass cost is unchanged; the
  deadline (POLRESET-006) still bounds a cold or invalidated pass. Cache
  invalidation is content-mtime driven so an edited pack recompiles.
- **Validation:** `cargo test -p eddacraft-anvil -- policy_prewrite_cache`
- **Dependencies:** OPAE-007
- **Confidence:** medium

### OPAE-009: Policy authoring user docs

- **Status:** Proposed
- **Intent:** Explain the supported first-slice policy authoring path without
  promising deferred enterprise or AI-generation features.
- **Expected Outcome:** Public docs show how to author, validate, install, run,
  and exception a policy pack.
- **Validation:** `pnpm docs:check`
- **Dependencies:** OPAE-008
- **Confidence:** high
