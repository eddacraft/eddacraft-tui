<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Project Scaffolding

| ID | Owner | Priority | Status | Progress |
| --- | --- | --- | --- | --- |
| PSCAF | @joshuaboys | P1 | Ready | 0/8 |

**Status:** Ready (2026-09-09). The operator approved the product contract and
[ADR-143](../decisions/143-project-scaffold-reconciliation.md). PSCAF-001 is the
entry point; later items execute in dependency order.

**Origin:** a beta report that L4 never fires exposed the missing acceptance
policy, followed by an operator review of what `anvil init` and `anvil start`
should mean. The approved design expands the repair into one coherent bootstrap
boundary instead of adding another isolated writer.

**Last reviewed:** 2026-09-09 — module created and promoted by operator approval.

## Purpose

Make `anvil init` a safe, repeatable project-scaffold reconciler and make
`anvil start` compose it before opinionated activation. Restore discoverable
configuration such as architecture and acceptance policy without silently
rewriting operator-owned files or claiming inactive protections are on.

## In Scope

- One typed scaffold catalogue and additive reconciliation engine.
- Mandatory foundation plus selectable protection components.
- Interactive and non-interactive init contracts, repeat-run review, profiles,
  component overrides, structured outcomes, and scoped force.
- Bounded architecture inference with tracked delegated output.
- Composition into the existing start activation plan and dependency handling.
- Generated public configuration-options documentation from the catalogue.
- L4 prerequisite installation, shared policy diagnosis, and honest status.

## Out of Scope

- Changing L4 policy evaluation, `l4_or_l3`, Serena's internal-error posture,
  or existing acceptance-policy content.
- Making direct init install hooks, MCP, workflows, daemon state, witnesses,
  baselines, or run the first scan.
- Replacing the general settings truth/mutation service.
- Generating a low-confidence architecture or treating inference as authority.
- Re-opening ADR-102 verdicts other than its superseded architecture-init
  decision.

## Interfaces

**Depends on:**

- [ADR-143](../decisions/143-project-scaffold-reconciliation.md) and the
  [approved specification](../specs/2026-09-09-project-scaffold-reconciliation.md).
- ADR-120 main-config discovery and `SectionOrSource<T>` delegation.
- ADR-037 default acceptance-policy posture.
- ADR-103 activation-plan consent and ADR-114 command roles.
- Existing init, start, status, doctor, architecture, hook, and L4 validation
  command surfaces in `crates/anvil-cli`.

**Exposes:**

- A shared scaffold catalogue and reconciliation plan consumed by init/start.
- Stable component identifiers, structured reconciliation outcomes, and
  separately evidenced component health.
- A generated public **Project configuration options** page.
- Evidence-based L4 activation state.

## Ready Checklist

- [x] Operator-approved product contract recorded in a final specification.
- [x] Durable architectural choices accepted in ADR-143.
- [x] Exclusive module and owner named.
- [x] Eight independently verifiable vertical slices defined.
- [x] Supersession boundaries recorded for ARCHCFG-007 and CIB-415.
- [x] Entry point and dependency order identified.

## Work Items

| ID | Task | Status | Depends on |
| --- | --- | --- | --- |
| PSCAF-001 | Catalogue and additive reconciliation kernel | Ready | — |
| PSCAF-002 | `anvil init` project-scaffold interface | Ready | PSCAF-001 |
| PSCAF-003 | Checks and enforcement component | Ready | PSCAF-001, PSCAF-002 |
| PSCAF-004 | Planning-support component | Ready | PSCAF-001, PSCAF-002 |
| PSCAF-005 | Detected architecture component | Ready | PSCAF-001, PSCAF-002 |
| PSCAF-006 | Compose scaffold planning into `anvil start` | Ready | PSCAF-001..005 |
| PSCAF-007 | Generated project-configuration catalogue | Ready | PSCAF-001..006 |
| PSCAF-008 | L4 activation honesty and integrated journey | Ready | PSCAF-001, PSCAF-002, PSCAF-006, CIB-267 |

### PSCAF-001: Catalogue and additive reconciliation kernel

- **Status:** Ready
- **Intent:** Establish one internal source of component truth and a race-safe,
  additive engine usable by both public commands.
- **Expected Outcome:** A typed catalogue defines foundation,
  acceptance-policy, architecture, checks, and planning components, including
  ownership, recommendation, dependencies, inactive consequence, and later
  command. The engine plans semantic changes, preserves every existing key/path
  by default, resolves profiles and dependencies, and returns the five approved
  mutation outcomes plus independent component health. Foundation is
  unskippable. Acceptance policy is never force-replaced. Existing-file updates
  use identity-and-digest compare-and-swap under the mandatory shared mutation
  lock, or emit a patch with `needs_input` when unavailable. Main-config target
  descriptors and writes derive from ADR-132's
  settings catalogue/service; a parity gate prevents duplicate writer truth.
  Foundation and acceptance-policy writers are the first functional vertical
  slice.
