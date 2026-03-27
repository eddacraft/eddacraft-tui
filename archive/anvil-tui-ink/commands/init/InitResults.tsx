import React, { useEffect } from 'react';
import { useInput, useApp } from 'ink';
import { ResultsDashboard, type InitAnalysisResults } from '../../components/ResultsDashboard.js';

interface InitResultsProps {
  results: InitAnalysisResults;
  onComplete?: () => void;
  onQuit?: () => void;
}

/**
 * InitResults command - displays analysis results after init
 *
 * Shows comprehensive dashboard with:
 * - Project metrics and configuration
 * - Quick wins identification
 * - Historical git analysis
 * - Next steps and navigation
 *
 * Input handling:
 * - Enter: Continue/complete
 * - q/Ctrl+C: Quit
 */
export function InitResults({ results, onComplete, onQuit }: InitResultsProps): React.ReactElement {
  const { exit } = useApp();

  useInput((input, key) => {
    if (input === 'q' || (key.ctrl && input === 'c')) {
      onQuit?.();
      exit();
      return;
    }

    if (key.return) {
      onComplete?.();
      exit();
      return;
    }
  });

  useEffect(() => {
    return () => {
      // Cleanup on unmount
      onQuit?.();
    };
  }, [onQuit]);

  return <ResultsDashboard results={results} focused={false} />;
}
