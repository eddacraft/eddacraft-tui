/// Controls which engine runs checks.
///
/// - `Rust` — uses the Rust kernel (fully functional).
/// - `Legacy` — delegates to the JS engine (stub, not yet implemented).
/// - `Dual` — runs both engines and diffs results (stub, not yet implemented).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMode {
    Rust,
    Legacy,
    Dual,
}

impl EngineMode {
    /// Parse from a CLI string argument.
    pub fn from_str_arg(s: &str) -> Result<Self, EngineModeError> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(Self::Rust),
            "legacy" => Ok(Self::Legacy),
            "dual" => Ok(Self::Dual),
            _ => Err(EngineModeError::UnknownMode(s.to_string())),
        }
    }

    /// Returns whether this mode is currently functional.
    pub fn is_implemented(self) -> bool {
        matches!(self, Self::Rust)
    }
}

impl Default for EngineMode {
    fn default() -> Self {
        Self::Rust
    }
}

impl std::fmt::Display for EngineMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
            Self::Legacy => write!(f, "legacy"),
            Self::Dual => write!(f, "dual"),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum EngineModeError {
    #[error("unknown engine mode '{0}' — expected rust, legacy, or dual")]
    UnknownMode(String),
    #[error("engine mode '{0}' is not yet implemented")]
    NotImplemented(EngineMode),
}

/// Validate that the selected mode can actually run.
/// Returns `Ok(())` for Rust mode, `Err` for Legacy and Dual.
pub fn validate_mode(mode: EngineMode) -> Result<(), EngineModeError> {
    if mode.is_implemented() {
        Ok(())
    } else {
        Err(EngineModeError::NotImplemented(mode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_mode_runs() {
        let mode = EngineMode::from_str_arg("rust").unwrap();
        assert_eq!(mode, EngineMode::Rust);
        assert!(validate_mode(mode).is_ok());
    }

    #[test]
    fn legacy_mode_not_implemented() {
        let mode = EngineMode::from_str_arg("legacy").unwrap();
        assert_eq!(mode, EngineMode::Legacy);
        assert!(validate_mode(mode).is_err());
    }

    #[test]
    fn dual_mode_not_implemented() {
        let mode = EngineMode::from_str_arg("dual").unwrap();
        assert_eq!(mode, EngineMode::Dual);
        assert!(validate_mode(mode).is_err());
    }

    #[test]
    fn unknown_mode_rejected() {
        assert!(EngineMode::from_str_arg("turbo").is_err());
    }

    #[test]
    fn case_insensitive_parsing() {
        assert_eq!(EngineMode::from_str_arg("RUST").unwrap(), EngineMode::Rust);
        assert_eq!(
            EngineMode::from_str_arg("Legacy").unwrap(),
            EngineMode::Legacy
        );
        assert_eq!(EngineMode::from_str_arg("DUAL").unwrap(), EngineMode::Dual);
    }

    #[test]
    fn default_is_rust() {
        assert_eq!(EngineMode::default(), EngineMode::Rust);
    }

    #[test]
    fn display_roundtrips() {
        assert_eq!(
            EngineMode::from_str_arg(&EngineMode::Rust.to_string()).unwrap(),
            EngineMode::Rust
        );
        assert_eq!(
            EngineMode::from_str_arg(&EngineMode::Legacy.to_string()).unwrap(),
            EngineMode::Legacy
        );
        assert_eq!(
            EngineMode::from_str_arg(&EngineMode::Dual.to_string()).unwrap(),
            EngineMode::Dual
        );
    }
}
