use regex::Regex;

use crate::command_safety::types::{
    CommandAction, CommandAnalysisResult, CommandCategory, CommandRule, CommandSafetyConfig,
    CommandSeverity, ParsedCommand, WorkingDirectoryCondition, WorkingDirectoryConfig,
};

#[derive(Debug, Clone, Default)]
pub struct MatcherContext {
    pub strict: Option<bool>,
    pub working_directory: Option<WorkingDirectoryConfig>,
    pub cwd: Option<String>,
}

const SPECIFICITY_COMMAND: u8 = 1;
const SPECIFICITY_SUBCOMMAND: u8 = 2;
const SPECIFICITY_FLAGS: u8 = 4;
const SPECIFICITY_ARGS: u8 = 8;

#[must_use]
pub fn calculate_specificity(rule: &CommandRule) -> u8 {
    let mut score = SPECIFICITY_COMMAND;

    if rule.subcommand.is_some() {
        score += SPECIFICITY_SUBCOMMAND;
    }

    if let Some(flags) = &rule.flags {
        let has_flags = flags.required.as_ref().is_some_and(|f| !f.is_empty())
            || flags.required_all.as_ref().is_some_and(|f| !f.is_empty())
            || flags.forbidden.as_ref().is_some_and(|f| !f.is_empty())
            || flags.dangerous.as_ref().is_some_and(|f| !f.is_empty());
        if has_flags {
            score += SPECIFICITY_FLAGS;
        }
    }

    if rule
        .args
        .as_ref()
        .and_then(|arg| arg.pattern.as_ref())
        .is_some()
    {
        score += SPECIFICITY_ARGS;
    }

    score
}

#[must_use]
fn normalise_flag(flag: &str) -> String {
    if flag.starts_with("--") {
        return flag.to_lowercase();
    }
    flag.to_string()
}

#[must_use]
fn has_flag(parsed_flags: &[String], rule_flag: &str) -> bool {
    let normalised = normalise_flag(rule_flag);
    parsed_flags
        .iter()
        .any(|flag| normalise_flag(flag) == normalised)
}

#[must_use]
fn has_any_flag(parsed_flags: &[String], rule_flags: &[String]) -> bool {
    rule_flags.iter().any(|flag| has_flag(parsed_flags, flag))
}

#[must_use]
fn has_all_flags(parsed_flags: &[String], rule_flags: &[String]) -> bool {
    rule_flags.iter().all(|flag| has_flag(parsed_flags, flag))
}

#[must_use]
fn match_flags(parsed: &ParsedCommand, rule: &CommandRule) -> bool {
    let Some(flags) = &rule.flags else {
        return true;
    };

    if let Some(required) = &flags.required
        && !required.is_empty()
        && !has_any_flag(&parsed.flags, required)
    {
        return false;
    }

    if let Some(required_all) = &flags.required_all
        && !required_all.is_empty()
        && !has_all_flags(&parsed.flags, required_all)
    {
        return false;
    }

    if let Some(forbidden) = &flags.forbidden
        && !forbidden.is_empty()
        && has_any_flag(&parsed.flags, forbidden)
    {
        return false;
    }

    if let Some(dangerous) = &flags.dangerous
        && !dangerous.is_empty()
        && !has_any_flag(&parsed.flags, dangerous)
    {
        return false;
    }

    true
}

#[must_use]
pub fn is_home_path(path: &str) -> bool {
    path.starts_with("/home/")
        || path.starts_with("/Users/")
        || path == "~"
        || path.starts_with("~/")
}

#[must_use]
pub fn is_root_path(path: &str) -> bool {
    path == "/" || path == "/root"
}

#[must_use]
pub fn is_temp_path(path: &str, temp_patterns: &[String]) -> bool {
    temp_patterns
        .iter()
        .any(|pattern| path.starts_with(pattern))
}

#[must_use]
fn is_path_in_temp_dir(path: &str, context: Option<&MatcherContext>) -> bool {
    let temp_patterns = context
        .and_then(|ctx| ctx.working_directory.as_ref())
        .and_then(|config| config.temp_dir_patterns.as_ref())
        .cloned()
        .unwrap_or_else(|| vec!["/tmp".to_string(), "/var/tmp".to_string()]);
    is_temp_path(path, &temp_patterns)
}

