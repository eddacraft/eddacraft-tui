import React, { useState, useCallback, useRef } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../../components/Header.js';
import { ProgressBar } from '../../../components/ProgressBar.js';
import { theme } from '../../../utils/theme.js';
import { TutorialPicker, resolveTutorialKey } from '../components/TutorialPicker.js';
import type { TutorialOption } from '../components/TutorialPicker.js';
import { IntroStep } from './policy-steps/IntroStep.js';
import { CreateDirStep } from './policy-steps/CreateDirStep.js';
import { WritePolicyStep } from './policy-steps/WritePolicyStep.js';
import { TestPolicyStep } from './policy-steps/TestPolicyStep.js';
import { SeePolicyFireStep } from './policy-steps/SeePolicyFireStep.js';
import { CustomiseStep } from './policy-steps/CustomiseStep.js';

export type PolicyStepId =
  | 'intro'
  | 'create-dir'
  | 'write-policy'
  | 'test-policy'
  | 'see-fire'
  | 'customise';

export const POLICY_STEPS: PolicyStepId[] = [
  'intro',
  'create-dir',
  'write-policy',
  'test-policy',
  'see-fire',
  'customise',
];

export interface PolicyStepDef {
  id: PolicyStepId;
  title: string;
  description: string;
}

export const POLICY_STEP_DEFINITIONS: Record<PolicyStepId, PolicyStepDef> = {
  intro: {
    id: 'intro',
    title: 'Introduction',
    description: 'What policies are and what you will learn',
  },
  'create-dir': {
    id: 'create-dir',
    title: 'Create Policy Directory',
    description: 'Set up the .anvil/policies/ directory',
  },
  'write-policy': {
    id: 'write-policy',
    title: 'Write a Policy',
    description: 'Create a max-file-length Rego rule',
  },
  'test-policy': {
    id: 'test-policy',
    title: 'Test the Policy',
    description: 'Validate your policy with OPA',
  },
  'see-fire': {
    id: 'see-fire',
    title: 'See It Fire',
    description: 'Watch the policy catch violations',
  },
  customise: {
    id: 'customise',
    title: 'Customise',
    description: 'Adjust thresholds and explore more ideas',
  },
};

function getStepIndex(step: PolicyStepId): number {
  return POLICY_STEPS.indexOf(step);
}

function getNextStep(current: PolicyStepId): PolicyStepId | null {
  const idx = getStepIndex(current);
  if (idx === POLICY_STEPS.length - 1) return null;
  return POLICY_STEPS[idx + 1];
}

function getPreviousStep(current: PolicyStepId): PolicyStepId | null {
  const idx = getStepIndex(current);
  if (idx === 0) return null;
  return POLICY_STEPS[idx - 1];
}

function canGoBack(current: PolicyStepId): boolean {
  return getStepIndex(current) > 0;
}

function isLastStep(current: PolicyStepId): boolean {
  return current === 'customise';
}

function getProgressPercentage(current: PolicyStepId): number {
  const idx = getStepIndex(current);
  return Math.round(((idx + 1) / POLICY_STEPS.length) * 100);
}

interface PolicyTutorialProps {
  onComplete?: () => void;
  onCleanup?: () => void;
  onSelectTutorial?: (topic: string) => void;
  tutorials?: TutorialOption[];
}

export function PolicyTutorial({
  onComplete,
  onCleanup,
  onSelectTutorial,
  tutorials = [],
}: PolicyTutorialProps): React.ReactElement {
  const { exit } = useApp();
  const [currentStep, setCurrentStep] = useState<PolicyStepId>('intro');
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

      const topic = resolveTutorialKey(tutorials, 'policies', input);
      if (topic) {
        onSelectTutorial?.(topic);
        exit();
      }
    }
  });

  const currentStepDef = POLICY_STEP_DEFINITIONS[currentStep];
  const stepNumber = POLICY_STEPS.indexOf(currentStep) + 1;
  const totalSteps = POLICY_STEPS.length;
  const progress = getProgressPercentage(currentStep);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Policy Tutorial" subtitle={currentStepDef.title} />

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
        {currentStep === 'create-dir' && <CreateDirStep />}
        {currentStep === 'write-policy' && <WritePolicyStep />}
        {currentStep === 'test-policy' && <TestPolicyStep />}
        {currentStep === 'see-fire' && <SeePolicyFireStep />}
        {currentStep === 'customise' && <CustomiseStep />}
      </Box>

      {isLastStep(currentStep) && tutorials.length > 0 && (
        <Box marginY={1}>
          <TutorialPicker tutorials={tutorials} currentTopic="policies" />
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
