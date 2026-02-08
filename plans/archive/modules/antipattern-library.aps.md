<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Anti-pattern Library

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| ANTI  | —     | high     | Complete |

## Purpose

Detect high-confidence AI anti-patterns — the "escape hatches" that AI tools use
to produce technically valid but architecturally wrong code. Focus on patterns
that are almost always wrong, not style preferences.

## In Scope

- ESLint disable detection (new disables)
- Type escape detection (`any`, `@ts-ignore`, `@ts-expect-error`)
- Empty catch block detection
- Console statement detection (in production code)
- Pattern library with explanations

## Out of Scope

- Style preferences (formatting, naming)
- Complex semantic analysis
- Auto-fixing

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- AST parsing (TypeScript compiler API or ts-morph)

**Exposes:**

- `AntipatternCheck` — check implementation
- `PatternLibrary` — catalogue of patterns with explanations

## Boundary Rules

- ANTI must focus on high-confidence patterns only
- ANTI must provide clear explanations for each pattern
- ANTI must suggest alternatives, not just flag

## Acceptance Criteria

- [ ] New `eslint-disable` comments trigger warnings
- [ ] New `any` types trigger warnings
- [ ] New `@ts-ignore` comments trigger warnings
- [ ] Each warning includes explanation and suggested fix
- [ ] Patterns configurable in `.anvilrc`

## Risks & Mitigations

| Risk                | Mitigation                       |
| ------------------- | -------------------------------- |
| False positives     | High-confidence patterns only    |
| Developer annoyance | Clear explanations; suppressions |

## Tasks

### ANTI-001: Pattern catalogue definition

- **Intent:** Define the initial set of anti-patterns with explanations
- **Expected Outcome:** Documented catalogue of patterns, each with: pattern,
  explanation, suggestion
- **Scope:** `core/src/antipattern/`
- **Non-scope:** Detection implementation
- **Files:** `core/src/antipattern/patterns.ts`
- **Dependencies:** —
- **Validation:** Code review
- **Confidence:** high

### ANTI-002: ESLint disable detection

- **Intent:** Detect new eslint-disable comments in changed files
- **Expected Outcome:** Scanner that finds eslint-disable comments, compares to
  baseline
- **Scope:** `core/src/antipattern/`
- **Non-scope:** Fixing violations
- **Files:** `core/src/antipattern/eslint-disable-detector.ts`
- **Dependencies:** ANTI-001
- **Validation:** `nx test core`
- **Confidence:** high

### ANTI-003: Type escape detection

- **Intent:** Detect new `any`, `@ts-ignore`, `@ts-expect-error` in changed
  files
- **Expected Outcome:** Scanner that finds type escapes, compares to baseline
- **Scope:** `core/src/antipattern/`
- **Non-scope:** Fixing violations
- **Files:** `core/src/antipattern/type-escape-detector.ts`
- **Dependencies:** ANTI-001
- **Validation:** `nx test core`
- **Confidence:** high

### ANTI-004: Anti-pattern check integration

- **Intent:** Wire anti-pattern detection into the check runner
- **Expected Outcome:** `AntipatternCheck` class that runs during analysis
- **Scope:** `core/src/gate/checks/`
- **Non-scope:** CLI commands
- **Files:** `core/src/gate/checks/antipattern.check.ts`
- **Dependencies:** ANTI-002, ANTI-003, CORE-002
- **Validation:** `nx test core`
- **Confidence:** high

## Decisions

- **D-001:** Start with TypeScript-specific patterns, expand later
- **D-002:** High-confidence only — patterns that are almost always wrong

## Notes

- Existing `core/src/antipattern/` directory exists
- Can leverage TypeScript compiler API for AST analysis
