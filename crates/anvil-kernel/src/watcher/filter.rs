use std::path::{Path, PathBuf};

/// Canonical local-noise directory denylist (ADOPT-004 / WATCHUX-002).
///
/// Single source of truth for every Anvil walking surface — watch
/// (this module), and the cli command surfaces audit / baseline /
/// check / drift / gate (consumed via `anvil-cli`'s
/// `crate::util::is_ignored_dir_name` re-export of
/// [`is_ignored_dir_name`]). Add new entries here, not at the call
/// site; downstream surfaces inherit automatically.
///
/// Lives in `anvil-kernel` because kernel is the lowest crate every
/// walking consumer can reach: `anvil-cli` depends on `anvil-kernel`,
/// but not the reverse. Entries must stay sorted so a single textual
/// diff captures any policy change.
pub const IGNORE_DIRS: &[&str] = &[
    ".anvil",
    ".claude",
    ".gemini",
    ".git",
    ".next",
    ".nx",
    ".opencode",
    ".serena",
    ".turbo",
    ".venv",
    ".worktrees",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
];

/// Returns `true` if `name` is one of the directories every Anvil
/// walker skips (see [`IGNORE_DIRS`]).
#[must_use]
pub fn is_ignored_dir_name(name: &str) -> bool {
    IGNORE_DIRS.contains(&name)
}

/// Determines whether a file path should be processed or ignored.
///
/// `respect_extensions` controls whether the parseable-extension gate is
/// applied in [`Self::should_process`]. When the caller has its own scoping
/// criterion (e.g. the user passed `--patterns '**/*.rs'` to `anvil watch`),
/// the hardcoded ts/js extension list must yield to the user's globs —
/// otherwise events for non-JS files are dropped before the user's pattern
/// matcher ever sees them. Default is `true` to preserve existing behaviour.
#[derive(Debug, Clone)]
pub struct FileFilter {
    ignore_patterns: Vec<String>,
    respect_extensions: bool,
    exempt_paths: Vec<PathBuf>,
}

impl FileFilter {
    pub fn new(ignore_patterns: Vec<String>) -> Self {
        Self {
            ignore_patterns,
            respect_extensions: true,
            exempt_paths: Vec::new(),
        }
    }

    /// Build a filter that bypasses the parseable-extension gate. Use this
    /// when the caller has its own scoping criterion (e.g. user-supplied
    /// `--patterns` globs) and the hardcoded ts/js list would silently drop
    /// matching events.
    #[must_use]
    pub fn with_respect_extensions(mut self, respect: bool) -> Self {
        self.respect_extensions = respect;
        self
    }

    /// Always deliver events for these paths (UCFG-013 architecture
    /// source). They may live under an ignored directory (`.anvil/`)
    /// or lack a parseable source extension (`.anvil.yaml`).
    #[must_use]
    pub fn with_exempt_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.exempt_paths = paths
            .into_iter()
            .map(|path| path.canonicalize().unwrap_or(path))
            .collect();
        self
    }

    fn is_exempt(&self, path: &Path) -> bool {
        if self.exempt_paths.is_empty() {
            return false;
        }
        if self.exempt_paths.iter().any(|exempt| exempt == path) {
            return true;
        }
        let Ok(canon) = path.canonicalize() else {
            return false;
        };
        self.exempt_paths.iter().any(|exempt| exempt == &canon)
    }

    /// Default ignore patterns for typical projects. Derived from the
    /// canonical [`IGNORE_DIRS`] so watch cannot drift from audit /
    /// baseline / check / drift / gate.
    pub fn default_patterns() -> Vec<String> {
        IGNORE_DIRS.iter().map(|s| (*s).to_string()).collect()
    }

    /// Check if a path should be ignored.
    pub fn should_ignore(&self, path: &Path) -> bool {
        if self.is_exempt(path) {
            return false;
        }
        for component in path.components() {
            let name = component.as_os_str().to_string_lossy();
            if self.ignore_patterns.iter().any(|p| p == name.as_ref()) {
                return true;
            }
        }
        false
    }

    /// Check if a file has a parseable extension.
    ///
    /// Delegates to [`Language::from_path`] so the parseable gate stays the
    /// single source of truth for "is this a language the kernel parses" —
    /// every supported anchor and tail-wave language (LANGTAIL) is admitted,
    /// and adding a language is a one-line change in `languages.rs` rather than
    /// a second list to keep in sync here.
    pub fn is_parseable(&self, path: &Path) -> bool {
        crate::parser::languages::Language::from_path(path).is_some()
    }

    /// Combined check: not ignored AND a plausible file path.
    ///
    /// In default mode (`respect_extensions = true`), restricts to the
    /// hardcoded ts/js parseable list. In bypass mode (caller supplied
    /// scoped patterns), still requires the path to have *some* extension
    /// so directory `Create` events and bare paths cannot reach the parser
    /// pipeline — only the JS/TS extension restriction yields, not the
    /// "must look like a file" floor.
    pub fn should_process(&self, path: &Path) -> bool {
        if self.is_exempt(path) {
            return true;
        }
        if self.should_ignore(path) {
            return false;
        }
        if self.respect_extensions {
            self.is_parseable(path)
        } else {
            path.extension().is_some()
        }
    }
}

