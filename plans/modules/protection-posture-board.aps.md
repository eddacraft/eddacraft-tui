<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Protection Posture Board

| ID     | Owner | Priority | Status   | Progress |
| ------ | ----- | -------- | -------- | -------- |
| POSBRD | —     | medium   | Proposed | 2/5      |

**Last reviewed:** 2026-09-14 — POSBRD-001/002 Merged via #4680 (status
reconciled in #4682). Design approved 2026-08-30; spec accepted at
[`plans/specs/2026-08-30-protection-posture-board.md`](../specs/2026-08-30-protection-posture-board.md).
Exclusive module. Stored `N/M` is reconciled separately under ADR-053. Not a
release claim.

## Purpose

Give operators and agents one place to see the enforcement action ladder and
the current configured / resolved / active level of each lifecycle surface.

v1 is a sibling **Posture** section on `anvil status` (TUI, plain, `--json`),
backed by the SETCON read model. SETINS later renders the same snapshot on its
Status tab. This module does not ship `/settings` or `anvil settings`.

## In Scope

- Correcting `protection.enforcement.mode` catalogue values to the kernel
  posture ladder (`off` < `warn` < `fence` < `interrupt`)
- A four-row posture snapshot (MCP pre-write, intercept, gate, acceptance)
  with configured / resolved / active columns and native verbs on gate and
  acceptance
- Rendering that snapshot as a sibling of the Layers grid on `anvil status`
- Intercept last-action detail (including interrupt-ladder stage) when attested
- Public CLI status docs for the section

## Out of Scope

- `anvil settings` / `/settings` (SETINS)
- A new `anvil protection` or `anvil enforcement` command
- Picking a single absent-key winner for MCP vs daemon (ADR-129 D-4.5)
- Treating kill, `ANVIL_POLICY_ENFORCEMENT=off`, or flag kill switches as
  posture rungs
- Evaluating the four stored rule modes
- Changing intercept routing, gate exit codes, or L4 `on_block` semantics
- POLFIT-006 / DOCDEF-007 public *key* catalogue (agree on ladder values; do
  not absorb those items)

## Interfaces

**Depends on:**

- [SETCON](../archive/modules/settings-truth-contract.aps.md) / [ADR-132](../decisions/132-settings-truth-contract.md) — catalogue, resolver, runtime-state vocabulary
- [ADR-098](../decisions/098-policy-enforcement-reset-gate.md) AD-3 — `EnforcementMode` and `ControlDecision`
- [ADR-129](../decisions/129-policy-surface-inventory-and-precedence.md) — surface inventory and MCP vs daemon absent-key split
- `crates/anvil-cli/src/commands/status.rs` and `crates/anvil-tui/src/surfaces/status` — existing status surfaces
- `schemas/anvil-status.v1.json` — additive `posture` object; keep `anvil.status.v1`

**Exposes:**

- Posture snapshot consumed by `anvil status` now and SETINS Status later
- Correct `protection.enforcement.mode` catalogue enum

**Coordinates with:**

- [SETINS](./settings-inspect-surface.aps.md) — later Status-tab consumer of the same snapshot
- [CLICT](./cli-command-truth.aps.md) — status-family docs after POSBRD-003; no new command name
- [DOCDEF](./docs-definition-layer.aps.md) / [POLFIT](./policy-fit-for-purpose.aps.md) — public key catalogue stays theirs

## Constraints

- Active is SETCON-strict: YAML is never proof.
- Kill is an interrupt-ladder stage, not a fifth posture.
- `block` aliases `interrupt` on posture; it is a decision only on surfaces with nothing to fence.
- Gate and acceptance keep native verbs; they never display `fence` or `interrupt` as their own value.
- Posture rows are not numbered L0–L5.
- Status renders the snapshot; it does not become a second resolver.
- Feature PRs update only their own item `Status:` line (ADR-053).

## Acceptance Criteria

- [ ] `anvil status` (TUI, plain, `--json`) shows the four posture rows with configured / resolved / active
- [ ] Absent `enforcement.mode` resolves MCP to `interrupt` and intercept to `warn`
- [ ] Active is `unknown` / `stale` / `failed` when evidence is missing
- [ ] Catalogue seed rejects `enforce` on `protection.enforcement.mode` and accepts `fence` / `interrupt`
- [ ] Public docs do not claim `anvil settings` exists

## Ready Checklist

Change a work item to **Ready** when:

- [x] Design approved 2026-08-30
- [x] Spec accepted at [`plans/specs/2026-08-30-protection-posture-board.md`](../specs/2026-08-30-protection-posture-board.md)
- [ ] The item has observable outcomes and exact validation commands (already drafted below)
- [ ] An operator promotes that item from Proposed to Ready

