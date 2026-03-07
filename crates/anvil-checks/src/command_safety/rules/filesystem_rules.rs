use crate::command_safety::types::{
    CommandAction, CommandArgConfig, CommandCategory, CommandConditions, CommandFlagConfig,
    CommandRule, CommandSeverity,
};

fn fs_rule(
    id: &str,
    command: &str,
    action: CommandAction,
    severity: CommandSeverity,
    reason: &str,
) -> CommandRule {
    CommandRule {
        id: id.to_string(),
        category: CommandCategory::Filesystem,
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

fn rm_rf_root() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-root",
        "rm",
        CommandAction::Block,
        CommandSeverity::Error,
        "rm -rf / would delete the entire filesystem",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec![
            "-r".to_string(),
            "-f".to_string(),
            "--recursive".to_string(),
            "--force".to_string(),
        ]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^/$".to_string()),
        position: None,
    });
    rule.suggestion =
        Some("NEVER delete the root filesystem. Review the target path carefully.".to_string());
    rule
}

fn rm_rf_home() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-home",
        "rm",
        CommandAction::Block,
        CommandSeverity::Error,
        "rm -rf on home directory is extremely dangerous",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^(~|~/.*|\$HOME|\$\{HOME\}|\$HOME/.*)$".to_string()),
        position: None,
    });
    rule.suggestion = Some(
        "NEVER delete your home directory. Specify exact subdirectories if needed.".to_string(),
    );
    rule
}

fn rm_rf_current_dir() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-current-dir",
        "rm",
        CommandAction::Block,
        CommandSeverity::Error,
        "rm -rf . deletes the entire current directory",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^\.$".to_string()),
        position: None,
    });
    rule.suggestion = Some("Specify exact subdirectories or files instead".to_string());
    rule
}

fn rm_rf_parent_traversal() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-parent-traversal",
        "rm",
        CommandAction::Block,
        CommandSeverity::Error,
        "rm -rf with parent directory traversal (..) can escape current directory",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"\.\.".to_string()),
        position: None,
    });
    rule.suggestion =
        Some("Use absolute paths or paths within the current directory only".to_string());
    rule
}

fn rm_rf_system_dirs() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-system-dirs",
        "rm",
        CommandAction::Block,
        CommandSeverity::Error,
        "rm -rf on system directories would break the operating system",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(
            r"^/(bin|boot|dev|etc|lib|lib64|opt|proc|root|sbin|sys|usr|var)$".to_string(),
        ),
        position: None,
    });
    rule.suggestion = Some("NEVER delete system directories.".to_string());
    rule
}

fn rm_rf_tmp_dir() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-tmp-dir",
        "rm",
        CommandAction::Allow,
        CommandSeverity::Info,
        "Temporary directory deletion is safe",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^(/tmp|/var/tmp|\$TMPDIR|\$\{TMPDIR\})(/.*)?$".to_string()),
        position: None,
    });
    rule
}

fn rm_rf_build_dirs() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-build-dirs",
        "rm",
        CommandAction::Allow,
        CommandSeverity::Info,
        "Common build/cache directory deletion is safe (reproducible artefacts)",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-r".to_string(), "-f".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^(\./)?(\.)?(node_modules|dist|build|target|\.next|\.cache|\.nuxt|\.output|coverage|__pycache__|\.pytest_cache|\.mypy_cache|\.tox|\.venv|venv)$".to_string()),
        position: None,
    });
    rule
}

fn rm_rf_with_recursive() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-with-recursive",
        "rm",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "rm -r recursively deletes directories - verify the target path",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-r".to_string(), "--recursive".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion =
        Some("List files first with \"ls -la\", review carefully before deleting".to_string());
    rule.conditions = Some(CommandConditions {
        strict_mode_only: Some(true),
        working_directory: None,
    });
    rule
}

fn rmdir_force() -> CommandRule {
    let mut rule = fs_rule(
        "rmdir-force",
        "rmdir",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "rmdir -p removes parent directories",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-p".to_string(), "--parents".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.suggestion = Some("Verify the full directory path before removing".to_string());
    rule
}

fn mv_overwrite() -> CommandRule {
    let mut rule = fs_rule(
        "mv-overwrite",
        "mv",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "mv -f to root/home paths can overwrite important files",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-f".to_string(), "--force".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^(/|~|\$HOME)".to_string()),
        position: None,
    });
    rule.suggestion = Some("Use mv without -f to get overwrite prompts".to_string());
    rule
}

fn chmod_recursive_777() -> CommandRule {
    let mut rule = fs_rule(
        "chmod-recursive-777",
        "chmod",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "chmod -R 777 makes all files world-writable (security risk)",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-R".to_string(), "--recursive".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^777$".to_string()),
        position: None,
    });
    rule.suggestion = Some(
        "Use more restrictive permissions like 755 for directories, 644 for files".to_string(),
    );
    rule
}

