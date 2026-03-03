/**
 * Hooks Command - Manage Git hooks for Anvil integration
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { createDebugger } from '@eddacraft/anvil-core';
import { existsSync, readFileSync, mkdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { getWorkspaceRoot, readJsonFileSync } from '../utils/file-io.js';
import { success, error, info } from '../utils/output.js';
import { HookInstaller } from '../services/hook-installer.js';
import { CliError } from '../utils/cli-error.js';

const log = createDebugger('cli');

/**
 * Resolve the .git directory, handling worktrees where .git is a file
 * containing "gitdir: <path>".
 * Returns the path to the actual .git directory, or null if not a git repo.
 */
function resolveGitDir(workspaceRoot: string): string | null {
  const gitPath = join(workspaceRoot, '.git');
  if (!existsSync(gitPath)) return null;

  // If .git is a directory, use it directly
  if (statSync(gitPath).isDirectory()) return gitPath;

  // Worktree: .git is a file with "gitdir: <path>"
  const content = readFileSync(gitPath, 'utf-8').trim();
  const match = content.match(/^gitdir:\s+(.+)$/);
  if (!match) return null;

  const gitDir = resolve(workspaceRoot, match[1]);
  return existsSync(gitDir) ? gitDir : null;
}

/**
 * Check if Husky is being used in the project
 */
function detectHusky(workspaceRoot: string): { detected: boolean; huskyDir: string | null } {
  const huskyDir = join(workspaceRoot, '.husky');
  const huskyDirExists = existsSync(huskyDir);

  // Also check package.json for husky dependency
  const packageJsonPath = join(workspaceRoot, 'package.json');
  let hasHuskyDep = false;

  const pkg = readJsonFileSync<Record<string, Record<string, unknown>>>(packageJsonPath);
  if (pkg) {
    hasHuskyDep = !!(pkg.devDependencies?.husky || pkg.dependencies?.husky);
  }

  return {
    detected: huskyDirExists || hasHuskyDep,
    huskyDir: huskyDirExists ? huskyDir : null,
  };
}

