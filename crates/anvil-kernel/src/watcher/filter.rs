use std::path::Path;

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
}

impl FileFilter {
    pub fn new(ignore_patterns: Vec<String>) -> Self {
        Self {
            ignore_patterns,
            respect_extensions: true,
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

    /// Default ignore patterns for typical projects.
    pub fn default_patterns() -> Vec<String> {
        vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "target".to_string(),
            "dist".to_string(),
            "build".to_string(),
            ".next".to_string(),
            ".turbo".to_string(),
            ".nx".to_string(),
            "coverage".to_string(),
            ".anvil".to_string(),
        ]
    }

    /// Check if a path should be ignored.
    pub fn should_ignore(&self, path: &Path) -> bool {
        for component in path.components() {
            let name = component.as_os_str().to_string_lossy();
            if self.ignore_patterns.iter().any(|p| p == name.as_ref()) {
                return true;
            }
        }
        false
    }

    /// Check if a file has a parseable extension.
    pub fn is_parseable(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
        )
    }

    /// Combined check: not ignored AND (extension gate disabled OR has a
    /// parseable extension).
    pub fn should_process(&self, path: &Path) -> bool {
        !self.should_ignore(path) && (!self.respect_extensions || self.is_parseable(path))
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
    fn allows_source_files() {
        let filter = FileFilter::default();
        assert!(!filter.should_ignore(Path::new("src/main.ts")));
        assert!(!filter.should_ignore(Path::new("packages/core/src/lib.ts")));
    }

    #[test]
    fn detects_parseable_extensions() {
        let filter = FileFilter::default();
        assert!(filter.is_parseable(Path::new("main.ts")));
        assert!(filter.is_parseable(Path::new("App.tsx")));
        assert!(filter.is_parseable(Path::new("index.js")));
        assert!(filter.is_parseable(Path::new("config.mjs")));
        assert!(filter.is_parseable(Path::new("util.cjs")));
        assert!(!filter.is_parseable(Path::new("README.md")));
        assert!(!filter.is_parseable(Path::new("Cargo.toml")));
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
        assert!(!filter.should_process(Path::new("target/debug/anvil")));
    }

    #[test]
    fn respect_extensions_default_keeps_ts_js_only() {
        let filter = FileFilter::default();
        assert!(filter.should_process(Path::new("src/main.ts")));
        assert!(!filter.should_process(Path::new("src/main.rs")));
    }
}
