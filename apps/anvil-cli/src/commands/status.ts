import { Command } from 'commander';
import chalk from 'chalk';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { gatherStatusData } from '../services/status-service.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { StatusDashboard } from '../tui/commands/status/index.js';
import { theme } from '../tui/utils/theme.js';
import type { StatusData } from '../tui/commands/status/types.js';
import { createProvenanceStore } from '@eddacraft/anvil-core';
import { ProposalStore, MemoryStore } from '@eddacraft/anvil-edda-stack';
import type { EmberStats, EddaStats } from '@eddacraft/anvil-edda-stack';

interface StatusOptions {
  json?: boolean;
  // Commander.js --no-tui sets options.tui = false (not options.noTui = true)
  tui?: boolean;
}

interface EmberStatsResult {
  hasDatabase: boolean;
  stats: EmberStats | null;
  error?: unknown;
}

interface EddaStatsResult {
  hasDatabase: boolean;
  stats: EddaStats | null;
  error?: unknown;
}

async function loadEmberStats(projectRoot: string): Promise<EmberStatsResult> {
  const emberDbPath = join(projectRoot, '.anvil', 'ember.db');
  if (!existsSync(emberDbPath)) {
    return { hasDatabase: false, stats: null };
  }

  let store: ProposalStore | null = null;
  try {
    store = new ProposalStore(emberDbPath);
    const stats = await store.getStats();
    return { hasDatabase: true, stats };
  } catch (error) {
    return { hasDatabase: true, stats: null, error };
  } finally {
    store?.close();
  }
}

async function loadEddaStats(projectRoot: string): Promise<EddaStatsResult> {
  const storagePath = join(projectRoot, '.anvil', 'edda');
  if (!existsSync(storagePath)) {
    return { hasDatabase: false, stats: null };
  }

  try {
    const store = new MemoryStore({
      type: 'git' as const,
      path: storagePath,
      format: 'yaml' as const,
    });
    const stats = await store.getStats();
    return { hasDatabase: true, stats };
  } catch (error) {
    return { hasDatabase: true, stats: null, error };
  }
}

async function formatJsonOutput(data: StatusData, projectRoot: string): Promise<string> {
  const store = createProvenanceStore(projectRoot);
  const provenanceStats = store.isInitialised() ? store.getStatistics() : null;
  const ember = await loadEmberStats(projectRoot);
  const eddaStats = await loadEddaStats(projectRoot);

  if (ember.error) {
    console.error(
      chalk.hex(theme.colours.molten)(`  ${theme.icons.warning} Ember stats unavailable`)
    );
  }

  if (eddaStats.error) {
    console.error(
      chalk.hex(theme.colours.molten)(`  ${theme.icons.warning} Edda stats unavailable`)
    );
  }

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
      ...(provenanceStats
        ? {
            provenance: {
              total: provenanceStats.total,
              passed: provenanceStats.passed,
              failed: provenanceStats.failed,
              passRate: Math.round(provenanceStats.passRate * 10) / 10,
              lastCheck: provenanceStats.lastCheck,
              lastPass: provenanceStats.lastPass,
              lastFail: provenanceStats.lastFail,
            },
          }
        : {}),
      ember: ember.stats,
      edda: eddaStats.stats,
    },
    null,
    2
  );
}

