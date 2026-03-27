import React from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../components/Header.js';
import { theme } from '../../utils/theme.js';
import type { RepoScanResult } from '../../../services/repo-scanner.js';

interface AuditResultsProps {
  result: RepoScanResult;
  onComplete?: () => void;
  onQuit?: () => void;
}

function ProjectPanel({ result }: { result: RepoScanResult }): React.ReactElement {
  const { project } = result;

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
      </Box>
    </Box>
  );
}

function CurrentIssuesPanel({ result }: { result: RepoScanResult }): React.ReactElement {
  const { currentIssues } = result;
  const hasIssues = currentIssues.totalWarnings > 0;

  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={hasIssues ? theme.colours.molten : theme.colours.charcoal}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Text bold color={hasIssues ? theme.colours.molten : theme.colours.ash}>
        {theme.icons.bullet} CURRENT ISSUES
      </Text>

      <Box flexDirection="column" marginTop={0}>
        <Box gap={2}>
          <Box width={18}>
            <Text color={theme.colours.smoke}>Files Scanned:</Text>
          </Box>
          <Text color={theme.colours.ash}>{currentIssues.filesScanned}</Text>
        </Box>

        <Box gap={2}>
          <Box width={18}>
            <Text color={theme.colours.smoke}>Checks Run:</Text>
          </Box>
          <Text color={theme.colours.ash}>{currentIssues.checksRun.join(', ')}</Text>
        </Box>

        {!hasIssues ? (
          <Box marginTop={1}>
            <Text color={theme.colours.success} bold>
              {theme.icons.check} No issues found!
            </Text>
          </Box>
        ) : (
          <>
            <Box gap={2} marginTop={1}>
              <Box width={18}>
                <Text color={theme.colours.smoke}>Total Issues:</Text>
              </Box>
              <Text color={theme.colours.ember} bold>
                {currentIssues.totalWarnings}
              </Text>
            </Box>

            {currentIssues.bySeverity.errors > 0 && (
              <Box gap={2}>
                <Box width={18}>
                  <Text color={theme.colours.smoke}>{theme.icons.error} Errors:</Text>
                </Box>
                <Text color={theme.colours.slag} bold>
                  {currentIssues.bySeverity.errors}
                </Text>
              </Box>
            )}

            {currentIssues.bySeverity.warnings > 0 && (
              <Box gap={2}>
                <Box width={18}>
                  <Text color={theme.colours.smoke}>{theme.icons.warning} Warnings:</Text>
                </Box>
                <Text color={theme.colours.molten}>{currentIssues.bySeverity.warnings}</Text>
              </Box>
            )}

            {currentIssues.bySeverity.info > 0 && (
              <Box gap={2}>
                <Box width={18}>
                  <Text color={theme.colours.smoke}>{theme.icons.info} Info:</Text>
                </Box>
                <Text color={theme.colours.ash}>{currentIssues.bySeverity.info}</Text>
              </Box>
            )}

            {currentIssues.topIssues.length > 0 && (
              <>
                <Box marginTop={1}>
                  <Text color={theme.colours.smoke} bold>
                    Top Issues:
                  </Text>
                </Box>
                {currentIssues.topIssues.slice(0, 5).map((issue) => (
                  <Box key={issue.id} gap={2}>
                    <Box width={18}>
                      <Text color={theme.colours.smoke}>[{issue.id}]</Text>
                    </Box>
                    <Text color={theme.colours.ash}>
                      {issue.title}: <Text bold>{issue.count}</Text>
                    </Text>
                  </Box>
                ))}
              </>
            )}
          </>
        )}
      </Box>
    </Box>
  );
}

