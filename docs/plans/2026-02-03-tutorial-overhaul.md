# Tutorial & Onboarding Overhaul Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to
> implement this plan task-by-task.

**Goal:** Restructure Anvil's tutorial system so a new user can install, scan
their project, turn on the watcher, and see immediate value — then progressively
discover specific capabilities through focused feature tutorials.

**Architecture:** Three-tier tutorial system: (1) Rewrite the interactive
`anvil tutorial` TUI to follow Scan → Watch → Fix instead of Plans → Validate →
Gates, running against the user's actual code. (2) Create a new
`docs/anvil/tutorials/` section in the docs-site with standalone feature
tutorials. (3) Add a new `anvil tutorial` subcommand system
(`anvil tutorial policies`, `anvil tutorial architecture`, etc.) for interactive
feature walkthroughs.

**Tech Stack:** Ink (React TUI), Commander.js, Docusaurus MDX, Rego/OPA,
existing anvil-runtime APIs

---

## Phase 1: Rewrite Core Tutorial (Scan → Watch → Fix)

### Task 1: Define new tutorial step types and state

**Files:**

- Modify: `apps/anvil-cli/src/tui/commands/tutorial/types.ts`
- Test: `apps/anvil-cli/src/tui/commands/tutorial/__tests__/types.test.ts`

**Step 1: Write the failing test**

Test that the new step IDs (`scan`, `watch`, `fix`, `next-steps`) exist, that
step ordering is correct, and that state tracks scan results.

```typescript
import { describe, it, expect } from 'vitest';
import {
  STEP_DEFINITIONS,
  getNextStep,
  getPreviousStep,
  createInitialTutorialState,
} from '../types.js';

describe('tutorial step definitions', () => {
  it('has scan-watch-fix steps in order', () => {
    const stepIds = Object.keys(STEP_DEFINITIONS);
    expect(stepIds).toEqual(['scan', 'watch', 'fix', 'next-steps']);
  });

  it('navigates forward through steps', () => {
    expect(getNextStep('scan')).toBe('watch');
    expect(getNextStep('watch')).toBe('fix');
    expect(getNextStep('fix')).toBe('next-steps');
    expect(getNextStep('next-steps')).toBeUndefined();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `nx test anvil-cli -- --testPathPattern tutorial` Expected: FAIL — old step
IDs (intro, plan, validate, gate, completion) don't match

**Step 3: Rewrite types.ts with new step definitions**

Replace the 5-step plan-centric flow with 4-step scan-centric flow:

- `scan` — run `anvil check --all` against user's project, show results
- `watch` — start watch mode, prompt user to edit a file
- `fix` — guide user to fix one issue, see green checkmark
- `next-steps` — show feature tutorials, docs links, next commands

State should track: `scanResults` (warning count, file count, execution time),
`watchTriggered` (boolean), `fixConfirmed` (boolean).

**Step 4: Run test to verify it passes**

Run: `nx test anvil-cli -- --testPathPattern tutorial` Expected: PASS

**Step 5: Commit**

```bash
git add apps/anvil-cli/src/tui/commands/tutorial/types.ts \
       apps/anvil-cli/src/tui/commands/tutorial/__tests__/types.test.ts
git commit -m "refactor(tutorial): replace plan-centric steps with scan-watch-fix flow"
```

---

### Task 2: Create ScanStep TUI component

**Files:**

- Create: `apps/anvil-cli/src/tui/commands/tutorial/steps/ScanStep.tsx`
- Test: `apps/anvil-cli/src/tui/commands/tutorial/__tests__/scan-step.test.tsx`

**Step 1: Write the failing test**

Test that ScanStep renders, calls `GateRunner.analyzeFiles`, and displays
results summary (file count, warning count, time).

**Step 2: Run test to verify it fails**

Run: `nx test anvil-cli -- --testPathPattern scan-step` Expected: FAIL —
component doesn't exist

**Step 3: Implement ScanStep**

Component should:

1. Auto-detect workspace root via `getWorkspaceRoot()`
2. Gather source files (reuse `getSourceFiles` from check.ts)
3. Run `GateRunner.analyzeFiles` with spinner
4. Display results: "Found N warnings across M files in Xms"
5. Show top 3 warnings as examples (file, line, description)
6. Show "Press Enter to start watch mode →"

Use existing Ink components: `Header`, `ProgressBar`, theme colours.

**Step 4: Run test to verify it passes**

Run: `nx test anvil-cli -- --testPathPattern scan-step` Expected: PASS

**Step 5: Commit**

```bash
git add apps/anvil-cli/src/tui/commands/tutorial/steps/ScanStep.tsx \
       apps/anvil-cli/src/tui/commands/tutorial/__tests__/scan-step.test.tsx
