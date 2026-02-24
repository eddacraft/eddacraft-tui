import { Command } from 'commander';
import chalk from 'chalk';
import { CliError } from '../utils/cli-error.js';
import {
  explainById,
  explainByRule,
  parseWarningId,
  isExplainable,
  getExplainableRules,
  createDebugger,
  type WarningExplanation,
} from '@eddacraft/anvil-core';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { loadRecentWarnings } from '../services/recent-warnings-store.js';

const log = createDebugger('cli');

interface ExplainOptions {
  list?: boolean;
  json?: boolean;
  rules?: boolean;
}

function formatSection(title: string, content: string): void {
  console.log('');
  console.log(chalk.bold.underline(title));
  console.log('');
  console.log(content.trim());
}

function formatExplanationText(
  explanation: WarningExplanation,
  context?: { file?: string; line?: number }
): void {
  console.log('');
  console.log(chalk.bold(`  Warning: ${explanation.ruleId} — ${explanation.title}`));

  if (context?.file && context?.line) {
    console.log(chalk.gray(`  File: ${context.file}:${context.line}`));
  }

  console.log('');
  console.log(chalk.gray('  ' + '─'.repeat(50)));

  formatSection(
    `  ${explanation.whyItMatters.title}`,
    indentContent(explanation.whyItMatters.content, '  ')
  );

  console.log('');
  console.log(chalk.gray('  ' + '─'.repeat(50)));

  formatSection(
    `  ${explanation.howToAddress.title}`,
    indentContent(explanation.howToAddress.content, '  ')
  );

  console.log('');
  console.log(chalk.gray('  ' + '─'.repeat(50)));

  formatSection(
    `  ${explanation.whenToSuppress.title}`,
    indentContent(explanation.whenToSuppress.content, '  ')
  );

  if (explanation.related) {
    console.log('');
    console.log(chalk.gray('  ' + '─'.repeat(50)));
    console.log('');
    console.log(chalk.bold.underline('  RELATED'));
    console.log('');
    if (explanation.related.documentation) {
      console.log(`  • Documentation: ${explanation.related.documentation}`);
    }
    if (explanation.related.ruleDefinition) {
      console.log(`  • Rule definition: ${explanation.related.ruleDefinition}`);
    }
    if (
      explanation.related.similarWarnings !== undefined &&
      explanation.related.similarWarnings > 0
    ) {
      console.log(`  • Similar warnings in this file: ${explanation.related.similarWarnings}`);
    }
  }

  console.log('');
}

function indentContent(content: string, indent: string): string {
  return content
    .split('\n')
    .map((line) => indent + line)
    .join('\n');
}

function formatExplanationJson(explanation: WarningExplanation): void {
  console.log(JSON.stringify(explanation, null, 2));
}

function listExplainableRules(json: boolean): void {
  const rules = getExplainableRules();

  if (json) {
    console.log(JSON.stringify({ rules }, null, 2));
    return;
  }

  console.log('');
  console.log(chalk.bold('Available rules with explanations:'));
  console.log('');

  console.log(chalk.yellow.bold('Anti-pattern rules:'));
  const apRules = rules.filter((r) => r.startsWith('AP-'));
  for (const rule of apRules) {
    console.log(`  ${rule}`);
  }

  console.log('');
  console.log(chalk.cyan.bold('Architecture rules:'));
  const archRules = rules.filter((r) => r.startsWith('ARCH-') || r.startsWith('BOUND-'));
  for (const rule of archRules) {
    console.log(`  ${rule}`);
  }

  console.log('');
  console.log(chalk.gray('Usage: anvil explain <rule-id>'));
  console.log(chalk.gray('       anvil explain AP-003'));
  console.log(chalk.gray('       anvil explain AP-003-src/file.ts:42'));
}

async function listRecentWarnings(json: boolean): Promise<void> {
  const workspaceRoot = getWorkspaceRoot();
  const warnings = await loadRecentWarnings(workspaceRoot);

  if (json) {
    console.log(JSON.stringify({ warnings }, null, 2));
    return;
  }

  console.log('');
  console.log(chalk.bold('Recent warnings (from last `anvil check` run):'));
  console.log('');

  if (warnings.length === 0) {
    console.log(chalk.gray('No recent warnings found. Run `anvil check` first.'));
    console.log('');
    return;
  }

  for (const warning of warnings) {
    const parsed = parseWarningId(warning.warningId);
    const rule = parsed?.rule ?? warning.warningId;
    console.log(chalk.yellow(`  ${warning.warningId}`));
    console.log(chalk.gray(`    ${rule} · ${warning.location.file}:${warning.location.line}`));
    console.log(`    ${warning.title}`);
    console.log('');
  }
}

async function handleExplainWarningId(warningId: string, options: ExplainOptions): Promise<void> {
  const workspaceRoot = getWorkspaceRoot();
  const recentWarnings = await loadRecentWarnings(workspaceRoot);

  const parsed = parseWarningId(warningId);

  if (parsed) {
    const explanation = explainById(warningId, recentWarnings);

    if (explanation) {
      if (options.json) {
        formatExplanationJson(explanation);
      } else {
        formatExplanationText(explanation, { file: parsed.file, line: parsed.line });
      }
      return;
    }
  }

  if (isExplainable(warningId)) {
    const explanation = explainByRule(warningId);
    if (explanation) {
      if (options.json) {
        formatExplanationJson(explanation);
      } else {
        formatExplanationText(explanation);
      }
      return;
    }
  }

  console.error(chalk.red(`Unknown warning ID or rule: ${warningId}`));
  console.log('');
  console.log(chalk.gray('Use --list to see recent warning IDs from check output'));
  console.log(chalk.gray('Use --rules to see all explainable rules'));
  console.log(chalk.gray('Format: RULE-ID or RULE-ID-file.ts:line'));
  console.log(chalk.gray('Examples:'));
  console.log(chalk.gray('  anvil explain AP-003'));
  console.log(chalk.gray('  anvil explain AP-003-src/utils.ts:42'));
  throw new CliError('Unknown warning ID or rule');
}

export function createExplainCommand(): Command {
  const command = new Command('explain');

  command
    .description('Get detailed explanation for a warning')
    .argument('[warning-id]', 'Warning ID (e.g., AP-003, AP-003-src/file.ts:42)')
    .option('--list', 'List warning IDs from the most recent check run')
    .option('--rules', 'List all explainable rules')
    .option('--json', 'Output as JSON')
    .action(async (warningId: string | undefined, options: ExplainOptions) => {
      log(
        `explain command entered: warningId=${warningId ?? '(none)'} list=${options.list} rules=${options.rules}`
      );
      if (options.rules) {
        listExplainableRules(options.json ?? false);
        return;
      }

      if (options.list || !warningId) {
        await listRecentWarnings(options.json ?? false);
        return;
      }

      await handleExplainWarningId(warningId, options);
    });

  return command;
}
