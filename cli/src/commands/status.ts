import { Command } from 'commander';
import chalk from 'chalk';
import { gatherStatusData } from '../services/status-service.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { StatusDashboard } from '../tui/commands/status/index.js';
import type { StatusData } from '../tui/commands/status/types.js';

interface StatusOptions {
  json?: boolean;
  tui?: boolean;
  noTui?: boolean;
}

function formatJsonOutput(data: StatusData): string {
  return JSON.stringify(
    {
      projectRoot: data.projectRoot,
      projectName: data.projectName,
      gatheredAt: data.gatheredAt.toISOString(),
      hooks: {
        huskyInstalled: data.hooks.huskyInstalled,
        hooksDir: data.hooks.hooksDir,
        hooks: data.hooks.hooks.map((h) => ({
          name: h.name,
          state: h.state,
          isAnvilManaged: h.isAnvilManaged,
          lastRun: h.lastRun?.toISOString(),
        })),
      },
      profile: {
        hasConfig: data.profile.hasConfig,
        configPath: data.profile.configPath,
        planningDir: data.profile.planningDir,
        format: data.profile.format,
        coverageThreshold: data.profile.coverageThreshold,
        checks: data.profile.checks,
      },
      recent: {
        hasCache: data.recent.hasCache,
        cacheDir: data.recent.cacheDir,
        results: data.recent.results.map((r) => ({
          id: r.id,
          timestamp: r.timestamp.toISOString(),
          planPath: r.planPath,
          passed: r.passed,
          passedChecks: r.passedChecks,
          totalChecks: r.totalChecks,
        })),
      },
    },
    null,
    2
  );
}

function printPlainTextStatus(data: StatusData): void {
  console.log(chalk.bold('\n🔨 Anvil Status\n'));
  console.log(chalk.dim(`Project: ${data.projectName ?? data.projectRoot}`));
  console.log('');

  console.log(chalk.cyan.bold('Hooks'));
  if (!data.hooks.huskyInstalled) {
    console.log(chalk.yellow('  ⚠ Husky not installed'));
  } else {
    for (const hook of data.hooks.hooks) {
      const icon = hook.state === 'active' ? '✓' : hook.state === 'disabled' ? '○' : '✗';
      const colour =
        hook.state === 'active'
          ? chalk.green
          : hook.state === 'disabled'
            ? chalk.yellow
            : chalk.red;
      console.log(colour(`  ${icon} ${hook.name}: ${hook.state}`));
    }
  }
  console.log('');

  console.log(chalk.cyan.bold('Configuration'));
  if (!data.profile.hasConfig) {
    console.log(chalk.yellow('  ⚠ No .anvilrc found — run `anvil init`'));
  } else {
    if (data.profile.planningDir) {
      console.log(chalk.dim(`  Plans: ${data.profile.planningDir}`));
    }
    if (data.profile.format) {
      console.log(chalk.dim(`  Format: ${data.profile.format}`));
    }
    if (data.profile.coverageThreshold) {
      console.log(chalk.dim(`  Coverage: ${data.profile.coverageThreshold}%`));
    }
    if (data.profile.checks.length > 0) {
      const enabled = data.profile.checks.filter((c) => c.enabled).map((c) => c.name);
      console.log(chalk.dim(`  Checks: ${enabled.join(', ') || 'none'}`));
    }
  }
  console.log('');

  console.log(chalk.cyan.bold('Recent Results'));
  if (!data.recent.hasCache || data.recent.results.length === 0) {
    console.log(chalk.dim('  No validation history yet'));
  } else {
    for (const result of data.recent.results) {
      const icon = result.passed ? '✓' : '✗';
      const colour = result.passed ? chalk.green : chalk.red;
      console.log(
        colour(`  ${icon} ${result.planPath} — ${result.passedChecks}/${result.totalChecks} checks`)
      );
    }
  }
  console.log('');
}

export function createStatusCommand(): Command {
  const command = new Command('status');

  command
    .description('Show Anvil workspace status at a glance')
    .option('--json', 'Output status as JSON')
    .option('--tui', 'Force TUI dashboard mode')
    .option('--no-tui', 'Use plain text output instead of TUI')
    .action(async (options: StatusOptions) => {
      const projectRoot = process.cwd();
      const data = gatherStatusData(projectRoot);

      if (options.json) {
        console.log(formatJsonOutput(data));
        return;
      }

      const useTUI = isTUIAvailable({ tui: options.tui, noTui: options.noTui });

      if (useTUI) {
        await new Promise<void>((resolve) => {
          renderTUI(StatusDashboard, { data, onQuit: resolve });
        });
      } else {
        printPlainTextStatus(data);
      }
    });

  return command;
}