git commit -m "feat(tutorial): add ScanStep component that analyses user's project"
```

---

### Task 3: Create WatchStep TUI component

**Files:**

- Create: `apps/anvil-cli/src/tui/commands/tutorial/steps/WatchStep.tsx`
- Test: `apps/anvil-cli/src/tui/commands/tutorial/__tests__/watch-step.test.tsx`

**Step 1: Write the failing test**

Test that WatchStep renders watch mode status, prompts user to edit a file, and
calls `onComplete` when a file change is detected.

**Step 2: Run test to verify it fails**

**Step 3: Implement WatchStep**

Component should:

1. Show "Watch mode active — edit any source file and save"
2. Start a file watcher (reuse `createWatchOrchestrator` from runtime)
3. When a change is detected, show the check result inline
4. After first detection, show "Press Enter to continue →"
5. If no change after 30s, show hint: "Try editing a .ts file and pressing save"

**Step 4: Run test to verify it passes**

**Step 5: Commit**

```bash
git add apps/anvil-cli/src/tui/commands/tutorial/steps/WatchStep.tsx \
       apps/anvil-cli/src/tui/commands/tutorial/__tests__/watch-step.test.tsx
git commit -m "feat(tutorial): add WatchStep with live file watching"
```

---

### Task 4: Create FixStep TUI component

**Files:**

- Create: `apps/anvil-cli/src/tui/commands/tutorial/steps/FixStep.tsx`
- Test: `apps/anvil-cli/src/tui/commands/tutorial/__tests__/fix-step.test.tsx`

**Step 1: Write the failing test**

Test that FixStep shows a specific warning, explains how to fix it, and detects
when the fix is applied.

**Step 2: Run test to verify it fails**

**Step 3: Implement FixStep**

Component should:

1. Pick the first fixable warning from scan results (prefer AP-003 or AP-006)
2. Show the warning with file path and line number
3. Show the suggested fix
4. Watch for the file to be saved
5. Re-run check — if warning gone, show green checkmark + "That's the loop"
6. If no warnings found in scan, show a simulated example instead

**Step 4: Run test to verify it passes**

**Step 5: Commit**

```bash
git add apps/anvil-cli/src/tui/commands/tutorial/steps/FixStep.tsx \
       apps/anvil-cli/src/tui/commands/tutorial/__tests__/fix-step.test.tsx
git commit -m "feat(tutorial): add FixStep with guided issue resolution"
```

---

### Task 5: Create NextStepsStep TUI component

**Files:**

- Create: `apps/anvil-cli/src/tui/commands/tutorial/steps/NextStepsStep.tsx`
- Test:
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/next-steps-step.test.tsx`

**Step 1: Write the failing test**

Test that NextStepsStep renders feature tutorial links, elapsed time, and
summary of what was learned.

**Step 2: Run test to verify it fails**

**Step 3: Implement NextStepsStep**

Component should show:

1. "Tutorial complete in Xs" with summary stats
2. "What you learned" — scan, watch, fix loop
3. "Explore further" section listing feature tutorials:
   - `anvil tutorial policies` — Write custom rules
   - `anvil tutorial architecture` — Define boundaries
   - `anvil tutorial drift` — Track changes over time
4. "Resources" — docs link, `anvil --help`
5. Cleanup option (same as current CompletionStep)

**Step 4: Run test to verify it passes**

**Step 5: Commit**

```bash
git add apps/anvil-cli/src/tui/commands/tutorial/steps/NextStepsStep.tsx \
       apps/anvil-cli/src/tui/commands/tutorial/__tests__/next-steps-step.test.tsx
git commit -m "feat(tutorial): add NextStepsStep with feature tutorial discovery"
```

---

### Task 6: Wire up new steps in Tutorial.tsx and tutorial command

**Files:**

- Modify: `apps/anvil-cli/src/tui/commands/tutorial/Tutorial.tsx`
- Modify: `apps/anvil-cli/src/tui/commands/tutorial/index.ts`
- Modify: `apps/anvil-cli/src/commands/tutorial.ts`
- Delete: `apps/anvil-cli/src/tui/commands/tutorial/steps/IntroStep.tsx`
- Delete: `apps/anvil-cli/src/tui/commands/tutorial/steps/PlanStep.tsx`
- Delete: `apps/anvil-cli/src/tui/commands/tutorial/steps/ValidateStep.tsx`
- Delete: `apps/anvil-cli/src/tui/commands/tutorial/steps/GateStep.tsx`
- Delete: `apps/anvil-cli/src/tui/commands/tutorial/steps/CompletionStep.tsx`
- Test: `apps/anvil-cli/src/tui/commands/tutorial/__tests__/tutorial.test.tsx`

