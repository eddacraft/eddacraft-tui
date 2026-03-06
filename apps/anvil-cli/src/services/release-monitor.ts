import { execFileSync, spawn } from 'node:child_process';
import chalk from 'chalk';
import inquirer from 'inquirer';

function ghAvailable(): boolean {
  try {
    execFileSync('gh', ['--version'], { encoding: 'utf8', stdio: 'pipe' });
    return true;
  } catch {
    return false;
  }
}

interface WorkflowRun {
  databaseId: number;
  status: string;
  name: string;
  event: string;
  headBranch: string;
}

function findTriggeredRun(
  workspaceRoot: string,
  tagName: string
): { run: WorkflowRun; exact: boolean } | null {
  try {
    const output = execFileSync(
      'gh',
      [
        'run',
        'list',
        '--repo',
        'EddaCraft/anvil-001',
        '--limit',
        '10',
        '--json',
        'databaseId,status,name,event,headBranch',
      ],
      { cwd: workspaceRoot, encoding: 'utf8', stdio: 'pipe' }
    );
    const runs = JSON.parse(output) as WorkflowRun[];

    // Best match: Publish workflow triggered by this tag
    const exactMatch = runs.find((r) => r.name === 'Publish to NPM' && r.headBranch === tagName);
    if (exactMatch) return { run: exactMatch, exact: true };

    // Good match: any run on this tag
    const tagMatch = runs.find((r) => r.headBranch === tagName);
    if (tagMatch) return { run: tagMatch, exact: false };

    // Fallback: most recent Publish workflow (may not be ours)
    const publishMatch = runs.find((r) => r.name === 'Publish to NPM');
    if (publishMatch) return { run: publishMatch, exact: false };

    return null;
  } catch {
    return null;
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function monitorWorkflow(
  workspaceRoot: string,
  tagName: string,
  execute: boolean
): Promise<number | undefined> {
  if (!execute) {
    console.log(`  ${chalk.yellow('[DRY RUN]')} Would run:`);
    console.log(chalk.dim(`    gh run list --repo EddaCraft/anvil-001 --limit 5`));
    console.log(chalk.dim(`    gh run watch <run-id> --repo EddaCraft/anvil-001`));
    return undefined;
  }

  if (!ghAvailable()) {
    console.log(chalk.dim('  gh CLI not found — skipping workflow monitoring'));
    console.log(chalk.dim('  Run manually: gh run list --repo EddaCraft/anvil-001 --limit 5'));
    return undefined;
  }

  console.log(chalk.dim('  Waiting for workflow to start...'));
  await sleep(5000);

  const result = findTriggeredRun(workspaceRoot, tagName);
  if (!result) {
    console.log(chalk.dim('  Could not find workflow run. Check manually:'));
    console.log(chalk.dim('    gh run list --repo EddaCraft/anvil-001 --limit 5'));
    return undefined;
  }

  const { run, exact } = result;
  if (!exact) {
    console.log(
      chalk.yellow(
        `  ⚠ Could not find a run matching tag ${tagName}; showing most recent Publish run`
      )
    );
  }
  console.log(`  ${chalk.green('✓')} Found run #${run.databaseId}: ${run.name} (${run.status})`);

  const { watch } = await inquirer.prompt<{ watch: boolean }>([
    {
      type: 'confirm',
      name: 'watch',
      message: `Watch workflow run #${run.databaseId}?`,
      default: true,
    },
  ]);

  if (watch) {
    console.log(chalk.dim('  Streaming workflow output (Ctrl+C to stop watching)...\n'));
    await new Promise<void>((resolve) => {
      const child = spawn(
        'gh',
        ['run', 'watch', String(run.databaseId), '--repo', 'EddaCraft/anvil-001'],
        {
          cwd: workspaceRoot,
          stdio: 'inherit',
        }
      );
      child.on('close', () => resolve());
      child.on('error', () => resolve());
    });
  }

  return run.databaseId;
}
