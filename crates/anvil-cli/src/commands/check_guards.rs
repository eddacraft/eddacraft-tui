//! OPSUP-006: file-presence guards and wall-time caps for long-running checks.

use std::time::Duration;

/// Result of evaluating a check's file-shape guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileShapeGuard {
    /// No file-shape patterns were declared — the check must run.
    Unguarded,
    /// At least one workspace file matched a declared pattern. Carries the
    /// number of matches so callers can surface "N files in scope" in
    /// progress output if they wish.
    Present(usize),
    /// Patterns were declared but no workspace file matched any of them.
    /// The caller should short-circuit the check and emit a "no matching
    /// files" skip result.
    Absent,
}

impl FileShapeGuard {
    /// Returns `true` when the caller should run the underlying check.
    pub(crate) fn should_run(&self) -> bool {
        !matches!(self, FileShapeGuard::Absent)
    }
}

/// Result of evaluating a check's **soft** wall-time budget after the
/// check ran.
///
/// The evaluator is report-only: it classifies the observed elapsed time
/// against the declared budget but **never cancels the check**. Rust
/// threads cannot be safely pre-empted, so the budget is a label, not a
/// deadline. Surface and pack modules relying on this should treat
/// budget breach as an observability signal — not a correctness contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WallTimeGuard {
    /// No soft budget was declared — nothing to report.
    Unbudgeted,
    /// The check finished within its soft budget.
    WithinBudget,
    /// The check exceeded its soft budget. Carries both the declared
    /// budget and the measured elapsed time so the caller can surface a
    /// precise overrun reason. The check itself ran to completion.
    Exceeded { budget_secs: u64, elapsed_ms: u128 },
}

impl WallTimeGuard {
    /// Human-readable reason suitable for appending to a check result
    /// message when the budget is exceeded. Returns `None` otherwise.
    pub(crate) fn timeout_reason(&self) -> Option<String> {
        match self {
            WallTimeGuard::Exceeded {
                budget_secs,
                elapsed_ms,
            } => Some(format!(
                "exceeded wall-time budget ({elapsed_ms} ms > {budget_secs}s cap)"
            )),
            _ => None,
        }
    }
}

/// Evaluate a check's file-shape guard against the walked workspace files.
///
/// `patterns` is a slice of simple glob patterns (see [`pattern_matches`]).
/// An empty slice means "no guard declared" and yields
/// [`FileShapeGuard::Unguarded`] — equivalent to the current always-run
/// behaviour and safe for migration: every existing check defaults to
/// unguarded.
pub(crate) fn evaluate_file_presence(patterns: &[&str], files: &[String]) -> FileShapeGuard {
    if patterns.is_empty() {
        return FileShapeGuard::Unguarded;
    }
    let mut matches = 0usize;
    for file in files {
        if patterns.iter().any(|pat| pattern_matches(pat, file)) {
            matches += 1;
        }
    }
    if matches == 0 {
        FileShapeGuard::Absent
    } else {
        FileShapeGuard::Present(matches)
    }
}

/// Evaluate a check's wall-time guard given a declared budget and the
/// measured elapsed duration.
pub(crate) fn evaluate_wall_time(budget_secs: Option<u64>, elapsed: Duration) -> WallTimeGuard {
    match budget_secs {
        None => WallTimeGuard::Unbudgeted,
        Some(budget) => {
            let budget_ms = u128::from(budget).saturating_mul(1000);
            let elapsed_ms = elapsed.as_millis();
            if elapsed_ms > budget_ms {
                WallTimeGuard::Exceeded {
                    budget_secs: budget,
                    elapsed_ms,
                }
            } else {
                WallTimeGuard::WithinBudget
            }
        }
    }
}

