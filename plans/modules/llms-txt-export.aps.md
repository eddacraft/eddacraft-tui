<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# llms.txt Export

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| LLMS  | —     | high     | Ready  |

## Purpose

Export Anvil architecture rules and constraints in formats AI tools can consume,
enabling "generation-time trust" where AI coding assistants understand project
boundaries before writing code.

**Problem:** AI tools generate code blind to architecture rules. They only learn
about violations after generating code that triggers warnings. This is reactive,
not preventive.

**Solution:** Export architecture baselines, anti-pattern rules, and suppression
policies as machine-readable context that AI tools can consume via:

- `llms.txt` — Standard format for AI tool context
- MCP resources — Real-time constraint exposure
- System prompt fragments — Copy-paste for manual use

## In Scope

- `anvil export --format llms.txt` command
- `anvil export --format mcp-resource` for MCP integration
- `anvil export --format prompt-fragment` for manual use
- Export architecture baseline as constraints
- Export anti-pattern catalogue as rules
- Export suppression policies
- Auto-generate from `.anvilrc` configuration

## Out of Scope

- MCP server implementation (separate module: mcp-server)
- Dynamic constraint updates during AI generation
- IDE extension integration
- Custom export formats beyond llms.txt/mcp/prompt

## Interfaces

**Depends on:**

- `architecture-safety` — Baseline data and boundary definitions
- `antipattern-library` — Pattern catalogue
- `suppressions` — Current suppression policies
- `core/src/config/` — Configuration loading

**Exposes:**

- `anvil export --format llms.txt` — Generate .llms.txt file
- `anvil export --format mcp-resource` — Generate MCP resource JSON
- `anvil export --format prompt-fragment` — Generate copy-paste text
- `ConstraintExporter` — Programmatic API for export

**Output Example (llms.txt):**

```markdown
# Anvil Architecture Constraints

## Boundary Rules

- `src/ui/` must not import from `src/api/` directly
- `src/core/` must not import from `src/cli/`
- Cross-package imports must use `@anvil/*` aliases

## Anti-patterns (Blocked)

- `as any` — Type safety is non-negotiable
- `@ts-ignore` — Fix the type error instead
- Empty catch blocks — Handle or propagate errors
- `eslint-disable` without scope — Disable specific rules only

## Conventions

- UK English spelling (organised, behaviour, colour)
- ESM imports with `.js` extensions required
- Zod schemas as source of truth for types
```

## Acceptance Criteria

- [ ] `anvil export --format llms.txt` generates valid llms.txt
- [ ] Export includes architecture boundaries from baseline
- [ ] Export includes anti-pattern rules with explanations
- [ ] Export includes active suppression policies
- [ ] Export updates when baseline changes
- [ ] MCP resource format compatible with MCP spec
- [ ] Prompt fragment is copy-paste ready
- [ ] < 100ms generation time for typical project

## Tasks

### LLMS-001: Constraint collector

- **Intent:** Aggregate constraints from baseline, patterns, and config
- **Expected Outcome:** Single data structure with all exportable constraints
- **Scope:** `core/src/export/`
- **Non-scope:** Output formatting
- **Files:**
  - `core/src/export/constraint-collector.ts`
  - `core/src/export/constraint-collector.test.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="ConstraintCollector"`
- **Confidence:** high

### LLMS-002: llms.txt formatter

- **Intent:** Format constraints as llms.txt markdown
- **Expected Outcome:** Valid llms.txt output matching community standard
- **Scope:** `core/src/export/formatters/`
- **Non-scope:** MCP or prompt formats
- **Files:**
  - `core/src/export/formatters/llms-txt-formatter.ts`
  - `core/src/export/formatters/llms-txt-formatter.test.ts`
- **Dependencies:** LLMS-001
- **Validation:** `nx test core --testNamePattern="LlmsTxtFormatter"`
- **Confidence:** high

### LLMS-003: MCP resource formatter

- **Intent:** Format constraints as MCP-compatible resource JSON
- **Expected Outcome:** JSON output usable by MCP servers
- **Scope:** `core/src/export/formatters/`
- **Non-scope:** MCP server implementation
- **Files:**
  - `core/src/export/formatters/mcp-resource-formatter.ts`
  - `core/src/export/formatters/mcp-resource-formatter.test.ts`
- **Dependencies:** LLMS-001
- **Validation:** `nx test core --testNamePattern="McpResourceFormatter"`
- **Confidence:** high

### LLMS-004: Prompt fragment formatter

- **Intent:** Format constraints as copy-paste system prompt text
- **Expected Outcome:** Human-readable text for manual AI tool configuration
- **Scope:** `core/src/export/formatters/`
- **Non-scope:** Automated injection
- **Files:**
  - `core/src/export/formatters/prompt-formatter.ts`
  - `core/src/export/formatters/prompt-formatter.test.ts`
- **Dependencies:** LLMS-001
- **Validation:** `nx test core --testNamePattern="PromptFormatter"`
- **Confidence:** high

### LLMS-005: CLI export command

- **Intent:** Add `--format` option to export command for constraint export
- **Expected Outcome:** `anvil export --format llms.txt` works end-to-end
- **Scope:** `cli/src/commands/`
- **Non-scope:** File watching for auto-regeneration
- **Files:**
  - `cli/src/commands/export.ts` — Add format handling
  - `cli/src/commands/export.test.ts` — Integration tests
- **Dependencies:** LLMS-001, LLMS-002, LLMS-003, LLMS-004
- **Validation:** `anvil export --format llms.txt && cat .llms.txt`
- **Confidence:** high

## Decisions

**D-LLMS-001:** Use llms.txt as primary format

- **Rationale:** Emerging standard adopted by Mintlify, Vercel, and others.
  Simple markdown format AI tools can parse easily.
- **Alternatives:** JSON-LD, OpenAPI extensions
- **Trade-offs:** Less structured than JSON, but more human-readable

**D-LLMS-002:** Generate on-demand, not continuously

- **Rationale:** Export when user requests, not on every file change. Avoids
  performance overhead and stale file issues.
- **Alternatives:** Watch mode for auto-regeneration
- **Trade-offs:** User must remember to regenerate; mitigated by CI integration

## Notes

**llms.txt community standard:**

- https://mintlify.com/blog/simplifying-docs-with-llms-txt
- Simple markdown format with sections
- AI tools increasingly support this format

**Future enhancements:**

- Git hook to auto-regenerate on baseline change
- VS Code extension to show constraints inline
- MCP server to expose constraints dynamically
