//! Shared identifiers for Anvil-managed Git hooks.
//!
//! Anvil installs Git 2.54 native config-mode hooks via `git config --add
//! hook.<event>.command "ANVIL_HOOK=1 anvil gate ..."`. Multiple surfaces
//! (CLI install/uninstall, CLI status/doctor, TUI onboarding) need to agree on
//! which `hook.<event>.command` entries belong to Anvil. Centralising the
//! pattern + predicate here avoids three sites drifting out of sync.
//!
//! The constant `ANVIL_CONFIG_HOOK_PATTERN` is the regex Anvil passes to
//! `git config --unset-all <key> <value-pattern>` so `uninstall --config`
//! removes only Anvil-owned entries and leaves user-authored commands intact.
//! Because the pattern is anchored at the start (`^`) with no other regex
//! metacharacters, [`is_anvil_managed_command`] uses [`str::starts_with`] for
//! the in-process predicate — a no-dependency match equivalent to the regex.

/// Regex passed to `git config --unset-all` to remove only Anvil-managed
/// `hook.<event>.command` entries. The leading `^ANVIL_HOOK=1 anvil gate`
/// segment doubles as the ownership marker matched by
/// [`is_anvil_managed_command`].
pub const ANVIL_CONFIG_HOOK_PATTERN: &str = "^ANVIL_HOOK=1 anvil gate";

/// Prefix that every Anvil-managed config-mode hook command starts with.
/// Equivalent to [`ANVIL_CONFIG_HOOK_PATTERN`] with the leading `^` stripped
/// — used for [`str::starts_with`] checks that do not need a regex engine.
const ANVIL_CONFIG_HOOK_PREFIX: &str = "ANVIL_HOOK=1 anvil gate";

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
/// `anvil gate` are intentionally not claimed.
#[must_use]
pub fn is_anvil_managed_command(cmd: &str) -> bool {
    cmd.starts_with(ANVIL_CONFIG_HOOK_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_is_anchored_at_start() {
        // The regex form must match the prefix form so the in-process
        // predicate stays equivalent to what `git config --unset-all`
        // does on disk. If anyone edits one without the other, this trips.
        assert!(ANVIL_CONFIG_HOOK_PATTERN.starts_with('^'));
        assert_eq!(
            &ANVIL_CONFIG_HOOK_PATTERN[1..],
            ANVIL_CONFIG_HOOK_PREFIX,
            "regex pattern minus the `^` anchor must equal the starts_with prefix",
        );
    }

    #[test]
    fn recognises_canonical_install_commands() {
        // The two strings the CLI installs today.
        assert!(is_anvil_managed_command(
            "ANVIL_HOOK=1 anvil gate --progress"
        ));
        assert!(is_anvil_managed_command("ANVIL_HOOK=1 anvil gate"));
    }

    #[test]
    fn rejects_user_authored_commands_with_anvil_hook_var() {
        // Setting ANVIL_HOOK=1 alone is not enough — the command must also
        // call `anvil gate` for Anvil to claim it.
        assert!(!is_anvil_managed_command("ANVIL_HOOK=1 npm run my-gate"));
        assert!(!is_anvil_managed_command("npm run lint-staged"));
        assert!(!is_anvil_managed_command(""));
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
