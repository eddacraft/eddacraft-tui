import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import type { TutorialOption } from '../components/TutorialPicker.js';

// Mock scan-project to prevent real filesystem access
vi.mock('../steps/scan-project.js', () => ({
  scanProject: vi.fn(() => new Promise(() => {})),
}));

// Mock watch-project to prevent real filesystem watchers
vi.mock('../steps/watch-project.js', () => ({
  createTutorialWatcher: vi.fn(() => ({ close: vi.fn(), ready: Promise.resolve() })),
  WATCHED_PATTERNS: ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.jsx'],
}));

// Mock getWorkspaceRoot
vi.mock('../../../../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

// Import Tutorial after mocks
import { Tutorial } from '../Tutorial.js';

const TUTORIALS: TutorialOption[] = [
  { topic: 'core', description: 'Core tutorial (scan, watch, fix)' },
  { topic: 'policies', description: 'Write custom OPA/Rego rules' },
  { topic: 'architecture', description: 'Define architecture boundaries' },
  { topic: 'drift', description: 'Track architecture drift over time' },
  { topic: 'ci', description: 'Set up CI integration' },
];

// resolveTutorialKey and TutorialPicker component tests are in tutorial-picker.test.tsx

describe('Tutorial continuation key handling', { timeout: 10000 }, () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('calls onSelectTutorial with correct topic when valid number key is pressed on last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin } = render(
      <Tutorial
        initialStep="next-steps"
        tutorials={TUTORIALS}
        onSelectTutorial={onSelectTutorial}
      />
    );

    stdin.write('1');

    await vi.waitFor(() => {
      expect(onSelectTutorial).toHaveBeenCalledWith('policies');
    });
  });

  it('does not call onSelectTutorial for invalid number keys on last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin } = render(
      <Tutorial
        initialStep="next-steps"
        tutorials={TUTORIALS}
        onSelectTutorial={onSelectTutorial}
      />
    );

    stdin.write('9');

    await new Promise((r) => setTimeout(r, 0));
    expect(onSelectTutorial).not.toHaveBeenCalled();
  });

  it('does not call onSelectTutorial for non-numeric keys on last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin } = render(
      <Tutorial
        initialStep="next-steps"
        tutorials={TUTORIALS}
        onSelectTutorial={onSelectTutorial}
      />
    );

    stdin.write('x');

    await new Promise((r) => setTimeout(r, 0));
    expect(onSelectTutorial).not.toHaveBeenCalled();
  });

  it('does not trigger onSelectTutorial before reaching last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin } = render(
      <Tutorial tutorials={TUTORIALS} onSelectTutorial={onSelectTutorial} />
    );

    // Still on step 1 (scan), press a number key
    stdin.write('1');

    await new Promise((r) => setTimeout(r, 0));
    expect(onSelectTutorial).not.toHaveBeenCalled();
  });
});
