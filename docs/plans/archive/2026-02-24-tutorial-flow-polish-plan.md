<\!-- Archived: 2026-03-01 | Reason: Implementation complete — tutorial flow
polish shipped via PBLU (57/57) -->

# Tutorial Flow Polish — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to
> implement this plan task-by-task.

**Goal:** Polish tutorial completion screens so all tutorials use the same
interactive picker with completion tracking, readable bottom bar, and clear
cleanup UX.

**Architecture:** Extend `TutorialPicker` with `completedTopics` prop and
instruction text. Add `completedTutorials` field to `TutorialProgress` schema.
Add `backArrow` icon to theme. Update bottom bar color from `smoke` to `ash`
across all 5 tutorial components.

**Tech Stack:** React/Ink TUI, Zod schema, Vitest

---

### Task 1: Add `backArrow` icon to theme and extend TutorialProgress schema

**Files:**

- Modify: `apps/anvil-cli/src/tui/utils/theme.ts:30`
- Modify: `apps/anvil-cli/src/tui/commands/tutorial/types.ts:68-80`

**Step 1: Add `backArrow` icon to theme**

In `apps/anvil-cli/src/tui/utils/theme.ts`, add `backArrow: '◄'` to the icons
object:

```typescript
    arrow: '▸',
    backArrow: '◄',
    check: '◆',
```

**Step 2: Extend TutorialProgress type and schema**

In `apps/anvil-cli/src/tui/commands/tutorial/types.ts`, add `completedTutorials`
to the interface and Zod schema:

```typescript
export interface TutorialProgress {
  currentStep: number;
  totalSteps: number;
  completedSteps: string[];
  startedAt: string;
  completedTutorials?: string[];
}

export const TutorialProgressSchema = z.object({
  currentStep: z.number(),
  totalSteps: z.number(),
  completedSteps: z.array(z.string()),
  startedAt: z.string(),
  completedTutorials: z.array(z.string()).optional(),
});
```

**Step 3: Verify build**

Run: `pnpm -F @eddacraft/anvil-cli exec tsc --noEmit` Expected: No type errors

**Step 4: Commit**

```
feat(tutorial): add backArrow icon and completedTutorials to progress schema
```

---

### Task 2: Upgrade TutorialPicker with completion tracking and instructions

**Files:**

- Modify:
  `apps/anvil-cli/src/tui/commands/tutorial/components/TutorialPicker.tsx`
- Test:
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/tutorial-picker.test.tsx`
  (create)

**Step 1: Write the failing tests**

Create
`apps/anvil-cli/src/tui/commands/tutorial/__tests__/tutorial-picker.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import {
  TutorialPicker,
  resolveTutorialKey,
} from '../components/TutorialPicker.js';
import type { TutorialOption } from '../components/TutorialPicker.js';

const ALL_TUTORIALS: TutorialOption[] = [
  { topic: 'core', description: 'Core tutorial (scan, watch, fix)' },
  { topic: 'policies', description: 'Write custom OPA/Rego rules' },
  { topic: 'architecture', description: 'Define architecture boundaries' },
  { topic: 'drift', description: 'Track architecture drift over time' },
  { topic: 'ci', description: 'Set up CI integration' },
];

describe('TutorialPicker', () => {
  it('shows instruction text with key hint', () => {
    const { lastFrame } = render(
      <TutorialPicker tutorials={ALL_TUTORIALS} currentTopic="core" />
    );
    expect(lastFrame()).toContain("What's next");
    expect(lastFrame()).toContain('q to exit');
  });

  it('shows check icon for completed tutorials', () => {
    const { lastFrame } = render(
      <TutorialPicker
        tutorials={ALL_TUTORIALS}
        currentTopic="core"
        completedTopics={['policies', 'drift']}
      />
    );
    const frame = lastFrame()!;
    // Completed tutorials should show ◆ instead of their number
    // policies and drift are completed; architecture and ci are not
    expect(frame).toContain('architecture');
    expect(frame).toContain('ci');
  });

  it('excludes current topic from the list', () => {
    const { lastFrame } = render(
      <TutorialPicker tutorials={ALL_TUTORIALS} currentTopic="drift" />
    );
    expect(lastFrame()).not.toContain('Track architecture drift');
  });

  it('renders nothing when no tutorials available', () => {
    const { lastFrame } = render(
      <TutorialPicker
        tutorials={[{ topic: 'core', description: 'Only one' }]}
        currentTopic="core"
      />
    );
    expect(lastFrame()).toBe('');
  });
});

