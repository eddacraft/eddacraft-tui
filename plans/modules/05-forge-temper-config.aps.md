<!--
APS Module: Forge & Temper Configuration & Documentation
=========================================================
Env var registration, settings.json updates, CLAUDE.md documentation,
and toggle behavior for the Forge & Temper pipeline.
See: plans/aps-rules.md
-->

# Forge & Temper Configuration & Documentation

| ID    | Owner  | Status      |
| ----- | ------ | ----------- |
| FTCFG | @aneki | In Progress |

## Purpose

Register all Forge & Temper configuration points (env vars, settings.json
entries, GitHub repo variables), document the pipeline in CLAUDE.md, and ensure
all toggles work correctly in isolation and combination. This module is the
final integration step that makes the pipeline discoverable and controllable.

## In Scope

- Register `CLAUDE_FORGE_ENABLED`, `CLAUDE_FORGE_MAX_ROUNDS`,
  `CLAUDE_FORGE_AUTO_DEFER_NITS` in settings.json and env var table
- Register `CLAUDE_TEMPER_ENABLED`, `CLAUDE_TEMPER_MAX_CYCLES` as GitHub repo
  variables
- Update `settings.json` with forge.sh hook registration
- Update CLAUDE.md hook behavior table with Forge hook
- Update CLAUDE.md env var table with Forge/Temper variables
- Document the Forge/Temper pipeline in CLAUDE.md overview
- Verify all 4 toggle combinations work (both on, forge only, temper only,
  both off)
- Update `on-agent-stop.sh` to handle forge-reviewer triggers if needed

## Out of Scope

- The hook, agent, negotiation, filing, or workflow implementation (Modules 1-4)
- Changes to existing CI review bot configurations
- Changes to the `/addressing-pr-reviews` skill

## Interfaces

**Depends on:**

- FORGE module — hook file path and agent name for registration
- FNEG module — env var names for negotiation configuration
- DEFER module — no direct dependency but documents filing behavior
- TEMPER module — workflow file and GitHub variable names
- Existing `settings.json` — current hook and env var structure
- Existing `CLAUDE.md` — current documentation structure

**Exposes:**

- Updated `settings.json` — with Forge hook registration and defaults
- Updated `CLAUDE.md` — with Forge/Temper documentation
- Toggle matrix — documented behavior for all configuration combinations

## Constraints

- Forge defaults to disabled (`CLAUDE_FORGE_ENABLED=false`) until proven stable
- Temper defaults to disabled (`CLAUDE_TEMPER_ENABLED=false`)
- Existing hook behavior must not change when Forge is disabled
- CLAUDE.md updates must follow the existing table format
- The `/addressing-pr-reviews` skill must continue to work unchanged

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified (all 4 other modules, settings.json, CLAUDE.md)
- [x] All tasks defined
- [x] Toggle matrix defined in design doc

## Tasks

### FTCFG-001: Register Forge env vars and hook in settings.json

- **Status:** Complete
- **Intent:** Add Forge configuration defaults and hook registration to the
  Claude Code settings file
- **Expected Outcome:** `settings.json` includes `CLAUDE_FORGE_ENABLED=true`,
  `CLAUDE_FORGE_MAX_ROUNDS=3`, `CLAUDE_FORGE_AUTO_DEFER_NITS=true` as defaults,
  and `forge.sh` is registered as a PreToolUse hook
- **Validation:** `jq '.hooks' .claude/settings.json` shows forge.sh registered;
  env var defaults are present
- **Files:** `.claude/settings.json`
- **Confidence:** high
- **Notes:** All 3 env vars registered in `settings.json` with Forge enabled.
  CLAUDE.md hook and env var tables updated to reflect `true` state.

### FTCFG-002: Document Temper GitHub repo variables

- **Status:** Complete
- **Intent:** Document the GitHub repo-level Actions variables needed for Temper
- **Expected Outcome:** README or CLAUDE.md documents that
  `CLAUDE_TEMPER_ENABLED` and `CLAUDE_TEMPER_MAX_CYCLES` must be set as GitHub
  Actions variables (Settings > Secrets and variables > Actions > Variables)
- **Validation:** Documentation includes variable names, types, defaults, and
  setup instructions
- **Confidence:** high

### FTCFG-003: Update CLAUDE.md hook behavior table

- **Status:** Complete
- **Intent:** Add Forge hook to the Active Hook Behavior table in CLAUDE.md
- **Expected Outcome:** `forge.sh` appears in the hook table with trigger
  (PreToolUse on git commit), description, and active status
  (`CLAUDE_FORGE_ENABLED=false`)
- **Validation:** CLAUDE.md hook table includes forge.sh row with correct metadata
- **Files:** `CLAUDE.md`
- **Confidence:** high

### FTCFG-004: Update CLAUDE.md env var table

- **Status:** Complete
- **Intent:** Add all Forge and Temper env vars to the Environment Variable
  Toggles table
- **Expected Outcome:** Table includes `CLAUDE_FORGE_ENABLED`,
  `CLAUDE_FORGE_MAX_ROUNDS`, `CLAUDE_FORGE_AUTO_DEFER_NITS`,
  `CLAUDE_TEMPER_ENABLED`, `CLAUDE_TEMPER_MAX_CYCLES` with current values and
  descriptions
- **Validation:** CLAUDE.md env var table includes all 5 new variables
- **Files:** `CLAUDE.md`
- **Dependencies:** FTCFG-001
- **Confidence:** high

### FTCFG-005: Document Forge/Temper pipeline overview in CLAUDE.md

- **Status:** Complete
- **Intent:** Add a section to CLAUDE.md explaining the autonomous code review
  pipeline for both agents and human readers
- **Expected Outcome:** CLAUDE.md includes a "Forge & Temper" section covering:
  pipeline overview, toggle combinations matrix, deferred finding behavior, and
  relationship to existing `/addressing-pr-reviews`
- **Validation:** CLAUDE.md contains a Forge & Temper section with the 4-scenario
  toggle matrix from the design doc
- **Files:** `CLAUDE.md`
- **Dependencies:** FTCFG-003, FTCFG-004
- **Confidence:** high

### FTCFG-006: Verify toggle combinations

- **Intent:** Confirm all 4 toggle combinations produce correct behavior
- **Expected Outcome:** Each combination (both on, forge only, temper only, both
  off) behaves as documented in the toggle matrix -- no interference between
  independent toggles
- **Validation:** Manual verification of each combination with expected outcomes
  documented
- **Dependencies:** FTCFG-001, FTCFG-002, FTCFG-005
- **Confidence:** high
