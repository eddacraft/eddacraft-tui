use regex::Regex;

use crate::antipattern::patterns::{get_default_patterns, get_enabled_patterns, get_pattern};
use crate::antipattern::types::{
    AntiPattern, ArtifactKind, Location, Suppression, SuppressionScope, Warning, WarningCategory,
    create_warning_fingerprint,
};

const LEGACY_JS_TS_EXTENSIONS: [&str; 6] = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub patterns: Option<Vec<String>>,
    pub include_opt_in: bool,
}

/// Unit of content passed to the scanner. `reference` identifies the source
/// of `content` — a file path for `source`, a PR number or URL for
/// `pr-description`, a commit SHA for `commit-message`, a session id for
/// `agent-output`. It surfaces verbatim on resulting warnings via
/// `location.file` so operators can trace the warning back to its origin.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub kind: ArtifactKind,
    pub reference: String,
    pub content: String,
}

impl Artifact {
    #[must_use]
    pub fn source(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            kind: ArtifactKind::Source,
            reference: path.into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub file: String,
    pub artifact_type: ArtifactKind,
    pub warnings: Vec<Warning>,
    pub patterns_checked: Vec<String>,
}

#[derive(Debug, Clone)]
struct PreparedPattern {
    pattern: AntiPattern,
    primary_regex: Option<Regex>,
    secondary_regex: Option<Regex>,
}

fn get_patterns_to_check(options: &ScanOptions) -> Vec<AntiPattern> {
    if let Some(pattern_ids) = &options.patterns
        && !pattern_ids.is_empty()
    {
        return pattern_ids
            .iter()
            .filter_map(|id| get_pattern(id))
            .collect();
    }

    if options.include_opt_in {
        get_enabled_patterns()
    } else {
        get_default_patterns()
    }
}

fn matches_file_extension(file_path: &str, file_extensions: &[String]) -> bool {
    let Some(dot_index) = file_path.rfind('.') else {
        return false;
    };

    let extension = file_path[dot_index..].to_ascii_lowercase();
    file_extensions
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&extension))
}

fn normalise_path(file_path: &str) -> String {
    file_path.replace('\\', "/")
}

fn basename(file_path: &str) -> &str {
    file_path.rsplit('/').next().unwrap_or(file_path)
}

fn glob_to_regex(pattern: &str) -> Option<Regex> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();

    if pattern.starts_with("**/") {
        regex.push_str("(?:.*/)?");
        let _ = chars.next();
        let _ = chars.next();
        let _ = chars.next();
    }

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    let _ = chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }

    regex.push('$');
    Regex::new(&regex).ok()
}

fn glob_match(file_path: &str, pattern: &str, match_base: bool) -> bool {
    let normalised = normalise_path(file_path);
    let target = if match_base && !pattern.contains('/') {
        basename(&normalised)
    } else {
        normalised.as_str()
    };

    glob_to_regex(pattern).is_some_and(|regex| regex.is_match(target))
}

fn is_file_allowlisted(file_path: &str, allowlist: &[String]) -> bool {
    allowlist
        .iter()
        .any(|pattern| glob_match(file_path, pattern, true))
}

fn create_warning_from_match(
    pattern: &AntiPattern,
    file_path: &str,
    line: usize,
    column: usize,
    suppressed: Option<Suppression>,
) -> Warning {
    let mut warning = Warning {
        id: pattern.id.clone(),
        fingerprint: None,
        category: WarningCategory::AntiPattern,
        severity: pattern.severity,
        confidence: pattern.confidence,
        title: pattern.title.clone(),
        message: format!("Found {} at line {line}", pattern.name),
        explanation: pattern.explanation.clone(),
        suggestion: pattern.suggestion.clone(),
        nudge: pattern.nudge.clone(),
        location: Location {
            file: file_path.to_string(),
            line,
            column: Some(column),
            end_line: None,
            end_column: None,
        },
        pattern: Some(pattern.id.clone()),
        suppressed,
        family: pattern.family.clone(),
        definition_ref: pattern.definition_ref.clone(),
        spectrum_position: pattern.spectrum_position,
    };
    warning.fingerprint = Some(create_warning_fingerprint(&warning));
    warning
}

