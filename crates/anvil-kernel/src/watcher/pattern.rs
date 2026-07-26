//! User-supplied glob filter for the watch loop.
//!
//! Distinct from [`super::filter::FileFilter`], which owns the hardcoded
//! denylist (`node_modules`, `.git`, `target`, …) and the parseable-extension
//! gate. Both filters apply: a watched event must clear the internal
//! denylist *and* match the user's include/exclude globs.
//!
//! Empty include = match everything (no positive filter).
//! Empty exclude = exclude nothing.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Compiled glob set for the user's `--patterns` / `--exclude` args.
/// Holds nothing if both are empty — the caller can short-circuit on
/// [`Self::is_noop`] to skip per-event filtering altogether.
#[derive(Debug, Clone)]
pub struct WatchPatternFilter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error("invalid glob pattern '{pattern}': {source}")]
    Compile {
        pattern: String,
        #[source]
        source: globset::Error,
    },
    #[error("failed to build glob set: {0}")]
    Build(#[source] globset::Error),
}

impl WatchPatternFilter {
    /// Compile include and exclude pattern lists. Empty lists are
    /// treated as "no filter" — see [`Self::is_noop`].
    pub fn new(include: &[String], exclude: &[String]) -> Result<Self, PatternError> {
        Ok(Self {
            include: build_set(include)?,
            exclude: build_set(exclude)?,
        })
    }

    /// True when neither include nor exclude was supplied, i.e. every
    /// path passes. Lets the watch loop skip the filter call entirely.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.include.is_none() && self.exclude.is_none()
    }

    /// Return true if the relative path should be tracked: passes the
    /// include filter (or no include filter) AND does not match the
    /// exclude filter.
    ///
    /// Paths must be repo-relative — globs like `src/**/*.ts` won't
    /// match an absolute path with a different prefix.
    ///
    /// Windows note: `Path::strip_prefix` preserves the OS separator
    /// (`\\`) on Windows, but globs are conventionally written with
    /// forward slashes (`/`). We normalise the path to forward-slash
    /// form before matching so `src/**/*.ts` matches `src\foo.ts` as
    /// users expect. The normalisation is platform-gated; on Unix the
    /// path is matched directly because forward-slash is already the
    /// separator.
    #[must_use]
    pub fn matches(&self, rel_path: &Path) -> bool {
        // Shadow the argument on Windows rather than branching into two
        // cfg'd blocks: with the blocks, the Windows arm's `return` was the
        // function's tail expression once the other arm was cfg-stripped, so
        // clippy's `needless_return` fired — but only on a Windows build,
        // which no PR-triggered job compiled until CIB-193 added one.
        #[cfg(windows)]
        let normalised = rel_path.to_string_lossy().replace('\\', "/");
        #[cfg(windows)]
        let rel_path = Path::new(&normalised);

        self.matches_inner(rel_path)
    }

    fn matches_inner(&self, rel_path: &Path) -> bool {
        if let Some(set) = &self.exclude
            && set.is_match(rel_path)
        {
            return false;
        }
        match &self.include {
            Some(set) => set.is_match(rel_path),
            None => true,
        }
    }
}

fn build_set(patterns: &[String]) -> Result<Option<GlobSet>, PatternError> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|source| PatternError::Compile {
            pattern: pattern.clone(),
            source,
        })?;
        builder.add(glob);
    }
    builder.build().map(Some).map_err(PatternError::Build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filters_match_everything() {
        let filter = WatchPatternFilter::new(&[], &[]).unwrap();
        assert!(filter.is_noop());
        assert!(filter.matches(Path::new("src/foo.ts")));
        assert!(filter.matches(Path::new("vendor/lib.js")));
    }

    #[test]
    fn include_only_filters_to_matching_paths() {
        let filter = WatchPatternFilter::new(&["src/**/*.ts".to_string()], &[]).unwrap();
        assert!(!filter.is_noop());
        assert!(filter.matches(Path::new("src/foo.ts")));
        assert!(filter.matches(Path::new("src/sub/bar.ts")));
        assert!(!filter.matches(Path::new("lib/foo.ts")));
        assert!(!filter.matches(Path::new("src/foo.js")));
    }

    #[test]
    fn exclude_only_drops_matching_paths() {
        let filter = WatchPatternFilter::new(&[], &["vendor/**".to_string()]).unwrap();
        assert!(!filter.is_noop());
        assert!(filter.matches(Path::new("src/foo.ts")));
        assert!(!filter.matches(Path::new("vendor/lib.ts")));
        assert!(!filter.matches(Path::new("vendor/sub/foo.ts")));
    }

    #[test]
    fn exclude_takes_precedence_over_include() {
        // src/**/*.ts matches src/vendor/foo.ts, but vendor/** also matches
        // — exclude must win.
        let filter =
            WatchPatternFilter::new(&["src/**/*.ts".to_string()], &["**/vendor/**".to_string()])
                .unwrap();
        assert!(filter.matches(Path::new("src/foo.ts")));
        assert!(!filter.matches(Path::new("src/vendor/foo.ts")));
    }

    #[test]
    fn multiple_includes_or_together() {
        let filter =
            WatchPatternFilter::new(&["src/**/*.ts".to_string(), "lib/**/*.ts".to_string()], &[])
                .unwrap();
        assert!(filter.matches(Path::new("src/foo.ts")));
        assert!(filter.matches(Path::new("lib/bar.ts")));
        assert!(!filter.matches(Path::new("vendor/baz.ts")));
    }

    #[test]
    fn invalid_pattern_returns_compile_error() {
        // An unmatched bracket is invalid.
        let err = WatchPatternFilter::new(&["src/[unclosed".to_string()], &[]).unwrap_err();
        match err {
            PatternError::Compile { pattern, .. } => {
                assert_eq!(pattern, "src/[unclosed");
            }
            other @ PatternError::Build(_) => {
                panic!("expected Compile error, got {other:?}");
            }
        }
    }

    #[test]
    fn bare_directory_name_does_not_match_files_inside() {
        // Documents the breaking change: bare names like "vendor" no
        // longer match files inside that directory. Users who used to
        // pass `--exclude vendor` must now pass `--exclude vendor/**`.
        let filter = WatchPatternFilter::new(&[], &["vendor".to_string()]).unwrap();
        // The bare name only matches a path *equal* to "vendor" — files
        // inside the directory are still tracked because the glob does
        // not auto-recurse.
        assert!(filter.matches(Path::new("vendor/lib.ts")));
    }

    #[cfg(windows)]
    #[test]
    fn windows_backslash_paths_match_forward_slash_globs() {
        // The watcher strips a windows-prefixed path, leaving backslash
        // separators in the relative form. Globs are written with forward
        // slashes by convention. Without normalisation, `src/**/*.ts`
        // would silently fail to match `src\foo.ts` on Windows.
        let filter = WatchPatternFilter::new(&["src/**/*.ts".to_string()], &[]).unwrap();
        assert!(filter.matches(Path::new("src\\foo.ts")));
        assert!(filter.matches(Path::new("src\\sub\\bar.ts")));
        // Forward-slash form still matches.
        assert!(filter.matches(Path::new("src/foo.ts")));
        // Excludes follow the same rule.
        let filter = WatchPatternFilter::new(&[], &["vendor/**".to_string()]).unwrap();
        assert!(!filter.matches(Path::new("vendor\\lib.ts")));
    }
}
