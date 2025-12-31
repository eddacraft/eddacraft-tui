/**
 * Hooks Command - Manage Git hooks for Anvil integration
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync, readFileSync, writeFileSync, mkdirSync, unlinkSync, chmodSync } from 'fs';
import { join } from 'path';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { success, error, info } from '../utils/output.js';

/** Hook script content for pre-commit */
const PRE_COMMIT_HOOK = `#!/bin/sh
# Anvil pre-commit hook
# Validates planning documents before commit

# Find modified plan files
PLAN_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\\.(md|yaml|yml|json)$' || true)

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Validating planning documents..."

  for file in $PLAN_FILES; do
    if anvil validate "$file" --quiet 2>/dev/null; then
      echo "  ✓ $file"
    fi
  done
fi

exit 0
`;

/** Hook script content for pre-push */
const PRE_PUSH_HOOK = `#!/bin/sh
# Anvil pre-push hook
# Runs quality gates before push

# Check for ANVIL_SKIP_HOOKS environment variable
if [ -n "$ANVIL_SKIP_HOOKS" ]; then
  echo "Anvil: Skipping hooks (ANVIL_SKIP_HOOKS is set)"
  exit 0
fi

# Find plan files in the repository
PLAN_FILES=$(find . -name "*.md" -path "*/planning/*" -o -name "*-plan.md" -o -name "*-prd.md" 2>/dev/null | head -5)

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Running quality gates..."

  for file in $PLAN_FILES; do
    if [ -f "$file" ]; then
      echo "  Checking: $file"
      if ! anvil gate "$file" --quiet 2>/dev/null; then
        echo "  ✗ Gate failed: $file"
        echo ""
        echo "Run 'anvil gate $file' to see details."
        echo "To bypass, set ANVIL_SKIP_HOOKS=1"
        exit 1
      fi
    fi
  done

  echo "  ✓ All gates passed"
fi

exit 0
`;

/** Marker comment to identify Anvil-managed hooks */
const ANVIL_MARKER = '# Anvil-managed hook';

/**
 * Check if a hook file contains the Anvil marker
 */
function isAnvilManagedHook(hookPath: string): boolean {
  if (!existsSync(hookPath)) return false;
  const content = readFileSync(hookPath, 'utf-8');
  return content.includes(ANVIL_MARKER);
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

  if (existsSync(packageJsonPath)) {
    try {
      const pkg = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
      hasHuskyDep = !!(pkg.devDependencies?.husky || pkg.dependencies?.husky);
    } catch {
      // Ignore parse errors
    }
  }

  return {
    detected: huskyDirExists || hasHuskyDep,
    huskyDir: huskyDirExists ? huskyDir : null,
  };
}

/**
 * Install a Git hook
 */
function installHook(
  hooksDir: string,
  hookName: string,
  hookContent: string,
  force: boolean
): { success: boolean; message: string; action: 'created' | 'updated' | 'skipped' } {
  const hookPath = join(hooksDir, hookName);

  // Check if hook already exists
  if (existsSync(hookPath)) {
    const existingContent = readFileSync(hookPath, 'utf-8');

    // If it's already an Anvil hook, update it
    if (existingContent.includes(ANVIL_MARKER)) {
      writeFileSync(hookPath, `${ANVIL_MARKER}\n${hookContent}`, 'utf-8');
      chmodSync(hookPath, 0o755);
      return { success: true, message: `Updated ${hookName}`, action: 'updated' };
    }

    // If force is set, overwrite
    if (force) {
      // Backup existing hook
      const backupPath = `${hookPath}.anvil-backup`;
      writeFileSync(backupPath, existingContent, 'utf-8');
      writeFileSync(hookPath, `${ANVIL_MARKER}\n${hookContent}`, 'utf-8');
      chmodSync(hookPath, 0o755);
      return {
        success: true,
        message: `Replaced ${hookName} (backup: ${hookName}.anvil-backup)`,
        action: 'updated',
      };
    }

    // Otherwise, skip
    return {
      success: false,
      message: `${hookName} already exists (use --force to overwrite)`,
      action: 'skipped',
    };
  }

  // Create new hook
  writeFileSync(hookPath, `${ANVIL_MARKER}\n${hookContent}`, 'utf-8');
  chmodSync(hookPath, 0o755);
  return { success: true, message: `Created ${hookName}`, action: 'created' };
}

/**
 * Uninstall a Git hook
 */
