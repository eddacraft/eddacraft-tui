use std::path::{Component, Path};

/// Default directory segments excluded from scan results (DD-1).
///
/// Correctness-only set: test fixtures, VCS metadata, dependency caches, and
/// Rust/Node build trees that scanners should never descend into regardless of
/// whether the project has a `.gitignore`. This list is part of `ScanFilter`'s
/// public default behaviour — additions here are breaking changes for
/// downstream users of `ScanFilter::default_excludes()`.
const DEFAULT_DIR_EXCLUDES: &[&str] = &[
    "__fixtures__",
    "__mocks__",
    "__tests__",
    "test-data",
    "fixtures",
    "node_modules",
    "target",
    ".git",
];

/// Framework build-output directories.
///
/// These are *not* part of `DEFAULT_DIR_EXCLUDES` because excluding them is a
/// policy choice (a user may legitimately want to scan their `dist/` for
/// secrets before shipping). Discovery flows that want the belt-and-suspenders
/// behaviour should compose this with `ScanFilter::new` explicitly.
pub const BUILD_ARTEFACT_DIRS: &[&str] = &[
    "dist",
    "build",
    "out",
    "coverage",
    ".next",
    ".nuxt",
    ".nx",
    ".turbo",
    ".cache",
    ".angular",
    ".svelte-kit",
];

/// Default file suffix patterns excluded from scan results (DD-1).
const DEFAULT_SUFFIX_EXCLUDES: &[&str] = &[".test.ts", ".spec.ts", ".test.rs", "_test.rs"];

/// File extensions treated as binary — scanners skip them without reading.
const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "woff", "woff2", "ttf", "otf", "eot", "pdf", "zip", "gz",
    "tar", "exe", "dll", "so", "dylib", "wasm", "o", "a",
];

/// Filenames that secret scanners should read regardless of `.gitignore`
/// status. These are the canonical locations where developers keep
/// credentials, and they are almost always gitignored — which is exactly why a
/// gitignore-aware walker misses them.
///
/// `.env*` files are deliberately **not** here. A `.env` file is the designated
/// place to keep local secrets, so reporting one back to the user as a
/// high-severity finding is noise they cannot action (GH #2584). The first-run
/// discovery scan therefore excludes `.env*` outright in `welcome::candidate_path`
/// (even under `ANVIL_SCAN_ALL`). A secret committed to a tracked `.env` is not
/// ignored, though: it is still caught by `anvil gate` (the `secret-detection`
/// guardrail), `anvil audit`, and the save-time intercept — surfaces whose job
/// is to catch committed secrets.
pub const ALWAYS_SCAN_FILENAMES: &[&str] = &[
    "credentials.json",
    "secrets.yml",
    "secrets.yaml",
    ".npmrc",
    ".pypirc",
    ".netrc",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "id_dsa",
];

/// Dependency lockfile basenames. Lockfiles pin resolved dependency versions
/// and carry integrity hashes (e.g. `sha512-…` in `package-lock.json`,
/// base64 module hashes in `go.sum`); those hashes are high-entropy by
/// construction and trip the secret scanner's entropy detector as false
/// positives (GH #2584). Lockfiles are generated dependency metadata, not a
/// place first-party secrets are authored, so they are not run through the full
/// secret scan. They are *not* ignored entirely: a credential embedded in a
/// `resolved`/source URL can still leak, so lockfiles get a restricted
/// URL-credential-only scan (see [`crate::secret::scan_lockfile_url_credentials`]).
///
/// Matched by exact basename rather than extension because most lockfiles do
/// not end in `.lock` — `package-lock.json`, `pnpm-lock.yaml`, `go.sum`, and
/// `npm-shrinkwrap.json` would otherwise slip past a `.lock` suffix filter.
pub const LOCKFILE_FILENAMES: &[&str] = &[
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "bun.lock",
    "bun.lockb",
    "Cargo.lock",
    "composer.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Pipfile.lock",
    "go.sum",
    "packages.lock.json",
    "pubspec.lock",
    "mix.lock",
    "flake.lock",
    "Podfile.lock",
    "gradle.lockfile",
];

/// Return `true` if the file extension indicates a binary asset that secret
/// and antipattern scanners should skip.
///
/// ASCII case-insensitive — `Logo.PNG` and `image.JPG` should be classified
/// the same as their lowercase forms on case-insensitive filesystems and in
/// mixed-case repos.
#[must_use]
pub fn is_binary_extension(ext: &str) -> bool {
    BINARY_EXTENSIONS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(ext))
}

