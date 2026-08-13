use std::path::{Component, Path, PathBuf};

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

/// Typed discovery failures. Invalid basenames become
/// [`std::io::ErrorKind::InvalidInput`] so existing `io::Result`
/// callers keep compiling; match the inner error to distinguish
/// hygiene failures from filesystem errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DiscoverError {
    #[error(
        "basename {basename:?} is not a single filename component — \
absolute paths, separators, and '.' / '..' are rejected"
    )]
    InvalidBasename { basename: String },
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
/// `basename` must be exactly one filename component. Absolute paths,
/// separators (`/` / `\`), `.`, `..`, empty strings, Windows drive
/// prefixes, and UNC forms are rejected as
/// [`std::io::ErrorKind::InvalidInput`] wrapping
/// [`DiscoverError::InvalidBasename`] before any filesystem probe.
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
    validate_basename(basename)?;
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

fn validate_basename(basename: &str) -> Result<(), DiscoverError> {
    if !is_single_filename_component(basename) {
        return Err(DiscoverError::InvalidBasename {
            basename: basename.to_string(),
        });
    }
    Ok(())
}

/// True only when `basename` is exactly one `Normal` path component
/// and cannot rewrite `dir` on Unix or Windows. Textual checks run
/// before `Path` parsing so `foo/../bar` cannot collapse into `bar`.
fn is_single_filename_component(basename: &str) -> bool {
    if basename.is_empty() || basename.contains('\0') {
        return false;
    }
    if basename.contains('/') || basename.contains('\\') {
        return false;
    }
    // Reject Windows drive / UNC forms even on Unix so a hostile
    // basename cannot become absolute after `Path::join` on Windows.
    let bytes = basename.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return false;
    }
    let mut components = Path::new(basename).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

impl From<DiscoverError> for std::io::Error {
    fn from(err: DiscoverError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, err)
    }
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

    fn assert_invalid_basename(err: &std::io::Error, expected: &str) {
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        let typed = err
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<DiscoverError>())
            .expect("invalid basename should carry typed DiscoverError");
        assert_eq!(
            typed,
            &DiscoverError::InvalidBasename {
                basename: expected.to_string(),
            }
        );
    }

    #[test]
    fn rejects_parent_dir_basename_without_probing_outside() {
        let root = TempDir::new().unwrap();
        let dir = root.path().join("project");
        std::fs::create_dir(&dir).unwrap();
        let outside = root.path().join("attacker");
        std::fs::create_dir(&outside).unwrap();
        touch(&outside, "policy.yaml");

        let err = discover(&dir, "../attacker/policy").unwrap_err();
        assert_invalid_basename(&err, "../attacker/policy");
    }

    #[test]
    fn rejects_absolute_basename() {
        let dir = TempDir::new().unwrap();
        let err = discover(dir.path(), "/tmp/outside-policy").unwrap_err();
        assert_invalid_basename(&err, "/tmp/outside-policy");
    }

    #[test]
    fn rejects_separator_backslash_and_dot_components() {
        let dir = TempDir::new().unwrap();
        for basename in ["..", ".", "", "foo/bar", "foo\\bar"] {
            let err = discover(dir.path(), basename).unwrap_err();
            assert_invalid_basename(&err, basename);
        }
    }

    #[test]
    fn rejects_windows_drive_and_unc_basenames() {
        let dir = TempDir::new().unwrap();
        for basename in ["C:policy", "C:\\policy", "\\\\server\\share\\policy"] {
            let err = discover(dir.path(), basename).unwrap_err();
            assert_invalid_basename(&err, basename);
        }
    }
}
