//! UJ-010: post-upgrade what's-new one-liner.
//!
//! The first plain `anvil status` run after a version change prints one line
//! announcing the upgrade with a changelog pointer — exactly once per
//! version, then never again. Complements DISTRIB-002's update-available
//! advisory, which covers the opposite direction (an upgrade you have not
//! taken yet). A fresh install seeds the marker silently: an install is not
//! an upgrade.
//!
//! Once-ness is persisted in a project-local marker under `.anvil/cache/`
//! (the same neighbourhood as the detected-agents cache). The marker is
//! written only when that cache directory already exists — pure status on a
//! never-activated tree leaves the project clean (CIB-264). When project
//! writes are gated (DISTRIB-006 candidate installs) the marker cannot be
//! persisted, so no hint is printed — printing without persisting would
//! repeat on every run and break the exactly-once contract. The same
//! silence applies when the cache directory is absent: no write, no hint.

use std::path::Path;

/// Marker file recording the last version this project saw, relative to the
/// workspace root.
const MARKER_REL: &str = ".anvil/cache/last-seen-version";

/// Opt-out env var, mirroring `ANVIL_DISABLE_UPDATE_HINT`'s convention.
const SUPPRESS_ENV: &str = "ANVIL_DISABLE_WHATS_NEW";

/// Pure decision: previous marker content × current version → hint line.
/// `None` previous means a fresh install (or pre-marker upgrade baseline):
/// seed silently rather than claiming an upgrade we cannot prove.
fn decide(previous: Option<&str>, current: &str) -> Option<String> {
    match previous {
        None => None,
        Some(prev) if prev.trim() == current => None,
        // ASCII-only, matching the one-line hint convention (Windows cp1252
        // consoles and CI log captures).
        Some(_) => Some(format!(
            "anvil upgraded to v{current} -- what's new: https://docs.eddacraft.ai/anvil/releases/changelog"
        )),
    }
}

/// Compute the post-upgrade hint for this run and persist the marker.
/// Returns `Some(line)` exactly once per version change; seeds silently on
/// the first marked run when project cache already exists; honours the
/// opt-out env var and gated project writes; does nothing on never-activated
/// trees (no `.anvil/cache/` yet).
pub fn post_upgrade_hint(root: &Path, current: &str) -> Option<String> {
    let suppressed = std::env::var_os(SUPPRESS_ENV).is_some();
    let writes_gated = crate::install_root::project_writes_gated();
    post_upgrade_hint_inner(root, current, suppressed, writes_gated)
}

/// Testable core of [`post_upgrade_hint`] with the environment folded into
/// parameters.
fn post_upgrade_hint_inner(
    root: &Path,
    current: &str,
    suppressed: bool,
    writes_gated: bool,
) -> Option<String> {
    if suppressed || writes_gated {
        return None;
    }
    let marker = root.join(MARKER_REL);
    let parent = marker.parent()?;
    // Never create project directories as a side effect of status (CIB-264).
    // Without a place to persist the marker, do not emit a hint either —
    // same exactly-once contract as gated writes.
    if !parent.is_dir() {
        return None;
    }
    let previous = std::fs::read_to_string(&marker).ok();
    let hint = decide(previous.as_deref(), current);
    // Persist on seed and on change when the cache dir already exists.
    // Best-effort: an unwritable marker means the hint may repeat, which is
    // preferable to suppressing it forever.
    if previous.as_deref().map(str::trim) != Some(current) {
        let _ = std::fs::write(&marker, current);
    }
    hint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_install_seeds_silently() {
        assert_eq!(decide(None, "0.8.0-beta"), None);
    }

    #[test]
    fn same_version_is_silent() {
        assert_eq!(decide(Some("0.8.0-beta"), "0.8.0-beta"), None);
        // Marker content may carry a trailing newline from manual edits.
        assert_eq!(decide(Some("0.8.0-beta\n"), "0.8.0-beta"), None);
    }

    #[test]
    fn version_change_announces_with_changelog_pointer() {
        let line = decide(Some("0.7.4-beta"), "0.8.0-beta").expect("upgrade announces");
        assert!(line.contains("0.8.0-beta"), "names the new version: {line}");
        assert!(
            line.contains("changelog"),
            "carries a changelog pointer: {line}"
        );
        assert!(
            line.is_ascii(),
            "one-line hints are ASCII-only for cp1252 consoles: {line}",
        );
    }

    /// Ensure `.anvil/cache` exists so marker writes are allowed (CIB-264).
    fn ensure_cache_dir(root: &Path) {
        std::fs::create_dir_all(root.join(".anvil/cache")).unwrap();
    }

    #[test]
    fn never_activated_tree_stays_clean() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(
            post_upgrade_hint_inner(dir.path(), "0.8.0-beta", false, false),
            None,
            "no cache dir: no hint",
        );
        assert!(
            !dir.path().join(".anvil").exists(),
            "pure status must not create .anvil/ on a never-activated tree",
        );
    }

    #[test]
    fn existing_cache_seeds_silently_without_hint() {
        let dir = tempfile::TempDir::new().unwrap();
        ensure_cache_dir(dir.path());
        assert_eq!(
            post_upgrade_hint_inner(dir.path(), "0.7.4-beta", false, false),
            None,
            "first marked run is a seed, not an upgrade",
        );
        let marker = dir.path().join(MARKER_REL);
        assert!(
            marker.exists(),
            "seed writes the marker when cache already exists",
        );
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap().trim(),
            "0.7.4-beta",
        );
    }

    #[test]
    fn hint_fires_exactly_once_per_version() {
        let dir = tempfile::TempDir::new().unwrap();
        ensure_cache_dir(dir.path());
        // First marked run seeds silently.
        assert_eq!(
            post_upgrade_hint_inner(dir.path(), "0.7.4-beta", false, false),
            None,
            "first marked run is a seed, not an upgrade",
        );
        // Version change: announce once…
        let hint = post_upgrade_hint_inner(dir.path(), "0.8.0-beta", false, false);
        assert!(hint.is_some(), "first run after a version change announces");
        // …then never again for that version.
        assert_eq!(
            post_upgrade_hint_inner(dir.path(), "0.8.0-beta", false, false),
            None,
            "the hint must not repeat for the same version",
        );
    }

    #[test]
    fn same_version_marker_stays_silent() {
        let dir = tempfile::TempDir::new().unwrap();
        ensure_cache_dir(dir.path());
        let marker = dir.path().join(MARKER_REL);
        std::fs::write(&marker, "0.8.0-beta").unwrap();
        assert_eq!(
            post_upgrade_hint_inner(dir.path(), "0.8.0-beta", false, false),
            None,
        );
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap().trim(),
            "0.8.0-beta",
            "same version does not rewrite the marker",
        );
    }

    #[test]
    fn suppression_and_gated_writes_stay_silent_and_do_not_mark() {
        let dir = tempfile::TempDir::new().unwrap();
        ensure_cache_dir(dir.path());
        assert_eq!(
            post_upgrade_hint_inner(dir.path(), "0.8.0-beta", true, false),
            None,
        );
        assert_eq!(
            post_upgrade_hint_inner(dir.path(), "0.8.0-beta", false, true),
            None,
        );
        assert!(
            !dir.path().join(MARKER_REL).exists(),
            "suppressed/gated runs must not write project state",
        );
    }
}
