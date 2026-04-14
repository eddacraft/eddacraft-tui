<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Explain Command

| Scope   | Owner | Priority | Status |
| ------- | ----- | -------- | ------ |
| EXPLAIN | —     | high     | Ready  |

## Purpose

Provide deep-dive explanations for warnings to build developer trust and
understanding. When a developer sees a warning they don't understand, they need
immediate context without leaving their terminal.

**Problem:** Developers receive warnings but don't understand why the warning
was raised or how to fix it. This leads to:

- Frustration and loss of trust in the tool
- Blind suppression without understanding the issue
- Repeated violations of the same pattern

**Solution:** `anvil explain <warning-id>` command that provides:

- Why this specific warning was raised
- What the underlying rule is trying to prevent
- How to fix the issue (without being prescriptive)
- When suppression is appropriate
- Links to relevant documentation

## In Scope

- `anvil explain <warning-id>` CLI command
- Warning ID lookup from recent check results
- Detailed explanation generation per warning type
- Architecture boundary violation explanations
- Anti-pattern explanations
- Suppression guidance with examples

## Out of Scope

- Auto-fix suggestions (separate feature: `anvil fix`)
- IDE integration (VS Code hover would use this API)
- Historical warning lookup (only recent results)
- Interactive Q&A about warnings

## Interfaces

**Depends on:**

- `architecture-safety` — Boundary violation context
- `antipattern-library` — Pattern explanations
- `core/src/cache/` — Recent check results
- `suppressions` — Suppression syntax examples

**Exposes:**

- `anvil explain <warning-id>` — CLI command
- `anvil explain --list` — List recent warnings
- `ExplainService` — Programmatic API for explanations
- `WarningExplanation` — Explanation data structure

**Output Example:**

```
$ anvil explain AP-003-src/utils/helpers.ts:42

  Warning: AP-003 — Explicit 'any' type
  File: src/utils/helpers.ts:42
  Code: function parse(data: any) {

  ────────────────────────────────────────────────────────

  WHY THIS WARNING EXISTS

  The 'any' type disables TypeScript's type checking for this value.
  This is problematic because:

  • Errors that would be caught at compile time slip through
  • IDE autocompletion and refactoring tools lose effectiveness
  • The type unsafety spreads to everything that touches this value

  ────────────────────────────────────────────────────────

  HOW TO ADDRESS

  1. If you know the type, use it directly:
     function parse(data: RequestPayload) { ... }

  2. If the type varies, use a union or generic:
     function parse<T>(data: T) { ... }

  3. If truly unknown at compile time, use 'unknown' with type guards:
     function parse(data: unknown) {
       if (isRequestPayload(data)) { ... }
     }

  ────────────────────────────────────────────────────────

  WHEN TO SUPPRESS

  Suppress only if:
  • Third-party library types are incorrect or missing
  • Migration in progress with tracked ticket
  • Performance-critical code where type erasure matters

  Suppression syntax:
  // @anvil-ignore AP-003: [reason]

  Example:
  // @anvil-ignore AP-003: legacy API returns untyped JSON, fixing in JIRA-123

  ────────────────────────────────────────────────────────

  RELATED

  • Documentation: docs/guides/type-safety.md
  • Rule definition: AP-003 in anti-pattern catalogue
  • Similar warnings in this file: 2
```

## Acceptance Criteria

- [ ] `anvil explain AP-003-file:line` shows detailed explanation
- [ ] `anvil explain --list` shows recent warnings with IDs
- [ ] Architecture boundary warnings explain the specific boundary violated
- [ ] Anti-pattern warnings explain the specific pattern and alternatives
- [ ] Explanations include suppression syntax with examples
- [ ] Explanations include links to relevant documentation
- [ ] Unknown warning IDs return helpful error message
- [ ] < 50ms response time (explanations are pre-computed)

## Tasks

### EXPLAIN-001: Warning ID system

- **Intent:** Define unique warning IDs that persist across check runs
- **Expected Outcome:** Warnings have stable IDs based on rule + location
- **Scope:** `core/src/warnings/`
- **Non-scope:** Explanation content
- **Files:**
  - `core/src/warnings/warning-id.ts`
  - `core/src/warnings/warning-id.test.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="WarningId"`
