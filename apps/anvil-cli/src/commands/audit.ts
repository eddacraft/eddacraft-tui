import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { createDebugger } from '@eddacraft/anvil-core';
import { RepoScanner, type RepoScanResult } from '../services/repo-scanner.js';
import { success, error, info } from '../utils/output.js';

const log = createDebugger('cli');
import { getWorkspaceRoot } from '../utils/file-io.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { AuditResults } from '../tui/commands/audit/AuditResults.js';

/**
 * JSON output format for audit results
 */
interface JSONAuditOutput {
  version: '1.0.0';
  timestamp: string;
  project: {
    framework: string;
    size: string;
    fileCount: number;
    monorepo: string;
    tsStrictness: string;
  };
  currentIssues: {
    filesScanned: number;
    totalWarnings: number;
    bySeverity: {
      errors: number;
      warnings: number;
      info: number;
    };
    byCategory: Record<string, number>;
    topIssues: Array<{
      id: string;
      title: string;
      count: number;
      severity: string;
    }>;
    hasBlockingWarnings: boolean;
    executionTimeMs: number;
    checksRun: string[];
  };
  historical: {
    totalCommits: number;
    totalViolations: number;
    avgViolationsPerCommit: number;
    patternOccurrences: Array<{
      patternId: string;
      patternName: string;
      count: number;
    }>;
    dateRange: {
      from: string;
      to: string;
    };
  };
  totalDurationMs: number;
}

interface ScanOptions {
  json?: boolean;
  tui?: boolean;
  verbose?: boolean;
  noCache?: boolean;
  skipHistorical?: boolean;
  daysBack?: number;
  maxCommits?: number;
}

function formatResultsJSON(result: RepoScanResult): void {
  const output: JSONAuditOutput = {
    version: '1.0.0',
    timestamp: result.timestamp.toISOString(),
    project: {
      framework: result.project.framework,
      size: result.project.size,
      fileCount: result.project.fileCount,
      monorepo: result.project.monorepo,
      tsStrictness: result.project.tsStrictness,
    },
    currentIssues: {
      filesScanned: result.currentIssues.filesScanned,
      totalWarnings: result.currentIssues.totalWarnings,
      bySeverity: result.currentIssues.bySeverity,
      byCategory: result.currentIssues.byCategory,
      topIssues: result.currentIssues.topIssues,
      hasBlockingWarnings: result.currentIssues.hasBlockingWarnings,
      executionTimeMs: result.currentIssues.executionTimeMs,
      checksRun: result.currentIssues.checksRun,
    },
    historical: {
      totalCommits: result.historical.totalCommits,
      totalViolations: result.historical.totalViolations,
      avgViolationsPerCommit: result.historical.avgViolationsPerCommit,
      patternOccurrences: result.historical.patternOccurrences.map((p) => ({
        patternId: p.patternId,
        patternName: p.patternName,
        count: p.count,
      })),
      dateRange: {
        from: result.historical.dateRange.from.toISOString(),
        to: result.historical.dateRange.to.toISOString(),
      },
    },
    totalDurationMs: result.totalDurationMs,
  };

  console.log(JSON.stringify(output, null, 2));
}

function formatResultsHuman(result: RepoScanResult, verbose: boolean): void {
  console.log('');
  console.log(chalk.bold.cyan('Repository Scan Results'));
  console.log(chalk.dim('═'.repeat(50)));
  console.log('');

  // Project overview
  console.log(chalk.bold('Project Overview'));
  console.log(chalk.dim('─'.repeat(30)));
  console.log(`  Framework:    ${chalk.yellow(result.project.framework)}`);
  console.log(
    `  Size:         ${result.project.size} (${result.project.fileCount.toLocaleString()} files)`
  );
  if (result.project.monorepo !== 'none') {
    console.log(
      `  Monorepo:     ${result.project.monorepo} (${result.project.workspacePackages.length} packages)`
    );
  }
  console.log(`  TypeScript:   ${result.project.tsStrictness}`);
  console.log('');

  // Current issues
  console.log(chalk.bold('Current Issues'));
  console.log(chalk.dim('─'.repeat(30)));
  console.log(`  Files scanned:  ${result.currentIssues.filesScanned}`);
  console.log(`  Checks run:     ${result.currentIssues.checksRun.join(', ')}`);
  console.log('');

  if (result.currentIssues.totalWarnings === 0) {
    console.log(chalk.green('  ✓ No issues found!'));
  } else {
    const { bySeverity } = result.currentIssues;
    console.log(`  Total issues:   ${chalk.bold(result.currentIssues.totalWarnings)}`);
    if (bySeverity.errors > 0) {
      console.log(`    ${chalk.red('✗')} Errors:    ${chalk.red(bySeverity.errors)}`);
    }
    if (bySeverity.warnings > 0) {
      console.log(`    ${chalk.yellow('⚠')} Warnings:  ${chalk.yellow(bySeverity.warnings)}`);
    }
    if (bySeverity.info > 0) {
      console.log(`    ${chalk.blue('ℹ')} Info:      ${chalk.blue(bySeverity.info)}`);
    }

    if (result.currentIssues.topIssues.length > 0) {
      console.log('');
      console.log(chalk.dim('  Top issues:'));
      const issuesToShow = verbose
        ? result.currentIssues.topIssues
        : result.currentIssues.topIssues.slice(0, 5);
      for (const issue of issuesToShow) {
        const severityIcon =
          issue.severity === 'error'
            ? chalk.red('✗')
            : issue.severity === 'warning'
              ? chalk.yellow('⚠')
              : chalk.blue('ℹ');
        console.log(
          `    ${severityIcon} [${chalk.dim(issue.id)}] ${issue.title}: ${chalk.bold(issue.count)}`
        );
      }
    }
  }
  console.log('');

  // Historical analysis
  if (result.historical.totalCommits > 0) {
    console.log(chalk.bold('Historical Analysis'));
    console.log(chalk.dim('─'.repeat(30)));
    console.log(`  Commits analysed:     ${result.historical.totalCommits}`);
    console.log(
      `  Anvil would have caught: ${chalk.yellow.bold(result.historical.totalViolations)} issues`
    );
    console.log(`  Average per commit:   ${result.historical.avgViolationsPerCommit.toFixed(1)}`);

    if (result.historical.patternOccurrences.length > 0) {
      console.log('');
      console.log(chalk.dim('  Most common patterns:'));
      const patternsToShow = verbose
        ? result.historical.patternOccurrences
        : result.historical.patternOccurrences.slice(0, 3);
      for (const pattern of patternsToShow) {
        console.log(`    • ${pattern.patternName}: ${chalk.bold(pattern.count)}`);
      }
    }

    if (result.historical.totalViolations > 0) {
      console.log('');
      const avgPerCommit = result.historical.totalViolations / result.historical.totalCommits;
      console.log(
        chalk.dim(
          `  💡 Anvil would have prevented ~${chalk.yellow(avgPerCommit.toFixed(1))} issues per commit on average`
        )
      );
    }
  } else {
    console.log(chalk.dim('Historical Analysis: No git history available'));
  }
  console.log('');

  // Timing
  console.log(chalk.dim(`Scan completed in ${result.totalDurationMs}ms`));
  console.log('');

  // Next steps
  if (result.currentIssues.totalWarnings > 0 || result.historical.totalViolations > 0) {
    console.log(chalk.bold('Next Steps'));
    console.log(chalk.dim('─'.repeat(30)));
    if (result.currentIssues.hasBlockingWarnings) {
      console.log(
        `  ${chalk.red('1.')} Fix blocking errors: ${chalk.cyan('anvil check --all --verbose')}`
      );
    }
    console.log(
      `  ${chalk.dim('•')} Review issues in detail: ${chalk.cyan('anvil check --all --verbose')}`
    );
    console.log(`  ${chalk.dim('•')} Set up continuous monitoring: ${chalk.cyan('anvil watch')}`);
    console.log(`  ${chalk.dim('•')} Install git hooks: ${chalk.cyan('anvil hooks install')}`);
    console.log('');
  }
}