fn parse_suppression(line: &str) -> Option<(String, String)> {
    let regex =
        Regex::new(r"(?://|/\*|#|<!--|--)\s*@anvil-ignore\s+(AP-\d{3})(?:\s*--\s*(.+))?").ok()?;
    let captures = regex.captures(line)?;
    let id = captures.get(1).map_or("", |capture| capture.as_str());
    let reason = captures
        .get(2)
        .map_or("No reason provided", |capture| capture.as_str())
        .trim();
    Some((id.to_string(), reason.to_string()))
}

fn suppression_for_line(
    lines: &[&str],
    line_number: usize,
    pattern_id: &str,
) -> Option<Suppression> {
    if line_number <= 1 {
        return None;
    }
    let previous_line = lines[line_number - 2];
    let (id, reason) = parse_suppression(previous_line)?;
    if id != pattern_id {
        return None;
    }

    Some(Suppression {
        reason,
        author: None,
        timestamp: None,
        scope: SuppressionScope::Line,
    })
}

fn prepare_pattern(pattern: AntiPattern) -> PreparedPattern {
    match pattern.id.as_str() {
        "AP-001" => PreparedPattern {
            pattern,
            primary_regex: Regex::new(r"/\*\s*eslint-disable\s*\*/").ok(),
            secondary_regex: Regex::new(r"//\s*eslint-disable\s*$").ok(),
        },
        "AP-009" => PreparedPattern {
            pattern,
            primary_regex: Regex::new(r"<script(?:\s[^>]*)?>").ok(),
            secondary_regex: Regex::new(r"^\s*</script>").ok(),
        },
        _ => PreparedPattern {
            primary_regex: Regex::new(&pattern.regex).ok(),
            secondary_regex: None,
            pattern,
        },
    }
}

fn find_match_columns(prepared: &PreparedPattern, line: &str) -> Vec<usize> {
    match prepared.pattern.id.as_str() {
        "AP-001" => {
            let mut columns = Vec::new();
            if let Some(regex) = &prepared.primary_regex {
                columns.extend(regex.find_iter(line).map(|matched| matched.start()));
            }
            if let Some(regex) = &prepared.secondary_regex {
                columns.extend(regex.find_iter(line).map(|matched| matched.start()));
            }
            columns.sort_unstable();
            columns
        }
        "AP-009" => {
            let mut columns = Vec::new();
            let Some(script_regex) = &prepared.primary_regex else {
                return columns;
            };

            for script_match in script_regex.find_iter(line) {
                let trailing = &line[script_match.end()..];
                let is_empty_script = prepared
                    .secondary_regex
                    .as_ref()
                    .is_some_and(|close_regex| close_regex.is_match(trailing));
                if !is_empty_script {
                    columns.push(script_match.start());
                }
            }

            columns
        }
        _ => prepared
            .primary_regex
            .as_ref()
            .map_or_else(Vec::new, |regex| {
                regex
                    .find_iter(line)
                    .map(|matched| matched.start())
                    .collect()
            }),
    }
}

fn pattern_runs_on_artifact(pattern: &AntiPattern, kind: ArtifactKind) -> bool {
    // Compiled `.anvil` patterns declare `targets`; skip if the artifact's
    // kind is not listed. Legacy patterns (hardcoded `PATTERN_DEFS`) have
    // `targets: None` and are treated as source-only, preserving
    // pre-ANVFMT-008 behaviour.
    match &pattern.targets {
        Some(targets) => targets.iter().any(|t| t == kind.as_str()),
        None => kind == ArtifactKind::Source,
    }
}