- **Scope:** New internal scaffold module(s), canonical config integration,
  Git-common-dir cross-platform config lock and migration of every anvil
  main-config writer, exclusive writer helper, catalogue and reconciliation
  tests.
- **Non-scope:** Public picker rendering, architecture detection, runtime
  activation, documentation prose.
- **Files:** `crates/anvil-cli/src/commands/init.rs` may delegate during
  extraction; place reusable logic outside command presentation. Exact module
  paths are fixed by the first red test and impact analysis.
- **Dependencies:** —
- **Validation:** Unit tests prove profile/dependency resolution, existing-byte
  preservation including scoped force on policy, settings, planning, and
  identity (including strict enforcement and project UUID), missing-section planning,
  rejection of foundation skip before writes, exclusive creation under a parent
  swap adversary, concurrent-writer re-plan for config/ignore updates,
  invalid/conflicting prerequisite health, and foundation/policy functional
  output; settings/scaffold catalogue target parity; multi-file failure
  injection at every boundary with rollback/resume and no dangling pointer;
  relevant CLI crate tests and clippy.
- **Confidence:** high

### PSCAF-002: `anvil init` project-scaffold interface

- **Status:** Ready
- **Intent:** Give direct init the approved interactive, repeatable, and
  automation-safe product contract.
- **Expected Outcome:** Interactive init asks which protections to configure;
  repeat runs show configured plus missing/skipped choices with **review all**.
  Non-interactive profiles, repeatable include/skip, structured results, exact
  rerun commands, and component-scoped force behave as specified. Direct init
  validates configuration but creates no runtime state and performs no scan.
- **Scope:** Init CLI parsing/presentation, plan preview/consent, machine output,
  removal of first-scan ownership from direct init.
- **Non-scope:** Start overlay, architecture detector, runtime installation.
- **Files:** `crates/anvil-cli/src/commands/init.rs`, CLI definitions and init
  process/integration tests.
- **Dependencies:** PSCAF-001
- **Validation:** Tests cover fresh/repeat/non-interactive flows, profile
  membership, skip precedence, foundation-skip rejection before writes,
  bare-force refusal, named-force confinement, separate mutation/health output,
  no runtime side effects, no first scan, and global exit-registry mapping;
  `cargo test -p eddacraft-anvil --bin anvil -- init`.
- **Confidence:** high

### PSCAF-003: Checks and enforcement component

- **Status:** Ready
- **Intent:** Make the advertised checks choice produce a functional declared
  check and enforcement posture.
- **Expected Outcome:** The component routes `protection.checks` and
  `protection.enforcement.mode` through the settings-service bootstrap writer to
  main-config `/checks` and `/enforcement/mode`. Existing values are preserved;
  invalid or conflicting values report health and block dependent activation.
  Fresh recommended values are exactly `secret-detection`,
  `import-boundaries`, and `antipattern-scan` at `warn`; interactive postures
  are `off`, `warn`, and `enforce` (`enforce` serialises as shipped `block`).
- **Scope:** Settings target descriptors, component planner/writer, semantic
  validation, interactive choices, and focused tests.
- **Non-scope:** Activating hooks/daemon, changing check implementations, or
  configuring `protection.fail_closed`.
- **Dependencies:** PSCAF-001, PSCAF-002
- **Validation:** Golden tests pin the recommended check set and each named
  posture; each supported posture round-trips through project config and
  the settings resolver; unknown checks and invalid modes are preserved but
  unhealthy; no unselected config pointer changes.
- **Confidence:** high

### PSCAF-004: Planning-support component

- **Status:** Ready
- **Intent:** Give selected planning support a functional tracked starting
  point rather than an empty placeholder directory.
- **Expected Outcome:** The component writes `/planning_dir` through the new
  settings key `project.planning.dir` and creates a minimal valid
  `plans/index.aps.md` only when the selected directory is absent. Existing
  directories and indexes are preserved and validated/reported. The fresh
  catalogue default is exactly `plans`.
- **Scope:** Settings catalogue target, minimal APS starter asset, component
  writer, and validation.
- **Non-scope:** Generating work items, replacing existing plans, or running APS
  workflows.
- **Dependencies:** PSCAF-001, PSCAF-002
- **Validation:** Fresh starter passes APS structural validation; existing
  planning content is byte-preserved; configured custom directory is honoured;
  multi-file failure recovery follows PSCAF-001.
- **Confidence:** high

### PSCAF-005: Detected architecture component

- **Status:** Ready
- **Intent:** Restore architecture onboarding inside the single project-scaffold
  flow instead of adding a competing architecture-specific init command.
- **Expected Outcome:** Bounded repository detection proposes suitable
  architecture; interactive ambiguity offers alternatives/skip; low-confidence
  non-interactive use returns `needs_input` with an exact rerun. Accepted output
  is tracked `anvil/architecture.yaml`, referenced by `architecture.source`, and
  passes architecture validation.
