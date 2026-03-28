import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import { stripAnsi } from '../../../../tui/__tests__/test-utils.js';
import {
  TUTORIAL_STEPS,
  STEP_DEFINITIONS,
  createInitialTutorialState,
  getStepIndex,
  getNextStep,
  getPreviousStep,
  canGoBack,
  canGoNext,
  isLastStep,
  getProgressPercentage,
  formatElapsedTime,
} from '../types.js';

// Mock scan-project to prevent real filesystem access
vi.mock('../steps/scan-project.js', () => ({
  scanProject: vi.fn(() => new Promise(() => {})), // never resolves by default
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

// Import after mocks
import { Tutorial } from '../Tutorial.js';

describe('Type utilities', () => {
  describe('TUTORIAL_STEPS', () => {
    it('has correct step order', () => {
      expect(TUTORIAL_STEPS).toEqual(['scan', 'watch', 'fix', 'next-steps']);
    });

    it('has 4 steps', () => {
      expect(TUTORIAL_STEPS.length).toBe(4);
    });
  });

  describe('STEP_DEFINITIONS', () => {
    it('has definition for each step', () => {
      for (const step of TUTORIAL_STEPS) {
        expect(STEP_DEFINITIONS[step]).toBeDefined();
        expect(STEP_DEFINITIONS[step].title).toBeDefined();
        expect(STEP_DEFINITIONS[step].description).toBeDefined();
      }
    });

    it('has correct titles', () => {
      expect(STEP_DEFINITIONS['scan'].title).toBe('Scan Your Project');
      expect(STEP_DEFINITIONS['watch'].title).toBe('Watch Mode');
      expect(STEP_DEFINITIONS['fix'].title).toBe('Fix an Issue');
      expect(STEP_DEFINITIONS['next-steps'].title).toBe("What's Next");
    });

    it('has correct descriptions', () => {
      expect(STEP_DEFINITIONS['scan'].description).toBe('Analyse your codebase for issues');
      expect(STEP_DEFINITIONS['watch'].description).toBe('See real-time validation as you edit');
      expect(STEP_DEFINITIONS['fix'].description).toBe('Resolve a warning and see it clear');
      expect(STEP_DEFINITIONS['next-steps'].description).toBe('Explore more Anvil features');
    });
  });

  describe('createInitialTutorialState', () => {
    it('starts on scan step', () => {
      const state = createInitialTutorialState();
      expect(state.currentStep).toBe('scan');
    });

    it('has empty completed steps', () => {
      const state = createInitialTutorialState();
      expect(state.completedSteps.size).toBe(0);
    });

    it('has startedAt timestamp', () => {
      const state = createInitialTutorialState();
      expect(state.startedAt).toBeInstanceOf(Date);
    });

    it('initialises watchTriggered to false', () => {
      const state = createInitialTutorialState();
      expect(state.watchTriggered).toBe(false);
    });

    it('initialises fixConfirmed to false', () => {
      const state = createInitialTutorialState();
      expect(state.fixConfirmed).toBe(false);
    });

    it('has no scanResults initially', () => {
      const state = createInitialTutorialState();
      expect(state.scanResults).toBeUndefined();
    });
  });

  describe('getStepIndex', () => {
    it('returns correct index', () => {
      expect(getStepIndex('scan')).toBe(0);
      expect(getStepIndex('watch')).toBe(1);
      expect(getStepIndex('fix')).toBe(2);
      expect(getStepIndex('next-steps')).toBe(3);
    });
  });

  describe('getNextStep', () => {
    it('returns next step', () => {
      expect(getNextStep('scan')).toBe('watch');
      expect(getNextStep('watch')).toBe('fix');
      expect(getNextStep('fix')).toBe('next-steps');
    });

    it('returns null at last step', () => {
      expect(getNextStep('next-steps')).toBeNull();
    });
  });

  describe('getPreviousStep', () => {
    it('returns previous step', () => {
      expect(getPreviousStep('watch')).toBe('scan');
      expect(getPreviousStep('fix')).toBe('watch');
      expect(getPreviousStep('next-steps')).toBe('fix');
    });

    it('returns null at first step', () => {
      expect(getPreviousStep('scan')).toBeNull();
    });
  });

  describe('canGoBack', () => {
    it('returns false at first step', () => {
      expect(canGoBack('scan')).toBe(false);
    });

    it('returns true at other steps', () => {
      expect(canGoBack('watch')).toBe(true);
      expect(canGoBack('fix')).toBe(true);
      expect(canGoBack('next-steps')).toBe(true);
    });
  });

  describe('canGoNext', () => {
    it('returns true at non-final steps', () => {
      expect(canGoNext('scan')).toBe(true);
      expect(canGoNext('watch')).toBe(true);
      expect(canGoNext('fix')).toBe(true);
    });

    it('returns false at last step', () => {
      expect(canGoNext('next-steps')).toBe(false);
    });
  });

  describe('isLastStep', () => {
    it('returns true only for next-steps', () => {
      expect(isLastStep('next-steps')).toBe(true);
      expect(isLastStep('scan')).toBe(false);
      expect(isLastStep('watch')).toBe(false);
      expect(isLastStep('fix')).toBe(false);
    });
  });

  describe('getProgressPercentage', () => {
    it('returns correct percentages', () => {
      expect(getProgressPercentage('scan')).toBe(25);
      expect(getProgressPercentage('watch')).toBe(50);
      expect(getProgressPercentage('fix')).toBe(75);
      expect(getProgressPercentage('next-steps')).toBe(100);
    });
  });

  describe('formatElapsedTime', () => {
    it('formats seconds only', () => {
      const startedAt = new Date(Date.now() - 30000);
      expect(formatElapsedTime(startedAt)).toBe('30s');
    });

    it('formats minutes and seconds', () => {
      const startedAt = new Date(Date.now() - 90000);
      expect(formatElapsedTime(startedAt)).toBe('1m 30s');
    });
  });
});

describe('Tutorial component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders and starts on scan step', () => {
    const { lastFrame } = render(<Tutorial />);

    const frame = lastFrame();
    expect(frame).toContain('Scan Your Project');
  });

  it('shows progress indicator with step count', () => {
    const { lastFrame } = render(<Tutorial />);

    expect(lastFrame()).toContain('Step 1 of 4');
  });

  it('shows step description', () => {
    const { lastFrame } = render(<Tutorial />);

    expect(lastFrame()).toContain('Analyse your codebase for issues');
  });

  it('shows keyboard shortcuts', () => {
    const { lastFrame } = render(<Tutorial />);

    const frame = stripAnsi(lastFrame()!);
    expect(frame).toContain('q quit');
  });

  it('shows the header title', () => {
    const { lastFrame } = render(<Tutorial />);

    expect(lastFrame()).toContain('ANVIL TUTORIAL');
  });
});