impl Default for FileFilter {
    fn default() -> Self {
        Self::new(Self::default_patterns())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_node_modules() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new("node_modules/foo/bar.ts")));
        assert!(filter.should_ignore(Path::new("packages/core/node_modules/x.js")));
    }

    #[test]
    fn ignores_git_directory() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new(".git/objects/abc")));
    }

    #[test]
    fn ignores_build_outputs() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new("target/debug/anvil")));
        assert!(filter.should_ignore(Path::new("dist/index.js")));
        assert!(filter.should_ignore(Path::new("build/output.js")));
    }

    #[test]
    fn ignores_local_tool_and_worktree_dirs() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new(".claude/worktrees/agent-a/src/main.ts")));
        assert!(filter.should_ignore(Path::new(".opencode/logs/session.jsonl")));
        assert!(filter.should_ignore(Path::new(".gemini/cache/state.json")));
        assert!(filter.should_ignore(Path::new(".serena/memories/index.json")));
        assert!(filter.should_ignore(Path::new(".worktrees/fix-bug/src/main.ts")));
    }

    #[test]
    fn allows_source_files() {
        let filter = FileFilter::default();
        assert!(!filter.should_ignore(Path::new("src/main.ts")));
        assert!(!filter.should_ignore(Path::new("packages/core/src/lib.ts")));
    }

    #[test]
    fn detects_parseable_extensions() {
        let filter = FileFilter::default();
        // Anchor languages.
        assert!(filter.is_parseable(Path::new("main.ts")));
        assert!(filter.is_parseable(Path::new("App.tsx")));
        assert!(filter.is_parseable(Path::new("index.js")));
        assert!(filter.is_parseable(Path::new("config.mjs")));
        assert!(filter.is_parseable(Path::new("util.cjs")));
        assert!(filter.is_parseable(Path::new("lib.rs")));
        assert!(filter.is_parseable(Path::new("mod.py")));
        // Tail-wave languages (LANGTAIL) — the gate delegates to
        // `Language::from_path`, so these are admitted as first-class.
        assert!(filter.is_parseable(Path::new("main.go")));
        assert!(filter.is_parseable(Path::new("App.java")));
        assert!(filter.is_parseable(Path::new("Main.kt")));
        assert!(filter.is_parseable(Path::new("Program.cs")));
        assert!(filter.is_parseable(Path::new("widget.dart")));
        assert!(filter.is_parseable(Path::new("engine.c")));
        assert!(filter.is_parseable(Path::new("engine.cpp")));
        // Non-source files stay out.
        assert!(!filter.is_parseable(Path::new("README.md")));
        assert!(!filter.is_parseable(Path::new("Cargo.toml")));
    }

    #[test]
    fn exempt_paths_are_processed_even_under_anvil_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        let arch = anvil_dir.join("architecture.yaml");
        std::fs::write(&arch, "layers: []\n").unwrap();
        let yaml = tmp.path().join(".anvil.yaml");
        std::fs::write(&yaml, "architecture: {}\n").unwrap();

        let filter = FileFilter::default().with_exempt_paths(vec![arch.clone(), yaml.clone()]);
        assert!(filter.should_process(&arch));
        assert!(filter.should_process(&yaml));
        assert!(!filter.should_ignore(&arch));
    }

    #[test]
    fn should_process_combines_checks() {
        let filter = FileFilter::default();
        assert!(filter.should_process(Path::new("src/main.ts")));
        assert!(!filter.should_process(Path::new("node_modules/foo.ts")));
        assert!(!filter.should_process(Path::new("src/README.md")));
    }

    #[test]
    fn ignores_coverage_directories() {
        let filter = FileFilter::default();
        // Relative paths
        assert!(filter.should_ignore(Path::new("coverage/foo.js")));
        assert!(filter.should_ignore(Path::new("apps/anvil-api/coverage/block-navigation.js")));
        // Absolute paths (as notify delivers them)
        assert!(filter.should_ignore(Path::new(
            "/home/user/project/apps/anvil-api/coverage/block-navigation.js"
        )));
        // Directory path itself (as walkdir delivers it)
        assert!(filter.should_ignore(Path::new("apps/anvil-api/coverage")));
        assert!(filter.should_ignore(Path::new("/home/user/project/apps/anvil-api/coverage")));
        // With trailing separator
        assert!(filter.should_ignore(Path::new("apps/anvil-api/coverage/")));
    }

    #[cfg(windows)]
    #[test]
    fn ignores_coverage_windows_paths() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new(
            r"C:\Users\dev\project\apps\anvil-api\coverage\block-navigation.js"
        )));
        assert!(filter.should_ignore(Path::new(r"apps\anvil-api\coverage\block-navigation.js")));
    }

    #[test]
    fn custom_patterns() {
        let filter = FileFilter::new(vec!["vendor".to_string(), "tmp".to_string()]);
        assert!(filter.should_ignore(Path::new("vendor/lib.ts")));
        assert!(filter.should_ignore(Path::new("tmp/scratch.ts")));
        assert!(!filter.should_ignore(Path::new("node_modules/x.ts")));
    }

    #[test]
    fn respect_extensions_disabled_passes_non_parseable_files() {
        // When the caller scopes by user-supplied globs, the parseable gate
        // must yield — otherwise `anvil watch --patterns '**/*.rs'` in a
        // Rust repo drops every event before the user's pattern matcher
        // sees it.
        let filter = FileFilter::default().with_respect_extensions(false);
        assert!(filter.should_process(Path::new("src/main.rs")));
        assert!(filter.should_process(Path::new("Cargo.toml")));
        assert!(filter.should_process(Path::new("README.md")));
        // Denylist still applies regardless of the extension gate.
        assert!(!filter.should_process(Path::new("node_modules/foo.rs")));
    }

    #[test]
    fn respect_extensions_disabled_still_rejects_directory_paths() {
        // Bypass mode must still keep directory `Create` events and bare
        // paths out of the parser pipeline — relaxing the JS/TS gate is not
        // the same as relaxing the "must look like a file" floor.
        let filter = FileFilter::default().with_respect_extensions(false);
        assert!(!filter.should_process(Path::new("src")));
        assert!(!filter.should_process(Path::new("crates/anvil-cli/src")));
        assert!(!filter.should_process(Path::new("Makefile")));
        // Denylist still applies — a denylisted directory remains rejected.
        assert!(!filter.should_process(Path::new("target/debug/anvil")));
    }

    #[test]
    fn respect_extensions_default_admits_all_supported_languages() {
        // The default gate now admits every language the kernel can parse
        // (anchors + LANGTAIL tail wave), not just JS/TS — but still rejects
        // files with no supported grammar so non-source never reaches the
        // parser.
        let filter = FileFilter::default();
        assert!(filter.should_process(Path::new("src/main.ts")));
        assert!(filter.should_process(Path::new("src/main.rs")));
        assert!(filter.should_process(Path::new("cmd/main.go")));
        assert!(filter.should_process(Path::new("App.java")));
        assert!(!filter.should_process(Path::new("src/notes.md")));
        assert!(!filter.should_process(Path::new("Cargo.toml")));
    }

    /// ADOPT-004: the canonical [`IGNORE_DIRS`] list must cover every
    /// directory name the adoption-friction module spells out. Per-surface
    /// consumers (`anvil-cli/src/util.rs` re-export, watcher
    /// `default_patterns()`) derive from this const, so adding an entry
    /// here propagates everywhere.
    #[test]
    fn ignore_policy_covers_all_surfaces() {
        // Required: the explicit set the ADOPT-004 work item names. These
        // must always be in the canonical const — removing one is a
        // policy regression.
        for required in [
            ".anvil",
            ".claude",
            ".gemini",
            ".git",
            ".next",
            ".nx",
            ".opencode",
            ".serena",
            ".turbo",
            ".venv",
            ".worktrees",
            "__pycache__",
            "build",
            "coverage",
            "dist",
            "node_modules",
            "target",
        ] {
            assert!(
                IGNORE_DIRS.contains(&required),
                "ADOPT-004: kernel IGNORE_DIRS missing {required}",
            );
        }
        // Defence in depth: every IGNORE_DIRS entry must round-trip
        // through is_ignored_dir_name. Catches a future entry that
        // somehow bypasses the helper.
        for entry in IGNORE_DIRS {
            assert!(
                is_ignored_dir_name(entry),
                "is_ignored_dir_name disagrees with IGNORE_DIRS for {entry}",
            );
        }
    }

    #[test]
    fn default_patterns_derives_from_canonical_const() {
        let patterns = FileFilter::default_patterns();
        for entry in IGNORE_DIRS {
            assert!(
                patterns.iter().any(|p| p == entry),
                "default_patterns missing {entry} — must derive from IGNORE_DIRS",
            );
        }
        assert_eq!(
            patterns.len(),
            IGNORE_DIRS.len(),
            "default_patterns must be exactly the IGNORE_DIRS set, no drift",
        );
    }

    #[test]
    fn ignores_python_caches_and_venvs() {
        let filter = FileFilter::default();
        assert!(filter.should_ignore(Path::new("__pycache__/foo.cpython-312.pyc")));
        assert!(filter.should_ignore(Path::new("pkg/__pycache__/x.pyc")));
        assert!(filter.should_ignore(Path::new(".venv/bin/python")));
        assert!(filter.should_ignore(Path::new("services/api/.venv/lib/site-packages/x.py")));
    }
}
