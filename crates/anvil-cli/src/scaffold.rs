//! Shared project-scaffold catalogue and additive reconciliation model.
//!
//! Command presentation belongs to `init` and `start`; this module owns the
//! component vocabulary and deterministic dependency/profile resolution both
//! commands consume.

#![allow(
    dead_code,
    reason = "PSCAF-001 exposes catalogue and selection seams consumed by the following slices"
)]

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ComponentId {
    Foundation,
    AcceptancePolicy,
    Architecture,
    Checks,
    Planning,
}

impl ComponentId {
    const ALL: [Self; 5] = [
        Self::Foundation,
        Self::AcceptancePolicy,
        Self::Architecture,
        Self::Checks,
        Self::Planning,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::AcceptancePolicy => "acceptance-policy",
            Self::Architecture => "architecture",
            Self::Checks => "checks",
            Self::Planning => "planning",
        }
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetKind {
    ConfigSetting,
    File,
    IdentityField,
    IgnoreLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct OwnedTarget {
    pub kind: TargetKind,
    pub target: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent catalogue facts are rendered directly in generated documentation"
)]
pub(crate) struct ComponentDefinition {
    pub id: ComponentId,
    pub purpose: &'static str,
    pub targets: &'static [OwnedTarget],
    pub mandatory: bool,
    pub recommended: bool,
    pub requires_suitability: bool,
    pub dependencies: &'static [ComponentId],
    pub inactive_consequence: &'static str,
    pub later_command: &'static str,
    pub replaceable: bool,
}

const FOUNDATION_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.schema_version",
    },
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.format",
    },
    OwnedTarget {
        kind: TargetKind::File,
        target: "anvil/project-id",
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "project_uuid",
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "created_at",
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "created_by_version",
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "scaffold_version",
    },
    OwnedTarget {
        kind: TargetKind::IgnoreLine,
        target: ".anvil/",
    },
];
const POLICY_TARGETS: &[OwnedTarget] = &[OwnedTarget {
    kind: TargetKind::File,
    target: "anvil/policy.yml",
}];
const ARCHITECTURE_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.architecture.source",
    },
    OwnedTarget {
        kind: TargetKind::File,
        target: "anvil/architecture.yaml",
    },
];
const CHECK_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "protection.checks",
    },
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "protection.enforcement.mode",
    },
];
const PLANNING_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.planning.dir",
    },
    OwnedTarget {
        kind: TargetKind::File,
        target: "plans/index.aps.md",
    },
];
const FOUNDATION_DEPENDENCY: &[ComponentId] = &[ComponentId::Foundation];

const CATALOGUE: &[ComponentDefinition] = &[
    ComponentDefinition {
        id: ComponentId::Foundation,
        purpose: "Give the project a stable anvil identity and scaffold version",
        targets: FOUNDATION_TARGETS,
        mandatory: true,
        recommended: true,
        requires_suitability: false,
        dependencies: &[],
        inactive_consequence: "No optional project protection can be activated safely",
        later_command: "anvil init --profile core",
        replaceable: false,
    },
    ComponentDefinition {
        id: ComponentId::AcceptancePolicy,
        purpose: "Make L4 acceptance evaluation reachable",
        targets: POLICY_TARGETS,
        mandatory: false,
        recommended: true,
        requires_suitability: false,
        dependencies: FOUNDATION_DEPENDENCY,
        inactive_consequence: "L4 deliberately passes without evaluating a change",
        later_command: "anvil init --include acceptance-policy",
        replaceable: false,
    },
    ComponentDefinition {
        id: ComponentId::Architecture,
        purpose: "Declare repository boundaries for architecture validation",
        targets: ARCHITECTURE_TARGETS,
        mandatory: false,
        recommended: true,
        requires_suitability: true,
        dependencies: FOUNDATION_DEPENDENCY,
        inactive_consequence: "Architecture-boundary checks remain unavailable",
        later_command: "anvil init --include architecture",
        replaceable: true,
    },
    ComponentDefinition {
        id: ComponentId::Checks,
        purpose: "Declare the desired check and enforcement posture",
        targets: CHECK_TARGETS,
        mandatory: false,
        recommended: true,
        requires_suitability: false,
        dependencies: FOUNDATION_DEPENDENCY,
        inactive_consequence: "Project checks keep their existing or built-in defaults",
        later_command: "anvil init --include checks",
        replaceable: false,
    },
    ComponentDefinition {
        id: ComponentId::Planning,
        purpose: "Configure supported planning integration",
        targets: PLANNING_TARGETS,
        mandatory: false,
        recommended: true,
        requires_suitability: false,
        dependencies: FOUNDATION_DEPENDENCY,
        inactive_consequence: "APS planning support is not scaffolded",
        later_command: "anvil init --include planning",
        replaceable: false,
    },
];

