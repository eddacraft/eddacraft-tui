use regex::Regex;

use crate::command_safety::matcher::MatcherContext;
use crate::command_safety::parser::CommandParser;
use crate::command_safety::rules::{
    default_filesystem_rules, default_git_rules, default_shell_rules,
};
use crate::command_safety::types::{
    CommandAction, CommandAnalysisSummary, CommandRule, CommandRuleOverrideAction,
    CommandSafetyCheckResult, CommandSafetyConfig, CommandSafetyDetails, CommandSafetyFinding,
    CommandSafetyResolvedConfigInfo, ResolvedCommandSafetyConfig,
    ResolvedCommandSafetyOutputConfig, ResolvedWorkingDirectoryConfig, ScriptChangeType,
    ScriptPlan, WorkingDirectoryConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommandSafetyCheckContext {
    pub plan: Option<ScriptPlan>,
    #[serde(rename = "check_config")]
    pub check_config: Option<CommandSafetyConfig>,
    #[serde(rename = "workspace_root")]
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandSource {
    command: String,
    source: Option<String>,
}

#[must_use]
fn extract_commands_from_plan(context: &CommandSafetyCheckContext) -> Vec<CommandSource> {
    let mut commands = Vec::new();
    let Some(plan) = &context.plan else {
        return commands;
    };

    let Ok(code_block_pattern) = Regex::new(r"(?s)```(?:bash|sh|shell)?\r?\n(.*?)```") else {
        return commands;
    };

    for change in &plan.proposed_changes {
        if !matches!(change.change_type, ScriptChangeType::ScriptExecute) {
            continue;
        }
        let Some(description) = &change.description else {
            continue;
        };

        let mut matched_any_block = false;
        for captures in code_block_pattern.captures_iter(description) {
            if let Some(body) = captures.get(1) {
                matched_any_block = true;
                let mut continued = String::new();
                for line in body.as_str().lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        if !continued.is_empty() {
                            commands.push(CommandSource {
                                command: std::mem::take(&mut continued),
                                source: change
                                    .path
                                    .clone()
                                    .or_else(|| Some("script_execute".to_string())),
                            });
                        }
                        continue;
                    }
                    if trimmed.ends_with('\\') {
                        continued.push_str(trimmed.trim_end_matches('\\'));
                        continued.push(' ');
                    } else {
                        continued.push_str(trimmed);
                        commands.push(CommandSource {
                            command: std::mem::take(&mut continued),
                            source: change
                                .path
                                .clone()
                                .or_else(|| Some("script_execute".to_string())),
                        });
                    }
                }
                if !continued.is_empty() {
                    commands.push(CommandSource {
                        command: continued,
                        source: change
                            .path
                            .clone()
                            .or_else(|| Some("script_execute".to_string())),
                    });
                }
            }
        }
        if !matched_any_block {
            let mut continued = String::new();
            for line in description.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    if !continued.is_empty() {
                        commands.push(CommandSource {
                            command: std::mem::take(&mut continued),
                            source: change
                                .path
                                .clone()
                                .or_else(|| Some("script_execute".to_string())),
                        });
                    }
                    continue;
                }
                if trimmed.ends_with('\\') {
                    continued.push_str(trimmed.trim_end_matches('\\'));
                    continued.push(' ');
                } else {
                    continued.push_str(trimmed);
                    commands.push(CommandSource {
                        command: std::mem::take(&mut continued),
                        source: change
                            .path
                            .clone()
                            .or_else(|| Some("script_execute".to_string())),
                    });
                }
            }
            if !continued.is_empty() {
                commands.push(CommandSource {
                    command: continued,
                    source: change
                        .path
                        .clone()
                        .or_else(|| Some("script_execute".to_string())),
                });
            }
        }
    }

    commands
}