/// True when an argument still contains shell expansion syntax that we refuse
/// to resolve. Command substitutions, parameter expansions, and special/positional
/// parameters can evaluate to protected targets (e.g. `$(printf /)` → `/`), so
/// matching treats them as potential hits against argument patterns rather than
/// allowing them.
#[must_use]
pub(crate) fn contains_unresolved_shell_expansion(argument: &str) -> bool {
    if argument.contains("$(") || argument.contains('`') || argument.contains("${") {
        return true;
    }
    let bytes = argument.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'$' && index + 1 < bytes.len() {
            let next = bytes[index + 1];
            // Named parameters: $HOME, $_foo
            if next.is_ascii_alphabetic() || next == b'_' {
                return true;
            }
            // Positional parameters: $0-$9 and multi-digit $12
            if next.is_ascii_digit() {
                return true;
            }
            // Special parameters: $@ $* $# $? $- $$ $! $0 already covered by digit
            if matches!(next, b'@' | b'*' | b'#' | b'?' | b'-' | b'$' | b'!') {
                return true;
            }
        }
        index += 1;
    }
    false
}

#[must_use]
fn argument_matches_pattern(
    argument: &str,
    pattern: &Regex,
    context: Option<&MatcherContext>,
) -> bool {
    if context
        .and_then(|ctx| ctx.working_directory.as_ref())
        .and_then(|config| config.allow_delete_in_cwd)
        .is_some_and(|allow| allow)
        && is_path_in_temp_dir(argument, context)
    {
        return false;
    }
    if pattern.is_match(argument) {
        return true;
    }
    // Conservative fail-closed: unresolved expansions can resolve to any
    // protected path the pattern would match if expanded.
    contains_unresolved_shell_expansion(argument)
}

#[must_use]
fn match_args(
    parsed: &ParsedCommand,
    rule: &CommandRule,
    context: Option<&MatcherContext>,
) -> bool {
    let Some(args_config) = &rule.args else {
        return true;
    };

    let Some(pattern_text) = &args_config.pattern else {
        return true;
    };

    let Ok(pattern) = Regex::new(pattern_text) else {
        return false;
    };

    let mut all_args = parsed.args.clone();
    if rule.subcommand.is_none()
        && let Some(subcommand) = &parsed.subcommand
    {
        all_args.insert(0, subcommand.clone());
    }

    if let Some(position) = args_config.position {
        let Some(argument) = all_args.get(position) else {
            return false;
        };
        return argument_matches_pattern(argument, &pattern, context);
    }

    all_args
        .into_iter()
        .any(|argument| argument_matches_pattern(&argument, &pattern, context))
}

#[must_use]
fn match_working_directory(
    rule_condition: WorkingDirectoryCondition,
    context: Option<&MatcherContext>,
) -> bool {
    if matches!(rule_condition, WorkingDirectoryCondition::Any) {
        return true;
    }

    let Some(cwd) = context.and_then(|ctx| ctx.cwd.as_ref()) else {
        return true;
    };

    match rule_condition {
        WorkingDirectoryCondition::Any => true,
        WorkingDirectoryCondition::Home => is_home_path(cwd),
        WorkingDirectoryCondition::Root => is_root_path(cwd),
    }
}

#[must_use]
fn match_conditions(rule: &CommandRule, context: Option<&MatcherContext>) -> bool {
    let Some(conditions) = &rule.conditions else {
        return true;
    };

    if conditions.strict_mode_only.is_some_and(|enabled| enabled)
        && !context
            .and_then(|ctx| ctx.strict)
            .is_some_and(|strict| strict)
    {
        return false;
    }

    if let Some(condition) = conditions.working_directory
        && !match_working_directory(condition, context)
    {
        return false;
    }

    true
}

#[must_use]
fn match_rule(
    parsed: &ParsedCommand,
    rule: &CommandRule,
    context: Option<&MatcherContext>,
) -> bool {
    if !match_conditions(rule, context) {
        return false;
    }
    if parsed.command != rule.command {
        return false;
    }
    if let Some(subcommand) = &rule.subcommand
        && parsed.subcommand.as_deref() != Some(subcommand.as_str())
    {
        return false;
    }
    if !match_flags(parsed, rule) {
        return false;
    }
    if !match_args(parsed, rule, context) {
        return false;
    }
    true
}

