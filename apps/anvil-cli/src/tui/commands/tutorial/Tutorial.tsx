import React, { useState, useCallback } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../components/Header.js';
import { ProgressBar } from '../../components/ProgressBar.js';
import { theme } from '../../utils/theme.js';
import type { TutorialState, TutorialStepId } from './types.js';
import {
  createInitialTutorialState,
  getNextStep,
  getPreviousStep,
  getProgressPercentage,
  STEP_DEFINITIONS,
  TUTORIAL_STEPS,
  canGoBack,
  isLastStep,
} from './types.js';

interface TutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
}

// TODO: Step components (ScanStep, WatchStep, FixStep, NextStepsStep) will be
// implemented in subsequent tasks. For now the Tutorial renders placeholder text.

export function Tutorial({ onComplete, onCleanup }: TutorialProps): React.ReactElement {
  const { exit } = useApp();
  const [state, setState] = useState<TutorialState>(createInitialTutorialState);

  const goToStep = useCallback((step: TutorialStepId) => {
    setState((prev) => ({
      ...prev,
      currentStep: step,
      completedSteps: new Set([...prev.completedSteps, prev.currentStep]),
    }));
  }, []);

  const handleNext = useCallback(() => {
    const next = getNextStep(state.currentStep);
    if (next) {
      goToStep(next);
    }
  }, [state.currentStep, goToStep]);

  const handleBack = useCallback(() => {
    const prev = getPreviousStep(state.currentStep);
    if (prev) {
      goToStep(prev);
    }
  }, [state.currentStep, goToStep]);

  const handleCleanup = useCallback(() => {
    setState((prev) => ({ ...prev, cleanupRequested: true }));
    onCleanup?.();
  }, [onCleanup]);

  const handleFinish = useCallback(() => {
    onComplete?.();
    exit();
  }, [onComplete, exit]);

  useInput((input, key) => {
    if (input === 'q' || (key.ctrl && input === 'c')) {
      handleFinish();
      return;
    }

    if (key.return && !isLastStep(state.currentStep)) {
      handleNext();
      return;
    }

    if (key.leftArrow && canGoBack(state.currentStep)) {
      handleBack();
      return;
    }

    if (isLastStep(state.currentStep)) {
      if (input === 'c') {
        handleCleanup();
      }
    }
  });

  const currentStepDef = STEP_DEFINITIONS[state.currentStep];
  const stepNumber = TUTORIAL_STEPS.indexOf(state.currentStep) + 1;
  const totalSteps = TUTORIAL_STEPS.length;
  const progress = getProgressPercentage(state.currentStep);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Tutorial" subtitle={currentStepDef.title} />

      <Box marginY={1} flexDirection="column">
        <Box marginBottom={1}>
          <Text color={theme.colours.smoke}>
            Step {stepNumber} of {totalSteps}: {currentStepDef.description}
          </Text>
        </Box>
        <ProgressBar percent={progress} width={40} showPercent={false} />
      </Box>

      <Box flexDirection="column" marginY={1}>
        {/* Step components will be added in later tasks */}
        <Text color={theme.colours.steel}>{currentStepDef.title}</Text>
        <Text color={theme.colours.smoke}>{currentStepDef.description}</Text>
      </Box>

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          {canGoBack(state.currentStep) && `${theme.icons.arrow} back `}
          {!isLastStep(state.currentStep) && 'Enter next '}
          {theme.icons.bullet} q quit
        </Text>
      </Box>
    </Box>
  );
}
