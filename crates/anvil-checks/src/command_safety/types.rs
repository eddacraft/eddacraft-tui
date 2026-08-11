use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandCategory {
    Git,
    Filesystem,
    Shell,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandAction {
    Block,
    Warn,
    Allow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkingDirectoryCondition {
    Home,
    Root,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandFlagConfig {
    pub required: Option<Vec<String>>,
    #[serde(rename = "requiredAll")]
    pub required_all: Option<Vec<String>>,
    pub forbidden: Option<Vec<String>>,
    pub dangerous: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandArgConfig {
    pub pattern: Option<String>,
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandConditions {
    #[serde(rename = "strictModeOnly")]
    pub strict_mode_only: Option<bool>,
    #[serde(rename = "workingDirectory")]
    pub working_directory: Option<WorkingDirectoryCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandRule {
    pub id: String,
    pub category: CommandCategory,
    pub command: String,
    pub subcommand: Option<String>,
    pub flags: Option<CommandFlagConfig>,
    pub args: Option<CommandArgConfig>,
    pub action: CommandAction,
    pub severity: CommandSeverity,
    pub reason: String,
    pub suggestion: Option<String>,
    pub references: Option<Vec<String>>,
    pub conditions: Option<CommandConditions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandRuleset {
    pub version: String,
    pub rules: Vec<CommandRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedCommand {
    pub raw: String,
    pub command: String,
    pub subcommand: Option<String>,
    pub flags: Vec<String>,
    pub args: Vec<String>,
    pub unwrapped: String,
    #[serde(rename = "wrapperChain")]
    pub wrapper_chain: Vec<String>,
    /// True when wrapper unwrapping stopped early (depth limit) while the
    /// remaining command still looked like a recognised wrapper. Consumers
    /// must treat this as incomplete analysis and fail closed.
    #[serde(default, rename = "unwrapIncomplete")]
    pub unwrap_incomplete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandAnalysisResult {
    pub command: String,
    #[serde(rename = "parsedCommand")]
    pub parsed_command: ParsedCommand,
    #[serde(rename = "matchedRule")]
    pub matched_rule: Option<CommandRule>,
    pub action: CommandAction,
    pub severity: CommandSeverity,
    pub reason: Option<String>,
    pub suggestion: Option<String>,
    pub references: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandAnalysisSummary {
    pub total: usize,
    pub blocked: usize,
    pub warned: usize,
    pub allowed: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandRuleOverrideAction {
    Block,
    Warn,
    Allow,
    Disable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandRuleOverride {
    pub id: String,
    pub action: Option<CommandRuleOverrideAction>,
    pub severity: Option<CommandSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandRulesConfig {
    pub overrides: Option<Vec<CommandRuleOverride>>,
    pub custom: Option<Vec<CommandRule>>,
    pub disabled: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkingDirectoryConfig {
    #[serde(rename = "allowDeleteInCwd")]
    pub allow_delete_in_cwd: Option<bool>,
    #[serde(rename = "tempDirPatterns")]
    pub temp_dir_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedWorkingDirectoryConfig {
    #[serde(rename = "allowDeleteInCwd")]
    pub allow_delete_in_cwd: bool,
    #[serde(rename = "tempDirPatterns")]
    pub temp_dir_patterns: Vec<String>,
}

impl Default for ResolvedWorkingDirectoryConfig {
    fn default() -> Self {
        Self {
            allow_delete_in_cwd: false,
            temp_dir_patterns: vec!["/tmp".to_string(), "/var/tmp".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandSafetyOutputConfig {
    pub verbose: Option<bool>,
    #[serde(rename = "showSuggestions")]
    pub show_suggestions: Option<bool>,
    #[serde(rename = "showReferences")]
    pub show_references: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedCommandSafetyOutputConfig {
    pub verbose: bool,
    #[serde(rename = "showSuggestions")]
    pub show_suggestions: bool,
    #[serde(rename = "showReferences")]
    pub show_references: bool,
}

impl Default for ResolvedCommandSafetyOutputConfig {
    fn default() -> Self {
        Self {
            verbose: true,
            show_suggestions: true,
            show_references: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandSafetyConfig {
    pub enabled: Option<bool>,
    pub strict: Option<bool>,
    pub rules: Option<CommandRulesConfig>,
    #[serde(rename = "workingDirectory")]
    pub working_directory: Option<WorkingDirectoryConfig>,
    pub output: Option<CommandSafetyOutputConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedCommandSafetyConfig {
    pub enabled: bool,
    pub strict: bool,
    pub rules: Vec<CommandRule>,
    #[serde(rename = "workingDirectory")]
    pub working_directory: ResolvedWorkingDirectoryConfig,
    pub output: ResolvedCommandSafetyOutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSafetyFinding {
    pub command: String,
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub category: CommandCategory,
    pub action: CommandAction,
    pub severity: CommandSeverity,
    pub reason: String,
    pub suggestion: Option<String>,
    pub references: Option<Vec<String>>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSafetyResolvedConfigInfo {
    pub strict: bool,
    #[serde(rename = "rulesCount")]
    pub rules_count: usize,
    #[serde(rename = "customRulesCount")]
    pub custom_rules_count: usize,
    #[serde(rename = "disabledRulesCount")]
    pub disabled_rules_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSafetyDetails {
    pub blocked: Vec<CommandSafetyFinding>,
    pub warnings: Vec<CommandSafetyFinding>,
    pub summary: CommandAnalysisSummary,
    pub config: Option<CommandSafetyResolvedConfigInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSafetyCheckResult {
    pub passed: bool,
    pub score: u8,
    pub message: String,
    pub blocked: Vec<CommandSafetyFinding>,
    pub warnings: Vec<CommandSafetyFinding>,
    pub summary: CommandAnalysisSummary,
    pub details: CommandSafetyDetails,
    #[serde(rename = "formattedBlockedMessage")]
    pub formatted_blocked_message: String,
    #[serde(rename = "formattedWarningMessage")]
    pub formatted_warning_message: String,
    pub skipped: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptChangeType {
    ScriptExecute,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptChange {
    #[serde(rename = "type")]
    pub change_type: ScriptChangeType,
    pub description: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScriptPlan {
    #[serde(rename = "proposed_changes")]
    pub proposed_changes: Vec<ScriptChange>,
}
