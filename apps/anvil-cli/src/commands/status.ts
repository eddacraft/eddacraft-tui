import { Command } from 'commander';
import chalk from 'chalk';
import { gatherStatusData } from '../services/status-service.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { StatusDashboard } from '../tui/commands/status/index.js';
import { theme } from '../tui/utils/theme.js';
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
  console.log(chalk.bold('\nANVIL STATUS\n'));
  console.log(chalk.hex(theme.colours.smoke)(`Project: ${data.projectName ?? data.projectRoot}`));
  console.log('');

  console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} HOOKS`));
  if (!data.hooks.huskyInstalled) {
    console.log(chalk.hex(theme.colours.molten)(`  ${theme.icons.warning} Husky not installed`));
  } else {
    for (const hook of data.hooks.hooks) {
      const icon =
        hook.state === 'active'
          ? theme.icons.success
          : hook.state === 'disabled'
            ? theme.icons.skipped
            : theme.icons.error;
      const colour =
        hook.state === 'active'
          ? chalk.hex(theme.colours.steel)
          : hook.state === 'disabled'
            ? chalk.hex(theme.colours.molten)
            : chalk.hex(theme.colours.slag);
      console.log(colour(`  ${icon} ${hook.name}: ${hook.state}`));
    }
  }
  console.log('');

  console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} CONFIGURATION`));
  if (!data.profile.hasConfig) {
    console.log(
      chalk.hex(theme.colours.molten)(
        `  ${theme.icons.warning} No .anvilrc found — run \`anvil init\``
      )
    );
  } else {
    if (data.profile.planningDir) {
      console.log(chalk.hex(theme.colours.smoke)(`  Plans: ${data.profile.planningDir}`));
    }
    if (data.profile.format) {
      console.log(chalk.hex(theme.colours.smoke)(`  Format: ${data.profile.format}`));
    }
    if (data.profile.coverageThreshold) {
      console.log(chalk.hex(theme.colours.smoke)(`  Coverage: ${data.profile.coverageThreshold}%`));
    }
    if (data.profile.checks.length > 0) {
      const enabled = data.profile.checks.filter((c) => c.enabled).map((c) => c.name);
      console.log(chalk.hex(theme.colours.smoke)(`  Checks: ${enabled.join(', ') || 'none'}`));
    }
  }
  console.log('');

  console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} RECENT RESULTS`));
  if (!data.recent.hasCache || data.recent.results.length === 0) {
    console.log(chalk.hex(theme.colours.smoke)(`  ${theme.icons.info} No validation history yet`));
  } else {
    for (const result of data.recent.results) {
      const icon = result.passed ? theme.icons.success : theme.icons.error;
      const colour = result.passed ? chalk.hex(theme.colours.steel) : chalk.hex(theme.colours.slag);
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
