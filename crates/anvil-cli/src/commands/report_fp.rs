//! `anvil report-fp` — record a false-positive report against a check
//! (OPSUP-007 / ADR-089).
//!
//! The report is written to the **local** Kindling record only; nothing
//! leaves the machine (no network call — air-gap-safe). The file path is
//! recorded as a one-way salted hash, never plaintext, and source content is
//! never included unless the operator explicitly opts in with
//! `--include-snippet` (fail-closed on anonymisation). The check identifier is
//! validated against the OPSUP-001 stable-ID registry; an unknown identifier is
//! rejected with a registry-backed suggestion (the OPSUP-002 surface).
//!
//! Note: like every command, the invocation is also recorded to the local
//! `usage.ndjson` sidecar, which captures only the *coarse length bucket* of
//! each argument (never values) — so the location argument contributes a
//! path-length signal, not the path. Both sidecars are local-only and never
//! transmitted (ADR-089), so this stays strictly within what a local reader of
//! the user's own workspace already has.

use anyhow::{Result, bail};
use clap::Args;

use crate::GlobalArgs;
use crate::commands::check_catalog::{
    closest_registered_id, definition_by_name, owning_check_for_finding_id,
};
use crate::output::{self, OutputMode};

#[derive(Debug, Args)]
pub struct ReportFpArgs {
    /// List locally recorded false-positive reports instead of recording a new
    /// one. Prints check ID, hashed path, line, and timestamp; never plaintext
    /// paths or snippets.
    #[arg(long)]
    list: bool,

    /// The check the false positive fired under — a stable `ANV-*` ID, a
    /// printed finding / rule id (`PY-008`, `AP-008`, `WC-*`, `RS-*`,
    /// `SECRET-*`), the canonical name, or a legacy alias.
    #[arg(required_unless_present = "list")]
    check_id: Option<String>,

    /// Where the false positive fired, as `<file>:<line>` (the path is hashed,
    /// never recorded in plaintext).
    #[arg(required_unless_present = "list")]
    location: Option<String>,

    /// Opt in to recording the single source line as a snippet. Off by
    /// default — source content is never included unless this is set. The line
    /// is stored verbatim and is NOT redacted, so do not opt in when the
    /// flagged line contains a real secret.
    #[arg(long, conflicts_with = "list")]
    include_snippet: bool,
}

/// Resolve a user-supplied check identifier (stable ID / canonical / alias) to
/// its stable `ANV-*` ID, or fail with a registry-backed suggestion.
fn resolve_check_id(input: &str) -> Result<&'static str> {
    if let Some(def) = definition_by_name(input) {
        return Ok(def.stable_id);
    }
    if let Some(stable_id) = owning_check_for_finding_id(input) {
        return Ok(stable_id);
    }
    let suggestion = closest_registered_id(input)
        .map(|s| format!(" (did you mean '{s}'?)"))
        .unwrap_or_default();
    bail!("unknown check '{input}'{suggestion}");
}

/// Split a `<file>:<line>` location into its path and 1-based line. Splits on
/// the **last** colon so Windows drive letters (`C:\…`) survive in the path.
fn parse_location(location: &str) -> Result<(&str, u32)> {
    let (path, line) = location
        .rsplit_once(':')
        .ok_or_else(|| anyhow::anyhow!("location must be '<file>:<line>', got '{location}'"))?;
    if path.is_empty() {
        bail!("location is missing a file path: '{location}'");
    }
    let line: u32 = line.parse().map_err(|_| {
        anyhow::anyhow!("location line must be a 1-based number that fits in u32, got '{line}'")
    })?;
    if line == 0 {
        bail!("location line is 1-based; got 0");
    }
    Ok((path, line))
}

/// Read the single referenced line from `path` for an opt-in snippet.
/// Best-effort: a missing file or out-of-range line yields `None` (the report
/// still records, just without a snippet) rather than failing the command.
///
/// Streams line-by-line and stops at the target line so a pathologically large
/// or generated file can't be fully materialised into memory.
fn read_snippet(path: &str, line: u32) -> Option<String> {
    use std::io::BufRead as _;

    let target = (line as usize).checked_sub(1)?;
    let file = std::fs::File::open(path).ok()?;
    std::io::BufReader::new(file).lines().nth(target)?.ok()
}

