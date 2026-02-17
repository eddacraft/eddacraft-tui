import type {
  CommandRule,
  ParsedCommand,
  CommandAnalysisResult,
  WorkingDirectoryConfig,
} from './types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('gate');

export interface MatcherContext {
  strict?: boolean;
  workingDirectory?: WorkingDirectoryConfig;
  cwd?: string;
}

const SPECIFICITY_COMMAND = 1;
const SPECIFICITY_SUBCOMMAND = 2;
const SPECIFICITY_FLAGS = 4;
const SPECIFICITY_ARGS = 8;

function calculateSpecificity(rule: CommandRule): number {
  let score = SPECIFICITY_COMMAND;

  if (rule.subcommand) {
    score += SPECIFICITY_SUBCOMMAND;
  }

  if (rule.flags) {
    const hasFlags =
      (rule.flags.required && rule.flags.required.length > 0) ||
      (rule.flags.requiredAll && rule.flags.requiredAll.length > 0) ||
      (rule.flags.forbidden && rule.flags.forbidden.length > 0) ||
      (rule.flags.dangerous && rule.flags.dangerous.length > 0);
    if (hasFlags) {
      score += SPECIFICITY_FLAGS;
    }
  }

  if (rule.args?.pattern) {
    score += SPECIFICITY_ARGS;
  }

  return score;
}

function normaliseFlag(flag: string): string {
  if (flag.startsWith('--')) {
    return flag;
  }
  return flag.toLowerCase();
}

function hasFlag(parsedFlags: string[], ruleFlag: string): boolean {
  const normalised = normaliseFlag(ruleFlag);
  return parsedFlags.some((f) => normaliseFlag(f) === normalised);
}

function hasAnyFlag(parsedFlags: string[], ruleFlags: string[]): boolean {
  return ruleFlags.some((f) => hasFlag(parsedFlags, f));
}

function hasAllFlags(parsedFlags: string[], ruleFlags: string[]): boolean {
  return ruleFlags.every((f) => hasFlag(parsedFlags, f));
}

function matchFlags(parsed: ParsedCommand, rule: CommandRule): boolean {
  if (!rule.flags) {
    return true;
  }

  const { required, requiredAll, forbidden, dangerous } = rule.flags;

  if (required && required.length > 0) {
    if (!hasAnyFlag(parsed.flags, required)) {
      return false;
    }
  }

  if (requiredAll && requiredAll.length > 0) {
    if (!hasAllFlags(parsed.flags, requiredAll)) {
      return false;
    }
  }

  if (forbidden && forbidden.length > 0) {
    if (hasAnyFlag(parsed.flags, forbidden)) {
      return false;
    }
  }

  if (dangerous && dangerous.length > 0) {
    if (!hasAnyFlag(parsed.flags, dangerous)) {
      return false;
    }
  }

  return true;
}

function isPathInTempDir(path: string, context?: MatcherContext): boolean {
  const tempPatterns = context?.workingDirectory?.tempDirPatterns ?? ['/tmp', '/var/tmp'];
  return isTempPath(path, tempPatterns);
}

function matchArgs(parsed: ParsedCommand, rule: CommandRule, context?: MatcherContext): boolean {
  if (!rule.args?.pattern) {
    return true;
  }

  const allArgs = [...parsed.args];
  if (parsed.subcommand) {
    allArgs.unshift(parsed.subcommand);
  }

  if (rule.args.position !== undefined) {
    const arg = allArgs[rule.args.position];
    if (arg === undefined) {
      return false;
    }
    if (context?.workingDirectory?.allowDeleteInCwd && isPathInTempDir(arg, context)) {
      return false;
    }
    return rule.args.pattern.test(arg);
  }

  return allArgs.some((arg) => {
    if (context?.workingDirectory?.allowDeleteInCwd && isPathInTempDir(arg, context)) {
      return false;
    }
    return rule.args!.pattern!.test(arg);
  });
}

function isHomePath(path: string): boolean {
  return (
    path.startsWith('/home/') || path.startsWith('/Users/') || path === '~' || path.startsWith('~/')
  );
}

function isRootPath(path: string): boolean {
  return path === '/' || path === '/root';
}

function isTempPath(path: string, tempPatterns: string[]): boolean {
  return tempPatterns.some((pattern) => path.startsWith(pattern));
}

