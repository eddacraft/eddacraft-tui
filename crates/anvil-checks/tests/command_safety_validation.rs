//! Integration tests for the command safety validation module.
//!
//! These tests exercise the public API (`anvil_checks::command_safety::*`)
//! with realistic shell commands that developers and CI systems run.

use anvil_checks::command_safety::{
    CommandAction, CommandSafetyConfig, ScriptChange, ScriptChangeType, ScriptPlan,
};
use anvil_checks::command_safety::{
    CommandParser, CommandSafetyCheckContext, MatcherContext, RuleMatcher, analyse_command,
    default_filesystem_rules, default_git_rules, default_shell_rules, find_matching_rule,
    parse_command, parse_compound_command, run_command_safety_check,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn all_rules() -> Vec<anvil_checks::command_safety::CommandRule> {
    let mut rules = default_git_rules();
    rules.extend(default_filesystem_rules());
    rules.extend(default_shell_rules());
    rules
}

fn plan_with(commands: &[&str]) -> ScriptPlan {
    let body = commands.join("\n");
    ScriptPlan {
        proposed_changes: vec![ScriptChange {
            change_type: ScriptChangeType::ScriptExecute,
            description: Some(format!("```bash\n{body}\n```")),
            path: Some("plans/test.md".to_string()),
        }],
    }
}

// ---------------------------------------------------------------------------
// Command parsing — realistic developer commands
// ---------------------------------------------------------------------------

#[test]
fn parses_simple_git_workflow() {
    let parser = CommandParser;

    let status = parser.parse("git status");
    assert_eq!(status.command, "git");
    assert_eq!(status.subcommand.as_deref(), Some("status"));
    assert!(status.flags.is_empty());

    let add = parser.parse("git add src/main.ts");
    assert_eq!(add.subcommand.as_deref(), Some("add"));
    assert_eq!(add.args, vec!["src/main.ts"]);

    let commit = parser.parse("git commit -m 'fix: resolve type error'");
    assert_eq!(commit.subcommand.as_deref(), Some("commit"));
    assert!(commit.flags.contains(&"-m".to_string()));
}

#[test]
fn parses_npm_ci_pipeline() {
    let parser = CommandParser;

    let install = parser.parse("npm ci --prefer-offline");
    assert_eq!(install.command, "npm");
    assert_eq!(install.subcommand.as_deref(), Some("ci"));
    assert!(install.flags.contains(&"--prefer-offline".to_string()));

    let test = parser.parse("npm run test -- --coverage");
    assert_eq!(test.subcommand.as_deref(), Some("run"));
}

#[test]
fn parses_docker_build_with_tags() {
    let parsed = parse_command("docker build -t myapp:latest -f Dockerfile.prod .");
    assert_eq!(parsed.command, "docker");
    assert_eq!(parsed.subcommand.as_deref(), Some("build"));
    assert!(parsed.flags.contains(&"-t".to_string()));
    assert!(parsed.flags.contains(&"-f".to_string()));
}

#[test]
fn parses_cargo_commands() {
    let parsed = parse_command("cargo test -p anvil-checks --release");
    assert_eq!(parsed.command, "cargo");
    assert_eq!(parsed.subcommand.as_deref(), Some("test"));
    assert!(parsed.flags.contains(&"-p".to_string()));
    assert!(parsed.flags.contains(&"--release".to_string()));
}

// ---------------------------------------------------------------------------
// Compound command parsing
// ---------------------------------------------------------------------------

#[test]
fn parses_typical_ci_pipeline_chain() {
    let result = parse_compound_command("npm ci && npm run lint && npm run test && npm run build");
    assert!(result.is_compound);
    assert_eq!(result.commands.len(), 4);
    assert_eq!(result.operators.len(), 3);
    assert!(result.operators.iter().all(|op| op == "&&"));
}

#[test]
fn parses_pipe_with_grep_and_head() {
    let result = parse_compound_command("git log --oneline | grep 'fix' | head -5");
    assert!(result.is_compound);
    assert_eq!(result.commands.len(), 3);
    assert_eq!(result.commands[0].command, "git");
    assert_eq!(result.commands[1].command, "grep");
    assert_eq!(result.commands[2].command, "head");
}

#[test]
fn parses_semicolon_separated_commands() {
    let result = parse_compound_command("echo 'starting'; npm install; echo 'done'");
    assert!(result.is_compound);
    assert_eq!(result.commands.len(), 3);
}

#[test]
fn parses_mixed_operators() {
    let result = parse_compound_command("git add . && git commit -m 'wip' || echo 'failed'");
    assert!(result.is_compound);
    assert_eq!(result.commands.len(), 3);
    assert!(result.operators.contains(&"&&".to_string()));
    assert!(result.operators.contains(&"||".to_string()));
}

// ---------------------------------------------------------------------------
// Wrapper unwrapping
// ---------------------------------------------------------------------------

#[test]
fn unwraps_sudo_with_user_flag() {
    let parsed = parse_command("sudo -u deploy git pull origin main");
    assert_eq!(parsed.command, "git");
    assert_eq!(parsed.subcommand.as_deref(), Some("pull"));
    assert_eq!(parsed.wrapper_chain, vec!["sudo"]);
}

#[test]
fn unwraps_bash_c_with_inner_command() {
    let parsed = parse_command("bash -c 'git push --force'");
    assert_eq!(parsed.command, "git");
    assert_eq!(parsed.subcommand.as_deref(), Some("push"));
    assert!(parsed.flags.contains(&"--force".to_string()));
    assert_eq!(parsed.wrapper_chain, vec!["bash"]);
}

#[test]
fn unwraps_env_with_variables() {
    let parsed = parse_command("env NODE_ENV=production npm start");
    assert_eq!(parsed.command, "npm");
    assert_eq!(parsed.subcommand.as_deref(), Some("start"));
    assert_eq!(parsed.wrapper_chain, vec!["env"]);
}

#[test]
fn unwraps_nested_sudo_bash() {
    // sudo wrapping is peeled first, then bash -c is peeled, revealing
    // the inner command. The compound parse path handles the full unwrap
    // better than single-command parse for nested wrappers.
    let result = parse_compound_command("sudo bash -c \"rm -rf /var/cache\"");

    // The compound parser should expose the inner rm command
    let rm_cmd = result.commands.iter().find(|c| c.command == "rm");
    assert!(rm_cmd.is_some(), "should find the inner rm command");

    let rm = rm_cmd.unwrap();
    // CLAWP-054: assert the FULL nested wrapper chain in order. The prior
    // `contains(sudo) || contains(bash)` passed if either token appeared,
    // so a regression that lost one wrapper (or reordered the provenance)
    // went undetected. `sudo bash -c "rm ..."` must unwrap to exactly
    // ["sudo", "bash"].
    assert_eq!(
        rm.wrapper_chain,
        vec!["sudo".to_string(), "bash".to_string()],
        "nested wrapper chain should be exactly [sudo, bash], got: {:?}",
        rm.wrapper_chain
    );
}

#[test]
fn identifies_wrapped_commands() {
    let parser = CommandParser;
    assert!(parser.is_wrapped("sudo git status"));
    assert!(parser.is_wrapped("bash -c 'echo hello'"));
    assert!(parser.is_wrapped("env FOO=bar npm test"));
    assert!(!parser.is_wrapped("git status"));
    assert!(!parser.is_wrapped("npm test"));
}

// ---------------------------------------------------------------------------
// Rule matching — dangerous git commands
// ---------------------------------------------------------------------------

#[test]
fn blocks_git_push_force() {
    let parsed = parse_command("git push --force");
    let rules = all_rules();
    let matched = find_matching_rule(&parsed, &rules, None);

    assert!(
        matched.is_some(),
        "should match a rule for git push --force"
    );
    let rule = matched.unwrap();
    assert_eq!(rule.action, CommandAction::Block);
}

#[test]
fn blocks_git_reset_hard() {
    let parsed = parse_command("git reset --hard HEAD~3");
    let rules = all_rules();
    let matched = find_matching_rule(&parsed, &rules, None);

    assert!(
        matched.is_some(),
        "should match a rule for git reset --hard"
    );
    assert_eq!(matched.unwrap().action, CommandAction::Block);
}

#[test]
fn warns_on_git_clean_f() {
    let parsed = parse_command("git clean -f");
    let rules = all_rules();
    let matched = find_matching_rule(&parsed, &rules, None);

    assert!(matched.is_some(), "should match a rule for git clean -f");
    let rule = matched.unwrap();
    // CLAWP-055: pin the action to exactly `Warn`. The loose
    // `Warn | Block` matcher was internally inconsistent with
    // `check_reports_correct_score_for_mixed_findings`, which counts
    // `git clean -f` as a single *warning* (`summary.warned == 1`). The
    // two tests must encode the same contract.
    assert_eq!(
        rule.action,
        CommandAction::Warn,
        "git clean -f is a Warn (matches the score test's warned==1)"
    );
}

#[test]
fn allows_safe_git_commands() {
    let safe_commands = [
        "git status",
        "git log --oneline",
        "git diff HEAD~1",
        "git branch -a",
        "git fetch origin",
        "git pull --rebase",
        "git stash list",
    ];

    let rules = all_rules();
    for cmd in safe_commands {
        let parsed = parse_command(cmd);
        let matched = find_matching_rule(&parsed, &rules, None);
        assert!(
            matched.is_none() || matches!(matched.as_ref().unwrap().action, CommandAction::Allow),
            "{cmd} should be allowed, but matched rule: {:?}",
            matched.map(|r| r.id)
        );
    }
}

// ---------------------------------------------------------------------------
// Rule matching — dangerous filesystem commands
// ---------------------------------------------------------------------------

#[test]
fn blocks_rm_rf_on_root() {
    let parsed = parse_command("rm -rf /");
    let rules = all_rules();
    let matched = find_matching_rule(&parsed, &rules, None);

    assert!(matched.is_some(), "should match rm -rf /");
    assert_eq!(matched.unwrap().action, CommandAction::Block);
}

#[test]
fn blocks_rm_rf_on_home_variable() {
    // The rm-rf-home rule matches shell variable patterns (~, $HOME),
    // not literal /home/ paths.
    let parsed = parse_command("rm -rf ~");
    let rules = all_rules();
    let matched = find_matching_rule(&parsed, &rules, None);

    assert!(matched.is_some(), "should match rm -rf ~ (home dir)");
    assert_eq!(matched.unwrap().action, CommandAction::Block);
}

#[test]
fn blocks_rm_rf_on_system_dir() {
    let parsed = parse_command("rm -rf /var");
    let rules = all_rules();
    let matched = find_matching_rule(&parsed, &rules, None);

    assert!(matched.is_some(), "should match rm -rf /var (system dir)");
    assert_eq!(matched.unwrap().action, CommandAction::Block);
}

// ---------------------------------------------------------------------------
// Wrapper-aware rule matching
// ---------------------------------------------------------------------------

#[test]
fn detects_force_push_through_bash_wrapper() {
    let result = parse_compound_command("bash -c 'git push --force'");
    let rules = all_rules();

    let has_blocked = result.commands.iter().any(|parsed| {
        find_matching_rule(parsed, &rules, None)
            .is_some_and(|rule| rule.action == CommandAction::Block)
    });

    assert!(
        has_blocked,
        "should detect force push even through bash -c wrapper"
    );
}

#[test]
fn detects_dangerous_command_through_sudo() {
    // Use a path that matches a known rule (system dir or root)
    let parsed = parse_command("sudo rm -rf /etc");
    let rules = all_rules();
    let matched = find_matching_rule(&parsed, &rules, None);

    assert!(
        matched.is_some(),
        "should detect dangerous rm on system dir through sudo wrapper"
    );
    assert_eq!(matched.unwrap().action, CommandAction::Block);
}

#[test]
fn blocks_deeply_nested_env_wrappers_around_rm_rf_root() {
    // CLAWP: nested recognised wrappers beyond MAX_UNWRAP_DEPTH must not
    // fail open. Six `env` wrappers exceed the historical depth-5 limit and
    // previously left the matcher comparing the residual `env` token, so the
    // root-deletion rule never matched.
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&["env env env env env env rm -rf /"])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);

    assert!(
        !result.passed,
        "six nested env wrappers around rm -rf / must not pass as allowed"
    );
    assert!(
        result.summary.blocked >= 1,
        "expected at least one blocked finding, got summary={:?}",
        result.summary
    );
}