pub fn run(args: &ReportFpArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);

    if args.list {
        return list_reports(mode);
    }

    let check_id_input = args
        .check_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing check id; pass --list to read local reports"))?;
    let location = args
        .location
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing location; pass --list to read local reports"))?;

    let check_id = resolve_check_id(check_id_input)?;
    let (path, line) = parse_location(location)?;

    let snippet = if args.include_snippet {
        read_snippet(path, line)
    } else {
        None
    };

    crate::usage::record_false_positive(check_id, path, line, snippet)?;

    match mode {
        OutputMode::Json => output::json::print(&serde_json::json!({
            "recorded": true,
            "check_id": check_id,
            "line": line,
        }))?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            output::plain::success(&format!(
                "Recorded false-positive report for {check_id} (path hashed; stored locally)"
            ));
        }
    }
    Ok(())
}

fn list_reports(mode: OutputMode) -> Result<()> {
    let reports = crate::usage::list_false_positive_reports()?;
    match mode {
        OutputMode::Json => output::json::print(&serde_json::json!({
            "count": reports.len(),
            "reports": reports,
        }))?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            if reports.is_empty() {
                output::plain::info("No false-positive reports recorded locally");
            } else {
                output::plain::section("Local false-positive reports");
                for report in reports {
                    println!(
                        "  {}:{} {} {}",
                        report.hashed_path, report.line, report.check_id, report.timestamp
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_check_id_accepts_stable_id_canonical_and_alias() {
        assert_eq!(resolve_check_id("ANV-CORE-001").unwrap(), "ANV-CORE-001");
        assert_eq!(
            resolve_check_id("secret-detection").unwrap(),
            "ANV-CORE-001"
        );
        // `secret` is a legacy alias of secret-detection.
        assert_eq!(resolve_check_id("secret").unwrap(), "ANV-CORE-001");
    }

    #[test]
    fn resolve_check_id_accepts_printed_finding_ids() {
        // `anvil check` prints rule ids (PY-008, AP-008, …), not ANV-* check ids.
        assert_eq!(resolve_check_id("PY-008").unwrap(), "ANV-CORE-003");
        assert_eq!(resolve_check_id("AP-008").unwrap(), "ANV-CORE-003");
        assert_eq!(resolve_check_id("WC-001").unwrap(), "ANV-CORE-003");
        assert_eq!(resolve_check_id("RS-001").unwrap(), "ANV-CORE-003");
        assert_eq!(
            resolve_check_id("SECRET-OPENAI-KEY").unwrap(),
            "ANV-CORE-001"
        );
    }

    #[test]
    fn report_fp_help_names_check_and_rule_ids() {
        use clap::{Args, Command};

        let help = ReportFpArgs::augment_args(Command::new("report-fp"))
            .render_long_help()
            .to_string();
        assert!(
            help.contains("ANV-"),
            "help must name ANV-* check ids: {help}"
        );
        assert!(
            help.contains("PY-") || help.contains("rule id"),
            "help must name printed rule ids: {help}"
        );
        assert!(
            help.contains("SECRET-"),
            "help must name SECRET-* finding ids: {help}"
        );
    }

    #[test]
    fn resolve_check_id_rejects_unknown_with_suggestion() {
        let err = resolve_check_id("lnt").unwrap_err().to_string();
        assert!(err.contains("unknown check 'lnt'"), "got: {err}");
        assert!(err.contains("did you mean 'lint'?"), "got: {err}");
    }

    #[test]
    fn parse_location_splits_path_and_line() {
        assert_eq!(parse_location("src/a.rs:12").unwrap(), ("src/a.rs", 12));
    }

    #[test]
    fn parse_location_keeps_windows_drive_letter() {
        // Splitting on the last colon keeps `C:\…` intact in the path.
        assert_eq!(
            parse_location(r"C:\src\a.rs:99").unwrap(),
            (r"C:\src\a.rs", 99)
        );
    }

    #[test]
    fn parse_location_rejects_missing_or_bad_line() {
        assert!(parse_location("src/a.rs").is_err());
        assert!(parse_location("src/a.rs:abc").is_err());
        assert!(parse_location("src/a.rs:0").is_err());
        assert!(parse_location(":12").is_err());
    }
}
