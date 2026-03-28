import { join } from 'node:path';
import { existsSync, mkdirSync, readdirSync, copyFileSync, writeFileSync } from 'node:fs';
import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { validatePathWithinRoot } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { error, print, blank } from '../../utils/output.js';
import { PolicyConfigManager } from '../../services/policy-config.js';
import { DEFAULT_POLICY_DIR } from './constants.js';

export function createPolicyScaffoldCommand(): Command {
  return new Command('scaffold')
    .description('Scaffold policy structure for org-wide sharing')
    .requiredOption('--org <name>', 'Organisation name')
    .option('--out <dir>', 'Output directory for org policy repo', './anvil-policies')
    .action(async (options: { org: string; out: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();

        let outDir: string;
        try {
          outDir = validatePathWithinRoot(options.out, workspaceRoot);
        } catch {
          error(`--out path escapes workspace: ${options.out}`);
          throw new CliError('Scaffold output path escapes workspace');
        }

        const configMgr = new PolicyConfigManager(workspaceRoot);
        const spinner = ora(`Scaffolding org policies for ${options.org}...`).start();

        mkdirSync(join(outDir, '.anvil', 'policies'), { recursive: true });

        const localPolicyDir = join(workspaceRoot, DEFAULT_POLICY_DIR);
        let copiedCount = 0;
        if (existsSync(localPolicyDir)) {
          const files = readdirSync(localPolicyDir).filter((f) => f.endsWith('.rego'));
          for (const file of files) {
            copyFileSync(join(localPolicyDir, file), join(outDir, '.anvil', 'policies', file));
            copiedCount++;
          }
        }

        const orgConfigYaml = configMgr.generateOrgScaffold(options.org);
        writeFileSync(join(outDir, '.anvil', 'config.yml'), orgConfigYaml, 'utf-8');

        const readme = [
          `# ${options.org} Anvil Policies`,
          '',
          'Shared policy repository for org-wide Anvil policies.',
          '',
          '## Usage',
          '',
          "In your project's `.anvil/config.yml`:",
          '',
          '```yaml',
          'policies:',
          '  org:',
          `    source: "git@github.com:${options.org}/anvil-policies.git"`,
          '    ref: "v1.0.0"',
          '```',
          '',
          'Then run:',
          '',
          '```sh',
          `anvil init --org ${options.org}`,
          '```',
          '',
          '## Policies',
          '',
          'Run `anvil policy list` to see all active policies.',
          'Run `anvil policy doc` to regenerate POLICIES.md.',
          '',
        ].join('\n');
        writeFileSync(join(outDir, 'README.md'), readme, 'utf-8');

        spinner.succeed(`Scaffolded org policy repo at ${options.out}`);

        blank();
        print(chalk.bold('Created:'));
        print(chalk.dim(`  ${options.out}/.anvil/config.yml`));
        print(chalk.dim(`  ${options.out}/.anvil/policies/ (${copiedCount} policies)`));
        print(chalk.dim(`  ${options.out}/README.md`));

        blank();
        print(chalk.bold('Next steps:'));
        print(chalk.dim(`  1. cd ${options.out}`));
        print(chalk.dim('  2. git init && git add . && git commit -m "Initial policy repo"'));
        print(chalk.dim(`  3. Push to git@github.com:${options.org}/anvil-policies.git`));
        print(chalk.dim('  4. In your project, add the org source to .anvil/config.yml'));
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to scaffold: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to scaffold org policies');
      }
    });
}
