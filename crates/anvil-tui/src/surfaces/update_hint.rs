//! Shared DTO for the DISTRIB-002 "update available" hint.
//!
//! Both the status surface and the watch surface render this hint
//! identically: one short line that names the upgrade and any attached
//! security advisory. The render itself lives in each surface's
//! `render.rs`; this module owns only the data shape so the two
//! consumers stay in sync.

/// One-line update-available hint, ready for render. anvil-cli computes
/// instances by combining the latest-version probe, the running
/// version, and the rate-limit gate. anvil-tui treats the DTO as
/// opaque-data-plus-format-helper — no logic, just rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateHint {
    /// The version advertised by the latest-release probe.
    pub latest_version: String,
    /// The running version (compile-time `CARGO_PKG_VERSION`).
    pub current_version: String,
    /// Optional advisory IDs attached to the running version, e.g.
    /// `GHSA-aaaa-bbbb-cccc`. Empty when the running version carries
    /// no advisory tags, or when `--check` was not requested at probe
    /// time. Surfaces should render the IDs verbatim — they are the
    /// actionable tokens a user pastes into GitHub's advisory search.
    pub advisory_ids: Vec<String>,
}

impl UpdateHint {
    /// Format the hint as the single line both surfaces render. Kept
    /// here (not in render.rs) so the watch and status renders cannot
    /// drift on wording.
    ///
    /// ASCII-only — the watch TUI claims ASCII output to survive
    /// Windows cp1252 consoles and CI log captures (see comment on
    /// `render_update_hint` in watch/render.rs). Use `->` not `→`,
    /// `--` not `—`.
    #[must_use]
    pub fn render_line(&self) -> String {
        if self.advisory_ids.is_empty() {
            format!(
                "Update available: anvil {} -> {} (run `anvil update`)",
                self.current_version, self.latest_version
            )
        } else {
            format!(
                "Update available: anvil {} -> {} -- security advisory: {} (run `anvil update`)",
                self.current_version,
                self.latest_version,
                self.advisory_ids.join(", "),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_line_without_advisories_lists_versions_and_command() {
        let hint = UpdateHint {
            latest_version: "0.7.0-beta".into(),
            current_version: "0.6.2-beta".into(),
            advisory_ids: vec![],
        };
        let line = hint.render_line();
        assert!(line.contains("0.6.2-beta"));
        assert!(line.contains("0.7.0-beta"));
        assert!(line.contains("anvil update"));
        assert!(
            !line.contains("advisory"),
            "no advisory text when list is empty"
        );
    }

    #[test]
    fn render_line_with_advisory_names_id_in_one_line() {
        let hint = UpdateHint {
            latest_version: "0.7.0-beta".into(),
            current_version: "0.6.2-beta".into(),
            advisory_ids: vec!["GHSA-aaaa-bbbb-cccc".into()],
        };
        let line = hint.render_line();
        assert!(line.contains("GHSA-aaaa-bbbb-cccc"));
        // One line — no newlines anywhere.
        assert!(!line.contains('\n'));
    }

    #[test]
    fn render_line_with_multiple_advisories_comma_separates() {
        let hint = UpdateHint {
            latest_version: "0.7.0-beta".into(),
            current_version: "0.6.2-beta".into(),
            advisory_ids: vec!["GHSA-aaaa".into(), "CVE-2026-1234".into()],
        };
        let line = hint.render_line();
        assert!(line.contains("GHSA-aaaa, CVE-2026-1234"));
    }

    #[test]
    fn render_line_is_pure_ascii() {
        // The watch surface claims ASCII output for Windows cp1252
        // safety. The arrow / dash characters in the rendered line
        // must stay encodable in cp1252 (which excludes U+2192 `→`
        // and U+2014 `—`).
        let hint = UpdateHint {
            latest_version: "0.7.0-beta".into(),
            current_version: "0.6.2-beta".into(),
            advisory_ids: vec!["GHSA-aaaa".into()],
        };
        let line = hint.render_line();
        assert!(
            line.is_ascii(),
            "hint line contains non-ASCII chars: {line:?}"
        );
    }
}
