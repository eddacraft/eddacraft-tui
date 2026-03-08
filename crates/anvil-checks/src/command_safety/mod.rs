pub mod check;
pub mod matcher;
pub mod parser;
pub mod rules;
pub mod types;

pub use check::{CommandSafetyCheckContext, run_command_safety_check};
pub use matcher::{
    MatcherContext, RuleMatcher, analyse_command, calculate_specificity, find_matching_rule,
};
pub use parser::{CommandParser, CompoundCommandResult, parse_command, parse_compound_command};
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
