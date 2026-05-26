//! INSIGHTS-002 — `anvil insights --suppressions` suppression health view.
//!
//! Derives from a **live antipattern scan**, not a durable log (there is
//! none — see the module's APS entry). Active suppressions come from
//! `Warning.suppressed`; stale ones from inline `@anvil-ignore` directives
//! that suppress no current finding. The directive parser is the ADR-029
//! authoritative `anvil_checks::antipattern::parse_suppression`.
//!
//! Scope is the antipattern checker's surfaces (TS/JS/HTML/CSS per the
//! default config); config-level `RuleModes` rule-disabling is a separate
//! concern and is not part of this view.

use std::path::{Path, PathBuf};

use anvil_checks::antipattern::{AntipatternCheckConfig, parse_suppression, run_antipattern_check};
use serde::Serialize;
use walkdir::WalkDir;

use crate::util::is_ignored_dir_name;

/// Schema id for the `--json` document; bump on any breaking field change.
pub const SCHEMA_VERSION: &str = "anvil.suppressions.v1";

/// A suppressed finding the live scan reported (the directive is load-bearing).
#[derive(Debug, Clone)]
pub(crate) struct SuppressedFinding {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub reason: String,
    pub date: Option<String>,
}

/// An `@anvil-ignore` directive found by sweeping source.
#[derive(Debug, Clone)]
pub(crate) struct Directive {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuppressionEntry {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub reason: String,
    /// Recorded suppression date, when the directive carries one. Inline
    /// `@anvil-ignore` directives usually do not, so this is often `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// `true` when the underlying violation is gone (a dead suppression to
    /// remove); `false` when the violation still fires under the directive.
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuppressionHealth {
    pub schema_version: &'static str,
    /// All `@anvil-ignore` directives found (active + stale).
    pub total: usize,
    /// Directives still suppressing a live finding (`total - stale`).
    pub active: usize,
    /// Directives whose underlying violation is gone — safe to remove.
    pub stale: usize,
    pub entries: Vec<SuppressionEntry>,
}

/// Combine suppressed findings (load-bearing) and swept directives into a
/// sorted health list. A directive is **stale** when no suppressed finding
/// for the same rule sits exactly one line below it. Stale entries sort first.
///
/// The line relationship mirrors the scanner: `anvil-checks`'s
/// `suppression_for_line` honours a directive only when it is on the line
/// *immediately above* the finding, so `finding.line == directive.line + 1`.
/// Both `file` strings are workspace-relative paths derived from the same
/// canonical root, so full-path equality is exact (no basename collisions).
///
/// Pure so it can be unit-tested without exercising the antipattern engine.
pub(crate) fn classify(
    suppressed: &[SuppressedFinding],
    directives: &[Directive],
) -> Vec<SuppressionEntry> {
    let mut entries: Vec<SuppressionEntry> = suppressed
        .iter()
        .map(|s| SuppressionEntry {
            file: s.file.clone(),
            line: s.line,
            rule: s.rule.clone(),
            reason: s.reason.clone(),
            date: s.date.clone(),
            stale: false,
        })
        .collect();

    for directive in directives {
        let load_bearing = suppressed.iter().any(|s| {
            s.rule == directive.rule && s.file == directive.file && s.line == directive.line + 1
        });
        if !load_bearing {
            entries.push(SuppressionEntry {
                file: directive.file.clone(),
                line: directive.line,
                rule: directive.rule.clone(),
                reason: directive.reason.clone(),
                date: None,
                stale: true,
            });
        }
    }

    // Stale first, then file, then line — stable, actionable ordering.
    entries.sort_by(|a, b| {
        b.stale
            .cmp(&a.stale)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    entries
}

/// Build the suppression health view by scanning `root`'s antipattern
/// surfaces for inline `@anvil-ignore` directives and cross-referencing the
/// live findings.
///
/// Best-effort and infallible: unreadable files and a non-canonicalisable
/// root are skipped rather than erroring, matching the once-a-week-glance
/// nature of the command.
#[must_use]
pub fn suppression_health(root: &Path) -> SuppressionHealth {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let config = AntipatternCheckConfig::default();
    let files = walk_scanned_files(&root, &config);

    let file_strs: Vec<String> = files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let file_refs: Vec<&str> = file_strs.iter().map(String::as_str).collect();
    let workspace_root = root.to_string_lossy().to_string();

    let result = run_antipattern_check(&file_refs, &config, Some(&workspace_root));

    let suppressed: Vec<SuppressedFinding> = result
        .warnings
        .warnings
        .iter()
        .filter_map(|w| {
            w.suppressed.as_ref().map(|supp| SuppressedFinding {
                file: w.location.file.clone(),
                line: w.location.line,
                rule: w.id.clone(),
                reason: supp.reason.clone(),
                date: supp.timestamp.clone(),
            })
        })
        .collect();

    // Sweep source for directives to catch stale ones. This re-reads files the
    // antipattern scan already read (it owns the bytes internally and does not
    // surface them); acceptable for a weekly-glance command, and a
    // content-returning scan variant would be the optimisation if it matters.
    let mut directives: Vec<Directive> = Vec::new();
    for path in &files {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = display_path(path, &root);
        for (idx, line) in content.lines().enumerate() {
            if let Some((rule, reason)) = parse_suppression(line) {
                directives.push(Directive {
                    file: rel.clone(),
                    line: idx + 1,
                    rule,
                    reason,
                });
            }
        }
    }

    let entries = classify(&suppressed, &directives);
    let stale = entries.iter().filter(|e| e.stale).count();
    SuppressionHealth {
        schema_version: SCHEMA_VERSION,
        total: entries.len(),
        active: entries.len() - stale,
        stale,
        entries,
    }
}

fn walk_scanned_files(root: &Path, config: &AntipatternCheckConfig) -> Vec<PathBuf> {
    // Use the canonical workspace ignore list (the same predicate every other
    // anvil-cli walker uses) rather than an ad-hoc set — notably it does NOT
    // prune `packages/anvil/`, real TS source, and DOES prune `.nx`, `.turbo`,
    // `.venv`, `.worktrees`, etc.
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir() && is_ignored_dir_name(&e.file_name().to_string_lossy()))
        })
    {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let s = path.to_string_lossy();
        if config
            .extensions
            .iter()
            .any(|ext| s.ends_with(ext.as_str()))
        {
            files.push(path.to_path_buf());
        }
    }
    files
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn finding(file: &str, line: usize, rule: &str) -> SuppressedFinding {
        SuppressedFinding {
            file: file.to_string(),
            line,
            rule: rule.to_string(),
            reason: "intentional".to_string(),
            date: None,
        }
    }

