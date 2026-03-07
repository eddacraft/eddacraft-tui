pub mod filesystem_rules;
pub mod git_rules;

use std::sync::LazyLock;

use crate::command_safety::types::CommandRule;

static DEFAULT_GIT_RULES: LazyLock<Vec<CommandRule>> =
    LazyLock::new(git_rules::build_default_git_rules);
static DEFAULT_FILESYSTEM_RULES: LazyLock<Vec<CommandRule>> =
    LazyLock::new(filesystem_rules::build_default_filesystem_rules);

#[must_use]
pub fn default_git_rules() -> Vec<CommandRule> {
    DEFAULT_GIT_RULES.clone()
}

#[must_use]
pub fn default_filesystem_rules() -> Vec<CommandRule> {
    DEFAULT_FILESYSTEM_RULES.clone()
}
