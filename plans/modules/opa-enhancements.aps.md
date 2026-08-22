# Policy Authoring and Runtime UX

| ID   | Owner | Priority | Status | Progress |
| ---- | ----- | -------- | ------ | -------- |
| OPAE | —     | high     | In Progress | 8/22      |

**Last reviewed:** 2026-08-22 — the policy-capability audit filed two defects
against this module (OPAE-021 public authoring door, OPAE-022 gate/pre-write
admission divergence) and confirmed OPAE-009..020 all remain Proposed with
nothing Ready, so no authoring-ease work has advanced since July. Sequencing is
now coordinated by [POLFIT](./policy-fit-for-purpose.aps.md). Prior review
2026-07-17 (ADR-108 remains accepted; POLRESET topology
flow-down confirmed OPAE-012..020 target only `anvil-policy-engine`, the Rust
CLI, and bounded agent surfaces; readiness still begins at OPAE-012).

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

- User policy pack discovery and loading through `crates/anvil-policy-engine`
  (`src/pack/discovery.rs`, OPAE-002; this line originally said
  `crates/anvil-policy`, corrected 2026-07-11 — PR-C deleted that crate's
  loader).
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
- Deterministic, target-aware policy lint and validation composition.
- A generated agent-guidance pilot for policy authoring, routed on demand with
  no ambient context load.
- A standard MCP resource-template route and securely leased compatibility
  files, each behind its own proof gate.
- A customer-facing `authoring-anvil-policy` skill distributed through the
  existing managed skill installer.

## Out of Scope

- An Anvil-hosted natural-language policy generator or runtime AI inference.
  External customer agents using the shipped authoring skill are an in-scope
  authoring client; their output still crosses deterministic Anvil admission.
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

- [POLRESET](../archive/modules/policy-value-enforcement-reset.aps.md) — reset sequence and
  enforcement design gate.
- [POLENG](../archive/modules/policy-engine.aps.md) / ADR-040 — regorus facade,
  `PolicyInput` v1, result post-processing, and `anvil policy eval` substrate.
- [POLVAL](../archive/modules/policy-pack-validation.aps.md) — pack metadata, manifests,
  validation, and test enforcement.
- [CPOL](../archive/modules/contextual-policy-assertions.aps.md) — deterministic context and
  guidance payloads.
- [IORISK](../archive/modules/io-risk-controls.aps.md) — shared risk vocabulary when starter packs
  need IO/prompt-risk dimensions.
- [EXCEPT](./git-native-exceptions.aps.md) — valid exception verification before
  fencing or interrupting.
- `crates/anvil-policy-engine`, `crates/anvil-cli`,
  `crates/anvil-intercept-rules`, and `crates/anvil-intercept-protocol`.
  Exception behaviour is consumed through EXCEPT and its future
  `anvil-exceptions` home; new OPAE work does not target the deletion-slated
  `anvil-policy` support crate.

**Exposes:**

- User policy pack discovery contract.
- Policy library/install UX contract.
- Remediation-first policy guidance contract.
- Save-time/pre-write policy input adapter contract.
- Enforcement-routing contract for `warn`, `fence`, and `interrupt`.
- Policy-pack target/input declaration and deterministic lint contract.
- Version-matched agent-guidance routing for policy authoring.

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
- [ ] New policy packs declare their enforcement targets and required input
      fields; unavailable target inputs fail lint before evaluation.
- [ ] `anvil policy lint --json` emits stable, actionable diagnostics and
      `anvil policy validate` composes lint, compilation, and executable tests.
- [ ] The `authoring-anvil-policy` skill retrieves only the requested embedded
      guidance topic and completes an end-to-end authoring journey offline.
