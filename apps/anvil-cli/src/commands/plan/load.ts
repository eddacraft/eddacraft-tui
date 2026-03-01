/**
 * Plan Load Command
 * Loads and filters an APS planning document
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'node:path';
import {
  loadPlan,
  filterPlan,
  buildContextBundleJSON,
  buildContextBundleText,
  type FilterCriteria,
  type LoadedPlan,
  type FilteredPlan,
} from '@eddacraft/anvil-aps';
import { CliError, CliExit } from '../../utils/cli-error.js';

export interface LoadOptions {
  scope?: string[];
  module?: string[];
  task?: string[];
  owner?: string[];
  tag?: string[];
  priority?: string[];
  confidence?: string[];
  json?: boolean;
  filesOnly?: boolean;
  text?: boolean;
}

/**
 * Format filtered plan as JSON
 */
function formatAsJson(filtered: FilteredPlan): string {
  const bundle = buildContextBundleJSON(filtered);
  return JSON.stringify(bundle, null, 2);
}

/**
 * Format filtered plan as text (for LLM context)
 */
function formatAsText(filtered: FilteredPlan): string {
  return buildContextBundleText(filtered);
}

/**
 * Format as files only
 */
function formatFilesOnly(filtered: FilteredPlan): string {
  const files = new Set<string>();

  // Add module files
  for (const module of filtered.modules) {
    files.add(module.resolvedPath);
  }

  // Add task source files
  for (const task of filtered.tasks) {
    if (task.sourcePath) {
      files.add(task.sourcePath);
    }
  }

  return Array.from(files).join('\n');
}

/**
 * Format human-readable summary
 */
function formatSummary(plan: LoadedPlan, filtered: FilteredPlan, criteria: FilterCriteria): void {
  console.log('');
  console.log(chalk.bold('Plan:'), plan.title);
  console.log(chalk.bold('Root:'), plan.rootPath);
  console.log(chalk.bold('Type:'), plan.isMultiModule ? 'Multi-module' : 'Single-file');
  console.log('');

  // Show applied filters
  const appliedFilters: string[] = [];
  if (criteria.scopes?.length) appliedFilters.push(`scope: ${criteria.scopes.join(', ')}`);
  if (criteria.modules?.length) appliedFilters.push(`module: ${criteria.modules.join(', ')}`);
  if (criteria.tasks?.length) appliedFilters.push(`task: ${criteria.tasks.join(', ')}`);
  if (criteria.owners?.length) appliedFilters.push(`owner: ${criteria.owners.join(', ')}`);
  if (criteria.tags?.length) appliedFilters.push(`tag: ${criteria.tags.join(', ')}`);
  if (criteria.priorities?.length)
    appliedFilters.push(`priority: ${criteria.priorities.join(', ')}`);
  if (criteria.confidences?.length)
    appliedFilters.push(`confidence: ${criteria.confidences.join(', ')}`);

  if (appliedFilters.length > 0) {
    console.log(chalk.bold('Filters:'), appliedFilters.join(' | '));
    console.log('');
  }

  // Show modules
  console.log(chalk.bold.underline('Modules'));
  if (filtered.modules.length === 0) {
    console.log(chalk.gray('  (no matching modules)'));
  } else {
    for (const module of filtered.modules) {
      const owner = module.metadata.owner ? chalk.gray(` (${module.metadata.owner})`) : '';
      const status = module.metadata.priority ? chalk.cyan(` [${module.metadata.priority}]`) : '';
      console.log(`  ${chalk.green(module.id)}${owner}${status}`);
      console.log(chalk.gray(`    ${module.resolvedPath}`));
    }
  }
  console.log('');

  // Show tasks
  console.log(chalk.bold.underline('Tasks'));
  if (filtered.tasks.length === 0) {
    console.log(chalk.gray('  (no matching tasks)'));
  } else {
    for (const task of filtered.tasks) {
      const confidence =
        task.confidence === 'low'
          ? chalk.yellow(`[${task.confidence}]`)
          : task.confidence === 'high'
            ? chalk.green(`[${task.confidence}]`)
            : chalk.gray(`[${task.confidence}]`);
      console.log(`  ${chalk.cyan(task.id)}: ${task.title} ${confidence}`);
      console.log(
        chalk.gray(
          `    Intent: ${task.intent.substring(0, 80)}${task.intent.length > 80 ? '...' : ''}`
        )
      );
    }
  }
  console.log('');

  // Summary
  console.log(
    chalk.bold('Summary:'),
    `${filtered.modules.length} module(s), ${filtered.tasks.length} task(s)`
  );
}

