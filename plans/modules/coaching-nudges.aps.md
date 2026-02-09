<!--
APS Module: Coaching Nudges
============================
Adds coaching prompts to anti-pattern detection for AI and human guidance.
See: plans/aps-rules.md
-->

# Coaching Nudges

| ID    | Owner | Status   |
| ----- | ----- | -------- |
| NUDGE | —     | Complete |

## Purpose

Add imperative coaching prompts ("nudges") to anti-pattern detection that
actively guide developers and AI agents toward the right solution instead of
passively reporting violations. Each anti-pattern gets a `nudge` field worded
as a direct instruction that works for both audiences:

- **For AI agents (MCP):** The nudge is returned in `anvil_check` tool
  responses. The AI reads it and naturally adjusts its output — no mechanical
  auto-fix needed. The AI IS the fix engine; the nudge is the prompt.
- **For humans (VS Code):** The nudge appears as a CodeAction ("Rethink") and
  inline hint. It's more actionable than a passive warning squiggle.
- **For humans (CLI):** An `--interactive` flag pauses on each violation and
  displays the nudge, asking the developer to reconsider before continuing.

**Key insight:** For AI agents, the fix for most violations is just a better
prompt. You don't need to mechanically rewrite `any` → `unknown` if you can
tell the AI "think about what type this actually holds." The AI will produce a
better solution than a mechanical transform.

## In Scope

- `nudge` field on `AntiPattern` type — imperative coaching text
- Nudge text for all 7 existing patterns (AP-001 through AP-007)
- Nudge text template for future HTML/CSS patterns (AP-008 through AP-013)
- `nudge` field on `Warning` type — populated from pattern during scan
- MCP: include nudge in `anvil_check` tool response
- VS Code: CodeAction provider that shows nudge as "Rethink" action
- CLI: `--interactive` flag that pauses on violations with nudge display
- Configuration: `nudge.enabled` (default: true), `nudge.interactive`
  (default: false for CLI, always on for MCP)
- `nudge.severity_threshold` — only nudge for warnings/errors, not info

## Out of Scope