/// Return `true` if the path's extension is a known binary asset.
#[must_use]
pub fn is_binary_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(is_binary_extension)
}

/// Return `true` if the path's filename matches `ALWAYS_SCAN_FILENAMES` —
/// i.e. the file should be scanned even when a gitignore-aware walker would
/// otherwise skip it.
#[must_use]
pub fn is_always_scan_filename(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| ALWAYS_SCAN_FILENAMES.contains(&name))
}

/// Return `true` if the path is a dependency lockfile (see
/// [`LOCKFILE_FILENAMES`]). Secret scanning skips these because their integrity
/// hashes are high-entropy by construction and produce only false positives.
#[must_use]
pub fn is_lockfile(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| LOCKFILE_FILENAMES.contains(&name))
}

/// Categorised exclusion pattern — either a directory segment match or a file
/// suffix match.
#[derive(Debug, Clone)]
enum ExcludeRule {
    /// Matches if any path segment equals the value exactly.
    DirSegment(String),
    /// Matches if the filename ends with the suffix.
    FileSuffix(String),
}

/// Path filter that excludes test fixtures, build artefacts, and other
/// non-source paths from scan results.
///
/// Directory-segment rules match when *any* component of the path equals the
/// pattern (e.g. `__fixtures__` matches `src/__fixtures__/sample.ts`).
///
/// File-suffix rules match against the final component (filename) of the path.
#[derive(Debug, Clone)]
pub struct ScanFilter {
    rules: Vec<ExcludeRule>,
}

impl ScanFilter {
    /// Create a filter with the default exclusion patterns from DD-1.
    #[must_use]
    pub fn default_excludes() -> Self {
        let mut rules: Vec<ExcludeRule> = DEFAULT_DIR_EXCLUDES
            .iter()
            .map(|s| ExcludeRule::DirSegment((*s).to_owned()))
            .collect();

        rules.extend(
            DEFAULT_SUFFIX_EXCLUDES
                .iter()
                .map(|s| ExcludeRule::FileSuffix((*s).to_owned())),
        );

        Self { rules }
    }

    /// Create a filter from custom patterns.
    ///
    /// Patterns ending with `/` are treated as directory-segment rules (the
    /// trailing slash is stripped before matching). All other patterns are
    /// treated as file-suffix rules with a leading `*` stripped if present
    /// (e.g. `*.test.ts` becomes suffix `.test.ts`).
    ///
    /// **Note:** A bare name without a trailing `/` (e.g. `"vendor"`) is
    /// treated as a file suffix, not a directory segment. Use `"vendor/"`
    /// to match a directory.
    #[must_use]
    pub fn new(patterns: Vec<String>) -> Self {
        let rules = patterns
            .into_iter()
            .map(|p| categorise_pattern(&p))
            .collect();
        Self { rules }
    }

    /// Create a filter starting from `default_excludes()` and adding the
    /// supplied extra patterns on top. Extra patterns follow the same
    /// categorisation rules as [`ScanFilter::new`] (trailing `/` for
    /// directory segments, otherwise file suffix).
    ///
    /// Prefer this over manually re-specifying the default exclude list in
    /// caller code — it guarantees the defaults cannot drift.
    #[must_use]
    pub fn default_with(extra: Vec<String>) -> Self {
        let mut filter = Self::default_excludes();
        filter
            .rules
            .extend(extra.into_iter().map(|p| categorise_pattern(&p)));
        filter
    }

