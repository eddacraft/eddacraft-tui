use std::path::{Path, PathBuf};

use crate::format::ConfigFormat;

/// The detection precedence: yaml → yml → json → toml. Exposed so
/// consumers can document the rule alongside their own discovery
/// surfaces without re-spelling it.
pub const DISCOVER_PRECEDENCE: [ConfigFormat; 4] = [
    ConfigFormat::Yaml,
    ConfigFormat::Yml,
    ConfigFormat::Json,
    ConfigFormat::Toml,
];

/// The result of a successful [`discover`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredConfig {
    pub path: PathBuf,
    pub format: ConfigFormat,
}

/// Find the first file matching `<basename>.<ext>` in `dir`, honouring
/// the precedence in [`DISCOVER_PRECEDENCE`].
///
/// Returns `Ok(None)` when no candidate file exists. The function
/// never reads file contents; it only checks for presence.
///
/// `basename` is the filename without extension — for `anvil/policy.*`
/// pass `"policy"` with `dir = Path::new("anvil")`; for `.anvil.*`
/// pass `".anvil"` with `dir = workspace_root`.
///
/// **Case sensitivity.** Discovery uses lowercase extensions only
/// (`.yaml`, `.yml`, `.json`, `.toml`). This is a deliberate choice:
/// project conventions in Anvil are lowercase by default, and
/// directory scans for case variants would be both slow and ambiguous
/// (which precedence wins between `policy.YAML` and `policy.yml` on
/// the same case-sensitive disk?). When a caller already has a path
/// — e.g. read from a CLI argument or env var — they can use
/// [`ConfigFormat::from_path`], which IS case-insensitive on the
/// extension, to recognise the format. Discovery and recognition are
/// deliberately split along this axis.
pub fn discover(dir: &Path, basename: &str) -> std::io::Result<Option<DiscoveredConfig>> {
    for &format in &DISCOVER_PRECEDENCE {
        let candidate = dir.join(format!("{basename}.{}", format.extension()));
        match candidate.try_exists() {
            Ok(true) => {
                return Ok(Some(DiscoveredConfig {
                    path: candidate,
                    format,
                }));
            }
            // Ok(false) falls through to the next format candidate.
            Ok(false) => {}
            // Surface the underlying io error rather than papering over
            // it; a permission-denied on the directory shouldn't be
            // silently confused with "no config exists".
            Err(e) => return Err(e),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn touch(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), b"{}").unwrap();
    }

    #[test]
    fn returns_none_when_no_match() {
        let dir = TempDir::new().unwrap();
        let result = discover(dir.path(), "policy").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn finds_yaml() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.yaml");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Yaml);
        assert_eq!(result.path, dir.path().join("policy.yaml"));
    }

    #[test]
    fn finds_yml() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.yml");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Yml);
    }

    #[test]
    fn finds_json() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.json");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Json);
    }

    #[test]
    fn finds_toml() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.toml");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Toml);
    }

    #[test]
    fn yaml_beats_yml() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.yaml");
        touch(dir.path(), "policy.yml");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Yaml);
    }

    #[test]
    fn yml_beats_json() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.yml");
        touch(dir.path(), "policy.json");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Yml);
    }

    #[test]
    fn json_beats_toml() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.json");
        touch(dir.path(), "policy.toml");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Json);
    }

    #[test]
    fn yaml_beats_all_others() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "policy.yaml");
        touch(dir.path(), "policy.yml");
        touch(dir.path(), "policy.json");
        touch(dir.path(), "policy.toml");
        let result = discover(dir.path(), "policy").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Yaml);
    }

    #[test]
    fn dotfile_basename_works() {
        // `.anvil.yaml` is a real intended use case (dotfile with
        // extension). Confirm the basename `.anvil` resolves correctly.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), ".anvil.toml");
        let result = discover(dir.path(), ".anvil").unwrap().unwrap();
        assert_eq!(result.format, ConfigFormat::Toml);
        assert_eq!(result.path, dir.path().join(".anvil.toml"));
    }
}