fn chown_recursive_root() -> CommandRule {
    let mut rule = fs_rule(
        "chown-recursive-root",
        "chown",
        CommandAction::Warn,
        CommandSeverity::Warning,
        "chown -R root can cause permission issues",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec!["-R".to_string(), "--recursive".to_string()]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^root".to_string()),
        position: None,
    });
    rule.suggestion = Some("Verify you want to change ownership recursively to root".to_string());
    rule
}

fn rm_rf_root_glob() -> CommandRule {
    let mut rule = fs_rule(
        "rm-rf-root-glob",
        "rm",
        CommandAction::Block,
        CommandSeverity::Error,
        "rm -rf /* would delete the entire filesystem contents",
    );
    rule.flags = Some(CommandFlagConfig {
        dangerous: Some(vec![
            "-r".to_string(),
            "-f".to_string(),
            "--recursive".to_string(),
            "--force".to_string(),
        ]),
        ..CommandFlagConfig::default()
    });
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^/\*$".to_string()),
        position: None,
    });
    rule.suggestion =
        Some("NEVER delete filesystem root. Review the target path carefully.".to_string());
    rule
}

fn chmod_777_sensitive() -> CommandRule {
    let mut rule = fs_rule(
        "chmod-777-sensitive",
        "chmod",
        CommandAction::Block,
        CommandSeverity::Error,
        "chmod on system paths can compromise system security",
    );
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"^/(etc|root|var|boot|lib|bin|sbin|usr)".to_string()),
        position: None,
    });
    rule.suggestion = Some(
        "System files should not be modified. Use proper configuration management.".to_string(),
    );
    rule
}

fn dd_block_device() -> CommandRule {
    let mut rule = fs_rule(
        "dd-block-device",
        "dd",
        CommandAction::Block,
        CommandSeverity::Error,
        "dd writing to block devices can destroy entire disks",
    );
    rule.args = Some(CommandArgConfig {
        pattern: Some(r"of=/dev/(sd[a-z]|hd[a-z]|nvme\d+n\d+|vd[a-z]|xvd[a-z])".to_string()),
        position: None,
    });
    rule.suggestion =
        Some("Verify the target device carefully. This will overwrite all data.".to_string());
    rule.references = Some(vec!["https://wiki.archlinux.org/title/Dd".to_string()]);
    rule
}

fn mkfs_any() -> CommandRule {
    let mut rule = fs_rule(
        "mkfs-any",
        "mkfs",
        CommandAction::Block,
        CommandSeverity::Error,
        "mkfs formats and destroys all data on the target device",
    );
    rule.suggestion =
        Some("Ensure the target device is correct. All existing data will be lost.".to_string());
    rule
}

fn mkfs_ext4() -> CommandRule {
    let mut rule = fs_rule(
        "mkfs-ext4",
        "mkfs.ext4",
        CommandAction::Block,
        CommandSeverity::Error,
        "mkfs.ext4 formats and destroys all data on the target device",
    );
    rule.suggestion =
        Some("Ensure the target device is correct. All existing data will be lost.".to_string());
    rule
}

fn mkfs_xfs() -> CommandRule {
    let mut rule = fs_rule(
        "mkfs-xfs",
        "mkfs.xfs",
        CommandAction::Block,
        CommandSeverity::Error,
        "mkfs.xfs formats and destroys all data on the target device",
    );
    rule.suggestion =
        Some("Ensure the target device is correct. All existing data will be lost.".to_string());
    rule
}

fn mkfs_btrfs() -> CommandRule {
    let mut rule = fs_rule(
        "mkfs-btrfs",
        "mkfs.btrfs",
        CommandAction::Block,
        CommandSeverity::Error,
        "mkfs.btrfs formats and destroys all data on the target device",
    );
    rule.suggestion =
        Some("Ensure the target device is correct. All existing data will be lost.".to_string());
    rule
}

#[must_use]
pub fn build_default_filesystem_rules() -> Vec<CommandRule> {
    vec![
        rm_rf_root(),
        rm_rf_home(),
        rm_rf_current_dir(),
        rm_rf_parent_traversal(),
        rm_rf_system_dirs(),
        rm_rf_tmp_dir(),
        rm_rf_build_dirs(),
        rm_rf_with_recursive(),
        rmdir_force(),
        mv_overwrite(),
        chmod_recursive_777(),
        chown_recursive_root(),
        rm_rf_root_glob(),
        chmod_777_sensitive(),
        dd_block_device(),
        mkfs_any(),
        mkfs_ext4(),
        mkfs_xfs(),
        mkfs_btrfs(),
    ]
}

#[cfg(test)]
mod tests {
    use crate::command_safety::rules::filesystem_rules::build_default_filesystem_rules;

    #[test]
    fn includes_all_default_filesystem_rules() {
        let rules = build_default_filesystem_rules();
        assert_eq!(rules.len(), 19);
    }
}