    /// Returns `true` if the path should be **included** (not excluded).
    /// Returns `false` if the path matches any exclusion pattern.
    #[must_use]
    pub fn includes(&self, path: &Path) -> bool {
        !self.rules.iter().any(|rule| matches_rule(rule, path))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Determine whether a single rule excludes the given path.
///
/// Uses `Path::components()` for directory-segment matching. On Unix,
/// backslashes are valid filename characters (not separators), so we also
/// split component strings on `\` to handle Windows-style paths that may
/// arrive cross-platform.
fn matches_rule(rule: &ExcludeRule, path: &Path) -> bool {
    match rule {
        ExcludeRule::DirSegment(segment) => path.components().any(|c| match c {
            Component::Normal(s) => s
                .to_string_lossy()
                .split(['/', '\\'])
                .any(|part| part == segment.as_str()),
            _ => false,
        }),
        ExcludeRule::FileSuffix(suffix) => {
            if let Some(filename) = path.file_name() {
                filename.to_string_lossy().ends_with(suffix.as_str())
            } else {
                let name = path.to_string_lossy();
                let filename = name.rsplit(['/', '\\']).next().unwrap_or(&name);
                filename.ends_with(suffix.as_str())
            }
        }
    }
}

/// Convert a user-supplied pattern string into the appropriate rule variant.
fn categorise_pattern(pattern: &str) -> ExcludeRule {
    if let Some(dir) = pattern.strip_suffix('/') {
        ExcludeRule::DirSegment(dir.to_owned())
    } else {
        let suffix = pattern.strip_prefix('*').unwrap_or(pattern);
        ExcludeRule::FileSuffix(suffix.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::ScanFilter;

    fn default_filter() -> ScanFilter {
        ScanFilter::default_excludes()
    }

    // ── Directory-segment exclusions (DD-1) ──────────────────────

    #[test]
    fn excludes_fixtures_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/__fixtures__/sample.ts")));
    }

    #[test]
    fn excludes_mocks_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new("lib/__mocks__/service.ts")));
    }

