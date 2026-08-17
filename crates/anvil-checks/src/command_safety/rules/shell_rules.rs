use crate::command_safety::types::{
    CommandAction, CommandArgConfig, CommandCategory, CommandRule, CommandSeverity,
};

/// Sentinel command name for the compound pipe-to-shell rule. It never matches
/// a parsed argv; `analyse_compound` looks the rule up by id instead.
pub const PIPE_TO_SHELL_SENTINEL: &str = "__pipeline__";
pub const PIPE_TO_SHELL_RULE_ID: &str = "pipe-to-shell";
pub const EVAL_DYNAMIC_RULE_ID: &str = "eval-dynamic";
pub const CHMOD_777_RULE_ID: &str = "chmod-777";

fn shell_rule(
    id: &str,
    command: &str,
    action: CommandAction,
    severity: CommandSeverity,
    reason: &str,
) -> CommandRule {
    CommandRule {
        id: id.to_string(),
        category: CommandCategory::Shell,
        command: command.to_string(),
        subcommand: None,
        flags: None,
        args: None,
        action,
        severity,
        reason: reason.to_string(),
        suggestion: None,
        references: None,
        conditions: None,
    }
}

fn chmod_777() -> CommandRule {
    let mut rule = shell_rule(
        CHMOD_777_RULE_ID,
        "chmod",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "chmod 777 makes the target world-writable",
    );
    rule.args = Some(CommandArgConfig {
        // Position 0 is the mode token (or a mode parsed as subcommand).
        // Checking only that slot avoids fail-closed hits on `chmod 644 $file`.
        pattern: Some(r"^(0)?777$".to_string()),
        position: Some(0),
    });
    rule.suggestion = Some(
        "Use more restrictive permissions like 755 for directories, 644 for files".to_string(),
    );
    rule
}

fn eval_dynamic() -> CommandRule {
    let mut rule = shell_rule(
        EVAL_DYNAMIC_RULE_ID,
        "eval",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "eval with a dynamic argument executes attacker-controlled shell",
    );
    rule.args = Some(CommandArgConfig {
        // Dynamic: command substitutions, named parameters (including when
        // followed by a suffix), and positional/special parameters. ANSI-C
        // quoted literals retain their quote marker in the parser and do not
        // match this expression.
        pattern: Some(r"(?:\$\(|\$\{|`|\$(?:[A-Za-z_][A-Za-z0-9_]*|[0-9]+|[@*#?$!-]))".to_string()),
        position: None,
    });
    rule.suggestion =
        Some("Avoid eval; use a static command or a quoted literal with no expansions".to_string());
    rule
}

fn pipe_to_shell() -> CommandRule {
    let mut rule = shell_rule(
        PIPE_TO_SHELL_RULE_ID,
        PIPE_TO_SHELL_SENTINEL,
        CommandAction::Block,
        CommandSeverity::Error,
        "piping a download straight to a shell runs unverified code",
    );
    rule.suggestion =
        Some("Download the script, verify its checksum or signature, then run it".to_string());
    rule
}

#[must_use]
pub fn build_default_shell_rules() -> Vec<CommandRule> {
    vec![chmod_777(), eval_dynamic(), pipe_to_shell()]
}

#[cfg(test)]
mod tests {
    use super::build_default_shell_rules;
    use crate::command_safety::matcher::find_matching_rule;
    use crate::command_safety::parser::parse_command;
    use crate::command_safety::types::CommandAction;

    fn rules() -> Vec<crate::command_safety::types::CommandRule> {
        build_default_shell_rules()
    }

    #[test]
    fn includes_three_shell_rules() {
        assert_eq!(build_default_shell_rules().len(), 3);
    }

    #[test]
    fn warns_on_chmod_777_and_0777() {
        for cmd in ["chmod 777 file", "chmod 0777 file"] {
            let parsed = parse_command(cmd);
            let matched = find_matching_rule(&parsed, &rules(), None)
                .unwrap_or_else(|| panic!("expected a match for {cmd}"));
            assert_eq!(matched.id, "chmod-777");
            assert_eq!(matched.action, CommandAction::Warn);
        }
    }

    #[test]
    fn ignores_restrictive_chmod() {
        let parsed = parse_command("chmod 755 file");
        assert!(find_matching_rule(&parsed, &rules(), None).is_none());
    }

    #[test]
    fn ignores_chmod_644_with_expansion_target() {
        // Fail-closed expansion matching must not treat `$file` as mode 777.
        let parsed = parse_command("chmod 644 $file");
        assert!(find_matching_rule(&parsed, &rules(), None).is_none());
        let parsed = parse_command("chmod $mode file");
        assert!(find_matching_rule(&parsed, &rules(), None).is_none());
    }

    #[test]
    fn warns_on_dynamic_eval() {
        for cmd in [
            "eval $cmd",
            "eval \"$x\"",
            "eval `echo hi`",
            "eval $cmd suffix",
            "eval $cmd/path",
            "eval $1",
            "eval $@",
            "eval $?",
            "eval $$",
        ] {
            let parsed = parse_command(cmd);
            let matched = find_matching_rule(&parsed, &rules(), None)
                .unwrap_or_else(|| panic!("expected a match for {cmd}"));
            assert_eq!(matched.id, "eval-dynamic");
            assert_eq!(matched.action, CommandAction::Warn);
        }
    }

    #[test]
    fn ignores_static_eval() {
        for cmd in ["eval 'echo ok'", "eval echo hello", "eval $'echo ok'"] {
            let parsed = parse_command(cmd);
            assert!(
                find_matching_rule(&parsed, &rules(), None).is_none(),
                "static eval should not match: {cmd} parsed={parsed:?}"
            );
        }
    }

    #[test]
    fn ignores_numeric_filename_when_chmod_uses_reference_mode() {
        let parsed = parse_command("chmod --reference=source 777");
        assert!(find_matching_rule(&parsed, &rules(), None).is_none());
    }
}
