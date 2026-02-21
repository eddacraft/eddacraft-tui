<!--
APS Module: .anvil File Format
===============================
Replaces hardcoded TypeScript anti-pattern catalogue with a family-based
file format. Rich definitions for humans and AI, lean rules for detection.
See: plans/aps-rules.md
-->

# .anvil File Format

| ID     | Owner | Status |
| ------ | ----- | ------ |
| ANVFMT | —     | Draft  |

## Purpose

Replace the hardcoded TypeScript anti-pattern catalogue
(`packages/anvil/core/src/antipattern/patterns.ts`) with a file-based format
that:

1. Groups detection rules into **families** sharing a common meta-issue
2. Provides **rich narrative definitions** consumable by humans and AI reviewers
3. Keeps **detection rules lean** — just the mechanics needed to scan
4. Allows the AI receiving a warning to **navigate to the family definition**
   for deeper context, resolving the "loophole at rule boundaries" problem
5. Supports **multiple artifact targets** (source code, PR descriptions, commit
   messages, agent output) — not just source files
6. Works both **real-time** (watch mode) and **after-the-fact** (scan mode)

## Background

### Problems with the current format

The current catalogue stores anti-patterns as TypeScript constants in
`patterns.ts`, `patterns-html.ts`, and `patterns-css.ts`. Each constant
carries its own `explanation`, `suggestion`, and `nudge` fields. This has
several issues discovered through use:

1. **No family grouping.** AP-001 (broad eslint-disable) and AP-004
   (@ts-ignore) are both forms of guardrail suppression, but nothing in the
   data model connects them. An AI agent that passes AP-001 treats it as
   permission, not knowing AP-002 is a sibling on the same spectrum.

2. **Duplicated reasoning.** The explanation for "why suppressing guardrails is
   bad" is repeated (with slight variations) across four patterns. Updating
   the reasoning requires editing four constants.

3. **Conflicting nudges.** AP-006 (empty catch) nudges "at minimum, log it"
   which can trigger AP-007 (console in production). No mechanism exists to
   express tensions between patterns.

4. **Flat severity.** All guardrail suppressions are `warning`. There is no way
   to express that blanket suppression (AP-001) is worse than targeted
   suppression with rationale (AP-005) — they're siblings on a spectrum, not
   independent equals.

5. **Source-only targeting.** The scanner assumes all patterns target source
   files. Behavioral patterns (e.g., AI agent claiming failures are
   "pre-existing" without evidence) target PR descriptions or agent output
   and cannot be represented.

6. **No AI navigation path.** When the nudge is insufficient for an AI to
   determine the right fix, there is no reference to deeper context. The AI
   gets a paragraph and nothing else.

7. **Weak patterns dilute the catalogue.** HTML/CSS patterns (AP-008 through
   AP-013) duplicate what HTMLHint and Stylelint already do better. They
   don't represent Anvil's unique value.

### Design principles

- **Families are first-class.** Every detection rule belongs to a family.
  The family definition is the source of truth for "what is this pattern
  and why does it matter."
- **Definitions are runtime artifacts.** They are not just documentation.
  An AI receiving a warning must be able to resolve and read the family
  definition to make a better decision.
- **One file format.** Both definitions and rules use the same structure:
  YAML frontmatter + markdown body. File extension: `.anvil`.
- **Rules are sensors, definitions are understanding.** A rule says "this
  regex matched." A definition says "here's what that means, why it
  matters, and what to do about it." The nudge (rule's markdown body)
  bridges the two as the error message both audiences see.
- **Targets are explicit.** Each rule declares what artifact types it scans.
  The scanner matches artifacts to rules by target type, not by file
  extension alone.

## In Scope

- `.anvil` file format specification (definition files and rule files)
- Directory structure convention (`patterns/<family>/`)
- YAML frontmatter schemas for both `type: definition` and `type: rule`
- Build/compile step that hydrates rules with definition context
- Scanner changes to load compiled patterns and support artifact types
- Migration of AP-001 through AP-007 to new format
- Addition of Responsibility Laundering family (RL-001 through RL-006)
- Removal of AP-008 through AP-013 (HTML/CSS — deferred to dedicated linters)
- Zod schemas for frontmatter validation

## Out of Scope

- AST-based detection (existing schema supports it; no changes needed)
- Auto-fixing or mechanical transforms
- AI-generated definitions (all definitions are human-authored)
- Custom user-defined patterns (future enhancement)
- Pattern marketplace or sharing mechanism