async function printPlainTextStatus(data: StatusData, projectRoot: string): Promise<void> {
  console.error(chalk.bold('\nANVIL STATUS\n'));
  console.error(chalk.hex(theme.colours.smoke)(`Project: ${data.projectName ?? data.projectRoot}`));
  console.error('');

  console.error(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} HOOKS`));
  if (!data.hooks.huskyInstalled) {
    console.error(chalk.hex(theme.colours.molten)(`  ${theme.icons.warning} Husky not installed`));
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
      console.error(colour(`  ${icon} ${hook.name}: ${hook.state}`));
    }
  }
  console.error('');

  console.error(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} CONFIGURATION`));
  if (!data.profile.hasConfig) {
    console.error(
      chalk.hex(theme.colours.molten)(
        `  ${theme.icons.warning} No .anvilrc found — run \`anvil init\``
      )
    );
  } else {
    if (data.profile.planningDir) {
      console.error(chalk.hex(theme.colours.smoke)(`  Plans: ${data.profile.planningDir}`));
    }
    if (data.profile.format) {
      console.error(chalk.hex(theme.colours.smoke)(`  Format: ${data.profile.format}`));
    }
    if (data.profile.coverageThreshold) {
      console.error(
        chalk.hex(theme.colours.smoke)(`  Coverage: ${data.profile.coverageThreshold}%`)
      );
    }
    if (data.profile.checks.length > 0) {
      const enabled = data.profile.checks.filter((c) => c.enabled).map((c) => c.name);
      console.error(chalk.hex(theme.colours.smoke)(`  Checks: ${enabled.join(', ') || 'none'}`));
    }
  }
  console.error('');

  console.error(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} RECENT RESULTS`));
  if (!data.recent.hasCache || data.recent.results.length === 0) {
    console.error(
      chalk.hex(theme.colours.smoke)(`  ${theme.icons.info} No validation history yet`)
    );
  } else {
    for (const result of data.recent.results) {
      const icon = result.passed ? theme.icons.success : theme.icons.error;
      const colour = result.passed ? chalk.hex(theme.colours.steel) : chalk.hex(theme.colours.slag);
      console.error(
        colour(`  ${icon} ${result.planPath} — ${result.passedChecks}/${result.totalChecks} checks`)
      );
    }
  }
  console.error('');

  // Provenance history
  const store = createProvenanceStore(projectRoot);
  if (store.isInitialised()) {
    const stats = store.getStatistics();
    console.error(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} PROVENANCE`));
    if (stats.total === 0) {
      console.error(
        chalk.hex(theme.colours.smoke)(`  ${theme.icons.info} No provenance records yet`)
      );
    } else {
      const passRateStr = `${Math.round(stats.passRate)}%`;
      const passColour =
        stats.passRate >= 80
          ? chalk.hex(theme.colours.steel)
          : stats.passRate >= 50
            ? chalk.hex(theme.colours.molten)
            : chalk.hex(theme.colours.slag);
      console.error(chalk.hex(theme.colours.smoke)(`  Total runs: ${stats.total}`));
      console.error(
        passColour(`  Pass rate: ${passRateStr} (${stats.passed} passed, ${stats.failed} failed)`)
      );
      if (stats.lastCheck) {
        console.error(chalk.hex(theme.colours.smoke)(`  Last check: ${stats.lastCheck}`));
      }
    }
    console.error('');
  }

  console.error(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} EMBER CANDIDATES`));
  const ember = await loadEmberStats(projectRoot);
  if (!ember.hasDatabase) {
    console.error(chalk.hex(theme.colours.smoke)(`  ${theme.icons.info} No ember database found`));
  } else if (ember.error || !ember.stats) {
    console.error(
      chalk.hex(theme.colours.molten)(`  ${theme.icons.warning} Ember stats unavailable`)
    );
  } else {
    const active = ember.stats.by_status.find((entry) => entry.status === 'active')?.count ?? 0;
    const promoted = ember.stats.by_status.find((entry) => entry.status === 'promoted')?.count ?? 0;
    const expired = ember.stats.by_status.find((entry) => entry.status === 'expired')?.count ?? 0;
    const dismissed =
      ember.stats.by_status.find((entry) => entry.status === 'dismissed')?.count ?? 0;

    const activeColour =
      active > 0 ? chalk.hex(theme.colours.success) : chalk.hex(theme.colours.smoke);
    const nearExpiryColour =
      ember.stats.expiring_soon > 0
        ? ember.stats.expiring_soon >= 5
          ? chalk.hex(theme.colours.error)
          : chalk.hex(theme.colours.warning)
        : chalk.hex(theme.colours.smoke);

    console.error(activeColour(`  Active: ${active}`));
    console.error(chalk.hex(theme.colours.smoke)(`  Promoted: ${promoted}`));
    console.error(chalk.hex(theme.colours.smoke)(`  Expired: ${expired}`));
    console.error(chalk.hex(theme.colours.smoke)(`  Dismissed: ${dismissed}`));
    console.error(nearExpiryColour(`  Near expiry: ${ember.stats.expiring_soon}`));
    console.error(
      chalk.hex(theme.colours.smoke)(
        `  Average confidence: ${(ember.stats.avg_confidence ?? 0).toFixed(2)}`
      )
    );

    const typeStats = ember.stats.by_type.filter((entry) => entry.count > 0);
    if (typeStats.length > 0) {
      console.error(
        chalk.hex(theme.colours.smoke)(
          `  By type: ${typeStats.map((entry) => `${entry.type}: ${entry.count}`).join(', ')}`
        )
      );
    }
  }
  console.error('');

  console.error(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} EDDA MEMORIES`));
  const eddaStats = await loadEddaStats(projectRoot);
  if (!eddaStats.hasDatabase) {
    console.error(chalk.hex(theme.colours.smoke)(`  ${theme.icons.info} No Edda storage found`));
  } else if (eddaStats.error || !eddaStats.stats) {
    console.error(
      chalk.hex(theme.colours.molten)(`  ${theme.icons.warning} Edda stats unavailable`)
    );
  } else {
    console.error(chalk.hex(theme.colours.smoke)(`  Active: ${eddaStats.stats.active_count}`));
    console.error(
      chalk.hex(theme.colours.smoke)(`  Superseded: ${eddaStats.stats.superseded_count}`)
    );
    console.error(chalk.hex(theme.colours.smoke)(`  Retired: ${eddaStats.stats.retired_count}`));

    const byType = eddaStats.stats.by_type.filter((entry) => entry.count > 0);
    if (byType.length > 0) {
      console.error(
        chalk.hex(theme.colours.smoke)(
          `  By type: ${byType.map((entry) => `${entry.type}: ${entry.count}`).join(', ')}`
        )
      );
    } else {
      console.error(
        chalk.hex(theme.colours.smoke)(`  ${theme.icons.info} No type breakdown available`)
      );
    }

    const byConfidence = eddaStats.stats.by_confidence.filter((entry) => entry.count > 0);
    if (byConfidence.length > 0) {
      console.error(
        chalk.hex(theme.colours.smoke)(
          `  By confidence: ${byConfidence.map((entry) => `${entry.level}: ${entry.count}`).join(', ')}`
        )
      );
    } else {
      console.error(
        chalk.hex(theme.colours.smoke)(`  ${theme.icons.info} No confidence breakdown available`)
      );
    }
  }
  console.error('');
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
        console.log(await formatJsonOutput(data, projectRoot));
        return;
      }

      const useTUI = isTUIAvailable({ tui: options.tui });

      if (useTUI) {
        await new Promise<void>((resolve) => {
          renderTUI(StatusDashboard, { data, onQuit: resolve });
        });
      } else {
        await printPlainTextStatus(data, projectRoot);
      }
    });

  return command;
}
