//! Architecture definition resolution over the unified config
//! (UCFG-007, ADR-120 pt 5).
//!
//! Resolution order:
//!
//! 1. The main config's `architecture` section — inline, or delegated
//!    via `architecture.source` through the hardened
//!    [`anvil_config::resolve_section`] pipeline (UCFG-006).
//! 2. Legacy fallback: a standalone `.anvil/architecture.yaml` parsed
//!    by `anvil-architecture`'s own reader — unmigrated repos behave
//!    exactly as before (`anvil migrate architecture` writes the
//!    explicit `source` line to move a repo onto the section form).
//!
//! `gate.rs`, `watch.rs`, and the architecture commands all bind to
//! this seam — it is the single resolution point for consumers.

use std::path::{Path, PathBuf};

use anvil_architecture::ArchitectureDefinition;
use anvil_config::SectionProvenance;
use anvil_kernel::policy::config::{ArchitectureConfig, LayerDef};
use anyhow::{Context, Result};

/// Where a resolved architecture definition came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArchitectureOrigin {
    /// The main config's `architecture` section (inline or delegated).
    Section(SectionProvenance),
    /// The standalone legacy `.anvil/architecture.yaml`.
    LegacyFile(PathBuf),
}

/// Resolve the project's architecture definition, if any.
///
/// `Ok(None)` when neither an `architecture` section nor the legacy
/// file exists — the project has not opted into architecture
/// governance.
pub(crate) fn resolve_architecture(
    root: &Path,
) -> Result<Option<(ArchitectureDefinition, ArchitectureOrigin)>> {
    let project = crate::commands::config::load_project_config(root)?;
    let resolved =
        anvil_config::resolve_section(&project.value, "architecture", root, &project.writable_path)
            .map_err(crate::output::invalid_config)?;

    if let Some(section) = resolved {
        let definition: ArchitectureDefinition =
            serde_json::from_value(section.value).map_err(|err| {
                crate::output::invalid_config(format!(
                    "invalid [architecture] section: does not match the architecture schema: {err}"
                ))
            })?;
        // Same normalisation as the legacy file parser, so a resolved
        // definition is identical regardless of origin.
        let definition = anvil_architecture::yaml_parser::apply_defaults(definition);
        return Ok(Some((
            definition,
            ArchitectureOrigin::Section(section.provenance),
        )));
    }

    // Legacy standalone file — pre-section repos keep working unchanged.
    if anvil_architecture::yaml_parser::architecture_yaml_exists(root) {
        let path = anvil_architecture::yaml_parser::get_architecture_yaml_path(root);
        let definition = anvil_architecture::yaml_parser::parse_architecture_definition_file(&path)
            .with_context(|| format!("parsing {}", path.display()))?;
        return Ok(Some((definition, ArchitectureOrigin::LegacyFile(path))));
    }

    Ok(None)
}

/// Path `anvil watch` should hand the kernel as its architecture
/// source so enforcement and reload cover every topology (UCFG-013):
///
/// - inline section → the main config file
/// - delegated `source` → the delegated target
/// - no section → standalone `.anvil/architecture.yaml` when present
///   (kernel-schema or definition-schema; the kernel maps both)
pub(crate) fn watch_architecture_source_path(root: &Path) -> Result<Option<PathBuf>> {
    let project = crate::commands::config::load_project_config(root)?;
    if let Some(section) =
        anvil_config::resolve_section(&project.value, "architecture", root, &project.writable_path)
            .map_err(crate::output::invalid_config)?
    {
        return Ok(Some(match section.provenance {
            SectionProvenance::Inline => project.writable_path,
            SectionProvenance::Delegated { path, .. } => path,
        }));
    }
    let standalone = anvil_architecture::yaml_parser::get_architecture_yaml_path(root);
    Ok(standalone.exists().then_some(standalone))
}

