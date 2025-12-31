import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { Tutorial } from '../Tutorial.js';
import { IntroStep } from '../steps/IntroStep.js';
import { PlanStep } from '../steps/PlanStep.js';
import { ValidateStep } from '../steps/ValidateStep.js';
import { GateStep } from '../steps/GateStep.js';
import { CompletionStep } from '../steps/CompletionStep.js';
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

describe('Tutorial', () => {
  it('renders header with title', () => {
    const { lastFrame } = render(<Tutorial />);
    expect(lastFrame()).toContain('ANVIL TUTORIAL');
  });

  it('starts on intro step', () => {
    const { lastFrame } = render(<Tutorial />);
    expect(lastFrame()).toContain('Welcome to Anvil');
  });

  it('shows progress indicator', () => {
    const { lastFrame } = render(<Tutorial />);
    expect(lastFrame()).toContain('Step');
    expect(lastFrame()).toContain('of 5');
  });

  it('shows keyboard shortcuts', () => {
    const { lastFrame } = render(<Tutorial />);
    expect(lastFrame()).toContain('q quit');
  });
});

describe('IntroStep', () => {
  it('renders intro content', () => {
    const onNext = vi.fn();
    const { lastFrame } = render(<IntroStep onNext={onNext} />);

    expect(lastFrame()).toContain('What is Anvil?');
    expect(lastFrame()).toContain('safe for production');
  });

  it('lists key features', () => {
    const onNext = vi.fn();
    const { lastFrame } = render(<IntroStep onNext={onNext} />);

    expect(lastFrame()).toContain('Validation');
    expect(lastFrame()).toContain('Quality Gates');
    expect(lastFrame()).toContain('Audit Trail');
    expect(lastFrame()).toContain('Rollback');
  });

  it('shows continue prompt', () => {
    const onNext = vi.fn();
    const { lastFrame } = render(<IntroStep onNext={onNext} />);

    expect(lastFrame()).toContain('Press Enter to continue');
  });
});

describe('PlanStep', () => {
  it('shows creating message initially', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<PlanStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Creating a Sample Plan');
  });

  it('shows sample plan content', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<PlanStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Sample Feature Plan');
    expect(lastFrame()).toContain('Intent');
  });

  it('uses provided path when given', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <PlanStep onComplete={onComplete} samplePlanPath="/custom/path.md" />
    );

    expect(lastFrame()).toContain('/custom/path.md');
  });
});

describe('ValidateStep', () => {
  it('shows validation command', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <ValidateStep planPath=".anvil/tutorial/sample.md" onComplete={onComplete} />
    );

    expect(lastFrame()).toContain('anvil validate');
    expect(lastFrame()).toContain('.anvil/tutorial/sample.md');
  });

  it('shows explanation text', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<ValidateStep planPath="test.md" onComplete={onComplete} />);

    expect(lastFrame()).toContain('Validating Your Plan');
    expect(lastFrame()).toContain('well-formed');
  });

  it('shows result when provided', () => {
    const onComplete = vi.fn();
    const result = { success: true, message: 'All checks passed' };
    const { lastFrame } = render(
      <ValidateStep planPath="test.md" onComplete={onComplete} validationResult={result} />
    );

    expect(lastFrame()).toContain('Validation passed');
    expect(lastFrame()).toContain('All checks passed');
  });
});

describe('GateStep', () => {
  it('shows gate command', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <GateStep planPath=".anvil/tutorial/sample.md" onComplete={onComplete} />
    );

    expect(lastFrame()).toContain('anvil gate');
    expect(lastFrame()).toContain('.anvil/tutorial/sample.md');
  });

  it('shows explanation text', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(<GateStep planPath="test.md" onComplete={onComplete} />);

    expect(lastFrame()).toContain('Running Quality Gates');
    expect(lastFrame()).toContain('verify your code');
  });

  it('shows gate checks', () => {
    const onComplete = vi.fn();
    const result = {
      success: true,
      checks: [
        { name: 'lint', passed: true, message: 'OK' },
        { name: 'test', passed: true, message: 'OK' },
      ],
    };
    const { lastFrame } = render(
      <GateStep planPath="test.md" onComplete={onComplete} gateResult={result} />
    );

    expect(lastFrame()).toContain('lint');
    expect(lastFrame()).toContain('test');
  });
});

