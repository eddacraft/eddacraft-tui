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
        } => apply_line_transform(Path::new(file), file, *line, None, |source| {
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

/// WOW-005: validate a first-win fix target before reading or writing it.
///
/// Fail-closed guard shared by [`preview_fix_request`] and
/// [`apply_previewed_fix_request`]. The discovery scanner reads through
/// symlinks, but a consented write must only land on a real file inside the
/// project root, so this refuses when:
/// - the target, or any path component at or below the project root, is a
///   symlink; or
/// - the canonicalised target resolves outside the canonicalised root
///   (e.g. `../` traversal or a symlinked directory pointing elsewhere).
fn guarded_fix_target(file: &str, root: &Path) -> Result<std::path::PathBuf, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|err| format!("Could not resolve the project root: {err}"))?;
    let raw = Path::new(file);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        canonical_root.join(raw)
    };

    // Refuse symlinks anywhere on the target's path at or below the root.
    // `canonical_root` itself is symlink-free by construction.
    let mut probe = joined.clone();
    loop {
        if let Ok(meta) = std::fs::symlink_metadata(&probe)
            && meta.file_type().is_symlink()
        {
            return Err(format!(
                "Refusing to modify {file}: the path contains a symlink"
            ));
        }
        if !probe.pop() || probe == canonical_root || !probe.starts_with(&canonical_root) {
            break;
        }
    }

    let canonical = joined
        .canonicalize()
        .map_err(|err| format!("Could not resolve {file}: {err}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "Refusing to modify {file}: it resolves outside the project root"
        ));
    }
    Ok(canonical)
}

/// WOW-005: compute the line change a consented apply would write, without
/// writing anything. The transform is the same function the apply path runs,
/// and [`apply_previewed_fix_request`] additionally refuses unless the
/// on-disk line still matches this preview's `before` text exactly — so what
/// was shown is the only thing that can ever be written. Returns `None` when
/// no deterministic preview exists (unsupported request kind, unreadable or
/// guarded-out target, out-of-range line, no transform for the line) — the
/// caller falls back to the next candidate rather than showing a diff it
/// cannot honour.
pub fn preview_fix_request(request: &FixRequest, root: &Path) -> Option<FixPreview> {
    let FixRequest::AntiPatternWarning {
        file,
        line,
        warning_id,
    } = request
    else {
        return None;
    };
    let path = guarded_fix_target(file, root).ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let before = content.lines().nth(line.checked_sub(1)?)?.to_string();
    let after = apply_antipattern_fix(&before, warning_id).ok()?;
    Some(FixPreview {
        line: *line,
        before,
        after,
    })
}

/// WOW-005: apply a fix the user consented to after seeing a preview.
///
/// Unlike [`apply_fix_request`], this refuses (writing nothing) unless the
/// on-disk line still equals `expected_before` — the exact text the consent
/// screen showed. Without this guard a concurrent edit that still matches the
/// same anti-pattern would be silently rewritten to content the user never
/// saw. The target is re-validated through [`guarded_fix_target`] at apply
/// time (fail closed on symlinks and root escapes).
pub fn apply_previewed_fix_request(
    request: &FixRequest,
    expected_before: &str,
    root: &Path,
) -> FixOutcome {
    let FixRequest::AntiPatternWarning {
        file,
        line,
        warning_id,
    } = request
    else {
        return FixOutcome::Failed {
            reason: "Unsupported fix request for a previewed apply".to_string(),
        };
    };
    let path = match guarded_fix_target(file, root) {
        Ok(path) => path,
        Err(reason) => return FixOutcome::Refused { reason },
    };
    apply_line_transform(&path, file, *line, Some(expected_before), |source| {
        apply_antipattern_fix(source, warning_id).map_err(|()| {
            format!("No deterministic auto-fix available for {warning_id} on this line")
        })
    })
}

