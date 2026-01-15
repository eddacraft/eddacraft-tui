/**
 * Category of command rule
 */
export type CommandCategory = 'git' | 'filesystem' | 'shell' | 'custom';

/**
 * Action to take when a rule matches
 */
export type CommandAction = 'block' | 'warn' | 'allow';

/**
 * Severity level for rule violations
 */
export type CommandSeverity = 'error' | 'warning' | 'info';

/**
 * Flag matching configuration for a rule
 */
export interface CommandFlagConfig {
  /** Flags where ANY ONE must be present for the rule to match (OR logic) */
  required?: string[];
  /** Flags where ALL must be present for the rule to match (AND logic) */
  requiredAll?: string[];
  /** Flags that must NOT be present for the rule to match */
  forbidden?: string[];
  /** Flags that make the command dangerous when present */
  dangerous?: string[];
}

/**
 * Argument matching configuration for a rule
 */
export interface CommandArgConfig {
  /** Regex pattern to match against arguments */
  pattern?: RegExp;
  /** Specific argument position to check (0-indexed) */
  position?: number;
}

/**
 * Conditions for when a rule applies
 */
export interface CommandConditions {
  /** Only apply in strict mode */
  strictModeOnly?: boolean;
  /** Working directory restrictions */
  workingDirectory?: 'home' | 'root' | 'any';
}

/**
 * A command safety rule definition.
 *
 * Rules are matched by specificity: command + subcommand + flags + args.
 * More specific rules take precedence over less specific ones.
 */
export interface CommandRule {
  /** Unique identifier for the rule (e.g., 'git-reset-hard') */
  id: string;

  /** Category of the rule */
  category: CommandCategory;

  /** Base command to match (e.g., 'git', 'rm') */
  command: string;

  /** Optional subcommand to match (e.g., 'reset', 'push') */
  subcommand?: string;

  /** Flag matching configuration */
  flags?: CommandFlagConfig;

  /** Argument matching configuration */
  args?: CommandArgConfig;

  /** Action to take when the rule matches */
  action: CommandAction;

  /** Severity level for reporting */
  severity: CommandSeverity;

  /** Human-readable explanation of why this is blocked/warned */
  reason: string;

  /** Suggested safe alternative */
  suggestion?: string;

  /** Reference links for more information */
  references?: string[];

  /** Conditions for when the rule applies */
  conditions?: CommandConditions;
}

/**
 * A collection of command rules
 */
export interface CommandRuleset {
  /** Ruleset version */
  version: string;
  /** Array of rules */
  rules: CommandRule[];
}

/**
 * A parsed shell command with extracted components.
 *
 * This is the output of the CommandParser.
 */
export interface ParsedCommand {
  /** Original raw command string */
  raw: string;

  /** Base command (e.g., 'git', 'rm', 'bash') */
  command: string;

  /** Subcommand if applicable (e.g., 'reset', 'push') */
  subcommand?: string;

  /** Extracted flags (e.g., ['--hard', '-f', '-r']) */
  flags: string[];

  /** Extracted arguments (non-flag tokens) */
  args: string[];

  /** The unwrapped command after stripping wrappers */
  unwrapped: string;

  /** Chain of wrappers that were stripped (e.g., ['sudo', 'bash']) */
  wrapperChain: string[];
}

/**
 * Result of analysing a single command against the ruleset.
 */
export interface CommandAnalysisResult {
  /** Original command string */
  command: string;

  /** Parsed command structure */
  parsedCommand: ParsedCommand;

  /** The rule that matched, if any */
  matchedRule?: CommandRule;

  /** Resolved action to take */
  action: CommandAction;

  /** Resolved severity level */
  severity: CommandSeverity;

  /** Reason for the action (from matched rule) */
  reason?: string;

  /** Suggested alternative (from matched rule) */
  suggestion?: string;

  /** Reference links (from matched rule) */
  references?: string[];
}

/**
 * Summary of command analysis results.
 */
export interface CommandAnalysisSummary {
  /** Total commands analysed */
  total: number;

  /** Number of blocked commands */
  blocked: number;

  /** Number of warned commands */
  warned: number;

  /** Number of allowed commands */
  allowed: number;
}

/**
 * Override configuration for a specific rule.
 */
export interface CommandRuleOverride {
  /** Rule ID to override */
  id: string;

  /** Override action (or 'disable' to completely disable) */
  action?: CommandAction | 'disable';

  /** Override severity */
  severity?: CommandSeverity;
}

/**
 * Rules configuration section.
 */
export interface CommandRulesConfig {
  /** Override existing rules */
  overrides?: CommandRuleOverride[];

  /** Add custom rules */
  custom?: CommandRule[];

  /** Disable specific rules by ID */
  disabled?: string[];
}

/**
 * Working directory configuration.
 */
export interface WorkingDirectoryConfig {
  /** Allow rm -rf in current working directory */
  allowDeleteInCwd?: boolean;

  /** Additional patterns to treat as temp directories */
  tempDirPatterns?: string[];
}

/**
 * Output configuration.
 */
export interface CommandSafetyOutputConfig {
  /** Include full command in error messages */
  verbose?: boolean;

  /** Show safe alternative suggestions */
  showSuggestions?: boolean;

  /** Show reference links */
  showReferences?: boolean;
}

/**
 * Command safety check configuration.
 *
 * This is loaded from `.anvilrc` under the `commandSafety` key.
 */
export interface CommandSafetyConfig {
  /** Enable/disable the check (default: true) */
  enabled?: boolean;

  /** Strict mode - block unparseable commands (default: false) */
  strict?: boolean;

  /** Rule customisation */
  rules?: CommandRulesConfig;

  /** Working directory restrictions */
  workingDirectory?: WorkingDirectoryConfig;

  /** Output customisation */
  output?: CommandSafetyOutputConfig;
}

/**
 * Resolved configuration with all defaults applied.
 */
export interface ResolvedCommandSafetyConfig {
  enabled: boolean;
  strict: boolean;
  rules: CommandRule[];
  workingDirectory: Required<WorkingDirectoryConfig>;
  output: Required<CommandSafetyOutputConfig>;
}

/**
 * Detailed information about a blocked or warned command.
 */
export interface CommandSafetyFinding {
  /** The command that was flagged */
  command: string;

  /** The rule ID that matched */
  ruleId: string;

  /** Category of the rule */
  category: CommandCategory;

  /** Action taken (block or warn) */
  action: Exclude<CommandAction, 'allow'>;

  /** Severity level */
  severity: CommandSeverity;

  /** Human-readable reason */
  reason: string;

  /** Suggested safe alternative */
  suggestion?: string;

  /** Reference links */
  references?: string[];

  /** Source location if available (e.g., plan file path) */
  source?: string;
}

/**
 * Result details from the command safety check.
 */
export interface CommandSafetyDetails {
  /** Commands that were blocked */
  blocked: CommandSafetyFinding[];

  /** Commands that triggered warnings */
  warnings: CommandSafetyFinding[];

  /** Summary statistics */
  summary: CommandAnalysisSummary;

  /** Configuration that was used */
  config?: {
    strict: boolean;
    rulesCount: number;
    customRulesCount: number;
    disabledRulesCount: number;
  };
}
