//! INTR-004: `PathDenyListRule` — a configurable, glob-based deny list
//! that interrupts on writes to paths a project has declared off-limits
//! for agent sessions.
//!
//! Path-only rule: `needs_content()` is `false`, so the registry
//! (INTR-006) can avoid reading file content when this is the only
//! registered rule. Glob compilation happens once at construction;
//! evaluation is a single `GlobSet::matches` call — well inside the
//! hot-path latency budget.
//!
//! `Removed` changes always Allow: a deleted file is not a write, and
//! the rule's intent is to prevent agent *creation/modification* of
//! protected paths.

use std::path::Path;

use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::{ChangeKind, InterceptRule, RuleDecision, RuleInput, mode_id_part, sanitise_id_part};

pub const PATH_DENY_RULE_ID: &str = "path-deny";

/// Configuration accepted by [`PathDenyListRule::new`].
///
/// Patterns use [`globset::Glob`] syntax (gitignore-flavoured glob with
/// `**` recursive wildcard). Empty pattern lists are accepted and produce
/// a no-op rule that always returns [`RuleDecision::Allow`].
#[derive(Debug, Clone, Default)]
pub struct PathDenyConfig {
    pub patterns: Vec<String>,
}

impl PathDenyConfig {
    #[must_use]
    pub fn new(patterns: Vec<String>) -> Self {
        Self { patterns }
    }
}

/// Construction-time error from compiling deny patterns.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathDenyError {
    /// One of the configured patterns could not be parsed as a glob.
    #[error("invalid glob pattern '{pattern}': {reason}")]
    InvalidGlob { pattern: String, reason: String },
    /// `GlobSetBuilder::build()` rejected the assembled set after every
    /// individual pattern parsed cleanly. Rare in practice — `globset`
    /// surfaces builder-level failures only in resource-exhaustion or
    /// internal-invariant cases — but we keep the variant distinct so
    /// the error message is not silently emptied of the failing pattern.
    #[error("failed to build glob set from configured patterns: {reason}")]
    BuildFailed { reason: String },
}

/// Hot-path rule that interrupts when a changed file's path matches one
/// of the configured deny patterns.
pub struct PathDenyListRule {
    patterns: Vec<String>,
    globset: GlobSet,
}

impl std::fmt::Debug for PathDenyListRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathDenyListRule")
            .field("patterns", &self.patterns)
            .finish_non_exhaustive()
    }
}

impl PathDenyListRule {
    /// Compile `config.patterns` into a [`GlobSet`] and return a ready
    /// rule. Errors are surfaced eagerly — callers configure once and
    /// can refuse to start if a pattern is malformed rather than failing
    /// silently on the hot path.
    pub fn new(config: PathDenyConfig) -> Result<Self, PathDenyError> {
        let mut builder = GlobSetBuilder::new();
        for pattern in &config.patterns {
            let glob = Glob::new(pattern).map_err(|err| PathDenyError::InvalidGlob {
                pattern: pattern.clone(),
                reason: err.to_string(),
            })?;
            builder.add(glob);
        }
        let globset = builder.build().map_err(|err| PathDenyError::BuildFailed {
            reason: err.to_string(),
        })?;
        Ok(Self {
            patterns: config.patterns,
            globset,
        })
    }

    #[must_use]
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Returns the first matching pattern for `path`, if any. The
    /// "first" is defined as the earliest registered pattern, which
    /// keeps diagnostic output deterministic across runs. `globset`
    /// does not guarantee the ordering of indices in the returned
    /// `matches` vector, so we take the minimum directly rather than
    /// sorting the whole vector to pick the smallest index.
    fn first_match(&self, path: &Path) -> Option<&str> {
        if self.patterns.is_empty() {
            return None;
        }
        let idx = self.globset.matches(path).into_iter().min()?;
        self.patterns.get(idx).map(String::as_str)
    }

    fn build_message(pattern: &str, path: &Path) -> String {
        format!(
            "Path matches deny pattern '{}': {}",
            pattern,
            path.display()
        )
    }
}

impl InterceptRule for PathDenyListRule {
    fn rule_id(&self) -> &str {
        PATH_DENY_RULE_ID
    }

    fn needs_content(&self) -> bool {
        false
    }

    fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
        if input.change_kind == ChangeKind::Removed {
            return RuleDecision::Allow;
        }
        match self.first_match(input.path) {
            None => RuleDecision::Allow,
            Some(pattern) => {
                RuleDecision::interrupt(PATH_DENY_RULE_ID, Self::build_message(pattern, input.path))
            }
        }
    }

    fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        if limit == 0 {
            return Vec::new();
        }
        if input.change_kind == ChangeKind::Removed {
            return Vec::new();
        }
        let Some(pattern) = self.first_match(input.path) else {
            return Vec::new();
        };
        vec![path_deny_to_diagnostic(input.path, pattern, mode.clone())]
    }
}