/// Map a resolved definition into the kernel's watch-time schema
/// (UCFG-013). `patterns` → `paths`, `depends_on` → `allowed_imports`.
pub(crate) fn architecture_config_from_definition(
    definition: &ArchitectureDefinition,
) -> ArchitectureConfig {
    let mut layers: Vec<LayerDef> = definition
        .layers
        .iter()
        .map(|(name, layer)| LayerDef {
            name: name.clone(),
            paths: layer.patterns.clone(),
            allowed_imports: layer.depends_on.clone(),
        })
        .collect();
    layers.sort_by(|left, right| left.name.cmp(&right.name));
    ArchitectureConfig { layers }
}

/// Map a definition only after preflight succeeds. Watch must not load
/// or reload a policy that `anvil architecture validate` would reject.
pub(crate) fn mapped_architecture_from_definition(
    definition: &ArchitectureDefinition,
) -> Result<ArchitectureConfig> {
    let diagnostics = anvil_architecture::diagnose_definition(definition);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        anyhow::bail!(
            "Architecture config preflight failed:\n{}",
            errors
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    Ok(architecture_config_from_definition(definition))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCH_YAML: &str = "schema_version: \"0.1.0\"\nlayers:\n  core:\n    patterns:\n      - \"src/core/**\"\n    depends_on: []\n";

    fn arch_inline_config() -> &'static str {
        "architecture:\n  schema_version: \"0.1.0\"\n  layers:\n    core:\n      patterns:\n        - \"src/core/**\"\n      depends_on: []\n"
    }

    #[test]
    fn none_when_no_architecture_anywhere() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(resolve_architecture(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn inline_section_resolves() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), arch_inline_config()).unwrap();
        let (definition, origin) = resolve_architecture(tmp.path()).unwrap().unwrap();
        assert!(definition.layers.contains_key("core"));
        assert_eq!(
            origin,
            ArchitectureOrigin::Section(SectionProvenance::Inline)
        );
    }

    /// UCFG-007 resolved-equality contract: inline section, delegated
    /// source, and the legacy standalone file all resolve to the same
    /// definition.
    #[test]
    fn inline_delegated_and_legacy_resolve_equal() {
        // Inline.
        let inline_repo = tempfile::TempDir::new().unwrap();
        std::fs::write(inline_repo.path().join(".anvil.yaml"), arch_inline_config()).unwrap();
        let (from_inline, _) = resolve_architecture(inline_repo.path()).unwrap().unwrap();

        // Delegated to the (former) legacy path.
        let delegated_repo = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(delegated_repo.path().join(".anvil")).unwrap();
        std::fs::write(
            delegated_repo.path().join(".anvil/architecture.yaml"),
            ARCH_YAML,
        )
        .unwrap();
        std::fs::write(
            delegated_repo.path().join(".anvil.yaml"),
            "architecture:\n  source: \".anvil/architecture.yaml\"\n",
        )
        .unwrap();
        let (from_delegated, origin) = resolve_architecture(delegated_repo.path())
            .unwrap()
            .unwrap();
        assert!(matches!(
            origin,
            ArchitectureOrigin::Section(SectionProvenance::Delegated { .. })
        ));

        // Legacy standalone (no main-config section at all).
        let legacy_repo = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(legacy_repo.path().join(".anvil")).unwrap();
        std::fs::write(
            legacy_repo.path().join(".anvil/architecture.yaml"),
            ARCH_YAML,
        )
        .unwrap();
        let (from_legacy, origin) = resolve_architecture(legacy_repo.path()).unwrap().unwrap();
        assert!(matches!(origin, ArchitectureOrigin::LegacyFile(_)));

        // ArchitectureDefinition has no PartialEq; compare canonical
        // serialisations (same discipline as the config digests).
        let inline_v = serde_json::to_value(&from_inline).unwrap();
        let delegated_v = serde_json::to_value(&from_delegated).unwrap();
        let legacy_v = serde_json::to_value(&from_legacy).unwrap();
        assert_eq!(inline_v, delegated_v);
        assert_eq!(delegated_v, legacy_v);
    }

    #[test]
    fn section_wins_over_legacy_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        // Legacy file has a "legacy" layer; the section has "core".
        std::fs::write(
            tmp.path().join(".anvil/architecture.yaml"),
            "schema_version: \"0.1.0\"\nlayers:\n  legacy:\n    patterns: [\"old/**\"]\n    depends_on: []\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), arch_inline_config()).unwrap();
        let (definition, origin) = resolve_architecture(tmp.path()).unwrap().unwrap();
        assert!(definition.layers.contains_key("core"));
        assert!(!definition.layers.contains_key("legacy"));
        assert!(matches!(origin, ArchitectureOrigin::Section(_)));
    }

    #[test]
    fn schema_mismatch_in_section_errors_clearly() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "architecture:\n  layers: \"not-a-table\"\n",
        )
        .unwrap();
        let err = resolve_architecture(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("architecture"), "got: {err:#}");
    }

    #[test]
    fn delegation_errors_propagate() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "architecture:\n  source: \"../outside.yaml\"\n",
        )
        .unwrap();
        let err = resolve_architecture(tmp.path()).unwrap_err();
        // The context is the top-level message; the delegation detail
        // lives in the chain (rendered by consumers via `{:#}`).
        assert!(format!("{err:#}").contains("traversal"), "got: {err:#}");
    }

    #[test]
    fn watch_path_inline_is_the_main_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), arch_inline_config()).unwrap();
        let path = watch_architecture_source_path(tmp.path()).unwrap().unwrap();
        assert_eq!(path, tmp.path().join(".anvil.yaml"));
    }

    #[test]
    fn watch_path_delegated_is_the_target() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(tmp.path().join(".anvil/architecture.yaml"), ARCH_YAML).unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "architecture:\n  source: \".anvil/architecture.yaml\"\n",
        )
        .unwrap();
        let path = watch_architecture_source_path(tmp.path()).unwrap().unwrap();
        assert_eq!(
            path,
            tmp.path()
                .join(".anvil/architecture.yaml")
                .canonicalize()
                .unwrap()
        );
    }

    #[test]
    fn watch_path_legacy_standalone_is_the_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(tmp.path().join(".anvil/architecture.yaml"), ARCH_YAML).unwrap();
        let path = watch_architecture_source_path(tmp.path()).unwrap().unwrap();
        assert_eq!(path, tmp.path().join(".anvil/architecture.yaml"));
    }

    #[test]
    fn watch_path_none_when_no_architecture() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(
            watch_architecture_source_path(tmp.path())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn mapped_architecture_rejects_unknown_depends_on() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "architecture:\n  schema_version: \"0.1.0\"\n  layers:\n    app:\n      patterns: [\"src/app/**\"]\n      depends_on: [missing]\n",
        )
        .unwrap();
        let (definition, _) = resolve_architecture(tmp.path()).unwrap().unwrap();
        let err = mapped_architecture_from_definition(&definition).unwrap_err();
        assert!(format!("{err:#}").contains("unknown layer"), "got: {err:#}");
    }

    #[test]
    fn definition_map_matches_double_star_patterns() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), arch_inline_config()).unwrap();
        let (definition, _) = resolve_architecture(tmp.path()).unwrap().unwrap();
        let config = architecture_config_from_definition(&definition);
        assert_eq!(config.layers.len(), 1);
        assert_eq!(config.layers[0].name, "core");
        assert_eq!(
            config.layer_for_file("src/core/user.ts").unwrap().name,
            "core"
        );
    }

    #[test]
    fn watch_path_kernel_schema_standalone_is_the_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(
            tmp.path().join(".anvil/architecture.yaml"),
            "layers:\n  - name: api\n    paths: [\"src/api/*\"]\n",
        )
        .unwrap();
        let path = watch_architecture_source_path(tmp.path()).unwrap().unwrap();
        assert_eq!(path, tmp.path().join(".anvil/architecture.yaml"));
    }
}
