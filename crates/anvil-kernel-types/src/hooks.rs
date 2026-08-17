//! Shared identifiers for Anvil-managed Git hooks.
//!
//! Anvil installs Git 2.54 native config-mode hooks via `git config --add
//! hook.<event>.command "ANVIL_HOOK=1 anvil ..."`. Multiple surfaces
//! (CLI install/uninstall, CLI status/doctor, TUI onboarding) need to agree on
//! which `hook.<event>.command` entries belong to Anvil. Centralising the
//! pattern + predicate here avoids three sites drifting out of sync.
//!
//! The constant `ANVIL_CONFIG_HOOK_PATTERN` is the regex Anvil passes to
//! `git config --unset-all <key> <value-pattern>` so `uninstall --config`
//! removes only Anvil-owned entries and leaves user-authored commands intact.
//! [`is_anvil_managed_command`] mirrors the same closed command set for
//! in-process ownership checks.

/// Regex passed to `git config --unset-all` to remove only Anvil-managed
/// `hook.<event>.command` entries. It recognises the legacy quality-gate
/// commands and the dedicated L3/L4 runtime commands used by current installs.
pub const ANVIL_CONFIG_HOOK_PATTERN: &str = "^ANVIL_HOOK=1 anvil (gate([[:space:]]|$)|hook (pre-commit|post-commit|pre-push)([[:space:]]|$))";

const ANVIL_CONFIG_HOOK_COMMANDS: &[&str] = &[
    "ANVIL_HOOK=1 anvil gate",
    "ANVIL_HOOK=1 anvil hook pre-commit",
    "ANVIL_HOOK=1 anvil hook post-commit",
    "ANVIL_HOOK=1 anvil hook pre-push",
];

/// True when `cmd` is a `hook.<event>.command` entry that Anvil owns.
///
/// Used by every surface that needs to distinguish user-authored hook
/// commands from ones Anvil installed:
///
/// - `anvil hooks install --config` (skip when already managed)
/// - `anvil hooks uninstall --config` (count managed entries before removing)
/// - `anvil hooks status` (label entries as Anvil-managed vs user-authored)
/// - `anvil status` (report config-mode hooks alongside file-mode hooks)
/// - `anvil doctor` (recognise config-mode entries as a valid hook source)
/// - The TUI onboarding hook detector (treat config-mode as a peer to Husky)
///
/// User-authored entries that happen to set `ANVIL_HOOK=1` but do not invoke
/// one of the closed managed commands are intentionally not claimed.
#[must_use]
pub fn is_anvil_managed_command(cmd: &str) -> bool {
    ANVIL_CONFIG_HOOK_COMMANDS.iter().any(|prefix| {
        cmd.strip_prefix(prefix).is_some_and(|suffix| {
            suffix.is_empty() || suffix.chars().next().is_some_and(char::is_whitespace)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_is_anchored_and_names_the_closed_command_set() {
        assert!(ANVIL_CONFIG_HOOK_PATTERN.starts_with('^'));
        assert!(ANVIL_CONFIG_HOOK_PATTERN.contains("gate"));
        assert!(ANVIL_CONFIG_HOOK_PATTERN.contains("pre-commit"));
        assert!(ANVIL_CONFIG_HOOK_PATTERN.contains("post-commit"));
        assert!(ANVIL_CONFIG_HOOK_PATTERN.contains("pre-push"));
    }

    #[test]
    fn recognises_canonical_install_commands() {
        // Current install commands plus the legacy pre-push gate command.
        assert!(is_anvil_managed_command(
            "ANVIL_HOOK=1 anvil gate --progress"
        ));
        assert!(is_anvil_managed_command("ANVIL_HOOK=1 anvil gate"));
        assert!(is_anvil_managed_command(
            "ANVIL_HOOK=1 anvil hook pre-commit"
        ));
        assert!(is_anvil_managed_command(
            "ANVIL_HOOK=1 anvil hook post-commit"
        ));
        assert!(is_anvil_managed_command("ANVIL_HOOK=1 anvil hook pre-push"));
    }

    #[test]
    fn rejects_user_authored_commands_with_anvil_hook_var() {
        // Setting ANVIL_HOOK=1 alone is not enough — the command must also
        // call one of Anvil's closed managed-hook commands.
        assert!(!is_anvil_managed_command("ANVIL_HOOK=1 npm run my-gate"));
        assert!(!is_anvil_managed_command("npm run lint-staged"));
        assert!(!is_anvil_managed_command(""));
        assert!(!is_anvil_managed_command("ANVIL_HOOK=1 anvil gatekeeper"));
        assert!(!is_anvil_managed_command(
            "ANVIL_HOOK=1 anvil hook pre-push-extra"
        ));
    }

    #[test]
    fn rejects_commands_that_only_contain_the_prefix() {
        // The marker is anchored at start — a user command that mentions
        // "ANVIL_HOOK=1 anvil gate" mid-string is not Anvil-managed.
        assert!(!is_anvil_managed_command(
            "echo running ANVIL_HOOK=1 anvil gate"
        ));
    }
}
