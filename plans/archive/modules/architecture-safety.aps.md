<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Architecture Safety

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| ARCH  | —     | high     | Complete |

## Purpose

Detect architecture boundary violations by identifying NEW cross-context
dependencies. The most reliable early signal of drift is a new dependency edge
where a function/class reaches across contexts.

## In Scope

- Architecture baseline inference from existing codebase
- New dependency edge detection
- Boundary rule definition (via config or init)
- Warning generation for violations

## Out of Scope

- Fixing existing legacy violations (acknowledge, don't warn)
- Auto-fixing violations
- Complex architecture modelling beyond init baseline

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- `dependency-cruiser` — already integrated in codebase

**Exposes:**

- `ArchitectureCheck` — check implementation
- `ArchitectureConfig` — boundary rule configuration

## Boundary Rules

- ARCH must not modify source files
- ARCH must acknowledge existing edges without warning

## Acceptance Criteria

- [ ] `anvil init` proposes architecture model from codebase
- [ ] New cross-boundary imports trigger warnings
- [ ] Existing violations are tracked but not warned
- [ ] Boundary rules configurable in `.anvilrc`

## Risks & Mitigations

| Risk                     | Mitigation                              |
| ------------------------ | --------------------------------------- |
| Too many false positives | New edges only; baseline existing       |
| Unclear boundaries       | Interactive init with user confirmation |

## Tasks

### ARCH-001: Baseline inference

- **Intent:** Analyse codebase to infer current architecture boundaries
- **Expected Outcome:** Generate initial boundary map from directory structure
  and imports
- **Scope:** `core/src/architecture/`
- **Non-scope:** Complex pattern detection
- **Files:** `core/src/architecture/analyzer.ts`,
  `core/src/architecture/baseline.ts`
- **Dependencies:** —
- **Validation:** `nx test core`
- **Confidence:** medium
- **Risks:** May need manual adjustment for non-standard structures

### ARCH-002: New edge detection

- **Intent:** Detect when a change introduces a new cross-boundary dependency
- **Expected Outcome:** Compare current imports against baseline, flag new edges
- **Scope:** `core/src/architecture/`
- **Non-scope:** Fixing violations
- **Files:** `core/src/architecture/edge-detector.ts`
- **Dependencies:** ARCH-001
- **Validation:** `nx test core`
- **Confidence:** high

### ARCH-003: Architecture check integration

- **Intent:** Wire architecture detection into the check runner
- **Expected Outcome:** `ArchitectureCheck` class that runs during analysis
- **Scope:** `core/src/gate/checks/`
- **Non-scope:** CLI commands
- **Files:** `core/src/gate/checks/architecture.check.ts`
- **Dependencies:** ARCH-002, CORE-002
- **Validation:** `nx test core`
- **Confidence:** high

### ARCH-004: Init command enhancement

- **Intent:** Enhance `anvil init` to propose architecture boundaries
- **Expected Outcome:** Interactive init that shows inferred boundaries, allows
  confirmation
- **Scope:** `cli/src/commands/`
- **Non-scope:** Complex UI
- **Files:** `cli/src/commands/init.ts`
- **Dependencies:** ARCH-001
- **Validation:** Manual test of `anvil init`
- **Confidence:** medium

## Decisions

- **D-001:** Use directory structure as primary boundary signal
- **D-002:** Baseline existing edges — don't warn on legacy violations

## Notes

- `dependency-cruiser` already integrated for dependency analysis
- Existing `core/src/architecture/` has some infrastructure