- [ ] Normal Anvil commands and MCP tool listings carry no policy-authoring
      reference payload; guidance retrieval and temporary files are opt-in.

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
- **Note (ADR-098 AD-1 — superseded planning note; the repoint and PR-C have
  both landed, see the Status paragraph above):** OPAE-003 was PR-B of the OPA
  replace-then-delete sequence. Its scope included repointing `anvil gate`'s
  `run_check_policy` (at planning time `gate.rs:1736`, then the OPA-subprocess
  path with no regorus backing) and `anvil policy list/explain` /
  `builtin_policies()` onto the facade, gated on eval-regression parity
  evidence. That repoint slice was allowed to start ahead of
  OPAE-002/POLVAL-004 per the gate's sequencing note; PR-C (mechanical deletion
  of the OPA modules) landed after it.
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3165 — `crates/anvil-intercept-rules/src/policy_routing.rs` adds the
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

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-04 via PR #3167
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
- **Intent:** Keep the public policy tutorial accurate and intentionally small
  while the comprehensive, version-matched agent reference ships through the
  routed guidance bundle.
- **Expected Outcome:** Public docs show the human quick path and skill install
  command without mirroring or linking the private-distribution agent reference;
  commands use the exact pack directory and distinguish `policy validate` from
  the discovery-only `policy test` surface.
- **Validation:** `pnpm docs:check`
- **Dependencies:** OPAE-008, OPAE-014, OPAE-017
- **Confidence:** high

### OPAE-012: Policy target and input authoring contract

- **Status:** Proposed — ADR-108 accepted by owner 2026-07-16; advance through
  the normal readiness gate before implementation.
- **Intent:** Version policy-pack manifests for new authoring and make intended
  admission targets, accepted `PolicyInput` availability, and executable cases
  explicit without implying automatic activation.
- **Expected Outcome:** Manifest v2 declares `explicit-eval`, `gate`, and/or
  `pre-write` targets, a v1 input contract, and typed positive/negative cases;
  one Rust registry classifies each field as available, partial,
  caller-supplied, or unavailable and is parity-tested beside the real
  gate/pre-write producers. Legacy packs remain readable in new binaries with a
  migration warning; old binaries are explicitly not v2-compatible.
- **Files:** `crates/anvil-policy-engine/src/pack/manifest.rs`,
  `crates/anvil-policy-engine/src/authoring.rs`, pack fixtures and starter pack
  manifests.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_authoring_contract`
- **Dependencies:** OPAE-001, ADR-108 (Accepted 2026-07-16)
- **Confidence:** high

### OPAE-013: Deterministic Anvil Rego linter

- **Status:** Proposed
- **Intent:** Check that a policy belongs to the supported Anvil/regorus family
  before deployment while separating static facts from executable evidence.
- **Expected Outcome:** A Rust lint engine emits stable `POL001`..`POL014`
  diagnostics for target/input availability, parser/compiler-proven namespace,
  identity and capabilities, declared cases, executable result shapes,
  metadata, and bounded semantic hazards. Unprovable rules are deferred or
  advisory rather than approximated lexically.
- **Files:** `crates/anvil-policy-engine/src/lint/`, engine lint fixtures and
  diagnostic snapshots.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_lint`
- **Dependencies:** OPAE-012
- **Confidence:** medium

### OPAE-014: Policy lint CLI and validation composition

- **Status:** Proposed
- **Intent:** Make deterministic authoring checks directly usable by customers
  and agents without creating a second admission path.
- **Expected Outcome:** `anvil policy lint <pack> [--target] [--json]` emits
  remediation-first reports with stable ordering; `anvil policy validate`
  composes structural admission, static lint, one regorus compile, and
  single-execution conformance cases without duplicate diagnostics.
- **Files:** `crates/anvil-cli/src/commands/policy/lint.rs`,
  `crates/anvil-cli/src/commands/policy/validate.rs`, policy CLI integration
  tests.
- **Validation:** `cargo test -p eddacraft-anvil --test policy_lint` and
  `cargo test -p eddacraft-anvil -- policy_validate`
- **Dependencies:** OPAE-013
- **Confidence:** high

### OPAE-015: Generated policy-authoring guidance bundle

