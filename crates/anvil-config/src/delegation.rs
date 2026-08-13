//! Section source delegation for the unified project config
//! (UCFG-006, ADR-120 pt 5).
//!
//! A main-config section (e.g. `architecture`) is either **inline** —
//! any non-delegating value, handed through for the caller to
//! shape-check — or **delegated** via a table whose single `source`
//! key names a workspace-relative file in any supported format:
//!
//! ```yaml
//! architecture:
//!   source: ".anvil/architecture.yaml"
//! ```
//!
//! The contract (ADR-120 pt 5, all rules pinned by tests):
//!
//! - **Exclusive**: `source` must be the section's only key — inline
//!   keys alongside it are an error, never merged.
//! - **One level deep**: a delegated file cannot itself delegate.
//! - **Path-safe**: workspace-relative only. Rejected: `..` traversal,
//!   absolute paths, Windows drive/UNC forms, symlink targets that
//!   escape the workspace root after canonicalisation, and
//!   self-reference back to the main config file.
//! - **Hardened parse**: delegated targets are read via
//!   [`crate::read_to_string_bounded`] and parsed via [`crate::parse_str`],
//!   inheriting the size cap, YAML alias rejection, and depth cap
//!   (ADR-046) — the legacy `anvil-architecture` YAML parser is never
//!   handed a delegated path.

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::{ConfigFormat, ParseError};

/// A resolved section value plus where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSection {
    pub value: Value,
    pub provenance: SectionProvenance,
}

/// Whether a section was inline or loaded from a delegated file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionProvenance {
    Inline,
    Delegated { path: PathBuf, format: ConfigFormat },
}

/// Delegation contract violations. Every message names the section and
/// is actionable on its own.
#[derive(Debug, thiserror::Error)]
pub enum DelegationError {
    #[error(
        "[{section}] has both 'source' and inline keys — delegation is exclusive; \
         keep either the inline table or the source line, not both"
    )]
    BothInlineAndSource { section: String },

    #[error("[{section}] 'source' must be a string path, found {found}")]
    SourceNotAString {
        section: String,
        found: &'static str,
    },

    #[error(
        "[{section}] source {path:?} is not workspace-relative — absolute paths, \
         drive letters, and UNC prefixes are rejected"
    )]
    AbsoluteSource { section: String, path: String },

    #[error("[{section}] source {path:?} contains '..' — path traversal is rejected")]
    TraversalSource { section: String, path: String },

    #[error(
        "[{section}] source {path:?} has no recognised config extension \
         (.yaml / .yml / .json / .toml)"
    )]
    UnknownFormat { section: String, path: String },

    #[error("[{section}] source file {path:?} does not exist under the workspace root")]
    MissingTarget { section: String, path: String },

    #[error(
        "[{section}] source {path:?} escapes the workspace root after symlink \
         resolution — delegated files must live inside the workspace"
    )]
    EscapesWorkspace { section: String, path: String },

    #[error(
        "[{section}] source {path:?} resolves to the main config file itself — \
         a section cannot delegate to the file that declares it"
    )]
    SelfReference { section: String, path: String },

    #[error(
        "[{section}] delegated file {path:?} itself contains a 'source' key — \
         delegation is one level deep"
    )]
    NestedDelegation { section: String, path: String },

    #[error("[{section}] delegated file {path:?} must contain a table at the top level")]
    DelegatedNotATable { section: String, path: String },

    #[error(
        "[{section}] source {path:?} is not a regular file — directories, FIFOs, \
         and devices are rejected (a non-regular target could hang the read)"
    )]
    NotARegularFile { section: String, path: String },

    #[error("[{section}] reading source {path:?}: {detail}")]
    Io {
        section: String,
        path: String,
        detail: String,
    },

    #[error("[{section}] parsing source {path:?}: {source}")]
    Parse {
        section: String,
        path: String,
        #[source]
        source: Box<ParseError>,
    },
}