fn apply_line_transform(
    path: &Path,
    display: &str,
    line: usize,
    expected_before: Option<&str>,
    transform: impl FnOnce(&str) -> Result<String, String>,
) -> FixOutcome {
    let Ok(content) = std::fs::read_to_string(path) else {
        return FixOutcome::Failed {
            reason: format!("Failed to read {display}"),
        };
    };
    let had_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    if line == 0 || line > lines.len() {
        return FixOutcome::Refused {
            reason: format!("Line {line} is out of range for {display}"),
        };
    }

    let line_index = line - 1;
    let source = lines[line_index].clone();
    // TOCTOU guard for previewed applies: the write is only valid for the
    // exact line the user consented to.
    if let Some(expected) = expected_before
        && source != expected
    {
        return FixOutcome::Refused {
            reason: format!(
                "{display}:{line} changed since the preview was shown; nothing was written"
            ),
        };
    }
    let replacement = match transform(&source) {
        Ok(line) => line,
        Err(reason) => return FixOutcome::Refused { reason },
    };
    lines[line_index] = replacement;

    match write_lines(path, &lines, had_trailing_newline) {
        Ok(()) => FixOutcome::Applied {
            summary: format!("Applied fix in {display}:{line}"),
        },
        Err(err) => FixOutcome::Failed {
            reason: format!("Failed to write {display}: {err}"),
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

/// Write the fixed content atomically: a straight truncate-and-write can
/// leave a half-written source file if the process dies mid-write, so the
/// content goes to a temporary file in the same directory (same filesystem,
/// so the rename is atomic) and is renamed over the original, preserving the
/// original file's permissions.
fn write_lines(path: &Path, lines: &[String], had_trailing_newline: bool) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut output = lines.join("\n");
    if had_trailing_newline && !output.is_empty() {
        output.push('\n');
    }
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(output.as_bytes())?;
    let permissions = std::fs::metadata(path)?.permissions();
    tmp.as_file().set_permissions(permissions)?;
    tmp.persist(path).map_err(|err| err.error)?;
    Ok(())
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

    struct TempSource {
        _dir: tempfile::TempDir,
        path: std::path::PathBuf,
    }

    impl TempSource {
        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    // A `NamedTempFile` would keep the destination open, and on Windows the fix
    // engine's atomic rename-over (`tmp.persist`) cannot replace an open file —
    // so the harness hands out a closed file inside its own temp dir instead.
    fn temp_file(name: &str, content: &str) -> TempSource {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write temp file");
        TempSource { _dir: dir, path }
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

    // ── WOW-005: preview_fix_request / apply_previewed_fix_request ──────

    /// A project root with one relative source file inside it, mirroring how
    /// discovery findings carry root-relative paths.
    fn temp_root(file_name: &str, content: &str) -> (tempfile::TempDir, FixRequest) {
        let root = tempfile::tempdir().expect("temp root");
        std::fs::write(root.path().join(file_name), content).expect("write source file");
        let request = FixRequest::AntiPatternWarning {
            file: file_name.to_string(),
            line: 1,
            warning_id: "AP-003".to_string(),
        };
        (root, request)
    }

    #[test]
    fn preview_matches_apply_and_does_not_write() {
        let root = tempfile::tempdir().expect("temp root");
        std::fs::write(
            root.path().join("preview.ts"),
            "const a = 1;\nconst value: any = source;\n",
        )
        .expect("write source file");
        let request = FixRequest::AntiPatternWarning {
            file: "preview.ts".to_string(),
            line: 2,
            warning_id: "AP-003".to_string(),
        };

        let preview = preview_fix_request(&request, root.path()).expect("preview");
        assert_eq!(preview.line, 2);
        assert_eq!(preview.before, "const value: any = source;");
        assert_eq!(preview.after, "const value: unknown = source;");

        // Preview must not write: the file is unchanged.
        let content = std::fs::read_to_string(root.path().join("preview.ts")).expect("read file");
        assert_eq!(content, "const a = 1;\nconst value: any = source;\n");

        // The consented apply writes exactly the previewed line.
        let outcome = apply_previewed_fix_request(&request, &preview.before, root.path());
        assert!(matches!(outcome, FixOutcome::Applied { .. }));
        let content =
            std::fs::read_to_string(root.path().join("preview.ts")).expect("read updated file");
        assert_eq!(content.lines().nth(1), Some(preview.after.as_str()));
    }

    #[test]
    fn previewed_apply_refuses_when_line_changed_since_preview() {
        // Council repro: a concurrent edit that still matches the same
        // anti-pattern must not be silently rewritten under a consent that
        // was given for different content.
        let (root, request) = temp_root("app.ts", "const value: any = source;\n");
        let preview = preview_fix_request(&request, root.path()).expect("preview");

        let mutated = "const totallyDifferent: any = elsewhere;\n";
        std::fs::write(root.path().join("app.ts"), mutated).expect("mutate file");

        let outcome = apply_previewed_fix_request(&request, &preview.before, root.path());
        match outcome {
            FixOutcome::Refused { reason } => {
                assert!(
                    reason.contains("changed since the preview"),
                    "reason must name the drift: {reason}"
                );
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        // Nothing was written: the concurrent edit is intact.
        let content = std::fs::read_to_string(root.path().join("app.ts")).expect("read file");
        assert_eq!(content, mutated);
    }

    #[test]
    fn previewed_apply_writes_when_line_still_matches() {
        let (root, request) = temp_root("app.ts", "const value: any = source;\n");
        let preview = preview_fix_request(&request, root.path()).expect("preview");
        let outcome = apply_previewed_fix_request(&request, &preview.before, root.path());
        assert!(matches!(outcome, FixOutcome::Applied { .. }));
        let content = std::fs::read_to_string(root.path().join("app.ts")).expect("read file");
        assert_eq!(content, "const value: unknown = source;\n");
    }

    #[cfg(unix)]
    #[test]
    fn preview_and_apply_refuse_symlinked_targets() {
        // A symlinked file inside the repo pointing outside must never be
        // offered, and a consented apply must refuse it (fail closed).
        let outside = tempfile::tempdir().expect("outside dir");
        let victim = outside.path().join("victim.ts");
        let original = "const value: any = source;\n";
        std::fs::write(&victim, original).expect("write victim");

        let root = tempfile::tempdir().expect("root dir");
        std::os::unix::fs::symlink(&victim, root.path().join("link.ts")).expect("symlink");
        let request = FixRequest::AntiPatternWarning {
            file: "link.ts".to_string(),
            line: 1,
            warning_id: "AP-003".to_string(),
        };

        assert!(preview_fix_request(&request, root.path()).is_none());
        let outcome =
            apply_previewed_fix_request(&request, "const value: any = source;", root.path());
        assert!(matches!(outcome, FixOutcome::Refused { .. }), "{outcome:?}");
        // The symlink target was never touched.
        let content = std::fs::read_to_string(&victim).expect("read victim");
        assert_eq!(content, original);
    }

    #[test]
    fn preview_and_apply_refuse_targets_outside_root() {
        let outside = tempfile::tempdir().expect("outside dir");
        let victim = outside.path().join("victim.ts");
        let original = "const value: any = source;\n";
        std::fs::write(&victim, original).expect("write victim");

        let root = tempfile::tempdir().expect("root dir");
        for file in [
            victim.to_string_lossy().to_string(),
            format!(
                "../{}/victim.ts",
                outside.path().file_name().unwrap().to_string_lossy()
            ),
        ] {
            let request = FixRequest::AntiPatternWarning {
                file,
                line: 1,
                warning_id: "AP-003".to_string(),
            };
            assert!(preview_fix_request(&request, root.path()).is_none());
            let outcome =
                apply_previewed_fix_request(&request, "const value: any = source;", root.path());
            assert!(matches!(outcome, FixOutcome::Refused { .. }), "{outcome:?}");
        }
        let content = std::fs::read_to_string(&victim).expect("read victim");
        assert_eq!(content, original);
    }

    #[cfg(unix)]
    #[test]
    fn previewed_apply_preserves_file_permissions() {
        use std::os::unix::fs::PermissionsExt as _;
        let (root, request) = temp_root("app.ts", "const value: any = source;\n");
        let path = root.path().join("app.ts");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("set mode");
        let preview = preview_fix_request(&request, root.path()).expect("preview");
        let outcome = apply_previewed_fix_request(&request, &preview.before, root.path());
        assert!(matches!(outcome, FixOutcome::Applied { .. }));
        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "atomic rewrite must preserve permissions");
    }

    #[test]
    fn preview_none_when_no_deterministic_transform() {
        let (root, request) = temp_root("preview.ts", "const value = source;\n");
        assert!(preview_fix_request(&request, root.path()).is_none());
    }

    #[test]
    fn preview_none_for_out_of_range_line() {
        let (root, _) = temp_root("preview.ts", "const value: any = source;\n");
        for line in [0, 9] {
            let request = FixRequest::AntiPatternWarning {
                file: "preview.ts".to_string(),
                line,
                warning_id: "AP-003".to_string(),
            };
            assert!(
                preview_fix_request(&request, root.path()).is_none(),
                "line {line}"
            );
        }
    }

    #[test]
    fn preview_none_for_non_antipattern_requests() {
        let root = tempfile::tempdir().expect("temp root");
        assert!(preview_fix_request(&FixRequest::DoctorCheck { index: 0 }, root.path()).is_none());
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