- **Status:** Proposed
- **Intent:** Compile exact product contracts and governed narrative into
  version-matched agent topics rather than maintaining a parallel manual
  reference.
- **Expected Outcome:** A deterministic generator combines Rust registries,
  governed extracts, and `docs/agent-guidance/policy-authoring/` narratives into
  bounded Markdown/JSON assets with stable IDs, provenance hashes, lint-code
  coverage, and a compact route index. `guidance:check` fails on drift, broken
  routes, forbidden links, or context-budget breaches.
- **Files:** `docs/agent-guidance/policy-authoring/`,
  `scripts/guidance/generate.mjs`, generated CLI guidance assets, generator
  tests and package scripts.
- **Validation:** `pnpm guidance:check` and `pnpm docs:check`
- **Dependencies:** OPAE-012, OPAE-013, ADR-108 (Accepted 2026-07-16)
- **Confidence:** medium

### OPAE-016: On-demand guidance CLI

- **Status:** Proposed
- **Intent:** Route agents to one relevant topic without charging every Anvil
  session for comprehensive reference material.
- **Expected Outcome:** One resolver powers `anvil guidance list/show/explain`,
  version-matched content through `anvil guidance list/show/explain`. Normal
  commands do not load guidance or initialise repository/daemon state.
- **Files:** `crates/anvil-cli/src/guidance/`,
  `crates/anvil-cli/src/commands/guidance.rs`, CLI guidance tests.
- **Validation:** `cargo test -p eddacraft-anvil --test guidance_cli`
- **Dependencies:** OPAE-015
- **Confidence:** medium

### OPAE-017: Ship `authoring-anvil-policy`

- **Status:** Proposed
- **Intent:** Give customer agents a small, version-matched workflow that uses
  deterministic Anvil authoring tools and progressive reference retrieval.
- **Expected Outcome:** The canonical catalogue skill stays distinct from
  `using-anvil`, routes by target/topic/lint code, never invents unavailable
  commands, inputs, or scaffold surfaces. OPAE owns and reviews the canonical
  content and its topic routes; SKPKG-009 separately owns vendoring and managed
  distribution without duplicating ADR-106's client registry.
- **Files:** canonical `eddacraft-skills/skills/eddacraft/authoring-anvil-policy/`
  (external), skill/topic route contract tests.
- **Validation:** the catalogue skill validation command recorded by
  `eddacraft-skills` plus route resolution against the Anvil guidance bundle.
- **Dependencies:** OPAE-014, OPAE-016
- **Confidence:** medium

### OPAE-018: Industry-scenario conformance and beta rollout

- **Status:** Proposed
- **Intent:** Prove policy authoring with realistic organisational invariants,
  not only engine unit fixtures.
- **Expected Outcome:** Payments, clinical-rule, and platform/SRE scenario packs
  each carry v2 manifests, Rego, complete tests, explicit inputs, passing and
  failing repositories, and expected diagnostics. Held-out prompts exercise
  distinct companion, same-stem, and multi-evidence behaviours. Built-binary
  journeys plus a downloaded-release smoke test run skill route → lint →
  validate → eval → policy gate. A governed primary-client evidence matrix
  records interventions, false results, context size, cleanup, and latency.
- **Files:** `policies/scenarios/`, policy-authoring journey tests, release/beta
  evidence and known-gap notes.
- **Validation:** `cargo test -p eddacraft-anvil --test policy_authoring_journey`,
  `pnpm guidance:check`, and `pnpm docs:check`
- **Dependencies:** OPAE-014, OPAE-016, OPAE-017, OPAE-019, OPAE-020, SKPKG-009
- **Confidence:** medium

### OPAE-019: Standard MCP guidance routing

- **Status:** Proposed
- **Intent:** Offer on-demand agent routing over MCP without turning guidance
  into an always-injected tool-schema tax.
