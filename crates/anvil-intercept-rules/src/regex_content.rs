//! INTR-005: `RegexContentRule` — a configurable list of regex patterns
//! matched against changed file content, the content-matching
//! counterpart to the INTR-004 path-deny rule.
//!
//! Patterns are compiled eagerly at construction so malformed patterns
//! surface as a typed [`RegexContentError::InvalidPattern`] rather than
//! failing on the hot path (the INTR-004 eager-compile precedent). The
//! `regex` crate's linear-time matching is the only ReDoS bound — no
//! additional execution budget is layered here.
//!
//! Determinism: "first registered pattern wins" — patterns are evaluated
//! in registration order, and the first pattern with a match reports its
//! earliest matching line. `Removed` changes and missing/binary content
//! always Allow.

use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
use regex::Regex;

use crate::{ChangeKind, InterceptRule, RuleDecision, RuleInput, mode_id_part, sanitise_id_part};

pub const REGEX_CONTENT_RULE_ID: &str = "regex-content";

/// Configuration accepted by [`RegexContentRule::new`].
///
/// Patterns use [`regex::Regex`] syntax and are matched per line of the
/// changed file's content. Empty pattern lists are accepted and produce
/// a no-op rule that always returns [`RuleDecision::Allow`].
#[derive(Debug, Clone, Default)]
pub struct RegexContentConfig {
    pub patterns: Vec<String>,
}

impl RegexContentConfig {
    #[must_use]
    pub fn new(patterns: Vec<String>) -> Self {
        Self { patterns }
    }
}

/// Construction-time error from compiling content patterns.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegexContentError {
    /// One of the configured patterns could not be parsed as a regex.
    #[error("invalid regex pattern '{pattern}': {reason}")]
    InvalidPattern { pattern: String, reason: String },
}

/// Hot-path rule that interrupts when a changed file's content matches
/// one of the configured regex patterns.
pub struct RegexContentRule {
    compiled: Vec<(String, Regex)>,
}

impl std::fmt::Debug for RegexContentRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegexContentRule")
            .field("patterns", &self.patterns().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

/// A single pattern match: the configured pattern and the 1-based line
/// it first matched on.
struct PatternMatch<'a> {
    pattern: &'a str,
    line: u32,
}

impl RegexContentRule {
    /// Compile `config.patterns` and return a ready rule. Errors are
    /// surfaced eagerly — callers configure once and can refuse to start
    /// if a pattern is malformed rather than failing silently on the hot
    /// path.
    pub fn new(config: RegexContentConfig) -> Result<Self, RegexContentError> {
        let compiled = config
            .patterns
            .into_iter()
            .map(|pattern| {
                Regex::new(&pattern)
                    .map(|regex| (pattern.clone(), regex))
                    .map_err(|err| RegexContentError::InvalidPattern {
                        pattern,
                        reason: err.to_string(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { compiled })
    }

    pub fn patterns(&self) -> impl Iterator<Item = &str> {
        self.compiled.iter().map(|(pattern, _)| pattern.as_str())
    }

    #[must_use]
    pub fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        self.diagnostics_with_limit(input, mode, usize::MAX)
    }

    #[must_use]
    pub fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        let path = input.path.to_string_lossy();
        self.matches_with_limit(input, limit)
            .into_iter()
            .map(|hit| match_to_diagnostic(path.as_ref(), &hit, mode.clone()))
            .collect()
    }

    /// Evaluate patterns in registration order; each matching pattern
    /// contributes its earliest matching line, up to `limit` matches.
    /// "First registered pattern wins" keeps operator-visible output
    /// stable across runs.
    fn matches_with_limit(&self, input: &RuleInput<'_>, limit: usize) -> Vec<PatternMatch<'_>> {
        if limit == 0 || self.compiled.is_empty() {
            return Vec::new();
        }
        if input.change_kind == ChangeKind::Removed {
            return Vec::new();
        }
        let Some(content) = input.content else {
            return Vec::new();
        };
        let Ok(content) = std::str::from_utf8(content) else {
            return Vec::new();
        };

        let mut matches = Vec::new();
        for (pattern, regex) in &self.compiled {
            let hit = content.lines().enumerate().find_map(|(index, line)| {
                if regex.is_match(line) {
                    u32::try_from(index + 1).ok()
                } else {
                    None
                }
            });
            if let Some(line) = hit {
                matches.push(PatternMatch { pattern, line });
                if matches.len() >= limit {
                    break;
                }
            }
        }
        matches
    }

    fn build_message(pattern: &str) -> String {
        format!("Content matches deny pattern '{pattern}'")
    }
}

impl InterceptRule for RegexContentRule {
    fn rule_id(&self) -> &str {
        REGEX_CONTENT_RULE_ID
    }

    fn needs_content(&self) -> bool {
        true
    }

    fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
        let matches = self.matches_with_limit(input, 1);
        let Some(first) = matches.first() else {
            return RuleDecision::Allow;
        };
        RuleDecision::interrupt_at(
            REGEX_CONTENT_RULE_ID,
            Self::build_message(first.pattern),
            first.line,
        )
    }

    fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        RegexContentRule::diagnostics(self, input, mode)
    }

    fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        RegexContentRule::diagnostics_with_limit(self, input, mode, limit)
    }
}

