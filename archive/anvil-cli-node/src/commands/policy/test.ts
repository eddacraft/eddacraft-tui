import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { error, warning, print, blank } from '../../utils/output.js';
import { PolicyLoader, OPAExecutor, getOPABinaryManager } from '@eddacraft/anvil-runtime';
import { DEFAULT_POLICY_DIR } from './constants.js';

export function createPolicyTestCommand(): Command {
  return new Command('test')
    .description('Run policy unit tests')
    .argument('[policy]')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .option('-v, --verbose', 'Show detailed test output')
    .action(async (policy: string | undefined, options: { dir: string; verbose?: boolean }) => {
      const spinner = ora('Running policy tests...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const policyDir = options.dir;

        const binaryManager = getOPABinaryManager();
        const binaryPath = await binaryManager.ensureBinary();

        const loader = new PolicyLoader();
        const discoveryResult = await loader.loadPolicies(workspaceRoot, { policyDir });

        if (discoveryResult.policies.length === 0) {
          spinner.warn('No policies found');
          print(chalk.dim('\nRun `anvil policy init` to create example policies'));
          return;
        }

        let policies = discoveryResult.policies;
        if (policy) {
          policies = policies.filter((p) => p.name === policy || p.name.includes(policy));
          if (policies.length === 0) {
            spinner.fail(`Policy '${policy}' not found`);
            throw new CliError(`Policy '${policy}' not found for testing`);
          }
        }

        const testFiles = loader.findTestFiles(discoveryResult.directory);
        if (testFiles.length === 0) {
          spinner.warn('No test files found');
          print(chalk.dim('\nCreate *_test.rego files to add tests'));
          return;
        }

        const executor = new OPAExecutor(binaryPath);
        const result = await executor.runTests(policies, testFiles);

        if (result.passed === 0 && result.failed === 0) {
          spinner.warn('No tests were executed');
          return;
        }

        const allPassed = result.failed === 0 && result.errors.length === 0;

        if (allPassed) {
          spinner.succeed(chalk.green(`All ${result.passed} tests passed`));
        } else {
          spinner.fail(chalk.red(`${result.failed} tests failed, ${result.passed} passed`));
        }

        if (options.verbose || !allPassed) {
          blank();
          for (const detail of result.details) {
            const icon = detail.passed ? chalk.green('✓') : chalk.red('✗');
            print(`  ${icon} ${detail.name}`);
            if (detail.message) {
              print(chalk.dim(`      ${detail.message}`));
            }
          }
        }

        if (result.errors.length > 0) {
          blank();
          warning('Errors occurred:');
          for (const err of result.errors) {
            print(chalk.red(`  • ${err}`));
          }
        }

        if (!allPassed) {
          throw new CliError('Policy tests failed');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Test run failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Policy test run failed');
      }
    });
}