- **Expected Outcome:** MCP exposes one sub-500-byte `anvil://guidance` index
  descriptor and one sub-700-byte resource template. CLI/MCP documents are
  byte-equivalent, aggregate discovery stays within 1,200 bytes in Claude Code,
  Codex, and OpenCode, and routed content is never eagerly injected.
- **Files:** `crates/anvil-cli/src/mcp/resources/guidance.rs`, MCP protocol and
  real-client context tests.
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_guidance_resources`
- **Dependencies:** OPAE-016, ADR-108 (Accepted 2026-07-16)
- **Confidence:** medium

### OPAE-020: Secure leased guidance materialisation

- **Status:** Proposed
- **Intent:** Give clients that require a file an opt-in compatibility surface
  without workspace writes or ordinary-command cleanup cost.
- **Expected Outcome:** Files live under the resolved install user root and use
  owner-only no-follow atomic creation, a cross-process lock, reference-counted
  leases, explicit release, guidance-command-only expiry sweep, and crash
  recovery. Filesystem race, symlink, ownership, and malformed-state tests gate
  shipment.
- **Files:** `crates/anvil-cli/src/guidance/materialise.rs`, CLI materialisation
  integration and adversarial filesystem tests.
- **Validation:** `cargo test -p eddacraft-anvil --test guidance_cli materialise`
- **Dependencies:** OPAE-016, ADR-108 (Accepted 2026-07-16)
- **Confidence:** low

### OPAE-021: Public authoring door resolves to a shipped surface

- **Status:** Proposed
- **Intent:** Stop the public policy-model page instructing users to install an
  authoring skill that does not exist yet.
- **Expected Outcome:** The `authoring-anvil-policy` instruction in
  `docs/public/anvil/concepts/policy-model.md` either describes a path that
  ships today or is removed until OPAE-017 lands. The page still refuses to
  become a pack-writing workshop; it simply stops naming an unshipped door.
- **Files:** `docs/public/anvil/concepts/policy-model.md`
- **Validation:** `pnpm docs:check && pnpm docs:public:check`
- **Dependencies:** OPAE-017 (Proposed), POLFIT-002
- **Confidence:** high

### OPAE-022: Gate and pre-write share one pack-admission contract

- **Status:** Proposed
- **Intent:** Remove the admission divergence between the two shipped policy
  evaluators, discovered during the 2026-08-22 policy-capability audit.
- **Expected Outcome:** `anvil gate --only-checks policy` and the MCP pre-write
  pass agree on which policies are admitted. The gate currently flat-walks every
  `*.rego` under `.anvil/policies/` and ignores `pack.yaml` entirely, while
  pre-write loads only manifest-declared members via `discover_and_load` — so a
  loose `.rego` fires at the gate but is invisible pre-write, and pack metadata
  buys nothing at the gate. Either both honour the manifest, or the difference
  is documented as intended with its user-visible consequence stated.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/mcp/policy_prewrite.rs`,
  `crates/anvil-policy-engine/src/pack/discovery.rs`
- **Validation:** `cargo test -p eddacraft-anvil -- policy_prewrite_routing` and
  `cargo test -p eddacraft-anvil -- run_check_policy`
- **Dependencies:** OPAE-002, OPAE-007, POLFIT-001
- **Confidence:** medium

## Designs

- [Policy Authoring Lint and Agent Guidance Pilot](../specs/2026-07-15-policy-authoring-lint-and-agent-guidance.md)
  — accepted design under ADR-108; implementation readiness begins at OPAE-012.
- [ADR-108](../decisions/108-policy-authoring-lint-and-agent-guidance.md) —
  deterministic authoring boundary and on-demand guidance delivery decision.

## Execution

- [OPAE-012..014 policy lint actions](../execution/OPAE-012-014-policy-lint.actions.md)
- [OPAE-015/016/019/020 agent guidance pilot actions](../execution/OPAE-015-016-agent-guidance-pilot.actions.md)
- [OPAE-017..018 authoring rollout actions](../execution/OPAE-017-018-authoring-rollout.actions.md)