pub(crate) const fn catalogue() -> &'static [ComponentDefinition] {
    CATALOGUE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Profile {
    Core,
    Recommended,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectionRequest {
    profile: Profile,
    include: BTreeSet<ComponentId>,
    skip: BTreeSet<ComponentId>,
    suitable: BTreeSet<ComponentId>,
}

impl SelectionRequest {
    pub(crate) fn new(profile: Profile) -> Self {
        Self {
            profile,
            include: BTreeSet::new(),
            skip: BTreeSet::new(),
            suitable: BTreeSet::new(),
        }
    }

    pub(crate) fn include(mut self, component: ComponentId) -> Self {
        self.include.insert(component);
        self
    }

    pub(crate) fn skip(mut self, component: ComponentId) -> Self {
        self.skip.insert(component);
        self
    }

    pub(crate) fn suitable(mut self, component: ComponentId) -> Self {
        self.suitable.insert(component);
        self
    }

    pub(crate) fn resolve(self) -> Result<Selection, SelectionError> {
        if self.skip.contains(&ComponentId::Foundation) {
            return Err(SelectionError::FoundationCannotBeSkipped);
        }

        let mut selected = BTreeSet::from([ComponentId::Foundation]);
        for definition in catalogue() {
            let profile_selects = match self.profile {
                Profile::Core => definition.mandatory,
                Profile::Recommended => {
                    definition.mandatory
                        || (definition.recommended
                            && (!definition.requires_suitability
                                || self.suitable.contains(&definition.id)))
                }
                Profile::All => true,
            };
            if profile_selects || self.include.contains(&definition.id) {
                selected.insert(definition.id);
            }
        }
        for skipped in &self.skip {
            selected.remove(skipped);
        }

        let selected_definitions = catalogue()
            .iter()
            .filter(|entry| selected.contains(&entry.id))
            .copied()
            .collect::<Vec<_>>();
        for definition in selected_definitions {
            for dependency in definition.dependencies {
                if self.skip.contains(dependency) {
                    return Err(SelectionError::SkippedDependency {
                        component: definition.id,
                        dependency: *dependency,
                    });
                }
                selected.insert(*dependency);
            }
        }

        let selected = ComponentId::ALL
            .into_iter()
            .filter(|component| selected.contains(component))
            .collect::<Vec<_>>();
        let outcomes = ComponentId::ALL
            .into_iter()
            .filter(|component| !selected.contains(component))
            .map(|component| (component, MutationOutcome::Skipped))
            .collect();
        Ok(Selection { selected, outcomes })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
    pub selected: Vec<ComponentId>,
    outcomes: BTreeMap<ComponentId, MutationOutcome>,
}

impl Selection {
    pub(crate) fn outcome_for(&self, component: ComponentId) -> Option<MutationOutcome> {
        self.outcomes.get(&component).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum SelectionError {
    #[error("foundation is mandatory and cannot be skipped")]
    FoundationCannotBeSkipped,
    #[error("{component} requires skipped component {dependency}")]
    SkippedDependency {
        component: ComponentId,
        dependency: ComponentId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MutationOutcome {
    Created,
    Configured,
    Preserved,
    Skipped,
    NeedsInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComponentHealth {
    Valid,
    Invalid,
    Conflict,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ComponentResult {
    pub component: ComponentId,
    pub outcome: MutationOutcome,
    pub health: ComponentHealth,
    pub diagnostic: Option<String>,
    pub patch: Option<String>,
}

impl ComponentResult {
    pub(crate) fn satisfies_dependency(&self) -> bool {
        self.health == ComponentHealth::Valid
    }
}

/// Reconcile only the acceptance-policy component. Existing variants are
/// always operator-owned: they are inspected for health but never rewritten.
pub(crate) fn reconcile_acceptance_policy(
    root: &std::path::Path,
) -> anyhow::Result<ComponentResult> {
    let variants = crate::policy_load::policy_variants(root)?;
    if !variants.is_empty() {
        return Ok(inspect_acceptance_policy(root, &variants));
    }

    let path = root.join("anvil/policy.yml");
    match crate::util::write_new_nofollow(
        &path,
        crate::policy_load::DEFAULT_ACCEPTANCE_POLICY_YML.as_bytes(),
    ) {
        Ok(()) => {
            let variants = crate::policy_load::policy_variants(root)?;
            if variants.len() == 1 && variants[0] == path {
                return Ok(ComponentResult {
                    component: ComponentId::AcceptancePolicy,
                    outcome: MutationOutcome::Created,
                    health: ComponentHealth::Valid,
                    diagnostic: None,
                    patch: None,
                });
            }

            // An external writer introduced another variant during our
            // exclusive create. Remove only the exact bytes we created.
            if std::fs::read(&path).ok().as_deref()
                == Some(crate::policy_load::DEFAULT_ACCEPTANCE_POLICY_YML.as_bytes())
            {
                crate::util::remove_file_nofollow(&path)?;
            }
            Ok(inspect_acceptance_policy(
                root,
                &crate::policy_load::policy_variants(root)?,
            ))
        }
        Err(write_error) => {
            let raced_variants = crate::policy_load::policy_variants(root)?;
            if raced_variants.is_empty() {
                return Err(write_error);
            }
            Ok(inspect_acceptance_policy(root, &raced_variants))
        }
    }
}

fn inspect_acceptance_policy(
    root: &std::path::Path,
    variants: &[std::path::PathBuf],
) -> ComponentResult {
    if variants.len() > 1 {
        return ComponentResult {
            component: ComponentId::AcceptancePolicy,
            outcome: MutationOutcome::Preserved,
            health: ComponentHealth::Conflict,
            diagnostic: Some(format!(
                "multiple acceptance-policy variants exist: {}",
                variants
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            patch: None,
        };
    }

    match crate::policy_load::load_policy(root) {
        Ok(Some(_)) => ComponentResult {
            component: ComponentId::AcceptancePolicy,
            outcome: MutationOutcome::Preserved,
            health: ComponentHealth::Valid,
            diagnostic: None,
            patch: None,
        },
        Ok(None) => ComponentResult {
            component: ComponentId::AcceptancePolicy,
            outcome: MutationOutcome::NeedsInput,
            health: ComponentHealth::Unknown,
            diagnostic: Some("acceptance-policy path changed while it was inspected".to_owned()),
            patch: Some(crate::policy_load::DEFAULT_ACCEPTANCE_POLICY_YML.to_owned()),
        },
        Err(error) => ComponentResult {
            component: ComponentId::AcceptancePolicy,
            outcome: MutationOutcome::Preserved,
            health: ComponentHealth::Invalid,
            diagnostic: Some(error.to_string()),
            patch: None,
        },
    }
}

const FOUNDATION_CONFIG_YAML: &str = "schema_version: \"1.0.0\"\nformat: yaml\n";
const IGNORE_LINES: [&str; 3] = [
    ".anvil/",
    "anvil/exceptions/.lock",
    "anvil/witness/.chain-initialised",
];

/// Reconcile the mandatory foundation without creating runtime state, plans,
/// hooks, scans, baselines, or integrations.
#[allow(
    clippy::too_many_lines,
    reason = "foundation preflight, ordered writes and exact rollback form one transaction boundary"
)]
pub(crate) fn reconcile_foundation(root: &std::path::Path) -> anyhow::Result<ComponentResult> {
    use anyhow::Context;

    let configs = existing_main_configs(root)?;
    if configs.len() > 1 {
        return Ok(foundation_needs_input(
            ComponentHealth::Conflict,
            format!(
                "multiple main-config variants exist: {}",
                configs
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FOUNDATION_CONFIG_YAML,
        ));
    }
    if let Some(config_path) = configs.first()
        && let Err(diagnostic) = validate_foundation_config(config_path)
    {
        return Ok(foundation_needs_input(
            ComponentHealth::Invalid,
            diagnostic,
            FOUNDATION_CONFIG_YAML,
        ));
    }

    let identity_path = root.join("anvil/project-id");
    match std::fs::symlink_metadata(&identity_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Ok(foundation_needs_input(
                ComponentHealth::Invalid,
                format!("{} is not a regular file", identity_path.display()),
                "scaffold_version: 1\n",
            ));
        }
        Ok(_) => {
            let raw = anvil_config::read_to_string_bounded(&identity_path)
                .with_context(|| format!("read {}", identity_path.display()))?;
            if let Err(error) = crate::activation::identity::ProjectIdentity::parse(&raw) {
                return Ok(foundation_needs_input(
                    ComponentHealth::Invalid,
                    error.to_string(),
                    "scaffold_version: 1\n",
                ));
            }
            if !raw.lines().any(|line| line.trim() == "scaffold_version: 1") {
                return Ok(foundation_needs_input(
                    ComponentHealth::Invalid,
                    "anvil/project-id has no scaffold_version: 1 metadata".to_owned(),
                    "scaffold_version: 1\n",
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("inspect anvil/project-id"),
    }

    let ignore_path = root.join(".gitignore");
    let initial_ignore = read_regular_optional(&ignore_path)?;
    let desired_ignore = append_missing_ignore_lines(initial_ignore.as_deref());
    if initial_ignore.is_some()
        && desired_ignore.is_some()
        && anvil_config::mutation_lock_path(root).is_err()
    {
        return Ok(foundation_needs_input(
            ComponentHealth::Invalid,
            "existing .gitignore cannot be augmented without a Git common-dir lock".to_owned(),
            desired_ignore.as_deref().unwrap_or_default(),
        ));
    }

    let mut created_paths = Vec::new();
    let mut created_any = false;
    if configs.is_empty() {
        let path = root.join(".anvil.yaml");
        crate::util::write_new_nofollow(&path, FOUNDATION_CONFIG_YAML.as_bytes())?;
        created_paths.push((path, FOUNDATION_CONFIG_YAML.as_bytes().to_vec()));
        created_any = true;
    }

    if !identity_path.exists() {
        let identity =
            crate::activation::identity::ProjectIdentity::new_fresh(env!("CARGO_PKG_VERSION"));
        let rendered = identity.render();
        if let Err(error) = crate::util::write_new_nofollow(&identity_path, rendered.as_bytes()) {
            rollback_exact_created(&created_paths);
            return Err(error);
        }
        created_paths.push((identity_path, rendered.into_bytes()));
        created_any = true;
    }

    let mut configured = false;
    if let Some(desired) = desired_ignore {
        if let Some(initial) = initial_ignore {
            let _lock = match crate::util::ConfigMutationLock::try_acquire(root) {
                Ok(lock) => lock,
                Err(error) => {
                    rollback_exact_created(&created_paths);
                    return Ok(foundation_needs_input(
                        ComponentHealth::Invalid,
                        error.to_string(),
                        &desired,
                    ));
                }
            };
            let current = read_regular_optional(&ignore_path)?.unwrap_or_default();
            if current != initial {
                rollback_exact_created(&created_paths);
                return Ok(foundation_needs_input(
                    ComponentHealth::Invalid,
                    ".gitignore changed while the foundation was being reconciled".to_owned(),
                    &append_missing_ignore_lines(Some(&current)).unwrap_or(current),
                ));
            }
            crate::util::atomic_write_nofollow(&ignore_path, desired.as_bytes())?;
            configured = true;
        } else {
            crate::util::write_new_nofollow(&ignore_path, desired.as_bytes())?;
            created_any = true;
        }
    }

    Ok(ComponentResult {
        component: ComponentId::Foundation,
        outcome: if created_any {
            MutationOutcome::Created
        } else if configured {
            MutationOutcome::Configured
        } else {
            MutationOutcome::Preserved
        },
        health: ComponentHealth::Valid,
        diagnostic: None,
        patch: None,
    })
}

fn existing_main_configs(root: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();
    for name in [
        ".anvil.yaml",
        ".anvil.yml",
        ".anvil.json",
        ".anvil.toml",
        ".anvilrc",
    ] {
        let path = root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => paths.push(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(paths)
}

fn validate_foundation_config(path: &std::path::Path) -> Result<(), String> {
    let value = if path.file_name().and_then(std::ffi::OsStr::to_str) == Some(".anvilrc") {
        let raw = anvil_config::read_to_string_bounded(path).map_err(|error| error.to_string())?;
        anvil_config::parse_str(&raw, anvil_config::ConfigFormat::Yaml, path)
            .map_err(|error| error.to_string())?
    } else {
        anvil_config::parse_file(path).map_err(|error| error.to_string())?
    };
    if value
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        != Some("1.0.0")
    {
        return Err(format!(
            "{} must preserve operator bytes; add schema_version: 1.0.0 manually",
            path.display()
        ));
    }
    if value
        .get("format")
        .and_then(serde_json::Value::as_str)
        .is_none()
    {
        return Err(format!(
            "{} must preserve operator bytes; add a supported format manually",
            path.display()
        ));
    }
    Ok(())
}

fn read_regular_optional(path: &std::path::Path) -> anyhow::Result<Option<String>> {
    use anyhow::Context;

    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            anyhow::bail!("{} is not a regular file", path.display())
        }
        Ok(_) => std::fs::read_to_string(path)
            .map(Some)
            .with_context(|| format!("read {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("inspect {}", path.display())),
    }
}

fn append_missing_ignore_lines(existing: Option<&str>) -> Option<String> {
    let existing = existing.unwrap_or_default();
    let present = existing.lines().map(str::trim).collect::<BTreeSet<_>>();
    let missing = IGNORE_LINES
        .into_iter()
        .filter(|line| !present.contains(line))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return None;
    }
    let mut desired = existing.to_owned();
    if !desired.is_empty() && !desired.ends_with('\n') {
        desired.push('\n');
    }
    for line in missing {
        desired.push_str(line);
        desired.push('\n');
    }
    Some(desired)
}

fn foundation_needs_input(
    health: ComponentHealth,
    diagnostic: String,
    patch: &str,
) -> ComponentResult {
    ComponentResult {
        component: ComponentId::Foundation,
        outcome: MutationOutcome::NeedsInput,
        health,
        diagnostic: Some(diagnostic),
        patch: Some(patch.to_owned()),
    }
}

fn rollback_exact_created(created: &[(std::path::PathBuf, Vec<u8>)]) {
    for (path, expected) in created.iter().rev() {
        if std::fs::read(path).ok().as_deref() == Some(expected.as_slice()) {
            let _ = crate::util::remove_file_nofollow(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_has_one_stable_definition_for_every_initial_component() {
        let catalogue = catalogue();
        assert_eq!(catalogue.len(), 5);
        assert_eq!(catalogue[0].id, ComponentId::Foundation);
        assert!(catalogue[0].mandatory);
        assert!(!catalogue[0].replaceable);
        assert!(
            catalogue
                .iter()
                .any(|entry| entry.id == ComponentId::AcceptancePolicy
                    && entry.later_command == "anvil init --include acceptance-policy"
                    && !entry.replaceable)
        );
    }

    #[test]
    fn every_config_target_resolves_through_the_settings_catalogue() {
        let settings = anvil_settings::first_release_catalogue().expect("settings catalogue");
        for target in catalogue()
            .iter()
            .flat_map(|component| component.targets)
            .filter(|target| target.kind == TargetKind::ConfigSetting)
        {
            assert!(
                settings.project_config_target(target.target).is_some(),
                "{} has no canonical project-config target",
                target.target
            );
        }
    }

    #[test]
    fn profiles_and_suitability_resolve_deterministically() {
        let core = SelectionRequest::new(Profile::Core).resolve().unwrap();
        assert_eq!(core.selected, vec![ComponentId::Foundation]);

        let recommended = SelectionRequest::new(Profile::Recommended)
            .suitable(ComponentId::Architecture)
            .resolve()
            .unwrap();
        assert_eq!(
            recommended.selected,
            vec![
                ComponentId::Foundation,
                ComponentId::AcceptancePolicy,
                ComponentId::Architecture,
                ComponentId::Checks,
                ComponentId::Planning,
            ]
        );

        let all = SelectionRequest::new(Profile::All).resolve().unwrap();
        assert_eq!(all.selected.len(), catalogue().len());
    }

    #[test]
    fn skip_wins_but_foundation_can_never_be_skipped() {
        let selection = SelectionRequest::new(Profile::Recommended)
            .include(ComponentId::Architecture)
            .skip(ComponentId::Architecture)
            .resolve()
            .unwrap();
        assert!(!selection.selected.contains(&ComponentId::Architecture));
        assert_eq!(
            selection.outcome_for(ComponentId::Architecture),
            Some(MutationOutcome::Skipped)
        );

        let error = SelectionRequest::new(Profile::Core)
            .skip(ComponentId::Foundation)
            .resolve()
            .unwrap_err();
        assert_eq!(error, SelectionError::FoundationCannotBeSkipped);
    }

    #[test]
    fn mutation_outcome_does_not_claim_component_health() {
        let result = ComponentResult {
            component: ComponentId::AcceptancePolicy,
            outcome: MutationOutcome::Preserved,
            health: ComponentHealth::Invalid,
            diagnostic: Some("existing policy does not parse".to_owned()),
            patch: None,
        };

        assert_eq!(result.outcome, MutationOutcome::Preserved);
        assert_eq!(result.health, ComponentHealth::Invalid);
        assert!(!result.satisfies_dependency());
    }

    #[test]
    fn acceptance_policy_is_created_once_and_existing_bytes_are_never_replaced() {
        let root = tempfile::tempdir().unwrap();
        let created = reconcile_acceptance_policy(root.path()).unwrap();
        assert_eq!(created.outcome, MutationOutcome::Created);
        assert_eq!(created.health, ComponentHealth::Valid);
        let path = root.path().join("anvil/policy.yml");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            crate::policy_load::DEFAULT_ACCEPTANCE_POLICY_YML
        );

        std::fs::write(&path, "operator-owned invalid bytes\n").unwrap();
        let preserved = reconcile_acceptance_policy(root.path()).unwrap();
        assert_eq!(preserved.outcome, MutationOutcome::Preserved);
        assert_eq!(preserved.health, ComponentHealth::Invalid);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "operator-owned invalid bytes\n"
        );
    }

    #[test]
    fn acceptance_policy_reports_conflicting_variants_without_writing() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("anvil")).unwrap();
        std::fs::write(root.path().join("anvil/policy.yaml"), "keep yaml\n").unwrap();
        std::fs::write(root.path().join("anvil/policy.toml"), "keep toml\n").unwrap();

        let result = reconcile_acceptance_policy(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::Preserved);
        assert_eq!(result.health, ComponentHealth::Conflict);
        assert!(!root.path().join("anvil/policy.yml").exists());
    }

    #[test]
    fn fresh_foundation_creates_only_config_identity_and_ignore_hints() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::Created);
        assert_eq!(result.health, ComponentHealth::Valid);
        assert_eq!(
            std::fs::read_to_string(root.path().join(".anvil.yaml")).unwrap(),
            "schema_version: \"1.0.0\"\nformat: yaml\n"
        );
        assert!(
            std::fs::read_to_string(root.path().join("anvil/project-id"))
                .unwrap()
                .contains("scaffold_version: 1\n")
        );
        assert!(!root.path().join(".anvil").exists());
        assert!(!root.path().join("plans").exists());
    }

    #[test]
    fn foundation_preserves_existing_config_bytes_and_reports_missing_keys() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let config = root.path().join(".anvil.yaml");
        let original = "enforcement:\n  mode: block\n";
        std::fs::write(&config, original).unwrap();

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::NeedsInput);
        assert_eq!(result.health, ComponentHealth::Invalid);
        assert_eq!(std::fs::read_to_string(config).unwrap(), original);
        assert!(result.patch.unwrap().contains("schema_version"));
    }
}
