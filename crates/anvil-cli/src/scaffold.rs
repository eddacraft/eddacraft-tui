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
    pub replaceable: bool,
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
}

const FOUNDATION_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.schema_version",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.format",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::File,
        target: "anvil/project-id",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "project_uuid",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "created_at",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "created_by_version",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::IdentityField,
        target: "scaffold_version",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::IgnoreLine,
        target: ".anvil/",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::IgnoreLine,
        target: "anvil/exceptions/.lock",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::IgnoreLine,
        target: "anvil/witness/.chain-initialised",
        replaceable: false,
    },
];
const POLICY_TARGETS: &[OwnedTarget] = &[OwnedTarget {
    kind: TargetKind::File,
    target: "anvil/policy.yml",
    replaceable: false,
}];
const ARCHITECTURE_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.architecture.source",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::File,
        target: "anvil/architecture.yaml",
        replaceable: true,
    },
];
const CHECK_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "protection.checks",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "protection.enforcement.mode",
        replaceable: false,
    },
];
const PLANNING_TARGETS: &[OwnedTarget] = &[
    OwnedTarget {
        kind: TargetKind::ConfigSetting,
        target: "project.planning.dir",
        replaceable: false,
    },
    OwnedTarget {
        kind: TargetKind::File,
        target: "plans/index.aps.md",
        replaceable: false,
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
            let created = crate::util::observe_regular_nofollow(&path)?;
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
            let _ = crate::util::remove_if_unchanged(&path, &created)?;
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

struct ExistingUpdate {
    path: std::path::PathBuf,
    before: crate::util::ObservedFile,
    desired: Vec<u8>,
}

type IdentityUpdatePlan = Result<Option<(crate::util::ObservedFile, Vec<u8>)>, String>;

enum AppliedMutation {
    Created {
        path: std::path::PathBuf,
        observed: crate::util::ObservedFile,
    },
    Published {
        path: std::path::PathBuf,
        before: crate::util::ObservedFile,
        published: crate::util::ObservedFile,
    },
}

/// Reconcile the mandatory foundation without creating runtime state, plans,
/// hooks, scans, baselines, or integrations.
#[allow(
    clippy::too_many_lines,
    reason = "foundation preflight, ordered writes and exact rollback form one transaction boundary"
)]
pub(crate) fn reconcile_foundation(root: &std::path::Path) -> anyhow::Result<ComponentResult> {
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
    let mut existing_updates = Vec::new();
    let config_create = if let Some(path) = configs.first() {
        match plan_foundation_config_update(path)? {
            Ok(Some(update)) => {
                existing_updates.push(update);
                None
            }
            Ok(None) => None,
            Err((diagnostic, patch)) => {
                return Ok(foundation_needs_input(
                    ComponentHealth::Invalid,
                    diagnostic,
                    &patch,
                ));
            }
        }
    } else {
        Some((
            root.join(".anvil.yaml"),
            FOUNDATION_CONFIG_YAML.as_bytes().to_vec(),
        ))
    };

    let identity_path = root.join("anvil/project-id");
    let identity_create = match crate::util::observe_regular_nofollow(&identity_path) {
        Ok(before) => match plan_identity_update(before)? {
            Ok(Some(update)) => {
                existing_updates.push(ExistingUpdate {
                    path: identity_path.clone(),
                    before: update.0,
                    desired: update.1,
                });
                None
            }
            Ok(None) => None,
            Err(diagnostic) => {
                return Ok(foundation_needs_input(
                    ComponentHealth::Invalid,
                    diagnostic,
                    "created_at: <RFC3339 time>\ncreated_by_version: <anvil version>\nscaffold_version: 1\n",
                ));
            }
        },
        Err(error) if is_not_found(&error) => {
            let rendered =
                crate::activation::identity::ProjectIdentity::new_fresh(env!("CARGO_PKG_VERSION"))
                    .render();
            Some((identity_path, rendered.into_bytes()))
        }
        Err(error) => {
            return Ok(foundation_needs_input(
                ComponentHealth::Invalid,
                error.to_string(),
                "scaffold_version: 1\n",
            ));
        }
    };

    let ignore_path = root.join(".gitignore");
    let ignore_create = match crate::util::observe_regular_nofollow(&ignore_path) {
        Ok(before) => {
            let raw = String::from_utf8(before.bytes.clone()).map_err(anyhow::Error::from)?;
            match append_missing_ignore_lines(Some(&raw)) {
                Some(desired) => {
                    existing_updates.push(ExistingUpdate {
                        path: ignore_path.clone(),
                        before,
                        desired: desired.into_bytes(),
                    });
                    None
                }
                None => None,
            }
        }
        Err(error) if is_not_found(&error) => Some((
            ignore_path,
            append_missing_ignore_lines(None)
                .unwrap_or_default()
                .into_bytes(),
        )),
        Err(error) => {
            return Ok(foundation_needs_input(
                ComponentHealth::Invalid,
                error.to_string(),
                &IGNORE_LINES.join("\n"),
            ));
        }
    };

    // Existing-file augmentation is automatic only when every linked
    // worktree can coordinate through the Git-common-dir lock.
    let _lock = if !existing_updates.is_empty() {
        match crate::util::ConfigMutationLock::try_acquire(root) {
            Ok(lock) => Some(lock),
            Err(error) => {
                let patch = existing_updates
                    .iter()
                    .map(|update| String::from_utf8_lossy(&update.desired))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Ok(foundation_needs_input(
                    ComponentHealth::Invalid,
                    error.to_string(),
                    &patch,
                ));
            }
        }
    } else if config_create.is_some() {
        match crate::util::lock_project_config_create(root) {
            Ok(lock) => lock,
            Err(error) => {
                return Ok(foundation_needs_input(
                    ComponentHealth::Invalid,
                    error.to_string(),
                    FOUNDATION_CONFIG_YAML,
                ));
            }
        }
    } else {
        None
    };

    // The initial variant scan is read-only. Repeat it while holding the Git
    // common-dir lock before creating the canonical config so a cooperating
    // format-specific writer cannot commit a second variant in the gap.
    if config_create.is_some() {
        let variants = existing_main_configs(root)?;
        if !variants.is_empty() {
            return Ok(foundation_needs_input(
                ComponentHealth::Conflict,
                format!(
                    "main config appeared while foundation was being planned: {}",
                    variants
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                FOUNDATION_CONFIG_YAML,
            ));
        }
    }

    let mut journal = Vec::new();
    let apply_result = (|| -> anyhow::Result<()> {
        if let Some((path, bytes)) = config_create {
            apply_create(&path, &bytes, &mut journal)?;
        }
        if let Some(config_path) = configs.first()
            && let Some(update) = existing_updates
                .iter()
                .find(|update| &update.path == config_path)
        {
            apply_existing(update, &mut journal)?;
        }
        foundation_boundary()?;
        if let Some((path, bytes)) = identity_create {
            apply_create(&path, &bytes, &mut journal)?;
        }
        if let Some(update) = existing_updates
            .iter()
            .find(|update| update.path == root.join("anvil/project-id"))
        {
            apply_existing(update, &mut journal)?;
        }
        foundation_boundary()?;
        if let Some((path, bytes)) = ignore_create {
            apply_create(&path, &bytes, &mut journal)?;
        }
        if let Some(update) = existing_updates
            .iter()
            .find(|update| update.path == root.join(".gitignore"))
        {
            apply_existing(update, &mut journal)?;
        }
        foundation_boundary()?;
        foundation_boundary()?;
        inspect_complete_foundation(root)
    })();

    if let Err(error) = apply_result {
        let rollback_errors = rollback_applied(&journal);
        let diagnostic = if rollback_errors.is_empty() {
            error.to_string()
        } else {
            format!(
                "{error}; rollback was incomplete: {}",
                rollback_errors.join("; ")
            )
        };
        return Ok(foundation_needs_input(
            ComponentHealth::Invalid,
            diagnostic,
            FOUNDATION_CONFIG_YAML,
        ));
    }

    let created_any = journal
        .iter()
        .any(|entry| matches!(entry, AppliedMutation::Created { .. }));
    let configured = journal
        .iter()
        .any(|entry| matches!(entry, AppliedMutation::Published { .. }));
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

fn plan_foundation_config_update(
    path: &std::path::Path,
) -> anyhow::Result<Result<Option<ExistingUpdate>, (String, String)>> {
    let before = match crate::util::observe_regular_nofollow(path) {
        Ok(observed) => observed,
        Err(error) => return Ok(Err((error.to_string(), FOUNDATION_CONFIG_YAML.to_owned()))),
    };
    let raw = match std::str::from_utf8(&before.bytes) {
        Ok(raw) => raw,
        Err(error) => return Ok(Err((error.to_string(), FOUNDATION_CONFIG_YAML.to_owned()))),
    };
    let format = config_format(path);
    let value = match anvil_config::parse_str(raw, format, path) {
        Ok(value) => value,
        Err(error) => return Ok(Err((error.to_string(), FOUNDATION_CONFIG_YAML.to_owned()))),
    };
    if let Some(schema) = value.get("schema_version")
        && schema.as_str() != Some("1.0.0")
    {
        return Ok(Err((
            format!("{} has unsupported schema_version", path.display()),
            FOUNDATION_CONFIG_YAML.to_owned(),
        )));
    }
    let expected_format = format_label_for_config(format);
    if let Some(configured) = value.get("format")
        && configured.as_str() != Some(expected_format)
    {
        return Ok(Err((
            format!(
                "{} format must be {expected_format} for this config path",
                path.display()
            ),
            FOUNDATION_CONFIG_YAML.to_owned(),
        )));
    }
    let catalogue = anvil_settings::first_release_catalogue()?;
    let additions = [
        anvil_settings::BootstrapSetting {
            key: "project.schema_version".to_owned(),
            value: serde_json::Value::String("1.0.0".to_owned()),
        },
        anvil_settings::BootstrapSetting {
            key: "project.format".to_owned(),
            value: serde_json::Value::String(expected_format.to_owned()),
        },
    ];
    match catalogue.plan_project_config_bootstrap(raw, format, &additions)? {
        anvil_settings::BootstrapMutation::Unchanged => Ok(Ok(None)),
        anvil_settings::BootstrapMutation::Updated(desired) => {
            validate_foundation_config_bytes(path, desired.as_bytes())
                .map_err(anyhow::Error::msg)?;
            Ok(Ok(Some(ExistingUpdate {
                path: path.to_path_buf(),
                before,
                desired: desired.into_bytes(),
            })))
        }
        anvil_settings::BootstrapMutation::NeedsInput { patch, reason } => Ok(Err((reason, patch))),
    }
}

fn config_format(path: &std::path::Path) -> anvil_config::ConfigFormat {
    if path.file_name().and_then(std::ffi::OsStr::to_str) == Some(".anvilrc") {
        anvil_config::ConfigFormat::Yaml
    } else {
        anvil_config::ConfigFormat::from_path(path).unwrap_or(anvil_config::ConfigFormat::Yaml)
    }
}

fn format_label_for_config(format: anvil_config::ConfigFormat) -> &'static str {
    match format {
        anvil_config::ConfigFormat::Yaml | anvil_config::ConfigFormat::Yml => "yaml",
        anvil_config::ConfigFormat::Json => "json",
        anvil_config::ConfigFormat::Toml => "toml",
    }
}

fn validate_foundation_config_bytes(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let raw = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let format = config_format(path);
    let value = anvil_config::parse_str(raw, format, path).map_err(|error| error.to_string())?;
    if value
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        != Some("1.0.0")
    {
        return Err(format!(
            "{} must contain schema_version 1.0.0",
            path.display()
        ));
    }
    let expected = format_label_for_config(format);
    if value.get("format").and_then(serde_json::Value::as_str) != Some(expected) {
        return Err(format!("{} must contain format {expected}", path.display()));
    }
    Ok(())
}

fn plan_identity_update(before: crate::util::ObservedFile) -> anyhow::Result<IdentityUpdatePlan> {
    let raw = match std::str::from_utf8(&before.bytes) {
        Ok(raw) => raw,
        Err(error) => return Ok(Err(error.to_string())),
    };
    let identity = match crate::activation::identity::ProjectIdentity::parse(raw) {
        Ok(identity) => identity,
        Err(error) => return Ok(Err(error.to_string())),
    };
    if let Some(created_at) = identity.created_at.as_deref()
        && chrono::DateTime::parse_from_rfc3339(created_at).is_err()
    {
        return Ok(Err(
            "anvil/project-id has invalid created_at metadata".to_owned()
        ));
    }
    if identity
        .created_by_version
        .as_deref()
        .is_some_and(str::is_empty)
    {
        return Ok(Err(
            "anvil/project-id has empty created_by_version metadata".to_owned(),
        ));
    }
    if identity
        .scaffold_version
        .is_some_and(|version| version != 1)
    {
        return Ok(Err(
            "anvil/project-id has unsupported scaffold_version metadata".to_owned(),
        ));
    }

    let mut desired = raw.to_owned();
    if !desired.is_empty() && !desired.ends_with('\n') {
        desired.push('\n');
    }
    if identity.created_at.is_none() {
        desired.push_str("created_at: ");
        desired.push_str(&chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        desired.push('\n');
    }
    if identity.created_by_version.is_none() {
        desired.push_str("created_by_version: ");
        desired.push_str(env!("CARGO_PKG_VERSION"));
        desired.push('\n');
    }
    if identity.scaffold_version.is_none() {
        desired.push_str("scaffold_version: 1\n");
    }
    if desired.as_bytes() == before.bytes {
        if let Err(error) = validate_complete_identity(&identity) {
            return Ok(Err(error));
        }
        return Ok(Ok(None));
    }
    let reparsed = crate::activation::identity::ProjectIdentity::parse(&desired)
        .map_err(|error| anyhow::anyhow!(error))?;
    if let Err(error) = validate_complete_identity(&reparsed) {
        return Ok(Err(error));
    }
    Ok(Ok(Some((before, desired.into_bytes()))))
}

fn validate_complete_identity(
    identity: &crate::activation::identity::ProjectIdentity,
) -> Result<(), String> {
    let created_at = identity
        .created_at
        .as_deref()
        .ok_or_else(|| "anvil/project-id has no created_at metadata".to_owned())?;
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| "anvil/project-id has invalid created_at metadata".to_owned())?;
    if identity
        .created_by_version
        .as_deref()
        .is_none_or(str::is_empty)
    {
        return Err("anvil/project-id has no created_by_version metadata".to_owned());
    }
    if identity.scaffold_version != Some(1) {
        return Err("anvil/project-id must contain scaffold_version 1".to_owned());
    }
    Ok(())
}

fn append_missing_ignore_lines(existing: Option<&str>) -> Option<String> {
    let existing = existing.unwrap_or_default();
    let present = existing.lines().collect::<BTreeSet<_>>();
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

fn apply_create(
    path: &std::path::Path,
    bytes: &[u8],
    journal: &mut Vec<AppliedMutation>,
) -> anyhow::Result<()> {
    crate::util::write_new_nofollow(path, bytes)?;
    let observed = crate::util::observe_regular_nofollow(path)?;
    journal.push(AppliedMutation::Created {
        path: path.to_path_buf(),
        observed,
    });
    Ok(())
}

fn apply_existing(
    update: &ExistingUpdate,
    journal: &mut Vec<AppliedMutation>,
) -> anyhow::Result<()> {
    let published =
        crate::util::compare_and_swap_nofollow(&update.path, &update.before, &update.desired)?;
    journal.push(AppliedMutation::Published {
        path: update.path.clone(),
        before: update.before.clone(),
        published,
    });
    Ok(())
}

fn rollback_applied(journal: &[AppliedMutation]) -> Vec<String> {
    let mut errors = Vec::new();
    for mutation in journal.iter().rev() {
        match mutation {
            AppliedMutation::Created { path, observed } => {
                match crate::util::remove_if_unchanged(path, observed) {
                    Ok(true) => {}
                    Ok(false) => errors.push(format!(
                        "{} changed after creation and was preserved",
                        path.display()
                    )),
                    Err(error) => errors.push(format!(
                        "could not remove transaction-created {}: {error}",
                        path.display()
                    )),
                }
            }
            AppliedMutation::Published {
                path,
                before,
                published,
            } => {
                if let Err(error) =
                    crate::util::compare_and_swap_nofollow(path, published, &before.bytes)
                {
                    errors.push(format!(
                        "could not restore transaction-updated {}: {error}",
                        path.display()
                    ));
                }
            }
        }
    }
    errors
}

fn inspect_complete_foundation(root: &std::path::Path) -> anyhow::Result<()> {
    let configs = existing_main_configs(root)?;
    if configs.len() != 1 {
        anyhow::bail!("foundation requires exactly one main config");
    }
    let config = crate::util::observe_regular_nofollow(&configs[0])?;
    validate_foundation_config_bytes(&configs[0], &config.bytes).map_err(anyhow::Error::msg)?;

    let identity = crate::util::observe_regular_nofollow(&root.join("anvil/project-id"))?;
    let raw_identity = std::str::from_utf8(&identity.bytes)?;
    let parsed = crate::activation::identity::ProjectIdentity::parse(raw_identity)?;
    validate_complete_identity(&parsed).map_err(anyhow::Error::msg)?;

    let ignore = crate::util::observe_regular_nofollow(&root.join(".gitignore"))?;
    let raw_ignore = std::str::from_utf8(&ignore.bytes)?;
    for expected in IGNORE_LINES {
        if !raw_ignore.lines().any(|line| line == expected) {
            anyhow::bail!(".gitignore is missing exact foundation line {expected}");
        }
    }
    Ok(())
}

fn is_not_found(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
    })
}