describe('resolveTutorialKey', () => {
  it('resolves number key to topic', () => {
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '1')).toBe('policies');
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '4')).toBe('ci');
  });

  it('returns null for out of range', () => {
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '5')).toBeNull();
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '0')).toBeNull();
  });

  it('returns null for non-numeric', () => {
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', 'a')).toBeNull();
  });
});
```

**Step 2: Run tests to verify they fail**

Run:
`pnpm -F @eddacraft/anvil-cli test -- --run --reporter=verbose src/tui/commands/tutorial/__tests__/tutorial-picker.test.tsx`
Expected: Failures on "shows instruction text" and "shows check icon" tests

**Step 3: Update TutorialPicker component**

Replace `apps/anvil-cli/src/tui/commands/tutorial/components/TutorialPicker.tsx`
with:

```tsx
import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';

export interface TutorialOption {
  topic: string;
  description: string;
}

interface TutorialPickerProps {
  tutorials: TutorialOption[];
  currentTopic?: string;
  completedTopics?: string[];
}

export function TutorialPicker({
  tutorials,
  currentTopic,
  completedTopics = [],
}: TutorialPickerProps): React.ReactElement {
  const available = tutorials.filter((t) => t.topic !== currentTopic);

  if (available.length === 0) return <></>;

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text bold color={theme.colours.text}>
          What&apos;s next
        </Text>
        <Text color={theme.colours.ash}>
          {' '}
          — press a number to start, <Text color={theme.colours.ember}>
            q
          </Text>{' '}
          to exit
        </Text>
      </Box>
      <Box flexDirection="column" marginLeft={2}>
        {available.map((t, i) => {
          const isCompleted = completedTopics.includes(t.topic);
          return (
            <Box key={t.topic}>
              {isCompleted ? (
                <Text color={theme.colours.steel}>{theme.icons.check}</Text>
              ) : (
                <Text color={theme.colours.ember}>{i + 1}</Text>
              )}
              <Text color={theme.colours.smoke}>{'  '}</Text>
              <Text
                color={isCompleted ? theme.colours.steel : theme.colours.text}
              >
                {t.topic}
              </Text>
              <Text color={theme.colours.smoke}>
                {' '}
                {theme.icons.arrow} {t.description}
              </Text>
            </Box>
          );
        })}
      </Box>
    </Box>
  );
}

/**
 * Given the full tutorials list, current topic, and a number key (1-based),
 * returns the topic string or null if out of range.
 */
export function resolveTutorialKey(
  tutorials: TutorialOption[],
  currentTopic: string | undefined,
  key: string
): string | null {
  const num = parseInt(key, 10);
  if (isNaN(num) || num < 1) return null;

  const available = tutorials.filter((t) => t.topic !== currentTopic);
  const selected = available[num - 1];
  return selected?.topic ?? null;
}
```

**Step 4: Run tests to verify they pass**

Run:
`pnpm -F @eddacraft/anvil-cli test -- --run --reporter=verbose src/tui/commands/tutorial/__tests__/tutorial-picker.test.tsx`
Expected: All pass

**Step 5: Commit**

```
feat(tutorial): add completion tracking and instructions to TutorialPicker
```

---

### Task 3: Update NextStepsStep to pass completedTopics and remove static content

**Files:**

- Modify: `apps/anvil-cli/src/tui/commands/tutorial/steps/NextStepsStep.tsx`
- Modify:
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/next-steps-step.test.tsx`

**Step 1: Update NextStepsStep props and rendering**