fn path_deny_to_diagnostic(path: &Path, pattern: &str, mode: Mode) -> Diagnostic {
    let path_str = path.to_string_lossy();
    Diagnostic::new(
        format!(
            "diag_path_deny_{}_{}_{}",
            mode_id_part(&mode),
            sanitise_id_part(path_str.as_ref()),
            sanitise_id_part(pattern),
        ),
        Severity::Error,
        PathDenyListRule::build_message(pattern, path),
        Location {
            file: path_str.into_owned(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: PATH_DENY_RULE_ID.to_string(),
            source_module: "anvil-intercept-rules::path_deny".to_string(),
        },
        mode,
    )
    .with_remediation_hint("Remove the path from the deny list, or write to an allowed location.")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_kernel_types::{Category, Mode, Severity};

    use super::*;
    use crate::{ChangeKind, InterceptRule, InterruptReason, RuleInput};

    fn rule(patterns: &[&str]) -> PathDenyListRule {
        PathDenyListRule::new(PathDenyConfig::new(
            patterns.iter().map(|p| (*p).to_string()).collect(),
        ))
        .expect("configured patterns must compile")
    }

    fn input(path: &Path, kind: ChangeKind) -> RuleInput<'_> {
        RuleInput {
            path,
            change_kind: kind,
            content: None,
        }
    }

    #[test]
    fn rule_id_is_path_deny() {
        assert_eq!(rule(&[]).rule_id(), PATH_DENY_RULE_ID);
    }

    #[test]
    fn needs_content_is_false() {
        assert!(!rule(&["**/*.env"]).needs_content());
    }

    #[test]
    fn empty_patterns_allow_all() {
        let r = rule(&[]);
        let path = Path::new("any/file.rs");
        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Created)),
            RuleDecision::Allow
        );
        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Modified)),
            RuleDecision::Allow
        );
    }

    #[test]
    fn literal_path_match_interrupts() {
        let r = rule(&[".env"]);
        let path = Path::new(".env");
        match r.evaluate(&input(path, ChangeKind::Created)) {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, PATH_DENY_RULE_ID);
                assert!(reason.message.contains(".env"));
                assert_eq!(reason.line, None);
            }
            RuleDecision::Allow => panic!("literal match should interrupt"),
        }
    }

    #[test]
    fn glob_pattern_matches_nested_paths() {
        let r = rule(&["**/.env*"]);
        for raw in [".env", "config/.env", "deep/nested/dir/.env.local"] {
            let path = Path::new(raw);
            match r.evaluate(&input(path, ChangeKind::Modified)) {
                RuleDecision::Interrupt(reason) => {
                    assert!(reason.message.contains("**/.env*"), "pattern in message");
                    assert!(reason.message.contains(raw), "path in message");
                }
                RuleDecision::Allow => panic!("expected interrupt for {raw}"),
            }
        }
    }

    #[test]
    fn non_matching_path_allows() {
        let r = rule(&["**/.env*", "secrets/**"]);
        let path = Path::new("src/main.rs");
        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Modified)),
            RuleDecision::Allow
        );
    }

    #[test]
    fn removed_change_allows_even_if_matching() {
        let r = rule(&[".env"]);
        let path = Path::new(".env");
        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Removed)),
            RuleDecision::Allow
        );
    }

    #[test]
    fn invalid_glob_returns_error() {
        let err = PathDenyListRule::new(PathDenyConfig::new(vec!["[".to_string()]))
            .expect_err("malformed glob must error");
        match err {
            PathDenyError::InvalidGlob { pattern, reason } => {
                assert_eq!(pattern, "[");
                assert!(!reason.is_empty());
            }
            PathDenyError::BuildFailed { .. } => {
                panic!("expected InvalidGlob for a per-pattern parse failure")
            }
        }
    }

    #[test]
    fn first_registered_pattern_wins() {
        // Both patterns match `.env`; we report the earliest one for
        // deterministic operator-visible output.
        let r = rule(&["**/.env*", ".env"]);
        let path = Path::new(".env");
        match r.evaluate(&input(path, ChangeKind::Created)) {
            RuleDecision::Interrupt(reason) => {
                assert!(reason.message.contains("**/.env*"));
            }
            RuleDecision::Allow => panic!("expected interrupt"),
        }
    }

    #[test]
    fn diagnostics_carry_policy_category_and_path() {
        let r = rule(&["**/.env"]);
        let path = Path::new("config/.env");
        let diagnostics = r.diagnostics(
            &input(path, ChangeKind::Created),
            &Mode::Unknown("pre-write".to_string()),
        );

        assert_eq!(diagnostics.len(), 1);
        let d = &diagnostics[0];
        assert_eq!(d.schema_version, "anvil.diagnostic.v1");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.category, Category::Policy);
        assert_eq!(d.location.file, "config/.env");
        assert_eq!(d.location.line, None);
        assert_eq!(d.source.rule_id, PATH_DENY_RULE_ID);
        assert!(d.summary.contains("**/.env"));
        assert!(d.summary.contains("config/.env"));
    }

    #[test]
    fn diagnostics_with_limit_one_returns_one_for_match() {
        let r = rule(&["**/.env"]);
        let path = Path::new("config/.env");
        let diagnostics = r.diagnostics_with_limit(
            &input(path, ChangeKind::Modified),
            &Mode::Known(anvil_kernel_types::diagnostics::KnownMode::SaveTime),
            1,
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].source.rule_id, PATH_DENY_RULE_ID);
    }

    #[test]
    fn diagnostics_with_limit_zero_returns_empty() {
        let r = rule(&[".env"]);
        let path = Path::new(".env");
        let diagnostics = r.diagnostics_with_limit(
            &input(path, ChangeKind::Created),
            &Mode::Known(anvil_kernel_types::diagnostics::KnownMode::SaveTime),
            0,
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_for_removed_change_are_empty() {
        let r = rule(&[".env"]);
        let path = Path::new(".env");
        let diagnostics = r.diagnostics(
            &input(path, ChangeKind::Removed),
            &Mode::Known(anvil_kernel_types::diagnostics::KnownMode::SaveTime),
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn interrupt_reason_has_no_line_number() {
        let r = rule(&[".env"]);
        let path = Path::new(".env");
        let RuleDecision::Interrupt(InterruptReason { line, .. }) =
            r.evaluate(&input(path, ChangeKind::Created))
        else {
            panic!("expected interrupt");
        };
        assert_eq!(line, None);
    }
}
