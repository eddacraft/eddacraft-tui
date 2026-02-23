<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Tutorial Path Continuation

| Scope    | Owner | Priority | Status    |
| -------- | ----- | -------- | --------- |
| Tutorial | —     | medium   | Completed |

## Purpose

When a user completes any tutorial path (core, policies, architecture, drift,
ci), the only options are `c` (cleanup) and `q` (quit). To try another tutorial,
they must exit and re-run the command. This adds friction — the user should be
able to pick the next path directly from the completion screen.

## In Scope

- "Continue with another tutorial" prompt on the last step of every tutorial
- Number-key selection to launch another tutorial in-place
- Loop in the command handler so tutorials chain without process exit
- Shared `TutorialPicker` display component

## Out of Scope

- Changing tutorial step content or ordering
- Adding new tutorial paths
- Progress tracking across tutorials (each tutorial is independent)
- Non-TUI (plain text) mode

## Interfaces

**Depends on:**

- Ink TUI framework (`ink`, `react`)
- Existing tutorial components (`Tutorial.tsx`, `*Tutorial.tsx`)
- `renderTUIAndWait()` in `tui/utils/renderer.tsx`
- `AVAILABLE_TUTORIALS` in `commands/tutorial.ts`

**Exposes:**

- `TutorialPicker` component (reusable numbered tutorial list)
- `onSelectTutorial` callback prop on all tutorial components

## Acceptance Criteria

- [ ] Completing any tutorial shows numbered list of other tutorials
- [ ] Pressing a number key launches that tutorial without restarting CLI
- [ ] Pressing `q` exits cleanly (no loop)
- [ ] Cleanup (`c`) still works before selecting next tutorial
- [ ] Current tutorial is excluded from the picker list
- [ ] Existing tests still pass

## Tasks

### TC-001: Create TutorialPicker shared component

**Intent:** Create a reusable Ink component that renders a numbered list of
available tutorials, excluding the current one. Pure display — input handling
stays in parent.

**Expected Outcome:** `TutorialPicker` component renders numbered tutorial list
with descriptions. Accepts `currentTopic` to exclude self and `tutorials` array.

**Confidence:** High
**Status:** completed
**Tags:** tui, component
**Files:**
  - apps/anvil-cli/src/tui/commands/tutorial/components/TutorialPicker.tsx

---

### TC-002: Add tutorial continuation to core Tutorial

**Intent:** On the last step (`next-steps`), show the `TutorialPicker` and
handle number key input to select a next tutorial. Add `onSelectTutorial`
callback prop. Update `NextStepsStep` to render numbered list instead of plain
command list.

**Expected Outcome:** Core tutorial completion screen shows numbered tutorial
options. Pressing 1-4 calls `onSelectTutorial` with the topic and exits.

**Confidence:** High
**Status:** completed
**Tags:** tui, tutorial
**Dependencies:** TC-001
**Files:**
  - apps/anvil-cli/src/tui/commands/tutorial/Tutorial.tsx
  - apps/anvil-cli/src/tui/commands/tutorial/steps/NextStepsStep.tsx

---

### TC-003: Add tutorial continuation to feature tutorials

**Intent:** Add `onSelectTutorial` callback and number key handling to the last
step of each feature tutorial (PolicyTutorial, ArchitectureTutorial,
DriftTutorial, CITutorial). Show `TutorialPicker` on last step alongside
existing cleanup options.

**Expected Outcome:** Each feature tutorial shows numbered list of other
tutorials on its final step. Number keys trigger continuation.

**Confidence:** High
**Status:** completed
**Tags:** tui, tutorial
**Dependencies:** TC-001
**Files:**
  - apps/anvil-cli/src/tui/commands/tutorial/features/PolicyTutorial.tsx
  - apps/anvil-cli/src/tui/commands/tutorial/features/ArchitectureTutorial.tsx
  - apps/anvil-cli/src/tui/commands/tutorial/features/DriftTutorial.tsx
  - apps/anvil-cli/src/tui/commands/tutorial/features/CITutorial.tsx

---

### TC-004: Add tutorial loop to command handler

**Intent:** Wrap `renderTUIAndWait` calls in `tutorial.ts` with a loop that
continues rendering tutorials as long as the user selects a next one. Track
selection via `onSelectTutorial` callback.

**Expected Outcome:** User can chain tutorials without restarting CLI. Loop
exits cleanly when user presses `q`.

**Confidence:** High
**Status:** completed
**Tags:** cli, command
**Dependencies:** TC-002, TC-003
**Files:**
  - apps/anvil-cli/src/commands/tutorial.ts

---

### TC-005: Build and test

**Intent:** Verify the feature builds and all existing tests pass.

**Expected Outcome:** `pnpm build` succeeds, `pnpm -F @eddacraft/anvil-cli test -- --run` passes.

**Confidence:** High
**Status:** completed
**Tags:** verification
**Dependencies:** TC-004