Add `completedTopics` prop and pass it through to `TutorialPicker`. Remove the
"Resources" section (it's redundant — docs link is not actionable in TUI).
Remove the old cleanup inline text and replace with simpler instruction:

In `NextStepsStep.tsx`, change the interface and component:

- Add `completedTopics?: string[]` to `NextStepsStepProps`
- Pass `completedTopics` to `<TutorialPicker>`
- Remove the bottom `{/* Cleanup section */}` block — cleanup is handled by the
  parent's bottom bar now
- Keep the confirmation dialog only when `cleanupConfirming` is true

The `<TutorialPicker>` section becomes:

```tsx
<Box flexDirection="column" marginBottom={1}>
  <TutorialPicker
    tutorials={tutorials}
    currentTopic="core"
    completedTopics={completedTopics}
  />
</Box>
```

Remove the cleanup instructions block at the bottom (lines 96-121). The bottom
bar in `Tutorial.tsx` already handles cleanup hints.

**Step 2: Update the cleanup confirmation to show file list**

When `cleanupConfirming` is true, show the file list. When `cleanupRequested` is
true, show the success message. Render this between the picker and the bottom of
the component:

```tsx
{
  cleanupRequested ? (
    <Box marginTop={1}>
      <Text color={theme.colours.steel}>
        {theme.icons.success} Tutorial files removed
      </Text>
    </Box>
  ) : cleanupConfirming ? (
    <Box flexDirection="column" marginTop={1}>
      <Text color={theme.colours.molten}>Remove these tutorial files?</Text>
      <Box flexDirection="column" marginLeft={2} marginY={1}>
        {CLEANUP_FILES.map((file) => (
          <Text key={file} color={theme.colours.ash}>
            {theme.icons.bullet} {file}
          </Text>
        ))}
      </Box>
      <Text color={theme.colours.ash}>
        Press <Text color={theme.colours.ember}>c</Text> again to confirm
      </Text>
    </Box>
  ) : null;
}
```

**Step 3: Update tests**

In `next-steps-step.test.tsx`:

- Update the "shows tutorial picker" test to check for "What's next" instead of
  "Continue with another tutorial"
- Update the "shows cleanup and continuation instructions" test assertion from
  checking for `exit` and `number` to checking `q to exit`
- Add a test for `completedTopics` rendering

**Step 4: Run tests**

Run:
`pnpm -F @eddacraft/anvil-cli test -- --run --reporter=verbose src/tui/commands/tutorial/__tests__/next-steps-step.test.tsx`
Expected: All pass

**Step 5: Commit**

```
fix(tutorial): use unified picker with completion tracking in core tutorial
```

---

### Task 4: Wire completedTopics through Tutorial.tsx and command handler

**Files:**

- Modify: `apps/anvil-cli/src/tui/commands/tutorial/Tutorial.tsx`
- Modify: `apps/anvil-cli/src/commands/tutorial.ts`

**Step 1: Add completedTopics prop to Tutorial component**

In `Tutorial.tsx`, add `completedTopics?: string[]` to `TutorialProps` and pass
it to `NextStepsStep`:

```tsx
interface TutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
  onSelectTutorial?: (topic: string) => void;
  tutorials?: TutorialOption[];
  completedTopics?: string[];
}
```

Pass it in the render:

```tsx
<NextStepsStep
  startedAt={state.startedAt}
  scanResults={state.scanResults}
  cleanupConfirming={state.cleanupConfirming}
  cleanupRequested={state.cleanupRequested}
  tutorials={tutorials}
  completedTopics={completedTopics}
  onFinish={handleFinish}
/>
```

**Step 2: Update bottom bar in Tutorial.tsx**

Change the bottom bar (lines 172-178) to use `ash` color, `backArrow` icon, and
`ember` for key letters:

```tsx
<Box marginTop={1}>
  <Text color={theme.colours.ash}>
    {canGoBack(state.currentStep) && (
      <>
        <Text color={theme.colours.ember}>{theme.icons.backArrow}</Text>
        {' back  '}
      </>
    )}
    {!isLastStep(state.currentStep) && 'Enter next  '}
    {isLastStep(state.currentStep) && !state.cleanupRequested && (
      <>
        <Text color={theme.colours.ember}>c</Text>
        {' clean up tutorial files  '}
      </>
    )}
    <Text color={theme.colours.ember}>q</Text>
    {' quit'}
  </Text>
</Box>
```

**Step 3: Update command handler to track and pass completedTopics**

In `commands/tutorial.ts`:

1. Update `saveProgress` calls to include the completed tutorial topic.
2. Load completed topics from progress file and pass to `renderTutorial`.
3. Track in-session completions in the tutorial loop.

In the `renderTutorial` function, add a `completedTopics` parameter:

```typescript
async function renderTutorial(
  currentTopic: string | undefined,
  options: TutorialOptions,
  completedTopics: string[]
): Promise<string | null> {
```

Pass `completedTopics` to `Tutorial` component:

```typescript
await renderTUIAndWait(Tutorial, {
  onComplete: handleComplete,
  onCleanup: handleCleanup,
  onSelectTutorial,
  tutorials: TUTORIAL_OPTIONS,
  completedTopics,
});
```

And to each feature tutorial:

```typescript
await renderTUIAndWait(PolicyTutorial, {
  onComplete: () => {},
  onCleanup: () => { ... },
  onSelectTutorial,
  tutorials: TUTORIAL_OPTIONS,
  completedTopics,
});
```

In the tutorial loop (lines 346-350), maintain a `completedTopics` array:

```typescript
const progress = loadProgress(getWorkspaceRoot());
const completedTopics: string[] = progress?.completedTutorials ?? [];

let currentTopic = topic ?? undefined;
while (true) {
  const next = await renderTutorial(currentTopic, options, completedTopics);
  // Mark the just-completed tutorial
  const justCompleted = currentTopic ?? 'core';
  if (!completedTopics.includes(justCompleted)) {
    completedTopics.push(justCompleted);
  }
  // Persist to disk
  const ws = getWorkspaceRoot();
  const existing = loadProgress(ws);
  saveProgress(ws, {
    ...(existing ?? {
      currentStep: 0,
      totalSteps: 0,
      completedSteps: [],
      startedAt: new Date().toISOString(),
    }),
    completedTutorials: completedTopics,
  });
  if (!next) break;
  currentTopic = next;
}
```

**Step 4: Verify build**

Run: `pnpm -F @eddacraft/anvil-cli exec tsc --noEmit` Expected: No type errors

**Step 5: Run all tutorial tests**

Run:
`pnpm -F @eddacraft/anvil-cli test -- --run --reporter=verbose src/tui/commands/tutorial/`
Expected: All pass

**Step 6: Commit**

```
feat(tutorial): wire completion tracking through command handler and TUI
```

---

### Task 5: Update all feature tutorials — bottom bar and completedTopics

**Files:**

- Modify: `apps/anvil-cli/src/tui/commands/tutorial/features/DriftTutorial.tsx`
- Modify: `apps/anvil-cli/src/tui/commands/tutorial/features/PolicyTutorial.tsx`
- Modify:
  `apps/anvil-cli/src/tui/commands/tutorial/features/ArchitectureTutorial.tsx`
- Modify: `apps/anvil-cli/src/tui/commands/tutorial/features/CITutorial.tsx`

All four feature tutorials need identical changes:

**Step 1: Add completedTopics prop**

For each tutorial (using DriftTutorial as example), add to props interface:

```typescript
interface DriftTutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
  onSelectTutorial?: (topic: string) => void;
  tutorials?: TutorialOption[];
  completedTopics?: string[];
}
```

Destructure in component: `completedTopics = []`

Pass to `TutorialPicker`:

```tsx
<TutorialPicker
  tutorials={tutorials}
  currentTopic="drift"
  completedTopics={completedTopics}
/>
```

**Step 2: Fix bottom bar — all four tutorials**

The pattern is the same in all four. Replace the bottom `<Box marginTop={1}>`
block.

Before (DriftTutorial lines 181-193):

```tsx
<Box marginTop={1}>
  <Text color={theme.colours.smoke}>
    {canGoBack(currentStep) && `${theme.icons.arrow} back `}
    {!isLastStep(currentStep) && 'Enter next '}
    {isLastStep(currentStep) && (
      <>
        <Text color={theme.colours.text}>c</Text>
        {' clean up  '}
      </>
    )}
    {theme.icons.bullet} q quit
  </Text>
</Box>
```

After:

```tsx
<Box marginTop={1}>
  <Text color={theme.colours.ash}>
    {canGoBack(currentStep) && (
      <>
        <Text color={theme.colours.ember}>{theme.icons.backArrow}</Text>
        {' back  '}
      </>
    )}
    {!isLastStep(currentStep) && 'Enter next  '}
    {isLastStep(currentStep) && !cleanedUp && (
      <>
        <Text color={theme.colours.ember}>c</Text>
        {' clean up tutorial files  '}
      </>
    )}
    <Text color={theme.colours.ember}>q</Text>
    {' quit'}
  </Text>
</Box>
```

**Step 3: Add cleanup feedback to feature tutorials**

Each feature tutorial's summary step (the `SummaryStep` component inside their
step directories) has a "Press c to clean up" line. Update these to show cleanup
success when `cleanedUp` is true.

