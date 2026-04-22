# Language Alignment Execution Slices

## Purpose

This document is the `CLAR-005` output. It converts the inventory, canonical
language design, and onboarding gap audit into bounded execution slices that can
be planned and delivered independently.

Inputs:

- `plans/specs/2026-04-21-check-language-inventory.md`
- `plans/specs/2026-04-21-anvil-quality-language-design.md`
- `plans/specs/2026-04-21-onboarding-language-gap-audit.md`

## Slicing Principles

1. Do not attempt one global rename across the repo.
2. Fix the user model at the highest-leverage teaching surfaces first.
3. Align names before adding more commands and UI surfaces.
4. Reconcile active plans before they harden more conflicting language.
5. Keep internal implementation naming changes out of scope unless they are
   required for user-facing clarity.

## Proposed Execution Slices

### Slice 1: Check and Gate Naming Alignment

**Why first:** The guided init and config contract is currently broken. Users
are told to enable checks whose names do not align with execution surfaces.

**Scope:**

- `crates/anvil-cli/src/commands/defaults.rs`
- `crates/anvil-cli/src/commands/gate.rs`
- `crates/anvil-cli/src/commands/gate_config.rs`
- `crates/anvil-cli/src/commands/init.rs`
- config docs under `docs/public/anvil/operations/`

**Intent:**

Create one canonical user-facing naming layer for checks and gate sub-results,
with explicit aliases or migration handling where needed.

**Expected Outcome:**

- onboarding/config and gate execution expose one consistent set of user-facing
  check names
- `gate-config` language makes clear whether it configures checks, gates, or
  both
- deprecated names, if any, are documented as aliases rather than silent drift

**Boundaries:**

- no gate-runner rearchitecture in this slice
- no new checks added
- internal runner dispatch may remain separate temporarily if the user-facing
  contract is unified

**Validation:**

- one canonical check-name table exists in docs and matches onboarding/gate help
- guided init names and gate output names are either identical or explicitly
  mapped

### Slice 2: Welcome and Tutorial Foundation Rewrite

**Why second:** This is where user understanding is formed. The current flow is
close, but it still teaches commands before the model.

**Scope:**

- `crates/anvil-cli/src/commands/welcome.rs`
- `crates/anvil-tui/src/surfaces/tutorial/`
- onboarding/welcome render copy

**Intent:**

Teach the canonical model explicitly before routing users into subsystem
tutorials or quick actions.

**Expected Outcome:**

- welcome/onboarding explains scan -> checks -> findings -> gate
- quick actions are labelled by user goal and relationship, not just command
  names
- tutorial path descriptions reflect the canonical model
- a short foundation step exists before subsystem path selection

**Boundaries:**

- no new tutorial domains
- no embedded editor redesign
- no dashboard work

**Validation:**

- first-run flow contains an explicit explanation of checks/findings/gate
- tutorial path chooser text no longer assumes prior subsystem knowledge

### Slice 3: Documentation Terminology Cleanup

**Why third:** Docs are long-lived and currently reinforce drift between config,
architecture, and gate surfaces.

**Scope:**

- `docs/public/anvil/operations/config.md`
- `docs/public/anvil/tutorials/architecture.md`
- adjacent onboarding/tutorial docs under `docs/public/anvil/`

**Intent:**

Update public docs to use the canonical model and stable terminology.

**Expected Outcome:**

- docs use `finding` as the generic result noun where appropriate
- architecture docs explain boundaries as one family of checks over the project
  graph
- config docs no longer imply multiple incompatible check systems

**Boundaries:**

- avoid a full docs-site information-architecture rewrite
- marketing copy remains out of scope

**Validation:**

- onboarding/config/tutorial docs are consistent with the canonical language doc

### Slice 4: Active APS Wording Reconciliation

**Why fourth:** Future modules are already introducing conflicting command and
surface language. Preventing drift in plans is cheaper than fixing it after
implementation ships.

**Scope:**

- `plans/modules/rust-cli-tier2.aps.md`
- `plans/modules/rust-cli-tier3.aps.md`
- dashboard modules referencing gate/check/graph surfaces
- policy-governance modules that overload `gate`
- other active specs identified in the inventory

**Intent:**

Align active and proposed APS text with the canonical language rules so future
implementation work starts from consistent concepts.

**Expected Outcome:**

- `RCLI2` and `RCLI3` explicitly follow the checks/findings/gates model where
  applicable
- planned dashboard/policy surfaces avoid unnecessary `gate` overload
- future command descriptions distinguish quality workflow surfaces from
  governance/workflow utilities

**Boundaries:**

- wording and framing changes only
- no expansion of module scope
- no implementation work

**Validation:**

- targeted APS modules updated to reference the canonical language model

## Suggested APS Shapes

These slices can be executed either as follow-on tasks in `CLAR` or as child
modules if the work grows. Current recommendation:

1. Keep `CLAR` as the discovery and alignment parent module.
2. Create follow-on modules only for slices that require multi-PR implementation.
3. Start with two executable follow-ons:
   - `quality-language-runtime-alignment`
   - `quality-language-onboarding-and-docs`
4. Treat APS wording reconciliation as a maintenance task that can run in
   parallel with implementation slices.

## Recommended Follow-On Work Items

### CLAR-006: Runtime naming alignment

- **Intent:** Align onboarding/config/gate naming into one user-facing check and
  gate contract
- **Expected Outcome:** Users see one coherent naming layer across init, config,
  gate output, and gate config
- **Validation:** canonical name table matches runtime surfaces and docs

### CLAR-007: Welcome and tutorial model rewrite

- **Intent:** Rewrite first-run teaching surfaces around the canonical mental
  model
- **Expected Outcome:** welcome/tutorial explains checks, findings, and gates
  before subsystem-specific commands
- **Validation:** first-run copy and tutorial path text match the canonical model

### CLAR-008: Public docs terminology cleanup

- **Intent:** Bring config/tutorial docs into alignment with the canonical
  language
- **Expected Outcome:** docs teach one quality model and one result hierarchy
- **Validation:** targeted docs reviewed against the language design doc

### CLAR-009: APS wording reconciliation for active modules

- **Intent:** Update active and proposed APS modules whose language would
  otherwise introduce new drift
- **Expected Outcome:** `RCLI2`, `RCLI3`, and other identified modules use the
  canonical terms or explicitly justify exceptions
- **Validation:** targeted module files updated

## Sequencing Recommendation

1. `CLAR-006` runtime naming alignment
2. `CLAR-007` welcome/tutorial rewrite
3. `CLAR-008` docs cleanup
4. `CLAR-009` APS wording reconciliation in parallel where low-risk

Reasoning:

- naming alignment removes the largest contradiction first
- onboarding/tutorial should not be rewritten against unstable runtime names
- docs should reflect the shipped naming and UX, not outrun them
- plan wording updates can happen opportunistically alongside implementation

## Immediate Plan Updates Recommended

Before code changes begin, the following plan updates should be made or queued:

- add a canonical-language note to `RCLI2`
- add a CLI naming note to `RCLI3`
- review dashboard modules for `gate detail`, `check tree`, and `dependency
  graph` wording
- review policy-governance and release specs for uses of `gate` that do not mean
  workflow judgement

## Exit Criteria

`CLAR-005` is complete when there is a clear, bounded path from discovery to
execution without needing another meta-planning round.
