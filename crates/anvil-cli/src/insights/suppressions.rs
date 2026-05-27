//! INSIGHTS-002 — `anvil insights --suppressions` suppression health view.
//!
//! Derives from a **live antipattern scan**, not a durable log (there is
//! none — see the module's APS entry). Entries come from the inline
//! `@anvil-ignore` directives swept from source (the ADR-029 authoritative
//! `anvil_checks::antipattern::parse_suppression`); each directive is
//! **active** when a suppressed finding for the same rule sits one line below
//! it, else **stale** (the underlying violation is gone). Suppressions the
//! scanner attributes to other sources (e.g. `eslint-disable`) are out of
//! scope and not counted.
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

/// A suppressed finding the live scan reported, used only to decide whether a
/// directive is load-bearing and to carry the suppression date.
#[derive(Debug, Clone)]
pub(crate) struct SuppressedFinding {
    pub file: String,
    pub line: usize,
    pub rule: String,
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

/// Build the sorted health list from the swept `@anvil-ignore` directives,
/// using the suppressed findings only to decide active-ness. A directive is
/// **active** when a suppressed finding for the same file + rule sits exactly
/// one line below it — the scanner (`anvil-checks`'s `suppression_for_line`)
/// honours a directive only on the line *immediately above* the finding, so
/// `finding.line == directive.line + 1`; otherwise the directive is **stale**
/// (the underlying violation is gone).
///
/// Directives are the **sole** source of entries, so suppressions the scanner
/// attributes to non-`@anvil-ignore` sources (e.g. `eslint-disable`) are not
/// counted — keeping this view to its stated scope and giving every entry a
/// consistent directive-line provenance. Both `file` strings are
/// workspace-relative paths from the same canonical root, so full-path
/// equality is exact (no basename collisions).
///
/// Pure so it can be unit-tested without exercising the antipattern engine.
pub(crate) fn classify(
    directives: &[Directive],
    suppressed: &[SuppressedFinding],
) -> Vec<SuppressionEntry> {
    let mut entries: Vec<SuppressionEntry> = directives
        .iter()
        .map(|directive| {
            let matched = suppressed.iter().find(|s| {
                s.rule == directive.rule && s.file == directive.file && s.line == directive.line + 1
            });
            SuppressionEntry {
                file: directive.file.clone(),
                line: directive.line,
                rule: directive.rule.clone(),
                reason: directive.reason.clone(),
                // Provenance date comes from the matched suppression when the
                // directive is load-bearing; inline directives usually omit it.
                date: matched.and_then(|s| s.date.clone()),
                stale: matched.is_none(),
            }
        })
        .collect();

    // Stale first, then a fully-deterministic key (file, line, rule, reason) so
    // multiple directives on the same line order stably regardless of the
    // (rayon-parallel) scan order.
    entries.sort_by(|a, b| {
        b.stale
            .cmp(&a.stale)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule.cmp(&b.rule))
            .then_with(|| a.reason.cmp(&b.reason))
    });
    entries
}

/// Build the suppression health view by scanning `root`'s antipattern
/// surfaces for inline `@anvil-ignore` directives and cross-referencing the
/// live findings.
///
/// Best-effort and infallible: unreadable files are skipped, and a
/// non-canonicalisable root falls back to the path as given, rather than
/// erroring — matching the once-a-week-glance nature of the command.
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

    let entries = classify(&directives, &suppressed);
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
    fn classify_orphan_directive_stale_active_directive_kept_and_sorts_first() {
        let directives = vec![
            directive("src/active.ts", 9, "AP-001"), // finding at line 10 (dir+1) -> active
            directive("src/dead.ts", 4, "AP-004"),   // no finding -> stale
        ];
        let suppressed = vec![finding("src/active.ts", 10, "AP-001")];
        let entries = classify(&directives, &suppressed);

        // One entry per directive; stale sorts first.
        assert_eq!(entries.len(), 2, "got: {entries:#?}");
        assert!(entries[0].stale);
        assert_eq!(entries[0].file, "src/dead.ts");
        assert_eq!(entries[0].rule, "AP-004");
        // Active entry reports the DIRECTIVE line (9), not the finding line (10).
        assert!(!entries[1].stale);
        assert_eq!(entries[1].rule, "AP-001");
        assert_eq!(
            entries[1].line, 9,
            "active entry must use the directive line"
        );
    }

    #[test]
    fn classify_directive_immediately_above_finding_is_active() {
        // The scanner honours a directive only on the line directly above the
        // finding, so finding.line == directive.line + 1 is load-bearing.
        let directives = vec![directive("a.ts", 7, "AP-002")];
        let suppressed = vec![finding("a.ts", 8, "AP-002")];
        let entries = classify(&directives, &suppressed);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].stale);
        assert_eq!(entries[0].line, 7);
    }

    #[test]
    fn classify_full_path_match_does_not_collide_on_basename() {
        // A suppressed finding in a same-named file in another dir must NOT
        // mark this directive active.
        let directives = vec![directive("src/index.ts", 7, "AP-001")];
        let suppressed = vec![finding("packages/a/index.ts", 8, "AP-001")];
        let entries = classify(&directives, &suppressed);
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0].stale,
            "basename collision must not mark it active"
        );
        assert_eq!(entries[0].file, "src/index.ts");
    }

    #[test]
    fn classify_directive_for_other_rule_is_stale() {
        let directives = vec![directive("a.ts", 6, "AP-009")];
        let suppressed = vec![finding("a.ts", 7, "AP-002")]; // different rule
        let entries = classify(&directives, &suppressed);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].rule, "AP-009");
        assert!(entries[0].stale);
    }

    #[test]
    fn classify_ignores_suppressed_findings_with_no_directive() {
        // A suppressed finding from a non-@anvil-ignore source (e.g. an
        // eslint-disable the scanner attributes) has no swept directive, so it
        // must NOT appear as an entry or inflate the counts (Copilot #1).
        let directives: Vec<Directive> = Vec::new();
        let suppressed = vec![finding("a.ts", 5, "AP-001")];
        let entries = classify(&directives, &suppressed);
        assert!(
            entries.is_empty(),
            "non-directive suppressions must not count"
        );
    }

    #[test]
    fn classify_active_entry_carries_matched_suppression_date() {
        let directives = vec![directive("a.ts", 3, "AP-001")];
        let suppressed = vec![SuppressedFinding {
            file: "a.ts".to_string(),
            line: 4,
            rule: "AP-001".to_string(),
            date: Some("2026-05-01".to_string()),
        }];
        let entries = classify(&directives, &suppressed);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].stale);
        assert_eq!(entries[0].date.as_deref(), Some("2026-05-01"));
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