- **Confidence:** high

### EXPLAIN-002: Explanation templates

- **Intent:** Create explanation templates for each warning type
- **Expected Outcome:** Template system with placeholders for context
- **Scope:** `core/src/explain/`
- **Non-scope:** CLI rendering
- **Files:**
  - `core/src/explain/templates/` — Template files per warning type
  - `core/src/explain/template-loader.ts`
  - `core/src/explain/template-loader.test.ts`
- **Dependencies:** EXPLAIN-001
- **Validation:** `nx test core --testNamePattern="ExplanationTemplate"`
- **Confidence:** high

### EXPLAIN-003: Architecture boundary explanations

- **Intent:** Generate explanations for architecture boundary violations
- **Expected Outcome:** Explain which boundary was crossed and why it matters
- **Scope:** `core/src/explain/`
- **Non-scope:** Generic explanations
- **Files:**
  - `core/src/explain/boundary-explainer.ts`
  - `core/src/explain/boundary-explainer.test.ts`
- **Dependencies:** EXPLAIN-002, architecture-safety
- **Validation:** `nx test core --testNamePattern="BoundaryExplainer"`
- **Confidence:** high

### EXPLAIN-004: Anti-pattern explanations

- **Intent:** Generate explanations for anti-pattern warnings
- **Expected Outcome:** Explain the pattern, why it's problematic, alternatives
- **Scope:** `core/src/explain/`
- **Non-scope:** Auto-fix suggestions
- **Files:**
  - `core/src/explain/antipattern-explainer.ts`
  - `core/src/explain/antipattern-explainer.test.ts`
- **Dependencies:** EXPLAIN-002, antipattern-library
- **Validation:** `nx test core --testNamePattern="AntipatternExplainer"`
- **Confidence:** high

### EXPLAIN-005: ExplainService

- **Intent:** Service that coordinates explanation generation
- **Expected Outcome:** Single API for getting explanations by warning ID
- **Scope:** `core/src/explain/`
- **Non-scope:** CLI rendering
- **Files:**
  - `core/src/explain/explain-service.ts`
  - `core/src/explain/explain-service.test.ts`
- **Dependencies:** EXPLAIN-001, EXPLAIN-003, EXPLAIN-004
- **Validation:** `nx test core --testNamePattern="ExplainService"`
- **Confidence:** high

### EXPLAIN-006: CLI explain command

- **Intent:** Add `anvil explain` command to CLI
- **Expected Outcome:** Working CLI command with formatted output
- **Scope:** `cli/src/commands/`
- **Non-scope:** TUI mode (plain text only for v1)
- **Files:**
  - `cli/src/commands/explain.ts`
  - `cli/src/commands/explain.test.ts`
- **Dependencies:** EXPLAIN-005
- **Validation:** `anvil explain --help && anvil explain --list`
- **Confidence:** high

## Decisions

**D-EXPLAIN-001:** Warning IDs include file and line

- **Rationale:** Allows precise lookup. Format: `{rule}-{file}:{line}`
- **Alternatives:** Sequential IDs (not stable across runs)
- **Trade-offs:** IDs are long, but unambiguous

**D-EXPLAIN-002:** Explanations are templates, not AI-generated

- **Rationale:** Predictable, fast, no API dependency. Hand-crafted quality.
- **Alternatives:** LLM-generated explanations
- **Trade-offs:** More maintenance, but more trustworthy

**D-EXPLAIN-003:** Focus on understanding, not prescriptive fixes

- **Rationale:** Developers should understand the issue and choose their fix.
  Prescriptive fixes feel patronising.
- **Alternatives:** Specific code suggestions
- **Trade-offs:** Less actionable, but builds understanding

## Notes

**Future enhancements:**

- `anvil fix <warning-id>` for auto-fix suggestions
- VS Code hover integration using ExplainService
- Interactive mode with follow-up questions
- Team-specific explanation customisation

**Success metrics:**

- Developers use explain command before suppressing
- Suppression rate decreases after explain feature ships
- Positive feedback on explanation quality
