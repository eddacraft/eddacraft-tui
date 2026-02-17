import { Command } from 'commander';
import chalk from 'chalk';
import { createDebugger } from '@eddacraft/anvil-core';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';

const log = createDebugger('cli');
import { renderTUI } from '../tui/utils/renderer.js';
import { Diagnostics } from '../tui/commands/doctor/Diagnostics.js';
import { theme } from '../tui/utils/theme.js';
import {
  NodeVersionCheck,
  GitCheck,
  GitRepoCheck,
  ConfigExistsCheck,
  ConfigValidCheck,
  AnvilDirCheck,
  HuskyInstalledCheck,
  PreCommitHookCheck,
  AnvilDirWritableCheck,
  PlansDirReadableCheck,
  PolicyConfigCheck,
  PolicyDirectoryCheck,
  PolicyDocumentationCheck,
  PolicyOrgVersionCheck,
} from '../tui/commands/doctor/checks/index.js';
import type {
  DiagnosticCheck,
  DiagnosticsData,
  DiagnosticResult,
  DiagnosticContext,
} from '../tui/commands/doctor/types.js';
import { calculateSummary } from '../tui/commands/doctor/types.js';

interface DoctorOptions {
  fix?: boolean;
  json?: boolean;
  // Commander.js --no-tui sets options.tui = false (not options.noTui = true)
  tui?: boolean;
  verbose?: boolean;
}

function getAllChecks(): DiagnosticCheck[] {
  return [
    new NodeVersionCheck(),
    new GitCheck(),
    new GitRepoCheck(),
    new ConfigExistsCheck(),
    new ConfigValidCheck(),
    new AnvilDirCheck(),
    new HuskyInstalledCheck(),
    new PreCommitHookCheck(),
    new AnvilDirWritableCheck(),
    new PlansDirReadableCheck(),
    // Policy health checks
    new PolicyConfigCheck(),
    new PolicyDirectoryCheck(),
    new PolicyDocumentationCheck(),
    new PolicyOrgVersionCheck(),
  ];
}

function getStatusIcon(status: DiagnosticResult['status']): string {
  switch (status) {
    case 'pass':
      return chalk.hex(theme.colours.steel)(theme.icons.success);
    case 'warn':
      return chalk.hex(theme.colours.molten)(theme.icons.warning);
    case 'fail':
      return chalk.hex(theme.colours.slag)(theme.icons.error);
    case 'skip':
      return chalk.hex(theme.colours.smoke)(theme.icons.skipped);
  }
}

function formatJsonOutput(data: DiagnosticsData): string {
  return JSON.stringify(
    {
      projectRoot: data.projectRoot,
      ranAt: data.ranAt.toISOString(),
      summary: data.summary,
      results: data.results,
    },
    null,
    2
  );
}

function printPlainTextDiagnostics(data: DiagnosticsData, verbose: boolean): void {
  console.log(chalk.bold('\nANVIL DOCTOR\n'));

  for (const result of data.results) {
    const icon = getStatusIcon(result.status);
    console.log(`${icon} ${result.name}: ${result.message}`);
    if (verbose && result.details) {
      console.log(chalk.hex(theme.colours.smoke)(`   ${result.details}`));
    }
    if (result.fixable && result.status !== 'pass' && result.suggestion) {
      console.log(chalk.hex(theme.colours.smoke)(`   ${theme.icons.arrow} ${result.suggestion}`));
    }
  }

  console.log('');

  const { summary } = data;
  if (summary.healthy) {
    console.log(chalk.hex(theme.colours.steel).bold(`${theme.icons.success} All checks passed`));
  } else {
    console.log(chalk.hex(theme.colours.slag).bold(`${theme.icons.error} Issues found`));
  }

  const parts: string[] = [];
  parts.push(chalk.hex(theme.colours.steel)(`${summary.passed} passed`));
  if (summary.warnings > 0)
    parts.push(chalk.hex(theme.colours.molten)(`${summary.warnings} warnings`));
  if (summary.failed > 0) parts.push(chalk.hex(theme.colours.slag)(`${summary.failed} failed`));
  if (summary.skipped > 0) parts.push(chalk.hex(theme.colours.smoke)(`${summary.skipped} skipped`));

  console.log(parts.join(` ${theme.icons.bullet} `));

  if (summary.fixable > 0) {
    console.log(
      chalk.hex(theme.colours.ash)(
        `\n${theme.icons.info} ${summary.fixable} issue(s) can be auto-fixed with: anvil doctor --fix`
      )
    );
  }

  console.log('');
}

