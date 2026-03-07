/**
 * Plan Command - APS planning document management
 *
 * Subcommands:
 * - anvil plan validate [path]        Validate a planning document
 * - anvil plan load [path]            Load and filter a planning document
 * - anvil plan lock <task>            Lock a task for execution
 * - anvil plan unlock <task>          Unlock (cancel) a locked task
 * - anvil plan status [task]          Show task states
 * - anvil plan create <intent>        Create a new execution plan (legacy)
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { CliError } from '../utils/cli-error.js';
import {
  APSPlan,
  generatePlanId,
  generateHash,
  APS_SCHEMA_VERSION,
  createDebugger,
  validatePathWithinRoot,
} from '@eddacraft/anvil-core';
import { savePlan, getWorkspaceRoot } from '../utils/file-io.js';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { data, print } from '../utils/output.js';
import {
  createValidateSubcommand,
  createLoadSubcommand,
  createLockSubcommand,
  createUnlockSubcommand,
  createStatusSubcommand,
} from './plan/index.js';

const log = createDebugger('cli');

/** Read a git field, returning '' on failure (e.g. not a git repo). */
function gitField(...args: string[]): string {
  try {
    return execFileSync('git', args, {
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'ignore'],
      timeout: 30_000,
    }).trim();
  } catch {
    return '';
  }
}

/**
 * Legacy create command (for backward compatibility)
 */
function createCreateSubcommand(): Command {
  return new Command('create')
    .description('Create a new Anvil execution plan (legacy)')
    .argument('<intent>', 'What you want to achieve (10-500 characters)')
    .option('-f, --format <format>', 'Output format (json|yaml)', 'json')
    .option('-o, --output <path>', 'Output file path')
    .option('--json', 'Output plan as JSON to stdout (no file written)')
    .action(
      async (intent: string, options: { format: string; output?: string; json?: boolean }) => {
        log(`plan create: intent length=${intent.length} format=${options.format}`);
        const spinner = ora('Creating plan...').start();

        try {
          // Validate intent length
          if (intent.length < 10) {
            throw new Error('Intent must be at least 10 characters long');
          }
          if (intent.length > 500) {
            throw new Error('Intent must not exceed 500 characters');
          }

          // Generate plan ID
          const planId = generatePlanId();

          // Create the plan structure
          const plan: Omit<APSPlan, 'hash'> = {
            schema_version: APS_SCHEMA_VERSION,
            id: planId,
            intent,
            proposed_changes: [],
            provenance: {
              timestamp: new Date().toISOString(),
              author: process.env['USER'] || 'unknown',
              source: 'cli',
              version: '0.0.0',
              repository: process.cwd(),
              branch: gitField('rev-parse', '--abbrev-ref', 'HEAD'),
              commit: gitField('rev-parse', '--short', 'HEAD'),
            },
            validations: {
              required_checks: ['lint', 'test', 'coverage', 'secrets'],
              skip_checks: [],
            },
            evidence: [],
            executions: [],
          };

          // Generate hash (excluding the hash field itself)
          const hash = generateHash(plan);

          // Add hash to the plan
          const completePlan: APSPlan = {
            ...plan,
            hash,
          } as APSPlan;

          if (options.json) {
            spinner.stop();
            data(JSON.stringify(completePlan, null, 2));
            return;
          }

          // Determine output path
          const workspaceRoot = getWorkspaceRoot();
          const defaultPath = join(
            workspaceRoot,
            '.anvil',
            'executions',
            `${planId}.${options.format}`
          );
          const rawPath = options.output || defaultPath;
          const outputPath = validatePathWithinRoot(rawPath, workspaceRoot);

          // Save the plan
          savePlan(completePlan, outputPath);

          log(`plan created: id=${planId} path=${outputPath}`);
          spinner.succeed(chalk.green(`✓ Plan created successfully`));
          print(chalk.gray('  ID:     '), chalk.cyan(planId));
          print(chalk.gray('  Hash:   '), chalk.cyan(hash.substring(0, 16) + '...'));
          print(chalk.gray('  Path:   '), chalk.cyan(outputPath));
          print(chalk.gray('  Intent: '), chalk.white(intent));
        } catch (error) {
          if (error instanceof CliError) throw error;
          spinner.fail(chalk.red('Failed to create plan'));
          const msg = error instanceof Error ? error.message : String(error);
          print(chalk.red('Error:'), msg);
          throw new CliError(msg);
        }
      }
    );
}

export function createPlanCommand(): Command {
  const planCommand = new Command('plan')
    .description('APS planning document management')
    .addCommand(createValidateSubcommand())
    .addCommand(createLoadSubcommand())
    .addCommand(createLockSubcommand())
    .addCommand(createUnlockSubcommand())
    .addCommand(createStatusSubcommand())
    .addCommand(createCreateSubcommand());

  return planCommand;
}