**Step 1: Update Tutorial.tsx**

Replace old step rendering with new components: ScanStep, WatchStep, FixStep,
NextStepsStep. Update state handling to pass scan results between steps.

**Step 2: Update tutorial command to support subcommands**

Add subcommand routing to `tutorial.ts`:

- `anvil tutorial` (no args) — runs core Scan → Watch → Fix tutorial
- `anvil tutorial policies` — (stub for now, Phase 2)
- `anvil tutorial architecture` — (stub for now, Phase 2)
- `anvil tutorial drift` — (stub for now, Phase 2)
- `anvil tutorial --list` — shows available tutorials

**Step 3: Remove old step files**

Delete IntroStep, PlanStep, ValidateStep, GateStep, CompletionStep.

**Step 4: Update and run tests**

Run: `nx test anvil-cli -- --testPathPattern tutorial` Expected: PASS

**Step 5: Commit**

```bash
git commit -m "feat(tutorial): wire up scan-watch-fix flow, add subcommand routing"
```

---

## Phase 2: Feature Tutorials (CLI Interactive)

### Task 7: Create "Write Your First Policy" interactive tutorial

**Files:**

- Create: `apps/anvil-cli/src/tui/commands/tutorial/features/PolicyTutorial.tsx`
- Create: `apps/anvil-cli/src/tui/commands/tutorial/features/policy-steps/`
- Test:
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/policy-tutorial.test.tsx`

Tutorial flow (interactive TUI):

1. **Intro** — "Policies let you enforce custom rules using OPA/Rego"
2. **Create policy dir** — `mkdir -p .anvil/policies`, show where policies live
3. **Write a policy** — Guide user to create `max-file-length.rego` with
   template:

   ```rego
   package anvil.policies.max_file_length

   violation[msg] {
     count(input.file.lines) > 300
     msg := sprintf("%s exceeds 300 lines (%d)", [input.file.path, count(input.file.lines)])
   }
   ```

4. **Test the policy** — Run `anvil policy test` to validate the Rego
5. **See it fire** — Run `anvil check --all` to see the policy trigger on files
   that exceed 300 lines
6. **Customise** — Show how to adjust the threshold, add more rules

Wire into `anvil tutorial policies` subcommand.

**Commit:**

```bash
git commit -m "feat(tutorial): add interactive policy creation tutorial"
```

---

### Task 8: Create "Define Architecture Boundaries" interactive tutorial

**Files:**

- Create:
  `apps/anvil-cli/src/tui/commands/tutorial/features/ArchitectureTutorial.tsx`
- Create:
  `apps/anvil-cli/src/tui/commands/tutorial/features/architecture-steps/`
- Test:
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/architecture-tutorial.test.tsx`

Tutorial flow:

1. **Intro** — "Architecture boundaries prevent imports from crossing contexts"
2. **Detect structure** — Run `anvil architecture generate` to auto-detect
   current structure
3. **Review template** — Show detected architecture YAML, explain layers
4. **Choose template** — Let user pick from
   starter/layered/hexagonal/clean/ddd/monorepo
5. **Compile** — Run `anvil architecture compile` to generate Rego rules
6. **Validate** — Run `anvil architecture validate` to check for existing
   violations
7. **Fix or suppress** — Guide through one violation fix

Wire into `anvil tutorial architecture` subcommand.

**Commit:**

```bash
git commit -m "feat(tutorial): add interactive architecture boundaries tutorial"
```

---

### Task 9: Create "Track Architecture Drift" interactive tutorial

**Files:**

