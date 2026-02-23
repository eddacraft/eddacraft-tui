import React, { useState, useCallback, useRef } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../../components/Header.js';
import { ProgressBar } from '../../../components/ProgressBar.js';
import { theme } from '../../../utils/theme.js';
import { TutorialPicker, resolveTutorialKey } from '../components/TutorialPicker.js';
import type { TutorialOption } from '../components/TutorialPicker.js';
import { IntroStep } from './drift-steps/IntroStep.js';
import { CaptureStep } from './drift-steps/CaptureStep.js';
import { InspectStep } from './drift-steps/InspectStep.js';
import { CompareStep } from './drift-steps/CompareStep.js';
import { SummaryStep } from './drift-steps/SummaryStep.js';

export type DriftStepId = 'intro' | 'capture' | 'inspect' | 'compare' | 'summary';

export const DRIFT_STEPS: DriftStepId[] = ['intro', 'capture', 'inspect', 'compare', 'summary'];

export interface DriftStepDef {
  id: DriftStepId;
  title: string;
  description: string;
}

export const DRIFT_STEP_DEFINITIONS: Record<DriftStepId, DriftStepDef> = {
  intro: {
    id: 'intro',
    title: 'Introduction',
    description: 'What drift detection is and why it matters',
  },
  capture: {
    id: 'capture',
    title: 'Capture Baseline',
    description: 'Take a snapshot of your current architecture',
  },
  inspect: {
    id: 'inspect',
    title: 'Inspect Snapshot',
    description: 'Explore what a snapshot contains',
  },
  compare: {
    id: 'compare',
    title: 'Compare Snapshots',
    description: 'Detect drift between two points in time',
  },
  summary: {
    id: 'summary',
    title: 'Summary',
    description: 'Quick reference and next steps',
  },
};

function getStepIndex(step: DriftStepId): number {
  return DRIFT_STEPS.indexOf(step);
}

function getNextStep(current: DriftStepId): DriftStepId | null {
  const idx = getStepIndex(current);
  if (idx === DRIFT_STEPS.length - 1) return null;
  return DRIFT_STEPS[idx + 1];
}

function getPreviousStep(current: DriftStepId): DriftStepId | null {
  const idx = getStepIndex(current);
  if (idx === 0) return null;
  return DRIFT_STEPS[idx - 1];
}

function canGoBack(current: DriftStepId): boolean {
  return getStepIndex(current) > 0;
}

function isLastStep(current: DriftStepId): boolean {
  return current === 'summary';
}

function getProgressPercentage(current: DriftStepId): number {
  const idx = getStepIndex(current);
  return Math.round(((idx + 1) / DRIFT_STEPS.length) * 100);
}

interface DriftTutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
  onSelectTutorial?: (topic: string) => void;
  tutorials?: TutorialOption[];
}

export function DriftTutorial({
  onComplete,
  onCleanup,
  onSelectTutorial,
  tutorials = [],
}: DriftTutorialProps): React.ReactElement {
  const { exit } = useApp();
  const [currentStep, setCurrentStep] = useState<DriftStepId>('intro');
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

      const topic = resolveTutorialKey(tutorials, 'drift', input);
      if (topic) {
        onSelectTutorial?.(topic);
        exit();
      }
    }
  });

  const currentStepDef = DRIFT_STEP_DEFINITIONS[currentStep];
  const stepNumber = DRIFT_STEPS.indexOf(currentStep) + 1;
  const totalSteps = DRIFT_STEPS.length;
  const progress = getProgressPercentage(currentStep);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Drift Tutorial" subtitle={currentStepDef.title} />

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
        {currentStep === 'capture' && <CaptureStep />}
        {currentStep === 'inspect' && <InspectStep />}
        {currentStep === 'compare' && <CompareStep />}
        {currentStep === 'summary' && <SummaryStep />}
      </Box>

      {isLastStep(currentStep) && tutorials.length > 0 && (
        <Box marginY={1}>
          <TutorialPicker tutorials={tutorials} currentTopic="drift" />
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