#[must_use]
pub fn find_matching_rule(
    parsed: &ParsedCommand,
    rules: &[CommandRule],
    context: Option<&MatcherContext>,
) -> Option<CommandRule> {
    let mut sorted = rules.to_vec();
    sorted.sort_by(|left, right| {
        let left_score = calculate_specificity(left);
        let right_score = calculate_specificity(right);
        right_score.cmp(&left_score)
    });

    sorted
        .into_iter()
        .find(|rule| match_rule(parsed, rule, context))
}

#[must_use]
pub fn analyse_command(
    command: &str,
    parsed: &ParsedCommand,
    rules: &[CommandRule],
    context: Option<&MatcherContext>,
) -> CommandAnalysisResult {
    if parsed.unwrap_incomplete {
        let incomplete_rule = CommandRule {
            id: "cmd-unwrap-incomplete".to_string(),
            category: CommandCategory::Shell,
            command: parsed.command.clone(),
            subcommand: None,
            flags: None,
            args: None,
            action: CommandAction::Block,
            severity: CommandSeverity::Error,
            reason: "Command wrapper nesting exceeded analysis depth; refusing to treat as safe"
                .to_string(),
            suggestion: Some(
                "Reduce nested wrappers (env/sudo/bash/...) or rewrite the command so it can be analysed fully."
                    .to_string(),
            ),
            references: None,
            conditions: None,
        };
        return CommandAnalysisResult {
            command: command.to_string(),
            parsed_command: parsed.clone(),
            action: incomplete_rule.action,
            severity: incomplete_rule.severity,
            reason: Some(incomplete_rule.reason.clone()),
            suggestion: incomplete_rule.suggestion.clone(),
            references: None,
            matched_rule: Some(incomplete_rule),
        };
    }
    if let Some(matched_rule) = find_matching_rule(parsed, rules, context) {
        return CommandAnalysisResult {
            command: command.to_string(),
            parsed_command: parsed.clone(),
            action: matched_rule.action,
            severity: matched_rule.severity,
            reason: Some(matched_rule.reason.clone()),
            suggestion: matched_rule.suggestion.clone(),
            references: matched_rule.references.clone(),
            matched_rule: Some(matched_rule),
        };
    }

    CommandAnalysisResult {
        command: command.to_string(),
        parsed_command: parsed.clone(),
        action: CommandAction::Allow,
        severity: CommandSeverity::Info,
        reason: None,
        suggestion: None,
        references: None,
        matched_rule: None,
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuleMatcher {
    rules: Vec<CommandRule>,
    context: Option<MatcherContext>,
}

impl RuleMatcher {
    #[must_use]
    pub fn new(rules: Vec<CommandRule>, context: Option<MatcherContext>) -> Self {
        Self { rules, context }
    }

    pub fn set_context(&mut self, context: MatcherContext) {
        self.context = Some(context);
    }

    pub fn add_rule(&mut self, rule: CommandRule) {
        self.rules.push(rule);
    }

    pub fn add_rules(&mut self, rules: Vec<CommandRule>) {
        self.rules.extend(rules);
    }

    pub fn set_rules(&mut self, rules: Vec<CommandRule>) {
        self.rules = rules;
    }

    #[must_use]
    pub fn get_rules(&self) -> Vec<CommandRule> {
        self.rules.clone()
    }

    #[must_use]
    pub fn find_matching_rule(
        &self,
        parsed: &ParsedCommand,
        context: Option<&MatcherContext>,
    ) -> Option<CommandRule> {
        find_matching_rule(parsed, &self.rules, context.or(self.context.as_ref()))
    }

    #[must_use]
    pub fn analyse(
        &self,
        command: &str,
        parsed: &ParsedCommand,
        context: Option<&MatcherContext>,
    ) -> CommandAnalysisResult {
        analyse_command(
            command,
            parsed,
            &self.rules,
            context.or(self.context.as_ref()),
        )
    }

    #[must_use]
    pub fn analyse_multiple(
        &self,
        commands: &[(String, ParsedCommand)],
        context: Option<&MatcherContext>,
    ) -> Vec<CommandAnalysisResult> {
        commands
            .iter()
            .map(|(command, parsed)| self.analyse(command, parsed, context))
            .collect()
    }
}

impl From<&CommandSafetyConfig> for MatcherContext {
    fn from(config: &CommandSafetyConfig) -> Self {
        Self {
            strict: config.strict,
            working_directory: config.working_directory.clone(),
            cwd: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::command_safety::matcher::{
        MatcherContext, analyse_command, calculate_specificity,
        contains_unresolved_shell_expansion, find_matching_rule, is_home_path, is_root_path,
        is_temp_path,
    };
    use crate::command_safety::parser::parse_command;
    use crate::command_safety::types::{
        CommandAction, CommandArgConfig, CommandCategory, CommandConditions, CommandFlagConfig,
        CommandRule, CommandSeverity, WorkingDirectoryCondition, WorkingDirectoryConfig,
    };

    fn make_rule(id: &str) -> CommandRule {
        CommandRule {
            id: id.to_string(),
            category: CommandCategory::Shell,
            command: "git".to_string(),
            subcommand: None,
            flags: None,
            args: None,
            action: CommandAction::Warn,
            severity: CommandSeverity::Warning,
            reason: "reason".to_string(),
            suggestion: None,
            references: None,
            conditions: None,
        }
    }

    #[test]
    fn specificity_prefers_more_precise_rule() {
        let mut generic = make_rule("generic");
        generic.subcommand = Some("push".to_string());

        let mut specific = generic.clone();
        specific.id = "specific".to_string();
        specific.flags = Some(CommandFlagConfig {
            dangerous: Some(vec!["--force".to_string()]),
            ..CommandFlagConfig::default()
        });

        assert!(calculate_specificity(&specific) > calculate_specificity(&generic));

        let parsed = parse_command("git push --force");
        let matched = find_matching_rule(&parsed, &[generic, specific], None);
        assert_eq!(
            matched.as_ref().map(|rule| rule.id.as_str()),
            Some("specific")
        );
    }

    #[test]
    fn matches_required_and_required_all_flags() {
        let mut rule = make_rule("flags");
        rule.command = "rm".to_string();
        rule.flags = Some(CommandFlagConfig {
            required: Some(vec!["-f".to_string()]),
            required_all: Some(vec!["-r".to_string(), "-f".to_string()]),
            ..CommandFlagConfig::default()
        });
        let parsed = parse_command("rm -rf target");
        assert!(find_matching_rule(&parsed, &[rule], None).is_some());
    }

    #[test]
    fn does_not_match_forbidden_flag() {
        let mut rule = make_rule("forbidden");
        rule.command = "git".to_string();
        rule.subcommand = Some("push".to_string());
        rule.flags = Some(CommandFlagConfig {
            dangerous: Some(vec!["--force".to_string()]),
            forbidden: Some(vec!["--force-with-lease".to_string()]),
            ..CommandFlagConfig::default()
        });
        let parsed = parse_command("git push --force --force-with-lease");
        assert!(find_matching_rule(&parsed, &[rule], None).is_none());
    }

    #[test]
    fn matches_args_with_regex_and_position() {
        let mut rule = make_rule("arg-match");
        rule.command = "echo".to_string();
        rule.subcommand = None;
        rule.args = Some(CommandArgConfig {
            pattern: Some("^risky$".to_string()),
            position: Some(0),
        });
        let parsed = parse_command("echo risky path");
        assert!(find_matching_rule(&parsed, &[rule], None).is_some());
    }

    #[test]
    fn strict_mode_condition_is_enforced() {
        let mut rule = make_rule("strict");
        rule.command = "rm".to_string();
        rule.conditions = Some(CommandConditions {
            strict_mode_only: Some(true),
            working_directory: None,
        });
        let parsed = parse_command("rm -r cache");

        let non_strict = MatcherContext {
            strict: Some(false),
            working_directory: None,
            cwd: None,
        };
        assert!(find_matching_rule(&parsed, &[rule.clone()], Some(&non_strict)).is_none());

        let strict = MatcherContext {
            strict: Some(true),
            working_directory: None,
            cwd: None,
        };
        assert!(find_matching_rule(&parsed, &[rule], Some(&strict)).is_some());
    }

    #[test]
    fn working_directory_condition_is_enforced() {
        let mut rule = make_rule("cwd");
        rule.command = "rm".to_string();
        rule.conditions = Some(CommandConditions {
            strict_mode_only: None,
            working_directory: Some(WorkingDirectoryCondition::Home),
        });
        let parsed = parse_command("rm -rf project");

        let home = MatcherContext {
            strict: None,
            working_directory: None,
            cwd: Some("/home/aneki/work".to_string()),
        };
        assert!(find_matching_rule(&parsed, &[rule.clone()], Some(&home)).is_some());

        let root = MatcherContext {
            strict: None,
            working_directory: None,
            cwd: Some("/".to_string()),
        };
        assert!(find_matching_rule(&parsed, &[rule], Some(&root)).is_none());
    }

    #[test]
    fn temp_path_exclusion_applies_when_enabled() {
        let mut rule = make_rule("tmp");
        rule.command = "rm".to_string();
        rule.flags = Some(CommandFlagConfig {
            dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
            ..CommandFlagConfig::default()
        });
        rule.args = Some(CommandArgConfig {
            pattern: Some("^/tmp".to_string()),
            position: None,
        });

        let parsed = parse_command("rm -rf /tmp/cache");
        let context = MatcherContext {
            strict: None,
            working_directory: Some(WorkingDirectoryConfig {
                allow_delete_in_cwd: Some(true),
                temp_dir_patterns: Some(vec!["/tmp".to_string()]),
            }),
            cwd: None,
        };
        assert!(find_matching_rule(&parsed, &[rule], Some(&context)).is_none());
    }

    #[test]
    fn analyse_returns_allow_when_no_rule_matches() {
        let parsed = parse_command("echo hello");
        let result = analyse_command("echo hello", &parsed, &[], None);
        assert_eq!(result.action, CommandAction::Allow);
        assert_eq!(result.severity, CommandSeverity::Info);
    }

    #[test]
    fn path_helpers_match_expected_paths() {
        assert!(is_home_path("/home/aneki"));
        assert!(is_root_path("/"));
        assert!(is_temp_path(
            "/tmp/cache",
            &["/tmp".to_string(), "/var/tmp".to_string()]
        ));
    }

    #[test]
    fn contains_unresolved_shell_expansion_detects_command_substitution() {
        assert!(contains_unresolved_shell_expansion("$(printf /)"));
        assert!(contains_unresolved_shell_expansion("`pwd`"));
        assert!(contains_unresolved_shell_expansion("${HOME}"));
        assert!(contains_unresolved_shell_expansion("$HOME"));
        assert!(contains_unresolved_shell_expansion("$1"));
        assert!(contains_unresolved_shell_expansion("$@"));
        assert!(contains_unresolved_shell_expansion("$$"));
        assert!(contains_unresolved_shell_expansion("$?"));
        assert!(contains_unresolved_shell_expansion("$*"));
        assert!(contains_unresolved_shell_expansion("$#"));
        assert!(!contains_unresolved_shell_expansion("/"));
        assert!(!contains_unresolved_shell_expansion("target"));
        assert!(!contains_unresolved_shell_expansion("file$"));
    }

    #[test]
    fn matches_rm_rf_root_when_target_is_command_substitution() {
        let rule = CommandRule {
            id: "rm-rf-root".to_string(),
            category: CommandCategory::Filesystem,
            command: "rm".to_string(),
            subcommand: None,
            flags: Some(CommandFlagConfig {
                dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
                ..CommandFlagConfig::default()
            }),
            args: Some(CommandArgConfig {
                pattern: Some(r"^/$".to_string()),
                position: None,
            }),
            action: CommandAction::Block,
            severity: CommandSeverity::Error,
            reason: "root".to_string(),
            suggestion: None,
            references: None,
            conditions: None,
        };
        let parsed = parse_command(r#"rm -rf "$(printf /)""#);
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.args, vec!["$(printf /)"]);
        assert!(find_matching_rule(&parsed, &[rule], None).is_some());
    }
}
