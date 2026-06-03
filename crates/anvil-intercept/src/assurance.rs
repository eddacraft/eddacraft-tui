//! DSV-003 Task 5 (ADR-061 §5): the default-deny invalidation taxonomy.
//!
//! This is the **taxonomy half** of the assurance module; the workspace
//! assurance *state machine* (Clean/Stale/Pending/Running/Unavailable
//! transitions + the `workspace_status` / `request_full_scan` verbs) lands
//! with the `validate_paths` orchestration in DSV-005.
//!
//! [`taxonomy_reason`] decides, for one classified change in its file-role
//! context, whether the change is *potentially certifiable* (returns `None`)
//! or carries a concrete [`StaleReason`]. The contract is **default-deny**:
//! `None` is returned for exactly one case — a plain content modify of an
//! ordinary file — and every other class, plus an explicitly unclassifiable
//! change, maps to a reason. An unknown change is stale, never clean.
//!
//! Note this taxonomy owns only the *change-classification* reasons. Four
//! `StaleReason` variants are raised elsewhere and are intentionally **not**
//! produced here: `ImpactSetOverflow` (the certify closure, DSV-004/Task 6),
//! `WarmStateEvicted` (the cache, DSV-004/Task 7), `ScanTimeout` (the scan
//! orchestration, DSV-005/-006), and `DaemonAbsent` (the client-side
//! fallback, DSV-007). The wire-level `StaleReason::Unknown` `#[serde(other)]`
//! fallback is a deserialisation affordance, not a taxonomy output.

use std::path::Path;

use anvil_intercept_proto::protocol::StaleReason;

use crate::change_class::CanonicalChange;

/// File-role context for a change, used to override the raw change class.
///
/// Editing (or deleting) certain files invalidates more than the file itself:
/// a `.gitignore` edit changes which paths are even in scope; a config /
/// policy / boundary edit changes the rule surface; a symlink retarget moves
/// the resolution target out from under the warm graph. These take precedence
/// over the underlying [`CanonicalChange`].
// Four independent boolean facts about one change; they are not a state enum
// (a change can be, e.g., both a symlink retarget and under `.anvil/`), so the
// flat-flags shape is the honest model.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ChangeCtx {
    /// The upstream classifier could not determine the change. Default-deny:
    /// this forces [`StaleReason::UnknownClass`].
    pub unclassifiable: bool,
    /// The change retargeted a symlink in the resolution path.
    pub symlink_retarget: bool,
    /// The changed path is a `.gitignore`.
    pub gitignore: bool,
    /// The changed path is an Anvil config / boundary / policy file.
    pub config_or_policy: bool,
}

impl ChangeCtx {
    /// Derive the role context for a root-relative path. `symlink_retarget` is
    /// supplied by the caller (it is a property of the change, observed at
    /// classification time, not of the path string).
    #[must_use]
    pub fn for_path(rel: &str, symlink_retarget: bool) -> Self {
        Self {
            unclassifiable: false,
            symlink_retarget,
            gitignore: is_gitignore(rel),
            config_or_policy: is_config_or_policy(rel),
        }
    }
}

/// `true` for a `.gitignore` at the root or any subdirectory.
#[must_use]
fn is_gitignore(rel: &str) -> bool {
    rel == ".gitignore" || rel.ends_with("/.gitignore")
}

/// `true` for an Anvil config / boundary / policy file: a `.anvil.<ext>` config
/// (`yaml`/`yml`/`json`/`toml`) or anything under a `.anvil/` directory.
///
/// The config-file set is kept **lock-step with the canonical recogniser**
/// `rule_cache::is_anvil_config_file` (and `anvil_config::discover`): stem
/// `.anvil`, extension in `{yaml, yml, json, toml}`, lowercase only. A false
/// negative here would wrongly certify a boundary change as clean, so the two
/// must not drift. The legacy `.anvilrc` is a separate MLP2-040 migration
/// concern, not in the discover precedence, and is intentionally excluded.
#[must_use]
fn is_config_or_policy(rel: &str) -> bool {
    let basename = rel.rsplit('/').next().unwrap_or(rel);
    let path = Path::new(basename);
    let is_anvil_config = path.file_stem().and_then(|s| s.to_str()) == Some(".anvil")
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| matches!(ext, "yaml" | "yml" | "json" | "toml"));
    is_anvil_config || rel == ".anvil" || rel.starts_with(".anvil/") || rel.contains("/.anvil/")
}

