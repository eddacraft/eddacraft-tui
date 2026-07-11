use std::path::Path;
use std::sync::LazyLock;

use anvil_tui::surfaces::doctor::DiagnosticCheck;
use anvil_tui::surfaces::fix_request::FixRequest;
use anvil_tui::surfaces::tutorial::first_win::FixPreview;
use regex::Regex;

static CONSOLE_STATEMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*console\.(log|error)\s*\(.*\)\s*;?\s*$")
        .expect("console statement regex compiles")
});

static ANY_ANNOTATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(:\s*)any\b").expect("annotation regex compiles"));

/// Outcome of a shared interactive fix request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixOutcome {
    Applied { summary: String },
    Refused { reason: String },
    Failed { reason: String },
}

pub fn apply_fix_request(
    request: &FixRequest,
    doctor_checks: Option<&mut [DiagnosticCheck]>,
) -> FixOutcome {
    match request {
        FixRequest::DoctorCheck { index } => {
            let Some(checks) = doctor_checks else {
                return FixOutcome::Failed {
                    reason: "doctor fix requested without doctor check state".to_string(),
                };
            };
            if *index >= checks.len() {
                return FixOutcome::Refused {
                    reason: format!("Doctor check index {index} is out of range"),
                };
            }
            crate::commands::doctor::apply_fix_at(checks, *index);
            FixOutcome::Applied {
                summary: "Applied doctor auto-fix".to_string(),
            }
        }
        FixRequest::AntiPatternWarning {
            file,
            line,
            warning_id,
        } => apply_line_transform(file, *line, |source| {
            apply_antipattern_fix(source, warning_id).map_err(|()| {
                format!("No deterministic auto-fix available for {warning_id} on this line")
            })
        }),
        FixRequest::AuditConsoleStatement { file, line } => {
            apply_line_removal(file, *line, |source| {
                if is_auto_fixable_console_statement(source) {
                    Ok(())
                } else {
                    Err("Console statement is not a standalone deterministic auto-fix".to_string())
                }
            })
        }
    }
}

pub fn is_auto_fixable_console_statement(line: &str) -> bool {
    CONSOLE_STATEMENT_RE.is_match(line)
}

/// WOW-005: compute the exact line change [`apply_fix_request`] would write,
/// without writing anything. The transform is the same function the apply
/// path runs, so the previewed diff is byte-for-byte what a consented apply
/// produces. Returns `None` when no deterministic preview exists (unsupported
/// request kind, unreadable file, out-of-range line, no transform for the
/// line) — the caller skips the first-win offer rather than showing a diff it
/// cannot honour.
pub fn preview_fix_request(request: &FixRequest) -> Option<FixPreview> {
    let FixRequest::AntiPatternWarning {
        file,
        line,
        warning_id,
    } = request
    else {
        return None;
    };
    let content = std::fs::read_to_string(Path::new(file)).ok()?;
    let before = content.lines().nth(line.checked_sub(1)?)?.to_string();
    let after = apply_antipattern_fix(&before, warning_id).ok()?;
    Some(FixPreview {
        line: *line,
        before,
        after,
    })
}

fn apply_line_transform(
    file: &str,
    line: usize,
    transform: impl FnOnce(&str) -> Result<String, String>,
) -> FixOutcome {
    let path = Path::new(file);
    let Ok(content) = std::fs::read_to_string(path) else {
        return FixOutcome::Failed {
            reason: format!("Failed to read {file}"),
        };
    };
    let had_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    if line == 0 || line > lines.len() {
        return FixOutcome::Refused {
            reason: format!("Line {line} is out of range for {file}"),
        };
    }

    let line_index = line - 1;
    let source = lines[line_index].clone();
    let replacement = match transform(&source) {
        Ok(line) => line,
        Err(reason) => return FixOutcome::Refused { reason },
    };
    lines[line_index] = replacement;

    match write_lines(path, &lines, had_trailing_newline) {
        Ok(()) => FixOutcome::Applied {
            summary: format!("Applied fix in {file}:{line}"),
        },
        Err(err) => FixOutcome::Failed {
            reason: format!("Failed to write {file}: {err}"),
        },
    }
}

