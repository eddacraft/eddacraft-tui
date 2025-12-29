import React from 'react';
import { Box, Text, useInput } from 'ink';
import { theme } from '../../../utils/theme.js';
import { type StepProps, MODE_OPTIONS, FORMAT_OPTIONS } from '../types.js';

export function SummaryStep({
  state,
  context,
  onNext,
  onBack,
  onCancel,
}: StepProps): React.ReactElement {
  useInput((_input, key) => {
    if (key.return) {
      onNext({});
    } else if (key.escape) {
      onCancel();
    } else if (key.leftArrow) {
      onBack();
    }
  });

  const modeLabel =
    MODE_OPTIONS.find((m) => m.value === state.configTemplate)?.label ?? state.configTemplate;
  const formatLabel = FORMAT_OPTIONS.find((f) => f.value === state.format)?.label ?? state.format;

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text bold>Review your configuration:</Text>
      </Box>

      <Box flexDirection="column" marginLeft={2}>
        <SummaryRow label="Project" value={context.environment.projectName ?? '(unnamed)'} />
        <SummaryRow label="Mode" value={modeLabel} />
        <SummaryRow label="Format" value={formatLabel} />
        <SummaryRow label="Plans Directory" value={`./${state.planningDir}/`} />
        <SummaryRow label="Create Example" value={state.createExample ? 'Yes' : 'No'} />
        <SummaryRow
          label="Enabled Checks"
          value={state.enabledChecks.length > 0 ? state.enabledChecks.join(', ') : 'None'}
        />
        {state.enabledChecks.includes('coverage') && (
          <SummaryRow label="Coverage Threshold" value={`${state.coverageThreshold}%`} />
        )}
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.muted}>Files to be created:</Text>
      </Box>

      <Box flexDirection="column" marginLeft={2} marginTop={1}>
        <Text color={theme.colours.info}>{theme.icons.check} .anvilrc</Text>
        <Text color={theme.colours.info}>{theme.icons.check} .anvil/</Text>
        <Text color={theme.colours.info}>
          {theme.icons.check} {state.planningDir}/
        </Text>
        {state.createExample && state.format !== 'skip' && (
          <Text color={theme.colours.info}>
            {theme.icons.check} {state.planningDir}/example-
            {state.format === 'speckit' ? 'spec.md' : 'plan.md'}
          </Text>
        )}
        {context.environment.hasGit && (
          <Text color={theme.colours.info}>{theme.icons.check} .gitignore (updated)</Text>
        )}
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.success}>
          Press Enter to create these files, or Esc to cancel.
        </Text>
      </Box>
    </Box>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }): React.ReactElement {
  return (
    <Box>
      <Box width={20}>
        <Text color={theme.colours.muted}>{label}:</Text>
      </Box>
      <Text color={theme.colours.text}>{value}</Text>
    </Box>
  );
}