/// Minimal glob matcher tailored to the patterns Track 3 surface and
/// Track 4 pack modules will declare. Intentionally narrow — no full glob
/// dialect — so the framework stays dependency-free:
///
/// * `*.ext`          — matches files whose basename ends with `.ext`.
/// * `**/*.ext`       — same as `*.ext` (any depth implied by basename).
/// * `prefix/*.ext`   — matches files under `prefix/` with extension `.ext`.
/// * `prefix/**`      — matches any file under `prefix/` (any depth).
/// * `exact/name`     — matches the literal workspace-relative path.
///
/// Path separators are normalised to `/` upstream by `walk_source_files`.
///
/// ## Patterns intentionally NOT supported
///
/// * `**/<name>` (extension-less, e.g. `**/Dockerfile`) — the `**/`
///   strip leaves a bare literal which then matches only the root path.
///   For any-depth name-only matching declare both `Dockerfile` and
///   `**/*/Dockerfile` explicitly, or use a `containers/**` prefix.
/// * Patterns whose directory components contain `.` (e.g.
///   `v1.2/*.sql`). The prefix split is order-sensitive on the first
///   `/*.`, so dotted directory segments produce nonsensical splits.
///   All call sites use compile-time `&'static str` patterns, so this is
///   a contract on pattern authors rather than a runtime check.
pub(crate) fn pattern_matches(pattern: &str, file: &str) -> bool {
    // Strip a leading `**/` since `*.ext` and `**/*.ext` mean the same
    // thing for the patterns we support.
    let pattern = pattern.strip_prefix("**/").unwrap_or(pattern);

    // Bare-extension or basename glob — anchor on the basename only.
    if let Some(ext) = pattern.strip_prefix("*.") {
        let basename = file.rsplit_once('/').map_or(file, |(_, name)| name);
        // Case-insensitive on the extension to match the surface scanners
        // (e.g. `is_shell_file`), so `RUN.SH` / `build.BaSh` stay in scope.
        return basename
            .rsplit_once('.')
            .is_some_and(|(_, file_ext)| file_ext.eq_ignore_ascii_case(ext));
    }

    // Prefix recursion: `dir/**` matches anything under `dir/`.
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return file == prefix || file.starts_with(&format!("{prefix}/"));
    }

    // Prefix + extension: `dir/*.ext`.
    if let Some((prefix, rest)) = pattern.split_once("/*.") {
        let Some(stripped) = file.strip_prefix(&format!("{prefix}/")) else {
            return false;
        };
        // Must live directly under prefix (no further slash) and end with
        // the declared extension.
        if stripped.contains('/') {
            return false;
        }
        return stripped
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case(rest));
    }

    // Literal match — exact workspace-relative path.
    pattern == file
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn empty_patterns_are_unguarded() {
        let result = evaluate_file_presence(&[], &files(&["src/main.rs"]));
        assert_eq!(result, FileShapeGuard::Unguarded);
        assert!(result.should_run());
    }

    #[test]
    fn extension_glob_matches_any_depth() {
        let result = evaluate_file_presence(
            &["*.sql"],
            &files(&[
                "src/main.rs",
                "db/migrations/0001_init.sql",
                "schemas/users.sql",
            ]),
        );
        assert_eq!(result, FileShapeGuard::Present(2));
    }

    #[test]
    fn absent_short_circuits() {
        let result = evaluate_file_presence(
            &["*.sql", "Dockerfile"],
            &files(&["src/main.rs", "README.md"]),
        );
        assert_eq!(result, FileShapeGuard::Absent);
        assert!(!result.should_run());
    }

    #[test]
    fn literal_filename_matches_basename_globally_only_at_root() {
        // Literal "Dockerfile" matches the exact workspace-relative path.
        // A nested Dockerfile under `images/` is NOT matched by the bare
        // literal — callers that want any-depth Dockerfile matching must
        // declare `Dockerfile` for the root and `**/Dockerfile` separately
        // (we explicitly do NOT support the latter yet; see
        // pattern_matches doc).
        let result = evaluate_file_presence(
            &["Dockerfile"],
            &files(&["Dockerfile", "images/runtime/Dockerfile"]),
        );
        assert_eq!(result, FileShapeGuard::Present(1));
    }

    #[test]
    fn prefix_extension_glob_is_single_depth() {
        let result = evaluate_file_presence(
            &["migrations/*.sql"],
            &files(&[
                "migrations/0001.sql",
                "migrations/sub/0002.sql",
                "other/0003.sql",
            ]),
        );
        assert_eq!(result, FileShapeGuard::Present(1));
    }

    #[test]
    fn prefix_recursion_matches_any_depth_under_dir() {
        let result = evaluate_file_presence(
            &["k8s/**"],
            &files(&[
                "k8s/base/deployment.yaml",
                "k8s/overlays/prod/kustomization.yaml",
                "src/main.rs",
            ]),
        );
        assert_eq!(result, FileShapeGuard::Present(2));
    }

    #[test]
    fn double_star_prefix_strip_is_idempotent_for_extension_globs() {
        let result = evaluate_file_presence(&["**/*.sql"], &files(&["db/0001.sql", "src/main.rs"]));
        assert_eq!(result, FileShapeGuard::Present(1));
    }

    #[test]
    fn extension_globs_match_case_insensitively() {
        // The file-presence guard aligns with the case-insensitive surface
        // scanners (e.g. `is_shell_file`): an upper/mixed-case extension is
        // still in scope, so the surface check is not silently skipped.
        assert!(pattern_matches("*.sh", "ci/RUN.SH"));
        assert!(pattern_matches("*.bash", "tools/build.BaSh"));
        assert!(pattern_matches("migrations/*.sql", "migrations/0001.SQL"));
        let result =
            evaluate_file_presence(&["*.sh", "*.bash"], &files(&["scripts/DEPLOY.SH", "a.rs"]));
        assert_eq!(result, FileShapeGuard::Present(1));
    }

    #[test]
    fn double_star_extension_less_name_pins_root_only_behaviour() {
        // Contract: `**/<name>` (no extension) is documented as NOT
        // supported and degrades to a literal root-only match after the
        // `**/` strip. This test pins that behaviour so a future author
        // who tries `**/Dockerfile` and finds it silently misses nested
        // Dockerfiles will discover the contract here rather than in
        // production.
        let result = evaluate_file_presence(
            &["**/Dockerfile"],
            &files(&["Dockerfile", "images/runtime/Dockerfile", "src/main.rs"]),
        );
        assert_eq!(result, FileShapeGuard::Present(1));
    }

    #[test]
    fn no_budget_is_unbudgeted() {
        let result = evaluate_wall_time(None, Duration::from_secs(10));
        assert_eq!(result, WallTimeGuard::Unbudgeted);
        assert!(result.timeout_reason().is_none());
    }

    #[test]
    fn within_budget_passes_silently() {
        let result = evaluate_wall_time(Some(5), Duration::from_millis(2_500));
        assert_eq!(result, WallTimeGuard::WithinBudget);
        assert!(result.timeout_reason().is_none());
    }

    #[test]
    fn exceeded_budget_carries_precise_overrun() {
        let result = evaluate_wall_time(Some(2), Duration::from_millis(2_500));
        assert_eq!(
            result,
            WallTimeGuard::Exceeded {
                budget_secs: 2,
                elapsed_ms: 2_500,
            }
        );
        let reason = result.timeout_reason().expect("reason for exceeded budget");
        assert!(
            reason.contains("2500 ms") && reason.contains("2s cap"),
            "reason should cite elapsed ms and budget seconds: {reason}"
        );
    }

    #[test]
    fn zero_budget_treats_any_elapsed_as_exceeded() {
        let result = evaluate_wall_time(Some(0), Duration::from_millis(1));
        assert!(matches!(result, WallTimeGuard::Exceeded { .. }));
    }

    #[test]
    fn elapsed_exactly_at_budget_is_within() {
        // Boundary: elapsed == budget is NOT exceeded — only strictly
        // greater. This prevents noisy timeout reasons from CI jitter that
        // hits the budget on the nose.
        let result = evaluate_wall_time(Some(1), Duration::from_secs(1));
        assert_eq!(result, WallTimeGuard::WithinBudget);
    }
}
