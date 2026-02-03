import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import type { WatchEvent } from '../steps/watch-project.js';

// Capture the onEvent callback so tests can simulate file changes.
let capturedOnEvent: ((event: WatchEvent) => void) | null = null;
const mockClose = vi.fn();

// Mock the watch-project module (separate file from the component)
vi.mock('../steps/watch-project.js', () => ({
  createTutorialWatcher: vi.fn((_workspaceRoot: string, onEvent: (event: WatchEvent) => void) => {
    capturedOnEvent = onEvent;
    return { close: mockClose };
  }),
  WATCHED_PATTERNS: ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.jsx'],
}));

// Mock getWorkspaceRoot
vi.mock('../../../../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

// Import after mocks are set up
import { WatchStep } from '../steps/WatchStep.js';
import { createTutorialWatcher } from '../steps/watch-project.js';

const mockCreateTutorialWatcher = vi.mocked(createTutorialWatcher);

describe('WatchStep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedOnEvent = null;
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows "Watch mode is active" initially', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Watch mode is active');
  });

  it('shows the component title', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Watch Mode');
  });

  it('shows watched patterns', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    const frame = lastFrame();
    expect(frame).toContain('**/*.ts');
    expect(frame).toContain('**/*.tsx');
    expect(frame).toContain('**/*.js');
    expect(frame).toContain('**/*.jsx');
  });

  it('shows instruction to edit a file', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Edit any source file and save to see Anvil in action');
  });

  it('shows "Change detected" with filename when a file change fires', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    // Simulate a file change event from the watcher
    expect(capturedOnEvent).not.toBeNull();
    capturedOnEvent!({ type: 'change', path: 'src/utils/parser.ts' });

    await vi.waitFor(() => {
      const frame = lastFrame();
      expect(frame).toContain('Change detected: src/utils/parser.ts');
    });
  });

  it('shows "Anvil validated your change" after detection', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    capturedOnEvent!({ type: 'change', path: 'src/index.ts' });

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Anvil validated your change');
    });
  });

  it('calls onComplete after a file change is detected', async () => {
    const onComplete = vi.fn();
    render(<WatchStep onComplete={onComplete} />);

    capturedOnEvent!({ type: 'change', path: 'src/app.ts' });

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalledTimes(1);
    });
  });

  it('shows the already-detected state when watchTriggered is true', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} watchTriggered />);

    const frame = lastFrame();
    expect(frame).toContain('Watch mode detected a change!');
    expect(frame).toContain('Press Enter to continue');
  });

  it('does not start a watcher when watchTriggered is true', () => {
    const onComplete = vi.fn();
    render(<WatchStep onComplete={onComplete} watchTriggered />);

    expect(mockCreateTutorialWatcher).not.toHaveBeenCalled();
  });

  it('shows 30s hint after timeout with no change', async () => {
    vi.useFakeTimers();

    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    // Should not show the hint initially
    expect(lastFrame()).not.toContain('Tip:');

    // Advance past the 30s mark
    vi.advanceTimersByTime(30_000);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain(
        'Tip: Try editing any .ts file in your project and pressing save'
      );
    });
  });

  it('cleans up the watcher on unmount', () => {
    const onComplete = vi.fn();
    const { unmount } = render(<WatchStep onComplete={onComplete} />);

    unmount();

    expect(mockClose).toHaveBeenCalled();
  });

  it('shows "Press Enter to continue" after detection', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    capturedOnEvent!({ type: 'add', path: 'src/new-file.ts' });

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Press Enter to continue');
    });
  });
});