/// Map a classified change + its file-role context to an invalidation reason,
/// or `None` if the change is potentially certifiable.
///
/// Default-deny precedence: an unclassifiable change is stale first; then the
/// file-role overrides (symlink retarget → gitignore → config/policy); then
/// the raw change class. Only a plain content modify of an ordinary file is
/// `None` (potentially certifiable — the certify closure decides the rest).
#[must_use]
pub fn taxonomy_reason(change: &CanonicalChange, ctx: &ChangeCtx) -> Option<StaleReason> {
    if ctx.unclassifiable {
        return Some(StaleReason::UnknownClass);
    }
    if ctx.symlink_retarget {
        return Some(StaleReason::SymlinkRetarget);
    }
    if ctx.gitignore {
        return Some(StaleReason::GitignoreScopeChange);
    }
    if ctx.config_or_policy {
        return Some(StaleReason::ConfigBoundaryPolicyEdit);
    }
    match change {
        CanonicalChange::Delete => Some(StaleReason::Deleted),
        CanonicalChange::Rename { .. } => Some(StaleReason::Renamed),
        // A new file needs cross-file resolution before its impact is known;
        // conservatively stale until the certify closure runs.
        CanonicalChange::Create => Some(StaleReason::CrossFileResolutionNeeded),
        // The single potentially-certifiable case.
        CanonicalChange::ContentModify => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn plain() -> ChangeCtx {
        ChangeCtx::default()
    }

    #[test]
    fn plain_content_modify_is_certifiable() {
        assert_eq!(
            taxonomy_reason(&CanonicalChange::ContentModify, &plain()),
            None,
            "a plain content modify is the one potentially-certifiable case"
        );
    }

    #[test]
    fn delete_maps_to_deleted() {
        assert_eq!(
            taxonomy_reason(&CanonicalChange::Delete, &plain()),
            Some(StaleReason::Deleted)
        );
    }

    #[test]
    fn rename_maps_to_renamed() {
        assert_eq!(
            taxonomy_reason(
                &CanonicalChange::Rename {
                    from: PathBuf::from("src/old.rs")
                },
                &plain()
            ),
            Some(StaleReason::Renamed)
        );
    }

    #[test]
    fn create_maps_to_cross_file_resolution_needed() {
        assert_eq!(
            taxonomy_reason(&CanonicalChange::Create, &plain()),
            Some(StaleReason::CrossFileResolutionNeeded)
        );
    }

    #[test]
    fn symlink_retarget_overrides_even_a_content_modify() {
        let ctx = ChangeCtx {
            symlink_retarget: true,
            ..ChangeCtx::default()
        };
        assert_eq!(
            taxonomy_reason(&CanonicalChange::ContentModify, &ctx),
            Some(StaleReason::SymlinkRetarget)
        );
    }

    #[test]
    fn gitignore_edit_maps_to_gitignore_scope_change() {
        let ctx = ChangeCtx::for_path(".gitignore", false);
        assert!(ctx.gitignore);
        assert_eq!(
            taxonomy_reason(&CanonicalChange::ContentModify, &ctx),
            Some(StaleReason::GitignoreScopeChange)
        );
        // Nested .gitignore too.
        assert!(ChangeCtx::for_path("crates/foo/.gitignore", false).gitignore);
    }

    #[test]
    fn config_or_policy_edit_maps_to_config_boundary_policy_edit() {
        // All four `.anvil.<ext>` forms the canonical recogniser accepts,
        // nested configs, and the `.anvil/` directory — lock-step with
        // rule_cache::is_anvil_config_file.
        for path in [
            ".anvil.yaml",
            ".anvil.yml",
            ".anvil.json",
            ".anvil.toml",
            "crates/foo/.anvil.toml",
            ".anvil/policy.toml",
        ] {
            let ctx = ChangeCtx::for_path(path, false);
            assert!(
                ctx.config_or_policy,
                "{path} should be a config/policy file"
            );
            assert_eq!(
                taxonomy_reason(&CanonicalChange::ContentModify, &ctx),
                Some(StaleReason::ConfigBoundaryPolicyEdit),
                "{path}"
            );
        }
        // Not config/policy: an ordinary source file, the dotless `anvil.toml`
        // (not a discover-precedence name), and the legacy `.anvilrc`.
        for path in ["src/lib.rs", "anvil.toml", ".anvilrc"] {
            assert!(
                !ChangeCtx::for_path(path, false).config_or_policy,
                "{path} must NOT be treated as a config/policy file"
            );
        }
    }

    #[test]
    fn file_role_takes_precedence_over_rename() {
        // Renaming `src/.ignore` → `src/.gitignore` is a scope change, not a
        // plain Renamed; same for renaming into a config path.
        let gitignore = ChangeCtx::for_path("src/.gitignore", false);
        assert_eq!(
            taxonomy_reason(
                &CanonicalChange::Rename {
                    from: PathBuf::from("src/.ignore")
                },
                &gitignore
            ),
            Some(StaleReason::GitignoreScopeChange)
        );
        let config = ChangeCtx::for_path(".anvil.toml", false);
        assert_eq!(
            taxonomy_reason(
                &CanonicalChange::Rename {
                    from: PathBuf::from(".anvil.toml.bak")
                },
                &config
            ),
            Some(StaleReason::ConfigBoundaryPolicyEdit)
        );
    }

    #[test]
    fn unknown_class_defaults_to_stale_not_clean() {
        let ctx = ChangeCtx {
            unclassifiable: true,
            ..ChangeCtx::default()
        };
        // Even for a content modify (the otherwise-certifiable case), an
        // unclassifiable change fails closed to stale.
        let reason = taxonomy_reason(&CanonicalChange::ContentModify, &ctx);
        assert_eq!(reason, Some(StaleReason::UnknownClass));
        assert!(reason.is_some(), "unknown class must be stale, never clean");
    }

    #[test]
    fn file_role_takes_precedence_over_delete() {
        // Deleting a .gitignore is a scope change, not a plain Deleted.
        let ctx = ChangeCtx::for_path(".gitignore", false);
        assert_eq!(
            taxonomy_reason(&CanonicalChange::Delete, &ctx),
            Some(StaleReason::GitignoreScopeChange)
        );
    }
}