// ---------------------------------------------------------------------------
// Compound command analysis
// ---------------------------------------------------------------------------

#[test]
fn compound_chain_with_one_dangerous_command() {
    let result = parse_compound_command("git add . && git commit -m 'deploy' && git push --force");
    let rules = all_rules();

    let mut blocked_count = 0;
    let mut safe_count = 0;
    for parsed in &result.commands {
        match find_matching_rule(parsed, &rules, None) {
            Some(rule) if rule.action == CommandAction::Block => blocked_count += 1,
            None => safe_count += 1,
            Some(rule) if rule.action == CommandAction::Allow => safe_count += 1,
            _ => {}
        }
    }

    assert_eq!(blocked_count, 1, "only the force push should be blocked");
    assert!(safe_count >= 1, "other commands should be safe");
}

// ---------------------------------------------------------------------------
// RuleMatcher struct
// ---------------------------------------------------------------------------

#[test]
fn rule_matcher_analyses_multiple_commands() {
    let matcher = RuleMatcher::new(all_rules(), None);

    let commands: Vec<(String, _)> = vec![
        ("git status".to_string(), parse_command("git status")),
        (
            "git push --force".to_string(),
            parse_command("git push --force"),
        ),
    ];

    let results = matcher.analyse_multiple(&commands, None);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].action, CommandAction::Allow);
    assert_eq!(results[1].action, CommandAction::Block);
}

