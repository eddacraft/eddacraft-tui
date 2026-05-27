<\!-- Archived: 2026-03-01 | Reason: Problem addressed and implementation
complete via PBLU -->

# Tutorial Flow Polish

**Date:** 2026-02-24 **Branch:** fix/tutorial-flow **Status:** Approved

## Problem

After the tutorial overhaul (TUT-001–012) and path continuation (TC-001–005),
the tutorial completion screens have several UX gaps:

1. **Core tutorial shows static text, not interactive picker.** NextStepsStep
   lists `anvil tutorial policies` etc. as plain text. Users must exit and
   re-run to try another tutorial.
2. **Feature tutorial picker is unlabeled.** The numbered list appears but
   nothing tells the user to press a number key.
3. **No completion tracking.** Chaining through tutorials gives no indication of
   which ones are already done.
4. **Bottom bar is illegible.** Text uses `smoke` color (#475569) on dark
   backgrounds. The back arrow `►` faces the wrong direction.
5. **Cleanup lacks explanation and feedback.** "clean up" doesn't say what it
   removes. No confirmation message after files are deleted.

## Design

### 1. Unified TutorialPicker on all completion screens

Replace the static "Explore further" command list in `NextStepsStep.tsx` with
the shared `TutorialPicker` component. Add instruction text to TutorialPicker:
"Press 1-N to start, or q to exit". Wire number key handling in `Tutorial.tsx`
for the final step (same pattern feature tutorials already use).

### 2. Completion tracking in the picker

Read completed tutorials from `.anvil/tutorial-progress.json` (infrastructure
already exists in `commands/tutorial.ts`). Pass `completedTopics: string[]` prop
to `TutorialPicker`. Completed tutorials show the theme check icon (`◆`) instead
of their number in `steel` color. Completed tutorials remain selectable.
In-session completions update the list immediately when chaining.

Display example:

```
What's next -- press a number to start, q to exit

  ◆  core ▸ Core tutorial (scan, watch, fix)
  2  policies ▸ Write custom OPA/Rego rules
  ◆  architecture ▸ Define architecture boundaries
  4  ci ▸ Set up CI integration
```

### 3. Bottom bar fixes

Fix back arrow from `►` to `◄`. Bump text color from `smoke` (#475569) to `ash`
(#94a3b8). Keep key letters (`c`, `q`) in `ember` for visibility.

### 4. Cleanup UX improvements

Change bottom bar label to "clean up tutorial files". On first press of `c`,
show file list being removed. On confirm, show success message
`◆ Tutorial files removed` in `steel` color. Feature tutorials list their
specific cleanup files (e.g. `.anvil/policies/max_file_length.rego`).

## Files to modify

- `apps/anvil-cli/src/tui/commands/tutorial/components/TutorialPicker.tsx`
- `apps/anvil-cli/src/tui/commands/tutorial/steps/NextStepsStep.tsx`
- `apps/anvil-cli/src/tui/commands/tutorial/Tutorial.tsx`
- `apps/anvil-cli/src/tui/commands/tutorial/features/DriftTutorial.tsx`
- `apps/anvil-cli/src/tui/commands/tutorial/features/PolicyTutorial.tsx`
- `apps/anvil-cli/src/tui/commands/tutorial/features/ArchitectureTutorial.tsx`
- `apps/anvil-cli/src/tui/commands/tutorial/features/CITutorial.tsx`
- `apps/anvil-cli/src/commands/tutorial.ts`
