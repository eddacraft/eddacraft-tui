# Quality Language Runtime Alignment

| ID    | Owner | Status   |
|-------|-------|----------|
| QLRUN | —     | Complete |

## Purpose

Ship the runtime and configuration language alignment identified by CLAR so
`anvil init`, `.anvilrc`, `anvil gate`, and `anvil gate-config` teach one
coherent checks/findings/gate model.

This module exists because CLAR completed the discovery, taxonomy, canonical
language design, onboarding audit, and follow-on slicing work. The remaining
work here is no longer discovery; it is bounded implementation against known
surfaces.

**Parent context:** `CLAR-006` from
`plans/archive/modules/check-language-and-onboarding.aps.md`

## In Scope

- Canonical check naming across guided init, `.anvilrc`, gate output, and
  `gate-config`
- Explicit alias or mapping behaviour where internal runner identifiers still
  differ from user-facing names
- Help text and docs on runtime/config surfaces that must explain the same
  contract
- Tests or validation updates required to prove the naming contract is stable

## Out of Scope

- Welcome or tutorial copy changes
- Broad public docs rewrites outside runtime/config surfaces
- Gate runner rearchitecture
- New checks or new enforcement capabilities

## Interfaces

**Depends on:**

- `plans/specs/2026-04-21-anvil-quality-language-design.md`
- `plans/specs/2026-04-21-language-alignment-execution-slices.md`
- `crates/anvil-cli/src/commands/check_catalog.rs`
- `crates/anvil-cli/src/commands/defaults.rs`
- `crates/anvil-cli/src/commands/gate.rs`
- `crates/anvil-cli/src/commands/gate_config.rs`
- `crates/anvil-cli/src/commands/init.rs`
- `docs/public/anvil/operations/config.md`

**Exposes:**

- One canonical check-name contract for runtime and config surfaces
- APS-tracked completion point for the CLAR runtime slice

## Acceptance Criteria

- [x] Guided init, `.anvilrc`, `gate`, and `gate-config` expose one canonical
      user-facing check catalogue or an explicit documented mapping
- [x] Runtime help and docs clearly distinguish checks, findings, and gate
      judgement
- [x] Internal alias handling is tested where migration or compatibility is
      required

## Tasks

### QLRUN-001: Canonical runtime naming alignment

- **Intent:** Align onboarding, config, gate execution, and gate-config around
  one user-facing naming layer for checks and gates
- **Expected Outcome:** Users see one coherent check/gate vocabulary across
  guided init, `.anvilrc`, gate output, and `.anvil/gate-config.json`, with
  explicit alias handling where migration is needed
- **Files:** `crates/anvil-cli/src/commands/check_catalog.rs`,
  `crates/anvil-cli/src/commands/defaults.rs`,
  `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/commands/gate_config.rs`,
  `crates/anvil-cli/src/commands/init.rs`,
  `docs/public/anvil/operations/config.md`
- **Validation:** One canonical check-name table matches onboarding, gate, and
  gate-config surfaces, or the mapping is explicitly documented and tested
- **Confidence:** medium
- **Dependencies:** CLAR-005
- **Status:** Complete
