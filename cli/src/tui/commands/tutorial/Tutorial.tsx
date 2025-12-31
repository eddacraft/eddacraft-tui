import React, { useState, useCallback } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../components/Header.js';
import { ProgressBar } from '../../components/ProgressBar.js';
import { theme } from '../../utils/theme.js';
import { IntroStep } from './steps/IntroStep.js';
import { PlanStep } from './steps/PlanStep.js';
import { ValidateStep } from './steps/ValidateStep.js';
import { GateStep } from './steps/GateStep.js';
import { CompletionStep } from './steps/CompletionStep.js';
import type { TutorialState, TutorialStepId } from './types.js';
import {
  createInitialTutorialState,
  getNextStep,
  getPreviousStep,
  getProgressPercentage,
  STEP_DEFINITIONS,
  canGoBack,
  isLastStep,
} from './types.js';

interface TutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
}

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

  const handlePlanComplete = useCallback((planPath: string) => {
    setState((prev) => ({ ...prev, samplePlanPath: planPath }));
  }, []);

  const handleValidationComplete = useCallback((result: { success: boolean; message: string }) => {
    setState((prev) => ({ ...prev, validationResult: result }));
  }, []);

  const handleGateComplete = useCallback(
    (result: {
      success: boolean;
      checks: Array<{ name: string; passed: boolean; message: string }>;
    }) => {
      setState((prev) => ({ ...prev, gateResult: result }));
    },
    []
  );

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
  const progress = getProgressPercentage(state.currentStep);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Tutorial" subtitle={currentStepDef.title} />

      <Box marginY={1} flexDirection="column">
        <Box marginBottom={1}>
          <Text color={theme.colours.smoke}>
            Step {progress / 20} of 5: {currentStepDef.description}
          </Text>
        </Box>
        <ProgressBar percent={progress} width={40} showPercent={false} />
      </Box>

      <Box flexDirection="column" marginY={1}>
        {state.currentStep === 'intro' && <IntroStep onNext={handleNext} />}
        {state.currentStep === 'plan' && (
          <PlanStep onComplete={handlePlanComplete} samplePlanPath={state.samplePlanPath} />
        )}
        {state.currentStep === 'validate' && state.samplePlanPath && (
          <ValidateStep
            planPath={state.samplePlanPath}
            onComplete={handleValidationComplete}
            validationResult={state.validationResult}
          />
        )}
        {state.currentStep === 'gate' && state.samplePlanPath && (
          <GateStep
            planPath={state.samplePlanPath}
            onComplete={handleGateComplete}
            gateResult={state.gateResult}
          />
        )}
        {state.currentStep === 'completion' && (
          <CompletionStep
            startedAt={state.startedAt}
            onCleanup={handleCleanup}
            onFinish={handleFinish}
          />
        )}
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
