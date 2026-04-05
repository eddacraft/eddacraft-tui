use std::path::Path;

/// Default directory segments excluded from scan results (DD-1).
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

/// Default file suffix patterns excluded from scan results (DD-1).
const DEFAULT_SUFFIX_EXCLUDES: &[&str] = &[".test.ts", ".spec.ts", ".test.rs", "_test.rs"];

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

    /// Create a filter from custom glob patterns.
    ///
    /// Patterns ending with `/` are treated as directory-segment rules (the
    /// trailing slash is stripped before matching). All other patterns are
    /// treated as file-suffix rules with a leading `*` stripped if present
    /// (e.g. `*.test.ts` becomes suffix `.test.ts`).
    #[must_use]
    pub fn new(patterns: Vec<String>) -> Self {
        let rules = patterns.into_iter().map(|p| categorise_pattern(&p)).collect();
        Self { rules }
    }

    /// Returns `true` if the path should be **included** (not excluded).
    /// Returns `false` if the path matches any exclusion pattern.
    #[must_use]
    pub fn includes(&self, path: &Path) -> bool {
        let normalised = normalise(path);
        !self.rules.iter().any(|rule| matches_rule(rule, &normalised))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Normalise a path to a forward-slash string for consistent matching.
fn normalise(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Determine whether a single rule excludes the given (already-normalised)
/// path string.
fn matches_rule(rule: &ExcludeRule, normalised: &str) -> bool {
    match rule {
        ExcludeRule::DirSegment(segment) => normalised
            .split('/')
            .any(|component| component == segment),
        ExcludeRule::FileSuffix(suffix) => {
            let filename = normalised.rsplit('/').next().unwrap_or(normalised);
            filename.ends_with(suffix.as_str())
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