/// Textual pre-checks that must reject a source string before any
/// filesystem access: absolute unix paths, Windows drive letters
/// (`C:`), UNC/verbatim prefixes (`\\`), and backslash-rooted paths.
fn is_absolute_like(raw: &str) -> bool {
    if Path::new(raw).is_absolute() {
        return true;
    }
    if raw.starts_with('/') || raw.starts_with('\\') {
        return true;
    }
    let bytes = raw.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn has_traversal(raw: &str) -> bool {
    Path::new(raw)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
}

/// The path-safety pipeline for a delegated `source` string: textual
/// pre-checks (absolute/UNC/traversal), format recognition, then
/// canonicalisation with workspace containment and self-reference
/// guards. Returns the canonical target path and its format.
fn safe_source_path(
    section: &str,
    rel: &str,
    workspace_root: &Path,
    main_config_path: &Path,
) -> Result<(PathBuf, ConfigFormat), DelegationError> {
    if is_absolute_like(rel) {
        return Err(DelegationError::AbsoluteSource {
            section: section.to_string(),
            path: rel.to_string(),
        });
    }
    if has_traversal(rel) {
        return Err(DelegationError::TraversalSource {
            section: section.to_string(),
            path: rel.to_string(),
        });
    }

    let Some(format) = ConfigFormat::from_path(Path::new(rel)) else {
        return Err(DelegationError::UnknownFormat {
            section: section.to_string(),
            path: rel.to_string(),
        });
    };

    let canonical = workspace_root
        .join(rel)
        .canonicalize()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => DelegationError::MissingTarget {
                section: section.to_string(),
                path: rel.to_string(),
            },
            _ => DelegationError::Io {
                section: section.to_string(),
                path: rel.to_string(),
                detail: e.to_string(),
            },
        })?;
    let canonical_root = workspace_root
        .canonicalize()
        .map_err(|e| DelegationError::Io {
            section: section.to_string(),
            path: rel.to_string(),
            detail: format!("canonicalising workspace root: {e}"),
        })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(DelegationError::EscapesWorkspace {
            section: section.to_string(),
            path: rel.to_string(),
        });
    }
    if let Ok(main_canonical) = main_config_path.canonicalize()
        && canonical == main_canonical
    {
        return Err(DelegationError::SelfReference {
            section: section.to_string(),
            path: rel.to_string(),
        });
    }
    // Defence-in-depth (UCFG-014): the bounded reader now refuses
    // non-regular files on the held descriptor. Keep the pre-open
    // stat so a FIFO is still rejected as NotARegularFile before
    // open, matching the existing error shape.
    let file_type = canonical
        .metadata()
        .map_err(|e| DelegationError::Io {
            section: section.to_string(),
            path: rel.to_string(),
            detail: e.to_string(),
        })?
        .file_type();
    if !file_type.is_file() {
        return Err(DelegationError::NotARegularFile {
            section: section.to_string(),
            path: rel.to_string(),
        });
    }

    Ok((canonical, format))
}

