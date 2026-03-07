use regex::Regex;

use crate::antipattern::patterns::{get_default_patterns, get_enabled_patterns, get_pattern};
use crate::antipattern::types::{
    AntiPattern, Location, Suppression, SuppressionScope, Warning, WarningCategory,
    create_warning_fingerprint,
};

const LEGACY_JS_TS_EXTENSIONS: [&str; 6] = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub patterns: Option<Vec<String>>,
    pub include_opt_in: bool,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub file: String,
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
    };
    warning.fingerprint = Some(create_warning_fingerprint(&warning));
    warning
}

fn parse_suppression(line: &str) -> Option<(String, String)> {
    let regex = Regex::new(r"@anvil-ignore\s+(AP-\d{3})(?:\s*--\s*(.+))?").ok()?;
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

#[must_use]
pub fn scan_file(file_path: &str, content: &str, options: Option<&ScanOptions>) -> ScanResult {
    let scan_options = options.cloned().unwrap_or_default();
    let patterns = get_patterns_to_check(&scan_options);
    let prepared_patterns = patterns
        .into_iter()
        .map(prepare_pattern)
        .collect::<Vec<_>>();
    let lines = content.split('\n').collect::<Vec<_>>();
    let mut warnings = Vec::new();

    for prepared in &prepared_patterns {
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
            && !matches_file_extension(file_path, extensions)
        {
            continue;
        }
        if is_file_allowlisted(file_path, &prepared.pattern.allowlist) {
            continue;
        }

        for (line_index, line) in lines.iter().enumerate() {
            let line_number = line_index + 1;
            let columns = find_match_columns(prepared, line);
            for column in columns {
                let suppressed = suppression_for_line(&lines, line_number, &prepared.pattern.id);
                warnings.push(create_warning_from_match(
                    &prepared.pattern,
                    file_path,
                    line_number,
                    column,
                    suppressed,
                ));
            }
        }
    }

    ScanResult {
        file: file_path.to_string(),
        warnings,
        patterns_checked: prepared_patterns
            .iter()
            .map(|prepared| prepared.pattern.id.clone())
            .collect(),
    }
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
}
