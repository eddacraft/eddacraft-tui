use crate::command_safety::types::{
    CommandAction, CommandArgConfig, CommandCategory, CommandFlagConfig, CommandRule,
    CommandSeverity,
};

fn git_rule(
    id: &str,
    subcommand: &str,
    action: CommandAction,
    severity: CommandSeverity,
    reason: &str,
) -> CommandRule {
    CommandRule {
        id: id.to_string(),
        category: CommandCategory::Git,
        command: "git".to_string(),
        subcommand: Some(subcommand.to_string()),
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

fn git_reset_hard() -> CommandRule {
    let mut rule = git_rule(
        "git-reset-hard",
        "reset",
        CommandAction::Block,
        CommandSeverity::Error,
        "git reset --hard permanently destroys uncommitted changes",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["--hard".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion = Some(
        "Use \"git stash\" first to preserve your work, or \"git reset --soft\" for a safer alternative"
            .to_string(),
    );
    rule.references = Some(vec![
        "https://git-scm.com/docs/git-reset".to_string(),
        "https://ohshitgit.com/#accidental-commit-wrong-branch".to_string(),
    ]);
    rule
}

fn git_reset_merge() -> CommandRule {
    let mut rule = git_rule(
        "git-reset-merge",
        "reset",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git reset --merge can lose uncommitted changes during conflict resolution",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["--merge".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion =
        Some("Ensure all changes are committed or stashed before using --merge".to_string());
    rule
}

fn git_checkout_discard() -> CommandRule {
    let mut rule = git_rule(
        "git-checkout-discard",
        "checkout",
        CommandAction::Block,
        CommandSeverity::Error,
        "git checkout -- discards uncommitted changes permanently",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["--".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion =
        Some("Use \"git stash\" to preserve changes, or \"git diff\" to review first".to_string());
    rule.references = Some(vec!["https://git-scm.com/docs/git-checkout".to_string()]);
    rule
}

fn git_checkout_all() -> CommandRule {
    let mut rule = git_rule(
        "git-checkout-all",
        "checkout",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git checkout . discards all uncommitted changes in the working tree",
    );
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^\.$".to_string()),
        position: None,
    });
    rule.suggestion =
        Some("Use \"git stash\" to preserve changes, or \"git diff\" to review first".to_string());
    rule
}

fn git_restore_worktree() -> CommandRule {
    let mut rule = git_rule(
        "git-restore-worktree",
        "restore",
        CommandAction::Block,
        CommandSeverity::Error,
        "git restore discards uncommitted changes permanently",
    );
    rule.flags = Some(CommandFlagConfig {
        forbidden: Some(vec!["--staged".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion =
        Some("Use \"git stash\" first, or \"git restore --staged\" to only unstage".to_string());
    rule
}

fn git_clean_force() -> CommandRule {
    let mut rule = git_rule(
        "git-clean-force",
        "clean",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git clean -f permanently removes untracked files",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-f".to_string(), "--force".to_string()]),
        forbidden: Some(vec!["-n".to_string(), "--dry-run".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion = Some("Preview with \"git clean -n\" (dry-run) first".to_string());
    rule
}

fn git_push_force() -> CommandRule {
    let mut rule = git_rule(
        "git-push-force",
        "push",
        CommandAction::Block,
        CommandSeverity::Error,
        "git push --force rewrites remote history and can cause data loss for collaborators",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-f".to_string(), "--force".to_string()]),
        forbidden: Some(vec!["--force-with-lease".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion = Some(
        "Use \"git push --force-with-lease\" for safer force pushing, or coordinate with your team"
            .to_string(),
    );
    rule.references = Some(vec!["https://git-scm.com/docs/git-push#Documentation/git-push.txt---force-with-leaseltrefnamegt".to_string()]);
    rule
}

fn git_branch_force_delete() -> CommandRule {
    let mut rule = git_rule(
        "git-branch-force-delete",
        "branch",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git branch -D force-deletes branches without merge verification",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-D".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion = Some("Use \"git branch -d\" for safe deletion with merge checks".to_string());
    rule
}

fn git_stash_drop() -> CommandRule {
    let mut rule = git_rule(
        "git-stash-drop",
        "stash",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git stash drop permanently deletes stashed changes",
    );
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^drop$".to_string()),
        position: Some(0),
    });
    rule.suggestion =
        Some("Review stashed changes with \"git stash show -p\" before dropping".to_string());
    rule
}

fn git_stash_clear() -> CommandRule {
    let mut rule = git_rule(
        "git-stash-clear",
        "stash",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git stash clear permanently deletes all stashed changes",
    );
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^clear$".to_string()),
        position: Some(0),
    });
    rule.suggestion = Some("Review stashes with \"git stash list\" before clearing".to_string());
    rule
}

fn git_rebase_abort() -> CommandRule {
    let mut rule = git_rule(
        "git-rebase-abort",
        "rebase",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git rebase --abort discards rebase progress",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["--abort".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion =
        Some("Ensure you want to discard all rebase progress before aborting".to_string());
    rule
}

fn git_merge_abort() -> CommandRule {
    let mut rule = git_rule(
        "git-merge-abort",
        "merge",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "git merge --abort discards merge progress",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["--abort".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion =
        Some("Ensure you want to discard all merge progress before aborting".to_string());
    rule
}

fn git_checkout_branch() -> CommandRule {
    let mut rule = git_rule(
        "git-checkout-branch",
        "checkout",
        CommandAction::Allow,
        CommandSeverity::Info,
        "Branch creation is a safe operation",
    );
    rule.flags = Some(CommandFlagConfig {
        required: Some(vec![
            "-b".to_string(),
            "-B".to_string(),
            "--orphan".to_string(),
        ]),
        ..CommandFlagConfig::default()
    });
    rule
}

fn git_restore_staged() -> CommandRule {
    let mut rule = git_rule(
        "git-restore-staged",
        "restore",
        CommandAction::Allow,
        CommandSeverity::Info,
        "Unstaging changes is a safe operation",
    );
    rule.flags = Some(CommandFlagConfig {
        required: Some(vec!["--staged".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule
}

fn git_push_force_with_lease() -> CommandRule {
    let mut rule = git_rule(
        "git-push-force-with-lease",
        "push",
        CommandAction::Allow,
        CommandSeverity::Info,
        "Force-with-lease is a safer alternative to --force",
    );
    rule.flags = Some(CommandFlagConfig {
        required: Some(vec!["--force-with-lease".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule
}

fn git_branch_safe_delete() -> CommandRule {
    let mut rule = git_rule(
        "git-branch-safe-delete",
        "branch",
        CommandAction::Allow,
        CommandSeverity::Info,
        "Safe branch deletion with merge verification",
    );
    rule.flags = Some(CommandFlagConfig {
        required: Some(vec!["-d".to_string()]),
        forbidden: Some(vec!["-D".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule
}

fn git_clean_dry_run() -> CommandRule {
    let mut rule = git_rule(
        "git-clean-dry-run",
        "clean",
        CommandAction::Allow,
        CommandSeverity::Info,
        "Dry-run preview is safe",
    );
    rule.flags = Some(CommandFlagConfig {
        required: Some(vec!["-n".to_string(), "--dry-run".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule
}

#[must_use]
pub fn build_default_git_rules() -> Vec<CommandRule> {
    vec![
        git_reset_hard(),
        git_reset_merge(),
        git_checkout_discard(),
        git_checkout_all(),
        git_restore_worktree(),
        git_clean_force(),
        git_push_force(),
        git_branch_force_delete(),
        git_stash_drop(),
        git_stash_clear(),
        git_rebase_abort(),
        git_merge_abort(),
        git_checkout_branch(),
        git_restore_staged(),
        git_push_force_with_lease(),
        git_branch_safe_delete(),
        git_clean_dry_run(),
    ]
}

#[cfg(test)]
mod tests {
    use crate::command_safety::rules::git_rules::build_default_git_rules;

    #[test]
    fn includes_all_default_git_rules() {
        let rules = build_default_git_rules();
        assert_eq!(rules.len(), 17);
    }
}