#[must_use]
fn load_rules(config: &CommandSafetyConfig) -> Vec<CommandRule> {
    let mut rules = [
        default_git_rules(),
        default_filesystem_rules(),
        default_shell_rules(),
    ]
    .concat();

    if let Some(disabled) = config
        .rules
        .as_ref()
        .and_then(|rules| rules.disabled.as_ref())
    {
        rules.retain(|rule| !disabled.contains(&rule.id));
    }

    if let Some(overrides) = config
        .rules
        .as_ref()
        .and_then(|rules| rules.overrides.as_ref())
    {
        for override_rule in overrides {
            if let Some(rule_index) = rules.iter().position(|rule| rule.id == override_rule.id) {
                match override_rule.action {
                    Some(CommandRuleOverrideAction::Disable) => {
                        let _ = rules.remove(rule_index);
                        continue;
                    }
                    Some(CommandRuleOverrideAction::Block) => {
                        rules[rule_index].action = CommandAction::Block;
                    }
                    Some(CommandRuleOverrideAction::Warn) => {
                        rules[rule_index].action = CommandAction::Warn;
                    }
                    Some(CommandRuleOverrideAction::Allow) => {
                        rules[rule_index].action = CommandAction::Allow;
                    }
                    None => {}
                }

                if let Some(severity) = override_rule.severity {
                    rules[rule_index].severity = severity;
                }
            }
        }
    }

    if let Some(custom) = config
        .rules
        .as_ref()
        .and_then(|rules| rules.custom.as_ref())
    {
        rules.extend(custom.iter().cloned());
    }

    rules
}