function HistoricalPanel({ result }: { result: RepoScanResult }): React.ReactElement {
  const { historical } = result;
  const hasHistory = historical.totalCommits > 0;

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
            <Text color={theme.colours.ash}>{historical.totalCommits}</Text>
          </Box>

          <Box gap={2}>
            <Box width={20}>
              <Text color={theme.colours.smoke}>Would Have Caught:</Text>
            </Box>
            <Text color={theme.colours.ember} bold>
              {historical.totalViolations} {historical.totalViolations === 1 ? 'issue' : 'issues'}
            </Text>
          </Box>

          <Box gap={2}>
            <Box width={20}>
              <Text color={theme.colours.smoke}>Average per Commit:</Text>
            </Box>
            <Text color={theme.colours.ash}>{historical.avgViolationsPerCommit.toFixed(1)}</Text>
          </Box>

          {historical.patternOccurrences.length > 0 && (
            <>
              <Box marginTop={1}>
                <Text color={theme.colours.smoke} bold>
                  Most Common Patterns:
                </Text>
              </Box>
              {historical.patternOccurrences.slice(0, 3).map((pattern) => (
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

          {historical.totalViolations > 0 && (
            <Box marginTop={1}>
              <Text color={theme.colours.smoke}>
                {theme.icons.info} Anvil would have prevented{' '}
                <Text color={theme.colours.ember} bold>
                  {(historical.totalViolations / historical.totalCommits).toFixed(1)}
                </Text>{' '}
                issues per commit on average
              </Text>
            </Box>
          )}
        </Box>
      )}
    </Box>
  );
}

function NextStepsPanel({ result }: { result: RepoScanResult }): React.ReactElement {
  const hasIssues = result.currentIssues.totalWarnings > 0;
  const hasBlockingIssues = result.currentIssues.hasBlockingWarnings;

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
        {hasBlockingIssues && (
          <Text color={theme.colours.slag}>
            {theme.icons.bullet} Fix blocking errors:{' '}
            <Text color={theme.colours.ember}>anvil check --all --verbose</Text>
          </Text>
        )}
        {hasIssues && !hasBlockingIssues && (
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Review issues in detail:{' '}
            <Text color={theme.colours.ember}>anvil check --all --verbose</Text>
          </Text>
        )}
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Set up continuous monitoring:{' '}
          <Text color={theme.colours.ember}>anvil watch</Text>
        </Text>
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Install git hooks:{' '}
          <Text color={theme.colours.ember}>anvil hooks install</Text>
        </Text>
        <Text color={theme.colours.ash}>
          {theme.icons.bullet} Run full gate check:{' '}
          <Text color={theme.colours.ember}>anvil gate</Text>
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

/**
 * AuditResults command - displays comprehensive repository audit results
 *
 * Shows:
 * - Project overview (framework, size, etc.)
 * - Current issues found in the codebase
 * - Historical git analysis (what Anvil would have caught)
 * - Next steps
 *
 * Input handling:
 * - Enter: Continue/complete
 * - q/Ctrl+C: Quit
 */
export function AuditResults({
  result,
  onComplete,
  onQuit,
}: AuditResultsProps): React.ReactElement {
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

  const statusText = result.currentIssues.hasBlockingWarnings
    ? 'Blocking issues found'
    : result.currentIssues.totalWarnings > 0
      ? 'Issues found (non-blocking)'
      : 'No issues found';

  const statusColor = result.currentIssues.hasBlockingWarnings
    ? theme.colours.slag
    : result.currentIssues.totalWarnings > 0
      ? theme.colours.molten
      : theme.colours.success;

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header
        title="Repository Scan Complete"
        subtitle={`${result.project.projectRoot} - Scanned in ${result.totalDurationMs}ms`}
      />

      <Box flexDirection="column" marginTop={1}>
        <Box>
          <Text color={statusColor} bold>
            {result.currentIssues.hasBlockingWarnings
              ? theme.icons.error
              : result.currentIssues.totalWarnings > 0
                ? theme.icons.warning
                : theme.icons.check}{' '}
            {statusText}
          </Text>
        </Box>
      </Box>

      {/* Project overview panel */}
      <ProjectPanel result={result} />

      {/* Current issues panel */}
      <CurrentIssuesPanel result={result} />

      {/* Historical analysis panel */}
      <HistoricalPanel result={result} />

      {/* Next steps panel */}
      <NextStepsPanel result={result} />

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          Press <Text color={theme.colours.ember}>Enter</Text> to continue or{' '}
          <Text color={theme.colours.ember}>q</Text> to quit
        </Text>
      </Box>
    </Box>
  );
}