#[test]
fn analyse_command_returns_allow_for_unknown_commands() {
    let parsed = parse_command("echo hello world");
    let result = analyse_command("echo hello world", &parsed, &all_rules(), None);

    assert_eq!(result.action, CommandAction::Allow);
    assert!(result.matched_rule.is_none());
}

// ---------------------------------------------------------------------------
// Matcher context — strict mode, working directory
// ---------------------------------------------------------------------------

#[test]
fn strict_mode_enables_additional_rules() {
    let rules = all_rules();

    // The rm-rf-with-recursive rule is strict-mode-only and requires -r flag.
    // Use a command that matches it (rm -r on an arbitrary path) but would
    // not be caught by more specific rules (avoid /, ~, system dirs).
    let parsed = parse_command("rm -r my-project-cache");

    let non_strict = MatcherContext {
        strict: Some(false),
        ..MatcherContext::default()
    };
    let strict = MatcherContext {
        strict: Some(true),
        ..MatcherContext::default()
    };

    let without = find_matching_rule(&parsed, &rules, Some(&non_strict));
    let with = find_matching_rule(&parsed, &rules, Some(&strict));

    assert!(
        without.is_none(),
        "non-strict context should not match strict-only rule"
    );
    assert!(
        with.is_some(),
        "strict context should match the rm-rf-with-recursive rule"
    );
    assert_eq!(with.unwrap().id, "rm-rf-with-recursive");
}

