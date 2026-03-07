import { Command } from 'commander';
import chalk from 'chalk';
import ora, { type Ora } from 'ora';
import { glob } from 'glob';
import { createDebugger } from '@eddacraft/anvil-core';
import {
  SnapshotStore,
  SnapshotCaptureService,
  compareSnapshots,
  generateReport,
  formatReportAsText,
  formatReportAsJson,
  type DriftSnapshot,
  type SnapshotMetadata,
} from '@eddacraft/anvil-core';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { CliError, CliExit } from '../utils/cli-error.js';
import { coercePositiveInt } from '../utils/option-coerce.js';
import { error, info, print, blank, data } from '../utils/output.js';

const log = createDebugger('cli');

function updateSpinner(spinner: Ora | null, text: string): void {
  if (spinner) {
    spinner.text = text;
  }
}

const ANALYSABLE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'];

interface SnapshotOptions {
  name?: string;
  json?: boolean;
}

interface CompareOptions {
  json?: boolean;
}

interface ReportOptions {
  since?: string;
  json?: boolean;
  details?: boolean;
}

interface ListOptions {
  json?: boolean;
  limit?: number;
}

async function getSourceFiles(workspaceRoot: string): Promise<string[]> {
  const patterns = ANALYSABLE_EXTENSIONS.map((ext) => `**/*${ext}`);
  const ignorePatterns = ['**/node_modules/**', '**/dist/**', '**/build/**', '**/.git/**'];

  const files: string[] = [];
  for (const pattern of patterns) {
    const matches = await glob(pattern, {
      cwd: workspaceRoot,
      ignore: ignorePatterns,
      nodir: true,
    });
    files.push(...matches);
  }

  return [...new Set(files)].sort();
}

function formatMetadata(meta: SnapshotMetadata): string {
  const name = meta.name ?? meta.filename.replace('snapshot-', '').replace('.json', '');
  const date = meta.created_at.split('T')[0];
  const violations = meta.metrics.boundary_violations;
  const antipatterns = meta.metrics.antipattern_count;
  const suppressions = meta.metrics.suppression_count;

  return `${name.padEnd(20)} ${date}  V:${violations} AP:${antipatterns} S:${suppressions}`;
}

async function handleSnapshot(options: SnapshotOptions): Promise<void> {
  log(`drift snapshot: name=${options.name ?? '(auto)'}`);
  const spinner = options.json ? null : ora('Capturing snapshot...').start();

  try {
    const workspaceRoot = getWorkspaceRoot();
    const store = new SnapshotStore(workspaceRoot);
    const captureService = new SnapshotCaptureService(workspaceRoot);

    updateSpinner(spinner, 'Gathering source files...');
    const files = await getSourceFiles(workspaceRoot);

    updateSpinner(spinner, `Analysing ${files.length} files...`);
    const snapshot = await captureService.capture(files, { name: options.name });

    updateSpinner(spinner, 'Saving snapshot...');
    const filePath = await store.save(snapshot, options.name);

    if (spinner) spinner.succeed(`Snapshot saved: ${filePath}`);

    if (options.json) {
      data(JSON.stringify(snapshot, null, 2));
    } else {
      blank();
      print(chalk.bold('Metrics:'));
      print(`  Boundary violations: ${snapshot.metrics.boundary_violations}`);
      print(`  Anti-patterns:       ${snapshot.metrics.antipattern_count}`);
      print(`  Suppressions:        ${snapshot.metrics.suppression_count}`);
      print(`  Files analysed:      ${snapshot.metrics.files_analysed}`);

      if (snapshot.name) {
        blank();
        info(`Use 'anvil drift compare ${snapshot.name} <other>' to compare`);
      }
    }
  } catch (err) {
    if (err instanceof CliError || err instanceof CliExit) throw err;
    spinner?.fail('Failed to capture snapshot');
    error(err instanceof Error ? err.message : String(err));
    throw new CliError('Failed to capture snapshot');
  }
}

async function handleCompare(
  snapshot1: string,
  snapshot2: string,
  options: CompareOptions
): Promise<void> {
  log(`drift compare: ${snapshot1} vs ${snapshot2}`);
  const spinner = options.json ? null : ora('Loading snapshots...').start();

  try {
    const workspaceRoot = getWorkspaceRoot();
    const store = new SnapshotStore(workspaceRoot);

    const before = await store.load(snapshot1);
    if (!before) {
      spinner?.fail(`Snapshot not found: ${snapshot1}`);
      throw new CliError(`Snapshot not found: ${snapshot1}`);
    }

    const after = await store.load(snapshot2);
    if (!after) {
      spinner?.fail(`Snapshot not found: ${snapshot2}`);
      throw new CliError(`Snapshot not found: ${snapshot2}`);
    }

    updateSpinner(spinner, 'Comparing snapshots...');
    const comparison = compareSnapshots(before, after);

    if (spinner) spinner.succeed('Comparison complete');

    if (options.json) {
      data(
        JSON.stringify(
          {
            before: { name: before.name, created_at: before.created_at },
            after: { name: after.name, created_at: after.created_at },
            duration_days: comparison.duration_days,
            metrics: comparison.metrics,
            net_change: comparison.net_change,
            overall_trend: comparison.overall_trend,
            violations: {
              added: comparison.violations.added.length,
              removed: comparison.violations.removed.length,
            },
            antipatterns: {
              added: comparison.antipatterns.added.length,
              removed: comparison.antipatterns.removed.length,
            },
          },
          null,
          2
        )
      );
    } else {
      blank();
      const report = generateReport(comparison, { includeDetails: true });
      print(formatReportAsText(report));
    }
  } catch (err) {
    if (err instanceof CliError || err instanceof CliExit) throw err;
    spinner?.fail('Comparison failed');
    error(err instanceof Error ? err.message : String(err));
    throw new CliError('Comparison failed');
  }
}