/// Resolve the named section of a parsed main-config value.
///
/// - Absent or null section → `Ok(None)` (presence requirements are the
///   caller's contract, not the resolver's).
/// - Inline table → returned as-is with `Inline` provenance.
/// - `source`-only table → the delegated file is safety-checked, read
///   bounded, parsed hardened, and returned with `Delegated` provenance.
///
/// `workspace_root` anchors relative sources; `main_config_path` is the
/// file `config` was parsed from (self-reference guard). Both may be
/// un-canonicalised — the resolver canonicalises internally.
pub fn resolve_section(
    config: &Value,
    section: &str,
    workspace_root: &Path,
    main_config_path: &Path,
) -> Result<Option<ResolvedSection>, DelegationError> {
    let Some(raw) = config.get(section) else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }

    let Some(table) = raw.as_object() else {
        // Non-table sections are the caller's shape problem, not a
        // delegation concern — hand the value through inline.
        return Ok(Some(ResolvedSection {
            value: raw.clone(),
            provenance: SectionProvenance::Inline,
        }));
    };

    let Some(source) = table.get("source") else {
        return Ok(Some(ResolvedSection {
            value: raw.clone(),
            provenance: SectionProvenance::Inline,
        }));
    };

    if table.len() > 1 {
        return Err(DelegationError::BothInlineAndSource {
            section: section.to_string(),
        });
    }

    let Some(rel) = source.as_str() else {
        return Err(DelegationError::SourceNotAString {
            section: section.to_string(),
            found: match source {
                Value::Null => "null",
                Value::Bool(_) => "bool",
                Value::Number(_) => "number",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                Value::String(_) => unreachable!("guarded by as_str"),
            },
        });
    };

    let (canonical, format) = safe_source_path(section, rel, workspace_root, main_config_path)?;

    let contents = crate::read_to_string_bounded(&canonical).map_err(|e| match e {
        ParseError::NotARegularFile { .. } => DelegationError::NotARegularFile {
            section: section.to_string(),
            path: rel.to_string(),
        },
        other => DelegationError::Io {
            section: section.to_string(),
            path: rel.to_string(),
            detail: other.to_string(),
        },
    })?;
    let value =
        crate::parse_str(&contents, format, &canonical).map_err(|e| DelegationError::Parse {
            section: section.to_string(),
            path: rel.to_string(),
            source: Box::new(e),
        })?;

    let Some(delegated_table) = value.as_object() else {
        return Err(DelegationError::DelegatedNotATable {
            section: section.to_string(),
            path: rel.to_string(),
        });
    };
    if delegated_table.contains_key("source") {
        return Err(DelegationError::NestedDelegation {
            section: section.to_string(),
            path: rel.to_string(),
        });
    }

    Ok(Some(ResolvedSection {
        value,
        provenance: SectionProvenance::Delegated {
            path: canonical,
            format,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Fixture {
        tmp: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                tmp: tempfile::TempDir::new().unwrap(),
            }
        }
        fn root(&self) -> &Path {
            self.tmp.path()
        }
        fn write(&self, rel: &str, body: &str) -> PathBuf {
            let path = self.root().join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, body).unwrap();
            path
        }
        fn main_config(&self) -> PathBuf {
            self.root().join(".anvil.yaml")
        }
        fn resolve(&self, config: &Value) -> Result<Option<ResolvedSection>, DelegationError> {
            resolve_section(config, "architecture", self.root(), &self.main_config())
        }
    }

    #[test]
    fn absent_and_null_sections_resolve_to_none() {
        let f = Fixture::new();
        assert!(f.resolve(&json!({})).unwrap().is_none());
        assert!(f.resolve(&json!({"architecture": null})).unwrap().is_none());
    }

    #[test]
    fn inline_section_passes_through_with_inline_provenance() {
        let f = Fixture::new();
        let resolved = f
            .resolve(&json!({"architecture": {"layers": {"core": {}}}}))
            .unwrap()
            .unwrap();
        assert_eq!(resolved.provenance, SectionProvenance::Inline);
        assert_eq!(resolved.value["layers"]["core"], json!({}));
    }

    #[test]
    fn delegated_section_resolves_with_provenance() {
        let f = Fixture::new();
        f.write(".anvil/architecture.yaml", "layers:\n  core: {}\n");
        let resolved = f
            .resolve(&json!({"architecture": {"source": ".anvil/architecture.yaml"}}))
            .unwrap()
            .unwrap();
        assert_eq!(resolved.value["layers"]["core"], json!({}));
        match resolved.provenance {
            SectionProvenance::Delegated { format, .. } => {
                assert_eq!(format, ConfigFormat::Yaml);
            }
            other @ SectionProvenance::Inline => {
                panic!("expected Delegated, got {other:?}")
            }
        }
    }

    #[test]
    fn delegated_target_may_be_any_supported_format() {
        let f = Fixture::new();
        f.write("arch.toml", "[layers.core]\n");
        let resolved = f
            .resolve(&json!({"architecture": {"source": "arch.toml"}}))
            .unwrap()
            .unwrap();
        assert_eq!(resolved.value["layers"]["core"], json!({}));
    }

    #[test]
    fn inline_keys_beside_source_are_exclusive_error() {
        let f = Fixture::new();
        let err = f
            .resolve(&json!({"architecture": {"source": "arch.yaml", "layers": {}}}))
            .unwrap_err();
        assert!(
            err.to_string().contains("both 'source' and inline keys"),
            "{err}"
        );
    }

    #[test]
    fn non_string_source_errors() {
        let f = Fixture::new();
        let err = f
            .resolve(&json!({"architecture": {"source": 42}}))
            .unwrap_err();
        assert!(err.to_string().contains("must be a string"), "{err}");
    }

    #[test]
    fn missing_target_names_the_path() {
        let f = Fixture::new();
        let err = f
            .resolve(&json!({"architecture": {"source": "gone.yaml"}}))
            .unwrap_err();
        assert!(err.to_string().contains("gone.yaml"), "{err}");
        assert!(err.to_string().contains("does not exist"), "{err}");
    }

    /// Adversarial path corpus (UCFG-006 validation): traversal,
    /// absolute, drive, UNC, backslash-rooted — every one rejected
    /// before any filesystem access.
    #[test]
    fn adversarial_source_paths_are_rejected() {
        let f = Fixture::new();
        let corpus = [
            "../outside.yaml",
            "a/../../outside.yaml",
            "..",
            "/etc/passwd.yaml",
            "/tmp/x.toml",
            "C:\\evil.yaml",
            "c:/evil.yaml",
            "\\\\server\\share\\x.yaml",
            "\\evil.yaml",
        ];
        for path in corpus {
            let err = f
                .resolve(&json!({"architecture": {"source": path}}))
                .unwrap_err();
            assert!(
                matches!(
                    err,
                    DelegationError::TraversalSource { .. }
                        | DelegationError::AbsoluteSource { .. }
                        | DelegationError::UnknownFormat { .. }
                ),
                "{path} must be rejected pre-IO, got: {err}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escaping_workspace_is_rejected() {
        let f = Fixture::new();
        let outside = tempfile::TempDir::new().unwrap();
        let target = outside.path().join("outside.yaml");
        std::fs::write(&target, "layers: {}\n").unwrap();
        std::os::unix::fs::symlink(&target, f.root().join("linked.yaml")).unwrap();
        let err = f
            .resolve(&json!({"architecture": {"source": "linked.yaml"}}))
            .unwrap_err();
        assert!(
            matches!(err, DelegationError::EscapesWorkspace { .. }),
            "{err}"
        );
    }

    #[test]
    fn self_reference_is_rejected() {
        let f = Fixture::new();
        f.write(".anvil.yaml", "architecture:\n  source: \".anvil.yaml\"\n");
        let err = f
            .resolve(&json!({"architecture": {"source": ".anvil.yaml"}}))
            .unwrap_err();
        assert!(
            matches!(err, DelegationError::SelfReference { .. }),
            "{err}"
        );
    }

    #[test]
    fn nested_delegation_is_rejected() {
        let f = Fixture::new();
        f.write("arch.yaml", "source: \"deeper.yaml\"\n");
        let err = f
            .resolve(&json!({"architecture": {"source": "arch.yaml"}}))
            .unwrap_err();
        assert!(
            matches!(err, DelegationError::NestedDelegation { .. }),
            "{err}"
        );
    }

    #[test]
    fn delegated_scalar_top_level_is_rejected() {
        let f = Fixture::new();
        f.write("arch.yaml", "just-a-string\n");
        let err = f
            .resolve(&json!({"architecture": {"source": "arch.yaml"}}))
            .unwrap_err();
        assert!(
            matches!(err, DelegationError::DelegatedNotATable { .. }),
            "{err}"
        );
    }

    /// Delegated targets inherit the hardened parse (ADR-046):
    /// alias-bearing YAML is rejected identically to the main config.
    #[test]
    fn delegated_alias_bomb_is_rejected_by_hardened_parse() {
        let f = Fixture::new();
        f.write("arch.yaml", "base: &a [1, 2]\nlayers: *a\n");
        let err = f
            .resolve(&json!({"architecture": {"source": "arch.yaml"}}))
            .unwrap_err();
        assert!(
            matches!(err, DelegationError::Parse { .. }),
            "alias must be refused by the shared gate, got: {err}"
        );
    }

    /// Oversized delegated targets are refused by the bounded reader
    /// before allocation, like every other config read.
    #[test]
    fn oversized_delegated_target_is_refused() {
        let f = Fixture::new();
        let big = format!(
            "# {}\nlayers: {{}}\n",
            "x".repeat(usize::try_from(crate::MAX_CONFIG_FILE_BYTES).unwrap())
        );
        f.write("arch.yaml", &big);
        let err = f
            .resolve(&json!({"architecture": {"source": "arch.yaml"}}))
            .unwrap_err();
        assert!(matches!(err, DelegationError::Io { .. }), "{err}");
    }

    /// Deterministic property pass (UCFG-006 validation): a seeded
    /// generator composes hostile source strings from traversal,
    /// absolute, separator, and control-character fragments. The
    /// resolver must never panic and must never resolve outside the
    /// workspace — every outcome is an error or a contained path.
    #[test]
    fn property_pass_hostile_sources_never_panic_or_escape() {
        let f = Fixture::new();
        f.write("ok.yaml", "layers: {}\n");
        let fragments = [
            "..", ".", "", "/", "\\", "a", "ok", ".anvil", "yaml", "C:", "\u{0}", "..\\", "../",
            "./", "//", "file:", "~", "*", "ok.yaml",
        ];
        // xorshift64 — deterministic, no new deps, seeds recorded.
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..2000 {
            let parts = 1 + usize::try_from(next() % 5).unwrap();
            let mut raw = String::new();
            for i in 0..parts {
                if i > 0 {
                    raw.push(if next() % 2 == 0 { '/' } else { '\\' });
                }
                let pick = usize::try_from(next() % fragments.len() as u64).unwrap();
                raw.push_str(fragments[pick]);
            }
            let config = json!({"architecture": {"source": raw}});
            match f.resolve(&config) {
                Err(_) | Ok(None) => {}
                Ok(Some(resolved)) => {
                    if let SectionProvenance::Delegated { path, .. } = &resolved.provenance {
                        let root = f.root().canonicalize().unwrap();
                        assert!(
                            path.starts_with(&root),
                            "escaped workspace: {raw:?} -> {path:?}"
                        );
                    }
                }
            }
        }
    }

    /// A FIFO delegation target must be rejected by stat, never
    /// opened — opening a FIFO with no writer blocks indefinitely,
    /// which would hand repo content a way to hang the gate.
    #[cfg(unix)]
    #[test]
    fn fifo_source_is_rejected_without_blocking() {
        let f = Fixture::new();
        let fifo = f.root().join("pipe.yaml");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo available on unix");
        assert!(status.success());
        let started = std::time::Instant::now();
        let err = f
            .resolve(&json!({"architecture": {"source": "pipe.yaml"}}))
            .unwrap_err();
        assert!(
            matches!(err, DelegationError::NotARegularFile { .. }),
            "{err}"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "rejection must not block"
        );
    }

    /// Inline and delegated spellings of the same section resolve to
    /// the same value (UCFG-007's resolved-equality contract, pinned
    /// here at the resolver).
    #[test]
    fn inline_and_delegated_resolve_to_equal_values() {
        let f = Fixture::new();
        f.write(
            ".anvil/architecture.yaml",
            "schema_version: \"0.1.0\"\nlayers:\n  core:\n    patterns: [\"src/core/**\"]\n",
        );
        let inline = f
            .resolve(&json!({"architecture": {
                "schema_version": "0.1.0",
                "layers": {"core": {"patterns": ["src/core/**"]}},
            }}))
            .unwrap()
            .unwrap();
        let delegated = f
            .resolve(&json!({"architecture": {"source": ".anvil/architecture.yaml"}}))
            .unwrap()
            .unwrap();
        assert_eq!(inline.value, delegated.value);
    }
}
