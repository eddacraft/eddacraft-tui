import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import type { WatchEvent } from '../steps/watch-project.js';

// Capture the onEvent callback so tests can simulate file changes.
let capturedOnEvent: ((event: WatchEvent) => void) | null = null;
const mockClose = vi.fn();
let mockReadyResolve: () => void;
let mockReadyReject: (err: Error) => void;
let mockReadyPromise: Promise<void>;

function resetReadyPromise(): void {
  mockReadyPromise = new Promise<void>((resolve, reject) => {
    mockReadyResolve = resolve;
    mockReadyReject = reject;
  });
}

// Mock the watch-project module (separate file from the component)
vi.mock('../steps/watch-project.js', () => ({
  createTutorialWatcher: vi.fn((_workspaceRoot: string, onEvent: (event: WatchEvent) => void) => {
    capturedOnEvent = onEvent;
    return { close: mockClose, ready: mockReadyPromise };
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
import type { ScanResults } from '../types.js';

const mockCreateTutorialWatcher = vi.mocked(createTutorialWatcher);

const SCAN_RESULTS_WITH_WARNINGS: ScanResults = {
  warningCount: 3,
  fileCount: 2,
  executionTimeMs: 150,
  topWarnings: [
    {
      id: 'AP-001',
      title: 'Test warning',
      file: 'src/utils/parser.ts',
      line: 42,
      message: 'Something is wrong',
      suggestion: 'Fix it',
    },
  ],
};

const SCAN_RESULTS_CLEAN: ScanResults = {
  warningCount: 0,
  fileCount: 10,
  executionTimeMs: 100,
  topWarnings: [],
};

describe('WatchStep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedOnEvent = null;
    resetReadyPromise();
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // --- Initialising phase ---

  it('shows Spinner while initialising', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Initialising file watcher');
  });

  it('transitions to watching after ready resolves', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    // Should be initialising
    expect(lastFrame()).toContain('Initialising file watcher');

    // Resolve the ready promise
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Watch mode is ready');
    });
  });

  it('falls through to watching if ready rejects', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    mockReadyReject(new Error('watcher error'));

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Watch mode is ready');
    });
  });

  // --- Watching phase (instructions) ---

  it('shows numbered instructions', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      const frame = lastFrame();
      expect(frame).toContain('1. Open your editor or IDE');
      expect(frame).toContain('2. Edit and save:');
      expect(frame).toContain('3. Come back here');
    });
  });

  it('shows workspace root path', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Watching: /mock/workspace');
    });
  });

  it('shows watched patterns', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      const frame = lastFrame();
      expect(frame).toContain('**/*.ts');
      expect(frame).toContain('**/*.tsx');
    });
  });

  it('suggests specific file when scan results have warnings', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <WatchStep onComplete={onComplete} scanResults={SCAN_RESULTS_WITH_WARNINGS} />
    );
    mockReadyResolve();

    await vi.waitFor(() => {
      const frame = lastFrame();
      expect(frame).toContain('src/utils/parser.ts (line 42)');
      expect(frame).toContain("warning you'll fix in the next step");
    });
  });

  it('shows generic instruction when no warnings', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <WatchStep onComplete={onComplete} scanResults={SCAN_RESULTS_CLEAN} />
    );
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('any .ts or .js file');
    });
  });

  it('shows "Waiting for a file change" spinner', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Waiting for a file change');
    });
  });

  // --- Detection phase ---

  it('shows prominent "Change detected!" with filename', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Watch mode is ready');
    });

    capturedOnEvent!({ type: 'change', path: 'src/utils/parser.ts' });

    await vi.waitFor(() => {
      const frame = lastFrame();
      expect(frame).toContain('Change detected!');
      expect(frame).toContain('src/utils/parser.ts');
    });
  });

  it('shows core feedback loop explanation after detection', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Watch mode is ready');
    });

    capturedOnEvent!({ type: 'change', path: 'src/index.ts' });

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('core feedback loop');
    });
  });

  it('mentions anvil watch for production use', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Watch mode is ready');
    });

    capturedOnEvent!({ type: 'change', path: 'src/index.ts' });

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('anvil watch');
    });
  });

  it('calls onComplete after a file change is detected', async () => {
    const onComplete = vi.fn();
    render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    // Wait for watching phase
    await vi.waitFor(() => {
      expect(capturedOnEvent).not.toBeNull();
    });

    capturedOnEvent!({ type: 'change', path: 'src/app.ts' });

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalledTimes(1);
    });
  });

  it('shows "Press Enter to continue" after detection', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);
    mockReadyResolve();

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Watch mode is ready');
    });

    capturedOnEvent!({ type: 'add', path: 'src/new-file.ts' });

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Press Enter to continue');
    });
  });

  // --- Already-triggered state ---

  it('shows the already-detected state when watchTriggered is true', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} watchTriggered />);

    const frame = lastFrame();
    expect(frame).toContain('Change detected!');
    expect(frame).toContain('Press Enter to continue');
  });

  it('does not start a watcher when watchTriggered is true', () => {
    const onComplete = vi.fn();
    render(<WatchStep onComplete={onComplete} watchTriggered />);

    expect(mockCreateTutorialWatcher).not.toHaveBeenCalled();
  });

  // --- Progressive hints ---

  it('shows first hint at 10s', async () => {
    vi.useFakeTimers();

    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    // Resolve ready synchronously (fake timers)
    mockReadyResolve();
    await vi.advanceTimersByTimeAsync(0);

    // Should not show the hint initially
    expect(lastFrame()).not.toContain('Still waiting');

    // Advance past the 10s mark + flush React state update
    await vi.advanceTimersByTimeAsync(10_000);
    await vi.advanceTimersByTimeAsync(0);

    expect(lastFrame()).toContain('Still waiting');
    expect(lastFrame()).toContain('make sure you save the file');
  });

  it('shows second hint at 20s', async () => {
    vi.useFakeTimers();

    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    mockReadyResolve();
    await vi.advanceTimersByTimeAsync(0);

    // Step through in increments so each timer's state update flushes
    await vi.advanceTimersByTimeAsync(10_000);
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(10_000);
    await vi.advanceTimersByTimeAsync(0);

    expect(lastFrame()).toContain("Make sure you're editing a file inside");
  });

  it('shows skip option at 30s', async () => {
    vi.useFakeTimers();

    const onComplete = vi.fn();
    const { lastFrame } = render(<WatchStep onComplete={onComplete} />);

    mockReadyResolve();
    await vi.advanceTimersByTimeAsync(0);

    // Step through each hint tier with multiple flush cycles
    // (Spinner interval timers compete for state update batching)
    for (let i = 0; i < 3; i++) {
      await vi.advanceTimersByTimeAsync(10_000);
      await vi.advanceTimersByTimeAsync(0);
      await vi.advanceTimersByTimeAsync(0);
    }

    expect(lastFrame()).toContain('skip this step');
  });

  // --- Cleanup ---

  it('cleans up the watcher on unmount', () => {
    const onComplete = vi.fn();
    const { unmount } = render(<WatchStep onComplete={onComplete} />);

    unmount();

    expect(mockClose).toHaveBeenCalled();
  });
});
