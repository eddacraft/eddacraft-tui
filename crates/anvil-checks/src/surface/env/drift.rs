//! Env-surface drift: compare declared env expectations to the live tree.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::surface::env::parser::parse_env;
use crate::surface::env::suppression::resolve_file_header_suppression;

/// Rule ID for the SURFENV-004 drift check.
pub const SURFENV_004_RULE_ID: &str = "SURFENV-004";

/// One drift finding between an example template and a concrete env
/// file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftFinding {
    /// Path of the `.env.example` (or template) file.
    pub example_file: String,
    /// Path of the concrete `.env` / `.env.local` / `.env.<env>` file.
    pub concrete_file: String,
    /// Which side is missing the key.
    pub kind: DriftKind,
    /// The drifting key.
    pub key: String,
    /// 1-indexed source line of the key in the file that *has* it.
    /// Surfaces in the CLI message so the operator can jump to the
    /// definition rather than scrolling.
    pub line: usize,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Which side of the file pair is missing the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftKind {
    /// Key is set in the concrete file but not in the template.
    MissingFromExample,
    /// Key is documented in the template but not set in the concrete
    /// file.
    MissingFromConcrete,
}

/// Compare a template `.env.example` against a concrete `.env*` file
/// and report drift in either direction.
///
/// Both files are parsed with the SURFENV-001 parser, so they
/// inherit the same key validation, comment handling, and quote
/// rules. Parse warnings are intentionally not surfaced here — that's
/// the parser-level scanner's job, not the drift rule's.
#[must_use]
pub fn check_env_drift(
    example_path: &str,
    example_content: &str,
    concrete_path: &str,
    concrete_content: &str,
) -> Vec<DriftFinding> {
    let (example_entries, _) = parse_env(example_content);
    let (concrete_entries, _) = parse_env(concrete_content);

    let example_keys: BTreeSet<&str> = example_entries.iter().map(|e| e.key.as_str()).collect();
    let concrete_keys: BTreeSet<&str> = concrete_entries.iter().map(|e| e.key.as_str()).collect();

    let (example_suppressed, example_reason) =
        resolve_file_header_suppression(example_content, SURFENV_004_RULE_ID);
    let (concrete_suppressed, concrete_reason) =
        resolve_file_header_suppression(concrete_content, SURFENV_004_RULE_ID);

    let mut findings = Vec::new();

    // Concrete-only keys → missing from example. Find each entry's
    // line in the *concrete* file (the operator wants the location of
    // the key, not the missing slot).
    for entry in &concrete_entries {
        if example_keys.contains(entry.key.as_str()) {
            continue;
        }
        findings.push(DriftFinding {
            example_file: example_path.to_string(),
            concrete_file: concrete_path.to_string(),
            kind: DriftKind::MissingFromExample,
            key: entry.key.clone(),
            line: entry.line,
            // The example is the file that needs editing, so its
            // header directive controls suppression.
            suppressed: example_suppressed,
            suppression_reason: example_reason.clone(),
        });
    }

    // Example-only keys → missing from concrete.
    for entry in &example_entries {
        if concrete_keys.contains(entry.key.as_str()) {
            continue;
        }
        findings.push(DriftFinding {
            example_file: example_path.to_string(),
            concrete_file: concrete_path.to_string(),
            kind: DriftKind::MissingFromConcrete,
            key: entry.key.clone(),
            line: entry.line,
            suppressed: concrete_suppressed,
            suppression_reason: concrete_reason.clone(),
        });
    }

    // Stable order: by kind first (MissingFromExample before
    // MissingFromConcrete — that's the order callers iterate above),
    // then by key. Makes test assertions and CLI output predictable.
    findings.sort_by(|a, b| (a.kind as u8, a.key.as_str()).cmp(&(b.kind as u8, b.key.as_str())));

    findings
}

#[cfg(test)]
mod tests {
    use super::{DriftKind, SURFENV_004_RULE_ID, check_env_drift};