#[cfg(test)]
thread_local! {
    static FOUNDATION_FAILURE_BOUNDARY: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
fn foundation_boundary() -> anyhow::Result<()> {
    FOUNDATION_FAILURE_BOUNDARY.with(|boundary| match boundary.get() {
        Some(0) => {
            boundary.set(None);
            anyhow::bail!("injected foundation failure")
        }
        Some(remaining) => {
            boundary.set(Some(remaining - 1));
            Ok(())
        }
        None => Ok(()),
    })
}

#[cfg(not(test))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "production and fault-injected test boundaries share one call contract"
)]
fn foundation_boundary() -> anyhow::Result<()> {
    Ok(())
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
        assert!(
            catalogue[0]
                .targets
                .iter()
                .all(|target| !target.replaceable)
        );
        assert!(
            catalogue
                .iter()
                .any(|entry| entry.id == ComponentId::AcceptancePolicy
                    && entry.later_command == "anvil init --include acceptance-policy"
                    && entry.targets.iter().all(|target| !target.replaceable))
        );
        let replaceable = catalogue
            .iter()
            .flat_map(|component| component.targets)
            .filter(|target| target.replaceable)
            .collect::<Vec<_>>();
        assert_eq!(replaceable.len(), 1);
        assert_eq!(replaceable[0].target, "anvil/architecture.yaml");
    }

    #[test]
    fn every_config_target_resolves_through_the_settings_catalogue() {
        let settings = anvil_settings::first_release_catalogue().expect("settings catalogue");
        let scaffold_targets = catalogue()
            .iter()
            .flat_map(|component| component.targets)
            .filter(|target| target.kind == TargetKind::ConfigSetting)
            .map(|target| target.target)
            .collect::<BTreeSet<_>>();
        let settings_targets = settings
            .project_config_targets()
            .map(|(key, _)| key)
            .collect::<BTreeSet<_>>();

        assert_eq!(scaffold_targets, settings_targets);
        for key in scaffold_targets {
            let entry = settings.get(key).expect("catalogued setting");
            assert_eq!(entry.canonical_writer, "settings-service");
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
    fn fresh_git_foundation_refuses_while_shared_config_lock_is_held() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let _held = crate::util::ConfigMutationLock::try_acquire(root.path()).unwrap();

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::NeedsInput);
        assert_eq!(result.health, ComponentHealth::Invalid);
        assert!(
            result
                .diagnostic
                .as_deref()
                .is_some_and(|value| value.contains("already modifying project configuration"))
        );
        assert!(!root.path().join(".anvil.yaml").exists());
        assert!(!root.path().join("anvil/project-id").exists());
        assert!(!root.path().join(".gitignore").exists());
    }

    #[test]
    fn foundation_adds_missing_config_keys_without_changing_existing_bytes() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let config = root.path().join(".anvil.yaml");
        let original = "enforcement:\n  mode: block\n";
        std::fs::write(&config, original).unwrap();

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::Created);
        assert_eq!(result.health, ComponentHealth::Valid);
        let updated = std::fs::read_to_string(config).unwrap();
        assert!(updated.starts_with(original));
        assert!(updated.contains("schema_version: \"1.0.0\"\n"));
        assert!(updated.contains("format: \"yaml\"\n"));
    }

    #[test]
    fn foundation_augments_every_supported_config_format() {
        for (name, body, preserved) in [
            (
                ".anvil.yml",
                "# keep yml\nchecks: []\n",
                "# keep yml\nchecks: []\n",
            ),
            (
                ".anvil.json",
                "{\n    \"checks\" : []\n}\n",
                "    \"checks\" : []",
            ),
            (
                ".anvil.toml",
                "# keep toml\nchecks = [\"lint\"]\n",
                "# keep toml\nchecks = [\"lint\"]\n",
            ),
        ] {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir(root.path().join(".git")).unwrap();
            let path = root.path().join(name);
            std::fs::write(&path, body).unwrap();

            let result = reconcile_foundation(root.path()).unwrap();

            assert_eq!(result.health, ComponentHealth::Valid, "{name}");
            let updated = std::fs::read_to_string(path).unwrap();
            assert!(updated.contains(preserved), "{name}: {updated}");
        }
    }

    #[test]
    fn foundation_augments_owned_identity_fields_without_rotating_uuid() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        std::fs::create_dir(root.path().join("anvil")).unwrap();
        std::fs::write(
            root.path().join(".anvil.yaml"),
            "schema_version: \"1.0.0\"\nformat: yaml\nenforcement:\n  mode: block\n",
        )
        .unwrap();
        let uuid = "01997e4a-1b2c-7345-8901-abcdef123456";
        std::fs::write(
            root.path().join("anvil/project-id"),
            format!("project_uuid: {uuid}\n"),
        )
        .unwrap();

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::Created);
        assert_eq!(result.health, ComponentHealth::Valid);
        let identity = std::fs::read_to_string(root.path().join("anvil/project-id")).unwrap();
        assert!(identity.starts_with(&format!("project_uuid: {uuid}\n")));
        assert!(identity.contains("created_at:"));
        assert!(identity.contains("created_by_version:"));
        assert!(identity.contains("scaffold_version: 1\n"));
        assert!(
            std::fs::read_to_string(root.path().join(".anvil.yaml"))
                .unwrap()
                .contains("mode: block")
        );
    }

    #[test]
    fn foundation_rejects_unsupported_identity_scaffold_version() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        std::fs::create_dir(root.path().join("anvil")).unwrap();
        std::fs::write(
            root.path().join(".anvil.yaml"),
            "schema_version: \"1.0.0\"\nformat: yaml\n",
        )
        .unwrap();
        let identity = "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\ncreated_at: 2026-09-10T00:00:00Z\ncreated_by_version: 0.9.7-beta\nscaffold_version: 0\n";
        std::fs::write(root.path().join("anvil/project-id"), identity).unwrap();

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::NeedsInput);
        assert_eq!(result.health, ComponentHealth::Invalid);
        assert_eq!(
            std::fs::read_to_string(root.path().join("anvil/project-id")).unwrap(),
            identity
        );
        assert!(!root.path().join(".gitignore").exists());
    }

    #[test]
    fn foundation_rejects_path_inconsistent_format_without_writing() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let config = root.path().join(".anvil.json");
        let original = r#"{"schema_version":"1.0.0","format":"yaml"}"#;
        std::fs::write(&config, original).unwrap();

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::NeedsInput);
        assert_eq!(result.health, ComponentHealth::Invalid);
        assert_eq!(std::fs::read_to_string(config).unwrap(), original);
        assert!(!root.path().join("anvil/project-id").exists());
    }

    #[test]
    fn foundation_rolls_back_each_fresh_multi_file_failure_boundary() {
        for boundary in 0..4 {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir(root.path().join(".git")).unwrap();
            FOUNDATION_FAILURE_BOUNDARY.with(|slot| slot.set(Some(boundary)));

            let result = reconcile_foundation(root.path()).unwrap();

            assert_eq!(result.outcome, MutationOutcome::NeedsInput);
            assert!(!root.path().join(".anvil.yaml").exists());
            assert!(!root.path().join("anvil/project-id").exists());
            assert!(!root.path().join(".gitignore").exists());
        }
    }

    #[test]
    fn foundation_failure_restores_existing_file_publications() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let config = root.path().join(".anvil.yaml");
        let original = "checks:\n  - lint\n";
        std::fs::write(&config, original).unwrap();
        FOUNDATION_FAILURE_BOUNDARY.with(|slot| slot.set(Some(0)));

        let result = reconcile_foundation(root.path()).unwrap();

        assert_eq!(result.outcome, MutationOutcome::NeedsInput);
        assert_eq!(std::fs::read_to_string(config).unwrap(), original);
        assert!(!root.path().join("anvil/project-id").exists());
    }
}