export function createAuditCommand(): Command {
  const command = new Command('audit');

  command
    .description(
      'Audit repository health: current issues, project overview, and historical analysis'
    )
    .option('--json', 'Output results as JSON')
    .option('--tui', 'Force TUI mode')
    .option('--no-tui', 'Disable TUI mode')
    .option('-v, --verbose', 'Show detailed output')
    .option('--no-cache', 'Disable caching')
    .option('--skip-historical', 'Skip historical git analysis')
    .option('--days-back <days>', 'Days to look back for historical analysis', '30')
    .option('--max-commits <count>', 'Maximum commits to analyse', '100')
    .action(async (options: ScanOptions) => {
      log(
        `audit command entered: json=${options.json} verbose=${options.verbose} skipHistorical=${options.skipHistorical} daysBack=${options.daysBack}`
      );
      const spinner = options.json ? null : ora('Starting repository audit...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const scanner = new RepoScanner(workspaceRoot);

        const rawDaysBack = options.daysBack ? Number(options.daysBack) : 30;
        const rawMaxCommits = options.maxCommits ? Number(options.maxCommits) : 100;

        if (!Number.isInteger(rawDaysBack) || rawDaysBack <= 0 || rawDaysBack > 730) {
          error('--days-back must be a positive integer (max 730)');
          process.exit(1);
        }
        if (!Number.isInteger(rawMaxCommits) || rawMaxCommits <= 0 || rawMaxCommits > 10000) {
          error('--max-commits must be a positive integer (max 10000)');
          process.exit(1);
        }
        const daysBack = rawDaysBack;
        const maxCommits = rawMaxCommits;

        const result = await scanner.scan({
          historicalDaysBack: daysBack,
          historicalMaxCommits: maxCommits,
          useCache: options.noCache !== true,
          skipHistorical: options.skipHistorical,
          onProgress: (stage, detail) => {
            if (spinner) {
              spinner.text = detail || stage;
            }
          },
        });

        spinner?.stop();

        if (options.json) {
          formatResultsJSON(result);
        } else if (isTUIAvailable({ tui: options.tui })) {
          // Show TUI results dashboard
          await new Promise<void>((resolve, reject) => {
            const tuiResult = renderTUI(AuditResults, {
              result,
              onComplete: () => resolve(),
              onQuit: () => resolve(),
            });

            if (!tuiResult) {
              // Fallback to human-readable output
              formatResultsHuman(result, options.verbose ?? false);
              resolve();
              return;
            }

            tuiResult.waitUntilExit().catch(reject);
          });
        } else {
          formatResultsHuman(result, options.verbose ?? false);
        }

        log(
          `audit complete: warnings=${result.currentIssues.totalWarnings} blocking=${result.currentIssues.hasBlockingWarnings} duration=${result.totalDurationMs}ms`
        );

        // Exit with appropriate code
        if (result.currentIssues.hasBlockingWarnings) {
          if (!options.json) {
            error('Blocking issues found');
          }
          process.exit(1);
        } else if (result.currentIssues.totalWarnings > 0) {
          if (!options.json) {
            info('Issues found but none are blocking');
          }
          process.exit(0);
        } else {
          if (!options.json) {
            success('Repository audit complete - no issues found!');
          }
          process.exit(0);
        }
      } catch (err) {
        spinner?.fail('Scan failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        process.exit(1);
      }
    });

  return command;
}
