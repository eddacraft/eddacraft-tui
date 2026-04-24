<!-- Archived: 2026-04-24 | Reason: Zombie module against retired Ink/TS stack; absorbed by LAUNCH (watch polish) and RTVF (validation core). -->

# Real-Time Validation: Simplified Scope (AI Output Validation) — SUPERSEDED

| ID   | Owner | Status     |
| ---- | ----- | ---------- |
| RTVS | —     | Superseded |

> **Superseded by:** [launch-flow-readiness (LAUNCH)](../../modules/launch-flow-readiness.aps.md)
> for the watch-mode and TUI dashboard work, and
> [real-time-validation-full (RTVF)](../../modules/real-time-validation-full.aps.md)
> for the validation core engine and reasoning patterns.
>
> This module was drafted against the retired TypeScript Ink CLI
> (`cli/src/tui/...`, `core/src/validation/fast-validator.ts`). Those
> surfaces no longer exist — see archived
> [ink-to-ratatui-port (PORT)](./ink-to-ratatui-port.aps.md)
> (Complete, 15/15). The current stack is the Rust CLI
> (`crates/anvil-cli/`) and Ratatui TUI (`crates/anvil-tui/`).
>
> Rather than re-scope this whole module against the new stack and
> duplicate work already owned by LAUNCH and RTVF, the surviving intent
> is forwarded into those modules and this spec is archived. See the
> per-phase "Superseded by" notes below for the mapping.

## Overview

Enable real-time validation of AI-generated planning documents through enhanced watch mode with fast feedback and terminal TUI. This validates the OUTPUT of external AI tools (Cursor, Aider, Copilot) without requiring AI inside Anvil or tool-specific integrations.

**Current State:** Watch mode exists but is optimized for human save cycles (300ms debounce, sequential checks, basic output). No validation of reasoning quality.

**Target State:** Watch mode optimized for AI-generated content validation with <150ms feedback, parallel check execution, reasoning quality validation, and rich TUI showing issues in real-time.

**Scope:** Terminal-based validation that works with ANY AI tool by watching file system changes. No LSP, no HTTP API, no tool-specific integrations.

## Problem Statement

**User Pain Points:**

1. **AI generates flawed reasoning** — Cursor/Aider write plans with appeals to authority ("Google does it"), unjustified precision ("10x faster"), missing trade-offs
2. **No automated quality check** — Human must manually review all AI output, slow and error-prone
3. **Late feedback loop** — Issues found at PR review, not during generation
4. **Can't trust AI output** — No safety net between AI generation and merge

**Success Criteria:**

- [ ] Watch mode validates AI-generated files in <150ms after file write
- [ ] Detects reasoning quality issues (unjustified claims, missing trade-offs, unstated assumptions)
- [ ] Terminal TUI shows issues clearly with line numbers and suggestions
- [ ] Works with Cursor, Aider, Copilot, Continue.dev (any tool that writes files)
- [ ] Zero false negatives on severe reasoning flaws
- [ ] <10% false positive rate on reasoning patterns

## Solution

### Architecture

```
External AI Tool (Cursor/Aider)
  ↓ Writes file to disk
File System Change
  ↓ Chokidar detects (50-100ms)
Watch Orchestrator
  ↓ Debounce (50ms for AI-generated changes)
Validation Engine (Parallel)
  ├─ Schema validation (~10ms)
  ├─ Antipattern scan (~50ms)
  ├─ AI reasoning validation (~30ms) ← NEW
  ├─ Markdown lint (~5ms)
  └─ Link validation (~20ms)
  ↓ Total: ~100ms
Terminal TUI
  └─ Shows issues with quick actions
```

### Core Components

**1. Fast Validation Engine** (`core/src/validation/fast-validator.ts`)