## File Format Specification

### Overview

Every `.anvil` file has:

```
---
YAML frontmatter (structured metadata)
---

Markdown body (prose content — meaning depends on type)
```

The `type` field in the frontmatter determines the file's role:

- `type: definition` — family definition (body = rich narrative)
- `type: rule` — detection rule (body = nudge text)

### Definition File Schema

File: `patterns/<family>/definition.anvil`

#### Frontmatter

```yaml
# Required
id: string               # Family identifier (kebab-case, e.g., "guardrail-suppression")
type: definition          # Literal
name: string              # Human-readable name (e.g., "Guardrail Suppression")
category: string          # High-level category (see categories below)

# Targeting
targets: string[]         # Artifact types this family applies to
                          # Values: source, pr-description, commit-message, agent-output

# Relationships
related: string[]         # IDs of related families (informational, for AI navigation)
tensions: string[]        # IDs of families with conflicting guidance

# Optional
rules: string[]           # IDs of rules in this family (derived from directory if omitted)
```

#### Categories

| Category              | Description                                     |
| --------------------- | ----------------------------------------------- |
| `escape-hatch`        | Suppressing or bypassing safety tooling          |
| `type-evasion`        | Opting out of the type system                    |
| `error-handling`      | Making failures invisible or poorly channelled   |
| `accountability`      | Deflecting ownership or overstating completion   |
| `deferred-debt`       | Promising future work without tracking artifacts |

Categories are extensible. New families may introduce new categories.

#### Markdown Body (Rich Narrative)

