//! `anvil dashboard suppressions` — native suppressions-overview dashboard
//! (TDASH-004). Loads active suppressions via the shared
//! `services::suppressions` loader (expired/malformed already filtered) and
//! renders them: `--json` emits a stable envelope, non-TTY prints a plain
//! summary, TTY runs the Ratatui surface. No active suppressions is a
//! legitimate empty state.

use std::io::IsTerminal;

use anvil_tui::surfaces::dashboard::suppressions::{
    SuppressionRow, SuppressionsDashboardState, SuppressionsView,
};
use serde::Serialize;

use crate::services::suppressions::{SuppressionEntry, load_suppressions};
use crate::{GlobalArgs, tui, util};

/// Stable `--json` envelope: `count` plus the active suppressions array, so the
/// top-level shape is consistent whether or not any suppressions exist.
#[derive(Debug, Serialize)]
struct SuppressionsJson {
    count: usize,
    suppressions: Vec<SuppressionEntry>,
}

/// Run the suppressions dashboard. Returns how the surface exited so the
/// picker can return to itself on [`SurfaceExit::Back`]. Non-interactive
/// branches (`--json`, no-TTY) print and report `Quit`.
pub fn run(global: &GlobalArgs) -> anyhow::Result<tui::SurfaceExit> {
    let root = util::workspace_root()?;
    let entries = load_suppressions(&root);

    if global.json {
        let payload = SuppressionsJson {
            count: entries.len(),
            suppressions: entries,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(tui::SurfaceExit::Quit);
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        print_summary(&entries);
        return Ok(tui::SurfaceExit::Quit);
    }

    let view = view_from(&entries);
    let (_, exit) = tui::run_surface_with_exit(SuppressionsDashboardState::new(view))?;
    Ok(exit)
}

fn view_from(entries: &[SuppressionEntry]) -> SuppressionsView {
    SuppressionsView {
        rows: entries
            .iter()
            .map(|entry| SuppressionRow {
                pattern_id: entry.pattern_id.clone(),
                scope: entry.scope.clone(),
                file: entry.file.clone(),
                reason: entry.reason.clone(),
                expires_at: entry.expires_at.clone(),
            })
            .collect(),
    }
}

fn print_summary(entries: &[SuppressionEntry]) {
    if entries.is_empty() {
        println!("No active suppressions.");
        return;
    }
    println!("Active suppressions: {}", entries.len());
    for entry in entries {
        let expires = entry.expires_at.as_deref().unwrap_or("—");
        println!(
            "  {:<10}  {:<6}  {}  (expires {expires}) — {}",
            entry.pattern_id, entry.scope, entry.file, entry.reason
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(pattern_id: &str, expires: Option<&str>) -> SuppressionEntry {
        SuppressionEntry {
            pattern_id: pattern_id.to_string(),
            file: "a.ts".to_string(),
            scope: "file".to_string(),
            reason: "because".to_string(),
            expires_at: expires.map(str::to_string),
        }
    }

    #[test]
    fn view_preserves_rows_and_expiry() {
        let entries = vec![
            entry("AP-001", Some("2099-12-31T00:00:00Z")),
            entry("AP-002", None),
        ];
        let view = view_from(&entries);
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].pattern_id, "AP-001");
        assert_eq!(
            view.rows[0].expires_at.as_deref(),
            Some("2099-12-31T00:00:00Z")
        );
        assert!(view.rows[1].expires_at.is_none());
    }

    #[test]
    fn json_envelope_has_count_and_array() {
        let entries = vec![entry("AP-001", None)];
        let payload = SuppressionsJson {
            count: entries.len(),
            suppressions: entries,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"count\":1"), "got: {json}");
        assert!(json.contains("\"suppressions\""), "got: {json}");
        assert!(json.contains("AP-001"), "got: {json}");
    }

    #[test]
    fn empty_json_envelope_is_stable_shape() {
        let payload = SuppressionsJson {
            count: 0,
            suppressions: vec![],
        };
        let json = serde_json::to_string(&payload).unwrap();
        // Always an object with count + array, never null.
        assert!(json.contains("\"count\":0"), "got: {json}");
        assert!(json.contains("\"suppressions\":[]"), "got: {json}");
    }
}