/// Scan an artifact for anti-patterns.
///
/// The scanner filters the pattern catalogue to the subset whose detection
/// is meaningful for the artifact's kind:
///   - Compiled `.anvil` patterns carry an explicit `targets` list —
///     artifacts with a kind outside that list are skipped.
///   - Legacy hardcoded patterns have no `targets` and are treated as
///     source-only.
///   - File-extension and allowlist filtering only applies to `source`
///     artifacts; for PR bodies / commit messages / agent output the
///     `reference` is not a path.
#[must_use]
pub fn scan_artifact(artifact: &Artifact, options: Option<&ScanOptions>) -> ScanResult {
    let scan_options = options.cloned().unwrap_or_default();
    let patterns = get_patterns_to_check(&scan_options);
    let prepared_patterns = patterns
        .into_iter()
        .map(prepare_pattern)
        .collect::<Vec<_>>();
    let lines = artifact.content.split('\n').collect::<Vec<_>>();
    let is_source = artifact.kind == ArtifactKind::Source;
    let mut warnings = Vec::new();

    for prepared in &prepared_patterns {
        if !pattern_runs_on_artifact(&prepared.pattern, artifact.kind) {
            continue;
        }

        if is_source {
            let effective_extensions =
                if let Some(pattern_extensions) = &prepared.pattern.file_extensions {
                    Some(pattern_extensions.clone())
                } else if prepared.pattern.all_file_types {
                    None
                } else {
                    Some(
                        LEGACY_JS_TS_EXTENSIONS
                            .iter()
                            .map(ToString::to_string)
                            .collect(),
                    )
                };

            if let Some(extensions) = &effective_extensions
                && !matches_file_extension(&artifact.reference, extensions)
            {
                continue;
            }
            if is_file_allowlisted(&artifact.reference, &prepared.pattern.allowlist) {
                continue;
            }
        }

        for (line_index, line) in lines.iter().enumerate() {
            let line_number = line_index + 1;
            let columns = find_match_columns(prepared, line);
            for column in columns {
                let suppressed = if is_source {
                    suppression_for_line(&lines, line_number, &prepared.pattern.id)
                } else {
                    None
                };
                warnings.push(create_warning_from_match(
                    &prepared.pattern,
                    &artifact.reference,
                    line_number,
                    column,
                    suppressed,
                ));
            }
        }
    }

    ScanResult {
        file: artifact.reference.clone(),
        artifact_type: artifact.kind,
        warnings,
        patterns_checked: prepared_patterns
            .iter()
            .map(|prepared| prepared.pattern.id.clone())
            .collect(),
    }
}

/// Scan a source file's content for anti-patterns. Backward-compatible
/// wrapper around `scan_artifact` with `kind: Source`.
#[must_use]
pub fn scan_file(file_path: &str, content: &str, options: Option<&ScanOptions>) -> ScanResult {
    scan_artifact(&Artifact::source(file_path, content), options)
}

/// Scan multiple artifacts for anti-patterns.
#[must_use]
pub fn scan_artifacts(artifacts: &[Artifact], options: Option<&ScanOptions>) -> Vec<ScanResult> {
    artifacts
        .iter()
        .map(|artifact| scan_artifact(artifact, options))
        .collect()
}