function uninstallHook(hooksDir: string, hookName: string): { success: boolean; message: string } {
  const hookPath = join(hooksDir, hookName);

  if (!existsSync(hookPath)) {
    return { success: true, message: `${hookName} not found (already removed)` };
  }

  // Check if it's an Anvil-managed hook
  if (!isAnvilManagedHook(hookPath)) {
    return {
      success: false,
      message: `${hookName} is not managed by Anvil (skipped)`,
    };
  }

  // Remove the hook
  unlinkSync(hookPath);

  // Restore backup if exists
  const backupPath = `${hookPath}.anvil-backup`;
  if (existsSync(backupPath)) {
    const backupContent = readFileSync(backupPath, 'utf-8');
    writeFileSync(hookPath, backupContent, 'utf-8');
    chmodSync(hookPath, 0o755);
    unlinkSync(backupPath);
    return { success: true, message: `Removed ${hookName} (restored backup)` };
  }

  return { success: true, message: `Removed ${hookName}` };
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
        const spinner = ora('Installing Git hooks...').start();

        try {
          const workspaceRoot = getWorkspaceRoot();

          // Check for Git repository
          const gitDir = join(workspaceRoot, '.git');
          if (!existsSync(gitDir)) {
            spinner.fail(chalk.red('Not a Git repository'));
            error('Run this command from a Git repository root');
            process.exit(1);
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
              console.log(
                chalk.yellow('\n⚠️  Husky detected - installing hooks in .husky directory')
              );
              console.log(
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

          const results: Array<{ hook: string; result: ReturnType<typeof installHook> }> = [];

          // Install pre-commit hook
          if (!options.prePushOnly) {
            const result = installHook(hooksDir, 'pre-commit', PRE_COMMIT_HOOK, !!options.force);
            results.push({ hook: 'pre-commit', result });
          }

          // Install pre-push hook
          if (!options.preCommitOnly) {
            const result = installHook(hooksDir, 'pre-push', PRE_PUSH_HOOK, !!options.force);
            results.push({ hook: 'pre-push', result });
          }

          spinner.succeed('Git hooks installation complete');

          // Display results
          console.log('');
          for (const { result } of results) {
            if (result.success) {
              if (result.action === 'created') {
                console.log(chalk.green(`  ✓ ${result.message}`));
              } else if (result.action === 'updated') {
                console.log(chalk.blue(`  ↻ ${result.message}`));
              }
            } else {
              console.log(chalk.yellow(`  ⚠ ${result.message}`));
            }
          }

          // Show usage info
          console.log('');
          info('Hooks installed successfully');
          console.log(chalk.gray('  pre-commit: Validates planning documents'));
          console.log(chalk.gray('  pre-push: Runs quality gates'));
          console.log('');
          console.log(chalk.gray('To bypass hooks temporarily:'));
          console.log(chalk.cyan('  ANVIL_SKIP_HOOKS=1 git push'));
        } catch (err) {
          spinner.fail(chalk.red('Hook installation failed'));
          error(`${err instanceof Error ? err.message : 'Unknown error'}`);
          process.exit(1);
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
      const spinner = ora('Removing Git hooks...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const gitDir = join(workspaceRoot, '.git');

        if (!existsSync(gitDir)) {
          spinner.fail(chalk.red('Not a Git repository'));
          process.exit(1);
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
          result: ReturnType<typeof uninstallHook>;
        }> = [];

        for (const { dir, name } of hooksDirs) {
          if (!options.prePushOnly) {
            const result = uninstallHook(dir, 'pre-commit');
            if (result.success || !result.message.includes('not found')) {
              results.push({ hook: 'pre-commit', dir: name, result });
            }
          }

          if (!options.preCommitOnly) {
            const result = uninstallHook(dir, 'pre-push');
            if (result.success || !result.message.includes('not found')) {
              results.push({ hook: 'pre-push', dir: name, result });
            }
          }
        }

        spinner.succeed('Git hooks removal complete');

        // Display results
        console.log('');
        for (const { result } of results) {
          if (result.success) {
            console.log(chalk.green(`  ✓ ${result.message}`));
          } else {
            console.log(chalk.yellow(`  ⚠ ${result.message}`));
          }
        }

        success('Anvil hooks removed');
      } catch (err) {
        spinner.fail(chalk.red('Hook removal failed'));
        error(`${err instanceof Error ? err.message : 'Unknown error'}`);
        process.exit(1);
      }
    });

  // Status subcommand
  command
    .command('status')
    .description('Show status of Anvil Git hooks')
    .action(async () => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const gitDir = join(workspaceRoot, '.git');

        if (!existsSync(gitDir)) {
          error('Not a Git repository');
          process.exit(1);
        }

        console.log(chalk.bold('\nAnvil Git Hooks Status\n'));

        // Check standard hooks directory
        const standardHooksDir = join(gitDir, 'hooks');
        const huskyDir = join(workspaceRoot, '.husky');

        const locations = [
          { dir: standardHooksDir, name: '.git/hooks', exists: existsSync(standardHooksDir) },
          { dir: huskyDir, name: '.husky', exists: existsSync(huskyDir) },
        ];

        for (const { dir, name, exists } of locations) {
          if (!exists) continue;

          console.log(chalk.cyan(`${name}:`));

          for (const hookName of ['pre-commit', 'pre-push']) {
            const hookPath = join(dir, hookName);

            if (!existsSync(hookPath)) {
              console.log(chalk.gray(`  ${hookName}: not installed`));
            } else if (isAnvilManagedHook(hookPath)) {
              console.log(chalk.green(`  ${hookName}: ✓ installed (Anvil-managed)`));
            } else {
              console.log(chalk.yellow(`  ${hookName}: ⚠ exists (not Anvil-managed)`));
            }
          }
          console.log('');
        }

        // Show Husky detection
        const husky = detectHusky(workspaceRoot);
        if (husky.detected) {
          console.log(chalk.blue('ℹ Husky detected in this project'));
          if (husky.huskyDir) {
            console.log(chalk.gray(`  Using: ${husky.huskyDir}`));
          }
        }
      } catch (err) {
        error(`${err instanceof Error ? err.message : 'Unknown error'}`);
        process.exit(1);
      }
    });

  return command;
}
