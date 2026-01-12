export type { PlanningFormat, ConfigTemplate } from '../../../services/template-generator.js';
export type { EnvironmentInfo } from '../../../services/environment-detector.js';
import type { PlanningFormat, ConfigTemplate } from '../../../services/template-generator.js';
import type { EnvironmentInfo } from '../../../services/environment-detector.js';

export type WizardStep = 'mode' | 'format' | 'directory' | 'checks' | 'summary';

export const WIZARD_STEPS: WizardStep[] = ['mode', 'format', 'directory', 'checks', 'summary'];

export interface WizardState {
  currentStep: WizardStep;
  configTemplate: ConfigTemplate;
  format: PlanningFormat;
  planningDir: string;
  createExample: boolean;
  enabledChecks: string[];
  coverageThreshold: number;
  installHooks: boolean;
}

export interface WizardContext {
  projectRoot: string;
  environment: EnvironmentInfo;
  recommendedChecks: string[];
}

export interface StepProps {
  state: WizardState;
  context: WizardContext;
  onNext: (updates: Partial<WizardState>) => void;
  onBack: () => void;
  onCancel: () => void;
}

export function getDefaultState(): WizardState {
  return {
    currentStep: 'mode',
    configTemplate: 'basic',
    format: 'generic',
    planningDir: 'docs/plans',
    createExample: true,
    enabledChecks: [],
    coverageThreshold: 80,
    installHooks: true,
  };
}

export function getStepIndex(step: WizardStep): number {
  return WIZARD_STEPS.indexOf(step);
}

export function getNextStep(current: WizardStep): WizardStep | null {
  const idx = getStepIndex(current);
  return idx < WIZARD_STEPS.length - 1 ? WIZARD_STEPS[idx + 1] : null;
}

export function getPreviousStep(current: WizardStep): WizardStep | null {
  const idx = getStepIndex(current);
  return idx > 0 ? WIZARD_STEPS[idx - 1] : null;
}

export interface ModeOption {
  value: ConfigTemplate;
  label: string;
  description: string;
}

export const MODE_OPTIONS: ModeOption[] = [
  {
    value: 'basic',
    label: 'Standard',
    description: '80% thresholds — recommended for most projects',
  },
  {
    value: 'strict',
    label: 'Strict',
    description: '90% thresholds — production-ready, high quality bar',
  },
  {
    value: 'ci',
    label: 'CI-optimised',
    description: 'Minimal checks — fast feedback, essential gates only',
  },
];

export interface FormatOption {
  value: PlanningFormat;
  label: string;
  description: string;
}

export const FORMAT_OPTIONS: FormatOption[] = [
  {
    value: 'aps',
    label: 'APS',
    description: 'Anvil Planning Spec — structured task tracking with dependencies',
  },
  {
    value: 'speckit',
    label: 'SpecKit',
    description: 'GitHub spec-kit format with spec.md, plan.md, tasks.md',
  },
  {
    value: 'bmad',
    label: 'BMAD',
    description: 'PRD/Architecture format for larger features',
  },
  {
    value: 'generic',
    label: 'Generic Markdown',
    description: 'Simple markdown with flexible structure',
  },
  {
    value: 'skip',
    label: 'Skip examples',
    description: 'Configure Anvil without creating example files',
  },
];
