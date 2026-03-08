# Refactor & Reuse Review — `.claude/` Configuration

**Date:** 2026-03-08
**Scope:** All files under `.claude/` — agents, commands, hooks, agent-bus
scripts, rules, skills, prompts

---

## Table of Contents

1. [Initial Findings (Exploration Phase)](#1-initial-findings-exploration-phase)
2. [Assessment Verdicts](#2-assessment-verdicts)
3. [Additional Findings (Deep Scan)](#3-additional-findings-deep-scan)
4. [Summary Matrix](#4-summary-matrix)
5. [Recommended Implementation Order](#5-recommended-implementation-order)

---

## 1. Initial Findings (Exploration Phase)

Fourteen duplication patterns identified across the `.claude/` directory.

### F-01: Negotiation Protocol

**Severity:** HIGH | **Type:** Exact duplication | **~Lines per file:** 15-20

Identical negotiation termination protocol repeated verbatim across all
specialist agent files:

| File | Lines |
| ---- | ----- |
| `agents/architect.md` | 153-162 |
| `agents/code-reviewer.md` | 101-110 |
| `agents/debugger.md` | 152-161 |
| `agents/tdd-coach.md` | 118-127 |
| `agents/security-analyst.md` | 108-117 |
| `agents/forge-reviewer.md` | 107-127 |
| `agents/planner.md` | (implied) |
| `agents/autonomous.md` | (implied) |

**Duplicated block:**

```markdown
When participating in a negotiation (via `/negotiate`), follow this structure:

1. **Read the topic and any previous positions** from other agents
2. **State your position clearly** with [domain-specific] reasoning
3. **End your response** with exactly one of:
   - `CONSENSUS: [agreed approach]`
   - `COUNTER: [your position]`
   - `QUESTION: [clarification needed]`
```

**Total duplicated lines:** ~120-160 across 8 files.

---

### F-02: Trigger Protocol

**Severity:** HIGH | **Type:** Structural duplication | **~Lines per file:**
25-30

Identical trigger emission structure and format in all specialist agents. Only
the agent names and finding categories change per file.

| File | Lines |
| ---- | ----- |
| `agents/architect.md` | 116-151 |
| `agents/code-reviewer.md` | 70-99 |
| `agents/debugger.md` | 119-150 |
| `agents/tdd-coach.md` | 87-116 |
| `agents/security-analyst.md` | 75-106 |
| `agents/planner.md` | 170-197 |
| `agents/autonomous.md` | 99-130 |

**Duplicated structure:**

```markdown
## Trigger Protocol

When your [analysis] reveals issues that another specialist should address,
emit a trigger:

TRIGGER:agent-name:context

### When to Trigger

| Finding | Trigger |
| ...     | ...     |

### Example Output

[markdown with TRIGGER lines]
```

**Total duplicated lines:** ~175-210 across 7 files.

---

### F-03: Agent Introduction Pattern

**Severity:** LOW | **Type:** Structural duplication | **~Lines per file:** 8-12

Nearly identical frontmatter and opening section structure across all 10 agent
definitions. Same YAML fields, same `# Agent Name` heading, same `## When to
Activate` section with bullet list.

**Files affected:** All 10 agents.

**Assessment note:** This is inherent to the agent file format — each agent
*must* have frontmatter and an introduction. Not actionable duplication.

---

### F-04: Review Checklist

**Severity:** MEDIUM | **Type:** Exact duplication | **~Lines:** 32

Identical review checklist with categories (Functionality, Security, Quality,
Testing) and identical checkboxes.

| File | Lines |
| ---- | ----- |
| `commands/review.md` | 12-57 |
| `agents/code-reviewer.md` | 25-57 |

**Total duplicated lines:** ~32 (exact copy).

---

### F-05: Severity Levels / Finding Format

**Severity:** MEDIUM | **Type:** Exact duplication | **~Lines per instance:**
4-8

Identical severity scale definitions (critical, major, minor, nit) with
identical descriptions:

| File | Context |
| ---- | ------- |
| `agents/forge-reviewer.md` | Lines 98-105 |
| `commands/forge.md` | Lines 59-75 |
| `agents/code-reviewer.md` | Lines 61-66 |
| `agents/security-analyst.md` | Line 64 (partial) |

**Duplicated block:**

```markdown
- **critical** — Security vulnerabilities, data loss, crashes. MUST be fixed.
- **major** — Logic errors, missing validation, correctness issues. MUST be fixed.
- **minor** — Edge cases, missing error handling, performance. Author decides.
- **nit** — Style, naming, formatting. Auto-deferred if configured.
```

---

### F-06: Hash / Identifier Generation

**Severity:** MEDIUM | **Type:** Exact code duplication | **~Lines:** 11

Cascading hash function fallback (shasum -> sha256sum -> md5 -> cksum -> tr
fallback) for generating session identifiers.

| File | Lines |
| ---- | ----- |
| `hooks/forge.sh` | 57-71 |

**Note:** Currently only in one file. Flagged because it's a utility pattern
that should be reusable if more scripts need unique identifiers.

---

### F-07: APS Rules Check Pattern

**Severity:** MEDIUM | **Type:** Exact duplication | **~Lines per instance:**
~13

Identical pre-flight check for APS rules existence before creating plans.

| File | Lines |
| ---- | ----- |
| `agents/architect.md` | 22-35 |
| `agents/planner.md` | 17-34 |
| `commands/plan.md` | 10-35 (implied) |

**Duplicated block:**

```markdown
## APS Planning System

When creating architectural plans or proposals, first check if
`plans/aps-rules.md` exists:

If it exists, read it and follow APS conventions:
- Create modules in `plans/modules/` for bounded work areas
- Use lean steps (checkpoints only, no implementation details)
- Tasks describe outcomes, not how to implement
```

---

### F-08: Automatic Consultation Pattern

**Severity:** HIGH | **Type:** Near-exact duplication | **~Lines per
instance:** 44-45

Identical "Automatic Consultation" section with same env var reference, same
"When to Consult" subsection, same "How to Consult" Task-spawning pattern, same
"Consultation Format" table, and same "Skip Consultation When" conditions.

| File | Lines |
| ---- | ----- |
| `agents/architect.md` | 70-114 |
| `agents/planner.md` | 126-169 |

**Total duplicated lines:** ~90 across 2 files. Largest single block of
agent-level exact duplication.

---

### F-09: Git/Bash Guard Patterns

**Severity:** LOW | **Type:** Structural duplication | **~Lines:** 3-5 per
instance

Identical file-existence checks and error-exit patterns across shell scripts.

| File | Pattern |
| ---- | ------- |
| `hooks/forge.sh` | Lines 11-16 |
| `agent-bus/forge-report.sh` | Lines 14-26 |
| `agent-bus/forge-defer.sh` | Lines 22-28 |

**Pattern:**

```bash
if [[ ! -f "$FILE" ]]; then
    echo "Error: ... not found" >&2
    exit 1
fi
```

---

### F-10: JSON Construction Pattern

**Severity:** LOW | **Type:** Structural (idiomatic) | **Instances:** 3+

Consistent use of `jq -n --arg` for safe JSON construction across forge
scripts. Structurally similar but builds different objects each time.

| File | Usage |
| ---- | ----- |
| `hooks/forge.sh` | Lines 104-124 (signal file) |
| `agent-bus/forge-report.sh` | Lines 60, 74, 103 |
| `agent-bus/forge-defer.sh` | Lines 98-99, 142-143, 250 |

---

### F-11: Directory Creation / Bootstrap Pattern

**Severity:** LOW | **Type:** Structural duplication | **~Lines:** 2-3 per
instance

Identical `mkdir -p` calls for logs, signals, and diffs directories.

| File | Lines |
| ---- | ----- |
| `hooks/forge.sh` | 72-77, 93-94 |
| `agent-bus/forge-report.sh` | 123-130 |
| `agent-bus/forge-defer.sh` | 184 |

---

### F-12: Finding/Response Table Rendering

**Severity:** MEDIUM | **Type:** Specification + implementation duplication |
**Instances:** 2

Identical Markdown table structure for rendering findings:

| File | Context |
| ---- | ------- |
| `agent-bus/forge-report.sh` | Lines 45-61 (programmatic generation) |
| `agents/forge-reviewer.md` | Lines 66-96 (specification) |

**Table format:**

```markdown
| ID | File | Severity | Category | Description | Status |
| -- | ---- | -------- | -------- | ----------- | ------ |
```

---

### F-13: Severity-Action Matrix

**Severity:** MEDIUM | **Type:** Exact duplication | **Instances:** 3

Identical decision matrix mapping severity to required action.

| File | Context |
| ---- | ----- |
| `hooks/forge.sh` | Comment block, lines 50-75 |
| `commands/forge.md` | Lines 59-75 |
| `agents/forge-reviewer.md` | Lines 98-105 |

---

### F-14: Stale File Cleanup Pattern

**Severity:** LOW | **Type:** Exact code duplication | **~Lines:** 2-3 per
instance

Identical `find ... -mtime +7 -delete` pattern for cleaning up forge artefacts
older than 7 days.

| File | Lines |
| ---- | ----- |
| `hooks/forge.sh` | Line 99 |
| `agent-bus/forge-report.sh` | Lines 126-130 |

---

## 2. Assessment Verdicts

Each finding assessed for whether refactoring is practical and beneficial.

### Key Constraint

Claude Code agent files (`.claude/agents/*.md`) are **self-contained markdown
documents**. There is no `#include` or import mechanism — each file is read
independently at agent spawn time. This limits deduplication options for agent
markdown to the **rules system** (`.claude/rules/*.md`), which is automatically
injected into all agent contexts.

Shell scripts **can** source shared libraries, so extraction is practical there.

---

### V-01: Negotiation Protocol — REJECT

**Reason:** Agent files must be self-contained. Extracting to a separate file
would require every agent to `Read` it before operating, adding latency and a
failure point. The protocol is short (~15 lines per agent), stable, and rarely
changes. Inline repetition is the right trade-off for self-contained agents.

**Alternative considered:** Moving to `.claude/rules/` — rejected because the
rules system injects content into *all* agents, including those that don't
participate in negotiation (librarian, anvil-plan-spec). This would add
irrelevant context.

---

### V-02: Trigger Protocol — REJECT

**Reason:** Same self-containment constraint as V-01. Additionally, the trigger
tables are **agent-specific** — each agent triggers different downstream agents
based on different finding categories. Only the structural framing is shared;
the actual routing content varies per agent.

---

### V-03: Agent Introduction Pattern — REJECT

**Reason:** This is inherent to the agent file format. Every agent *must* have
YAML frontmatter and an introductory section. Not actionable duplication — it's
a format requirement.

---

### V-04: Review Checklist — ACCEPT

**Mechanism:** Create `.claude/rules/review-checklist.md` with the canonical
checklist. Remove duplicated sections from `commands/review.md` and
`agents/code-reviewer.md`.

**Benefit:** Single source of truth for review criteria. Also makes the
checklist available to other agents (architect, debugger) when they encounter
review-worthy issues.

**Lines saved:** ~32.

---

### V-05: Severity Levels — ACCEPT

**Mechanism:** Create `.claude/rules/severity-levels.md` with canonical
definitions. Remove duplicated definitions from forge-reviewer, forge command,
and code-reviewer. Keep forge-reviewer's specific action rules (MUST fix vs
auto-defer) inline since those are behaviour-specific.

**Lines saved:** ~15.

---

### V-06: Hash Generation — ACCEPT (as part of lib/common.sh)

**Mechanism:** Extract into `.claude/lib/common.sh` as `generate_hash()`
function. Source from `hooks/forge.sh` and any future scripts needing unique
IDs.

**Lines saved:** ~11 per consumer (currently 1, but prevents future
duplication).

---

### V-07: APS Rules Check — ACCEPT

**Mechanism:** Create `.claude/rules/aps-planning.md` with the APS planning
preamble. Remove duplicated sections from architect and planner agents.

**Benefit:** Both agents inherit it via rules system. The `/plan` command also
benefits.

**Lines saved:** ~26.

---

### V-08: Automatic Consultation — ACCEPT

**Mechanism:** Create `.claude/rules/auto-consultation.md` with the
consultation protocol. Remove ~45-line duplicated sections from architect and
planner.

**Benefit:** Largest single deduplication win. Clean separation — the protocol
is identical in both files.

**Lines saved:** ~45.

---

### V-09: Git/Bash Guards — REJECT

**Reason:** These are 3-5 line patterns that are standard shell idioms. The
overhead of sourcing a shared library for basic file-existence checks exceeds
the maintenance cost of the duplication. Not worth abstracting.

---

### V-10: JSON Construction — REJECT

**Reason:** The `jq -n --arg` calls look structurally similar but build
**different JSON objects** with different fields each time. Wrapping in helpers
would add indirection without reducing complexity — field definitions would
just move from `--arg` flags to function arguments.

---

### V-11: Directory Creation — ACCEPT (as part of lib/common.sh)

**Mechanism:** Extract `ensure_forge_dirs()` into `.claude/lib/common.sh`.
Consolidates the 3 places that create forge-related directories.

**Lines saved:** ~6-9.

---

### V-12: Table Rendering — REJECT

**Reason:** Only one place generates tables programmatically
(`forge-report.sh`). The agent file documents the expected format — that's
specification, not duplication. One place to change if the format changes.

---

### V-13: Severity-Action Matrix — PARTIAL ACCEPT

**Reason:** The core severity definitions are covered by V-05. The
action-mapping (MUST fix vs author decides vs auto-defer) is Forge-specific
behaviour and should remain in the Forge command and forge-reviewer agent. No
further action beyond V-05.

---

### V-14: Stale File Cleanup — ACCEPT (as part of lib/common.sh)

**Mechanism:** Extract `cleanup_stale_files()` into `.claude/lib/common.sh`.
Currently in 2 files with identical logic.

**Lines saved:** ~4-6.

---

## 3. Additional Findings (Deep Scan)

Further patterns discovered during deep comparison of all agent and command
files.

### F-15: Autonomous Execution Pattern

**Severity:** MEDIUM | **Type:** Near-exact duplication | **~Lines:** 40-50

The autonomous agent (`agents/autonomous.md`) and the `/autonomous` command
(`commands/autonomous.md`) contain near-identical content:

- Task planning with discrete steps, dependencies, rollback points
- Execution loop: start -> execute -> verify -> checkpoint -> continue
- Progress reporting format: `[PROGRESS]`, `[SUCCESS]`, `[WARNING]`, `[ERROR]`,
  `[BLOCKED]`
- Error handling: 3x retries with exponential backoff
- Safety guardrails: backup before delete, checkpoints before major changes

**Assessment:** The command likely delegates to the agent, making the
duplication redundant. The command should contain only invocation instructions
and defer behaviour to the agent.

**Verdict:** ACCEPT — trim the command to delegation-only, keep full spec in
the agent.

---

### F-16: Debug Methodology Pattern

**Severity:** MEDIUM | **Type:** Near-exact duplication | **~Lines:** 30-35

The debugger agent (`agents/debugger.md`) and `/debug` command
(`commands/debug.md`) share nearly identical debugging methodology:

- 5-step process: Reproduce -> Isolate -> Analyze -> Verify -> Fix
- Output format: Symptoms, Root Cause, Evidence, Fix, Prevention
- "Never guess, always gather evidence" instruction

**Assessment:** Same pattern as F-15 — command duplicates agent behaviour.

**Verdict:** ACCEPT — trim the command to delegation-only.

---

### F-17: Status Reporting Format

**Severity:** LOW | **Type:** Structural duplication | **~Lines per
instance:** 5-8

Multiple agents and commands use the same progress-reporting markers:

| Marker | Used in |
| ------ | ------- |
| `[PROGRESS]` | autonomous agent, autonomous command |
| `[SUCCESS]` | autonomous agent, autonomous command |
| `[WARNING]` | autonomous agent, autonomous command |
| `[ERROR]` | autonomous agent, autonomous command |
| `[BLOCKED]` | autonomous agent, autonomous command |

**Verdict:** REJECT — subsumed by F-15. Once the command is trimmed, this
duplication disappears.

---

### F-18: APS File Structure References

**Severity:** LOW | **Type:** Structural duplication | **~Lines:** 10-15

Both `agents/anvil-plan-spec.md` and `agents/librarian.md` extensively describe
the `plans/` directory structure and file naming conventions.

**Assessment:** Different purposes — APS agent uses the structure to *create*
plans, librarian uses it to *organise* the repo. The overlap is in reference
material, not operational logic. Each agent needs its own copy to operate
independently.

**Verdict:** REJECT — different usage contexts justify the repetition.

---

### F-19: Security Domain Checklist Overlap

**Severity:** LOW | **Type:** Partial duplication | **~Lines:** 8-12

The review checklist (in code-reviewer and review command) includes a "Security"
section that partially overlaps with the security-analyst agent's domain
knowledge:

- "No hardcoded secrets" — both
- "Input validation present" — both
- "No injection vulnerabilities" — both
- "Proper authentication/authorization" — both

**Assessment:** The code-reviewer's checklist is a quick scan; the
security-analyst's analysis is deep. These serve different depths of review and
should remain separate.

**Verdict:** REJECT — different levels of analysis.

---

### F-20: "When to Activate" vs "When to Use" Sections

**Severity:** LOW | **Type:** Structural pattern | **Instances:** 10

Every agent has a "When to Activate" section with bullet-list use cases. These
follow the same structural pattern but have unique content per agent.

**Verdict:** REJECT — inherent to the agent format. Each agent *must* describe
its activation criteria. Content is unique.

---

### F-21: Tool Usage Overlap in Frontmatter

**Severity:** LOW | **Type:** Structural | **Instances:** 10

All agents declare tool access in YAML frontmatter. The common set (Read, Glob,
Grep, Bash) appears in 9 of 10 agents.

**Assessment:** This is configuration, not duplication. Each agent *must*
declare its own tool access. Sharing would require a tool-inheritance mechanism
that doesn't exist.

**Verdict:** REJECT — required by the agent format.

---

### F-22: Forge-Specific Duplication Cluster

**Severity:** MEDIUM | **Type:** Cross-file specification duplication

The Forge pipeline has its specification spread across 4 files with overlapping
content:

| File | Content |
| ---- | ------- |
| `hooks/forge.sh` | Shell implementation + inline documentation |
| `commands/forge.md` | Orchestration protocol + severity matrix |
| `agents/forge-reviewer.md` | Review protocol + finding format + severity |
| `agent-bus/forge-report.sh` | Report format + finding table structure |

Specific overlaps:
- Severity definitions: 3 of 4 files (covered by F-05)
- Finding JSON schema: described in forge-reviewer, consumed by forge-report
- Round progression logic: described in forge command, implied in forge.sh

**Assessment:** Some overlap is inherent — the hook, command, agent, and
utility each need context to operate. But the severity definitions (handled by
V-05) and the action matrix can be centralised.

**Verdict:** PARTIAL ACCEPT — addressed by V-05 and V-13. Remaining overlap is
acceptable specification-near-implementation co-location.

---

### F-23: Planning Methodology Overlap

**Severity:** LOW | **Type:** Thematic duplication

The architect, planner, and `/plan` command all describe planning methodology
with similar themes:

- Architect: "Understand context, map dependencies, analyse patterns, propose"
- Planner: "Requirements analysis, task decomposition, sequencing, risk
  assessment"
- `/plan` command: "Assess state, bootstrap, report, help plan, create files"

**Assessment:** Despite thematic similarity, the actual content is different.
The architect focuses on *design*, the planner on *task breakdown*, the command
on *APS file management*. These are complementary perspectives, not duplication.

**Verdict:** REJECT — different concerns with superficial similarity.

---

### F-24: Error Handling / Retry Logic

**Severity:** LOW | **Type:** Structural duplication | **~Lines:** 3-5

The autonomous agent and autonomous command both specify "3x retries with
exponential backoff" for error handling.

**Verdict:** REJECT — subsumed by F-15. Resolved when command is trimmed.

---

### F-25: Command-Agent Delegation Pattern

**Severity:** MEDIUM | **Type:** Systemic pattern

Multiple commands appear to duplicate the agent they invoke:

| Command | Agent | Overlap |
| ------- | ----- | ------- |
| `/autonomous` | `autonomous.md` | ~80% content overlap |
| `/debug` | `debugger.md` | ~70% content overlap |
| `/review` | `code-reviewer.md` | ~60% content overlap |

**Assessment:** This is a systemic issue. Commands should be thin wrappers that
describe *invocation* (arguments, defaults, delegation target) while agents
contain the *behaviour*. Currently several commands duplicate agent behaviour
because the command prompt is expanded inline rather than delegating.

**Verdict:** ACCEPT — establish a convention that commands are delegation
wrappers. Trim `/autonomous`, `/debug`, and `/review` commands to focus on
argument parsing and agent delegation.

---

### F-26: Consultation Expert Lists

**Severity:** LOW | **Type:** Partial duplication | **Instances:** 2

Both architect and planner agents list which specialists to consult:

- Architect: security-analyst, debugger, tdd-coach
- Planner: architect, security-analyst, code-reviewer

**Assessment:** The lists are *different* — each agent consults a different set
of specialists appropriate to its domain. This is routing configuration, not
duplication.

**Verdict:** REJECT — agent-specific routing.

---

### F-27: `/delegate` Expert Prompt Templates

**Severity:** LOW | **Type:** Partial duplication

The `/delegate` command contains expert prompt templates that partially overlap
with the corresponding agent definitions (architect, code-reviewer,
security-analyst). These are condensed versions used for Codex delegation.

**Assessment:** These prompts target a *different model* (GPT via Codex MCP)
and are intentionally condensed. They share thematic content with agent files
but serve a different execution path.

**Verdict:** REJECT — different execution context (Codex vs Claude agents).

---

## 4. Summary Matrix

### All Findings

| ID | Pattern | Files | Dup Lines | Verdict | Mechanism |
| -- | ------- | ----- | --------- | ------- | --------- |
| F-01 | Negotiation protocol | 8 agents | ~135 | REJECT | Self-contained agents |
| F-02 | Trigger protocol | 7 agents | ~190 | REJECT | Agent-specific content |
| F-03 | Agent introduction | 10 agents | ~100 | REJECT | Format requirement |
| F-04 | Review checklist | 2 files | ~32 | ACCEPT | rules/review-checklist.md |
| F-05 | Severity levels | 4 files | ~15 | ACCEPT | rules/severity-levels.md |
| F-06 | Hash generation | 1 file | ~11 | ACCEPT | lib/common.sh |
| F-07 | APS rules check | 3 files | ~26 | ACCEPT | rules/aps-planning.md |
| F-08 | Auto-consultation | 2 files | ~90 | ACCEPT | rules/auto-consultation.md |
| F-09 | Git/bash guards | 3 files | ~12 | REJECT | Standard shell idiom |
| F-10 | JSON construction | 3 files | ~20 | REJECT | Different objects each time |
| F-11 | Directory creation | 3 files | ~9 | ACCEPT | lib/common.sh |
| F-12 | Table rendering | 2 files | ~12 | REJECT | Spec ≠ implementation |
| F-13 | Severity-action matrix | 3 files | ~12 | PARTIAL | Covered by F-05 |
| F-14 | Stale file cleanup | 2 files | ~6 | ACCEPT | lib/common.sh |
| F-15 | Autonomous exec pattern | 2 files | ~45 | ACCEPT | Trim command |
| F-16 | Debug methodology | 2 files | ~33 | ACCEPT | Trim command |
| F-17 | Status reporting format | 2 files | ~8 | REJECT | Subsumed by F-15 |
| F-18 | APS file structure refs | 2 files | ~12 | REJECT | Different usage contexts |
| F-19 | Security checklist overlap | 2 files | ~10 | REJECT | Different analysis depth |
| F-20 | "When to Activate" sections | 10 files | ~60 | REJECT | Format requirement |
| F-21 | Tool frontmatter overlap | 10 files | ~30 | REJECT | Format requirement |
| F-22 | Forge specification cluster | 4 files | ~30 | PARTIAL | Covered by F-05/F-13 |
| F-23 | Planning methodology | 3 files | ~25 | REJECT | Different concerns |
| F-24 | Error/retry logic | 2 files | ~5 | REJECT | Subsumed by F-15 |
| F-25 | Command-agent delegation | 3 pairs | ~120 | ACCEPT | Trim commands |
| F-26 | Consultation expert lists | 2 files | ~8 | REJECT | Agent-specific routing |
| F-27 | Delegate prompt templates | 1 file | ~20 | REJECT | Different execution context |

### Accepted Changes

| # | Action | Type | Lines Saved | Effort |
| - | ------ | ---- | ----------- | ------ |
| 1 | Create `.claude/rules/review-checklist.md` | Rule extraction | ~32 | Low |
| 2 | Create `.claude/rules/severity-levels.md` | Rule extraction | ~15 | Low |
| 3 | Create `.claude/rules/aps-planning.md` | Rule extraction | ~26 | Low |
| 4 | Create `.claude/rules/auto-consultation.md` | Rule extraction | ~45 | Low |
| 5 | Create `.claude/lib/common.sh` | Shell refactor | ~21 | Low |
| 6 | Trim `/autonomous` command to delegation wrapper | Command cleanup | ~45 | Medium |
| 7 | Trim `/debug` command to delegation wrapper | Command cleanup | ~33 | Medium |
| 8 | Trim `/review` command to delegation wrapper | Command cleanup | ~25 | Medium |

**Estimated total lines deduplicated:** ~242

### Rejected with Reasons

| ID | Pattern | Reason |
| -- | ------- | ------ |
| F-01 | Negotiation protocol | Agent files must be self-contained; no import mechanism; protocol is short and stable |
| F-02 | Trigger protocol | Content is agent-specific (different routing tables); only framing is shared |
| F-03 | Agent introduction | Required by agent file format; not actionable |
| F-09 | Git/bash guards | Standard 3-line shell idiom; abstraction overhead exceeds benefit |
| F-10 | JSON construction | Builds different objects each time; wrapping adds indirection without reducing complexity |
| F-12 | Table rendering | Only one programmatic consumer; agent file documents format (spec ≠ duplication) |
| F-17 | Status reporting | Subsumed by F-15; resolves when autonomous command is trimmed |
| F-18 | APS file structure | Different usage contexts (creation vs organisation); each agent needs own copy |
| F-19 | Security checklist | Code-reviewer does quick scan; security-analyst does deep analysis; different depths |
| F-20 | "When to Activate" | Required structural element; content is unique per agent |
| F-21 | Tool frontmatter | Configuration declaration; each agent must declare its own tools |
| F-23 | Planning methodology | Architect (design), planner (tasks), command (files) — different concerns |
| F-24 | Error/retry logic | Subsumed by F-15 |
| F-26 | Consultation lists | Agent-specific routing tables; lists are intentionally different |
| F-27 | Delegate templates | Targets Codex/GPT, not Claude agents; intentionally condensed |

---

## 5. Recommended Implementation Order

Priority based on impact (lines saved), risk (low = safe), and dependencies.

### Wave 1 — Rules extraction (no dependencies, parallelisable)

1. **`.claude/rules/review-checklist.md`** — Extract from code-reviewer +
   review command
2. **`.claude/rules/severity-levels.md`** — Extract from forge-reviewer + forge
   command + code-reviewer
3. **`.claude/rules/aps-planning.md`** — Extract from architect + planner
4. **`.claude/rules/auto-consultation.md`** — Extract from architect + planner

### Wave 2 — Shell library (independent of Wave 1)

5. **`.claude/lib/common.sh`** — Extract `generate_hash()`,
   `cleanup_stale_files()`, `ensure_forge_dirs()` from forge scripts

### Wave 3 — Command trimming (depends on agent files being stable)

6. **Trim `/autonomous` command** — Keep invocation instructions, remove
   duplicated behaviour spec
7. **Trim `/debug` command** — Keep invocation instructions, remove duplicated
   methodology
8. **Trim `/review` command** — Keep invocation instructions, remove duplicated
   checklist (now in rules)

---

*Review complete. 27 patterns catalogued, 8 accepted for implementation, 15
rejected with documented reasoning, 4 partially accepted (covered by other
accepted items).*