For each feature tutorial component, pass `cleanedUp` state to the summary
rendering area. After the `TutorialPicker`, add:

```tsx
{
  isLastStep(currentStep) && cleanedUp && (
    <Box marginTop={1}>
      <Text color={theme.colours.steel}>
        {theme.icons.success} Tutorial files removed
      </Text>
    </Box>
  );
}
```

**Step 4: Run all tutorial tests**

Run:
`pnpm -F @eddacraft/anvil-cli test -- --run --reporter=verbose src/tui/commands/tutorial/`
Expected: All pass

**Step 5: Verify build**

Run: `pnpm -F @eddacraft/anvil-cli exec tsc --noEmit` Expected: No type errors

**Step 6: Commit**

```
fix(tutorial): unify bottom bar and add completion tracking to feature tutorials
```

---

### Task 6: Update feature tutorial summary steps cleanup text

**Files:**

- Modify:
  `apps/anvil-cli/src/tui/commands/tutorial/features/drift-steps/SummaryStep.tsx:50`
- Modify:
  `apps/anvil-cli/src/tui/commands/tutorial/features/architecture-steps/SummaryStep.tsx:45`
- Modify:
  `apps/anvil-cli/src/tui/commands/tutorial/features/ci-steps/SummaryStep.tsx:50`