- Create: `apps/anvil-cli/src/tui/commands/tutorial/features/DriftTutorial.tsx`
- Create: `apps/anvil-cli/src/tui/commands/tutorial/features/drift-steps/`
- Test:
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/drift-tutorial.test.tsx`

Tutorial flow:

1. **Intro** — "Drift detection captures snapshots and tracks architectural
   changes over time"
2. **Capture baseline** — Run `anvil drift snapshot --name baseline`
3. **Show snapshot** — Display what was captured (module count, edges, patterns)
4. **Explain comparison** — Show `anvil drift compare baseline latest` (will be
   identical)
5. **Prompt a change** — Guide user to add an import, save, re-snapshot
6. **Show drift** — Run comparison, highlight the new edge
7. **Generate report** — Run `anvil drift report` to see trend

Wire into `anvil tutorial drift` subcommand.

**Commit:**

```bash
git commit -m "feat(tutorial): add interactive drift tracking tutorial"
```

---

### Task 10: Create "Set Up CI Integration" interactive tutorial

**Files:**

- Create: `apps/anvil-cli/src/tui/commands/tutorial/features/CITutorial.tsx`
- Create: `apps/anvil-cli/src/tui/commands/tutorial/features/ci-steps/`
- Test:
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/ci-tutorial.test.tsx`

Tutorial flow:

1. **Intro** — "Anvil can run in CI to gate pull requests"
2. **Detect CI system** — Check for `.github/workflows/`, `.gitlab-ci.yml`, etc.
3. **Generate workflow** — Create appropriate CI config file with
   `anvil check --all --ci`
4. **Explain exit codes** — 0 = clean, 1 = blocking warnings
5. **Show Git hooks option** — `anvil hooks install` for pre-commit
6. **Summary** — What runs where (local watch + CI gate + optional pre-commit)

Wire into `anvil tutorial ci` subcommand.

**Commit:**

```bash
git commit -m "feat(tutorial): add interactive CI integration tutorial"
```

---

## Phase 3: Documentation Site Tutorials

### Task 11: Rewrite quickstart.md for scan-watch-fix flow

**Files:**

- Modify: `apps/docs-site/docs/anvil/quickstart.md`

Restructure to match the new core tutorial:

1. Install → `npm install -D @eddacraft/anvil-cli`
2. Init → `anvil init`
3. Scan → `anvil check --all` (show example output with real findings)
4. Watch → `anvil watch --source` (show watch mode output)
5. Fix → Fix one issue, see green
6. Next → Link to feature tutorials

Remove the "once published to npm" hedge — write it as if published. Update
"expected output" blocks to show the scan-first experience.

**Commit:**

```bash
git commit -m "docs(quickstart): rewrite for scan-watch-fix flow"
```

---

### Task 12: Create docs-site tutorials section

**Files:**

- Create: `apps/docs-site/docs/anvil/tutorials/index.md`
- Create: `apps/docs-site/docs/anvil/tutorials/writing-policies.md`
- Create: `apps/docs-site/docs/anvil/tutorials/architecture-boundaries.md`
- Create: `apps/docs-site/docs/anvil/tutorials/drift-tracking.md`
- Create: `apps/docs-site/docs/anvil/tutorials/ci-integration.md`
- Create: `apps/docs-site/docs/anvil/tutorials/suppressions.md`

Each tutorial is a standalone, hands-on guide with:

- Prerequisites
- Step-by-step instructions with code blocks
- Expected output at each step
- "Try it yourself" exercises
- Links to reference docs

**tutorials/index.md** — Hub page listing all tutorials with one-line
descriptions and difficulty levels (beginner/intermediate).

**tutorials/writing-policies.md** — Full walkthrough:

1. Where policies live (`.anvil/policies/`)
2. The input schema (what data your policy receives)
3. Writing a `violation` rule from scratch
4. Writing a `warning` rule
5. Testing with `anvil policy test`
6. Seeing it fire with `anvil check`
7. Real-world examples: max file length, banned imports, naming conventions

**tutorials/architecture-boundaries.md** — Full walkthrough:

1. What architecture boundaries protect
2. Using templates vs custom definitions
3. `anvil architecture create` walkthrough
4. Understanding `architecture.yaml`
5. Compiling to Rego rules
6. Running validation
7. Handling violations: fix vs suppress

**tutorials/drift-tracking.md** — Full walkthrough:

1. What architectural drift is
2. Capturing baseline snapshots
3. Comparing snapshots
4. Reading drift reports
5. Integrating with CI for trend tracking

**tutorials/ci-integration.md** — Full walkthrough:

1. GitHub Actions setup (complete YAML)
2. GitLab CI setup
3. Exit codes and what they mean
4. PR comment integration
5. Pre-commit hooks as complement

**tutorials/suppressions.md** — Full walkthrough:

1. When to suppress vs when to fix
2. Inline suppressions (`@anvil-ignore`)
3. File-level suppressions (`@anvil-ignore-file`)
4. Config-level suppressions (directories, patterns)
5. Mandatory explanations and why they matter
6. Reviewing suppressions over time (`anvil evidence`)