## Work Items

### POSBRD-001: Correct SETCON enforcement-mode catalogue

- **Status:** Merged 2026-09-13 via PR #4680
- **Intent:** Stop advertising `off` / `warn` / `enforce` on `protection.enforcement.mode`.
- **Expected Outcome:** Catalogue enum is `off`, `warn`, `fence`, `interrupt`; default remains `warn`. Rule-mode keys keep `off` / `warn` / `enforce`. Min-posture constraints on this key use `EnforcementMode` order and do not collapse `fence` / `interrupt` to `enforce`.
- **Files:** `crates/anvil-settings/src/seed.rs`, `crates/anvil-settings/src/types.rs`, `crates/anvil-settings/src/constraints.rs`
- **Validation:** `cargo test -p eddacraft-anvil-settings -- catalogue_seed`
- **Dependencies:** none
- **Confidence:** high
- **Design:** [protection posture board](../specs/2026-08-30-protection-posture-board.md)

### POSBRD-002: Posture snapshot read model

- **Status:** Merged 2026-09-13 via PR #4680
- **Intent:** Compute one snapshot of four projection rows with configured, resolved, and active cells.
- **Expected Outcome:** Rows are `mcp_pre_write`, `intercept`, `gate`, `acceptance`. Ladder verbs on the first two; native gate and acceptance verbs on the last two. Configured is `(unset)` / `(none)` when the source is absent. Runtime state is `unknown` / `stale` / `failed` / `drift` / `active` per SETCON. Mapping legend is data, not a fifth row.
- **Files:** `crates/anvil-settings/src/posture.rs`, `crates/anvil-settings/src/lib.rs`
- **Validation:** `cargo test -p eddacraft-anvil-settings -- posture_snapshot`
- **Dependencies:** POSBRD-001
- **Confidence:** high
- **Design:** [protection posture board](../specs/2026-08-30-protection-posture-board.md)

### POSBRD-003: `anvil status` Posture section

- **Status:** In Progress
- **Intent:** Show the snapshot beside Layers on every `anvil status` surface.
- **Expected Outcome:** TUI, plain, and `--json` render the same snapshot. JSON adds optional `posture` on `anvil.status.v1` and documents it in `schemas/anvil-status.v1.json`. Layers remain armed/not-armed. No new command name.
- **Files:** `crates/anvil-cli/src/commands/status.rs`, `crates/anvil-cli/src/commands/status_posture.rs`, `crates/anvil-tui/src/surfaces/status/`, `schemas/anvil-status.v1.json`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast status && cargo test -p eddacraft-anvil-tui -- status`
- **Dependencies:** POSBRD-002
- **Confidence:** high
- **Design:** [protection posture board](../specs/2026-08-30-protection-posture-board.md)

### POSBRD-004: Intercept last-action evidence

- **Status:** In Progress
- **Intent:** Show the last attested intercept decision and interrupt-ladder stage without making kill a posture rung.
- **Expected Outcome:** Intercept-row detail carries last decision, stage (`sigint` / `sigterm` / `sigkill` / `job_object` / `already_exited`), and timestamp when the daemon attests them. If `query_status` cannot attest, the detail is omitted. Fences alone do not synthesise last-action.
- **Files:** `crates/anvil-intercept-proto/src/status.rs`, `crates/anvil-intercept/src/last_action.rs`, `crates/anvil-intercept/src/`, `crates/anvil-cli/src/commands/status.rs`
- **Validation:** `cargo test -p eddacraft-anvil-intercept -- last_action`
- **Dependencies:** POSBRD-002
- **Confidence:** medium
- **Design:** [protection posture board](../specs/2026-08-30-protection-posture-board.md)

### POSBRD-005: Public status docs for the Posture section

- **Status:** In Progress
- **Intent:** Document the board on the existing `anvil status` surface.
- **Expected Outcome:** Public CLI status docs name the four rows, the ladder, native gate/acceptance verbs, SETCON-strict Active, and that kill is a ladder stage. They do not claim `anvil settings` exists. POLFIT-006 / DOCDEF-007 remain the key catalogue.
- **Files:** `docs/public/anvil/reference/status.md`, `docs/public/anvil/reference/cli.md`, `apps/anvil-docs-private/sidebars/anvil.ts`
- **Validation:** `pnpm docs:check && pnpm docs:public:check`
- **Dependencies:** POSBRD-003
- **Confidence:** high
- **Design:** [protection posture board](../specs/2026-08-30-protection-posture-board.md)