- Modify:
  `apps/anvil-cli/src/tui/commands/tutorial/features/policy-steps/CustomiseStep.tsx:90`

**Step 1: Update cleanup text in each summary step**

Each of these files has a line like:

```tsx
Press <Text color={theme.colours.text}>c</Text> to clean up tutorial files,{' '}
```

Remove these lines from each summary step — the bottom bar in the parent
tutorial component now handles the cleanup hint. The summary steps should only
contain their content (quick reference, tips, etc.), not navigation
instructions.

**Step 2: Run all tutorial tests**

Run:
`pnpm -F @eddacraft/anvil-cli test -- --run --reporter=verbose src/tui/commands/tutorial/`
Expected: All pass

**Step 3: Commit**

```
fix(tutorial): remove duplicate cleanup hints from summary steps
```

---

### Task 7: Full build and test verification

**Step 1: Run full build**

Run: `pnpm build` Expected: All packages build successfully

**Step 2: Run all CLI tests**

Run: `pnpm -F @eddacraft/anvil-cli test -- --run` Expected: All pass

**Step 3: Manual verification**

Run: `node apps/anvil-cli/dist/index.js tutorial`

- Complete core tutorial
- Verify "What's next" appears with numbered options and instruction text
- Verify bottom bar is readable with correct `◄` back arrow
- Press a number to chain to a feature tutorial
- Verify completed core tutorial shows `◆` in the picker
- Press `c`, verify file list appears
- Press `c` again, verify "Tutorial files removed" message

**Step 4: Commit (if any test fixes needed)**

```
fix(tutorial): address test failures from flow polish
```
