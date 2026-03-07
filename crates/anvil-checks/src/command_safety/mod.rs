pub mod check;
pub mod matcher;
pub mod parser;
pub mod rules;
pub mod types;

pub use check::{run_command_safety_check, CommandSafetyCheckContext};
pub use matcher::{
    analyse_command, calculate_specificity, find_matching_rule, MatcherContext, RuleMatcher,
};
pub use parser::{parse_command, parse_compound_command, CommandParser, CompoundCommandResult};
pub use rules::{default_filesystem_rules, default_git_rules};
pub use types::{
    CommandAction, CommandAnalysisResult, CommandAnalysisSummary, CommandArgConfig,
    CommandCategory, CommandConditions, CommandFlagConfig, CommandRule, CommandRuleOverride,
    CommandRuleOverrideAction, CommandRulesConfig, CommandRuleset, CommandSafetyCheckResult,
    CommandSafetyConfig, CommandSafetyDetails, CommandSafetyFinding, CommandSafetyOutputConfig,
    CommandSafetyResolvedConfigInfo, CommandSeverity, ParsedCommand, ResolvedCommandSafetyConfig,
    ResolvedCommandSafetyOutputConfig, ResolvedWorkingDirectoryConfig, ScriptChange,
    ScriptChangeType, ScriptPlan, WorkingDirectoryCondition, WorkingDirectoryConfig,
};