- AI-generated nudges (nudges are static, human-authored text)
- Nudge customisation per-project (use suppressions for that)
- Nudge for architecture/boundary violations (these require context-specific
  guidance that can't be templated — future enhancement)
- Blocking mode (nudges inform, they don't prevent)

## Interfaces

**Depends on:**

- `antipattern-library` — pattern catalogue, scanner
- `save-time-trust` — warning schema
- `mcp-server` — tool response format
- `ide-integration` — VS Code diagnostics

**Exposes:**

- `nudge` field on `AntiPattern` and `Warning` types
- `NudgeService` — resolves nudge text with context interpolation
- VS Code `CodeActionProvider` for nudge actions
- CLI `--interactive` flag

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] All tasks defined
- [x] Nudge text drafted for all patterns

## Nudge Text Catalogue

### Existing Patterns

**AP-001 (Broad eslint-disable):**
> Don't disable all linting rules. Identify which specific rule is failing and
> either fix the underlying issue or disable only that one rule with
> `/* eslint-disable specific-rule */`. Blanket disables hide real problems.

**AP-002 (Rule-specific eslint-disable):**
> Before disabling this rule, try to fix the code so it passes. If the disable
> is genuinely necessary, add a comment explaining why this specific case
> can't follow the rule.

**AP-003 (Explicit `any`):**
> Don't use `any` here. Think about what type this value actually holds and
> declare it explicitly. If it comes from an API, define an interface for the
> response shape. If the type is truly unknown, use `unknown` and narrow it
> with type guards before use.

**AP-004 (`@ts-ignore`):**
> Don't suppress this TypeScript error — fix it. If you must suppress, use
> `@ts-expect-error` instead so it fails when the underlying issue is
> resolved. But first, read the actual error message and address the type
> mismatch directly.

**AP-005 (`@ts-expect-error`):**
> This type error is being suppressed rather than fixed. Read the error
> message and resolve the type mismatch. If it's a genuine limitation of the
> type system, keep the `@ts-expect-error` but ensure the comment explains
> exactly why.

**AP-006 (Empty catch block):**
> Don't swallow this error silently. At minimum, log it so failures are
> visible. Better: decide whether this error is recoverable (handle it) or
> not (re-throw it). Silent catch blocks make debugging impossible.

**AP-007 (Console in production):**
> Remove this console statement or replace it with a proper logger that
> supports log levels. Console output in production leaks information and
> clutters output. If this is intentional debugging, wrap it in a
> development-only check.

### HTML/CSS Patterns (for HTMLCSS module)

**AP-008 (Inline styles):**
> Move this inline style to a CSS class. Inline styles can't be overridden
> by stylesheets, break consistency, and make maintenance harder. Define a
> class in your stylesheet and apply it instead.

**AP-009 (Inline scripts):**
> Move this script to an external `.js` file and reference it with
> `<script src="...">`. Inline scripts can't be cached, violate CSP
> policies, and make code harder to test.

**AP-010 (Inline event handlers):**
> Remove this inline event handler and use `addEventListener()` in an
> external script instead. Inline handlers mix behaviour with markup and
> are blocked by strict Content Security Policies.

**AP-011 (Deprecated HTML tags):**
> Replace this deprecated HTML tag with its modern CSS equivalent. Use CSS
> for visual presentation instead of presentational HTML elements.

**AP-012 (`!important` in CSS):**
> Don't use `!important` — it breaks the cascade and makes styles nearly
> impossible to override. Instead, increase the specificity of your
> selector or restructure your CSS to avoid the conflict.

**AP-013 (CSS `@import`):**
> Replace this CSS `@import` with a `<link>` tag in your HTML. `@import`
> blocks parallel downloads and slows page load. Each `@import` creates a
> sequential request.

## Tasks

### NUDGE-001: Add nudge field to schemas ✅

- **Intent:** Add `nudge` field to `AntiPattern` and `Warning` schemas
- **Expected Outcome:** `AntiPatternSchema` has optional `nudge: string`
  field; `WarningSchema` has optional `nudge: string` field; scanner
  populates nudge from pattern during warning creation
- **Files:** `packages/anvil/core/src/antipattern/types.ts`,
  `packages/anvil/core/src/antipattern/scanner.ts`
- **Dependencies:** None (foundational)
- **Validation:** `pnpm -F anvil-core test`
- **Confidence:** high
- **Notes:** Add `nudge` to `AntiPatternSchema` at line 183 alongside
  `suggestion`. Add `nudge` to `WarningSchema` at line 111 alongside
  `suggestion`. Update `createWarningFromMatch` in `scanner.ts:42` to copy
  `pattern.nudge` to the warning.

### NUDGE-002: Author nudge text for all patterns ✅

- **Intent:** Write and add nudge text for AP-001 through AP-013
- **Expected Outcome:** All 7 patterns in `patterns.ts` have `nudge` fields
  with imperative coaching text
- **Files:** `packages/anvil/core/src/antipattern/patterns.ts`
- **Dependencies:** NUDGE-001
- **Validation:** `pnpm -F anvil-core test`
- **Confidence:** high
- **Notes:** Use the nudge text from the catalogue above. Text should be:
  - Imperative voice ("Don't use...", "Move this...", "Remove...")
  - 2-4 sentences maximum
  - Include what to do, not just what not to do
  - Work for both AI agents and human developers
  - Not reference "you" as AI or human specifically

### NUDGE-003: MCP tool nudge integration ✅

- **Intent:** Include nudge in `anvil_check` MCP tool responses
- **Expected Outcome:** When `anvil_check` returns warnings, each warning
  includes its `nudge` field. AI agents read the nudge and adjust their
  code generation accordingly.
- **Files:** `packages/mcp-server/src/tools/check.tool.ts`
- **Dependencies:** NUDGE-001, MCP-002
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="nudge"`
- **Confidence:** high
- **Notes:** The nudge is the most valuable field in the MCP response. It
  should be prominent in the output — not buried in metadata. Consider
  putting it in the `text` content block alongside the warning message so
  the AI can't miss it. Format:
  ```
  ⚠ AP-003: Explicit any type at src/foo.ts:42
  → Don't use `any` here. Think about what type this value actually holds...
  ```

### NUDGE-004: VS Code CodeAction provider ✅

- **Intent:** Add a CodeAction provider that surfaces nudges as "Rethink"
  quick-fix actions in the VS Code lightbulb menu
- **Expected Outcome:** When hovering over an anti-pattern diagnostic, the
  lightbulb menu shows "Anvil: Rethink — [nudge text]" as a hint action.
  For patterns with deterministic fixes (AP-004), also show "Anvil: Fix —
  replace @ts-ignore with @ts-expect-error" as a quick-fix action.
- **Files:** `packages/vscode-extension/src/services/codeActions.ts`,
  `packages/vscode-extension/src/extension.ts`
- **Dependencies:** NUDGE-001
- **Validation:** Manual testing in VS Code
- **Confidence:** high
- **Notes:** Register a `CodeActionProvider` for relevant languages. Map
  diagnostics with `anvil:antipattern` source to CodeActions. The nudge
  action itself doesn't modify code — it shows the coaching text in a
  notification or inline. Deterministic fix actions (AP-003, AP-004) can
  actually modify code via `WorkspaceEdit`.

### NUDGE-005: CLI interactive mode ✅

- **Intent:** Add `--interactive` flag to `anvil check` that pauses on each
  violation and displays the nudge
- **Expected Outcome:** `anvil check --interactive` shows each warning with
  its nudge text and prompts: `[s]kip / [f]ix (if available) / [u]ppress /
  [q]uit`. Non-interactive mode (default) is unchanged.
- **Files:** `apps/anvil-cli/src/commands/check.ts`
- **Dependencies:** NUDGE-002
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="interactive"`
- **Confidence:** medium
- **Notes:** Use Enquirer for the interactive prompt (already a dependency).
  In non-TTY environments (CI), `--interactive` should be ignored with a
  warning. The `[f]ix` option should only appear for patterns with
  deterministic fixes (AP-003, AP-004, AP-001). The `[u]ppress` option
  should insert a suppression comment and re-run.

### NUDGE-006: Configuration and severity threshold ✅

- **Intent:** Add configuration for nudge behaviour
- **Expected Outcome:** Config supports `nudge.enabled` (default: true),
  `nudge.interactive` (default: false), `nudge.severityThreshold`
  (default: `warning` — nudge on warnings and errors, not info)
- **Files:** `packages/platform/config/src/nudge-config.ts`,
  `apps/anvil-cli/src/commands/check.ts`
- **Dependencies:** NUDGE-001
- **Validation:** `pnpm -F config test`
- **Confidence:** high

### NUDGE-007: Tests and documentation ✅

- **Intent:** Full test coverage and documentation for nudge feature
- **Expected Outcome:** Unit tests for nudge field propagation, MCP nudge
  output, CLI interactive mode; docs-site page explaining nudges; pattern
  reference updated with nudge text
- **Files:** test files alongside source, `apps/docs-site/docs/anvil/`
- **Dependencies:** NUDGE-001 through NUDGE-006
- **Validation:** `pnpm test`; docs build succeeds
- **Confidence:** high

## Execution

Steps: [../execution/NUDGE-001.steps.md](../execution/NUDGE-001.steps.md)

## Risks

| Risk                            | Impact | Mitigation                               |
| ------------------------------- | ------ | ---------------------------------------- |
| Nudge text too aggressive       | Medium | Imperative but respectful tone; user testing |
| AI ignores nudge in long output | Medium | Put nudge first in tool response; format prominently |
| Interactive mode slows workflow | Low    | Off by default; only on with explicit flag |
| Nudge fatigue (same message)    | Low    | Short text; suppress mechanism exists     |

## Decisions

- **D-001:** Nudges are static human-authored text, not AI-generated. This
  keeps them deterministic, fast, and auditable.
- **D-002:** Nudges work at the pattern level, not the instance level. Every
  `any` gets the same nudge. Context-specific nudges (e.g., "this variable
  holds a User") would require type analysis and are out of scope.
- **D-003:** Nudges don't block. They inform and coach. Blocking is the job
  of severity levels and gate configuration.
- **D-004:** The MCP integration is the highest-value surface. An AI that
  reads "don't use `any`, think about what type this holds" will produce a
  better fix than a mechanical `any` → `unknown` transform.

## Open Questions

- [ ] Should nudges support variable interpolation (e.g., inserting the
      detected type or import path)? Adds complexity but improves
      specificity.
- [ ] Should the VS Code "Rethink" action open an inline chat prompt with
      the nudge as context? (Requires VS Code chat API)
- [ ] Should the MCP server's `anvil_explain` tool return an expanded nudge
      with more context than the inline version?
