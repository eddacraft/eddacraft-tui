pub mod filesystem_rules;
pub mod git_rules;
pub mod shell_rules;

use std::sync::LazyLock;

use crate::command_safety::types::CommandRule;

static DEFAULT_GIT_RULES: LazyLock<Vec<CommandRule>> =
    LazyLock::new(git_rules::build_default_git_rules);
static DEFAULT_FILESYSTEM_RULES: LazyLock<Vec<CommandRule>> =
    LazyLock::new(filesystem_rules::build_default_filesystem_rules);
static DEFAULT_SHELL_RULES: LazyLock<Vec<CommandRule>> =
    LazyLock::new(shell_rules::build_default_shell_rules);

#[must_use]
pub fn default_git_rules() -> Vec<CommandRule> {
    DEFAULT_GIT_RULES.clone()
}

#[must_use]
pub fn default_filesystem_rules() -> Vec<CommandRule> {
    DEFAULT_FILESYSTEM_RULES.clone()
}

#[must_use]
pub fn default_shell_rules() -> Vec<CommandRule> {
    DEFAULT_SHELL_RULES.clone()
}
