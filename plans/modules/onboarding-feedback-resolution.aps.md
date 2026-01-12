# Onboarding Feedback Resolution

| ID    | Owner    | Status |
| ----- | -------- | ------ |
| ONFBK | @aneki   | Draft  |

## Purpose

Address UX issues discovered during new-user onboarding testing. These issues
prevent the `anvil init` experience from providing meaningful value to first-time
users, particularly in monorepo environments.

## In Scope

- Layer diagram improvements (detection, display, explanation)
- Entry points presentation and grouping
- Architecture summary usefulness
- Monorepo structure recognition

## Out of Scope

- Core architecture analysis engine changes (belongs in architecture-safety)
- New architecture templates (belongs in opa-architecture-integration)
- TUI component library changes (belongs in tui module)

## Interfaces

**Depends on:**

- architecture-safety — layer detection and baseline logic
- tui — display components

**Exposes:**

- Improved `anvil init` architecture summary output

## Feedback Log

### ONFBK-F01: Layer Diagram Shows Empty Buckets

**Observed:** During `anvil init`, the "Detected layer structure" diagram shows
all layers with `[0 files]` even when the project has 77 modules.

**Root cause:** Hardcoded layer patterns (`src/controllers/**`, `src/services/**`,
`src/domain/**`, etc.) don't match monorepo structures like `packages/*/src/...`.

**Impact:** New users see a useless diagram that provides no insight into their
actual architecture.

**Screenshot evidence:** Layer diagram showing:
```
presentation (src/controllers/**, src/routes/**) [0 files]
application (src/services/**, src/use-cases/**) [0 files]
domain (src/domain/**, src/entities/**) [0 files]
infrastructure (src/repositories/**, src/data/**) [0 files]
shared (src/utils/**, src/lib/**) [0 files]
```

### ONFBK-F02: Entry Points List is Overwhelming

**Observed:** 28 entry points displayed as a raw bullet list with paths and types
(package, application, cli) but no grouping or explanation.

**Impact:** New users don't understand what entry points mean, why they matter,
or what to do with this information. Information overload without insight.

**Desired:** Group by package/type, limit display, or summarise with option to
expand.

### ONFBK-F03: No Architecture Explanation

**Observed:** The init output shows numbers (77 modules, 28 entry points) and
empty layer buckets, but doesn't explain:
- What the detected architecture pattern is
- How the codebase is actually organised
- What layers mean or how they relate
- Any actionable insight about this specific project

**Impact:** First impression is that Anvil doesn't understand the codebase.

### ONFBK-F04: Monorepo Structure Not Recognised

**Observed:** Project `kindling-monorepo` uses `packages/kindling-*/src/...`
structure. Layer detection uses single-app patterns that don't match.

**Impact:** Monorepos are common; failing to detect their structure makes Anvil
seem unsuitable for real-world projects.

### ONFBK-F05: TUI Wizard Crashes After Architecture Confirmation

**Observed:** After confirming the architecture summary, the TUI wizard screen
renders (Step 1 of 5 - Configuration Mode) but immediately exits, dropping the
user back to the terminal. No input is accepted; the wizard is not interactive.

**Steps to reproduce:**
1. Run `anvil init`
2. View architecture summary
3. Type "yes" to confirm architecture
4. TUI wizard renders briefly then exits

**Expected:** TUI wizard should remain interactive and accept arrow key / enter
input for option selection.

**Impact:** Complete blocker - user cannot complete setup through TUI wizard.

### ONFBK-F06: --no-tui Flag Ignored

**Observed:** Running `anvil init --force --no-tui` still launches the TUI
wizard instead of falling back to classic CLI prompts.

**Expected:** The `--no-tui` flag should force classic inquirer-based prompts.

**Root cause:** Commander.js `--no-*` flags set `options.tui = false`, not
`options.noTui = true`. The code checks `options.noTui` which is always undefined.
See `cli/src/tui/utils/tty-detection.ts:22` and `cli/src/commands/init.ts:152`.

**Workaround:** Use environment variable instead: `NO_TUI=1 anvil init --force`

## Ready Checklist

Change status to **Ready** when:

- [ ] Purpose and scope are clear
- [ ] Dependencies identified
- [ ] At least one task defined
- [ ] All feedback items have proposed solutions

## Tasks

*No tasks yet — gathering feedback first*

<!-- Tasks will be added once feedback collection is complete and solutions designed -->