#[must_use]
fn format_blocked_message(
    blocked: &[CommandSafetyFinding],
    output_config: &ResolvedCommandSafetyOutputConfig,
) -> String {
    if blocked.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        format!("Blocked {} dangerous command(s):", blocked.len()),
        String::new(),
    ];
    for (index, finding) in blocked.iter().enumerate() {
        lines.push(format!("{}. {}", index + 1, finding.command));
        if output_config.verbose {
            lines.push(format!("   Reason: {}", finding.reason));
        }
        if output_config.show_suggestions
            && let Some(suggestion) = &finding.suggestion
        {
            lines.push(format!("   Suggestion: {suggestion}"));
        }
        if output_config.show_references
            && let Some(references) = &finding.references
            && let Some(reference) = references.first()
        {
            lines.push(format!("   Reference: {reference}"));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

#[must_use]
fn format_warning_message(
    warnings: &[CommandSafetyFinding],
    output_config: &ResolvedCommandSafetyOutputConfig,
) -> String {
    if warnings.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        format!("Found {} potentially dangerous command(s):", warnings.len()),
        String::new(),
    ];
    for (index, finding) in warnings.iter().enumerate() {
        lines.push(format!("{}. {}", index + 1, finding.command));
        if output_config.verbose {
            lines.push(format!("   Reason: {}", finding.reason));
        }
        if output_config.show_suggestions
            && let Some(suggestion) = &finding.suggestion
        {
            lines.push(format!("   Suggestion: {suggestion}"));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

#[must_use]
fn resolve_config(context: &CommandSafetyCheckContext) -> ResolvedCommandSafetyConfig {
    let config = context.check_config.clone().unwrap_or_default();
    let resolved_working_directory = ResolvedWorkingDirectoryConfig {
        allow_delete_in_cwd: config
            .working_directory
            .as_ref()
            .and_then(|working_directory| working_directory.allow_delete_in_cwd)
            .unwrap_or(false),
        temp_dir_patterns: config
            .working_directory
            .as_ref()
            .and_then(|working_directory| working_directory.temp_dir_patterns.clone())
            .unwrap_or_else(|| vec!["/tmp".to_string(), "/var/tmp".to_string()]),
    };
    let resolved_output = ResolvedCommandSafetyOutputConfig {
        verbose: config
            .output
            .as_ref()
            .and_then(|output| output.verbose)
            .unwrap_or(true),
        show_suggestions: config
            .output
            .as_ref()
            .and_then(|output| output.show_suggestions)
            .unwrap_or(true),
        show_references: config
            .output
            .as_ref()
            .and_then(|output| output.show_references)
            .unwrap_or(true),
    };

    let rules = load_rules(&config);

    ResolvedCommandSafetyConfig {
        enabled: config.enabled.unwrap_or(true),
        strict: config.strict.unwrap_or(false),
        rules,
        working_directory: resolved_working_directory,
        output: resolved_output,
    }
}

#[must_use]
fn calculate_score(summary: &CommandAnalysisSummary) -> u8 {
    if summary.total == 0 {
        return 100;
    }

    let blocked_penalty = summary.blocked.saturating_mul(25);
    let warned_penalty = summary.warned.saturating_mul(5);
    let total_penalty = blocked_penalty.saturating_add(warned_penalty);
    let score = 100_usize.saturating_sub(total_penalty);
    u8::try_from(score).unwrap_or(0)
}

#[derive(Debug, Clone, Default)]
struct AnalysisAggregate {
    blocked: Vec<CommandSafetyFinding>,
    warnings: Vec<CommandSafetyFinding>,
    allowed: usize,
    total_analysed: usize,
}

#[must_use]
fn resolved_config_info(
    context: &CommandSafetyCheckContext,
    resolved: &ResolvedCommandSafetyConfig,
) -> CommandSafetyResolvedConfigInfo {
    CommandSafetyResolvedConfigInfo {
        strict: resolved.strict,
        rules_count: resolved.rules.len(),
        custom_rules_count: context
            .check_config
            .as_ref()
            .and_then(|config| config.rules.as_ref())
            .and_then(|rules| rules.custom.as_ref())
            .map_or(0, Vec::len),
        disabled_rules_count: context
            .check_config
            .as_ref()
            .and_then(|config| config.rules.as_ref())
            .and_then(|rules| rules.disabled.as_ref())
            .map_or(0, Vec::len),
    }
}

#[must_use]
fn skipped_result() -> CommandSafetyCheckResult {
    let summary = CommandAnalysisSummary::default();
    let details = CommandSafetyDetails {
        blocked: Vec::new(),
        warnings: Vec::new(),
        summary: summary.clone(),
        config: None,
    };

    CommandSafetyCheckResult {
        passed: true,
        score: 100,
        message: "Command safety check disabled".to_string(),
        blocked: Vec::new(),
        warnings: Vec::new(),
        summary,
        details,
        formatted_blocked_message: String::new(),
        formatted_warning_message: String::new(),
        skipped: true,
    }
}

#[must_use]
fn no_commands_result(
    context: &CommandSafetyCheckContext,
    resolved: &ResolvedCommandSafetyConfig,
) -> CommandSafetyCheckResult {
    let summary = CommandAnalysisSummary::default();
    let details = CommandSafetyDetails {
        blocked: Vec::new(),
        warnings: Vec::new(),
        summary: summary.clone(),
        config: Some(resolved_config_info(context, resolved)),
    };

    CommandSafetyCheckResult {
        passed: true,
        score: 100,
        message: "No commands to analyse".to_string(),
        blocked: Vec::new(),
        warnings: Vec::new(),
        summary,
        details,
        formatted_blocked_message: String::new(),
        formatted_warning_message: String::new(),
        skipped: false,
    }
}

fn analyse_command_sources(
    command_sources: &[CommandSource],
    resolved: &ResolvedCommandSafetyConfig,
    workspace_root: Option<&str>,
) -> AnalysisAggregate {
    let parser = CommandParser;
    let working_config = WorkingDirectoryConfig {
        allow_delete_in_cwd: Some(resolved.working_directory.allow_delete_in_cwd),
        temp_dir_patterns: Some(resolved.working_directory.temp_dir_patterns.clone()),
    };
    let match_context = MatcherContext {
        strict: Some(resolved.strict),
        working_directory: Some(working_config),
        cwd: workspace_root.map(ToString::to_string),
    };

    let mut aggregate = AnalysisAggregate::default();

    for source in command_sources {
        let compound = parser.parse_compound(&source.command);
        for analysis in crate::command_safety::matcher::analyse_compound(
            &compound,
            &resolved.rules,
            Some(&match_context),
        ) {
            if analysis.parsed_command.command.is_empty()
                && !analysis.parsed_command.unwrap_incomplete
            {
                continue;
            }
            aggregate.total_analysed += 1;

            if matches!(analysis.action, CommandAction::Allow) || analysis.matched_rule.is_none() {
                aggregate.allowed += 1;
                continue;
            }

            let rule = analysis.matched_rule.as_ref().expect("matched");
            let finding = CommandSafetyFinding {
                command: if compound.is_compound {
                    format!("{} (from: {})", analysis.parsed_command.raw, source.command)
                } else {
                    analysis.parsed_command.raw.clone()
                },
                rule_id: rule.id.clone(),
                category: rule.category,
                action: analysis.action,
                severity: analysis.severity,
                reason: analysis
                    .reason
                    .clone()
                    .unwrap_or_else(|| rule.reason.clone()),
                suggestion: analysis.suggestion.clone(),
                references: analysis.references.clone(),
                source: source.source.clone(),
            };

            match analysis.action {
                CommandAction::Block => aggregate.blocked.push(finding),
                CommandAction::Warn => aggregate.warnings.push(finding),
                CommandAction::Allow => aggregate.allowed += 1,
            }
        }
    }

    aggregate
}

#[must_use]
fn final_result(
    context: &CommandSafetyCheckContext,
    resolved: &ResolvedCommandSafetyConfig,
    aggregate: AnalysisAggregate,
) -> CommandSafetyCheckResult {
    let summary = CommandAnalysisSummary {
        total: aggregate.total_analysed,
        blocked: aggregate.blocked.len(),
        warned: aggregate.warnings.len(),
        allowed: aggregate.allowed,
    };
    let score = calculate_score(&summary);
    let passed = aggregate.blocked.is_empty();
    let message = if passed && aggregate.warnings.is_empty() {
        format!("All {} command(s) passed safety check", summary.total)
    } else if passed {
        format!(
            "{} command(s) analysed: {} warning(s)",
            summary.total,
            aggregate.warnings.len()
        )
    } else {
        format!(
            "Command safety check failed: {} blocked, {} warning(s)",
            aggregate.blocked.len(),
            aggregate.warnings.len()
        )
    };
    let formatted_blocked_message = format_blocked_message(&aggregate.blocked, &resolved.output);
    let formatted_warning_message = format_warning_message(&aggregate.warnings, &resolved.output);
    let details = CommandSafetyDetails {
        blocked: aggregate.blocked.clone(),
        warnings: aggregate.warnings.clone(),
        summary: summary.clone(),
        config: Some(resolved_config_info(context, resolved)),
    };

    CommandSafetyCheckResult {
        passed,
        score,
        message,
        blocked: aggregate.blocked,
        warnings: aggregate.warnings,
        summary,
        details,
        formatted_blocked_message,
        formatted_warning_message,
        skipped: false,
    }
}

#[must_use]
pub fn run_command_safety_check(context: &CommandSafetyCheckContext) -> CommandSafetyCheckResult {
    let resolved = resolve_config(context);
    if !resolved.enabled {
        return skipped_result();
    }

    let command_sources = extract_commands_from_plan(context);
    if command_sources.is_empty() {
        return no_commands_result(context, &resolved);
    }

    let aggregate = analyse_command_sources(
        &command_sources,
        &resolved,
        context.workspace_root.as_deref(),
    );
    final_result(context, &resolved, aggregate)
}

#[cfg(test)]
mod tests {
    use crate::command_safety::check::{CommandSafetyCheckContext, run_command_safety_check};
    use crate::command_safety::types::{
        CommandAction, CommandAnalysisSummary, CommandRuleOverride, CommandRuleOverrideAction,
        CommandRulesConfig, CommandSafetyConfig, ScriptChange, ScriptChangeType, ScriptPlan,
    };

    fn plan_with_commands(commands: &[&str]) -> ScriptPlan {
        let body = commands.join("\n");
        ScriptPlan {
            proposed_changes: vec![ScriptChange {
                change_type: ScriptChangeType::ScriptExecute,
                description: Some(format!("```bash\n{body}\n```")),
                path: Some("plans/execution/TUI-001.steps.md".to_string()),
            }],
        }
    }

    #[test]
    fn passes_when_no_commands_found() {
        let context = CommandSafetyCheckContext::default();
        let result = run_command_safety_check(&context);
        assert!(result.passed);
        assert_eq!(result.score, 100);
        assert_eq!(result.message, "No commands to analyse");
    }

    #[test]
    fn passes_when_all_commands_are_allowed() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["git clean -n"])),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert!(result.passed);
        assert_eq!(
            result.summary,
            CommandAnalysisSummary {
                total: 1,
                blocked: 0,
                warned: 0,
                allowed: 1
            }
        );
    }

    #[test]
    fn blocked_command_fails_with_expected_score() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["git push --force"])),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert!(!result.passed);
        assert_eq!(result.score, 75);
        assert_eq!(result.summary.blocked, 1);
    }

    #[test]
    fn mixed_blocked_and_warned_calculates_score() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["git push --force", "git clean -f"])),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert_eq!(result.score, 70);
        assert_eq!(result.summary.blocked, 1);
        assert_eq!(result.summary.warned, 1);
    }

    #[test]
    fn disabled_config_skips_check() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["git push --force"])),
            check_config: Some(CommandSafetyConfig {
                enabled: Some(false),
                ..CommandSafetyConfig::default()
            }),
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert!(result.passed);
        assert!(result.skipped);
        assert_eq!(result.message, "Command safety check disabled");
    }

    #[test]
    fn compound_command_detects_force_push() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["git add . && git push --force"])),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert_eq!(result.summary.total, 2);
        assert_eq!(result.blocked.len(), 1);
        assert!(result.blocked[0].command.contains("git push --force"));
    }

    #[test]
    fn applies_rule_override_disable() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["git push --force"])),
            check_config: Some(CommandSafetyConfig {
                enabled: Some(true),
                strict: None,
                rules: Some(CommandRulesConfig {
                    overrides: Some(vec![CommandRuleOverride {
                        id: "git-push-force".to_string(),
                        action: Some(CommandRuleOverrideAction::Disable),
                        severity: None,
                    }]),
                    custom: None,
                    disabled: None,
                }),
                working_directory: None,
                output: None,
            }),
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert!(result.passed);
    }

    #[test]
    fn applies_custom_rule() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["echo risky"])),
            check_config: Some(CommandSafetyConfig {
                enabled: Some(true),
                strict: None,
                rules: Some(CommandRulesConfig {
                    overrides: None,
                    custom: Some(vec![crate::command_safety::types::CommandRule {
                        id: "custom-echo".to_string(),
                        category: crate::command_safety::types::CommandCategory::Custom,
                        command: "echo".to_string(),
                        subcommand: None,
                        flags: None,
                        args: Some(crate::command_safety::types::CommandArgConfig {
                            pattern: Some("^risky$".to_string()),
                            position: None,
                        }),
                        action: CommandAction::Warn,
                        severity: crate::command_safety::types::CommandSeverity::Warning,
                        reason: "custom rule".to_string(),
                        suggestion: None,
                        references: None,
                        conditions: None,
                    }]),
                    disabled: None,
                }),
                working_directory: None,
                output: None,
            }),
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert_eq!(result.summary.warned, 1);
    }

    #[test]
    fn extracts_commands_from_multiline_unfenced_description() {
        let context = CommandSafetyCheckContext {
            plan: Some(ScriptPlan {
                proposed_changes: vec![ScriptChange {
                    change_type: ScriptChangeType::ScriptExecute,
                    description: Some(
                        "git push --force\nThis pushes changes to the remote".to_string(),
                    ),
                    path: None,
                }],
            }),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert_eq!(result.summary.blocked, 1);
        assert!(result.blocked[0].command.contains("git push --force"));
    }

    #[test]
    fn blocks_rm_rf_root_via_absolute_executable_path() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&["/bin/rm -rf /"])),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert!(
            !result.passed,
            "absolute path form must not evade rm-rf-root"
        );
        assert_eq!(result.summary.blocked, 1);
        assert_eq!(result.blocked[0].rule_id, "rm-rf-root");
    }

    #[test]
    fn blocks_rm_rf_root_via_shell_command_substitution_target() {
        // Shell resolves "$(printf /)" to "/"; lexical safety must not allow it.
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&[r#"bash -c 'rm -rf "$(printf /)"'"#])),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert!(
            !result.passed,
            "command-substitution root target must not be allowed; summary={:?}",
            result.summary
        );
        assert!(
            result.summary.blocked >= 1,
            "expected at least one blocked finding; got {:?}",
            result.blocked
        );
    }

    #[test]
    fn blocks_rm_rf_root_via_path_form_shell_wrapper() {
        let context = CommandSafetyCheckContext {
            plan: Some(plan_with_commands(&[r#"/bin/bash -c "rm -rf /""#])),
            check_config: None,
            workspace_root: Some("/home/aneki/project".to_string()),
        };
        let result = run_command_safety_check(&context);
        assert!(!result.passed);
        assert_eq!(result.summary.blocked, 1);
        assert_eq!(result.blocked[0].rule_id, "rm-rf-root");
    }
}