describe('CompletionStep', () => {
  it('shows completion message', () => {
    const startedAt = new Date();
    const { lastFrame } = render(
      <CompletionStep startedAt={startedAt} onCleanup={vi.fn()} onFinish={vi.fn()} />
    );

    expect(lastFrame()).toContain('Tutorial Complete');
  });

  it('shows what user learned', () => {
    const startedAt = new Date();
    const { lastFrame } = render(
      <CompletionStep startedAt={startedAt} onCleanup={vi.fn()} onFinish={vi.fn()} />
    );

    expect(lastFrame()).toContain('Creating plans');
    expect(lastFrame()).toContain('Validating plans');
    expect(lastFrame()).toContain('quality gates');
  });

  it('shows next steps', () => {
    const startedAt = new Date();
    const { lastFrame } = render(
      <CompletionStep startedAt={startedAt} onCleanup={vi.fn()} onFinish={vi.fn()} />
    );

    expect(lastFrame()).toContain('anvil init');
    expect(lastFrame()).toContain('anvil doctor');
    expect(lastFrame()).toContain('anvil new');
  });

  it('shows cleanup instructions', () => {
    const startedAt = new Date();
    const { lastFrame } = render(
      <CompletionStep startedAt={startedAt} onCleanup={vi.fn()} onFinish={vi.fn()} />
    );

    expect(lastFrame()).toContain('c');
    expect(lastFrame()).toContain('clean up');
    expect(lastFrame()).toContain('q');
  });
});

describe('Type utilities', () => {
  describe('TUTORIAL_STEPS', () => {
    it('has correct step order', () => {
      expect(TUTORIAL_STEPS).toEqual(['intro', 'plan', 'validate', 'gate', 'completion']);
    });

    it('has 5 steps', () => {
      expect(TUTORIAL_STEPS.length).toBe(5);
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
  });

  describe('createInitialTutorialState', () => {
    it('starts on intro step', () => {
      const state = createInitialTutorialState();
      expect(state.currentStep).toBe('intro');
    });

    it('has empty completed steps', () => {
      const state = createInitialTutorialState();
      expect(state.completedSteps.size).toBe(0);
    });

    it('has startedAt timestamp', () => {
      const state = createInitialTutorialState();
      expect(state.startedAt).toBeInstanceOf(Date);
    });
  });

  describe('getStepIndex', () => {
    it('returns correct index', () => {
      expect(getStepIndex('intro')).toBe(0);
      expect(getStepIndex('plan')).toBe(1);
      expect(getStepIndex('validate')).toBe(2);
      expect(getStepIndex('gate')).toBe(3);
      expect(getStepIndex('completion')).toBe(4);
    });
  });

  describe('getNextStep', () => {
    it('returns next step', () => {
      expect(getNextStep('intro')).toBe('plan');
      expect(getNextStep('plan')).toBe('validate');
      expect(getNextStep('gate')).toBe('completion');
    });

    it('returns null at last step', () => {
      expect(getNextStep('completion')).toBeNull();
    });
  });

  describe('getPreviousStep', () => {
    it('returns previous step', () => {
      expect(getPreviousStep('plan')).toBe('intro');
      expect(getPreviousStep('completion')).toBe('gate');
    });

    it('returns null at first step', () => {
      expect(getPreviousStep('intro')).toBeNull();
    });
  });

  describe('canGoBack', () => {
    it('returns false at first step', () => {
      expect(canGoBack('intro')).toBe(false);
    });

    it('returns true at other steps', () => {
      expect(canGoBack('plan')).toBe(true);
      expect(canGoBack('completion')).toBe(true);
    });
  });

  describe('canGoNext', () => {
    it('returns true at non-final steps', () => {
      expect(canGoNext('intro')).toBe(true);
      expect(canGoNext('gate')).toBe(true);
    });

    it('returns false at last step', () => {
      expect(canGoNext('completion')).toBe(false);
    });
  });

  describe('isLastStep', () => {
    it('returns true only for completion', () => {
      expect(isLastStep('completion')).toBe(true);
      expect(isLastStep('intro')).toBe(false);
      expect(isLastStep('gate')).toBe(false);
    });
  });

  describe('getProgressPercentage', () => {
    it('returns correct percentages', () => {
      expect(getProgressPercentage('intro')).toBe(20);
      expect(getProgressPercentage('plan')).toBe(40);
      expect(getProgressPercentage('validate')).toBe(60);
      expect(getProgressPercentage('gate')).toBe(80);
      expect(getProgressPercentage('completion')).toBe(100);
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
