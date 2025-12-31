export type {
  CommandCategory,
  CommandAction,
  CommandSeverity,
  CommandFlagConfig,
  CommandArgConfig,
  CommandConditions,
  CommandRule,
  CommandRuleset,
  ParsedCommand,
  CommandAnalysisResult,
  CommandAnalysisSummary,
  CommandRuleOverride,
  CommandRulesConfig,
  WorkingDirectoryConfig,
  CommandSafetyOutputConfig,
  CommandSafetyConfig,
  ResolvedCommandSafetyConfig,
  CommandSafetyFinding,
  CommandSafetyDetails,
} from './types.js';

export {
  findMatchingRule,
  analyseCommand,
  calculateSpecificity,
  RuleMatcher,
} from './rule-matcher.js';

export { DEFAULT_GIT_RULES } from './default-git-rules.js';
export { DEFAULT_FILESYSTEM_RULES } from './default-filesystem-rules.js';
