import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import type { WatchEvent } from '../steps/watch-project.js';
import type { ScanResults } from '../types.js';

// Capture the onEvent callback so tests can simulate file changes.
let capturedOnEvent: ((event: WatchEvent) => void) | null = null;
const mockClose = vi.fn();

// Mock the watch-project module
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
import { FixStep } from '../steps/FixStep.js';
import { createTutorialWatcher } from '../steps/watch-project.js';

const mockCreateTutorialWatcher = vi.mocked(createTutorialWatcher);

const mockResultsWithWarnings: ScanResults = {
  warningCount: 3,
  fileCount: 2,
  executionTimeMs: 120,
  topWarnings: [
    {
      id: 'AP-003',
      title: "Explicit 'any' type",
      file: 'src/utils/parser.ts',
      line: 42,
      message: "Using 'any' defeats type safety. Consider using 'unknown' or a specific type.",
      suggestion: "Replace 'any' with a proper type definition or use 'unknown'",
    },
    {
      id: 'AP-001',
      title: 'Broad eslint-disable added',
      file: 'src/api/handler.ts',
      line: 12,
      message: 'eslint-disable without specific rule',
      suggestion: 'Specify the exact rule to disable instead of disabling all rules',
    },
  ],
};

const mockResultsClean: ScanResults = {
  warningCount: 0,
  fileCount: 0,
  executionTimeMs: 89,
  topWarnings: [],
};

describe('FixStep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedOnEvent = null;
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('with warnings', () => {
    it('shows the first warning file and line number', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain('src/utils/parser.ts:42');
    });

    it('shows the first warning title and ID', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      const frame = lastFrame();
      expect(frame).toContain('[AP-003]');
      expect(frame).toContain("Explicit 'any' type");
    });

    it('shows the warning message', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain("Using 'any' defeats type safety");
    });

    it('shows the suggestion', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain("Replace 'any' with a proper type definition or use 'unknown'");
    });

    it('shows instruction to fix and save the file', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain('Fix the issue above, then save the file');
    });

    it('shows "Checking..." when the target file changes', async () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      expect(capturedOnEvent).not.toBeNull();
      capturedOnEvent!({ type: 'change', path: 'src/utils/parser.ts' });

      await vi.waitFor(() => {
        expect(lastFrame()).toContain('Checking...');
      });
    });

    it('shows "Fixed! Warning resolved." after fix is detected', async () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      capturedOnEvent!({ type: 'change', path: 'src/utils/parser.ts' });

      // Advance past the 500ms checking delay
      vi.advanceTimersByTime(500);

      await vi.waitFor(() => {
        expect(lastFrame()).toContain('Fixed! Warning resolved.');
      });
    });

    it('calls onComplete after fix is detected', async () => {
      const onComplete = vi.fn();
      render(<FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />);

      capturedOnEvent!({ type: 'change', path: 'src/utils/parser.ts' });

      // Advance past the 500ms checking delay
      vi.advanceTimersByTime(500);

      await vi.waitFor(() => {
        expect(onComplete).toHaveBeenCalledTimes(1);
      });
    });

    it('does not react to changes in unrelated files', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      capturedOnEvent!({ type: 'change', path: 'src/unrelated/other.ts' });

      const frame = lastFrame();
      expect(frame).toContain('Fix the issue above');
      expect(frame).not.toContain('Checking...');
      expect(onComplete).not.toHaveBeenCalled();
    });

    it('shows the component title', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain('Fix an Issue');
    });

    it('shows the intro text', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain("Let's fix a real warning from your project");
    });

    it('cleans up the watcher on unmount', () => {
      const onComplete = vi.fn();
      const { unmount } = render(
        <FixStep scanResults={mockResultsWithWarnings} onComplete={onComplete} />
      );

      unmount();

      expect(mockClose).toHaveBeenCalled();
    });
  });

  describe('clean project', () => {
    it('shows clean project message', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsClean} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain('Your project is clean! No warnings to fix.');
    });

    it('shows simulated example with warning ID and title', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsClean} onComplete={onComplete} />
      );

      const frame = lastFrame();
      expect(frame).toContain('[AP-003]');
      expect(frame).toContain("Explicit 'any' type");
    });

    it('shows simulated example file and line', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsClean} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain('src/example.ts:42');
    });

    it('shows fix explanation for clean project', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsClean} onComplete={onComplete} />
      );

      expect(lastFrame()).toContain(
        "You'd fix the issue and save — Anvil confirms the fix instantly."
      );
    });

    it('calls onComplete automatically for clean projects', () => {
      const onComplete = vi.fn();
      render(<FixStep scanResults={mockResultsClean} onComplete={onComplete} />);

      // Advance past the auto-complete delay
      vi.advanceTimersByTime(2_000);

      expect(onComplete).toHaveBeenCalledTimes(1);
    });

    it('does not start a watcher for clean projects', () => {
      const onComplete = vi.fn();
      render(<FixStep scanResults={mockResultsClean} onComplete={onComplete} />);

      expect(mockCreateTutorialWatcher).not.toHaveBeenCalled();
    });
  });

  describe('fixConfirmed (navigated back)', () => {
    it('shows success state when fixConfirmed is true', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} fixConfirmed onComplete={onComplete} />
      );

      const frame = lastFrame();
      expect(frame).toContain('Fixed! Warning resolved.');
      expect(frame).toContain('Press Enter to continue');
    });

    it('does not start a watcher when fixConfirmed is true', () => {
      const onComplete = vi.fn();
      render(
        <FixStep scanResults={mockResultsWithWarnings} fixConfirmed onComplete={onComplete} />
      );

      expect(mockCreateTutorialWatcher).not.toHaveBeenCalled();
    });

    it('shows the component title when fixConfirmed', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(
        <FixStep scanResults={mockResultsWithWarnings} fixConfirmed onComplete={onComplete} />
      );

      expect(lastFrame()).toContain('Fix an Issue');
    });
  });

  describe('no scanResults provided', () => {
    it('falls back to clean project view when scanResults is undefined', () => {
      const onComplete = vi.fn();
      const { lastFrame } = render(<FixStep onComplete={onComplete} />);

      expect(lastFrame()).toContain('Your project is clean! No warnings to fix.');
    });

    it('calls onComplete automatically when scanResults is undefined', () => {
      const onComplete = vi.fn();
      render(<FixStep onComplete={onComplete} />);

      vi.advanceTimersByTime(2_000);

      expect(onComplete).toHaveBeenCalledTimes(1);
    });
  });
});