#[must_use]
pub fn scan_files(files: &[(&str, &str)], options: Option<&ScanOptions>) -> Vec<ScanResult> {
    files
        .iter()
        .map(|(path, content)| scan_file(path, content, options))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::antipattern::scanner::{ScanOptions, scan_file};

    #[test]
    fn scans_default_patterns_only() {
        let content = "const value: any = input;\nconsole.log(value);";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings.iter().any(|warning| warning.id == "AP-003"));
        assert!(!result.warnings.iter().any(|warning| warning.id == "AP-007"));
    }

    #[test]
    fn include_opt_in_detects_console_pattern() {
        let options = ScanOptions {
            patterns: None,
            include_opt_in: true,
        };
        let result = scan_file("src/app.ts", "console.log('x')", Some(&options));
        assert!(result.warnings.iter().any(|warning| warning.id == "AP-007"));
    }

    #[test]
    fn filters_by_requested_pattern_ids() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-006".to_string()]),
            include_opt_in: true,
        };
        let content = "try { x(); } catch (e) {}\nconst v: any = x;";
        let result = scan_file("src/app.ts", content, Some(&options));

        assert_eq!(result.patterns_checked, vec!["AP-006"]);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].id, "AP-006");
    }

    #[test]
    fn applies_legacy_js_ts_extension_defaults() {
        let js_result = scan_file("src/a.ts", "const v: any = input;", None);
        let html_result = scan_file("src/a.html", "const v: any = input;", None);

        assert_eq!(js_result.warnings.len(), 1);
        assert!(html_result.warnings.is_empty());
    }

    #[test]
    fn html_patterns_only_scan_html_files() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-008".to_string()]),
            include_opt_in: true,
        };
        let html_result = scan_file("templates/page.html", "<div style=\"x\">", Some(&options));
        let ts_result = scan_file("src/page.ts", "<div style=\"x\">", Some(&options));

        assert_eq!(html_result.warnings.len(), 1);
        assert!(ts_result.warnings.is_empty());
    }

    #[test]
    fn allowlist_skips_paths_matching_glob_rules() {
        let result = scan_file("src/foo/__tests__/sample.ts", "const x: any = 1;", None);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn suppression_on_previous_line_marks_warning_as_suppressed() {
        let content = "// @anvil-ignore AP-003 -- legacy contract\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        let warning = &result.warnings[0];
        assert!(warning.suppressed.is_some());
        if let Some(suppression) = &warning.suppressed {
            assert_eq!(suppression.reason, "legacy contract");
        }
    }

    #[test]
    fn suppression_does_not_apply_to_different_pattern() {
        let content = "// @anvil-ignore AP-001\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].suppressed.is_none());
    }

    #[test]
    fn finds_multiple_matches_per_line() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-002".to_string()]),
            include_opt_in: true,
        };
        let content = "/* eslint-disable foo */ // eslint-disable-next-line bar";
        let result = scan_file("src/app.ts", content, Some(&options));

        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn handles_ap001_negative_lookahead_semantics() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-001".to_string()]),
            include_opt_in: true,
        };
        let content = "// eslint-disable-next-line no-console\n// eslint-disable";
        let result = scan_file("src/app.ts", content, Some(&options));

        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].location.line, 2);
    }

    #[test]
    fn suppression_requires_comment_syntax() {
        let content =
            "console.log('@anvil-ignore AP-003 -- not a comment');\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].suppressed.is_none());
    }

    #[test]
    fn suppression_works_with_hash_comment() {
        let content = "# @anvil-ignore AP-003 -- legacy\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].suppressed.is_some());
    }

    #[test]
    fn handles_ap009_negative_lookahead_semantics() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-009".to_string()]),
            include_opt_in: true,
        };
        let content = "<script></script>\n<script>let x = 1;</script>";
        let result = scan_file("templates/page.html", content, Some(&options));

        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].location.line, 2);
    }

    #[test]
    fn warning_carries_family_provenance_from_pattern() {
        use crate::antipattern::registry_loader::{
            LoadRegistryOptions, load_registry_patterns, reset_registry_cache,
        };
        use std::path::PathBuf;

        reset_registry_cache();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let registry = manifest
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("patterns/compiled/registry.json");

        let registry_patterns = load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(registry),
        });
        let ap003 = registry_patterns
            .into_iter()
            .find(|p| p.id == "AP-003")
            .expect("AP-003 in registry");
        assert_eq!(ap003.family.as_deref(), Some("type-system-evasion"));

        let warning = super::create_warning_from_match(&ap003, "src/app.ts", 1, 0, None);
        assert_eq!(warning.family.as_deref(), Some("type-system-evasion"));
        assert!(
            warning.definition_ref.is_some(),
            "definition_ref should propagate"
        );
        assert_eq!(warning.spectrum_position, Some(1));
    }

    #[test]
    fn warning_from_legacy_pattern_has_no_family_provenance() {
        let legacy =
            crate::antipattern::patterns::get_pattern("AP-001").expect("legacy AP-001 available");
        let warning = super::create_warning_from_match(&legacy, "src/app.ts", 1, 0, None);
        assert!(warning.family.is_none());
        assert!(warning.definition_ref.is_none());
        assert!(warning.spectrum_position.is_none());
    }

    // ---- scan_artifact: artifact-aware filtering ---------------------------

    #[test]
    fn scan_artifact_source_matches_legacy_scan_file() {
        use super::{Artifact, scan_artifact};
        use crate::antipattern::types::ArtifactKind;

        let content = "const v: any = input;";
        let via_file = scan_file("src/app.ts", content, None);
        let via_artifact = scan_artifact(
            &Artifact {
                kind: ArtifactKind::Source,
                reference: "src/app.ts".to_string(),
                content: content.to_string(),
            },
            None,
        );

        assert_eq!(via_file.warnings.len(), via_artifact.warnings.len());
        assert_eq!(via_artifact.artifact_type, ArtifactKind::Source);
    }

    #[test]
    fn scan_artifact_skips_legacy_pattern_on_non_source() {
        use super::{Artifact, scan_artifact};
        use crate::antipattern::types::ArtifactKind;

        // AP-001 is a legacy (hardcoded) pattern — no targets — so it must
        // not fire against a PR description regardless of content.
        let options = ScanOptions {
            patterns: Some(vec!["AP-001".to_string()]),
            include_opt_in: true,
        };
        let artifact = Artifact {
            kind: ArtifactKind::PrDescription,
            reference: "PR#42".to_string(),
            content: "/* eslint-disable */".to_string(),
        };

        let result = scan_artifact(&artifact, Some(&options));
        assert!(
            result.warnings.is_empty(),
            "legacy pattern fired on PR description: {:?}",
            result.warnings
        );
        assert_eq!(result.artifact_type, ArtifactKind::PrDescription);
    }

    #[test]
    fn scan_artifact_respects_registry_pattern_targets() {
        use super::{Artifact, scan_artifact};
        use crate::antipattern::registry_loader::{
            LoadRegistryOptions, load_registry_patterns, reset_registry_cache,
        };
        use crate::antipattern::types::ArtifactKind;
        use std::path::PathBuf;

        reset_registry_cache();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let registry_path = manifest
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("patterns/compiled/registry.json");

        let patterns = load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(registry_path),
        });
        // Pick an AP-* pattern with targets = ["source"] only — one such is
        // AP-003 (any type usage). We prove that a pr-description artifact
        // does not trigger AP-003 even with matching content.
        let ap003 = patterns
            .iter()
            .find(|p| p.id == "AP-003")
            .cloned()
            .expect("AP-003 registry pattern");
        assert_eq!(
            ap003.targets.as_deref(),
            Some(vec!["source".to_string()].as_slice()),
            "AP-003 should target source only"
        );

        let runs_on_pr = super::pattern_runs_on_artifact(&ap003, ArtifactKind::PrDescription);
        assert!(
            !runs_on_pr,
            "source-only pattern must not run on pr-description"
        );
        let runs_on_source = super::pattern_runs_on_artifact(&ap003, ArtifactKind::Source);
        assert!(runs_on_source, "source-only pattern must run on source");

        // End-to-end: scan_artifact returns no warnings for the source-only
        // pattern against a pr-description, regardless of content.
        let result = scan_artifact(
            &Artifact {
                kind: ArtifactKind::PrDescription,
                reference: "PR#1".to_string(),
                content: "const x: any = 1;".to_string(),
            },
            Some(&ScanOptions {
                patterns: Some(vec!["AP-003".to_string()]),
                include_opt_in: true,
            }),
        );
        assert!(result.warnings.is_empty());
    }
}
