import React, { useState, useCallback, useRef } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../../components/Header.js';
import { ProgressBar } from '../../../components/ProgressBar.js';
import { theme } from '../../../utils/theme.js';
import { TutorialPicker, resolveTutorialKey } from '../components/TutorialPicker.js';
import type { TutorialOption } from '../components/TutorialPicker.js';
import { IntroStep } from './ci-steps/IntroStep.js';
import { DetectStep } from './ci-steps/DetectStep.js';
import { WorkflowStep } from './ci-steps/WorkflowStep.js';
import { ExitCodesStep } from './ci-steps/ExitCodesStep.js';
import { HooksStep } from './ci-steps/HooksStep.js';
import { SummaryStep } from './ci-steps/SummaryStep.js';

export type CIStepId = 'intro' | 'detect' | 'workflow' | 'exit-codes' | 'hooks' | 'summary';

export const CI_STEPS: CIStepId[] = [
  'intro',
  'detect',
  'workflow',
  'exit-codes',
  'hooks',
  'summary',
];

export interface CIStepDef {
  id: CIStepId;
  title: string;
  description: string;
}

export const CI_STEP_DEFINITIONS: Record<CIStepId, CIStepDef> = {
  intro: {
    id: 'intro',
    title: 'Introduction',
    description: 'Why CI integration matters for architecture enforcement',
  },
  detect: {
    id: 'detect',
    title: 'Detect CI System',
    description: 'Scan for CI configuration in your project',
  },
  workflow: {
    id: 'workflow',
    title: 'Generate Workflow',
    description: 'See a ready-made CI workflow for your system',
  },
  'exit-codes': {
    id: 'exit-codes',
    title: 'Exit Codes',
    description: 'Understand how Anvil signals pass/fail to CI',
  },
  hooks: {
    id: 'hooks',
    title: 'Git Hooks',
    description: 'Set up pre-commit hooks as an extra safety layer',
  },
  summary: {
    id: 'summary',
    title: 'Summary',
    description: 'Quick reference and next steps',
  },
};

function getStepIndex(step: CIStepId): number {
  return CI_STEPS.indexOf(step);
}

function getNextStep(current: CIStepId): CIStepId | null {
  const idx = getStepIndex(current);
  if (idx === CI_STEPS.length - 1) return null;
  return CI_STEPS[idx + 1];
}

function getPreviousStep(current: CIStepId): CIStepId | null {
  const idx = getStepIndex(current);
  if (idx === 0) return null;
  return CI_STEPS[idx - 1];
}

function canGoBack(current: CIStepId): boolean {
  return getStepIndex(current) > 0;
}

function isLastStep(current: CIStepId): boolean {
  return current === 'summary';
}

function getProgressPercentage(current: CIStepId): number {
  const idx = getStepIndex(current);
  return Math.round(((idx + 1) / CI_STEPS.length) * 100);
}

interface CITutorialProps {
  onComplete?: () => void;
  onSelectTutorial?: (topic: string) => void;
  tutorials?: TutorialOption[];
  completedTopics?: string[];
}

export function CITutorial({
  onComplete,
  onSelectTutorial,
  tutorials = [],
  completedTopics = [],
}: CITutorialProps): React.ReactElement {
  const { exit } = useApp();
  const [currentStep, setCurrentStep] = useState<CIStepId>('intro');
  const currentStepRef = useRef(currentStep);
  currentStepRef.current = currentStep;

  const handleNext = useCallback(() => {
    setCurrentStep((prev) => getNextStep(prev) ?? prev);
  }, []);

  const handleBack = useCallback(() => {
    setCurrentStep((prev) => getPreviousStep(prev) ?? prev);
  }, []);

  const handleFinish = useCallback(() => {
    if (isLastStep(currentStepRef.current)) onComplete?.();
    exit();
  }, [onComplete, exit]);

  useInput((input, key) => {
    const step = currentStepRef.current;

    if (input === 'q' || (key.ctrl && input === 'c')) {
      handleFinish();
      return;
    }

    if (key.return && !isLastStep(step)) {
      handleNext();
      return;
    }

    if (key.leftArrow && canGoBack(step)) {
      handleBack();
      return;
    }

    if (isLastStep(step)) {
      const topic = resolveTutorialKey(tutorials, 'ci', input, completedTopics);
      if (topic) {
        onSelectTutorial?.(topic);
        exit();
      }
    }
  });

  const currentStepDef = CI_STEP_DEFINITIONS[currentStep];
  const stepNumber = CI_STEPS.indexOf(currentStep) + 1;
  const totalSteps = CI_STEPS.length;
  const progress = getProgressPercentage(currentStep);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil CI Tutorial" subtitle={currentStepDef.title} />

      <Box marginY={1} flexDirection="column">
        <Box marginBottom={1}>
          <Text color={theme.colours.smoke}>
            Step {stepNumber} of {totalSteps}: {currentStepDef.description}
          </Text>
        </Box>
        <ProgressBar percent={progress} width={40} showPercent={false} />
      </Box>

      <Box flexDirection="column" marginY={1}>
        {currentStep === 'intro' && <IntroStep />}
        {currentStep === 'detect' && <DetectStep />}
        {currentStep === 'workflow' && <WorkflowStep />}
        {currentStep === 'exit-codes' && <ExitCodesStep />}
        {currentStep === 'hooks' && <HooksStep />}
        {currentStep === 'summary' && <SummaryStep />}
      </Box>

      {isLastStep(currentStep) && tutorials.length > 0 && (
        <Box marginY={1}>
          <TutorialPicker
            tutorials={tutorials}
            currentTopic="ci"
            completedTopics={completedTopics}
          />
        </Box>
      )}

      <Box marginTop={1}>
        <Text color={theme.colours.ash}>
          {canGoBack(currentStep) && (
            <>
              <Text color={theme.colours.ember}>{theme.icons.backArrow}</Text>
              {' back  '}
            </>
          )}
          {!isLastStep(currentStep) && 'Enter next  '}
          <Text color={theme.colours.ember}>q</Text>
          {' quit'}
        </Text>
      </Box>
    </Box>
  );
}