fn match_to_diagnostic(path: &str, hit: &PatternMatch<'_>, mode: Mode) -> Diagnostic {
    Diagnostic::new(
        format!(
            "diag_regex_content_{}_{}_{}_{}",
            mode_id_part(&mode),
            sanitise_id_part(path),
            hit.line,
            sanitise_id_part(hit.pattern),
        ),
        Severity::Error,
        RegexContentRule::build_message(hit.pattern),
        Location {
            file: path.to_string(),
            line: Some(hit.line),
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: REGEX_CONTENT_RULE_ID.to_string(),
            source_module: "anvil-intercept-rules::regex_content".to_string(),
        },
        mode,
    )
    .with_remediation_hint(
        "Remove the matching content, or adjust the configured regex patterns.",
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_kernel_types::{Category, Mode, Severity};

    use super::*;
    use crate::{ChangeKind, InterceptRule, RuleInput};

    fn rule(patterns: &[&str]) -> RegexContentRule {
        RegexContentRule::new(RegexContentConfig::new(
            patterns.iter().map(|p| (*p).to_string()).collect(),
        ))
        .expect("configured patterns must compile")
    }

    fn input<'a>(path: &'a Path, kind: ChangeKind, content: Option<&'a [u8]>) -> RuleInput<'a> {
        RuleInput {
            path,
            change_kind: kind,
            content,
        }
    }

    #[test]
    fn rule_id_is_regex_content() {
        assert_eq!(rule(&[]).rule_id(), REGEX_CONTENT_RULE_ID);
    }

    #[test]
    fn needs_content_is_true() {
        assert!(rule(&["forbidden"]).needs_content());
    }

    #[test]
    fn invalid_pattern_is_rejected_at_construction() {
        let err = RegexContentRule::new(RegexContentConfig::new(vec!["(unclosed".to_string()]))
            .expect_err("malformed regex must error");
        let RegexContentError::InvalidPattern { pattern, reason } = err;
        assert_eq!(pattern, "(unclosed");
        assert!(!reason.is_empty());
    }

    #[test]
    fn single_pattern_interrupts_with_correct_line() {
        let r = rule(&["FORBIDDEN_TOKEN"]);
        let path = Path::new("src/api.rs");
        let body = b"fn main() {}\nconst X: &str = \"FORBIDDEN_TOKEN\";\n";

        match r.evaluate(&input(path, ChangeKind::Modified, Some(body))) {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, REGEX_CONTENT_RULE_ID);
                assert_eq!(reason.line, std::num::NonZeroU32::new(2));
                assert!(reason.message.contains("FORBIDDEN_TOKEN"));
            }
            RuleDecision::Allow => panic!("matching content should interrupt"),
        }
    }

    #[test]
    fn first_registered_pattern_wins() {
        // Both patterns match; the first registered one is reported even
        // though the second matches an earlier line.
        let r = rule(&["beta", "alpha"]);
        let path = Path::new("src/api.rs");
        let body = b"alpha\nbeta\n";

        match r.evaluate(&input(path, ChangeKind::Modified, Some(body))) {
            RuleDecision::Interrupt(reason) => {
                assert!(reason.message.contains("'beta'"));
                assert_eq!(reason.line, std::num::NonZeroU32::new(2));
            }
            RuleDecision::Allow => panic!("expected interrupt"),
        }
    }

    #[test]
    fn clean_content_allows() {
        let r = rule(&["FORBIDDEN_TOKEN"]);
        let path = Path::new("src/api.rs");
        let body = b"fn main() {}\n";

        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Modified, Some(body))),
            RuleDecision::Allow
        );
    }

    #[test]
    fn empty_patterns_allow_all() {
        let r = rule(&[]);
        let path = Path::new("src/api.rs");
        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Created, Some(b"anything"))),
            RuleDecision::Allow
        );
    }

    #[test]
    fn removed_change_allows_even_if_matching() {
        let r = rule(&["FORBIDDEN_TOKEN"]);
        let path = Path::new("src/api.rs");
        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Removed, Some(b"FORBIDDEN_TOKEN"))),
            RuleDecision::Allow
        );
    }

    #[test]
    fn missing_or_binary_content_allows() {
        let r = rule(&["FORBIDDEN_TOKEN"]);
        let path = Path::new("src/api.rs");

        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Modified, None)),
            RuleDecision::Allow
        );
        assert_eq!(
            r.evaluate(&input(path, ChangeKind::Modified, Some(b"\xffFORBIDDEN_TOKEN"))),
            RuleDecision::Allow
        );
    }

    #[test]
    fn diagnostics_carry_policy_category_and_line() {
        let r = rule(&["FORBIDDEN_TOKEN"]);
        let path = Path::new("src/api.rs");
        let body = b"fn main() {}\nconst X: &str = \"FORBIDDEN_TOKEN\";\n";

        let diagnostics = r.diagnostics(
            &input(path, ChangeKind::Modified, Some(body)),
            &Mode::Unknown("pre-write".to_string()),
        );

        assert_eq!(diagnostics.len(), 1);
        let d = &diagnostics[0];
        assert_eq!(d.schema_version, "anvil.diagnostic.v1");
        assert_eq!(
            d.id,
            "diag_regex_content_pre_write_src_api_rs_2_forbidden_token"
        );
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.category, Category::Policy);
        assert_eq!(d.location.file, "src/api.rs");
        assert_eq!(d.location.line, Some(2));
        assert_eq!(d.source.rule_id, REGEX_CONTENT_RULE_ID);
        assert!(d.remediation_hint.is_some());
    }

    #[test]
    fn diagnostics_with_limit_zero_returns_empty() {
        let r = rule(&["FORBIDDEN_TOKEN"]);
        let path = Path::new("src/api.rs");
        let diagnostics = r.diagnostics_with_limit(
            &input(path, ChangeKind::Modified, Some(b"FORBIDDEN_TOKEN")),
            &Mode::Known(anvil_kernel_types::diagnostics::KnownMode::SaveTime),
            0,
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_report_each_matching_pattern() {
        let r = rule(&["alpha", "beta"]);
        let path = Path::new("src/api.rs");
        let body = b"alpha\nbeta\n";

        let diagnostics = r.diagnostics(
            &input(path, ChangeKind::Modified, Some(body)),
            &Mode::Unknown("pre-write".to_string()),
        );

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].location.line, Some(1));
        assert_eq!(diagnostics[1].location.line, Some(2));
    }

    #[test]
    fn rule_is_registry_composable() {
        let boxed: Box<dyn InterceptRule> = Box::new(rule(&["forbidden"]));
        assert_eq!(boxed.rule_id(), REGEX_CONTENT_RULE_ID);
    }
}
