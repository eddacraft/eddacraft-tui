/// Controls which engine runs checks.
///
/// Only the Rust kernel is supported. The Legacy and Dual modes were removed
/// as unimplemented stubs that would never ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EngineMode {
    #[default]
    Rust,
}

impl EngineMode {
    /// Parse from a CLI string argument.
    pub fn from_str_arg(s: &str) -> Result<Self, EngineModeError> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(Self::Rust),
            _ => Err(EngineModeError::UnknownMode(s.to_string())),
        }
    }
}

impl std::fmt::Display for EngineMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum EngineModeError {
    #[error("unknown engine mode '{0}' — expected rust")]
    UnknownMode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_mode_parses() {
        let mode = EngineMode::from_str_arg("rust").unwrap();
        assert_eq!(mode, EngineMode::Rust);
    }

    #[test]
    fn unknown_mode_rejected() {
        assert!(EngineMode::from_str_arg("turbo").is_err());
        assert!(EngineMode::from_str_arg("legacy").is_err());
        assert!(EngineMode::from_str_arg("dual").is_err());
    }

    #[test]
    fn case_insensitive_parsing() {
        assert_eq!(EngineMode::from_str_arg("RUST").unwrap(), EngineMode::Rust);
        assert_eq!(EngineMode::from_str_arg("Rust").unwrap(), EngineMode::Rust);
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
    }
}
