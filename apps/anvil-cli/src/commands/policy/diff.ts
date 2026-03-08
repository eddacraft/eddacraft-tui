import { join } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { error, info, print, blank, debug } from '../../utils/output.js';
import { DEFAULT_POLICY_DIR } from './constants.js';

export function createPolicyDiffCommand(): Command {
  return new Command('diff')
    .description('Show policy changes since last sync or commit')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .action(async (options: { dir: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { execFileSync } = await import('node:child_process');

        const policyDir = options.dir;
        const configPath = join('.anvil', 'config.yml');

        print(chalk.bold('\nPolicy Changes:\n'));

        let hasChanges = false;

        const printDiffSection = (label: string, diffOutput: string) => {
          if (!diffOutput) return;
          hasChanges = true;
          print(chalk.bold(`  ${label}:`));
          for (const line of diffOutput.split('\n')) {
            const [status, ...pathParts] = line.split('\t');
            const filePath = pathParts.join('\t');
            const statusLabel =
              status === 'M'
                ? chalk.yellow('modified')
                : status === 'A'
                  ? chalk.green('added')
                  : status === 'D'
                    ? chalk.red('deleted')
                    : chalk.dim(status ?? '');
            print(`    ${statusLabel} ${filePath}`);
          }
          blank();
        };

        try {
          const configDiff = execFileSync(
            'git',
            ['diff', '--name-status', 'HEAD', '--', configPath],
            {
              cwd: workspaceRoot,
              encoding: 'utf-8',
              timeout: 30_000,
            }
          ).trim();
          printDiffSection('Config changes', configDiff);
        } catch {
          debug('policy: git diff for config.yml failed');
        }

        try {
          const policyDiff = execFileSync(
            'git',
            ['diff', '--name-status', 'HEAD', '--', policyDir],
            {
              cwd: workspaceRoot,
              encoding: 'utf-8',
              timeout: 30_000,
            }
          ).trim();
          printDiffSection('Policy file changes', policyDiff);
        } catch {
          debug('policy: git diff for policy directory failed');
        }

        try {
          const untracked = execFileSync(
            'git',
            ['ls-files', '--others', '--exclude-standard', '--', policyDir],
            {
              cwd: workspaceRoot,
              encoding: 'utf-8',
              timeout: 30_000,
            }
          ).trim();
          if (untracked) {
            hasChanges = true;
            print(chalk.bold('  Untracked policy files:'));
            for (const file of untracked.split('\n')) {
              print(`    ${chalk.green('new')} ${file}`);
            }
            blank();
          }
        } catch {
          debug('policy: git ls-files for untracked policy files failed');
        }

        if (!hasChanges) {
          info('No policy changes detected');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to diff policies: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to diff policies');
      }
    });
}
