<!--
APS Module: Quality Language Onboarding and Docs
===============================================
Implements the onboarding/docs naming slices reparented from CLAR-007/008.
See: plans/aps-rules.md
-->

# Quality Language Onboarding and Docs

| ID    | Owner | Status   | Progress |
|-------|-------|----------|----------|
| QLODX | —     | Complete | 2/2      |

## Purpose

Apply CLAR's canonical language model to first-run teaching surfaces and public
docs so Anvil explains the same checks/findings/gate model everywhere a new
user learns it.

This module exists because CLAR already completed the discovery and planning
work. The remaining tasks are now bounded implementation changes across welcome,
tutorial, and docs surfaces.

**Parent context:** `CLAR-007` and `CLAR-008` from
`plans/archive/modules/check-language-and-onboarding.aps.md`

## In Scope

- Welcome flow copy and structure changes needed to teach the canonical model
- Tutorial path labelling and descriptions aligned to the same model
- Public docs terminology cleanup for onboarding, config, and architecture
  tutorial surfaces
- Validation that these surfaces no longer introduce conflicting result nouns or
  overloaded gate language

## Out of Scope

- Runtime/config contract changes already tracked under `QLRUN`
- Dashboard or marketing-site information architecture
- Low-level internal renames with no user-facing impact

## Interfaces

**Depends on:**

- `plans/specs/2026-04-21-anvil-quality-language-design.md`
- `plans/specs/2026-04-21-onboarding-language-gap-audit.md`
- `plans/specs/2026-04-21-language-alignment-execution-slices.md`
- `crates/anvil-cli/src/commands/welcome.rs`
- `crates/anvil-tui/src/surfaces/tutorial/`
- `docs/public/anvil/operations/config.md`
- `docs/public/anvil/tutorials/architecture.md`
- `docs/public/anvil/`

**Exposes:**

- Aligned first-run teaching surfaces
- Aligned public docs for the CLAR language model

## Acceptance Criteria

- [x] Welcome and tutorial teach scan -> checks -> findings -> gate before
      subsystem detail
- [x] Tutorial path descriptions are framed around user understanding rather
      than raw command names
- [x] Targeted public docs use the canonical terminology and avoid incompatible
      parallel result vocabularies

## Tasks

### QLODX-001: Welcome and tutorial model rewrite

- **Intent:** Rewrite first-run teaching surfaces so they explain the canonical
  model before introducing subsystem-specific commands and modes
- **Expected Outcome:** Welcome and tutorial explicitly teach
  scan -> checks -> findings -> gate, and quick actions/path descriptions are
  framed around user goals rather than raw command names
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/`
- **Validation:** First-run flow contains an explicit model explanation and
  tutorial path text aligns with the canonical language design
- **Confidence:** medium
- **Dependencies:** QLRUN-001
- **Status:** Complete

### QLODX-002: Public docs terminology cleanup

- **Intent:** Bring config and tutorial docs into alignment with the canonical
  quality language
- **Expected Outcome:** Public docs teach one quality model, use `finding` as
  the generic result noun where appropriate, and no longer imply incompatible
  check systems
- **Files:** `docs/public/anvil/operations/config.md`,
  `docs/public/anvil/tutorials/architecture.md`, `docs/public/anvil/`
- **Validation:** Targeted docs reviewed against
  `plans/specs/2026-04-21-anvil-quality-language-design.md`
- **Confidence:** medium
- **Dependencies:** QLRUN-001, QLODX-001
- **Status:** Complete
