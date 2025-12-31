import { BaseCheck } from '../check.interface.js';
import type { CheckContext, GateResult } from '../../types/gate.types.js';
import { CommandParser } from '../parsers/command-parser.js';
import { findMatchingRule, DEFAULT_GIT_RULES, DEFAULT_FILESYSTEM_RULES } from '../rules/index.js';
import type {
  CommandRule,
  CommandSafetyConfig,
  CommandSafetyFinding,
  CommandAnalysisSummary,
  CommandSafetyOutputConfig,
  CommandRulesConfig,
  WorkingDirectoryConfig,
} from '../rules/types.js';

interface CommandSource {
  command: string;
  source?: string;
}

interface ResolvedConfig {
  enabled: boolean;
  strict: boolean;
  rules: CommandRulesConfig;
  workingDirectory: Required<WorkingDirectoryConfig>;
  output: Required<CommandSafetyOutputConfig>;
}

function extractCommandsFromPlan(context: CheckContext): CommandSource[] {
  const commands: CommandSource[] = [];

  if (!context.plan) {
    return commands;
  }

  for (const change of context.plan.proposed_changes) {
    if (change.type === 'script_execute' && change.description) {
      const codeBlockMatch = change.description.match(/```(?:bash|sh|shell)?\n([\s\S]*?)```/);
      if (codeBlockMatch) {
        const lines = codeBlockMatch[1]
          .split('\n')
          .filter((line: string) => line.trim() && !line.startsWith('#'));
        for (const line of lines) {
          commands.push({ command: line.trim(), source: change.path || 'script_execute' });
        }
      } else if (!change.description.includes('\n')) {
        commands.push({ command: change.description, source: change.path || 'script_execute' });
      }
    }
  }

  return commands;
}

function loadRules(config: CommandSafetyConfig): CommandRule[] {
  let rules: CommandRule[] = [...DEFAULT_GIT_RULES, ...DEFAULT_FILESYSTEM_RULES];

  if (config.rules?.disabled && config.rules.disabled.length > 0) {
    const disabledSet = new Set(config.rules.disabled);
    rules = rules.filter((r) => !disabledSet.has(r.id));
  }

  if (config.rules?.overrides && config.rules.overrides.length > 0) {
    for (const override of config.rules.overrides) {
      const ruleIndex = rules.findIndex((r) => r.id === override.id);
      if (ruleIndex !== -1) {
        if (override.action === 'disable') {
          rules.splice(ruleIndex, 1);
        } else {
          rules[ruleIndex] = {
            ...rules[ruleIndex],
            ...(override.action && { action: override.action }),
            ...(override.severity && { severity: override.severity }),
          };
        }
      }
    }
  }

  if (config.rules?.custom && config.rules.custom.length > 0) {
    rules.push(...config.rules.custom);
  }

  return rules;
}

function formatBlockedMessage(
  blocked: CommandSafetyFinding[],
  outputConfig: Required<CommandSafetyOutputConfig>
): string {
  if (blocked.length === 0) {
    return '';
  }

  const lines = [`Blocked ${blocked.length} dangerous command(s):`, ''];

  for (let i = 0; i < blocked.length; i++) {
    const finding = blocked[i];
    lines.push(`${i + 1}. ${finding.command}`);
    if (outputConfig.verbose) {
      lines.push(`   Reason: ${finding.reason}`);
    }
    if (outputConfig.showSuggestions && finding.suggestion) {
      lines.push(`   Suggestion: ${finding.suggestion}`);
    }
    if (outputConfig.showReferences && finding.references && finding.references.length > 0) {
      lines.push(`   Reference: ${finding.references[0]}`);
    }
    lines.push('');
  }

  return lines.join('\n');
}

function formatWarningMessage(
  warnings: CommandSafetyFinding[],
  outputConfig: Required<CommandSafetyOutputConfig>
): string {
  if (warnings.length === 0) {
    return '';
  }

  const lines = [`Found ${warnings.length} potentially dangerous command(s):`, ''];

  for (let i = 0; i < warnings.length; i++) {
    const finding = warnings[i];
    lines.push(`${i + 1}. ${finding.command}`);
    if (outputConfig.verbose) {
      lines.push(`   Reason: ${finding.reason}`);
    }
    if (outputConfig.showSuggestions && finding.suggestion) {
      lines.push(`   Suggestion: ${finding.suggestion}`);
    }
    lines.push('');
  }

  return lines.join('\n');
}