fn apply_line_removal(
    file: &str,
    line: usize,
    validate: impl FnOnce(&str) -> Result<(), String>,
) -> FixOutcome {
    let path = Path::new(file);
    let Ok(content) = std::fs::read_to_string(path) else {
        return FixOutcome::Failed {
            reason: format!("Failed to read {file}"),
        };
    };
    let had_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    if line == 0 || line > lines.len() {
        return FixOutcome::Refused {
            reason: format!("Line {line} is out of range for {file}"),
        };
    }

    let line_index = line - 1;
    let source = lines[line_index].clone();
    if let Err(reason) = validate(&source) {
        return FixOutcome::Refused { reason };
    }
    lines.remove(line_index);

    match write_lines(path, &lines, had_trailing_newline) {
        Ok(()) => FixOutcome::Applied {
            summary: format!("Removed console statement in {file}:{line}"),
        },
        Err(err) => FixOutcome::Failed {
            reason: format!("Failed to write {file}: {err}"),
        },
    }
}

fn write_lines(path: &Path, lines: &[String], had_trailing_newline: bool) -> std::io::Result<()> {
    let mut output = lines.join("\n");
    if had_trailing_newline && !output.is_empty() {
        output.push('\n');
    }
    std::fs::write(path, output)
}

fn apply_antipattern_fix(source: &str, warning_id: &str) -> Result<String, ()> {
    match warning_id {
        "AP-001" => source
            .contains("/* eslint-disable */")
            .then(|| source.replace("/* eslint-disable */", "// eslint-disable-next-line"))
            .ok_or(()),
        "AP-004" => source
            .contains("@ts-ignore")
            .then(|| source.replace("@ts-ignore", "@ts-expect-error"))
            .ok_or(()),
        "AP-003" => replace_any_annotation(source).ok_or(()),
        _ => Err(()),
    }
}

