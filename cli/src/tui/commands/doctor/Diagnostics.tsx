import React, { useState, useEffect } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import Spinner from 'ink-spinner';
import { Header } from '../../components/Header.js';
import { theme } from '../../utils/theme.js';
import type {
  DiagnosticCheck,
  DiagnosticResult,
  DiagnosticsSummary,
  DiagnosticsData,
  DiagnosticContext,
} from './types.js';
import { calculateSummary } from './types.js';

interface DiagnosticsProps {
  checks: DiagnosticCheck[];
  context: DiagnosticContext;
  autoFix?: boolean;
  onComplete?: (data: DiagnosticsData) => void;
  onQuit?: () => void;
}

type Phase = 'running' | 'complete' | 'fixing';

function getStatusIcon(status: DiagnosticResult['status']): string {
  switch (status) {
    case 'pass':
      return theme.icons.success;
    case 'warn':
      return theme.icons.warning;
    case 'fail':
      return theme.icons.error;
    case 'skip':
      return theme.icons.bullet;
  }
}

function getStatusColour(status: DiagnosticResult['status']): string {
  switch (status) {
    case 'pass':
      return theme.colours.steel;
    case 'warn':
      return theme.colours.molten;
    case 'fail':
      return theme.colours.slag;
    case 'skip':
      return theme.colours.smoke;
  }
}

function ResultRow({ result }: { result: DiagnosticResult }): React.ReactElement {
  return (
    <Box>
      <Text color={getStatusColour(result.status)}>{getStatusIcon(result.status)} </Text>
      <Text color={theme.colours.ash}>{result.name}: </Text>
      <Text color={getStatusColour(result.status)}>{result.message}</Text>
      {result.fixable && result.status !== 'pass' && (
        <Text color={theme.colours.smoke}> (fixable)</Text>
      )}
    </Box>
  );
}

function SummaryBox({ summary }: { summary: DiagnosticsSummary }): React.ReactElement {
  const statusText = summary.healthy ? 'Healthy' : 'Issues Found';
  const statusColour = summary.healthy ? theme.colours.steel : theme.colours.slag;

  return (
    <Box
      flexDirection="column"
      marginTop={1}
      borderStyle={summary.healthy ? 'single' : 'double'}
      borderColor={statusColour}
      paddingX={1}
    >
      <Text bold color={statusColour}>
        {summary.healthy ? theme.icons.success : theme.icons.error} {statusText}
      </Text>
      <Box marginTop={1}>
        <Text color={theme.colours.steel}>{summary.passed} passed</Text>
        {summary.warnings > 0 && (
          <Text color={theme.colours.molten}>
            {' '}
            {theme.icons.bullet} {summary.warnings} warnings
          </Text>
        )}
        {summary.failed > 0 && (
          <Text color={theme.colours.slag}>
            {' '}
            {theme.icons.bullet} {summary.failed} failed
          </Text>
        )}
        {summary.skipped > 0 && (
          <Text color={theme.colours.smoke}>
            {' '}
            {theme.icons.bullet} {summary.skipped} skipped
          </Text>
        )}
      </Box>
      {summary.fixable > 0 && (
        <Text color={theme.colours.ash}>
          {theme.icons.info} {summary.fixable} issue(s) can be auto-fixed with --fix
        </Text>
      )}
    </Box>
  );
}

export function Diagnostics({
  checks,
  context,
  autoFix = false,
  onComplete,
  onQuit,
}: DiagnosticsProps): React.ReactElement {
  const { exit } = useApp();
  const [phase, setPhase] = useState<Phase>('running');
  const [currentCheck, setCurrentCheck] = useState(0);
  const [results, setResults] = useState<DiagnosticResult[]>([]);
  const [fixingIndex, setFixingIndex] = useState(-1);

  useInput((input, key) => {
    if (input === 'q' || (key.ctrl && input === 'c')) {
      onQuit?.();
      exit();
    }
  });

  useEffect(() => {
    if (phase !== 'running') return;

    const runChecks = async () => {
      const allResults: DiagnosticResult[] = [];

      for (let i = 0; i < checks.length; i++) {
        setCurrentCheck(i);
        const check = checks[i];
        const result = await check.run(context);
        allResults.push(result);
        setResults([...allResults]);
      }

      if (autoFix) {
        setPhase('fixing');
        setFixingIndex(0);
      } else {
        setPhase('complete');
        onComplete?.({
          projectRoot: context.projectRoot,
          results: allResults,
          summary: calculateSummary(allResults),
          ranAt: new Date(),
        });
      }
    };

    runChecks();
  }, [phase, checks, context, autoFix, onComplete]);

  useEffect(() => {
    if (phase !== 'fixing' || fixingIndex < 0) return;

    const applyFixes = async () => {
      const updatedResults = [...results];

      for (let i = fixingIndex; i < results.length; i++) {
        const result = updatedResults[i];
        if (result.fixable && result.status !== 'pass') {
          const check = checks.find((c) => c.id === result.checkId);
          if (check?.fix) {
            setFixingIndex(i);
            const fixResult = await check.fix(context);
            if (fixResult.success) {
              updatedResults[i] = {
                ...result,
                status: 'pass',
                message: `Fixed: ${fixResult.message}`,
                fixable: false,
              };
              setResults([...updatedResults]);
            }
          }
        }
      }

      setPhase('complete');
      onComplete?.({
        projectRoot: context.projectRoot,
        results: updatedResults,
        summary: calculateSummary(updatedResults),
        ranAt: new Date(),
      });
    };

    applyFixes();
  }, [phase, fixingIndex, results, checks, context, onComplete]);

  const summary = calculateSummary(results);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Doctor" subtitle="Diagnostics" />

      <Box flexDirection="column" marginTop={1}>
        {results.map((result, _idx) => (
          <ResultRow key={result.checkId} result={result} />
        ))}

        {phase === 'running' && currentCheck < checks.length && (
          <Box>
            <Text color={theme.colours.ember}>
              <Spinner type="dots" /> Running: {checks[currentCheck].name}...
            </Text>
          </Box>
        )}

        {phase === 'fixing' && fixingIndex >= 0 && fixingIndex < results.length && (
          <Box marginTop={1}>
            <Text color={theme.colours.ember}>
              <Spinner type="dots" /> Fixing: {results[fixingIndex].name}...
            </Text>
          </Box>
        )}
      </Box>

      {phase === 'complete' && <SummaryBox summary={summary} />}

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>{theme.icons.info} q to quit</Text>
      </Box>
    </Box>
  );
}
