import React, { useState, useCallback, useRef } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../../components/Header.js';
import { ProgressBar } from '../../../components/ProgressBar.js';
import { theme } from '../../../utils/theme.js';
import { TutorialPicker, resolveTutorialKey } from '../components/TutorialPicker.js';
import type { TutorialOption } from '../components/TutorialPicker.js';
import { IntroStep } from './architecture-steps/IntroStep.js';
import { DetectStep } from './architecture-steps/DetectStep.js';
import { TemplateStep } from './architecture-steps/TemplateStep.js';
import { CompileStep } from './architecture-steps/CompileStep.js';
import { ValidateStep } from './architecture-steps/ValidateStep.js';
import { SummaryStep } from './architecture-steps/SummaryStep.js';

export type ArchitectureStepId =
  | 'intro'
  | 'detect'
  | 'template'
  | 'compile'
  | 'validate'
  | 'summary';

export const ARCHITECTURE_STEPS: ArchitectureStepId[] = [
  'intro',
  'detect',
  'template',
  'compile',
  'validate',
  'summary',
];

export interface ArchitectureStepDef {
  id: ArchitectureStepId;
  title: string;
  description: string;
}

export const ARCHITECTURE_STEP_DEFINITIONS: Record<ArchitectureStepId, ArchitectureStepDef> = {
  intro: {
    id: 'intro',
    title: 'Introduction',
    description: 'What architecture boundaries are and why they matter',
  },
  detect: {
    id: 'detect',
    title: 'Detect Structure',
    description: 'Scan your project for recognised patterns',
  },
  template: {
    id: 'template',
    title: 'Choose Template',
    description: 'Pick an architecture template for your project',
  },
  compile: {
    id: 'compile',
    title: 'Compile Rules',
    description: 'Generate Rego rules from your architecture definition',
  },
  validate: {
    id: 'validate',
    title: 'Validate Boundaries',
    description: 'Check for boundary violations and learn to fix them',
  },
  summary: {
    id: 'summary',
    title: 'Summary',
    description: 'Quick reference and next steps',
  },
};

function getStepIndex(step: ArchitectureStepId): number {
  return ARCHITECTURE_STEPS.indexOf(step);
}

function getNextStep(current: ArchitectureStepId): ArchitectureStepId | null {
  const idx = getStepIndex(current);
  if (idx === ARCHITECTURE_STEPS.length - 1) return null;
  return ARCHITECTURE_STEPS[idx + 1];
}

function getPreviousStep(current: ArchitectureStepId): ArchitectureStepId | null {
  const idx = getStepIndex(current);
  if (idx === 0) return null;
  return ARCHITECTURE_STEPS[idx - 1];
}

function canGoBack(current: ArchitectureStepId): boolean {
  return getStepIndex(current) > 0;
}

function isLastStep(current: ArchitectureStepId): boolean {
  return current === 'summary';
}

function getProgressPercentage(current: ArchitectureStepId): number {
  const idx = getStepIndex(current);
  return Math.round(((idx + 1) / ARCHITECTURE_STEPS.length) * 100);
}

interface ArchitectureTutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
  onSelectTutorial?: (topic: string) => void;
  tutorials?: TutorialOption[];
}

export function ArchitectureTutorial({
  onComplete,
  onCleanup,
  onSelectTutorial,
  tutorials = [],
}: ArchitectureTutorialProps): React.ReactElement {
  const { exit } = useApp();
  const [currentStep, setCurrentStep] = useState<ArchitectureStepId>('intro');
  const [cleanedUp, setCleanedUp] = useState(false);
  const currentStepRef = useRef(currentStep);
  currentStepRef.current = currentStep;

  const handleNext = useCallback(() => {
    setCurrentStep((prev) => getNextStep(prev) ?? prev);
  }, []);

  const handleBack = useCallback(() => {
    setCurrentStep((prev) => getPreviousStep(prev) ?? prev);
  }, []);

  const handleCleanup = useCallback(() => {
    setCleanedUp(true);
    onCleanup?.();
  }, [onCleanup]);

  const handleFinish = useCallback(() => {
    onComplete?.();
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
      if (input === 'c' && !cleanedUp) {
        handleCleanup();
        return;
      }
      if (input === 'q') {
        handleFinish();
        return;
      }

      const topic = resolveTutorialKey(tutorials, 'architecture', input);
      if (topic) {
        onSelectTutorial?.(topic);
        exit();
      }
    }
  });

  const currentStepDef = ARCHITECTURE_STEP_DEFINITIONS[currentStep];
  const stepNumber = ARCHITECTURE_STEPS.indexOf(currentStep) + 1;
  const totalSteps = ARCHITECTURE_STEPS.length;
  const progress = getProgressPercentage(currentStep);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Architecture Tutorial" subtitle={currentStepDef.title} />

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
        {currentStep === 'template' && <TemplateStep />}
        {currentStep === 'compile' && <CompileStep />}
        {currentStep === 'validate' && <ValidateStep />}
        {currentStep === 'summary' && <SummaryStep />}
      </Box>

      {isLastStep(currentStep) && tutorials.length > 0 && (
        <Box marginY={1}>
          <TutorialPicker tutorials={tutorials} currentTopic="architecture" />
        </Box>
      )}

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          {canGoBack(currentStep) && `${theme.icons.arrow} back `}
          {!isLastStep(currentStep) && 'Enter next '}
          {isLastStep(currentStep) && (
            <>
              <Text color={theme.colours.text}>c</Text>
              {' clean up  '}
            </>
          )}
          {theme.icons.bullet} q quit
        </Text>
      </Box>
    </Box>
  );
}