    #[test]
    fn excludes_tests_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/__tests__/app.test.ts")));
    }

    #[test]
    fn excludes_test_data_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new("test-data/golden/report.json")));
    }

    #[test]
    fn excludes_bare_fixtures_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new("core/fixtures/sample.json")));
    }

    #[test]
    fn excludes_node_modules_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new("node_modules/lodash/index.js")));
    }

    #[test]
    fn excludes_target_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new("target/debug/anvil")));
    }

    #[test]
    fn excludes_git_dir() {
        let f = default_filter();
        assert!(!f.includes(Path::new(".git/config")));
    }

    // ── File-suffix exclusions (DD-1) ────────────────────────────

    #[test]
    fn excludes_test_ts_suffix() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/custom_filter.test.ts")));
    }

    #[test]
    fn excludes_spec_ts_suffix() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/widget.spec.ts")));
    }

    #[test]
    fn excludes_test_rs_suffix() {
        let f = default_filter();
        assert!(!f.includes(Path::new("crates/anvil/checks.test.rs")));
    }

    #[test]
    fn excludes_underscore_test_rs_suffix() {
        let f = default_filter();
        assert!(!f.includes(Path::new("crates/anvil/checks_test.rs")));
    }

    // ── Non-matching paths are included ──────────────────────────

    #[test]
    fn includes_regular_source_file() {
        let f = default_filter();
        assert!(f.includes(Path::new("src/main.rs")));
    }

    #[test]
    fn includes_regular_ts_file() {
        let f = default_filter();
        assert!(f.includes(Path::new("src/custom_filter.ts")));
    }

    #[test]
    fn includes_regular_json_file() {
        let f = default_filter();
        assert!(f.includes(Path::new("config/settings.json")));
    }

    // ── Nested fixtures ──────────────────────────────────────────

    #[test]
    fn excludes_deeply_nested_fixtures() {
        let f = default_filter();
        assert!(!f.includes(Path::new("a/b/__fixtures__/c/d.ts")));
    }

    #[test]
    fn excludes_deeply_nested_test_data() {
        let f = default_filter();
        assert!(!f.includes(Path::new("packages/core/test-data/golden/snapshot.json")));
    }

    // ── Partial matches must NOT falsely exclude ─────────────────

    #[test]
    fn does_not_exclude_partial_fixtures_match() {
        let f = default_filter();
        assert!(f.includes(Path::new("my_fixtures/file.ts")));
    }

    #[test]
    fn does_not_exclude_partial_test_data_match() {
        let f = default_filter();
        assert!(f.includes(Path::new("my-test-data-extra/file.ts")));
    }

    #[test]
    fn does_not_exclude_partial_git_match() {
        let f = default_filter();
        assert!(f.includes(Path::new(".github/workflows/ci.yml")));
    }

    #[test]
    fn does_not_exclude_partial_target_match() {
        let f = default_filter();
        assert!(f.includes(Path::new("src/target_impl/main.rs")));
    }

    // ── File suffix edge cases ───────────────────────────────────

    #[test]
    fn test_ts_excluded_but_plain_ts_included() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/widget.test.ts")));
        assert!(f.includes(Path::new("src/widget.ts")));
    }

    #[test]
    fn spec_ts_excluded_but_inspector_ts_included() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/thing.spec.ts")));
        assert!(f.includes(Path::new("src/inspector.ts")));
    }

    #[test]
    fn test_rs_excluded_but_plain_rs_included() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/scanner.test.rs")));
        assert!(f.includes(Path::new("src/scanner.rs")));
    }

    #[test]
    fn underscore_test_rs_excluded_but_plain_rs_included() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src/filter_test.rs")));
        assert!(f.includes(Path::new("src/filter.rs")));
    }

    // ── Path normalisation (Windows-style backslashes) ───────────

    #[test]
    fn normalises_backslash_paths_for_dir_segment() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src\\__fixtures__\\sample.ts")));
    }

    #[test]
    fn normalises_backslash_paths_for_suffix() {
        let f = default_filter();
        assert!(!f.includes(Path::new("src\\widget.test.ts")));
    }

    // ── Empty / root paths don't panic ───────────────────────────

    #[test]
    fn empty_path_does_not_panic() {
        let f = default_filter();
        assert!(f.includes(Path::new("")));
    }

    #[test]
    fn root_path_does_not_panic() {
        let f = default_filter();
        assert!(f.includes(Path::new("/")));
    }

    #[test]
    fn dot_path_does_not_panic() {
        let f = default_filter();
        assert!(f.includes(Path::new(".")));
    }

    // ── Absolute paths ───────────────────────────────────────────

    #[test]
    fn excludes_absolute_path_with_fixtures() {
        let f = default_filter();
        assert!(!f.includes(Path::new("/home/user/project/__fixtures__/data.json")));
    }

    #[test]
    fn includes_absolute_path_without_excludes() {
        let f = default_filter();
        assert!(f.includes(Path::new("/home/user/project/src/main.rs")));
    }

    // ── Custom filter via ScanFilter::new ────────────────────────

    #[test]
    fn custom_dir_pattern_excludes() {
        let f = ScanFilter::new(vec!["vendor/".to_owned()]);
        assert!(!f.includes(Path::new("deps/vendor/lib.rs")));
        assert!(f.includes(Path::new("src/main.rs")));
    }

    #[test]
    fn custom_suffix_pattern_excludes() {
        let f = ScanFilter::new(vec!["*.snapshot".to_owned()]);
        assert!(!f.includes(Path::new("tests/output.snapshot")));
        assert!(f.includes(Path::new("tests/output.txt")));
    }

    #[test]
    fn custom_mixed_patterns() {
        let f = ScanFilter::new(vec!["build/".to_owned(), "*.gen.ts".to_owned()]);
        assert!(!f.includes(Path::new("out/build/index.js")));
        assert!(!f.includes(Path::new("src/schema.gen.ts")));
        assert!(f.includes(Path::new("src/schema.ts")));
    }

    #[test]
    fn bare_name_without_slash_is_suffix_not_dir() {
        // A bare name without trailing `/` becomes a file suffix rule.
        // Use "vendor/" (with slash) to match a directory segment.
        let f = ScanFilter::new(vec!["vendor".to_owned()]);
        // Does NOT exclude "deps/vendor/lib.rs" (no dir-segment match)
        assert!(f.includes(Path::new("deps/vendor/lib.rs")));
        // DOES exclude a file named "vendor"
        assert!(!f.includes(Path::new("deps/vendor")));
    }

    // ── Build artefacts are NOT in the default excludes ──────────
    //
    // Build output directories are the caller's decision — adding them to
    // `DEFAULT_DIR_EXCLUDES` would silently change behaviour for every
    // downstream `ScanFilter::default_excludes()` caller.

    #[test]
    fn default_filter_does_not_exclude_dist() {
        let f = default_filter();
        assert!(f.includes(Path::new("dist/bundle.js")));
    }

    #[test]
    fn default_filter_does_not_exclude_next() {
        let f = default_filter();
        assert!(f.includes(Path::new(".next/static/chunks/app.js")));
    }

    #[test]
    fn build_artefact_list_can_be_composed_into_filter() {
        use super::BUILD_ARTEFACT_DIRS;
        let patterns: Vec<String> = BUILD_ARTEFACT_DIRS
            .iter()
            .map(|d| format!("{d}/"))
            .collect();
        let f = ScanFilter::new(patterns);
        assert!(!f.includes(Path::new("dist/bundle.js")));
        assert!(!f.includes(Path::new(".next/static/chunks/app.js")));
        assert!(f.includes(Path::new("src/main.rs")));
    }

    #[test]
    fn default_with_preserves_defaults_and_adds_extras() {
        use super::BUILD_ARTEFACT_DIRS;
        let patterns: Vec<String> = BUILD_ARTEFACT_DIRS
            .iter()
            .map(|d| format!("{d}/"))
            .collect();
        let f = ScanFilter::default_with(patterns);
        assert!(!f.includes(Path::new("node_modules/lodash/index.js")));
        assert!(!f.includes(Path::new("target/debug/anvil")));
        assert!(!f.includes(Path::new("src/widget.test.ts")));
        assert!(!f.includes(Path::new("dist/bundle.js")));
        assert!(!f.includes(Path::new(".next/static/chunks/app.js")));
        assert!(f.includes(Path::new("src/main.rs")));
    }

    // ── is_binary_path ───────────────────────────────────────────

    #[test]
    fn is_binary_path_detects_common_binaries() {
        use super::is_binary_path;
        assert!(is_binary_path(Path::new("assets/logo.png")));
        assert!(is_binary_path(Path::new("bin/anvil.exe")));
        assert!(is_binary_path(Path::new("target/release/libfoo.so")));
    }

    #[test]
    fn is_binary_path_returns_false_for_source() {
        use super::is_binary_path;
        assert!(!is_binary_path(Path::new("src/main.rs")));
        assert!(!is_binary_path(Path::new(".env")));
        assert!(!is_binary_path(Path::new("README")));
    }

    // ── is_always_scan_filename ──────────────────────────────────

    #[test]
    fn always_scan_does_not_force_scan_dotenv_variants() {
        use super::is_always_scan_filename;
        // GH #2584: `.env*` files are the designated local secret store, so a
        // gitignored one must NOT be force-scanned. (A committed `.env` is
        // still caught by the ordinary gitignore-respecting walk.)
        assert!(!is_always_scan_filename(Path::new(".env")));
        assert!(!is_always_scan_filename(Path::new("src/.env.local")));
        assert!(!is_always_scan_filename(Path::new(
            "packages/app/.env.production"
        )));
    }

    #[test]
    fn always_scan_matches_credential_files() {
        use super::is_always_scan_filename;
        assert!(is_always_scan_filename(Path::new("credentials.json")));
        assert!(is_always_scan_filename(Path::new(".ssh/id_rsa")));
        assert!(is_always_scan_filename(Path::new("~/.netrc")));
    }

    #[test]
    fn always_scan_ignores_similar_names() {
        use super::is_always_scan_filename;
        // Only exact filename matches — these near-misses must NOT match.
        assert!(!is_always_scan_filename(Path::new("config.env.example")));
        assert!(!is_always_scan_filename(Path::new("id_rsa.pub")));
        assert!(!is_always_scan_filename(Path::new("env.ts")));
    }

    // ── is_lockfile (GH #2584) ───────────────────────────────────

    #[test]
    fn is_lockfile_matches_non_dot_lock_lockfiles() {
        use super::is_lockfile;
        // These are the ones a `.lock` suffix filter misses.
        assert!(is_lockfile(Path::new("package-lock.json")));
        assert!(is_lockfile(Path::new("frontend/pnpm-lock.yaml")));
        assert!(is_lockfile(Path::new("npm-shrinkwrap.json")));
        assert!(is_lockfile(Path::new("services/api/go.sum")));
    }

    #[test]
    fn is_lockfile_matches_dot_lock_lockfiles() {
        use super::is_lockfile;
        assert!(is_lockfile(Path::new("Cargo.lock")));
        assert!(is_lockfile(Path::new("app/yarn.lock")));
        assert!(is_lockfile(Path::new("composer.lock")));
        assert!(is_lockfile(Path::new("Gemfile.lock")));
    }

    #[test]
    fn is_lockfile_ignores_non_lockfiles() {
        use super::is_lockfile;
        assert!(!is_lockfile(Path::new("package.json")));
        assert!(!is_lockfile(Path::new("src/main.rs")));
        // A bespoke `*.lock` (e.g. an app's own lockfile) is not a dependency
        // lockfile and is not silently skipped by basename.
        assert!(!is_lockfile(Path::new("my-app.lock")));
    }
}
