//! Showcase mode: curated example findings for when a filtered scan returns
//! zero warnings. Each finding is prefixed with `[Example]` so the render
//! layer can distinguish showcase findings from real ones.

use super::discovery::{Finding, FindingSeverity, FindingSource};

/// Return curated example findings covering different check types.
///
/// Used when the discovery scan finds nothing — the user's project is clean,
/// but we still want to show what Anvil is capable of detecting.
pub fn showcase_findings() -> Vec<Finding> {
    vec![
        Finding {
            file: "src/services/auth.rs".to_string(),
            line: Some(42),
            severity: FindingSeverity::Error,
            source: FindingSource::Secret,
            title: "[Example] Hard-coded API key detected".to_string(),
            message: "A string matching the pattern `sk_live_*` was found in \
                      source code. Secrets committed to version control can be \
                      extracted from history even after removal."
                .to_string(),
            suggestion: "Move the value to an environment variable or a secrets \
                         manager and reference it at runtime."
                .to_string(),
            warning_id: None,
        },
        Finding {
            file: "src/handlers/api.rs".to_string(),
            line: Some(87),
            severity: FindingSeverity::Warning,
            source: FindingSource::AntiPattern,
            title: "[Example] TODO left in production code".to_string(),
            message: "An unresolved TODO comment was found. Outstanding TODOs \
                      in shipped code indicate incomplete work that may affect \
                      reliability."
                .to_string(),
            suggestion: "Resolve the TODO or convert it to a tracked issue \
                         before merging."
                .to_string(),
            warning_id: None,
        },
        Finding {
            file: "src/db/queries.rs".to_string(),
            line: Some(15),
            severity: FindingSeverity::Warning,
            source: FindingSource::Architecture,
            title: "[Example] Cross-layer import violation".to_string(),
            message: "Module `db::queries` imports directly from \
                      `handlers::api`, bypassing the service layer. This \
                      creates a circular dependency between the data and \
                      presentation layers."
                .to_string(),
            suggestion: "Route the dependency through the service layer or \
                         extract a shared contract."
                .to_string(),
            warning_id: None,
        },
        Finding {
            file: "src/models/user.rs".to_string(),
            line: Some(3),
            severity: FindingSeverity::Info,
            source: FindingSource::AntiPattern,
            title: "[Example] Inconsistent naming convention".to_string(),
            message: "Struct field `userId` uses camelCase, but the project \
                      convention is snake_case. Inconsistent naming makes \
                      code harder to navigate."
                .to_string(),
            suggestion: "Rename to `user_id` to match the project convention.".to_string(),
            warning_id: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::super::discovery::{FindingSeverity, FindingSource};
    use super::*;

    #[test]
    fn returns_exactly_four_findings() {
        let findings = showcase_findings();
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn all_titles_start_with_example_prefix() {
        for finding in &showcase_findings() {
            assert!(
                finding.title.starts_with("[Example] "),
                "title {:?} missing [Example] prefix",
                finding.title,
            );
        }
    }

    #[test]
    fn covers_all_finding_sources() {
        let findings = showcase_findings();
        let sources: Vec<FindingSource> = findings.iter().map(|f| f.source).collect();

        assert!(
            sources.contains(&FindingSource::Secret),
            "missing Secret source",
        );
        assert!(
            sources.contains(&FindingSource::AntiPattern),
            "missing AntiPattern source",
        );
        assert!(
            sources.contains(&FindingSource::Architecture),
            "missing Architecture source",
        );
    }

    #[test]
    fn covers_error_warning_and_info_severities() {
        let findings = showcase_findings();
        let severities: Vec<FindingSeverity> = findings.iter().map(|f| f.severity).collect();

        assert!(
            severities.contains(&FindingSeverity::Error),
            "missing Error severity",
        );
        assert!(
            severities.contains(&FindingSeverity::Warning),
            "missing Warning severity",
        );
        assert!(
            severities.contains(&FindingSeverity::Info),
            "missing Info severity",
        );
    }

    #[test]
    fn all_fields_are_non_empty() {
        for finding in &showcase_findings() {
            assert!(!finding.file.is_empty(), "file is empty");
            assert!(finding.line.is_some(), "line is None");
            assert!(!finding.title.is_empty(), "title is empty");
            assert!(!finding.message.is_empty(), "message is empty");
            assert!(!finding.suggestion.is_empty(), "suggestion is empty");
        }
    }
}
