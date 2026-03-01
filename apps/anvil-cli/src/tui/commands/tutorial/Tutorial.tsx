import React, { useState, useCallback } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../components/Header.js';
import { ProgressBar } from '../../components/ProgressBar.js';
import { theme } from '../../utils/theme.js';
import type { TutorialState, TutorialStepId, ScanResults } from './types.js';
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
import { ScanStep } from './steps/ScanStep.js';
import { WatchStep } from './steps/WatchStep.js';
import { FixStep } from './steps/FixStep.js';
import { NextStepsStep } from './steps/NextStepsStep.js';

import { resolveTutorialKey } from './components/TutorialPicker.js';
import type { TutorialOption } from './components/TutorialPicker.js';

interface TutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
  onSelectTutorial?: (topic: string) => void;
  tutorials?: TutorialOption[];
  completedTopics?: string[];
  /** The topic this tutorial instance covers (default: 'core') */
  currentTopic?: string;
}

export function Tutorial({
  onComplete,
  onCleanup,
  onSelectTutorial,
  tutorials = [],
  completedTopics = [],
  currentTopic = 'core',
}: TutorialProps): React.ReactElement {
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

  const handleScanComplete = useCallback((results: ScanResults) => {
    setState((prev) => ({ ...prev, scanResults: results }));
  }, []);

  const handleWatchComplete = useCallback(() => {
    setState((prev) => ({ ...prev, watchTriggered: true }));
  }, []);

  const handleFixComplete = useCallback(() => {
    setState((prev) => ({ ...prev, fixConfirmed: true }));
  }, []);

  const handleCleanupConfirm = useCallback(() => {
    setState((prev) => {
      if (!prev.cleanupConfirming) {
        return { ...prev, cleanupConfirming: true };
      }
      try {
        onCleanup?.();
        return { ...prev, cleanupRequested: true, cleanupConfirming: false };
      } catch {
        return { ...prev, cleanupConfirming: false };
      }
    });
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
      if (input === 'c' && !state.cleanupRequested) {
        handleCleanupConfirm();
        return;
      }

      const topic = resolveTutorialKey(tutorials, currentTopic, input, completedTopics);
      if (topic) {
        onSelectTutorial?.(topic);
        exit();
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
        {state.currentStep === 'scan' && (
          <ScanStep onComplete={handleScanComplete} scanResults={state.scanResults} />
        )}
        {state.currentStep === 'watch' && (
          <WatchStep
            onComplete={handleWatchComplete}
            watchTriggered={state.watchTriggered}
            scanResults={state.scanResults}
          />
        )}
        {state.currentStep === 'fix' && (
          <FixStep
            scanResults={state.scanResults}
            fixConfirmed={state.fixConfirmed}
            onComplete={handleFixComplete}
          />
        )}
        {state.currentStep === 'next-steps' && (
          <NextStepsStep
            startedAt={state.startedAt}
            scanResults={state.scanResults}
            cleanupConfirming={state.cleanupConfirming}
            cleanupRequested={state.cleanupRequested}
            tutorials={tutorials}
            completedTopics={completedTopics}
            currentTopic={currentTopic}
            onFinish={handleFinish}
          />
        )}
      </Box>

      <Box marginTop={1}>
        <Text color={theme.colours.ash}>
          {canGoBack(state.currentStep) && (
            <>
              <Text color={theme.colours.ember}>{theme.icons.backArrow}</Text>
              {' back  '}
            </>
          )}
          {!isLastStep(state.currentStep) && 'Enter next  '}
          {isLastStep(state.currentStep) && !state.cleanupRequested && !state.cleanupConfirming && (
            <>
              <Text color={theme.colours.ember}>c</Text>
              {' clean up tutorial files  '}
            </>
          )}
          <Text color={theme.colours.ember}>q</Text>
          {' quit'}
        </Text>
      </Box>
    </Box>
  );
}
