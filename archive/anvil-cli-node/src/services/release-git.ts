import { gitExecSync } from '@eddacraft/anvil-core';
import chalk from 'chalk';
import { print } from '../utils/output.js';

function run(args: string[], cwd: string, timeout = 30_000): string {
  return gitExecSync(args, { cwd, timeout });
}

export function isCleanWorkingTree(workspaceRoot: string): boolean {
  const status = run(['status', '--porcelain'], workspaceRoot);
  return status.length === 0;
}

export function getCurrentBranch(workspaceRoot: string): string {
  return run(['rev-parse', '--abbrev-ref', 'HEAD'], workspaceRoot);
}

export interface GitOpsResult {
  commitHash?: string;
  tagName: string;
}

export async function commitTagPush(
  workspaceRoot: string,
  version: string,
  tagName: string,
  filesToStage: string[],
  execute: boolean
): Promise<GitOpsResult> {
  const commitMessage = `chore(release): ${tagName}\n\nAuthored-By: Aneki (joshuaboys)`;

  if (!execute) {
    print(`  ${chalk.yellow('[DRY RUN]')} Would run:`);
    print(chalk.dim(`    git add ${filesToStage.join(' ')}`));
    print(chalk.dim(`    git commit -m "chore(release): ${tagName}"`));
    print(chalk.dim(`    git tag -a ${tagName} -m "${tagName}"`));
    print(chalk.dim(`    git push origin main`));
    print(chalk.dim(`    git push origin ${tagName}`));
    return { tagName };
  }

  // Stage
  for (const file of filesToStage) {
    run(['add', file], workspaceRoot);
  }
  print(
    `  ${chalk.green('✓')} Staged ${filesToStage.length} file${filesToStage.length !== 1 ? 's' : ''}`
  );

  // Commit (120s — pre-commit hooks may be slow)
  run(['commit', '-m', commitMessage], workspaceRoot, 120_000);
  const hash = run(['rev-parse', '--short', 'HEAD'], workspaceRoot);
  print(`  ${chalk.green('✓')} Committed ${chalk.dim(hash)}`);

  // Tag
  run(['tag', '-a', tagName, '-m', tagName], workspaceRoot);
  print(`  ${chalk.green('✓')} Tagged ${chalk.bold(tagName)}`);

  // Push (120s — slow remotes)
  run(['push', 'origin', 'main'], workspaceRoot, 120_000);
  print(`  ${chalk.green('✓')} Pushed main`);

  run(['push', 'origin', tagName], workspaceRoot, 120_000);
  print(`  ${chalk.green('✓')} Pushed tag ${tagName}`);

  return { commitHash: hash, tagName };
}