    fn directive(file: &str, line: usize, rule: &str) -> Directive {
        Directive {
            file: file.to_string(),
            line,
            rule: rule.to_string(),
            reason: "old reason".to_string(),
        }
    }

    #[test]
    fn classify_flags_orphan_directive_stale_and_sorts_first() {
        // One load-bearing suppression (finding present) + one stale directive.
        let suppressed = vec![finding("src/active.ts", 10, "AP-001")];
        let directives = vec![
            directive("src/active.ts", 9, "AP-001"), // matches finding at line 10 (dir+1)
            directive("src/dead.ts", 4, "AP-004"),   // no finding -> stale
        ];
        let entries = classify(&suppressed, &directives);

        // Active suppression + one stale directive; the matched directive is
        // not double-counted.
        assert_eq!(entries.len(), 2, "got: {entries:#?}");
        // Stale sorts first.
        assert!(entries[0].stale);
        assert_eq!(entries[0].file, "src/dead.ts");
        assert_eq!(entries[0].rule, "AP-004");
        // The active one is present and not stale.
        assert!(!entries[1].stale);
        assert_eq!(entries[1].rule, "AP-001");
    }

    #[test]
    fn classify_directive_immediately_above_finding_is_load_bearing() {
        // The scanner honours a directive only on the line directly above the
        // finding, so finding.line == directive.line + 1 is load-bearing.
        let suppressed = vec![finding("a.ts", 8, "AP-002")];
        let directives = vec![directive("a.ts", 7, "AP-002")];
        let entries = classify(&suppressed, &directives);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].stale);
    }

    #[test]
    fn classify_full_path_match_does_not_collide_on_basename() {
        // A suppressed finding in one dir must NOT mask a stale directive of
        // the same rule in a same-named file in another dir.
        let suppressed = vec![finding("packages/a/index.ts", 8, "AP-001")];
        let directives = vec![directive("src/index.ts", 7, "AP-001")];
        let entries = classify(&suppressed, &directives);
        // active (the suppressed finding) + stale (the unmatched directive).
        assert_eq!(entries.len(), 2);
        let stale: Vec<&SuppressionEntry> = entries.iter().filter(|e| e.stale).collect();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].file, "src/index.ts");
    }

    #[test]
    fn classify_directive_for_other_rule_is_stale() {
        let suppressed = vec![finding("a.ts", 7, "AP-002")];
        let directives = vec![directive("a.ts", 6, "AP-009")]; // different rule
        let entries = classify(&suppressed, &directives);
        // The AP-002 finding (active) + the unmatched AP-009 directive (stale).
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].rule, "AP-009");
        assert!(entries[0].stale);
    }

    #[test]
    fn suppression_health_detects_stale_directive_from_real_scan() {
        // A directive that suppresses no actual finding -> stale, end to end.
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("clean.ts"),
            "// @anvil-ignore AP-003 -- left over from a refactor\nexport const x = 1;\n",
        )
        .unwrap();

        let health = suppression_health(tmp.path());
        assert_eq!(health.schema_version, SCHEMA_VERSION);
        let stale: Vec<&SuppressionEntry> = health.entries.iter().filter(|e| e.stale).collect();
        assert_eq!(stale.len(), 1, "entries: {:#?}", health.entries);
        assert_eq!(stale[0].rule, "AP-003");
        assert!(stale[0].file.ends_with("clean.ts"));
        assert_eq!(health.stale, 1);
    }

    #[test]
    fn suppression_health_marks_load_bearing_directive_active_from_real_scan() {
        // A directive that suppresses a real finding (AP-001 fires on
        // `/* eslint-disable */`) must classify ACTIVE, not stale. This also
        // proves the scan's normalised `location.file` matches the swept
        // `display_path` output (full-path cross-ref works end to end).
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("legacy.ts"),
            "// @anvil-ignore AP-001 -- intentional for a generated bundle\n\
             /* eslint-disable */\n\
             export const x = 1;\n",
        )
        .unwrap();

        let health = suppression_health(tmp.path());
        assert!(
            health
                .entries
                .iter()
                .any(|e| e.rule == "AP-001" && !e.stale),
            "expected a load-bearing (active) AP-001 suppression, got: {:#?}",
            health.entries
        );
        assert_eq!(
            health.stale, 0,
            "no directive should be stale: {:#?}",
            health.entries
        );
        assert_eq!(health.active, health.total);
    }

    #[test]
    fn suppression_health_empty_when_no_directives() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("plain.ts"), "export const y = 2;\n").unwrap();
        let health = suppression_health(tmp.path());
        assert_eq!(health.total, 0);
        assert_eq!(health.stale, 0);
        assert!(health.entries.is_empty());
    }
}
