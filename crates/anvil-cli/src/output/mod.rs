pub mod json;
pub mod plain;
pub mod sarif;

/// Sentinel error: the command already printed its output and only needs
/// `main` to exit with `EXIT_ERROR` without reprinting the message.
#[derive(Debug)]
pub struct AlreadyReported;

impl std::fmt::Display for AlreadyReported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("output already reported")
    }
}

impl std::error::Error for AlreadyReported {}

/// Sentinel error: the command already printed an auth-required message
/// and needs `main` to exit with `EXIT_AUTH_REQUIRED`.
#[derive(Debug)]
pub struct AuthRequired;

impl std::fmt::Display for AuthRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("authentication required")
    }
}

impl std::error::Error for AuthRequired {}

/// The output selector (`--format <FORMAT>`), an opt-in value-enum that
/// supersedes the legacy `--json` / `--no-tui` booleans (ADR-056).
///
/// `--format` is a per-command flag on the finding-emitting commands
/// (`check` / `gate` / `audit`), not global — `--format` already collides with
/// the domain flags on `export` / `validate`. `--json` / `--no-tui` stay global
/// and are honoured as aliases: `from_command_format` resolves `--format`
/// together with those booleans at read time (no write-back), so an explicit
/// `--format` wins and an absent/`auto` `--format` defers to them.
///
/// `Auto` (the default) defers to the legacy TUI/Plain/JSON truth table.
/// `Sarif` is **never** auto-selected — it must be named explicitly, and clap
/// only exposes `--format` (hence `sarif`) on the finding-emitting commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Format {
    /// Pick the renderer from `--json` / `--no-tui` + TTY detection.
    #[default]
    Auto,
    /// Force the interactive TUI renderer.
    Tui,
    /// Force plain-text output.
    Plain,
    /// Force the bespoke per-command JSON shape (same as `--json`).
    Json,
    /// Emit SARIF 2.1.0 (finding-emitting commands only).
    Sarif,
}

/// Determines how command output is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Tui,
    Plain,
    Json,
    Sarif,
}

impl OutputMode {
    /// Legacy resolver for the `--json` / `--no-tui` booleans.
    ///
    /// Priority: `--json` > `--no-tui` > TTY detection. Retained because it is
    /// the `Format::Auto` fall-through and keeps the existing boolean contract
    /// intact; `--format` callers go through [`Self::resolve_format`].
    pub fn resolve(json: bool, no_tui: bool, is_tty: bool) -> Self {
        if json {
            Self::Json
        } else if no_tui || !is_tty {
            Self::Plain
        } else {
            Self::Tui
        }
    }

    /// Single precedence-ordered resolver for `--format` (ADR-056).
    ///
    /// An explicit non-`auto` `--format` wins outright; otherwise the legacy
    /// `--json` / `--no-tui` / TTY truth table applies. SARIF is reachable
    /// **only** via an explicit `--format sarif`, never through `auto`/TTY
    /// detection.
    pub fn resolve_format(format: Option<Format>, json: bool, no_tui: bool, is_tty: bool) -> Self {
        match format {
            Some(Format::Sarif) => Self::Sarif,
            Some(Format::Json) => Self::Json,
            Some(Format::Plain) => Self::Plain,
            Some(Format::Tui) => Self::Tui,
            Some(Format::Auto) | None => Self::resolve(json, no_tui, is_tty),
        }
    }

    /// Convenience: resolve the legacy `--json` / `--no-tui` booleans from
    /// [`GlobalArgs`] + stdout TTY check. Used by commands that have no
    /// `--format` flag of their own (everything except the finding-emitting
    /// commands), so it can never yield [`OutputMode::Sarif`].
    pub fn from_global(global: &crate::GlobalArgs) -> Self {
        use std::io::IsTerminal;
        Self::resolve(global.json, global.no_tui, std::io::stdout().is_terminal())
    }

    /// Resolve a finding-emitting command's local `--format` against the global
    /// `--json` / `--no-tui` booleans + stdout TTY check (ADR-056).
    ///
    /// `--format` is per-command (only check / gate / audit) because `--format`
    /// is already a domain flag on `export` / `validate`; a global one would
    /// collide. The precedence + alias semantics are identical to a global
    /// selector — `--format` wins, then `--json`, then `--no-tui` / non-TTY.
    pub fn from_command_format(format: Option<Format>, global: &crate::GlobalArgs) -> Self {
        use std::io::IsTerminal;
        Self::resolve_format(
            format,
            global.json,
            global.no_tui,
            std::io::stdout().is_terminal(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_flag_selects_json() {
        assert_eq!(OutputMode::resolve(true, false, true), OutputMode::Json);
    }

    #[test]
    fn no_tui_flag_selects_plain() {
        assert_eq!(OutputMode::resolve(false, true, true), OutputMode::Plain);
    }

    #[test]
    fn non_tty_selects_plain() {
        assert_eq!(OutputMode::resolve(false, false, false), OutputMode::Plain);
    }

    #[test]
    fn tty_with_no_flags_selects_tui() {
        assert_eq!(OutputMode::resolve(false, false, true), OutputMode::Tui);
    }

    #[test]
    fn json_overrides_no_tui() {
        assert_eq!(OutputMode::resolve(true, true, true), OutputMode::Json);
    }

    #[test]
    fn json_overrides_non_tty() {
        assert_eq!(OutputMode::resolve(true, false, false), OutputMode::Json);
    }

    // ── `--format` resolver (ADR-056) ───────────────────────────────

    #[test]
    fn explicit_format_sarif_wins() {
        // SARIF is selected regardless of the legacy booleans / TTY.
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Sarif), false, false, true),
            OutputMode::Sarif
        );
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Sarif), true, true, false),
            OutputMode::Sarif
        );
    }

    #[test]
    fn explicit_format_json_matches_json_flag() {
        // `--format json` parity with `--json` (the documented alias).
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Json), false, false, true),
            OutputMode::resolve(true, false, true),
        );
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Json), false, false, true),
            OutputMode::Json
        );
    }

    #[test]
    fn explicit_format_plain_and_tui() {
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Plain), false, false, true),
            OutputMode::Plain
        );
        // `--format tui` forces the TUI even on a non-TTY.
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Tui), false, false, false),
            OutputMode::Tui
        );
        // An explicit non-auto `--format` wins over the legacy booleans:
        // `--format plain` beats `--json`, `--format tui` beats `--no-tui`.
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Plain), true, false, true),
            OutputMode::Plain
        );
        assert_eq!(
            OutputMode::resolve_format(Some(Format::Tui), false, true, true),
            OutputMode::Tui
        );
    }

    #[test]
    fn format_auto_falls_through_to_legacy_resolver() {
        for (json, no_tui, tty) in [
            (false, false, true),
            (false, true, true),
            (false, false, false),
            (true, false, true),
        ] {
            assert_eq!(
                OutputMode::resolve_format(Some(Format::Auto), json, no_tui, tty),
                OutputMode::resolve(json, no_tui, tty),
            );
            assert_eq!(
                OutputMode::resolve_format(None, json, no_tui, tty),
                OutputMode::resolve(json, no_tui, tty),
            );
        }
    }

    #[test]
    fn sarif_is_never_auto_selected() {
        // No combination of legacy flags + TTY can yield SARIF without an
        // explicit `--format sarif`.
        for fmt in [None, Some(Format::Auto)] {
            for json in [false, true] {
                for no_tui in [false, true] {
                    for tty in [false, true] {
                        assert_ne!(
                            OutputMode::resolve_format(fmt, json, no_tui, tty),
                            OutputMode::Sarif,
                        );
                    }
                }
            }
        }
    }
}
