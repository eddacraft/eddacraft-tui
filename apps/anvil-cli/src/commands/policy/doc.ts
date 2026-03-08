import { dirname } from 'node:path';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { Command } from 'commander';
import chalk from 'chalk';
import { validatePathWithinRoot } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { success, error, print } from '../../utils/output.js';
import { PolicyConfigManager } from '../../services/policy-config.js';

export function createPolicyDocCommand(): Command {
  return new Command('doc')
    .description('Generate .anvil/POLICIES.md from current policy configuration')
    .option('-o, --output <path>', 'Output file path', '.anvil/POLICIES.md')
    .action(async (options: { output: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);
        const markdown = configMgr.generatePoliciesDoc();

        const outputPath = validatePathWithinRoot(options.output, workspaceRoot);
        const outputDir = dirname(outputPath);
        if (!existsSync(outputDir)) {
          mkdirSync(outputDir, { recursive: true });
        }

        writeFileSync(outputPath, markdown, 'utf-8');
        success(`Generated ${options.output}`);
        print(chalk.dim('  Commit this file so the team can read it in any editor.'));
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to generate docs: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to generate policy docs');
      }
    });
}