```typescript
export interface FastValidationOptions {
  checks: {
    schema: boolean;
    antipatterns: boolean;
    aiReasoning: boolean;
    markdown: boolean;
    links: boolean;
  };
  parallelLimit?: number;
}

export async function validateInMemory(
  content: string,
  filePath: string,
  options: FastValidationOptions
): Promise<ValidationResult> {
  // Run all checks in parallel
  const results = await Promise.all([
    options.checks.schema ? validateSchema(content) : [],
    options.checks.antipatterns ? scanAntipatterns(content) : [],
    options.checks.aiReasoning ? validateAIReasoning(content) : [],
    options.checks.markdown ? lintMarkdown(content) : [],
    options.checks.links ? validateLinks(content, filePath) : [],
  ]);

  return aggregateResults(results);
}
```

**2. AI Reasoning Patterns** (`core/src/validation/ai-reasoning/`)

Seven patterns that detect flawed reasoning:

- **AI-001: Unstated Assumptions** — Decisions without explicit assumptions
- **AI-002: Unjustified Precision** — Specific metrics without evidence ("10x faster")
- **AI-003: Appeal to Authority** — "Google does it" without contextual reasoning
- **AI-004: False Dichotomy** — Presents two options as only choices
- **AI-005: Circular Reasoning** — Conclusion restates premise
- **AI-006: Missing Trade-offs** — Only lists benefits, no costs
- **AI-007: Confidence Miscalibration** — Confidence doesn't match evidence

**3. Enhanced Watch Mode** (`cli/src/commands/watch.ts`)

```typescript
// Optimized for AI-generated content
const watchConfig: WatchConfig = {
  debounceMs: 50,              // Fast for AI writes (vs 300ms for humans)
  parallelLimit: undefined,    // All checks in parallel
  checks: {
    schema: true,
    antipatterns: true,
    aiReasoning: true,          // NEW: Reasoning quality
    markdown: true,
    links: true,
  },
};
```

**4. Terminal TUI Dashboard** (`cli/src/tui/commands/watch/ValidationDashboard.tsx`)

```
┌─ Anvil Watch: Real-Time Validation ───────────────────────┐
│ Watching: docs/planning/auth-plan.aps.md                  │
│ Status: ✗ 3 issues found (validated in 94ms)             │
├────────────────────────────────────────────────────────────┤
│ Issues:                                                    │
│                                                            │
│ ⚠ AI-002 (line 42): Unjustified precision                │
│   "10x faster" lacks supporting evidence                  │
│   → Add benchmark data or profiling results               │
│                                                            │
│ ⚠ AI-003 (line 58): Appeal to authority                  │
│   "Netflix uses microservices" without YOUR context       │
│   → Explain why this fits your specific requirements      │
│                                                            │
│ ⚠ AI-006 (line 102): Missing trade-offs                  │
│   Decision lists only benefits without costs              │
│   → Add trade-offs section acknowledging limitations      │
│                                                            │
│ [f] Show full report  [c] Copy issues  [q] Quit          │
└────────────────────────────────────────────────────────────┘
```

### Workflow

**Human using Cursor:**

```
1. Human runs: anvil watch --tui

2. Human prompts Cursor: "Generate authentication plan"

3. Cursor writes plan.aps.md:
   ## AUTH-001: Add OAuth
   Google uses OAuth, so we should too. It's 10x more secure.

4. Watch detects file change (50ms after write)

5. Validates in parallel (~100ms):
   Schema ✓ | Antipatterns ✓ | AI Reasoning ✗ (2 issues)

6. TUI shows:
   ⚠ AI-003: Appeal to authority ("Google uses")
   ⚠ AI-002: Unjustified precision ("10x more secure")

7. Human sees issues, prompts Cursor:
   "Fix the validation issues on lines 42 and 58"

8. Cursor regenerates with context

9. Watch validates again: ✓ All checks passed

10. Human: "Looks good, proceed with implementation"
```

## Implementation

### Phase 1: Fast Validation Engine (3 days)

> **Superseded by:** RTVF Phase 1 (`SERVER-001` — extract validation core).
> RTVF must own the shared validation engine to serve its LSP/HTTP/stdin
> interfaces; the AI reasoning patterns (VALID-002) and content-hash
> caching (VALID-003) move into that core. Note: the original TypeScript
> file paths (`core/src/validation/fast-validator.ts`,
> `core/src/validation/ai-reasoning/`) are obsolete — RTVF will need to
> re-target the Rust crates when it leaves Draft.

