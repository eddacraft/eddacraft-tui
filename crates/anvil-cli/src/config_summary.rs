use std::path::Path;

use anvil_config::{ConfigFormat, RuleModes, discover, parse_file, parse_str};

/// CIB-256 (START-1): the source line is keyed `rule source:`, not
/// `config:`. `anvil start` splices this summary onto the activation block,
/// which already spends a top-level `config:` key on a different question —
/// "has anvil set this repository up?" (`ConfigStatus`) versus "where did
/// these rule modes come from?" (this line). Two meanings under one label
/// read as a contradiction (`config: absent` then `config: defaults` in the
/// same block). The standalone `anvil config` command renders no activation
/// block, so it keeps the plain `config:` key.
pub fn render_rule_mode_summary(root: &Path) -> String {
    let (path, value) = match load_config_value(root) {
        ConfigLoad::Loaded { path, value } => (path, value),
        ConfigLoad::Missing => (String::from("defaults"), serde_json::json!({})),
        ConfigLoad::Invalid { path, error } => {
            return format!("  rule modes: invalid ({error})\n  rule source: {path}\n");
        }
    };

    match RuleModes::from_value(&value) {
        Ok(modes) => format!("  rule modes: {}\n  rule source: {path}\n", modes.summary()),
        Err(error) => format!("  rule modes: invalid ({error})\n  rule source: {path}\n"),
    }
}

enum ConfigLoad {
    Loaded {
        path: String,
        value: serde_json::Value,
    },
    Invalid {
        path: String,
        error: String,
    },
    Missing,
}

fn load_config_value(root: &Path) -> ConfigLoad {
    match discover(root, ".anvil") {
        Ok(Some(discovered)) => {
            let path = discovered
                .path
                .strip_prefix(root)
                .unwrap_or(&discovered.path)
                .to_string_lossy()
                .into_owned();
            return match parse_file(&discovered.path) {
                Ok(value) => ConfigLoad::Loaded { path, value },
                Err(error) => ConfigLoad::Invalid {
                    path,
                    error: error.to_string(),
                },
            };
        }
        Ok(None) => {}
        Err(error) => {
            return ConfigLoad::Invalid {
                path: String::from(".anvil.*"),
                error: error.to_string(),
            };
        }
    }

    let rc_path = root.join(".anvilrc");
    let contents = match std::fs::read_to_string(&rc_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return ConfigLoad::Missing,
        Err(error) => {
            return ConfigLoad::Invalid {
                path: String::from(".anvilrc"),
                error: error.to_string(),
            };
        }
    };
    match serde_json::from_str(&contents)
        .or_else(|_| parse_str(&contents, ConfigFormat::Yaml, &rc_path))
    {
        Ok(value) => ConfigLoad::Loaded {
            path: String::from(".anvilrc"),
            value,
        },
        Err(error) => ConfigLoad::Invalid {
            path: String::from(".anvilrc"),
            error: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn renders_default_advisory_modes_without_config() {
        let tmp = TempDir::new().unwrap();

        let summary = render_rule_mode_summary(tmp.path());

        assert!(summary.contains("public-api-expansion=warn"));
        assert!(summary.contains("new-dependency-introduction=warn"));
        assert!(summary.contains("cross-layer-violation=warn"));
        assert!(summary.contains("privilege-expansion=warn"));
        // CIB-256: keyed `rule source:` so it cannot collide with the
        // activation block's `config:` key when `anvil start` splices them.
        assert!(summary.contains("rule source: defaults"), "{summary}");
        assert!(
            !summary.lines().any(|l| l.starts_with("  config:")),
            "the rule-mode summary must not claim the top-level `config:` key: {summary}"
        );
    }

    /// legacy-fallback coverage (.anvilrc deliberately): pins the
    /// legacy branch of `load_config_value` after the incidental
    /// status-surface coverage converted to the canonical name.
    #[test]
    fn renders_modes_from_legacy_anvilrc_fallback() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"enforcement":{"rules":{"public-api-expansion":{"mode":"off"}}}}"#,
        )
        .unwrap();
        let summary = render_rule_mode_summary(tmp.path());
        assert!(summary.contains("public-api-expansion=off"), "{summary}");
        assert!(summary.contains("rule source: .anvilrc"), "{summary}");
    }

    #[test]
    fn renders_modes_from_discovered_config() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            r"
enforcement:
  rules:
    public-api-expansion:
      mode: off
    new-dependency-introduction:
      mode: warn
    cross-layer-violation:
      mode: enforce
    privilege-expansion:
      mode: block
",
        )
        .unwrap();

        let summary = render_rule_mode_summary(tmp.path());

        assert!(summary.contains("public-api-expansion=off"));
        assert!(summary.contains("new-dependency-introduction=warn"));
        assert!(summary.contains("cross-layer-violation=enforce"));
        assert!(summary.contains("privilege-expansion=enforce"));
        assert!(summary.contains("rule source: .anvil.yaml"), "{summary}");
    }

    #[test]
    fn renders_invalid_when_discovered_config_does_not_parse() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "enforcement: [").unwrap();

        let summary = render_rule_mode_summary(tmp.path());

        assert!(summary.contains("rule modes: invalid"));
        assert!(summary.contains("rule source: .anvil.yaml"), "{summary}");
    }

    #[test]
    fn renders_invalid_when_configured_mode_is_unknown() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            r"
enforcement:
  rules:
    public-api-expansion:
      mode: enfroce
",
        )
        .unwrap();

        let summary = render_rule_mode_summary(tmp.path());

        assert!(summary.contains("rule modes: invalid"));
        assert!(summary.contains("enfroce"));
        assert!(summary.contains("rule source: .anvil.yaml"), "{summary}");
    }

    #[test]
    fn renders_invalid_when_discovered_config_shape_is_invalid() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "").unwrap();

        let summary = render_rule_mode_summary(tmp.path());

        assert!(summary.contains("rule modes: invalid"));
        assert!(summary.contains("rule source: .anvil.yaml"), "{summary}");
    }
}
