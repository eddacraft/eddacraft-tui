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
import { print, blank, data } from '../utils/output.js';

const log = createDebugger('cli');

interface ExplainOptions {
  list?: boolean;
  json?: boolean;
  rules?: boolean;
}

function formatSection(title: string, content: string): void {
  blank();
  print(chalk.bold.underline(title));
  blank();
  print(content.trim());
}

function formatExplanationText(
  explanation: WarningExplanation,
  context?: { file?: string; line?: number }
): void {
  blank();
  print(chalk.bold(`  Warning: ${explanation.ruleId} — ${explanation.title}`));

  if (context?.file && context?.line) {
    print(chalk.gray(`  File: ${context.file}:${context.line}`));
  }

  blank();
  print(chalk.gray('  ' + '─'.repeat(50)));

  formatSection(
    `  ${explanation.whyItMatters.title}`,
    indentContent(explanation.whyItMatters.content, '  ')
  );

  blank();
  print(chalk.gray('  ' + '─'.repeat(50)));

  formatSection(
    `  ${explanation.howToAddress.title}`,
    indentContent(explanation.howToAddress.content, '  ')
  );

  blank();
  print(chalk.gray('  ' + '─'.repeat(50)));

  formatSection(
    `  ${explanation.whenToSuppress.title}`,
    indentContent(explanation.whenToSuppress.content, '  ')
  );

  if (explanation.related) {
    blank();
    print(chalk.gray('  ' + '─'.repeat(50)));
    blank();
    print(chalk.bold.underline('  RELATED'));
    blank();
    if (explanation.related.documentation) {
      print(`  • Documentation: ${explanation.related.documentation}`);
    }
    if (explanation.related.ruleDefinition) {
      print(`  • Rule definition: ${explanation.related.ruleDefinition}`);
    }
    if (
      explanation.related.similarWarnings !== undefined &&
      explanation.related.similarWarnings > 0
    ) {
      print(`  • Similar warnings in this file: ${explanation.related.similarWarnings}`);
    }
  }

  blank();
}

function indentContent(content: string, indent: string): string {
  return content
    .split('\n')
    .map((line) => indent + line)
    .join('\n');
}

function formatExplanationJson(explanation: WarningExplanation): void {
  data(JSON.stringify(explanation, null, 2));
}

function listExplainableRules(json: boolean): void {
  const rules = getExplainableRules();

  if (json) {
    data(JSON.stringify({ rules }, null, 2));
    return;
  }

  blank();
  print(chalk.bold('Available rules with explanations:'));
  blank();

  print(chalk.yellow.bold('Anti-pattern rules:'));
  const apRules = rules.filter((r) => r.startsWith('AP-'));
  for (const rule of apRules) {
    print(`  ${rule}`);
  }

  blank();
  print(chalk.cyan.bold('Architecture rules:'));
  const archRules = rules.filter((r) => r.startsWith('ARCH-') || r.startsWith('BOUND-'));
  for (const rule of archRules) {
    print(`  ${rule}`);
  }

  blank();
  print(chalk.gray('Usage: anvil explain <rule-id>'));
  print(chalk.gray('       anvil explain AP-003'));
  print(chalk.gray('       anvil explain AP-003-src/file.ts:42'));
}

async function listRecentWarnings(json: boolean): Promise<void> {
  const workspaceRoot = getWorkspaceRoot();
  const warnings = await loadRecentWarnings(workspaceRoot);

  if (json) {
    data(JSON.stringify({ warnings }, null, 2));
    return;
  }

  blank();
  print(chalk.bold('Recent warnings (from last `anvil check` run):'));
  blank();

  if (warnings.length === 0) {
    print(chalk.gray('No recent warnings found. Run `anvil check` first.'));
    blank();
    return;
  }

  for (const warning of warnings) {
    const parsed = parseWarningId(warning.warningId);
    const rule = parsed?.rule ?? warning.warningId;
    print(chalk.yellow(`  ${warning.warningId}`));
    print(chalk.gray(`    ${rule} · ${warning.location.file}:${warning.location.line}`));
    print(`    ${warning.title}`);
    blank();
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

  print(chalk.red(`Unknown warning ID or rule: ${warningId}`));
  blank();
  print(chalk.gray('Use --list to see recent warning IDs from check output'));
  print(chalk.gray('Use --rules to see all explainable rules'));
  print(chalk.gray('Format: RULE-ID or RULE-ID-file.ts:line'));
  print(chalk.gray('Examples:'));
  print(chalk.gray('  anvil explain AP-003'));
  print(chalk.gray('  anvil explain AP-003-src/utils.ts:42'));
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
