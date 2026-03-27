import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { validatePathWithinRoot } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { error, print } from '../../utils/output.js';
import { OPAExecutor, getOPABinaryManager } from '@eddacraft/anvil-runtime';

export function createPolicyValidateCommand(): Command {
  return new Command('validate')
    .description('Validate Rego syntax for a policy file')
    .argument('<file>')
    .action(async (file: string) => {
      const spinner = ora('Validating policy syntax...').start();

      try {
        const { resolve } = await import('node:path');
        const workspaceRoot = getWorkspaceRoot();
        const absolutePath = resolve(file);
        const validatedPath = validatePathWithinRoot(absolutePath, workspaceRoot);

        const binaryManager = getOPABinaryManager();
        const binaryPath = await binaryManager.ensureBinary();

        const { readFile } = await import('node:fs/promises');
        const content = await readFile(validatedPath, 'utf-8');

        const executor = new OPAExecutor(binaryPath);
        const result = await executor.validateSyntax(content);

        if (result.valid) {
          spinner.succeed(chalk.green('Policy syntax is valid'));
        } else {
          spinner.fail(chalk.red('Policy syntax is invalid'));
          for (const err of result.errors) {
            print(chalk.red(`  • ${err}`));
          }
          throw new CliError('Policy syntax is invalid');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Validation failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Policy syntax validation failed');
      }
    });
}