async function runChecksPlain(
  checks: DiagnosticCheck[],
  context: DiagnosticContext,
  autoFix: boolean
): Promise<DiagnosticsData> {
  const results: DiagnosticResult[] = [];
  const isTTY = process.stdout.isTTY;

  for (const check of checks) {
    if (isTTY) {
      process.stdout.write(chalk.dim(`Running ${check.name}...`));
    }
    const result = await check.run(context);
    if (isTTY && process.stdout.clearLine && process.stdout.cursorTo) {
      process.stdout.clearLine(0);
      process.stdout.cursorTo(0);
    }
    results.push(result);
    console.log(`${getStatusIcon(result.status)} ${result.name}: ${result.message}`);
  }

  if (autoFix) {
    console.log(chalk.cyan('\nApplying fixes...'));
    for (let i = 0; i < results.length; i++) {
      const result = results[i];
      if (result.fixable && result.status !== 'pass') {
        const check = checks.find((c) => c.id === result.checkId);
        if (check?.fix) {
          const fixResult = await check.fix(context);
          if (fixResult.success) {
            results[i] = {
              ...result,
              status: 'pass',
              message: `Fixed: ${fixResult.message}`,
              fixable: false,
            };
            console.log(chalk.green(`  ✓ Fixed: ${check.name}`));
          } else {
            console.log(chalk.red(`  ✗ Failed to fix ${check.name}: ${fixResult.message}`));
          }
        }
      }
    }
  }

  return {
    projectRoot: context.projectRoot,
    results,
    summary: calculateSummary(results),
    ranAt: new Date(),
  };
}

export function createDoctorCommand(): Command {
  const command = new Command('doctor');

  command
    .description('Run diagnostic checks and fix common issues')
    .option('--fix', 'Auto-fix fixable issues')
    .option('--json', 'Output diagnostics as JSON')
    .option('--tui', 'Force TUI mode')
    .option('--no-tui', 'Use plain text output instead of TUI')
    .option('-v, --verbose', 'Show detailed information')
    .action(async (options: DoctorOptions) => {
      log(
        `doctor command entered: fix=${options.fix} json=${options.json} tui=${options.tui} verbose=${options.verbose}`
      );
      const projectRoot = process.cwd();
      const checks = getAllChecks();
      const context: DiagnosticContext = {
        projectRoot,
        verbose: options.verbose ?? false,
      };

      if (options.json) {
        log('doctor: running in JSON mode');
        const data = await runChecksPlain(checks, context, options.fix ?? false);
        log(
          `doctor result: passed=${data.summary.passed} failed=${data.summary.failed} warnings=${data.summary.warnings}`
        );
        console.log(formatJsonOutput(data));
        process.exit(data.summary.healthy ? 0 : 1);
        return;
      }

      const useTUI = isTUIAvailable({ tui: options.tui });
      log(`doctor: useTUI=${useTUI} checks=${checks.length}`);

      if (useTUI) {
        let exitCode = 0;
        await new Promise<void>((resolve) => {
          renderTUI(Diagnostics, {
            checks,
            context,
            autoFix: options.fix ?? false,
            onComplete: (data: DiagnosticsData) => {
              exitCode = data.summary.healthy ? 0 : 1;
            },
            onQuit: resolve,
          });
        });
        process.exit(exitCode);
      } else {
        const data = await runChecksPlain(checks, context, options.fix ?? false);
        printPlainTextDiagnostics(data, options.verbose ?? false);
        process.exit(data.summary.healthy ? 0 : 1);
      }
    });

  return command;
}