/**
 * Type guard to check if value is a valid priority
 */
function isValidPriority(value: string): value is 'low' | 'medium' | 'high' {
  return value === 'low' || value === 'medium' || value === 'high';
}

/**
 * Type guard to check if array contains only valid priorities
 */
function isValidPriorityArray(arr: string[]): arr is Array<'low' | 'medium' | 'high'> {
  return arr.every(isValidPriority);
}

/**
 * Type guard to check if value is a valid confidence level
 */
function isValidConfidence(value: string): value is 'low' | 'medium' | 'high' {
  return value === 'low' || value === 'medium' || value === 'high';
}

/**
 * Type guard to check if array contains only valid confidence levels
 */
function isValidConfidenceArray(arr: string[]): arr is Array<'low' | 'medium' | 'high'> {
  return arr.every(isValidConfidence);
}

export function createLoadSubcommand(): Command {
  return new Command('load')
    .description('Load and filter an APS planning document')
    .argument('[path]', 'Path to planning document', 'docs/plans/APS.md')
    .option('--scope <scopes...>', 'Filter by scope (e.g., AUTH, PAY)')
    .option('--module <modules...>', 'Filter by module ID (e.g., auth, payments)')
    .option('--task <tasks...>', 'Filter by task ID (e.g., AUTH-001)')
    .option('--owner <owners...>', 'Filter by owner (e.g., @alice)')
    .option('--tag <tags...>', 'Filter by tag (e.g., security, api)')
    .option('--priority <priorities...>', 'Filter by priority (low, medium, high)')
    .option('--confidence <confidences...>', 'Filter by confidence (low, medium, high)')
    .option('--json', 'Output as JSON context bundle')
    .option('--text', 'Output as text context bundle (for LLM)')
    .option('--files-only', 'Output only file paths')
    .action(async (path: string, options: LoadOptions) => {
      const filePath = resolve(path);

      // Build filter criteria
      const criteria: FilterCriteria = {};
      if (options.scope) criteria.scopes = options.scope;
      if (options.module) criteria.modules = options.module;
      if (options.task) criteria.tasks = options.task;
      if (options.owner) criteria.owners = options.owner;
      if (options.tag) criteria.tags = options.tag;
      if (options.priority) {
        if (isValidPriorityArray(options.priority)) {
          criteria.priorities = options.priority;
        } else {
          const msg = `Invalid priority values. Must be one of: low, medium, high. Got: ${options.priority.join(', ')}`;
          console.error(chalk.red('Error:'), msg);
          throw new CliError(msg);
        }
      }
      if (options.confidence) {
        if (isValidConfidenceArray(options.confidence)) {
          criteria.confidences = options.confidence;
        } else {
          const msg = `Invalid confidence values. Must be one of: low, medium, high. Got: ${options.confidence.join(', ')}`;
          console.error(chalk.red('Error:'), msg);
          throw new CliError(msg);
        }
      }

      // Determine output mode
      const isStructuredOutput = options.json || options.text || options.filesOnly;

      if (isStructuredOutput) {
        // Structured output - no spinner
        try {
          const plan = await loadPlan(filePath);
          const filtered = filterPlan(plan, criteria);

          if (options.json) {
            console.log(formatAsJson(filtered));
          } else if (options.text) {
            console.log(formatAsText(filtered));
          } else if (options.filesOnly) {
            console.log(formatFilesOnly(filtered));
          }
          throw new CliExit();
        } catch (error) {
          if (error instanceof CliError || error instanceof CliExit) throw error;
          console.error(error instanceof Error ? error.message : String(error));
          throw new CliError(error instanceof Error ? error.message : String(error));
        }
      } else {
        // Human-readable mode
        const spinner = ora(`Loading ${path}...`).start();

        try {
          const plan = await loadPlan(filePath);
          const filtered = filterPlan(plan, criteria);

          spinner.succeed(chalk.green('Plan loaded'));
          formatSummary(plan, filtered, criteria);
        } catch (error) {
          if (error instanceof CliError || error instanceof CliExit) throw error;
          spinner.fail(chalk.red('Failed to load plan'));
          console.error(
            chalk.red('Error:'),
            error instanceof Error ? error.message : String(error)
          );
          throw new CliError(error instanceof Error ? error.message : 'Failed to load plan');
        }
      }
    });
}