The body is structured markdown with **required** sections. If a section
does not apply to a particular family, include it with a brief explanation
of why (e.g., "## The Spectrum\n\nThis family has a single rule — no
spectrum applies."). This ensures every definition is self-contained and
a reader never has to guess whether a section was omitted intentionally
or by mistake.

Required sections:

```markdown
## What It Is
Brief description of the meta-issue this family represents.

## Why It's Harmful
Concrete harms — what goes wrong when this pattern appears.

## The Spectrum (if applicable)
How rules in this family relate, from worst to most acceptable.

## The Right Response
What to do instead — the family-level suggestion. This is what the
definition provides that individual nudges cannot.

## Detection Signals
Prose description of what to look for (supplements mechanical regex).

## Example
At least one observed example showing the pattern in context,
why it happened, and better alternatives.
```

### Rule File Schema

File: `patterns/<family>/<ID>.anvil`

#### Frontmatter

```yaml
# Required
id: string               # Rule identifier (e.g., "AP-001", "RL-001")
type: rule                # Literal
family: string            # Family ID this rule belongs to (must match directory)
title: string             # Short title for warning display (e.g., "Broad eslint-disable added")
version: number           # Rule version — increment on regex, severity, or nudge changes

# Classification
severity: error | warning | info
confidence: high | medium | low
spectrum_position: number # Position within family spectrum (1 = worst)

# Targeting
targets: string[]         # Artifact types this rule scans
                          # Values: source, pr-description, commit-message, agent-output

# Detection
detection:
  type: regex | ast
  pattern: string         # Required if type: regex
  flags: string           # Optional regex flags (e.g., 'i' for case-insensitive)
  ast_query: string       # Required if type: ast

# File targeting (for source targets only)
file_extensions: string[] # e.g., [.ts, .tsx, .js, .jsx]
allowlist: string[]       # Glob patterns to skip

# Relationships
related: string[]         # IDs of sibling or related rules

# Enablement
enabled: boolean          # Default: true
opt_in: boolean           # Default: false (if true, not included by default)
```

#### Markdown Body (Nudge)

The body is the nudge text — the error message shown to both human reviewers
and AI agents when this rule fires.

Guidelines for nudge text:

- **Imperative voice.** "Don't disable all linting rules." not "All linting
  rules have been disabled."
- **2-4 sentences.** Short enough to read in a terminal, long enough to be
  actionable.
- **Say what to do, not just what not to do.** "Fix the underlying issue or
  disable only that rule" not just "Don't do this."
- **No audience-specific language.** Don't say "you" as if addressing a
  human or an AI specifically. The nudge works for both.
- **Can use markdown formatting.** Inline code, emphasis, etc. The nudge
  may be rendered in multiple contexts (terminal, IDE, MCP response).

### Artifact Types

| Type              | Description                          | Available in     |
| ----------------- | ------------------------------------ | ---------------- |
| `source`          | Source code files (.ts, .js, etc.)   | Real-time + scan |
| `pr-description`  | GitHub PR body text                  | Scan only        |
| `commit-message`  | Git commit messages                  | Scan only        |
| `agent-output`    | AI agent conversation / streaming    | Real-time + scan |

Rules declare which artifact types they target. The scanner matches artifacts
to rules by type. A rule with `targets: [source, pr-description]` will run
against both source files and PR bodies.

### Agent Output Capture

Agent output is available in real-time (watch mode) by nature — the agent
is streaming. The challenge is making it available **after the fact** for
projects that don't run watch mode.

**Solution: session recorder.** The Anvil CLI ships with a lightweight
capture hook that records agent output to a temporary file during the
session:

```
~/.anvil/sessions/<session-id>.log    # Temp file during session
```

**Lifecycle:**

1. **Start** — When a coding session begins (e.g., Claude Code session
   start, detected via shell hook or explicit `anvil session start`),
   create a session log file.
2. **Capture** — Agent output is appended to the log. The capture
   mechanism varies by tool:
   - Claude Code: SessionStart hook writes to log; PostToolUse/Stop
     hooks append agent output
   - Generic: `anvil wrap <command>` proxies stdout/stderr to the log
   - CI: `anvil ci capture` reads from tool-specific log locations
3. **Scan** — On `anvil check` (or at session end), the session log is
   scanned as an `agent-output` artifact. Patterns targeting
   `agent-output` (e.g., RL-001 through RL-006) run against it.
4. **Cleanup** — After scanning, the session log is deleted. No agent
   output persists on disk beyond the session unless explicitly retained
   via `anvil session keep`.

**Privacy considerations:**

- Session logs are local-only, never transmitted
- Stored in a user-specific directory (`~/.anvil/sessions/`)
- Deleted automatically after scan
- `anvil session keep` opt-in for retention (e.g., for auditing)
- `.anvil/sessions/` should be in `.gitignore`

**Fallback:** If no session log exists when `anvil check` runs, agent
output scanning is simply skipped. PR descriptions and commit messages
are still available from git and GitHub. The session recorder is a
best-effort enhancement, not a requirement.

### Directory Structure

```
patterns/
  guardrail-suppression/
    definition.anvil         # Family definition
    AP-001.anvil             # Broad eslint-disable
    AP-002.anvil             # Rule-specific eslint-disable
    AP-004.anvil             # @ts-ignore
    AP-005.anvil             # @ts-expect-error
    GS-001.anvil             # Non-null assertion (!) — new
  type-system-evasion/
    definition.anvil
    AP-003.anvil             # Explicit any (: any declaration)
  error-visibility/
    definition.anvil
    AP-006.anvil             # Empty catch block
    AP-007.anvil             # Console in production code
  responsibility-laundering/
    definition.anvil
    RL-001.anvil             # Unverified pre-existing claim
    RL-002.anvil             # Phantom follow-up tracking
    RL-003.anvil             # Blanket unrelated dismissal
    RL-004.anvil             # Unverified "not touched" claim
    RL-005.anvil             # Deferred without artifact
    RL-006.anvil             # Reply disguised as fix
  deferred-debt/
    definition.anvil
    DD-001.anvil             # TODO/FIXME without tracking reference
    DD-002.anvil             # HACK comment without tracking reference
    DD-003.anvil             # Temporary code without expiry
    DD-004.anvil             # Completion claim with outstanding TODOs
```

Note: AP-003's `as any` variant (type assertion) is conceptually closer to
Guardrail Suppression than Type System Evasion. During migration, consider
whether to split AP-003 into two rules: one for `: any` (stays in
type-system-evasion) and one for `as any` (moves to guardrail-suppression).

## Build / Compile Step

### Input

The `patterns/` directory tree of `.anvil` files.

### Process

1. **Parse** each `.anvil` file: extract YAML frontmatter + markdown body.
2. **Validate** frontmatter against Zod schemas (definition or rule).
3. **Resolve families**: for each rule, locate its family definition.
4. **Hydrate**: combine rule + definition into a compiled pattern object
   that carries everything the scanner and warning emitter need:
   - From the rule: id, detection config, severity, confidence, targets,
     file_extensions, allowlist, nudge (markdown body), related rules
   - From the definition: name, category, explanation (parsed from body),
     suggestion (parsed from body), family ID, tensions, related families
5. **Output** a compiled pattern registry as a JSON file that the scanner
   loads at runtime. JSON avoids a TypeScript build step for pattern-only
   changes — editing a `.anvil` file and recompiling the registry is all
   that's needed. Type safety is enforced by the Zod schemas at load time,
   not by generated TypeScript.

### Output Format

```typescript
interface CompiledPattern {
  // From rule
  id: string;
  family: string;
  title: string;
  severity: 'error' | 'warning' | 'info';
  confidence: 'high' | 'medium' | 'low';
  spectrum_position: number;
  targets: ArtifactType[];
  detection: DetectionConfig;
  file_extensions?: string[];
  allowlist?: string[];
  nudge: string;
  related: string[];
  enabled: boolean;
  opt_in: boolean;

  // From definition
  family_name: string;
  category: string;
  explanation: string;      // Extracted from definition body
  suggestion: string;       // Extracted from definition body
  definition_ref: string;   // Path to definition.anvil for AI navigation
  tensions: string[];
  related_families: string[];
}
```

### Extraction from Definition Body

The build step extracts structured content from the definition's markdown:

- `explanation` ← content under `## Why It's Harmful`
- `suggestion` ← content under `## The Right Response`

These are used to populate the `Warning` object at scan time so warnings
remain self-contained for consumers that don't resolve definition files.

## Scanner Changes

### Artifact-Based Scanning

Replace the current `scanFile(filePath, content)` signature with:

```typescript
interface Artifact {
  type: 'source' | 'pr-description' | 'commit-message' | 'agent-output';
  ref: string;        // File path, PR number, commit SHA, or session ID
  content: string;    // The text to scan
}

function scanArtifact(artifact: Artifact, options?: ScanOptions): ScanResult;
function scanArtifacts(artifacts: Artifact[], options?: ScanOptions): ScanResult[];
```

The scanner matches artifacts to rules by `targets` field. For `source`
artifacts, file extension and allowlist filtering still apply. For other
artifact types, those fields are ignored.

### Backward Compatibility

The existing `scanFile` and `scanFiles` functions become thin wrappers:

```typescript
function scanFile(filePath: string, content: string, options?: ScanOptions): ScanResult {
  return scanArtifact({ type: 'source', ref: filePath, content }, options);
}
```

### Warning Changes

The `Warning` type gains:

```typescript
// New fields
family: string;              // Family ID for AI navigation
definition_ref: string;      // Path to definition.anvil
spectrum_position: number;   // Position within family
```

The existing `explanation`, `suggestion`, and `nudge` fields remain but are
now populated from the compiled pattern (hydrated from definition + rule)
rather than hardcoded on each pattern constant.

## Migration Plan

### Phase 1: Format and Build

1. Define Zod schemas for `.anvil` frontmatter (definition + rule)
2. Create `patterns/` directory structure
3. Write definition.anvil for each family
4. Migrate AP-001 through AP-007 to rule .anvil files
5. Build compiler that produces compiled pattern registry
6. Validate compiled output matches current PATTERNS array behavior

### Phase 2: Scanner Changes

1. Add `Artifact` type and `scanArtifact` function
2. Add `family`, `definition_ref`, `spectrum_position` to Warning
3. Wire scanner to load compiled patterns instead of hardcoded array
4. Keep `scanFile`/`scanFiles` as backward-compatible wrappers
5. Update all existing consumers (CLI, MCP server, VS Code extension)

### Phase 3: New Patterns

1. Write Responsibility Laundering definition.anvil
2. Add RL-001 through RL-006 rule files
3. Write Deferred Debt definition.anvil
4. Add DD-001 through DD-004 rule files
5. Add Non-null assertion (GS-001) to Guardrail Suppression family
6. Implement PR description scanning (artifact type: pr-description)

### Phase 4: Cleanup

1. Remove AP-008 through AP-013 (HTML/CSS patterns)
2. Remove `patterns.ts`, `patterns-html.ts`, `patterns-css.ts`
3. Update documentation and pattern reference
4. Consider splitting AP-003 into declaration vs assertion variants

## Interfaces

**Depends on:**

- `antipattern-library` — current pattern catalogue (being replaced)
- `save-time-trust` — warning schema, gate integration
- Zod — frontmatter validation
- gray-matter or similar — YAML frontmatter parsing

**Exposes:**

- `.anvil` file format (consumed by build step)
- Compiled pattern registry (consumed by scanner)
- `scanArtifact` / `scanArtifacts` API (consumed by CLI, MCP, IDE, CI)

## Risks

| Risk                             | Impact | Mitigation                              |
| -------------------------------- | ------ | --------------------------------------- |
| Build step adds complexity       | Medium | Keep it simple — parse + validate + merge |
| Breaking change to Warning type  | Medium | New fields are additive; existing fields preserved |
| Definition body parsing fragile  | Medium | Use heading-based extraction with fallbacks |
| Regex in YAML needs escaping     | Low    | Document escaping rules; validate in build |
| Over-extraction from definitions | Low    | Only extract explanation + suggestion sections |

## Decisions

- **D-001:** Single file format (`.anvil`) for both definitions and rules.
  Discriminated by `type` field in frontmatter. Avoids needing two parsers.
- **D-002:** Remove AP-008 through AP-013. HTML/CSS patterns are better served
  by dedicated linters. Anvil's value is in patterns that require context,
  families, and AI navigation — not syntax checks that HTMLHint does better.
- **D-003:** Nudge is the markdown body of a rule file, not a YAML field.
  This allows richer formatting and avoids YAML string escaping issues.
- **D-004:** Definitions are runtime artifacts, not just documentation. The
  `definition_ref` on warnings enables AI reviewers to navigate to deeper
  context when the nudge alone is insufficient.
- **D-005:** Backward compatibility via wrapper functions. Existing callers
  of `scanFile` continue to work unchanged.
- **D-006:** `as any` (assertion) vs `: any` (declaration) split is deferred
  to Phase 4. It's the right thing to do but not blocking.
- **D-007:** Compiled output is JSON, not generated TypeScript. Zod validates
  at load time. Pattern edits don't require a TypeScript rebuild.
- **D-008:** Definition body sections are strictly required. Missing sections
  must include an explanation of why they don't apply.
- **D-009:** Pattern versioning via integer `version` field on rules. Bump on
  any change to regex, severity, or nudge text. Enables baseline management,
  changelog generation, and scan caching.
- **D-010:** ID prefixes are registered in a prefix registry to ensure
  uniqueness and provide a mapping from prefix to family. Known prefixes:

  | Prefix | Family                      |
  | ------ | --------------------------- |
  | AP     | (Legacy — mixed families)   |
  | GS     | Guardrail Suppression       |
  | TE     | Type System Evasion         |
  | EV     | Error Visibility            |
  | RL     | Responsibility Laundering   |
  | DD     | Deferred Debt               |

  Legacy `AP-` IDs are kept for backward compatibility. New rules in
  existing families use the family prefix (e.g., GS-001 for non-null
  assertion in Guardrail Suppression).

## Resolved Questions

- **Compiled registry format:** JSON loaded at runtime. Type safety is
  enforced by Zod schemas at load time. Pattern-only changes don't need a
  TypeScript rebuild.
- **Definition body sections:** Strictly required. If a section doesn't
  apply, include it with a brief explanation of why. Every definition
  should be self-contained.
- **Agent output collection:** Session recorder ships with the CLI (see
  Agent Output Capture section). Best-effort capture, graceful degradation
  when unavailable.
- **Pattern versioning:** Yes. The `.anvil` format includes a `version`
  field in rule frontmatter. This tracks when a pattern's regex, severity,
  or nudge text changes, independent of git history. Useful for:
  - Baseline management (a version bump means re-evaluate suppressions)
  - Changelog generation (what changed between Anvil releases)
  - Cache invalidation (scanner can skip unchanged patterns)
- **ID namespacing:** A prefix registry maps prefixes to families and
  ensures uniqueness. Stored alongside the compiled pattern registry.

## Open Questions

- [ ] Should the prefix registry be a separate file (`prefixes.json`) or
      derived from the pattern directory structure at build time?
- [ ] Should `version` be semver or a simple integer? Semver gives more
      granularity (patch = nudge tweak, minor = new detection logic,
      major = breaking change to what fires). Integer is simpler.
- [ ] Should the session recorder support tool-specific adapters (Claude
      Code adapter, Cursor adapter) or a single generic capture mechanism?