**Goal:** In-memory validation with parallel execution and reasoning patterns

**Tasks:**

#### VALID-001: Create fast validation core

**Intent:** Build in-memory validation engine that runs all checks in parallel

**Implementation:**
1. Create `core/src/validation/fast-validator.ts`
2. Extract existing validation logic to use in-memory content
3. Add parallel execution with `Promise.all()`
4. Add timing instrumentation
5. Unit tests targeting <150ms total latency

**Acceptance:**
- All checks run in parallel
- Total execution time <150ms for typical planning doc
- Content-hash caching for repeat validations

**Confidence:** high (parallel execution is straightforward, existing validators are fast)

#### VALID-002: Implement AI reasoning patterns

**Intent:** Detect common reasoning flaws in planning documents

**Implementation:**
1. Create `core/src/validation/ai-reasoning/` directory
2. Implement pattern detectors:
   - `assumptions.pattern.ts` (AI-001)
   - `precision.pattern.ts` (AI-002)
   - `authority.pattern.ts` (AI-003)
   - `fallacies.pattern.ts` (AI-004, AI-005)
   - `tradeoffs.pattern.ts` (AI-006)
   - `confidence.pattern.ts` (AI-007)
3. Pattern testing with fixtures (false positive tuning)
4. Integration with fast validator

**Acceptance:**
- Detects test cases for all 7 patterns
- <10% false positive rate on real documents
- Each pattern runs in <5ms

**Confidence:** medium (pattern detection requires tuning, false positives are likely initially)

#### VALID-003: Add validation caching

**Intent:** Skip re-validation when content hasn't changed

**Implementation:**
1. Add content-hash computation (SHA-256 of file content)
2. LRU cache mapping hash → validation results
3. Cache invalidation on file change
4. Cache statistics tracking

**Acceptance:**
- Cache hit returns results in <1ms
- Cache correctly invalidates on content change
- Cache size limited (e.g., 100 entries max)

**Confidence:** high (content hashing is deterministic, LRU cache is standard pattern)

### Phase 2: Enhanced Watch Mode (2 days)

> **Superseded by:** LAUNCH (watch-flow polish on the Rust CLI / Ratatui
> stack). Specifically: WATCH-001 (debouncing) is absorbed into
> LAUNCH's broader watch-config work; WATCH-002 (parallel validation
> wiring) lands inside the kernel watch loop owned by LAUNCH-001 /
> LAUNCH-003 once RTVF provides the validation core; WATCH-003
> (progress indicators) is folded into LAUNCH-003 (real-time stats
> rollup in the watch TUI).

**Goal:** Optimize watch mode for AI-generated content with faster feedback

**Tasks:**

#### WATCH-001: Optimize watch mode debouncing

**Intent:** Reduce debounce delay for AI-generated file writes

**Implementation:**
1. Add `--debounce` CLI option
2. Detect large file changes (>500 chars) as likely AI-generated
3. Use 50ms debounce for AI changes, 300ms for human typing patterns
4. Add configuration option for debounce override

**Acceptance:**
- Large changes (AI-generated) validated in 50ms after file stabilizes
- Small changes (human edits) still use 300ms debounce
- CLI option overrides auto-detection

**Confidence:** high (debounce logic is simple, file change size is detectable)

#### WATCH-002: Integrate fast validation engine

**Intent:** Wire fast validator into watch mode with parallel execution

**Implementation:**
1. Replace sequential validation with fast validator call
2. Enable parallel check execution
3. Add timing reporting to watch output
4. Handle validation errors gracefully

**Acceptance:**
- Watch mode uses parallel validation
- Total latency <200ms (file detect + validate + render)
- Errors don't crash watch process

**Confidence:** high (integration is straightforward, error handling already exists)

#### WATCH-003: Add progress indicators

**Intent:** Show validation progress for long-running checks

**Implementation:**
1. Add `onProgress` callback to fast validator
2. Show spinner with current check name
3. Show parallel checks running simultaneously
4. Update TUI in real-time

**Acceptance:**
- User sees which checks are running
- Parallel checks shown simultaneously
- TUI updates smoothly (<50ms render time)

