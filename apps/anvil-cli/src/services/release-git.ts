import { execFileSync } from 'node:child_process';
import chalk from 'chalk';

function run(args: string[], cwd: string, timeout = 30_000): string {
  return execFileSync('git', args, { cwd, encoding: 'utf8', timeout }).trim();
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
    console.log(`  ${chalk.yellow('[DRY RUN]')} Would run:`);
    console.log(chalk.dim(`    git add ${filesToStage.join(' ')}`));
    console.log(chalk.dim(`    git commit -m "chore(release): ${tagName}"`));
    console.log(chalk.dim(`    git tag -a ${tagName} -m "${tagName}"`));
    console.log(chalk.dim(`    git push origin main`));
    console.log(chalk.dim(`    git push origin ${tagName}`));
    return { tagName };
  }

  // Stage
  for (const file of filesToStage) {
    run(['add', file], workspaceRoot);
  }
  console.log(
    `  ${chalk.green('✓')} Staged ${filesToStage.length} file${filesToStage.length !== 1 ? 's' : ''}`
  );

  // Commit (120s — pre-commit hooks may be slow)
  run(['commit', '-m', commitMessage], workspaceRoot, 120_000);
  const hash = run(['rev-parse', '--short', 'HEAD'], workspaceRoot);
  console.log(`  ${chalk.green('✓')} Committed ${chalk.dim(hash)}`);

  // Tag
  run(['tag', '-a', tagName, '-m', tagName], workspaceRoot);
  console.log(`  ${chalk.green('✓')} Tagged ${chalk.bold(tagName)}`);

  // Push (120s — slow remotes)
  run(['push', 'origin', 'main'], workspaceRoot, 120_000);
  console.log(`  ${chalk.green('✓')} Pushed main`);

  run(['push', 'origin', tagName], workspaceRoot, 120_000);
  console.log(`  ${chalk.green('✓')} Pushed tag ${tagName}`);

  return { commitHash: hash, tagName };
}
