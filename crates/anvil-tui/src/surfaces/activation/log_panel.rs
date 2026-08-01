//! Tier-evidence log panel for the activation TUI (ACTTUI-006).
//!
//! The panel deliberately consumes presentation-safe evidence rows (typed
//! evidence from the CLI, plain activation output, or orchestrator lifecycle
//! lines). It does **not** read daemon skip tracing internals; daemon
//! attestation copy remains owned by the activation render layer.

use eddacraft_tui::prelude::{LogEntry, LogLevel, LogPanel, LogPanelState};
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

/// Convert the compact human verdict into log rows so the in-surface evidence
/// view always has the same install/tier facts as the plain contract.
#[must_use]
pub fn entries_from_verdict(verdict: &str) -> Vec<LogEntry> {
    entries_from_lines_with_prefix(verdict.lines(), "activation")
}

/// Convert orchestrator lifecycle/log lines into log rows.
#[must_use]
pub fn entries_from_lifecycle(lines: &[String]) -> Vec<LogEntry> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            let message = line.trim();
            if message.is_empty() {
                return None;
            }
            Some(entry(
                "activation-lifecycle",
                idx,
                classify_level(message),
                message,
                "orchestrator",
            ))
        })
        .collect()
}

fn entries_from_lines_with_prefix<'a>(
    lines: impl Iterator<Item = &'a str>,
    prefix: &str,
) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    let mut section = "activation".to_string();
    let mut mcp_client: Option<String> = None;

    for raw in lines {
        let trimmed = raw.trim();
        if trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("ACTIVATION")
            || trimmed == "ACTIVATION (verbose)"
        {
            continue;
        }

        match trimmed {
            "mcp:" => {
                section = "mcp".to_string();
                mcp_client = None;
                continue;
            }
            "install:" => {
                section = "install".to_string();
                mcp_client = None;
                continue;
            }
            "languages:" => {
                section = "languages".to_string();
                mcp_client = None;
                continue;
            }
            _ => {}
        }

        let indent = raw.chars().take_while(|ch| ch.is_whitespace()).count();
        // Top-level keys (indent <= 2) close any open section such as `mcp:`.
        // Without this reset a trailing `daemon-attestation:`/`why:` line would
        // be misattributed to the last MCP client block.
        if indent <= 2 {
            section = "activation".to_string();
            mcp_client = None;
        }
        if section == "mcp" && (4..6).contains(&indent) && trimmed.ends_with(':') {
            mcp_client = Some(trimmed.trim_end_matches(':').to_string());
            continue;
        }

        let (source, message) = source_and_message(&section, mcp_client.as_deref(), trimmed);
        let idx = entries.len();
        entries.push(entry(
            prefix,
            idx,
            classify_level(&message),
            message,
            source,
        ));
    }

    entries
}

fn source_and_message(section: &str, mcp_client: Option<&str>, row: &str) -> (String, String) {
    match section {
        "mcp" => {
            if let Some(client) = mcp_client {
                return (format!("mcp/{client}"), normalise_kv(row));
            }
            if let Some((client, tier)) = row.split_once(':') {
                return (
                    format!("mcp/{}", client.trim()),
                    format!("tier: {}", tier.trim()),
                );
            }
            ("mcp".to_string(), row.to_string())
        }
        "install" => {
            if let Some((client, detail)) = row.split_once(':') {
                return (
                    format!("install/{}", client.trim()),
                    detail.trim().to_string(),
                );
            }
            ("install".to_string(), row.to_string())
        }
        "languages" => ("languages".to_string(), row.to_string()),
        _ => {
            if let Some((key, value)) = row.split_once(':') {
                let key = key.trim();
                let source = match key {
                    "daemon-attestation" => "daemon",
                    "why" => "why",
                    "last_error" => "activation/error",
                    _ => "activation",
                };
                (source.to_string(), format!("{key}: {}", value.trim()))
            } else {
                ("activation".to_string(), row.to_string())
            }
        }
    }
}

fn normalise_kv(row: &str) -> String {
    if let Some((key, value)) = row.split_once(':') {
        format!("{}: {}", key.trim(), value.trim())
    } else {
        row.to_string()
    }
}

fn classify_level(message: &str) -> LogLevel {
    let lower = message.to_ascii_lowercase();
    if lower.contains("failed") || lower.contains("last_error") || lower.contains("error") {
        LogLevel::Error
    } else if lower.contains("refused")
        || lower.contains("unsafe")
        || lower.contains("quarantined")
        || lower.contains("stale")
        || lower.contains("not running")
        || lower.contains("not registered")
        || lower.contains("unreachable")
    {
        LogLevel::Warn
    } else if lower.contains("skipped")
        || lower.contains("deferred")
        || lower.contains("not probed")
    {
        LogLevel::Debug
    } else {
        LogLevel::Info
    }
}

fn entry(
    prefix: &str,
    idx: usize,
    level: LogLevel,
    message: impl Into<String>,
    source: impl Into<String>,
) -> LogEntry {
    LogEntry::new(
        format!("{prefix}-{idx:03}"),
        format!("{idx:02}"),
        level,
        message.into(),
        source.into(),
    )
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    entries: &[LogEntry],
    state: &mut LogPanelState,
    theme: &EddaCraftTheme,
) {
    render_with_title(frame, area, entries, state, theme, "Tier evidence", true);
}

pub fn render_activity(
    frame: &mut Frame,
    area: Rect,
    entries: &[LogEntry],
    state: &mut LogPanelState,
    theme: &EddaCraftTheme,
) {
    render_with_title(
        frame,
        area,
        entries,
        state,
        theme,
        "Activation activity",
        false,
    );
}

fn render_with_title(
    frame: &mut Frame,
    area: Rect,
    entries: &[LogEntry],
    state: &mut LogPanelState,
    theme: &EddaCraftTheme,
    title: &str,
    interactive: bool,
) {
    let chrome_height = if interactive { 5 } else { 2 };
    let panel = LogPanel::new(entries, theme)
        .title(title)
        .focused(true)
        .max_visible(usize::from(area.height.saturating_sub(chrome_height)).max(1));
    let panel = if interactive {
        panel
    } else {
        panel.show_filter(false).show_search(false).show_help(false)
    };
    panel.render(area, frame.buffer_mut(), state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn activity_panel_uses_rows_freed_by_hidden_chrome() {
        let entries: Vec<_> = (0..6)
            .map(|idx| {
                entry(
                    "activity",
                    idx,
                    LogLevel::Info,
                    format!("activity row {idx}"),
                    "orchestrator",
                )
            })
            .collect();
        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = LogPanelState::default();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render_activity(frame, frame.area(), &entries, &mut state, &theme))
            .unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(rendered.contains("activity row 5"));
    }

    #[test]
    fn parses_install_detail_from_compact_verdict() {
        let verdict = "ACTIVATION\n  state: protecting\n  mcp:\n    Cursor: live_validation\n  install:\n    Cursor: skipped — already up to date\n";

        let entries = entries_from_verdict(verdict);

        assert!(entries.iter().any(|entry| {
            entry.source == "mcp/Cursor" && entry.message == "tier: live_validation"
        }));
        assert!(entries.iter().any(|entry| {
            entry.source == "install/Cursor" && entry.message == "skipped — already up to date"
        }));
    }
}