**Commit:**

```bash
git commit -m "docs: add tutorials section with policies, architecture, drift, CI, suppressions"
```

---

### Task 13: Update docs-site navigation

**Files:**

- Modify: `apps/docs-site/docusaurus.config.ts` (or sidebar config)
- Modify: `apps/docs-site/docs/anvil/overview.md` (update "Next" links)
- Modify: `apps/docs-site/docs/start-here/choose-your-path.md` (add tutorial
  links)

Add a "Tutorials" section to the Anvil sidebar between "Getting Started" and
"Concepts":

```
Anvil
├── Overview
├── When to Use
├── Quickstart
├── First Project
├── First Gate
├── Tutorials          ← NEW section
│   ├── Index
│   ├── Writing Policies
│   ├── Architecture Boundaries
│   ├── Drift Tracking
│   ├── CI Integration
│   └── Suppressions
├── Concepts
│   ├── Gates
│   ├── Plans
│   └── ...
```

Update "Choose Your Path" to include tutorial links for each persona.

**Commit:**

```bash
git commit -m "docs: add tutorials to sidebar navigation and update cross-links"
```

---

### Task 14: Update first-project.md and first-gate.md

**Files:**

- Modify: `apps/docs-site/docs/anvil/first-project.md`
- Modify: `apps/docs-site/docs/anvil/first-gate.md`

Update both to:

- Reference `anvil check --all` instead of `anvil run` (which doesn't exist as a
  command)
- Cross-link to the new tutorials section
- Ensure consistency with quickstart terminology
- Remove redundancy with the new tutorials (first-project can be shorter,
  linking to tutorials/architecture-boundaries for details)

**Commit:**

```bash
git commit -m "docs: align first-project and first-gate with new tutorial structure"
```

---

## Phase 4: Polish & Integration

### Task 15: Add `anvil tutorial --list` output

**Files:**

- Modify: `apps/anvil-cli/src/commands/tutorial.ts`

Implement `--list` flag that shows:

```
Available tutorials:

  Core:
    anvil tutorial              Scan, watch, and fix (2 min)

  Features:
    anvil tutorial policies     Write custom OPA/Rego rules (5 min)
    anvil tutorial architecture Define architecture boundaries (5 min)
    anvil tutorial drift        Track architecture drift (3 min)
    anvil tutorial ci           Set up CI integration (3 min)

Run any tutorial:  anvil tutorial <name>
Reset progress:    anvil tutorial --reset
```

**Commit:**

```bash
git commit -m "feat(tutorial): add --list flag showing available tutorials"
```

---

### Task 16: End-to-end test for core tutorial flow

**Files:**

- Create: `apps/e2e/src/tutorial.spec.ts`

Playwright test that:

1. Runs `anvil tutorial` in a test project with known anti-patterns
2. Verifies scan step finds expected warnings
3. Verifies watch step activates
4. Verifies next-steps step renders

**Commit:**

```bash
git commit -m "test(e2e): add tutorial flow end-to-end test"
```

---

## Dependency Graph

```
Task 1 (types) ──┬── Task 2 (ScanStep) ──┬── Task 6 (wire up)
                  ├── Task 3 (WatchStep) ─┤
                  ├── Task 4 (FixStep) ───┤
                  └── Task 5 (NextSteps) ─┘
                                            │
                                            ├── Task 7  (policy tutorial)
                                            ├── Task 8  (architecture tutorial)
                                            ├── Task 9  (drift tutorial)
                                            ├── Task 10 (CI tutorial)
                                            │
Task 6 ── Task 11 (quickstart rewrite)     │
Task 7-10 ── Task 12 (docs tutorials)     │
Task 12 ── Task 13 (navigation)            │
Task 11,13 ── Task 14 (update existing)    │
Task 6,7-10 ── Task 15 (--list flag)      │
Task 6 ── Task 16 (e2e test)              │
```

## Summary

| Phase                | Tasks | What it delivers                                        |
| -------------------- | ----- | ------------------------------------------------------- |
| 1: Core Tutorial     | 1-6   | Scan → Watch → Fix interactive experience               |
| 2: Feature Tutorials | 7-10  | Policies, Architecture, Drift, CI interactive tutorials |
| 3: Docs Tutorials    | 11-14 | Written tutorials on docs-site with navigation          |
| 4: Polish            | 15-16 | Discovery (`--list`) and e2e confidence                 |
