import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import { TutorialPicker, resolveTutorialKey } from '../components/TutorialPicker.js';
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

describe('resolveTutorialKey', () => {
  it('maps number keys to the correct topic', () => {
    expect(resolveTutorialKey(TUTORIALS, 'core', '1')).toBe('policies');
    expect(resolveTutorialKey(TUTORIALS, 'core', '2')).toBe('architecture');
    expect(resolveTutorialKey(TUTORIALS, 'core', '3')).toBe('drift');
    expect(resolveTutorialKey(TUTORIALS, 'core', '4')).toBe('ci');
  });

  it('returns null for out-of-range keys', () => {
    expect(resolveTutorialKey(TUTORIALS, 'core', '5')).toBeNull();
    expect(resolveTutorialKey(TUTORIALS, 'core', '99')).toBeNull();
  });

  it('returns null for zero', () => {
    expect(resolveTutorialKey(TUTORIALS, 'core', '0')).toBeNull();
  });

  it('returns null for negative numbers', () => {
    expect(resolveTutorialKey(TUTORIALS, 'core', '-1')).toBeNull();
  });

  it('returns null for non-numeric input', () => {
    expect(resolveTutorialKey(TUTORIALS, 'core', 'a')).toBeNull();
    expect(resolveTutorialKey(TUTORIALS, 'core', '')).toBeNull();
    expect(resolveTutorialKey(TUTORIALS, 'core', 'abc')).toBeNull();
  });

  it('excludes currentTopic from the available list', () => {
    // With 'core' excluded, index 1 = 'policies'
    expect(resolveTutorialKey(TUTORIALS, 'core', '1')).toBe('policies');

    // With 'policies' excluded, index 1 = 'core'
    expect(resolveTutorialKey(TUTORIALS, 'policies', '1')).toBe('core');
    expect(resolveTutorialKey(TUTORIALS, 'policies', '2')).toBe('architecture');
  });

  it('handles undefined currentTopic (no exclusion)', () => {
    expect(resolveTutorialKey(TUTORIALS, undefined, '1')).toBe('core');
    expect(resolveTutorialKey(TUTORIALS, undefined, '5')).toBe('ci');
  });

  it('returns null when all tutorials are filtered out', () => {
    const single: TutorialOption[] = [{ topic: 'only', description: 'Only one' }];
    expect(resolveTutorialKey(single, 'only', '1')).toBeNull();
  });
});

describe('TutorialPicker', () => {
  it('displays tutorials excluding the current one', () => {
    const { lastFrame } = render(<TutorialPicker tutorials={TUTORIALS} currentTopic="core" />);

    const frame = lastFrame();
    expect(frame).toContain('Continue with another tutorial');
    expect(frame).toContain('policies');
    expect(frame).toContain('architecture');
    expect(frame).toContain('drift');
    expect(frame).toContain('ci');
    expect(frame).not.toContain('Core tutorial');
  });

  it('shows numbered indices starting from 1', () => {
    const { lastFrame } = render(<TutorialPicker tutorials={TUTORIALS} currentTopic="core" />);

    const frame = lastFrame();
    expect(frame).toContain('1');
    expect(frame).toContain('2');
    expect(frame).toContain('3');
    expect(frame).toContain('4');
  });

  it('shows descriptions for each tutorial', () => {
    const { lastFrame } = render(<TutorialPicker tutorials={TUTORIALS} currentTopic="core" />);

    const frame = lastFrame();
    expect(frame).toContain('Write custom OPA/Rego rules');
    expect(frame).toContain('Define architecture boundaries');
    expect(frame).toContain('Track architecture drift over time');
    expect(frame).toContain('Set up CI integration');
  });

  it('returns empty fragment when no tutorials are available', () => {
    const { lastFrame } = render(
      <TutorialPicker
        tutorials={[{ topic: 'only', description: 'Only one' }]}
        currentTopic="only"
      />
    );

    expect(lastFrame()).toBe('');
  });

  it('returns empty fragment for empty tutorials list', () => {
    const { lastFrame } = render(<TutorialPicker tutorials={[]} />);

    expect(lastFrame()).toBe('');
  });

  it('shows all tutorials when currentTopic is not set', () => {
    const { lastFrame } = render(<TutorialPicker tutorials={TUTORIALS} />);

    const frame = lastFrame();
    expect(frame).toContain('core');
    expect(frame).toContain('policies');
    expect(frame).toContain('architecture');
    expect(frame).toContain('drift');
    expect(frame).toContain('ci');
  });
});

describe('Tutorial continuation key handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  async function advanceToLastStep(
    stdin: { write: (s: string) => void },
    lastFrame: () => string | undefined
  ) {
    // Advance scan -> watch
    stdin.write('\r');
    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 4');
    });
    // Advance watch -> fix
    stdin.write('\r');
    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 3 of 4');
    });
    // Advance fix -> next-steps
    stdin.write('\r');
    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 4 of 4');
    });
  }

  it('calls onSelectTutorial with correct topic when valid number key is pressed on last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin, lastFrame } = render(
      <Tutorial tutorials={TUTORIALS} onSelectTutorial={onSelectTutorial} />
    );

    await advanceToLastStep(stdin, lastFrame);

    stdin.write('1');

    await vi.waitFor(() => {
      expect(onSelectTutorial).toHaveBeenCalledWith('policies');
    });
  });

  it('does not call onSelectTutorial for invalid number keys on last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin, lastFrame } = render(
      <Tutorial tutorials={TUTORIALS} onSelectTutorial={onSelectTutorial} />
    );

    await advanceToLastStep(stdin, lastFrame);

    stdin.write('9');

    await new Promise((r) => setTimeout(r, 50));
    expect(onSelectTutorial).not.toHaveBeenCalled();
  });

  it('does not call onSelectTutorial for non-numeric keys on last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin, lastFrame } = render(
      <Tutorial tutorials={TUTORIALS} onSelectTutorial={onSelectTutorial} />
    );

    await advanceToLastStep(stdin, lastFrame);

    stdin.write('x');

    await new Promise((r) => setTimeout(r, 50));
    expect(onSelectTutorial).not.toHaveBeenCalled();
  });

  it('does not trigger onSelectTutorial before reaching last step', async () => {
    const onSelectTutorial = vi.fn();
    const { stdin } = render(
      <Tutorial tutorials={TUTORIALS} onSelectTutorial={onSelectTutorial} />
    );

    // Still on step 1 (scan), press a number key
    stdin.write('1');

    await new Promise((r) => setTimeout(r, 50));
    expect(onSelectTutorial).not.toHaveBeenCalled();
  });
});