fn replace_any_annotation(source: &str) -> Option<String> {
    let trimmed = source.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("* ") {
        return None;
    }
    if !ANY_ANNOTATION_RE.is_match(source) && !source.contains(": any") && !source.contains(":any")
    {
        return None;
    }

    let mut result = String::with_capacity(source.len());
    let mut chars = source.char_indices().peekable();
    let mut in_string: Option<char> = None;
    let mut in_block_comment = false;

    while let Some((idx, ch)) = chars.next() {
        let next = chars.peek().copied();

        if let Some(delimiter) = in_string {
            result.push(ch);
            if ch == '\\' {
                if let Some((_, escaped)) = chars.next() {
                    result.push(escaped);
                }
                continue;
            }
            if ch == delimiter {
                in_string = None;
            }
            continue;
        }

        if in_block_comment {
            result.push(ch);
            if ch == '*'
                && let Some((_, '/')) = next
            {
                result.push('/');
                chars.next();
                in_block_comment = false;
            }
            continue;
        }

        if ch == '/' {
            if let Some((_, '/')) = next {
                result.push_str(&source[idx..]);
                break;
            }
            if let Some((_, '*')) = next {
                result.push('/');
                result.push('*');
                chars.next();
                in_block_comment = true;
                continue;
            }
        }

        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = Some(ch);
            result.push(ch);
            continue;
        }

        if ch == ':' {
            let rest = &source[idx..];
            if let Some(captures) = ANY_ANNOTATION_RE.captures(rest) {
                let matched = captures.get(0).expect("whole match exists");
                let prefix = captures.get(1).expect("prefix exists").as_str();
                result.push_str(prefix);
                result.push_str("unknown");
                let end_idx = idx + matched.end();
                while let Some((next_idx, _)) = chars.peek().copied() {
                    if next_idx < end_idx {
                        chars.next();
                    } else {
                        break;
                    }
                }
                continue;
            }
        }

        result.push(ch);
    }

    (result != source).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, content: &str) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new()
            .prefix(name)
            .tempfile()
            .expect("temp file");
        std::fs::write(file.path(), content).expect("write temp file");
        file
    }

    #[test]
    fn replaces_any_annotation() {
        let path = temp_file("sample.ts", "const value: any = source;\n");
        let outcome = apply_fix_request(
            &FixRequest::AntiPatternWarning {
                file: path.path().to_string_lossy().to_string(),
                line: 1,
                warning_id: "AP-003".to_string(),
            },
            None,
        );
        assert!(matches!(outcome, FixOutcome::Applied { .. }));
        let content = std::fs::read_to_string(path.path()).expect("read updated file");
        assert_eq!(content, "const value: unknown = source;\n");
    }

    #[test]
    fn replaces_ts_ignore() {
        let path = temp_file("ignore.ts", "// @ts-ignore\nconst x = 1;\n");
        let outcome = apply_fix_request(
            &FixRequest::AntiPatternWarning {
                file: path.path().to_string_lossy().to_string(),
                line: 1,
                warning_id: "AP-004".to_string(),
            },
            None,
        );
        assert!(matches!(outcome, FixOutcome::Applied { .. }));
        let content = std::fs::read_to_string(path.path()).expect("read updated file");
        assert_eq!(content, "// @ts-expect-error\nconst x = 1;\n");
    }

    #[test]
    fn removes_console_statement_line() {
        let path = temp_file(
            "console.ts",
            "const x = 1;\nconsole.log(x);\nconst y = 2;\n",
        );
        let outcome = apply_fix_request(
            &FixRequest::AuditConsoleStatement {
                file: path.path().to_string_lossy().to_string(),
                line: 2,
            },
            None,
        );
        assert!(matches!(outcome, FixOutcome::Applied { .. }));
        let content = std::fs::read_to_string(path.path()).expect("read updated file");
        assert_eq!(content, "const x = 1;\nconst y = 2;\n");
    }

    #[test]
    fn refuses_non_standalone_console_statement() {
        let path = temp_file("console.ts", "if (debug) console.log(x);\n");
        let outcome = apply_fix_request(
            &FixRequest::AuditConsoleStatement {
                file: path.path().to_string_lossy().to_string(),
                line: 1,
            },
            None,
        );
        assert!(matches!(outcome, FixOutcome::Refused { .. }));
    }

    // ── WOW-005: preview_fix_request ─────────────────────────────────────

    #[test]
    fn preview_matches_apply_and_does_not_write() {
        let path = temp_file("preview.ts", "const a = 1;\nconst value: any = source;\n");
        let request = FixRequest::AntiPatternWarning {
            file: path.path().to_string_lossy().to_string(),
            line: 2,
            warning_id: "AP-003".to_string(),
        };

        let preview = preview_fix_request(&request).expect("preview");
        assert_eq!(preview.line, 2);
        assert_eq!(preview.before, "const value: any = source;");
        assert_eq!(preview.after, "const value: unknown = source;");

        // Preview must not write: the file is unchanged.
        let content = std::fs::read_to_string(path.path()).expect("read file");
        assert_eq!(content, "const a = 1;\nconst value: any = source;\n");

        // The consented apply writes exactly the previewed line.
        let outcome = apply_fix_request(&request, None);
        assert!(matches!(outcome, FixOutcome::Applied { .. }));
        let content = std::fs::read_to_string(path.path()).expect("read updated file");
        assert_eq!(content.lines().nth(1), Some(preview.after.as_str()));
    }

    #[test]
    fn preview_none_when_no_deterministic_transform() {
        let path = temp_file("preview.ts", "const value = source;\n");
        let request = FixRequest::AntiPatternWarning {
            file: path.path().to_string_lossy().to_string(),
            line: 1,
            warning_id: "AP-003".to_string(),
        };
        assert!(preview_fix_request(&request).is_none());
    }

    #[test]
    fn preview_none_for_out_of_range_line() {
        let path = temp_file("preview.ts", "const value: any = source;\n");
        for line in [0, 9] {
            let request = FixRequest::AntiPatternWarning {
                file: path.path().to_string_lossy().to_string(),
                line,
                warning_id: "AP-003".to_string(),
            };
            assert!(preview_fix_request(&request).is_none(), "line {line}");
        }
    }

    #[test]
    fn preview_none_for_non_antipattern_requests() {
        assert!(preview_fix_request(&FixRequest::DoctorCheck { index: 0 }).is_none());
    }

    #[test]
    fn refuses_out_of_range_doctor_check() {
        let mut checks = Vec::new();
        let outcome = apply_fix_request(
            &FixRequest::DoctorCheck { index: 0 },
            Some(checks.as_mut_slice()),
        );
        assert!(matches!(outcome, FixOutcome::Refused { .. }));
    }
}
