/**
 * Tutorial types and state management
 */

export type TutorialStepId = 'intro' | 'plan' | 'validate' | 'gate' | 'completion';

export const TUTORIAL_STEPS: TutorialStepId[] = ['intro', 'plan', 'validate', 'gate', 'completion'];

export interface TutorialStep {
  id: TutorialStepId;
  title: string;
  description: string;
  duration: string; // e.g., "~30s", "~1min"
}

export const STEP_DEFINITIONS: Record<TutorialStepId, TutorialStep> = {
  intro: {
    id: 'intro',
    title: 'Welcome to Anvil',
    description: 'Learn what Anvil does and why it matters',
    duration: '~30s',
  },
  plan: {
    id: 'plan',
    title: 'Create a Plan',
    description: 'Generate a sample plan from a template',
    duration: '~1min',
  },
  validate: {
    id: 'validate',
    title: 'Validate',
    description: 'Check your plan for correctness',
    duration: '~30s',
  },
  gate: {
    id: 'gate',
    title: 'Quality Gates',
    description: 'Run quality checks on your code',
    duration: '~1min',
  },
  completion: {
    id: 'completion',
    title: 'Complete!',
    description: 'Next steps and resources',
    duration: '~30s',
  },
};

export interface TutorialState {
  currentStep: TutorialStepId;
  completedSteps: Set<TutorialStepId>;
  samplePlanPath?: string;
  validationResult?: {
    success: boolean;
    message: string;
  };
  gateResult?: {
    success: boolean;
    checks: Array<{ name: string; passed: boolean; message: string }>;
  };
  startedAt: Date;
  cleanupRequested: boolean;
}

export interface TutorialProgress {
  currentStep: number;
  totalSteps: number;
  completedSteps: string[];
  startedAt: string;
}

export function createInitialTutorialState(): TutorialState {
  return {
    currentStep: 'intro',
    completedSteps: new Set(),
    startedAt: new Date(),
    cleanupRequested: false,
  };
}

export function getStepIndex(step: TutorialStepId): number {
  return TUTORIAL_STEPS.indexOf(step);
}

export function getNextStep(current: TutorialStepId): TutorialStepId | null {
  const idx = getStepIndex(current);
  if (idx === TUTORIAL_STEPS.length - 1) return null;
  return TUTORIAL_STEPS[idx + 1];
}

export function getPreviousStep(current: TutorialStepId): TutorialStepId | null {
  const idx = getStepIndex(current);
  if (idx === 0) return null;
  return TUTORIAL_STEPS[idx - 1];
}

export function canGoBack(current: TutorialStepId): boolean {
  return getStepIndex(current) > 0;
}

export function canGoNext(current: TutorialStepId): boolean {
  return getStepIndex(current) < TUTORIAL_STEPS.length - 1;
}

export function isLastStep(current: TutorialStepId): boolean {
  return current === 'completion';
}

export function getProgressPercentage(current: TutorialStepId): number {
  const idx = getStepIndex(current);
  return Math.round(((idx + 1) / TUTORIAL_STEPS.length) * 100);
}

export function formatElapsedTime(startedAt: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - startedAt.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const mins = Math.floor(diffSecs / 60);
  const secs = diffSecs % 60;

  if (mins === 0) return `${secs}s`;
  return `${mins}m ${secs}s`;
}