async function handleReport(options: ReportOptions): Promise<void> {
  log(`drift report: since=${options.since ?? '(latest)'}`);
  const spinner = options.json ? null : ora('Generating report...').start();

  try {
    const workspaceRoot = getWorkspaceRoot();
    const store = new SnapshotStore(workspaceRoot);

    let before: DriftSnapshot | null = null;
    let after: DriftSnapshot | null = null;

    if (options.since) {
      before = await store.load(options.since);
      if (!before) {
        spinner?.fail(`Snapshot not found: ${options.since}`);
        throw new CliError(`Snapshot not found: ${options.since}`);
      }

      after = await store.getLatest();
      if (!after) {
        spinner?.fail('No current snapshot found. Run `anvil drift snapshot` first.');
        throw new CliError('No current snapshot found');
      }
    } else {
      const snapshots = await store.list();

      if (snapshots.length < 2) {
        spinner?.fail('Need at least 2 snapshots to generate a report');
        info('Run `anvil drift snapshot` to create snapshots');
        throw new CliError('Need at least 2 snapshots to generate a report');
      }

      after = await store.load(snapshots[0].filename);
      before = await store.load(snapshots[1].filename);
    }

    if (!before || !after) {
      spinner?.fail('Could not load snapshots');
      throw new CliError('Could not load snapshots');
    }

    updateSpinner(spinner, 'Comparing snapshots...');
    const comparison = compareSnapshots(before, after);
    const report = generateReport(comparison, {
      format: options.json ? 'json' : 'text',
      includeDetails: options.details !== false,
    });

    if (spinner) spinner.succeed('Report generated');

    if (options.json) {
      data(formatReportAsJson(report));
    } else {
      blank();
      print(formatReportAsText(report));
    }
  } catch (err) {
    if (err instanceof CliError || err instanceof CliExit) throw err;
    spinner?.fail('Report generation failed');
    error(err instanceof Error ? err.message : String(err));
    throw new CliError('Report generation failed');
  }
}

async function handleList(options: ListOptions): Promise<void> {
  log(`drift list: limit=${options.limit ?? '(all)'}`);
  const spinner = options.json ? null : ora('Loading snapshots...').start();

  try {
    const workspaceRoot = getWorkspaceRoot();
    const store = new SnapshotStore(workspaceRoot);

    let snapshots = await store.list();

    if (options.limit && options.limit > 0) {
      snapshots = snapshots.slice(0, options.limit);
    }

    spinner?.succeed(`Found ${snapshots.length} snapshot(s)`);

    if (snapshots.length === 0) {
      info('No snapshots found. Run `anvil drift snapshot` to create one.');
      return;
    }

    if (options.json) {
      data(JSON.stringify(snapshots, null, 2));
    } else {
      blank();
      print(chalk.bold('NAME'.padEnd(20) + ' DATE        METRICS'));
      print(chalk.gray('-'.repeat(60)));

      for (const meta of snapshots) {
        print(formatMetadata(meta));
      }

      blank();
      print(chalk.gray('V=violations, AP=anti-patterns, S=suppressions'));
    }
  } catch (err) {
    if (err instanceof CliError || err instanceof CliExit) throw err;
    spinner?.fail('Failed to list snapshots');
    error(err instanceof Error ? err.message : String(err));
    throw new CliError('Failed to list snapshots');
  }
}

export function createDriftCommand(): Command {
  const command = new Command('drift');

  command.description('Track architecture drift over time');

  command
    .command('snapshot')
    .description('Capture current state as a snapshot')
    .option('--name <name>', 'Give the snapshot a name (e.g., release-1.0)')
    .option('--json', 'Output snapshot as JSON')
    .action(handleSnapshot);

  command
    .command('compare <snapshot1> <snapshot2>')
    .description('Compare two snapshots')
    .option('--json', 'Output comparison as JSON')
    .action(handleCompare);

  command
    .command('report')
    .description('Generate a drift report')
    .option('--since <snapshot>', 'Compare against specific snapshot')
    .option('--json', 'Output report as JSON')
    .option('--no-details', 'Exclude detailed violation lists')
    .action(handleReport);

  command
    .command('list')
    .description('List available snapshots')
    .option('--json', 'Output list as JSON')
    .option('--limit <n>', 'Limit number of results', (v: string) =>
      coercePositiveInt(v, '--limit')
    )
    .action(handleList);

  return command;
}
