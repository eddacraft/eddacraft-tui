use std::ffi::OsStr;
use std::path::Path;

/// One of the four recognised on-disk formats.
///
/// Ordering matches the **detection precedence** required by MLP-011:
/// when more than one file with the same basename exists, `Yaml` wins
/// over `Yml`, `Yml` wins over `Json`, `Json` wins over `Toml`. The
/// `Ord` derive is deliberate so callers can compare without spelling
/// the rule out themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConfigFormat {
    Yaml,
    Yml,
    Json,
    Toml,
}

impl ConfigFormat {
    /// The on-disk extension, without the leading dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Yml => "yml",
            Self::Json => "json",
            Self::Toml => "toml",
        }
    }

    /// Recognise a format from a path's extension. Case-insensitive on
    /// the extension itself; returns `None` if the path has no
    /// extension or the extension is unrecognised.
    pub fn from_path(path: &Path) -> Option<Self> {
        Self::from_extension(path.extension()?)
    }

    /// Recognise a format from a raw extension `OsStr` (the value
    /// returned by `Path::extension`). Case-insensitive.
    pub fn from_extension(ext: &OsStr) -> Option<Self> {
        // Case-insensitive match on ASCII-lowercase form. Extensions
        // are always ASCII so a UTF-8 conversion of the lowercased
        // bytes is safe.
        let ext = ext.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "yaml" => Some(Self::Yaml),
            "yml" => Some(Self::Yml),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_order_yaml_yml_json_toml() {
        // Spec: yaml > yml > json > toml.
        assert!(ConfigFormat::Yaml < ConfigFormat::Yml);
        assert!(ConfigFormat::Yml < ConfigFormat::Json);
        assert!(ConfigFormat::Json < ConfigFormat::Toml);
    }

    #[test]
    fn extension_round_trip() {
        for fmt in [
            ConfigFormat::Yaml,
            ConfigFormat::Yml,
            ConfigFormat::Json,
            ConfigFormat::Toml,
        ] {
            let path = std::path::PathBuf::from(format!("config.{}", fmt.extension()));
            assert_eq!(ConfigFormat::from_path(&path), Some(fmt));
        }
    }

    #[test]
    fn extension_is_case_insensitive() {
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.YAML")),
            Some(ConfigFormat::Yaml),
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("Config.Json")),
            Some(ConfigFormat::Json),
        );
    }

    #[test]
    fn unknown_extension_is_none() {
        assert!(ConfigFormat::from_path(Path::new("config.ini")).is_none());
        assert!(ConfigFormat::from_path(Path::new("config")).is_none());
    }
}