export function createHooksCommand(): Command {
  const command = new Command('hooks');

  command.description('Manage Git hooks for Anvil integration');

  // Install subcommand
  command
    .command('install')
    .description('Install Anvil Git hooks (pre-commit and pre-push)')
    .option('-f, --force', 'Overwrite existing hooks (creates backup)')
    .option('--pre-commit-only', 'Only install pre-commit hook')
    .option('--pre-push-only', 'Only install pre-push hook')
    .option('--husky', 'Install hooks in .husky directory (for Husky v5+)')
    .action(
      async (options: {
        force?: boolean;
        preCommitOnly?: boolean;
        prePushOnly?: boolean;
        husky?: boolean;
      }) => {
        log(
          `hooks install: force=${options.force} husky=${options.husky} preCommitOnly=${options.preCommitOnly} prePushOnly=${options.prePushOnly}`
        );
        const spinner = ora('Installing Git hooks...').start();

        try {
          const workspaceRoot = getWorkspaceRoot();
          const hookInstaller = new HookInstaller();

          // Warn on Windows about limited shell hook support
          if (process.platform === 'win32') {
            console.error(
              chalk.yellow(
                '\nWarning: Git hooks use POSIX shell scripts which require' +
                  '\na compatible shell (Git Bash, WSL, or MSYS2) on Windows.' +
                  '\nHooks may not work in PowerShell or cmd.exe.\n'
              )
            );
          }

          // Check for Git repository (handles worktrees where .git is a file)
          const gitDir = resolveGitDir(workspaceRoot);
          if (!gitDir) {
            spinner.fail(chalk.red('Not a Git repository'));
            error('Run this command from a Git repository root');
            throw new CliError('Not a Git repository');
          }

          // Detect Husky
          const husky = detectHusky(workspaceRoot);

          // Determine hooks directory
          let hooksDir: string;

          if (options.husky || (husky.detected && husky.huskyDir)) {
            // Use Husky directory
            hooksDir = husky.huskyDir || join(workspaceRoot, '.husky');

            if (!existsSync(hooksDir)) {
              mkdirSync(hooksDir, { recursive: true });
            }

            spinner.text = 'Installing hooks in .husky directory...';

            if (!options.husky && husky.detected) {
              console.error(
                chalk.yellow('\n⚠️  Husky detected - installing hooks in .husky directory')
              );
              console.error(
                chalk.gray('  Use --husky flag to explicitly enable Husky integration\n')
              );
            }
          } else {
            // Use standard .git/hooks directory
            hooksDir = join(gitDir, 'hooks');

            if (!existsSync(hooksDir)) {
              mkdirSync(hooksDir, { recursive: true });
            }
          }

          const results: Array<{
            hook: string;
            result: ReturnType<typeof hookInstaller.installTo>;
          }> = [];

          // Install pre-commit hook
          if (!options.prePushOnly) {
            const result = hookInstaller.installTo(hooksDir, 'pre-commit', {
              force: !!options.force,
            });
            results.push({ hook: 'pre-commit', result });
          }

          // Install pre-push hook
          if (!options.preCommitOnly) {
            const result = hookInstaller.installTo(hooksDir, 'pre-push', {
              force: !!options.force,
            });
            results.push({ hook: 'pre-push', result });
          }

          spinner.succeed('Git hooks installation complete');

          // Display results
          console.error('');
          for (const { result } of results) {
            if (result.success) {
              if (result.action === 'created') {
                console.error(chalk.green(`  ✓ ${result.message}`));
              } else if (result.action === 'updated') {
                console.error(chalk.blue(`  ↻ ${result.message}`));
              }
            } else {
              console.error(chalk.yellow(`  ⚠ ${result.message}`));
            }
          }

          // Show usage info
          console.error('');
          info('Hooks installed successfully');
          console.error(chalk.gray('  pre-commit: Validates planning documents'));
          console.error(chalk.gray('  pre-push: Runs quality gates'));
          console.error('');
          console.error(chalk.gray('To bypass hooks temporarily:'));
          console.error(chalk.cyan('  ANVIL_SKIP_HOOKS=1 git push'));
        } catch (err) {
          if (err instanceof CliError) throw err;
          const msg = err instanceof Error ? err.message : 'Unknown error';
          spinner.fail(chalk.red(`Hook installation failed: ${msg}`));
          throw new CliError(msg);
        }
      }
    );

  // Uninstall subcommand
  command
    .command('uninstall')
    .description('Remove Anvil Git hooks')
    .option('--pre-commit-only', 'Only remove pre-commit hook')
    .option('--pre-push-only', 'Only remove pre-push hook')
    .action(async (options: { preCommitOnly?: boolean; prePushOnly?: boolean }) => {
      log(
        `hooks uninstall: preCommitOnly=${options.preCommitOnly} prePushOnly=${options.prePushOnly}`
      );
      const spinner = ora('Removing Git hooks...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const hookInstaller = new HookInstaller();
        const gitDir = resolveGitDir(workspaceRoot);

        if (!gitDir) {
          spinner.fail(chalk.red('Not a Git repository'));
          throw new CliError('Not a Git repository');
        }

        // Check both standard and Husky directories
        const standardHooksDir = join(gitDir, 'hooks');
        const huskyDir = join(workspaceRoot, '.husky');

        const hooksDirs = [
          { dir: standardHooksDir, name: '.git/hooks' },
          { dir: huskyDir, name: '.husky' },
        ].filter(({ dir }) => existsSync(dir));

        const results: Array<{
          hook: string;
          dir: string;
          result: ReturnType<typeof hookInstaller.uninstallFrom>;
        }> = [];

        for (const { dir, name } of hooksDirs) {
          if (!options.prePushOnly) {
            const result = hookInstaller.uninstallFrom(dir, 'pre-commit');
            if (result.success || !result.message.includes('not found')) {
              results.push({ hook: 'pre-commit', dir: name, result });
            }
          }

          if (!options.preCommitOnly) {
            const result = hookInstaller.uninstallFrom(dir, 'pre-push');
            if (result.success || !result.message.includes('not found')) {
              results.push({ hook: 'pre-push', dir: name, result });
            }
          }
        }

        spinner.succeed('Git hooks removal complete');

        // Display results
        console.error('');
        for (const { result } of results) {
          if (result.success) {
            console.error(chalk.green(`  ✓ ${result.message}`));
          } else {
            console.error(chalk.yellow(`  ⚠ ${result.message}`));
          }
        }

        success('Anvil hooks removed');
      } catch (err) {
        if (err instanceof CliError) throw err;
        const msg = err instanceof Error ? err.message : 'Unknown error';
        spinner.fail(chalk.red(`Hook removal failed: ${msg}`));
        throw new CliError(msg);
      }
    });

  // Status subcommand
  command
    .command('status')
    .description('Show status of Anvil Git hooks')
    .option('--json', 'Output as JSON')
    .action(async (options: { json?: boolean }) => {
      log('hooks status');
      try {
        const workspaceRoot = getWorkspaceRoot();
        const hookInstaller = new HookInstaller();
        const gitDir = resolveGitDir(workspaceRoot);

        if (!gitDir) {
          error('Not a Git repository');
          throw new CliError('Not a Git repository');
        }

        // Check standard hooks directory
        const standardHooksDir = join(gitDir, 'hooks');
        const huskyDir = join(workspaceRoot, '.husky');

        const locations = [
          { dir: standardHooksDir, name: '.git/hooks', exists: existsSync(standardHooksDir) },
          { dir: huskyDir, name: '.husky', exists: existsSync(huskyDir) },
        ];

        const hooks: Array<{
          location: string;
          hook: string;
          installed: boolean;
          anvilManaged: boolean;
        }> = [];

        for (const { dir, name, exists } of locations) {
          if (!exists) continue;
          for (const hookName of ['pre-commit', 'pre-push']) {
            const hookPath = join(dir, hookName);
            const installed = existsSync(hookPath);
            hooks.push({
              location: name,
              hook: hookName,
              installed,
              anvilManaged: installed && hookInstaller.isAnvilManagedHook(hookPath),
            });
          }
        }

        const husky = detectHusky(workspaceRoot);

        if (options.json) {
          console.log(JSON.stringify({ hooks, husky }, null, 2));
          return;
        }

        console.error(chalk.bold('\nAnvil Git Hooks Status\n'));

        for (const { dir, name, exists } of locations) {
          if (!exists) continue;

          console.error(chalk.cyan(`${name}:`));

          for (const hookName of ['pre-commit', 'pre-push']) {
            const hookPath = join(dir, hookName);

            if (!existsSync(hookPath)) {
              console.error(chalk.gray(`  ${hookName}: not installed`));
            } else if (hookInstaller.isAnvilManagedHook(hookPath)) {
              console.error(chalk.green(`  ${hookName}: ✓ installed (Anvil-managed)`));
            } else {
              console.error(chalk.yellow(`  ${hookName}: ⚠ exists (not Anvil-managed)`));
            }
          }
          console.error('');
        }

        // Show Husky detection
        if (husky.detected) {
          console.error(chalk.blue('ℹ Husky detected in this project'));
          if (husky.huskyDir) {
            console.error(chalk.gray(`  Using: ${husky.huskyDir}`));
          }
        }
      } catch (err) {
        if (err instanceof CliError) throw err;
        error(`${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      }
    });

  return command;
}
