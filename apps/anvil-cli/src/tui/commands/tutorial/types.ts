import { z } from 'zod';

export type TutorialStepId = 'scan' | 'watch' | 'fix' | 'next-steps';

export const TUTORIAL_STEPS: TutorialStepId[] = ['scan', 'watch', 'fix', 'next-steps'];

export interface TutorialStep {
  id: TutorialStepId;
  title: string;
  description: string;
  duration: string; // e.g., "~30s", "~1min"
}

export const STEP_DEFINITIONS: Record<TutorialStepId, TutorialStep> = {
  scan: {
    id: 'scan',
    title: 'Scan Your Project',
    description: 'Analyse your codebase for issues',
    duration: '~1min',
  },
  watch: {
    id: 'watch',
    title: 'Watch Mode',
    description: 'See real-time validation as you edit',
    duration: '~1min',
  },
  fix: {
    id: 'fix',
    title: 'Fix an Issue',
    description: 'Resolve a warning and see it clear',
    duration: '~1min',
  },
  'next-steps': {
    id: 'next-steps',
    title: "What's Next",
    description: 'Explore more Anvil features',
    duration: '~30s',
  },
};

export interface ScanWarning {
  id: string;
  title: string;
  file: string;
  line: number;
  message: string;
  suggestion: string;
}

export interface ScanResults {
  warningCount: number;
  fileCount: number;
  executionTimeMs: number;
  topWarnings: ScanWarning[];
}

export interface TutorialState {
  currentStep: TutorialStepId;
  completedSteps: Set<TutorialStepId>;
  scanResults?: ScanResults;
  watchTriggered: boolean;
  fixConfirmed: boolean;
  startedAt: Date;
  cleanupConfirming: boolean;
  cleanupRequested: boolean;
}

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

export function createInitialTutorialState(initialStep?: TutorialStepId): TutorialState {
  const step = initialStep ?? 'scan';
  const idx = getStepIndex(step);
  const completed = new Set<TutorialStepId>(TUTORIAL_STEPS.slice(0, idx));
  return {
    currentStep: step,
    completedSteps: completed,
    watchTriggered: false,
    fixConfirmed: false,
    startedAt: new Date(),
    cleanupConfirming: false,
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
  return current === 'next-steps';
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
