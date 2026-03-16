/// TUI rendering backend selection for the Ink-to-Ratatui migration.
///
/// Default is `Ink` until Ratatui feature parity is validated.
/// Users opt in with `--tui=ratatui`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiBackend {
    Ink,
    Ratatui,
}

impl TuiBackend {
    /// Parse from a CLI string argument.
    pub fn from_str_arg(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "ink" => Ok(Self::Ink),
            "ratatui" => Ok(Self::Ratatui),
            _ => Err(format!(
                "unknown TUI backend '{s}' — expected ink or ratatui"
            )),
        }
    }
}

impl Default for TuiBackend {
    fn default() -> Self {
        Self::Ink
    }
}

impl std::fmt::Display for TuiBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ink => write!(f, "ink"),
            Self::Ratatui => write!(f, "ratatui"),
        }
    }
}

/// Select the TUI backend based on user preference.
/// Falls back to `Ink` when no preference is given.
pub fn select_backend(preference: Option<TuiBackend>) -> TuiBackend {
    preference.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_returns_ink() {
        assert_eq!(select_backend(None), TuiBackend::Ink);
    }

    #[test]
    fn explicit_ratatui_preference_respected() {
        assert_eq!(
            select_backend(Some(TuiBackend::Ratatui)),
            TuiBackend::Ratatui
        );
    }

    #[test]
    fn explicit_ink_preference_respected() {
        assert_eq!(select_backend(Some(TuiBackend::Ink)), TuiBackend::Ink);
    }

    #[test]
    fn parse_ink_string() {
        assert_eq!(TuiBackend::from_str_arg("ink").unwrap(), TuiBackend::Ink);
    }

    #[test]
    fn parse_ratatui_string() {
        assert_eq!(
            TuiBackend::from_str_arg("ratatui").unwrap(),
            TuiBackend::Ratatui
        );
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(TuiBackend::from_str_arg("INK").unwrap(), TuiBackend::Ink);
        assert_eq!(
            TuiBackend::from_str_arg("Ratatui").unwrap(),
            TuiBackend::Ratatui
        );
    }

    #[test]
    fn parse_unknown_rejected() {
        assert!(TuiBackend::from_str_arg("ncurses").is_err());
    }

    #[test]
    fn display_roundtrips() {
        assert_eq!(
            TuiBackend::from_str_arg(&TuiBackend::Ink.to_string()).unwrap(),
            TuiBackend::Ink
        );
        assert_eq!(
            TuiBackend::from_str_arg(&TuiBackend::Ratatui.to_string()).unwrap(),
            TuiBackend::Ratatui
        );
    }
}
