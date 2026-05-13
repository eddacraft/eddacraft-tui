use std::path::Path;

/// Hook framework detected in a worktree.
///
/// Detection is non-destructive and read-only: we only look at
/// known marker files. The ordering of variants reflects detection
/// precedence per ADR-038 §D-4 — the first match wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookFramework {
    /// `.husky/` directory present.
    Husky,
    /// `lefthook.yml` or `lefthook.toml` present at repo root.
    Lefthook,
    /// `.pre-commit-config.yaml` present at repo root.
    PreCommitFramework,
    /// `.cargo-husky/hooks/` directory present.
    CargoHusky,
    /// `core.hooksPath` set to a directory other than `.git/hooks`
    /// (detected by the consumer; this variant exists for
    /// completeness — the library doesn't shell out to git).
    /// Detection of this variant is a CLI concern; the library
    /// reports `Plain` when only marker files are inspected.
    CoreHooksPath,
    /// No framework detected; Anvil installs at `.git/hooks/`.
    Plain,
}

impl HookFramework {
    /// Short identifier used in install-path strings and error
    /// messages. Stable; consumers should not change it.
    pub fn id(self) -> &'static str {
        match self {
            HookFramework::Husky => "husky",
            HookFramework::Lefthook => "lefthook",
            HookFramework::PreCommitFramework => "pre-commit-framework",
            HookFramework::CargoHusky => "cargo-husky",
            HookFramework::CoreHooksPath => "core.hooksPath",
            HookFramework::Plain => "plain",
        }
    }
}

/// Detect the hook framework rooted at `repo_root`.
///
/// Precedence per ADR-038 §D-4: Husky > Lefthook > pre-commit
/// framework > cargo-husky > Plain. (`core.hooksPath` is checked by
/// the CLI consumer because it requires invoking git; the library
/// stays I/O-free beyond marker-file existence checks.)
///
/// Returns [`HookFramework::Plain`] when nothing matches.
pub fn detect_framework(repo_root: &Path) -> HookFramework {
    if repo_root.join(".husky").is_dir() {
        return HookFramework::Husky;
    }
    if repo_root.join("lefthook.yml").is_file()
        || repo_root.join("lefthook.toml").is_file()
        || repo_root.join("lefthook.yaml").is_file()
    {
        return HookFramework::Lefthook;
    }
    if repo_root.join(".pre-commit-config.yaml").is_file()
        || repo_root.join(".pre-commit-config.yml").is_file()
    {
        return HookFramework::PreCommitFramework;
    }
    if repo_root.join(".cargo-husky").join("hooks").is_dir() {
        return HookFramework::CargoHusky;
    }
    HookFramework::Plain
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detect_plain_when_no_marker_present() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Plain);
    }

    #[test]
    fn detect_husky_when_dot_husky_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".husky")).unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Husky);
    }

    #[test]
    fn detect_lefthook_yml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("lefthook.yml"), "").unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Lefthook);
    }

    #[test]
    fn detect_lefthook_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("lefthook.yaml"), "").unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Lefthook);
    }

    #[test]
    fn detect_lefthook_toml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("lefthook.toml"), "").unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Lefthook);
    }

    #[test]
    fn detect_pre_commit_framework_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".pre-commit-config.yaml"), "").unwrap();
        assert_eq!(
            detect_framework(tmp.path()),
            HookFramework::PreCommitFramework
        );
    }

    #[test]
    fn detect_pre_commit_framework_yml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".pre-commit-config.yml"), "").unwrap();
        assert_eq!(
            detect_framework(tmp.path()),
            HookFramework::PreCommitFramework
        );
    }

    #[test]
    fn detect_cargo_husky() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".cargo-husky").join("hooks")).unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::CargoHusky);
    }

    #[test]
    fn husky_wins_over_lefthook_per_adr_038_precedence() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".husky")).unwrap();
        fs::write(tmp.path().join("lefthook.yml"), "").unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Husky);
    }

    #[test]
    fn lefthook_wins_over_pre_commit_framework() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("lefthook.yml"), "").unwrap();
        fs::write(tmp.path().join(".pre-commit-config.yaml"), "").unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Lefthook);
    }

    #[test]
    fn pre_commit_framework_wins_over_cargo_husky() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".pre-commit-config.yaml"), "").unwrap();
        fs::create_dir_all(tmp.path().join(".cargo-husky").join("hooks")).unwrap();
        assert_eq!(
            detect_framework(tmp.path()),
            HookFramework::PreCommitFramework
        );
    }

    #[test]
    fn framework_ids_are_stable() {
        // The id strings appear in user-facing messages and install
        // paths. Don't change them without a release note.
        assert_eq!(HookFramework::Husky.id(), "husky");
        assert_eq!(HookFramework::Lefthook.id(), "lefthook");
        assert_eq!(
            HookFramework::PreCommitFramework.id(),
            "pre-commit-framework"
        );
        assert_eq!(HookFramework::CargoHusky.id(), "cargo-husky");
        assert_eq!(HookFramework::CoreHooksPath.id(), "core.hooksPath");
        assert_eq!(HookFramework::Plain.id(), "plain");
    }

    #[test]
    fn empty_husky_dir_still_counts_as_husky() {
        // An empty `.husky/` is a real Husky setup that hasn't had a
        // hook installed yet — we still want to integrate with it,
        // not install at .git/hooks.
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".husky")).unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Husky);
    }

    #[test]
    fn marker_file_must_be_a_file_not_directory() {
        let tmp = tempfile::tempdir().unwrap();
        // A directory named lefthook.yml should NOT count.
        fs::create_dir(tmp.path().join("lefthook.yml")).unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Plain);
    }

    #[test]
    fn cargo_husky_hooks_must_be_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        // A file named .cargo-husky/hooks should NOT count.
        fs::create_dir(tmp.path().join(".cargo-husky")).unwrap();
        fs::write(tmp.path().join(".cargo-husky").join("hooks"), "").unwrap();
        assert_eq!(detect_framework(tmp.path()), HookFramework::Plain);
    }
}