- **Scope:** Detector, inference presentation model, component writer, delegated
  config integration, tests across representative repository shapes.
- **Non-scope:** Proving inferred architecture correct, changing architecture
  validation/evaluation, deleting legacy fallback.
- **Dependencies:** PSCAF-001, PSCAF-002
- **Validation:** Fixtures cover each layered/hexagonal/workspace evidence row,
  ambiguous/low confidence, every entry/depth/manifest/byte bound, ignored paths,
  symlinks, ties, workspace-plus-layered/hexagonal collisions, and heterogeneous
  workspaces; no low-confidence write;
  unrelated components still reconcile; generated output validates; existing
  architecture bytes are preserved; failure injection cannot leave a dangling
  `architecture.source`; member paths reject traversal, absolute/UNC/device,
  symlink, reparse, and out-of-repository shapes before reads.
- **Confidence:** medium — confidence heuristics need fixture-driven bounds

### PSCAF-006: Compose scaffold planning into `anvil start`

- **Status:** Ready
- **Intent:** Keep start opinionated while eliminating bootstrap drift.
- **Expected Outcome:** The first section of the existing activation plan uses
  the shared recommended scaffold profile; no nested wizard or duplicate writer
  exists. Skipped prerequisites remove dependent runtime protections and explain
  why. First scan and baseline ownership live only in start/derived activation
  flows.
- **Scope:** Start plan composition, TUI/plain parity, dependency presentation,
  first-scan/baseline movement, activation integration tests.
- **Non-scope:** Changing existing consent defaults or bare-`anvil` ensure
  behaviour.
- **Dependencies:** PSCAF-001, PSCAF-002, PSCAF-003, PSCAF-004, PSCAF-005
- **Validation:** Journey tests prove start and init produce byte-identical
  scaffold plans for the same selections, start performs activation after
  consent, init does not, and an L4 hook cannot be selected without acceptance
  policy. Malformed policy, invalid project id, ambiguous policy variants, and
  dangling architecture sources cannot satisfy runtime dependencies. Existing
  `start --verify`, `start --json`, and piped compact stdout fixtures remain
  byte-identical; read-only and gated modes perform no scaffold write.
- **Confidence:** high

### PSCAF-007: Generated project-configuration catalogue

- **Status:** Ready
- **Intent:** Make every available scaffold option discoverable without
  maintaining a second handwritten inventory.
- **Expected Outcome:** The public **Project configuration options** page is
  generated from the typed catalogue, documents every required field, has a
  stable public URL, and is linked with an exact rerun command after init.
  Documentation validation detects stale generated output.
- **Scope:** Catalogue export/generator, public docs page, guide links,
  docs-staleness check.
- **Non-scope:** General configuration reference or settings UI.
- **Dependencies:** PSCAF-001, PSCAF-002, PSCAF-003, PSCAF-004, PSCAF-005,
  PSCAF-006
- **Validation:** Generator golden/parity test plus `pnpm docs:check` and
  `pnpm format:check`.
- **Confidence:** high

### PSCAF-008: L4 activation honesty and integrated journey

- **Status:** Ready
- **Intent:** Close the beta-reported “L4 never fires” path without altering the
  evaluation engine.
- **Expected Outcome:** Fresh recommended scaffolds include the ADR-037 policy;
  existing policies are byte-preserved. Status reports L4 `On` only with valid
  project id, hook, and parseable policy. Doctor parses the same source and
  provides direct remediation. Public guidance links to CIB-267's hook-time PATH
  troubleshooting and covers the remaining silent passes, `l4_or_l3`, and a
  deterministic `l4_only` hex-SHA exercise. Policy writes use PSCAF-001's
  race-safe primitive.
- **Scope:** Shared policy discovery/parse result, status, doctor, L4 docs and
  integrated user-facing tests.
- **Non-scope:** Git pre-push positional parsing (CIB-267), policy semantics,
  existing-policy modification, Serena internal-error changes.
- **Dependencies:** PSCAF-001, PSCAF-002, PSCAF-006, CIB-267
- **Validation:** Focused init/status/doctor/L4 tests, production-engine
  `l4_only` journey, hook-without-policy and missing-id non-On cases,
  `cargo test -p eddacraft-anvil --no-fail-fast`, docs and format checks.
- **Confidence:** high — the engine path is already proven; prerequisites and
  claims are the defect

## Completion Criteria

- All eight work items are Merged and released through the normal lifecycle.
- The eleven acceptance journeys in the specification pass on supported
  platforms, including Windows Git PATH coverage for CIB-267 coordination.
- No public documentation claims a scaffold option that is absent from the
  catalogue or an active L4 layer without prerequisite evidence.
- ADR-102 and the old CIB advisory remain linked as superseded provenance, not
  parallel implementation authority.