// ---------------------------------------------------------------------------
// run_command_safety_check — full orchestration
// ---------------------------------------------------------------------------

#[test]
fn check_passes_for_safe_plan() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&["git status", "npm test", "cargo build"])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);

    assert!(result.passed);
    assert_eq!(result.score, 100);
    assert!(result.blocked.is_empty());
    assert!(result.warnings.is_empty());
}

#[test]
fn check_blocks_force_push_in_plan() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&["git push --force origin main"])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);

    assert!(!result.passed);
    assert!(result.score < 100);
    assert_eq!(result.summary.blocked, 1);
    assert!(!result.formatted_blocked_message.is_empty());
}

#[test]
fn check_handles_compound_commands_in_plan() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&["git add . && git push --force"])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);

    assert!(!result.passed);
    assert_eq!(result.summary.total, 2);
    assert_eq!(result.summary.blocked, 1);
}

#[test]
fn check_disabled_returns_skipped_result() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&["git push --force"])),
        check_config: Some(CommandSafetyConfig {
            enabled: Some(false),
            ..CommandSafetyConfig::default()
        }),
        workspace_root: None,
    };
    let result = run_command_safety_check(&context);

    assert!(result.passed);
    assert!(result.skipped);
    assert_eq!(result.score, 100);
}

#[test]
fn check_with_empty_plan_returns_no_commands() {
    let context = CommandSafetyCheckContext {
        plan: None,
        check_config: None,
        workspace_root: None,
    };
    let result = run_command_safety_check(&context);

    assert!(result.passed);
    assert_eq!(result.message, "No commands to analyse");
}

