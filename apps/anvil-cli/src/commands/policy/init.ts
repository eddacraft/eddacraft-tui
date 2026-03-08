import { join } from 'node:path';
import { existsSync, mkdirSync, readdirSync, copyFileSync, writeFileSync } from 'node:fs';
import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { validatePathWithinRoot } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { success, error, print, blank } from '../../utils/output.js';
import { DEFAULT_POLICY_DIR, EXAMPLE_POLICIES, getExamplePoliciesPath } from './constants.js';

export function createPolicyInitCommand(): Command {
  return new Command('init')
    .description('Initialise policy directory with example policies')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .option('--force', 'Overwrite existing policies')
    .action(async (options: { dir: string; force?: boolean }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const policyDir = validatePathWithinRoot(options.dir, workspaceRoot);

        if (existsSync(policyDir) && !options.force) {
          const files = readdirSync(policyDir).filter((f) => f.endsWith('.rego'));
          if (files.length > 0) {
            error(`Policy directory already contains ${files.length} policies`);
            print(chalk.dim('\nUse --force to overwrite existing policies'));
            throw new CliError('Policy directory already contains policies');
          }
        }

        const spinner = ora('Creating policy directory...').start();

        if (!existsSync(policyDir)) {
          mkdirSync(policyDir, { recursive: true });
        }

        const fixturesPath = getExamplePoliciesPath();
        let copiedCount = 0;

        if (fixturesPath && existsSync(fixturesPath)) {
          const fixtures = readdirSync(fixturesPath).filter((f) => f.endsWith('.rego'));
          for (const file of fixtures) {
            const src = join(fixturesPath, file);
            const dest = join(policyDir, file);
            copyFileSync(src, dest);
            copiedCount++;
          }
        } else {
          for (const [filename, content] of Object.entries(EXAMPLE_POLICIES)) {
            const dest = join(policyDir, filename);
            writeFileSync(dest, content, 'utf-8');
            copiedCount++;
          }
        }

        spinner.succeed(chalk.green(`Created ${copiedCount} example policies`));

        print('\n' + chalk.bold('Created policies:'));
        print(chalk.cyan('  • coverage_min.rego') + chalk.dim(' - Enforce minimum test coverage'));
        print(chalk.cyan('  • change_scope.rego') + chalk.dim(' - Limit files per change'));
        print(
          chalk.cyan('  • security_baseline.rego') + chalk.dim(' - Security review requirements')
        );

        print('\n' + chalk.bold('Next steps:'));
        print(chalk.dim('  1. Review and customise policies in ') + chalk.cyan(options.dir));
        print(chalk.dim('  2. List policies: ') + chalk.cyan('anvil policy list'));
        print(chalk.dim('  3. Run policy tests: ') + chalk.cyan('anvil policy test'));
        print(chalk.dim('  4. Enable policy check in .anvilrc'));

        print('\n' + chalk.bold('Add to .anvilrc:'));
        print(
          chalk.dim(`  {
            "name": "policy",
            "enabled": true,
            "config": {
              "policy_dir": "${options.dir}",
              "severity_threshold": "error"
            }
          }`)
        );

        blank();
        success('Policy directory initialised!');
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(
          `Failed to initialise policies: ${err instanceof Error ? err.message : 'Unknown error'}`
        );
        throw new CliError('Failed to initialise policies');
      }
    });
}