export class CommandSafetyCheck extends BaseCheck {
  name = 'command-safety';
  description = 'Validates shell commands for destructive operations';

  private parser: CommandParser;

  constructor() {
    super();
    this.parser = new CommandParser();
  }

  async run(context: CheckContext): Promise<GateResult> {
    const config = this.getConfig(context);

    if (!config.enabled) {
      return this.createResult(true, 'Command safety check disabled', 100, { skipped: true });
    }

    const commandSources = extractCommandsFromPlan(context);

    if (commandSources.length === 0) {
      return this.createSuccess('No commands to analyse', 100, {
        summary: { total: 0, blocked: 0, warned: 0, allowed: 0 } as CommandAnalysisSummary,
      });
    }

    const rules = loadRules(config);
    const blocked: CommandSafetyFinding[] = [];
    const warnings: CommandSafetyFinding[] = [];
    let allowed = 0;
    let totalAnalysed = 0;

    for (const { command: rawCommand, source } of commandSources) {
      const compoundResult = this.parser.parseCompound(rawCommand);

      for (const parsed of compoundResult.commands) {
        if (!parsed.command) {
          continue;
        }

        totalAnalysed++;

        const matchedRule = findMatchingRule(parsed, rules, {
          strict: config.strict,
          workingDirectory: config.workingDirectory,
        });

        if (!matchedRule) {
          allowed++;
          continue;
        }

        if (matchedRule.action === 'allow') {
          allowed++;
          continue;
        }

        const finding: CommandSafetyFinding = {
          command: compoundResult.isCompound ? `${parsed.raw} (from: ${rawCommand})` : parsed.raw,
          ruleId: matchedRule.id,
          category: matchedRule.category,
          action: matchedRule.action,
          severity: matchedRule.severity,
          reason: matchedRule.reason,
          suggestion: matchedRule.suggestion,
          references: matchedRule.references,
          source,
        };

        if (matchedRule.action === 'block') {
          blocked.push(finding);
        } else if (matchedRule.action === 'warn') {
          warnings.push(finding);
        }
      }
    }

    const summary: CommandAnalysisSummary = {
      total: totalAnalysed,
      blocked: blocked.length,
      warned: warnings.length,
      allowed,
    };

    const passed = blocked.length === 0;
    const score = this.calculateScore(summary);

    let message: string;
    if (passed && warnings.length === 0) {
      message = `All ${summary.total} command(s) passed safety check`;
    } else if (passed) {
      message = `${summary.total} command(s) analysed: ${warnings.length} warning(s)`;
    } else {
      message = `Command safety check failed: ${blocked.length} blocked, ${warnings.length} warning(s)`;
    }

    return this.createResult(passed, message, score, {
      blocked,
      warnings,
      summary,
      config: {
        strict: config.strict ?? false,
        rulesCount: rules.length,
        customRulesCount: config.rules?.custom?.length ?? 0,
        disabledRulesCount: config.rules?.disabled?.length ?? 0,
      },
      formattedBlockedMessage: formatBlockedMessage(blocked, config.output),
      formattedWarningMessage: formatWarningMessage(warnings, config.output),
    });
  }

  private getConfig(context: CheckContext): ResolvedConfig {
    const checkConfig = (context.check_config ?? {}) as CommandSafetyConfig;
    return {
      enabled: checkConfig.enabled ?? true,
      strict: checkConfig.strict ?? false,
      rules: checkConfig.rules ?? {},
      workingDirectory: {
        allowDeleteInCwd: checkConfig.workingDirectory?.allowDeleteInCwd ?? false,
        tempDirPatterns: checkConfig.workingDirectory?.tempDirPatterns ?? ['/tmp', '/var/tmp'],
      },
      output: {
        verbose: checkConfig.output?.verbose ?? true,
        showSuggestions: checkConfig.output?.showSuggestions ?? true,
        showReferences: checkConfig.output?.showReferences ?? true,
      },
    };
  }

  private calculateScore(summary: CommandAnalysisSummary): number {
    if (summary.total === 0) {
      return 100;
    }

    const blockedPenalty = summary.blocked * 25;
    const warnedPenalty = summary.warned * 5;
    const totalPenalty = blockedPenalty + warnedPenalty;

    return Math.max(0, 100 - totalPenalty);
  }
}
