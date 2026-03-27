import React, { useState, useEffect } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../components/Header.js';
import { HooksPanel } from './panels/HooksPanel.js';
import { ProfilePanel } from './panels/ProfilePanel.js';
import { ResultsPanel } from './panels/ResultsPanel.js';
import { theme } from '../../utils/theme.js';
import type { StatusData, PanelId } from './types.js';
import { getNextPanel, getPreviousPanel } from './types.js';

interface StatusDashboardProps {
  data: StatusData;
  onQuit?: () => void;
}

export function StatusDashboard({ data, onQuit }: StatusDashboardProps): React.ReactElement {
  const { exit } = useApp();
  const [focusedPanel, setFocusedPanel] = useState<PanelId>('hooks');

  useInput((input, key) => {
    if (input === 'q' || (key.ctrl && input === 'c')) {
      onQuit?.();
      exit();
      return;
    }

    if (input === 'j' || key.downArrow) {
      setFocusedPanel(getNextPanel(focusedPanel));
    } else if (input === 'k' || key.upArrow) {
      setFocusedPanel(getPreviousPanel(focusedPanel));
    }
  });

  useEffect(() => {
    return () => {
      onQuit?.();
    };
  }, [onQuit]);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Status" subtitle={data.projectName ?? data.projectRoot} />

      <Box flexDirection="column" marginTop={1}>
        <HooksPanel data={data.hooks} focused={focusedPanel === 'hooks'} />
        <ProfilePanel data={data.profile} focused={focusedPanel === 'profile'} />
        <ResultsPanel data={data.recent} focused={focusedPanel === 'results'} />
      </Box>

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          {theme.icons.info} j/k or arrows to navigate {theme.icons.bullet} q to quit
        </Text>
      </Box>
    </Box>
  );
}
