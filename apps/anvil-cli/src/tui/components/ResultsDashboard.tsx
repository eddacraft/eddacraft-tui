import React from 'react';
import { Box, Text } from 'ink';
import { Header } from './Header.js';
import { QuickWinsPanel } from './QuickWinsPanel.js';
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/ResultsDashboard.tsx
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/ResultsDashboard.tsx
import { theme } from '../utils/theme.js';
import type { ProjectContext } from '../../services/project-detector.js';
import type { QuickWinsAnalysis } from '../../services/quick-wins.js';
import type { HistoricalAnalysis } from '../../services/historical-analyser.js';
=======
import { StatusBadge } from './StatusBadge.js';
=======
>>>>>>> 2e7659b (fix: Remove unused imports in IFR components):cli/src/tui/components/ResultsDashboard.tsx
import { theme } from '../utils/theme.js';
import type { ProjectContext } from '../../services/project-detector.js';
import type { QuickWinsAnalysis } from '../../services/quick-wins.js';
import type { HistoricalAnalysis } from '../../services/historical-analyzer.js';
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/ResultsDashboard.tsx

export interface InitAnalysisResults {
  /** Project context detection */
  project: ProjectContext;
  /** Analysis results (warnings, errors) */
  analysis?: {
    totalChecks: number;
    passedChecks: number;
    warnings: number;
    errors: number;
    suppressions: number;
  };
  /** Quick wins identification */
  quickWins?: QuickWinsAnalysis;
  /** Historical git analysis */
  historical?: HistoricalAnalysis;
  /** Generated config path */
  configPath?: string;
  /** Sample files analyzed */
  sampleFiles?: {
    analyzed: number;
    total: number;
  };
}

interface ResultsDashboardProps {
  results: InitAnalysisResults;
  focused?: boolean;
}

function MetricsPanel({ results }: { results: InitAnalysisResults }): React.ReactElement {
  const { project, analysis, sampleFiles } = results;

  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={theme.colours.charcoal}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Text bold color={theme.colours.ash}>
        {theme.icons.bullet} PROJECT OVERVIEW
      </Text>

      <Box flexDirection="column" marginTop={0}>
        {/* Framework and size */}
        <Box gap={2}>
          <Box width={18}>
            <Text color={theme.colours.smoke}>Framework:</Text>
          </Box>
          <Text color={theme.colours.ember} bold>
            {project.framework}
          </Text>
        </Box>

        <Box gap={2}>
          <Box width={18}>
            <Text color={theme.colours.smoke}>Project Size:</Text>
          </Box>
          <Text color={theme.colours.ash}>
            {project.size} ({project.fileCount.toLocaleString()} files)
          </Text>
        </Box>

        {project.monorepo !== 'none' && (
          <Box gap={2}>
            <Box width={18}>
              <Text color={theme.colours.smoke}>Monorepo:</Text>
            </Box>
            <Text color={theme.colours.ash}>
              {project.monorepo} ({project.workspacePackages.length} packages)
            </Text>
          </Box>
        )}

        <Box gap={2}>
          <Box width={18}>
            <Text color={theme.colours.smoke}>TypeScript:</Text>
          </Box>
          <Text color={theme.colours.ash}>{project.tsStrictness}</Text>
        </Box>

        {/* Analysis results */}
        {analysis && (
          <>
            <Box marginTop={1}>
              <Text color={theme.colours.smoke} bold>
                Analysis Results:
              </Text>
            </Box>

            {sampleFiles && (
              <Box gap={2}>
                <Box width={18}>
                  <Text color={theme.colours.smoke}>Files Analyzed:</Text>
                </Box>
                <Text color={theme.colours.ash}>
                  {sampleFiles.analyzed} of {sampleFiles.total}
                </Text>
              </Box>
            )}

            <Box gap={2}>
              <Box width={18}>
                <Text color={theme.colours.smoke}>Gate Checks:</Text>
              </Box>
              <Text color={theme.colours.ash}>
                {analysis.passedChecks}/{analysis.totalChecks} passed
              </Text>
              {analysis.passedChecks === analysis.totalChecks && (
                <Text color={theme.colours.success}> {theme.icons.check}</Text>
              )}
            </Box>

            {analysis.warnings > 0 && (
              <Box gap={2}>
                <Box width={18}>
                  <Text color={theme.colours.smoke}>Warnings:</Text>
                </Box>
                <Text color={theme.colours.warning}>
                  {analysis.warnings} {analysis.warnings === 1 ? 'issue' : 'issues'}
                </Text>
              </Box>
            )}

            {analysis.errors > 0 && (
              <Box gap={2}>
                <Box width={18}>
                  <Text color={theme.colours.smoke}>Errors:</Text>
                </Box>
                <Text color={theme.colours.error}>
                  {analysis.errors} {analysis.errors === 1 ? 'issue' : 'issues'}
                </Text>
              </Box>
            )}

            {analysis.suppressions > 0 && (
              <Box gap={2}>
                <Box width={18}>
                  <Text color={theme.colours.smoke}>Suppressions:</Text>
                </Box>
                <Text color={theme.colours.ash}>{analysis.suppressions}</Text>
              </Box>
            )}
          </>
        )}
      </Box>
    </Box>
  );
}

