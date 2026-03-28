pub mod json;
pub mod plain;

/// Determines how command output is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Tui,
    Plain,
    Json,
}

impl OutputMode {
    /// Priority: `--json` > `--no-tui` > TTY detection.
    pub fn resolve(json: bool, no_tui: bool, is_tty: bool) -> Self {
        if json {
            Self::Json
        } else if no_tui || !is_tty {
            Self::Plain
        } else {
            Self::Tui
        }
    }

    /// Convenience: resolve from [`GlobalArgs`] + stdout TTY check.
    pub fn from_global(global: &crate::GlobalArgs) -> Self {
        use std::io::IsTerminal;
        Self::resolve(global.json, global.no_tui, std::io::stdout().is_terminal())
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
}