#[test]
fn check_reports_correct_score_for_mixed_findings() {
    // One blocked (25 points) + one warned (5 points) = score 70
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&["git push --force", "git clean -f"])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);

    assert!(!result.passed);
    assert_eq!(result.score, 70);
    assert_eq!(result.summary.blocked, 1);
    assert_eq!(result.summary.warned, 1);
}

// ---------------------------------------------------------------------------
// Realistic CI/CD script plans
// ---------------------------------------------------------------------------

#[test]
fn realistic_deploy_script_with_safe_commands() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&[
            "npm ci --prefer-offline",
            "npm run lint",
            "npm run test -- --coverage",
            "npm run build",
            "git tag v1.2.3",
            "git push origin v1.2.3",
        ])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);

    assert!(
        result.passed,
        "a standard deploy pipeline should pass safety checks"
    );
}

#[test]
fn realistic_dangerous_cleanup_script() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&[
            "git reset --hard HEAD~5",
            "git clean -fdx",
            "rm -rf node_modules dist .next",
            "git push --force origin main",
        ])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);

    assert!(!result.passed, "dangerous cleanup script should fail");
    assert!(
        result.summary.blocked >= 2,
        "should block at least force push and hard reset"
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn handles_empty_command_gracefully() {
    let parsed = parse_command("");
    assert!(parsed.command.is_empty());

    let result = parse_compound_command("   ");
    assert!(!result.is_compound);
    assert_eq!(result.commands.len(), 1);
    assert!(result.commands[0].command.is_empty());
}

#[test]
fn handles_commands_with_quoted_arguments() {
    let parsed = parse_command("git commit -m 'feat: add new gate check'");
    assert_eq!(parsed.command, "git");
    assert_eq!(parsed.subcommand.as_deref(), Some("commit"));
    assert!(parsed.flags.contains(&"-m".to_string()));
}

#[test]
fn handles_commands_with_environment_variable_prefix() {
    let parsed = parse_command("NODE_ENV=production npm start");
    assert_eq!(parsed.command, "npm");
    assert_eq!(parsed.subcommand.as_deref(), Some("start"));
}

#[test]
fn handles_git_global_options_before_subcommand() {
    let parsed = parse_command("git -C /tmp/repo status");
    assert_eq!(parsed.subcommand.as_deref(), Some("status"));
    // /tmp/repo is a global option value, not a positional arg
    assert!(!parsed.args.contains(&"/tmp/repo".to_string()));
}

#[test]
fn check_blocks_pipe_to_shell_install() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&[
            "curl --proto '=https' -LsSf https://example.com/install.sh | sh",
        ])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);
    assert!(!result.passed);
    assert!(
        result
            .blocked
            .iter()
            .any(|finding| finding.rule_id == "pipe-to-shell"),
        "expected pipe-to-shell block, got {:?}",
        result.blocked
    );
}

#[test]
fn check_warns_on_dynamic_eval_and_chmod_777() {
    let context = CommandSafetyCheckContext {
        plan: Some(plan_with(&["eval \"$cmd\"", "chmod 777 secret.key"])),
        check_config: None,
        workspace_root: Some("/home/dev/project".to_string()),
    };
    let result = run_command_safety_check(&context);
    assert!(result.passed, "warn-only rules must not fail the check");
    let ids: Vec<&str> = result
        .warnings
        .iter()
        .map(|finding| finding.rule_id.as_str())
        .collect();
    assert!(ids.contains(&"eval-dynamic"), "warnings={ids:?}");
    assert!(ids.contains(&"chmod-777"), "warnings={ids:?}");
}