    #[test]
    fn no_findings_when_keys_match() {
        let example = "DATABASE_URL=\nAPI_KEY=\n";
        let concrete = "DATABASE_URL=postgres://localhost/dev\nAPI_KEY=local\n";
        let findings = check_env_drift(".env.example", example, ".env.local", concrete);
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn flags_concrete_only_key_as_missing_from_example() {
        let example = "DATABASE_URL=\n";
        let concrete = "DATABASE_URL=postgres://x\nNEW_FLAG=true\n";
        let findings = check_env_drift(".env.example", example, ".env.local", concrete);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, DriftKind::MissingFromExample);
        assert_eq!(findings[0].key, "NEW_FLAG");
        assert_eq!(findings[0].line, 2);
    }

    #[test]
    fn flags_example_only_key_as_missing_from_concrete() {
        let example = "DATABASE_URL=\nAPI_KEY=\n";
        let concrete = "DATABASE_URL=postgres://x\n";
        let findings = check_env_drift(".env.example", example, ".env.local", concrete);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, DriftKind::MissingFromConcrete);
        assert_eq!(findings[0].key, "API_KEY");
        assert_eq!(findings[0].line, 2, "line points at example definition");
    }

    #[test]
    fn drift_in_both_directions() {
        let example = "OLD_FLAG=\nKEEP=\n";
        let concrete = "KEEP=value\nNEW_FLAG=value\n";
        let findings = check_env_drift(".env.example", example, ".env.local", concrete);
        assert_eq!(findings.len(), 2);
        // Stable ordering: MissingFromExample first, then MissingFromConcrete.
        assert_eq!(findings[0].kind, DriftKind::MissingFromExample);
        assert_eq!(findings[0].key, "NEW_FLAG");
        assert_eq!(findings[1].kind, DriftKind::MissingFromConcrete);
        assert_eq!(findings[1].key, "OLD_FLAG");
    }

    #[test]
    fn directive_in_example_suppresses_missing_from_example_findings() {
        let example = format!(
            "# @anvil-ignore {SURFENV_004_RULE_ID} -- template intentionally lean\nDATABASE_URL=\n"
        );
        let concrete = "DATABASE_URL=postgres://x\nNEW_FLAG=true\n";
        let findings = check_env_drift(".env.example", &example, ".env.local", concrete);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].suppressed);
        assert_eq!(
            findings[0].suppression_reason.as_deref(),
            Some("template intentionally lean")
        );
    }

    #[test]
    fn directive_in_concrete_suppresses_missing_from_concrete_findings() {
        let example = "OLD_FLAG=\nKEEP=\n";
        let concrete =
            format!("# @anvil-ignore {SURFENV_004_RULE_ID} -- legacy concrete\nKEEP=value\n");
        let findings = check_env_drift(".env.example", example, ".env.local", &concrete);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, DriftKind::MissingFromConcrete);
        assert!(findings[0].suppressed);
    }

    #[test]
    fn directive_in_concrete_does_not_suppress_missing_from_example() {
        // Cross-direction immunity: a directive in the *concrete* file
        // must not silence findings about the *example* — that's the
        // point of having two kinds.
        let example = "DATABASE_URL=\n";
        let concrete = format!(
            "# @anvil-ignore {SURFENV_004_RULE_ID} -- concrete suppressed\nDATABASE_URL=x\nNEW_FLAG=y\n"
        );
        let findings = check_env_drift(".env.example", example, ".env.local", &concrete);
        let missing_from_example: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == DriftKind::MissingFromExample)
            .collect();
        assert_eq!(missing_from_example.len(), 1);
        assert!(!missing_from_example[0].suppressed);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored_for_drift() {
        let example = "# leading comment\n\nFOO=\n# trailing comment\n";
        let concrete = "FOO=set\n";
        let findings = check_env_drift(".env.example", example, ".env.local", concrete);
        assert!(findings.is_empty());
    }
}
