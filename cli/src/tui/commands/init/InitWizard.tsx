import React, { useState, useCallback } from 'react';
import { Box, Text, useApp } from 'ink';
import { Header, Container } from '../../components/index.js';
import { theme } from '../../utils/theme.js';
import {
  type WizardState,
  type WizardContext,
  type WizardStep,
  getDefaultState,
  getNextStep,
  getPreviousStep,
  getStepIndex,
  WIZARD_STEPS,
} from './types.js';
import { ModeStep } from './steps/ModeStep.js';
import { FormatStep } from './steps/FormatStep.js';
import { DirectoryStep } from './steps/DirectoryStep.js';
import { ChecksStep } from './steps/ChecksStep.js';
import { SummaryStep } from './steps/SummaryStep.js';

interface InitWizardProps {
  context: WizardContext;
  onComplete: (state: WizardState) => void;
  onCancel: () => void;
}

export function InitWizard({ context, onComplete, onCancel }: InitWizardProps): React.ReactElement {
  const { exit } = useApp();
  const [state, setState] = useState<WizardState>(() => {
    const defaults = getDefaultState();
    return {
      ...defaults,
      enabledChecks: [...context.recommendedChecks],
    };
  });

  const handleNext = useCallback(
    (updates: Partial<WizardState>) => {
      setState((prev) => {
        const newState = { ...prev, ...updates };
        const nextStep = getNextStep(prev.currentStep);

        if (nextStep) {
          return { ...newState, currentStep: nextStep };
        }

        onComplete(newState);
        return newState;
      });
    },
    [onComplete]
  );

  const handleBack = useCallback(() => {
    setState((prev) => {
      const prevStep = getPreviousStep(prev.currentStep);
      if (prevStep) {
        return { ...prev, currentStep: prevStep };
      }
      return prev;
    });
  }, []);

  const handleCancel = useCallback(() => {
    onCancel();
    exit();
  }, [onCancel, exit]);

  const stepProps = {
    state,
    context,
    onNext: handleNext,
    onBack: handleBack,
    onCancel: handleCancel,
  };

  const currentStepIndex = getStepIndex(state.currentStep);
  const totalSteps = WIZARD_STEPS.length;

  return (
    <Box flexDirection="column">
      <Header title="Anvil Setup" subtitle="Configure your project" />

      <Box marginBottom={1}>
        <Text color={theme.colours.muted}>
          Step {currentStepIndex + 1} of {totalSteps}
        </Text>
        <Text color={theme.colours.muted}> · </Text>
        <Text color={theme.colours.info}>{getStepTitle(state.currentStep)}</Text>
      </Box>

      <Container>{renderStep(state.currentStep, stepProps)}</Container>

      <Box marginTop={1}>
        <Text color={theme.colours.muted}>
          {currentStepIndex > 0 ? '[←] Back  ' : ''}
          [Enter] Continue · [Esc] Cancel
        </Text>
      </Box>
    </Box>
  );
}

function getStepTitle(step: WizardStep): string {
  const titles: Record<WizardStep, string> = {
    mode: 'Configuration Mode',
    format: 'Planning Format',
    directory: 'Directory Setup',
    checks: 'Quality Checks',
    summary: 'Review & Confirm',
  };
  return titles[step];
}

function renderStep(
  step: WizardStep,
  props: {
    state: WizardState;
    context: WizardContext;
    onNext: (updates: Partial<WizardState>) => void;
    onBack: () => void;
    onCancel: () => void;
  }
): React.ReactElement {
  switch (step) {
    case 'mode':
      return <ModeStep {...props} />;
    case 'format':
      return <FormatStep {...props} />;
    case 'directory':
      return <DirectoryStep {...props} />;
    case 'checks':
      return <ChecksStep {...props} />;
    case 'summary':
      return <SummaryStep {...props} />;
  }
}