function matchWorkingDirectory(
  ruleCondition: 'home' | 'root' | 'any',
  context?: MatcherContext
): boolean {
  if (ruleCondition === 'any') {
    return true;
  }

  const cwd = context?.cwd;
  if (!cwd) {
    return true;
  }

  if (ruleCondition === 'home') {
    return isHomePath(cwd);
  }

  if (ruleCondition === 'root') {
    return isRootPath(cwd);
  }

  return true;
}

function matchConditions(rule: CommandRule, context?: MatcherContext): boolean {
  if (!rule.conditions) {
    return true;
  }

  if (rule.conditions.strictModeOnly && !context?.strict) {
    return false;
  }

  if (rule.conditions.workingDirectory) {
    if (!matchWorkingDirectory(rule.conditions.workingDirectory, context)) {
      return false;
    }
  }

  return true;
}

function matchRule(parsed: ParsedCommand, rule: CommandRule, context?: MatcherContext): boolean {
  if (!matchConditions(rule, context)) {
    return false;
  }

  if (parsed.command !== rule.command) {
    return false;
  }

  if (rule.subcommand && parsed.subcommand !== rule.subcommand) {
    return false;
  }

  if (!matchFlags(parsed, rule)) {
    return false;
  }

  if (!matchArgs(parsed, rule, context)) {
    return false;
  }

  return true;
}

export function findMatchingRule(
  parsed: ParsedCommand,
  rules: CommandRule[],
  context?: MatcherContext
): CommandRule | undefined {
  debug(
    'findMatchingRule: command=%s subcommand=%s rules=%d',
    parsed.command,
    parsed.subcommand,
    rules.length
  );
  const sorted = [...rules].sort((a, b) => {
    const scoreA = calculateSpecificity(a);
    const scoreB = calculateSpecificity(b);
    return scoreB - scoreA;
  });

  for (const rule of sorted) {
    if (matchRule(parsed, rule, context)) {
      debug('findMatchingRule: matched rule=%s action=%s', rule.id, rule.action);
      return rule;
    }
  }

  debug('findMatchingRule: no rule matched');
  return undefined;
}

export function analyseCommand(
  command: string,
  parsed: ParsedCommand,
  rules: CommandRule[],
  context?: MatcherContext
): CommandAnalysisResult {
  debug('analyseCommand: command=%s', command);
  const matchedRule = findMatchingRule(parsed, rules, context);

  if (!matchedRule) {
    debug('analyseCommand: allowed (no matching rule)');
    return {
      command,
      parsedCommand: parsed,
      action: 'allow',
      severity: 'info',
    };
  }

  debug(
    'analyseCommand: result action=%s severity=%s rule=%s',
    matchedRule.action,
    matchedRule.severity,
    matchedRule.id
  );
  return {
    command,
    parsedCommand: parsed,
    matchedRule,
    action: matchedRule.action,
    severity: matchedRule.severity,
    reason: matchedRule.reason,
    suggestion: matchedRule.suggestion,
    references: matchedRule.references,
  };
}

export class RuleMatcher {
  private rules: CommandRule[];
  private context?: MatcherContext;

  constructor(rules: CommandRule[] = [], context?: MatcherContext) {
    this.rules = rules;
    this.context = context;
  }

  setContext(context: MatcherContext): void {
    this.context = context;
  }

  addRule(rule: CommandRule): void {
    this.rules.push(rule);
  }

  addRules(rules: CommandRule[]): void {
    this.rules.push(...rules);
  }

  setRules(rules: CommandRule[]): void {
    this.rules = [...rules];
  }

  getRules(): CommandRule[] {
    return [...this.rules];
  }

  findMatchingRule(parsed: ParsedCommand, context?: MatcherContext): CommandRule | undefined {
    return findMatchingRule(parsed, this.rules, context ?? this.context);
  }

  analyse(command: string, parsed: ParsedCommand, context?: MatcherContext): CommandAnalysisResult {
    return analyseCommand(command, parsed, this.rules, context ?? this.context);
  }

  analyseMultiple(
    commands: Array<{ command: string; parsed: ParsedCommand }>,
    context?: MatcherContext
  ): CommandAnalysisResult[] {
    const ctx = context ?? this.context;
    return commands.map(({ command, parsed }) => analyseCommand(command, parsed, this.rules, ctx));
  }
}

export { calculateSpecificity };
