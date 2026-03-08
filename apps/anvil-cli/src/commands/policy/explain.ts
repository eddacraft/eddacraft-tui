import { readFileSync } from 'node:fs';
import { Command } from 'commander';
import chalk from 'chalk';
import { createDebugger } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { error, print, blank, debug } from '../../utils/output.js';
import { PolicyConfigManager } from '../../services/policy-config.js';
import { formatEnforcement, formatSource } from './formatting.js';

const log = createDebugger('cli');

export function createPolicyExplainCommand(): Command {
  return new Command('explain')
    .description('Show detailed explanation for a policy')
    .argument('<name>')
    .action(async (name: string) => {
      log(`policy explain: name=${name}`);
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);
        const resolved = configMgr.resolvePolicies();
        const policy = resolved.find((p) => p.name === name);

        if (!policy) {
          error(`Policy '${name}' not found`);
          print(chalk.dim('\nRun `anvil policy list --all` to see available policies'));
          throw new CliError(`Policy '${name}' not found`);
        }

        blank();
        print(chalk.bold(`Policy: ${policy.name}`));
        print(chalk.dim('─'.repeat(50)));
        blank();

        print(`  ${chalk.bold('Source:')}        ${formatSource(policy.source)}`);
        print(`  ${chalk.bold('Enforcement:')}   ${formatEnforcement(policy.enforcement)}`);
        print(
          `  ${chalk.bold('Status:')}        ${policy.active ? chalk.green('active') : chalk.yellow('inactive')}`
        );

        if (policy.owner) {
          print(`  ${chalk.bold('Owner:')}         ${policy.owner}`);
        }

        if (policy.effective) {
          const effectiveDate = new Date(policy.effective);
          const isEffective = effectiveDate <= new Date();
          print(
            `  ${chalk.bold('Effective:')}     ${policy.effective} ${isEffective ? chalk.green('(in effect)') : chalk.yellow('(pending)')}`
          );
        }

        if (policy.tags && policy.tags.length > 0) {
          print(`  ${chalk.bold('Tags:')}          ${policy.tags.join(', ')}`);
        }

        if (policy.reason) {
          blank();
          print(chalk.bold('  Why this policy exists:'));
          print(`  ${policy.reason}`);
        }

        if (policy.hasRegoFile && policy.regoPath) {
          blank();
          print(chalk.bold('  Rego file:'));
          print(chalk.dim(`  ${policy.regoPath}`));

          try {
            const content = readFileSync(policy.regoPath, 'utf-8');
            const commentLines = content
              .split('\n')
              .filter((line) => line.startsWith('#'))
              .slice(0, 5)
              .map((line) => line.replace(/^#\s?/, ''));

            if (commentLines.length > 0) {
              blank();
              print(chalk.bold('  Description (from source):'));
              for (const line of commentLines) {
                print(chalk.dim(`  ${line}`));
              }
            }
          } catch {
            debug('policy: failed to read policy source for description');
          }
        }

        blank();
        print(chalk.dim('  Commands:'));
        print(chalk.dim(`    anvil policy disable ${name}    # turn it off`));
        print(chalk.dim(`    anvil gate --skip ${name}       # skip just this once`));
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to explain policy: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to explain policy');
      }
    });
}