function HistoricalPanel({ analysis }: { analysis: HistoricalAnalysis }): React.ReactElement {
  const hasHistory = analysis.totalCommits > 0;

  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={theme.colours.charcoal}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Text bold color={theme.colours.ash}>
        {theme.icons.bullet} GIT HISTORY INSIGHTS
      </Text>

      {!hasHistory ? (
        <Box marginTop={0}>
          <Text color={theme.colours.smoke}>
            {theme.icons.info} No git history available for analysis
          </Text>
        </Box>
      ) : (
        <Box flexDirection="column" marginTop={0}>
          <Box gap={2}>
            <Box width={20}>
              <Text color={theme.colours.smoke}>Commits Analyzed:</Text>
            </Box>
            <Text color={theme.colours.ash}>{analysis.totalCommits}</Text>
          </Box>

          <Box gap={2}>
            <Box width={20}>
              <Text color={theme.colours.smoke}>Would Have Caught:</Text>
            </Box>
            <Text color={theme.colours.ember} bold>
              {analysis.totalViolations} {analysis.totalViolations === 1 ? 'issue' : 'issues'}
            </Text>
          </Box>

          <Box gap={2}>
            <Box width={20}>
              <Text color={theme.colours.smoke}>Average per Commit:</Text>
            </Box>
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/ResultsDashboard.tsx
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/ResultsDashboard.tsx
            <Text color={theme.colours.ash}>{analysis.avgViolationsPerCommit.toFixed(1)}</Text>
=======
            <Text color={theme.colours.ash}>
              {analysis.avgViolationsPerCommit.toFixed(1)}
            </Text>
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/ResultsDashboard.tsx
=======
            <Text color={theme.colours.ash}>{analysis.avgViolationsPerCommit.toFixed(1)}</Text>
>>>>>>> 177f91e (style: Apply Prettier formatting to IFR files):cli/src/tui/components/ResultsDashboard.tsx
          </Box>

          {analysis.patternOccurrences.length > 0 && (
            <>
              <Box marginTop={1}>
                <Text color={theme.colours.smoke} bold>
                  Most Common Patterns:
                </Text>
              </Box>
              {analysis.patternOccurrences.slice(0, 3).map((pattern) => (
                <Box key={pattern.patternId} gap={2}>
                  <Box width={20}>
                    <Text color={theme.colours.smoke}>{pattern.patternName}:</Text>
                  </Box>
                  <Text color={theme.colours.ash}>
                    {pattern.count} {pattern.count === 1 ? 'occurrence' : 'occurrences'}
                  </Text>
                </Box>
              ))}
            </>
          )}

          <Box marginTop={1}>
            <Text color={theme.colours.smoke}>
              {theme.icons.info} Anvil would have prevented{' '}
              <Text color={theme.colours.ember} bold>
                {Math.round((analysis.totalViolations / analysis.totalCommits) * 10) / 10}
              </Text>{' '}
              issues per commit on average
            </Text>
          </Box>
        </Box>
      )}
    </Box>
  );
}

function NavigationPanel(): React.ReactElement {
  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={theme.colours.ember}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Text bold color={theme.colours.ember}>
        {theme.icons.arrow} NEXT STEPS
      </Text>

      <Box flexDirection="column" marginTop={0}>
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Review generated configuration:{' '}
          <Text color={theme.colours.ember}>.anvilrc</Text>
        </Text>
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Run initial check:{' '}
          <Text color={theme.colours.ember}>anvil gate check</Text>
        </Text>
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Apply quick wins:{' '}
          <Text color={theme.colours.ember}>anvil suppress --batch</Text>
        </Text>
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Set up git hooks:{' '}
          <Text color={theme.colours.ember}>anvil git install</Text>
        </Text>
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Explore results:{' '}
          <Text color={theme.colours.ember}>anvil gate explore</Text>
        </Text>
      </Box>

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          {theme.icons.info} Get help: <Text color={theme.colours.ember}>anvil --help</Text>
        </Text>
      </Box>
    </Box>
  );
}

<<<<<<< HEAD:apps/anvil-cli/src/tui/components/ResultsDashboard.tsx
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/ResultsDashboard.tsx
=======
>>>>>>> 177f91e (style: Apply Prettier formatting to IFR files):cli/src/tui/components/ResultsDashboard.tsx
export function ResultsDashboard({
  results,
  focused = false,
}: ResultsDashboardProps): React.ReactElement {
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/ResultsDashboard.tsx
=======
export function ResultsDashboard({ results, focused = false }: ResultsDashboardProps): React.ReactElement {
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/ResultsDashboard.tsx
=======
>>>>>>> 177f91e (style: Apply Prettier formatting to IFR files):cli/src/tui/components/ResultsDashboard.tsx
  const { project, quickWins, historical } = results;

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header
        title="Anvil Initialization Complete"
        subtitle={`${project.projectRoot} • ${project.framework} project`}
      />

      <Box flexDirection="column" marginTop={1}>
        <Box>
          <Text color={theme.colours.success} bold>
            {theme.icons.check} Configuration generated and analysis complete!
          </Text>
        </Box>
      </Box>

      {/* Metrics panel */}
      <MetricsPanel results={results} />

      {/* Quick wins panel */}
      {quickWins && <QuickWinsPanel analysis={quickWins} focused={focused} />}

      {/* Historical analysis panel */}
      {historical && <HistoricalPanel analysis={historical} />}

      {/* Navigation panel */}
      <NavigationPanel />

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          Press <Text color={theme.colours.ember}>Enter</Text> to continue or{' '}
          <Text color={theme.colours.ember}>q</Text> to quit
        </Text>
      </Box>
    </Box>
  );
}