**Confidence:** medium (TUI rendering needs optimization, Ink performance can vary)

### Phase 3: Terminal TUI Dashboard (2 days)

> **Superseded by:** LAUNCH-003 (real-time stats rollup in the watch
> TUI) and TUIDASH (the json-render dashboard surface that ultimately
> replaces the bespoke watch panes). The Ink references here
> (`cli/src/tui/commands/watch/ValidationDashboard.tsx`, "Ink
> components", "Ink renderer") are obsolete; equivalent surfaces now
> live under `crates/anvil-tui/src/surfaces/watch/`. TUI-013/-014/-015
> intent (issue list, detail view, copy-to-clipboard) is reframed as
> validation-issue presentation inside the Ratatui watch dashboard and
> belongs to TUIDASH if and when those affordances are wanted.

**Goal:** Rich terminal UI showing validation results with quick actions

**Tasks:**

#### TUI-013: Build validation dashboard component

**Intent:** Interactive TUI showing validation issues with navigation

**Implementation:**
1. Create `cli/src/tui/commands/watch/ValidationDashboard.tsx`
2. Components:
   - Header with file name, status, timing
   - Issue list with line numbers, messages, suggestions
   - Footer with keyboard shortcuts
3. Integrate with Ink renderer
4. Add keyboard shortcuts (f=full report, c=copy, q=quit)

**Acceptance:**
- Shows all validation issues clearly
- Line numbers link to source location
- Keyboard navigation works
- Renders in <50ms

**Confidence:** high (Ink components are mature, similar to existing TUI work)

#### TUI-014: Add issue detail view

**Intent:** Expand selected issue to show full context and suggestions

**Implementation:**
1. Arrow key navigation through issue list
2. Enter key expands issue to show:
   - Full message and explanation
   - Code context (3 lines before/after)
   - Suggested fix
   - Pattern documentation link
3. ESC key collapses back to list view

**Acceptance:**
- Navigation feels responsive
- Code context shows relevant lines
- Suggestions are actionable

**Confidence:** high (similar to existing gate explorer TUI)

#### TUI-015: Add copy-to-clipboard support

**Intent:** Enable copying issues for pasting into AI prompts

**Implementation:**
1. Add 'c' keyboard shortcut
2. Format issues as plain text for clipboard
3. Use `clipboardy` package for cross-platform clipboard
4. Show confirmation message after copy

**Acceptance:**
- Issues copy to clipboard correctly
- Works on macOS, Linux, Windows
- Format is readable for AI prompts

**Confidence:** high (clipboardy is battle-tested)

### Phase 4: Testing & Documentation (2 days)

> **Superseded by:** the receiving modules. Cursor / Aider integration
> testing (TEST-001, TEST-002) belongs to RTVF Phase 3 (AI Tool
> Integrations) where those surfaces are actually built. User
> documentation (DOC-001) and the demo video (DOC-002) ship with
> whichever module lands the user-facing capability — RTVF for the
> validation server, LAUNCH for the watch flow polish.

**Goal:** Validate with real AI tools and document workflows

**Tasks:**

#### TEST-001: Integration testing with Cursor

**Intent:** Validate workflow with real Cursor AI generations

**Implementation:**
1. Generate 10 test plans using Cursor
2. Capture validation results
3. Measure false positive/negative rates
4. Tune reasoning patterns based on results

**Acceptance:**
- <10% false positive rate
- Zero false negatives on severe reasoning flaws
- Validation completes in <150ms

**Confidence:** medium (tuning patterns requires iteration)

#### TEST-002: Integration testing with Aider

**Intent:** Validate workflow with Aider CLI

**Implementation:**
1. Generate 10 test plans using Aider
2. Capture validation results
3. Compare with Cursor results (consistency check)
4. Tune patterns if needed

**Acceptance:**
- Consistent results across AI tools
- Watch mode detects Aider file writes correctly
- Performance remains <150ms

**Confidence:** high (Aider writes files like any other tool)

#### DOC-001: Write user documentation

**Intent:** Document simplified scope workflow for users

**Implementation:**
1. Create `docs/guides/ai-output-validation.md`
2. Sections:
   - Quick start guide
   - Watch mode options
   - Understanding validation issues
   - Workflow with Cursor/Aider
   - Troubleshooting
3. Add examples and screenshots

**Acceptance:**
- New user can set up in <5 minutes
- Common workflows documented
- Troubleshooting covers frequent issues

**Confidence:** high (documentation is straightforward)

#### DOC-002: Create demo video

**Intent:** Show real-time validation workflow

**Implementation:**
1. Record demo showing:
   - Starting watch mode
   - Cursor generating plan with issues
   - Validation catching problems
   - Cursor fixing based on feedback
   - Final validated output
2. Upload to docs/media/
3. Embed in README and docs

**Acceptance:**
- Video shows complete workflow (2-3 minutes)
- Demonstrates value clearly
- Good video quality

**Confidence:** high (screen recording is simple)

## Dependencies

**External:**
- Existing watch mode infrastructure (`core/src/watch/`)
- Existing validation logic (`packages/aps/src/validator/`)
- Existing TUI components (`cli/src/tui/`)
- Ink rendering library

**Internal:**
- None (independent feature)

## Testing Strategy

### Unit Tests

- Fast validator: Parallel execution, caching, timing
- AI reasoning patterns: Each pattern with fixtures
- Watch mode: Debounce logic, file detection
- TUI components: Rendering, keyboard input

### Integration Tests

- End-to-end: File write → validation → TUI update
- Performance: <150ms total latency
- Real AI tools: Cursor and Aider validation

### User Acceptance Testing

- Generate 50 plans with Cursor/Aider
- Measure false positive/negative rates
- Collect user feedback on TUI UX

## Documentation

**User Documentation:**
- [ ] Quick start guide (`docs/guides/ai-output-validation.md`)
- [ ] Watch mode configuration
- [ ] Understanding validation patterns
- [ ] Workflows with different AI tools

**Developer Documentation:**
- [ ] Fast validator API (`core/src/validation/README.md`)
- [ ] Adding new reasoning patterns
- [ ] Performance optimization guide

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| False positives in reasoning patterns | high | high | Extensive testing with real AI output, tunable severity levels |
| Validation too slow (>150ms) | high | medium | Parallel execution, content caching, skip expensive checks in real-time mode |
| TUI performance issues | medium | medium | Throttle updates, optimize Ink rendering, fallback to basic output |
| AI tools don't trigger file changes correctly | high | low | Watch mode already handles file writes from any source |
| Users don't understand validation messages | medium | medium | Clear messages with suggestions, good documentation, examples |

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Validation latency | <150ms | Timing instrumentation |
| False positive rate | <10% | User testing with 50 AI-generated plans |
| False negative rate (severe flaws) | 0% | Manual review of test cases |
| User setup time | <5 minutes | Timed user testing |
| Watch mode stability | >99.9% uptime | Monitor for crashes during 24h test |
| User satisfaction | >8/10 | Post-release survey |

## Open Questions

- [x] Should validation be opt-in or opt-out? **Decision:** Opt-out (enabled by default with `--no-ai-checks` to disable)
- [x] What severity levels for reasoning patterns? **Decision:** warning/info only (not blocking errors)
- [ ] Should we show suggestions inline or in separate panel? **Decision:** TBD after UX testing
- [ ] How to handle very large files (>10k lines)? **Decision:** TBD, may need streaming validation

## Future Work (Out of Scope)

**Not included in simplified scope:**

- ❌ LSP server (editor integration)
- ❌ HTTP API (AI agent integration)
- ❌ stdin interface (CLI tool integration)
- ❌ Auto-fix suggestions
- ❌ AI-powered validation (we validate AI output, don't use AI for validation)
- ❌ VS Code extension enhancements
- ❌ Integration with CI/CD pipelines

**These may be considered for full scope or future phases.**

---

**Status:** Superseded (2026-04-24)
**Superseded by:** LAUNCH (watch + TUI work), RTVF (validation core + reasoning patterns)
**Priority:** —
**Dependencies:** —
**Target Milestone:** —
**Estimated Effort:** — (work redistributed across LAUNCH and RTVF)
