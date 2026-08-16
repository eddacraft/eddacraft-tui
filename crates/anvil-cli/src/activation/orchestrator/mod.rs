//! `anvil start` activation orchestration (LAUNCH-006 / LAUNCH-009).
//!
//! Composes init, MCP install, and verify into one entry path using the
//! read-safe activation primitives.

use std::collections::BTreeSet;
use std::io::{IsTerminal, Write as _};
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::GlobalArgs;
use crate::activation::agent_registry::{AgentClientId, InstallScope};
use crate::activation::baseline;
use crate::activation::detect_agents::{self, AgentKind, DetectionEnv, RealDetectionEnv};
use crate::activation::diagnostic::{
    ActivationDiagnostic, ConfigStatus, McpClientId, verify_with_home,
};
use crate::activation::identity;
use crate::commands::{hooks, init, mcp_installer};
use crate::registration::{self, WorktreeRegistration};
use crate::services::sample_analyser;

pub mod install;

pub use install::{InstallOutcome, InstallReport, SkipReason};

/// How `anvil start` will present activation to the operator.
///
/// The distinction is deliberately owned by the activation orchestrator rather
/// than by the renderer: the orchestrator is the layer that decides whether a
/// [`demand`] picker may be invoked. `StartRenderMode::Tui` therefore means
/// "an activation surface will own consent", not "fall back to the plain
/// interactive picker before opening the surface".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartRenderMode {
    /// Plain stdout/stderr output; existing interactive [`demand`] pickers are
    /// allowed when the session is otherwise interactive.
    Plain,
    /// Opt-in activation TUI; suppress [`demand`] pickers and record lifecycle
    /// detail for the surface seam instead.
    Tui,
}

impl StartRenderMode {
    pub(crate) fn allows_demand_pickers(self) -> bool {
        matches!(self, Self::Plain)
    }
}

/// Typed activation spine steps emitted by the orchestrator.
///
/// Ordering is the user-facing spine used by ACTTUI: durable work happens in
/// the initial working steps, then write-consent decisions move through
/// [`ActivationStep::WorkflowConsent`] and [`ActivationStep::McpConsent`], and
/// the final diagnostic becomes the verdict. The unit fixture below pins this
/// mapping so the future live TUI can render `Working -> Consent -> Verdict`
/// without reverse-engineering human diagnostic strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ActivationStep {
    InitialProbe,
    InitConfig,
    ProjectIdentity,
    WitnessAttributes,
    WorkflowConsent,
    GitHooks,
    BaselineSample,
    WorktreeRegistration,
    McpConsent,
    FinalProbe,
    Verdict,
}

impl ActivationStep {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::InitialProbe => "initial-probe",
            Self::InitConfig => "init-config",
            Self::ProjectIdentity => "project-identity",
            Self::WitnessAttributes => "witness-attributes",
            Self::WorkflowConsent => "workflow-consent",
            Self::GitHooks => "git-hooks",
            Self::BaselineSample => "baseline-sample",
            Self::WorktreeRegistration => "worktree-registration",
            Self::McpConsent => "mcp-consent",
            Self::FinalProbe => "final-probe",
            Self::Verdict => "verdict",
        }
    }
}

/// Lifecycle edge for one [`ActivationStep`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationStepLifecycle {
    Started,
    Completed,
    /// Write work is intentionally awaiting the activation TUI's explicit
    /// selection. This is typed state, not presentation copy.
    Deferred,
    Skipped,
    Failed,
}

impl ActivationStepLifecycle {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Deferred => "deferred",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivationStepEvent {
    pub step: ActivationStep,
    pub lifecycle: ActivationStepLifecycle,
    pub detail: Option<String>,
}

impl ActivationStepEvent {
    fn new(
        step: ActivationStep,
        lifecycle: ActivationStepLifecycle,
        detail: Option<String>,
    ) -> Self {
        Self {
            step,
            lifecycle,
            detail,
        }
    }

    pub(crate) fn render_line(&self) -> String {
        match &self.detail {
            Some(detail) => format!(
                "{}: {} — {detail}",
                self.step.label(),
                self.lifecycle.label()
            ),
            None => format!("{}: {}", self.step.label(), self.lifecycle.label()),
        }
    }
}

/// Accumulates lifecycle events and operator-facing log lines for one
/// activation run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ActivationRun {
    events: Vec<ActivationStepEvent>,
    log_lines: Vec<String>,
}

impl ActivationRun {
    #[cfg(test)]
    pub(crate) fn from_events(events: Vec<ActivationStepEvent>) -> Self {
        Self {
            events,
            log_lines: Vec::new(),
        }
    }

    #[cfg(test)]
    fn start(&mut self, step: ActivationStep) {
        self.events.push(ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Started,
            None,
        ));
    }

    #[cfg(test)]
    fn complete(&mut self, step: ActivationStep) {
        self.events.push(ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Completed,
            None,
        ));
    }

    #[cfg(test)]
    fn fail(&mut self, step: ActivationStep, detail: impl Into<String>) {
        self.events.push(ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Failed,
            Some(detail.into()),
        ));
    }

    fn log(&mut self, line: impl Into<String>) {
        self.log_lines.push(line.into());
    }

    pub(crate) fn events(&self) -> &[ActivationStepEvent] {
        &self.events
    }

    pub(crate) fn log_lines(&self) -> &[String] {
        &self.log_lines
    }

    /// CIB-183: whether this run's init step found the project config
    /// already on disk. This is the honest "the repo was activated before
    /// this run started" marker for repeat detection — derived from the
    /// recorded lifecycle evidence, never a timestamp guess. A first run
    /// records `InitConfig: completed` instead; a write-gated `ANVIL_HOME`
    /// records a different skip detail and deliberately does not count.
    pub(crate) fn config_present_before_run(&self) -> bool {
        self.events.iter().any(|event| {
            event.step == ActivationStep::InitConfig
                && event.lifecycle == ActivationStepLifecycle::Skipped
                && event.detail.as_deref() == Some(INIT_CONFIG_ALREADY_PRESENT_DETAIL)
        })
    }
}

pub(crate) type ActivationEventObserver<'a> =
    dyn FnMut(&ActivationStepEvent) -> anyhow::Result<()> + 'a;

struct ActivationRunRecorder<'a> {
    run: ActivationRun,
    observer: Option<&'a mut ActivationEventObserver<'a>>,
    observer_error: Option<anyhow::Error>,
}

impl<'a> ActivationRunRecorder<'a> {
    fn new(observer: Option<&'a mut ActivationEventObserver<'a>>) -> Self {
        Self {
            run: ActivationRun::default(),
            observer,
            observer_error: None,
        }
    }

    fn record(&mut self, event: &ActivationStepEvent) {
        self.run.events.push(event.clone());
        if self.observer_error.is_none()
            && let Some(observer) = self.observer.as_mut()
            && let Err(error) = observer(event)
        {
            self.observer_error = Some(error);
        }
    }

    fn start(&mut self, step: ActivationStep) {
        self.record(&ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Started,
            None,
        ));
    }

    fn complete(&mut self, step: ActivationStep) {
        self.record(&ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Completed,
            None,
        ));
    }

    fn skip(&mut self, step: ActivationStep, detail: impl Into<String>) {
        self.record(&ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Skipped,
            Some(detail.into()),
        ));
    }

    fn defer(&mut self, step: ActivationStep, detail: impl Into<String>) {
        self.record(&ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Deferred,
            Some(detail.into()),
        ));
    }

    fn fail(&mut self, step: ActivationStep, detail: impl Into<String>) {
        self.record(&ActivationStepEvent::new(
            step,
            ActivationStepLifecycle::Failed,
            Some(detail.into()),
        ));
    }

    fn finish(self) -> anyhow::Result<ActivationRun> {
        match self.observer_error {
            Some(error) => Err(error),
            None => Ok(self.run),
        }
    }
}

impl std::ops::Deref for ActivationRunRecorder<'_> {
    type Target = ActivationRun;

    fn deref(&self) -> &Self::Target {
        &self.run
    }
}

impl std::ops::DerefMut for ActivationRunRecorder<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.run
    }
}

/// CIB-183: the skip detail the init step records when the project config
/// already exists. Shared between the skip site and
/// [`ActivationRun::config_present_before_run`] so the repeat-detection
/// evidence cannot drift from the recorded event.
pub(crate) const INIT_CONFIG_ALREADY_PRESENT_DETAIL: &str = "project config already present";

/// Full orchestrator outcome used by the activation TUI seam.
#[derive(Debug)]
pub(crate) struct ActivationOutcome {
    pub diagnostic: ActivationDiagnostic,
    pub install_report: InstallReport,
    pub run: ActivationRun,
}

impl ActivationOutcome {
    #[cfg(test)]
    fn into_legacy_parts(self) -> (ActivationDiagnostic, InstallReport) {
        (self.diagnostic, self.install_report)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpInstallPolicy {
    Install,
    Skip,
}

/// Write category presented by the activation TUI consent surface.
///
/// CIB-245: the categories double as consent **sections/steps**, so git hook
/// installation is its own kind rather than a `Project` row — operators read
/// "change my git workflow" as a different decision from "seed project files".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiConsentOfferKind {
    Mcp,
    Workflow,
    Project,
    Hooks,
}

#[derive(Debug, Clone)]
enum TuiProjectAction {
    InitConfig {
        format: Option<anvil_config::ConfigFormat>,
    },
    ProjectIdentity {
        rotate: bool,
    },
    WitnessAttributes,
    GitHooks,
    Baseline,
}

impl TuiProjectAction {
    fn step(&self) -> ActivationStep {
        match self {
            Self::InitConfig { .. } => ActivationStep::InitConfig,
            Self::ProjectIdentity { .. } => ActivationStep::ProjectIdentity,
            Self::WitnessAttributes => ActivationStep::WitnessAttributes,
            Self::GitHooks => ActivationStep::GitHooks,
            Self::Baseline => ActivationStep::BaselineSample,
        }
    }
}

/// One stable, unticked-by-default write offer for the activation TUI.
///
/// CIB-245: `blurb` is the plain-language "what is this" — why anvil wants the
/// write and what happens if it is skipped. It is owned here, next to offer
/// construction, so the TUI and plain paths cannot drift. `description` stays
/// the path/action detail and is rendered as the secondary line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiConsentOffer {
    pub id: String,
    pub label: String,
    pub description: String,
    pub blurb: String,
    pub kind: TuiConsentOfferKind,
    pub repo_scoped: bool,
    pub unsafe_drift: Option<String>,
}

/// Deferred workflow/MCP actions paired with their user-facing offers.
#[derive(Debug, Clone)]
pub(crate) struct TuiConsentPlan {
    root: PathBuf,
    offers: Vec<TuiConsentOffer>,
    project_actions: Vec<(String, TuiProjectAction)>,
    workflows: std::collections::BTreeMap<String, WorkflowTemplate>,
    mcp_candidates: std::collections::BTreeMap<String, install::Candidate>,
    registry_mcp_candidates: std::collections::BTreeMap<String, RegistryMcpCandidate>,
    /// ACTTUI-018/020: MCP clients already configured (no write needed), for
    /// Verdict Install rows when Consent is skipped or filtered.
    settled_mcp: Vec<String>,
    home: Option<PathBuf>,
    fresh: Option<crate::activation::mcp_client::AnvilEntry>,
    enabled: BTreeSet<McpClientId>,
    project_writes_gated: bool,
}

#[derive(Debug, Clone, Copy)]
struct RegistryMcpCandidate {
    client: AgentClientId,
    scope: InstallScope,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RegistryMcpSelection<'a> {
    /// MCP install scope for registry offers (global vs project).
    pub scope: InstallScope,
    /// Explicit `--mcp-client` selections to force-include.
    pub explicit_clients: &'a [AgentClientId],
}

/// CIB-244: what actually happened to one registry MCP client this run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegistryInstallStatus {
    Installed { path: PathBuf },
    AlreadyConfigured { path: PathBuf },
    Skipped { reason: String },
    Failed { error: String },
}

/// CIB-244: one typed this-run install outcome for a registry (`AgentClientId`)
/// MCP client, so the verdict Install section can name the clients the operator
/// actually selected instead of only the dual-era `McpClientId` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryInstallRow {
    pub display_name: String,
    pub status: RegistryInstallStatus,
}

impl RegistryInstallRow {
    /// Human-readable row body, shared by the Install verdict section and the
    /// evidence log so the two cannot drift.
    pub(crate) fn label(&self) -> String {
        match &self.status {
            RegistryInstallStatus::Installed { path } => {
                format!("MCP installed at {}", path.display())
            }
            RegistryInstallStatus::AlreadyConfigured { path } => {
                format!("MCP already configured at {}", path.display())
            }
            RegistryInstallStatus::Skipped { reason } => format!("MCP install skipped: {reason}"),
            RegistryInstallStatus::Failed { error } => format!("MCP install failed: {error}"),
        }
    }

    /// Full evidence/log line (`"<client> MCP installed at …"`).
    pub(crate) fn line(&self) -> String {
        format!("{} {}", self.display_name, self.label())
    }

    /// Whether this run actually wrote the client's config.
    pub(crate) fn wrote(&self) -> bool {
        matches!(self.status, RegistryInstallStatus::Installed { .. })
    }

    pub(crate) fn is_error(&self) -> bool {
        matches!(
            self.status,
            RegistryInstallStatus::Skipped { .. } | RegistryInstallStatus::Failed { .. }
        )
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TuiConsentApplyOutcome {
    pub install_report: InstallReport,
    pub written_workflows: Vec<PathBuf>,
    pub workflow_error: Option<String>,
    pub selected_ids: BTreeSet<String>,
    pub project_applied: BTreeSet<ActivationStep>,
    pub project_skipped: std::collections::BTreeMap<ActivationStep, String>,
    pub project_errors: std::collections::BTreeMap<ActivationStep, String>,
    /// CIB-244: typed this-run registry MCP outcomes. `first_wave_mcp_lines`
    /// and `first_wave_mcp_errors` are rendered projections of these rows.
    pub registry_installs: Vec<RegistryInstallRow>,
    pub first_wave_mcp_lines: Vec<String>,
    pub first_wave_mcp_errors: Vec<String>,
}

impl TuiConsentPlan {
    pub(crate) fn offers(&self) -> &[TuiConsentOffer] {
        &self.offers
    }

    /// ACTTUI-018/020: human-readable settled MCP rows (already configured).
    pub(crate) fn settled_mcp(&self) -> &[String] {
        &self.settled_mcp
    }

    /// Apply only IDs that the returned TUI state says were ticked.
    ///
    /// Every write primitive re-checks the filesystem at apply time. A gated
    /// `ANVIL_HOME` also rejects repo-scoped workflow IDs defensively even if a
    /// caller fabricates a selection outside the consent widget.
    pub(crate) fn apply(&self, selected_ids: &[String]) -> TuiConsentApplyOutcome {
        let root = self.root.as_path();
        let selected = selected_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut project_applied = BTreeSet::new();
        let mut project_skipped = std::collections::BTreeMap::new();
        let mut project_errors = std::collections::BTreeMap::new();
        if !self.project_writes_gated {
            for (id, action) in &self.project_actions {
                if !selected.contains(id.as_str()) {
                    continue;
                }
                match apply_tui_project_action(root, action) {
                    Ok(ProjectActionOutcome::Applied) => {
                        project_applied.insert(action.step());
                    }
                    Ok(ProjectActionOutcome::Skipped(reason)) => {
                        project_skipped.insert(action.step(), reason);
                    }
                    Err(error) => {
                        project_errors.insert(action.step(), format!("{error:#}"));
                    }
                }
            }
        }
        let selected_workflows = if self.project_writes_gated {
            Vec::new()
        } else {
            self.workflows
                .iter()
                .filter(|(id, _)| selected.contains(id.as_str()))
                .map(|(_, workflow)| *workflow)
                .collect::<Vec<_>>()
        };
        let (written_workflows, workflow_error) =
            match install_selected_workflows(root, &selected_workflows) {
                Ok(written) => (written, None),
                Err(error) => (Vec::new(), Some(error.to_string())),
            };

        let selected_candidates = self
            .mcp_candidates
            .iter()
            .filter(|(id, candidate)| {
                selected.contains(id.as_str())
                    && !(self.project_writes_gated
                        && candidate.scope == crate::activation::mcp_client::ConfigScope::Workspace)
            })
            .map(|(_, candidate)| (candidate.id, candidate.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut install_report = self
            .fresh
            .as_ref()
            .map_or_else(InstallReport::default, |fresh| {
                install::install_selected_clients(
                    root,
                    self.home.as_deref(),
                    fresh,
                    &self.enabled,
                    &selected_candidates,
                )
            });
        install_report.hooks_active = project_applied.contains(&ActivationStep::GitHooks)
            && hooks::activation_hooks_active(root).unwrap_or(false);

        let registry_installs = self.apply_registry_mcp_candidates(&selected);
        // CIB-244: the log/evidence projections stay derived from the typed
        // rows so Install, Evidence, and the load-bearing failure check can
        // never disagree about what happened this run.
        let first_wave_mcp_lines = registry_installs
            .iter()
            .filter(|row| !row.is_error())
            .map(RegistryInstallRow::line)
            .collect();
        let first_wave_mcp_errors = registry_installs
            .iter()
            .filter(|row| row.is_error())
            .map(RegistryInstallRow::line)
            .collect();

        TuiConsentApplyOutcome {
            install_report,
            written_workflows,
            workflow_error,
            selected_ids: selected.into_iter().map(str::to_string).collect(),
            project_applied,
            project_skipped,
            project_errors,
            registry_installs,
            first_wave_mcp_lines,
            first_wave_mcp_errors,
        }
    }

    fn apply_registry_mcp_candidates(&self, selected: &BTreeSet<&str>) -> Vec<RegistryInstallRow> {
        let mut rows: Vec<RegistryInstallRow> = Vec::new();
        let command = crate::activation::mcp_client::preferred_mcp_command(None);
        for (id, candidate) in &self.registry_mcp_candidates {
            if !selected.contains(id.as_str()) {
                continue;
            }
            let display_name = candidate.client.entry().display_name.to_string();
            if self.project_writes_gated && candidate.scope == InstallScope::Project {
                rows.push(RegistryInstallRow {
                    display_name,
                    status: RegistryInstallStatus::Skipped {
                        reason: "project writes are gated for this ANVIL_HOME".to_string(),
                    },
                });
                continue;
            }
            let root = match candidate.scope {
                InstallScope::Global => {
                    let Some(home) = self.home.as_deref() else {
                        rows.push(RegistryInstallRow {
                            display_name,
                            status: RegistryInstallStatus::Failed {
                                error: "could not determine home directory".to_string(),
                            },
                        });
                        continue;
                    };
                    home
                }
                InstallScope::Project => self.root.as_path(),
            };
            let status = match mcp_installer::install(
                candidate.client,
                candidate.scope,
                root,
                command,
                false,
                false,
            ) {
                Ok(report) if report.wrote => {
                    RegistryInstallStatus::Installed { path: report.path }
                }
                Ok(report) => RegistryInstallStatus::AlreadyConfigured { path: report.path },
                Err(error) => RegistryInstallStatus::Failed {
                    error: format!("{error:#}"),
                },
            };
            rows.push(RegistryInstallRow {
                display_name,
                status,
            });
        }
        rows
    }
}

enum ProjectActionOutcome {
    Applied,
    Skipped(String),
}

fn apply_tui_project_action(
    root: &Path,
    action: &TuiProjectAction,
) -> anyhow::Result<ProjectActionOutcome> {
    match action {
        TuiProjectAction::InitConfig {
            format: Some(format),
        } => crate::commands::start::pre_write_anvil_config_format(root, *format)
            .map(|()| ProjectActionOutcome::Applied),
        TuiProjectAction::InitConfig { format: None } => {
            init::generate_config(&init::AnvilConfig::default(), root)
                .map(|_| ProjectActionOutcome::Applied)
        }
        TuiProjectAction::ProjectIdentity { rotate: true } => {
            identity::mint_new_identity(root, env!("CARGO_PKG_VERSION"))
                .map(|_| ProjectActionOutcome::Applied)
                .map_err(Into::into)
        }
        TuiProjectAction::ProjectIdentity { rotate: false } => {
            identity::ensure_project_id(root, env!("CARGO_PKG_VERSION"))
                .map(|_| ProjectActionOutcome::Applied)
                .map_err(Into::into)
        }
        TuiProjectAction::WitnessAttributes => ensure_witness_gitattributes(root)
            .context("write witness attributes")
            .map(|()| ProjectActionOutcome::Applied),
        TuiProjectAction::GitHooks => hooks::install_activation_hooks_silent(root).map(|active| {
            if active {
                ProjectActionOutcome::Applied
            } else {
                ProjectActionOutcome::Skipped("activation hooks remain inactive".to_string())
            }
        }),
        TuiProjectAction::Baseline => {
            if baseline::baseline_exists(root) {
                return Ok(ProjectActionOutcome::Skipped(
                    "activation baseline already present".to_string(),
                ));
            }
            if let Some(scan) = sample_analyser::run_baseline_scan(root) {
                let new_baseline = baseline::build_baseline(&scan.warnings, &scan.secrets);
                baseline::write_baseline(root, &new_baseline)?;
                Ok(ProjectActionOutcome::Applied)
            } else {
                Ok(ProjectActionOutcome::Skipped(
                    "no analysable files for baseline".to_string(),
                ))
            }
        }
    }
}

pub(crate) fn build_tui_consent_plan(
    root: &Path,
    mcp_install_policy: McpInstallPolicy,
    project_writes_gated: bool,
    config_format: Option<anvil_config::ConfigFormat>,
    rotate_identity: bool,
    registry_selection: RegistryMcpSelection<'_>,
) -> TuiConsentPlan {
    let home = crate::util::user_home_dir();
    // TUI consent is multi-harness: always *offer* every shipping legacy
    // adapter (Cursor + Claude Code) plus every scope-eligible registry
    // client. Offers stay unticked (CIB-184); nothing is written until the
    // user selects it. Non-interactive AutoInstall still uses detection
    // gating via `resolve_enabled_clients` (ACTMO-012).
    let mut enabled = crate::activation::mcp_client::all_client_ids();
    extend_enabled_with_explicit_clients(&mut enabled, registry_selection.explicit_clients);
    let fresh = if matches!(mcp_install_policy, McpInstallPolicy::Install) {
        Some(crate::activation::mcp_client::AnvilEntry::preferred_stdio())
    } else {
        None
    };
    let mut plan = build_tui_consent_plan_with_project_options(
        root,
        home.as_deref(),
        mcp_install_policy,
        &enabled,
        fresh,
        project_writes_gated,
        config_format,
        rotate_identity,
    );
    if matches!(mcp_install_policy, McpInstallPolicy::Install) {
        // Interactive consent always offers every scope-eligible registry
        // client (unticked until selected). Non-interactive AutoInstall of
        // undetected clients remains gated by `--all-mcp-clients` on the
        // plain path, not this offer list.
        plan.add_registry_mcp_offers(
            true,
            registry_selection.scope,
            registry_selection.explicit_clients,
        );
        // Clients that only support project scope (VS Code, Zed) still appear
        // on a default global start as project-scoped offers.
        if registry_selection.scope == InstallScope::Global {
            plan.add_project_only_registry_mcp_offers(registry_selection.explicit_clients);
        }
    }
    plan
}

#[cfg(test)]
pub(crate) fn build_tui_consent_plan_with_home(
    root: &Path,
    home: Option<&Path>,
    mcp_install_policy: McpInstallPolicy,
    enabled: &BTreeSet<McpClientId>,
    fresh: Option<crate::activation::mcp_client::AnvilEntry>,
    project_writes_gated: bool,
) -> TuiConsentPlan {
    build_tui_consent_plan_with_project_options(
        root,
        home,
        mcp_install_policy,
        enabled,
        fresh,
        project_writes_gated,
        None,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_tui_consent_plan_with_project_options(
    root: &Path,
    home: Option<&Path>,
    mcp_install_policy: McpInstallPolicy,
    enabled: &BTreeSet<McpClientId>,
    fresh: Option<crate::activation::mcp_client::AnvilEntry>,
    project_writes_gated: bool,
    config_format: Option<anvil_config::ConfigFormat>,
    rotate_identity: bool,
) -> TuiConsentPlan {
    let mut offers = Vec::new();
    let mut project_actions = Vec::new();
    add_tui_project_offers(
        root,
        home,
        config_format,
        rotate_identity,
        &mut offers,
        &mut project_actions,
    );
    let mut workflows = std::collections::BTreeMap::new();
    for workflow in pending_workflows(root) {
        let id = workflow.consent_id().to_string();
        offers.push(TuiConsentOffer {
            id: id.clone(),
            label: workflow.to_string(),
            description: workflow.label(root),
            blurb: workflow.blurb().to_string(),
            kind: TuiConsentOfferKind::Workflow,
            repo_scoped: true,
            unsafe_drift: None,
        });
        workflows.insert(id, workflow);
    }

    let mut mcp_candidates = std::collections::BTreeMap::new();
    if matches!(mcp_install_policy, McpInstallPolicy::Install)
        && let Some(fresh_entry) = fresh.as_ref()
    {
        for candidate in install::collect_candidates(root, home, fresh_entry) {
            let offerable = enabled.contains(&candidate.id)
                || !matches!(
                    candidate.drift,
                    crate::activation::mcp_client::DriftClass::NotPresent
                );
            if !offerable
                || matches!(
                    candidate.drift,
                    crate::activation::mcp_client::DriftClass::UpToDate
                )
            {
                continue;
            }

            let id = format!("mcp:{}", candidate.id.label());
            let unsafe_drift = match &candidate.drift {
                crate::activation::mcp_client::DriftClass::UnsafeDrift { reason } => {
                    Some(reason.clone())
                }
                _ => None,
            };
            let action = match &candidate.drift {
                crate::activation::mcp_client::DriftClass::NotPresent => "Write",
                crate::activation::mcp_client::DriftClass::SafeDrift { .. } => "Update",
                crate::activation::mcp_client::DriftClass::UnsafeDrift { .. } => "Inspect",
                crate::activation::mcp_client::DriftClass::UpToDate => unreachable!(),
            };
            offers.push(TuiConsentOffer {
                id: id.clone(),
                label: format!("{} MCP", candidate.id.display_name()),
                description: format!("{action} {}", candidate.target_path.display()),
                blurb: mcp_offer_blurb(candidate.id.display_name()),
                kind: TuiConsentOfferKind::Mcp,
                repo_scoped: candidate.scope
                    == crate::activation::mcp_client::ConfigScope::Workspace,
                unsafe_drift,
            });
            mcp_candidates.insert(id, candidate);
        }
    }

    TuiConsentPlan {
        root: root.to_path_buf(),
        offers,
        project_actions,
        workflows,
        mcp_candidates,
        registry_mcp_candidates: std::collections::BTreeMap::new(),
        settled_mcp: Vec::new(),
        home: home.map(Path::to_path_buf),
        fresh,
        enabled: enabled.clone(),
        project_writes_gated,
    }
}

impl TuiConsentPlan {
    fn add_registry_mcp_offers(
        &mut self,
        force_all: bool,
        scope: InstallScope,
        explicit_clients: &[AgentClientId],
    ) {
        if self.project_writes_gated && scope == InstallScope::Project {
            return;
        }
        let root = match scope {
            InstallScope::Global => {
                let Some(home) = self.home.as_ref() else {
                    return;
                };
                home.clone()
            }
            InstallScope::Project => self.root.clone(),
        };
        let env = RealDetectionEnv;
        if scope == InstallScope::Project {
            self.offers
                .retain(|offer| offer.id != "mcp:claude-code" && offer.id != "mcp:cursor");
            self.mcp_candidates
                .retain(|id, _| id != "mcp:claude-code" && id != "mcp:cursor");
        }
        // When force_all is true (TUI default), consider every client that
        // supports this scope (not only detected hosts). ACTTUI-018 still
        // filters out clients whose anvil MCP entry is already correct.
        // When false, keep detection/explicit gating. Writes still require a tick.
        for entry in AgentClientId::all().iter().filter(|entry| {
            !(scope == InstallScope::Global
                && matches!(entry.id, AgentClientId::ClaudeCode | AgentClientId::Cursor))
                && entry.supports_mcp(scope)
                && (force_all
                    || explicit_clients.contains(&entry.id)
                    || entry.detected_for_mcp(&env, scope, &root))
        }) {
            self.push_registry_mcp_offer(entry, scope, &root);
        }
    }

    /// Offer clients that only support project-scope MCP (VS Code, Zed) even
    /// when the start default scope is global, so the interactive list covers
    /// the full install registry without requiring `--mcp-scope project`.
    fn add_project_only_registry_mcp_offers(&mut self, explicit_clients: &[AgentClientId]) {
        if self.project_writes_gated {
            return;
        }
        let root = self.root.clone();
        for entry in AgentClientId::all().iter().filter(|entry| {
            entry.supports_mcp(InstallScope::Project)
                && !entry.supports_mcp(InstallScope::Global)
                && (explicit_clients.is_empty() || explicit_clients.contains(&entry.id))
        }) {
            self.push_registry_mcp_offer(entry, InstallScope::Project, &root);
        }
    }

    fn push_registry_mcp_offer(
        &mut self,
        entry: &crate::activation::agent_registry::AgentClient,
        scope: InstallScope,
        root: &Path,
    ) {
        let Some(path) = entry.mcp_path(scope, root) else {
            return;
        };
        let id = format!("mcp:{}", entry.id.label());
        if self.registry_mcp_candidates.contains_key(&id) {
            return;
        }

        // ACTTUI-018: dry-run the registry installer. When the expected anvil
        // entry is already present and matches, skip the offer (settled).
        let command = crate::activation::mcp_client::preferred_mcp_command(None);
        if let Ok(report) = crate::commands::mcp_installer::install(
            entry.id, scope, root, command, false, true, // dry_run
        ) && !report.changed
        {
            self.settled_mcp.push(format!(
                "{}: already configured at {}",
                entry.display_name,
                report.path.display(),
            ));
            return;
        }

        self.offers.push(TuiConsentOffer {
            id: id.clone(),
            label: format!("{} MCP", entry.display_name),
            description: format!("Write {}", path.display()),
            blurb: format!(
                "{} {}",
                mcp_offer_blurb(entry.display_name),
                entry.reload_hint,
            ),
            kind: TuiConsentOfferKind::Mcp,
            repo_scoped: scope == InstallScope::Project,
            unsafe_drift: None,
        });
        self.registry_mcp_candidates.insert(
            id,
            RegistryMcpCandidate {
                client: entry.id,
                scope,
            },
        );
    }
}

/// CIB-245: one-line class blurb for an MCP client row. MCP names are already
/// legible, so this stays short — it says what wiring the client up buys.
fn mcp_offer_blurb(display_name: &str) -> String {
    format!("Lets {display_name} call anvil's tools directly instead of guessing about your code.")
}

fn add_tui_config_offer(
    root: &Path,
    config_format: Option<anvil_config::ConfigFormat>,
    offers: &mut Vec<TuiConsentOffer>,
    actions: &mut Vec<(String, TuiProjectAction)>,
) {
    let id = "project:init-config".to_string();
    let target = config_format.map_or_else(
        || root.join(".anvil.yaml"),
        |format| root.join(format!(".anvil.{}", format.extension())),
    );
    let description = config_format.map_or_else(
        || {
            format!(
                "Create {} and its documented local project support files",
                target.display(),
            )
        },
        |_| format!("Create {}", target.display()),
    );
    offers.push(TuiConsentOffer {
        id: id.clone(),
        label: "Project configuration".to_string(),
        description,
        blurb: "anvil's settings file for this repo — which checks run and how \
                strict they are. Skip it and anvil runs on built-in defaults \
                with nothing to tune or commit."
            .to_string(),
        kind: TuiConsentOfferKind::Project,
        repo_scoped: true,
        unsafe_drift: None,
    });
    actions.push((
        id,
        TuiProjectAction::InitConfig {
            format: config_format,
        },
    ));
}

fn add_tui_project_offers(
    root: &Path,
    home: Option<&Path>,
    config_format: Option<anvil_config::ConfigFormat>,
    rotate_identity: bool,
    offers: &mut Vec<TuiConsentOffer>,
    actions: &mut Vec<(String, TuiProjectAction)>,
) {
    let initial = verify_with_home(root, home);
    if matches!(initial.config, ConfigStatus::Absent) {
        add_tui_config_offer(root, config_format, offers, actions);
    }

    let project_id = identity::project_id_path(root);
    if rotate_identity || !project_id.exists() {
        let id = "project:identity".to_string();
        offers.push(TuiConsentOffer {
            id: id.clone(),
            label: "Project identity".to_string(),
            description: format!(
                "{} {}",
                if rotate_identity { "Replace" } else { "Create" },
                project_id.display(),
            ),
            blurb: "A stable id so anvil can tell this project's history apart \
                    from other checkouts on this machine. Skip it and \
                    per-project evidence cannot be attributed across runs."
                .to_string(),
            kind: TuiConsentOfferKind::Project,
            repo_scoped: true,
            unsafe_drift: None,
        });
        actions.push((
            id,
            TuiProjectAction::ProjectIdentity {
                rotate: rotate_identity,
            },
        ));
    }

    if witness_gitattributes_needs_update(root).unwrap_or(true) {
        let id = "project:witness-attributes".to_string();
        offers.push(TuiConsentOffer {
            id: id.clone(),
            label: "Witness merge attributes".to_string(),
            description: format!("Update {}", root.join(".gitattributes").display()),
            blurb: "Tells git how to merge anvil's evidence files so two \
                    branches do not produce conflict markers in them. Skip it \
                    and you resolve those conflicts by hand."
                .to_string(),
            kind: TuiConsentOfferKind::Project,
            repo_scoped: true,
            unsafe_drift: None,
        });
        actions.push((id, TuiProjectAction::WitnessAttributes));
    }

    if root.join(".git").exists() && !hooks::activation_hooks_active(root).unwrap_or(false) {
        let id = "project:git-hooks".to_string();
        offers.push(TuiConsentOffer {
            id: id.clone(),
            label: "Commit and push hooks".to_string(),
            description: "Install anvil-managed pre-commit and pre-push hooks".to_string(),
            blurb: "Runs anvil's checks automatically when you commit or push, \
                    so new problems are caught before they leave your machine. \
                    Skip it and checks only run when you ask for them."
                .to_string(),
            kind: TuiConsentOfferKind::Hooks,
            repo_scoped: true,
            unsafe_drift: None,
        });
        actions.push((id, TuiProjectAction::GitHooks));
    }

    if !baseline::baseline_exists(root) {
        let id = "project:baseline".to_string();
        offers.push(TuiConsentOffer {
            id: id.clone(),
            label: "Activation baseline".to_string(),
            description: format!(
                "Record current findings at {} when analysable files exist",
                root.join(".anvil/baseline.json").display(),
            ),
            blurb: "Records today's findings as accepted, so anvil only warns \
                    about new problems instead of your whole backlog. Skip it \
                    and the first checks report existing code too."
                .to_string(),
            kind: TuiConsentOfferKind::Project,
            repo_scoped: true,
            unsafe_drift: None,
        });
        actions.push((id, TuiProjectAction::Baseline));
    }
}

/// Run the orchestration on `root` under `mcp_install_policy` and return the
/// final diagnostic alongside the install report.
///
/// The caller is responsible for rendering — the orchestrator is mute on
/// the activation diagnostic itself so unit tests can assert against the
/// returned struct without parsing stdout. Init's own output (config
/// success copy + first-scan summary) goes to stdout when init runs;
/// re-runs against an existing config produce no init output.
pub(crate) fn run_with_mcp_policy_and_mode(
    root: &Path,
    global: &GlobalArgs,
    mcp_install_policy: McpInstallPolicy,
    force_all_mcp_clients: bool,
    explicit_clients: &[AgentClientId],
    render_mode: StartRenderMode,
    rotate_identity: bool,
) -> anyhow::Result<ActivationOutcome> {
    let home = crate::util::user_home_dir();
    let mut enabled = resolve_enabled_clients(&RealDetectionEnv, force_all_mcp_clients);
    extend_enabled_with_explicit_clients(&mut enabled, explicit_clients);
    run_with_home_and_policy(
        root,
        home.as_deref(),
        global,
        mcp_install_policy,
        &enabled,
        render_mode,
        rotate_identity,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_with_mcp_policy_and_mode_observing<'a>(
    root: &Path,
    global: &GlobalArgs,
    mcp_install_policy: McpInstallPolicy,
    force_all_mcp_clients: bool,
    explicit_clients: &[AgentClientId],
    render_mode: StartRenderMode,
    rotate_identity: bool,
    observer: &'a mut ActivationEventObserver<'a>,
) -> anyhow::Result<ActivationOutcome> {
    let home = crate::util::user_home_dir();
    let mut enabled = resolve_enabled_clients(&RealDetectionEnv, force_all_mcp_clients);
    extend_enabled_with_explicit_clients(&mut enabled, explicit_clients);
    run_with_home_and_policy(
        root,
        home.as_deref(),
        global,
        mcp_install_policy,
        &enabled,
        render_mode,
        rotate_identity,
        Some(observer),
    )
}

#[allow(clippy::too_many_arguments)]
fn run_with_home_and_policy<'a>(
    root: &Path,
    home: Option<&Path>,
    global: &GlobalArgs,
    mcp_install_policy: McpInstallPolicy,
    enabled: &BTreeSet<McpClientId>,
    render_mode: StartRenderMode,
    rotate_identity: bool,
    observer: Option<&'a mut ActivationEventObserver<'a>>,
) -> anyhow::Result<ActivationOutcome> {
    run_with_home_and_registration_outcome(
        root,
        home,
        global,
        registration::register_worktree_with_daemon,
        mcp_install_policy,
        enabled,
        render_mode,
        rotate_identity,
        observer,
    )
}

/// Map a detected [`AgentKind`] to its [`McpClientId`], or `None` for
/// agents anvil detects but does not install an MCP entry for.
fn agent_to_mcp_client(kind: AgentKind) -> Option<McpClientId> {
    match kind {
        AgentKind::ClaudeCode => Some(McpClientId::ClaudeCode),
        AgentKind::Cursor => Some(McpClientId::Cursor),
        AgentKind::Codex => Some(McpClientId::Codex),
        // Aider / Windsurf are detected for the "AI tools" line but have
        // no first-wave MCP adapter.
        AgentKind::Aider | AgentKind::Windsurf => None,
    }
}

/// ACTMO-012: resolve which MCP clients are eligible for a *fresh*
/// install on this host.
///
/// `force_all` (the `--all-mcp-clients` flag or a non-empty
/// `ANVIL_ALL_MCP_CLIENTS`) returns every shipping client, preserving the
/// pre-ACTMO-012 "wire both editors" behaviour for power users who want
/// each editor pre-configured. Otherwise the set is the editors actually
/// detected on this host (binary on PATH / pre-existing editor state),
/// so `anvil start` never writes `~/.cursor/mcp.json` for an editor the
/// user does not have. Editors with an existing anvil entry are still
/// managed by the install path regardless of this set — see
/// `install_for_clients`.
fn resolve_enabled_clients(env: &dyn DetectionEnv, force_all: bool) -> BTreeSet<McpClientId> {
    // Read `ANVIL_ALL_MCP_CLIENTS` through the injected `DetectionEnv`
    // (presence-based, like `ANVIL_NO_MCP`: any non-empty value opts in)
    // so unit tests stay hermetic — `RealDetectionEnv::env` reads the
    // process environment in production; stubs return `None`.
    let env_opt_in = env
        .env("ANVIL_ALL_MCP_CLIENTS")
        .is_some_and(|value| !value.is_empty());
    if force_all || env_opt_in {
        return crate::activation::mcp_client::all_client_ids();
    }
    detect_agents::detect_all(env)
        .detected
        .iter()
        .filter_map(|a| agent_to_mcp_client(a.kind))
        .collect()
}

fn extend_enabled_with_explicit_clients(
    enabled: &mut BTreeSet<McpClientId>,
    explicit_clients: &[AgentClientId],
) {
    enabled.extend(explicit_clients.iter().copied());
}

#[cfg(test)]
fn run_with_home_and_registration(
    root: &Path,
    home: Option<&Path>,
    global: &GlobalArgs,
    register_worktree: impl FnOnce(&Path) -> WorktreeRegistration,
    mcp_install_policy: McpInstallPolicy,
    enabled: &BTreeSet<McpClientId>,
) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
    Ok(run_with_home_and_registration_outcome(
        root,
        home,
        global,
        register_worktree,
        mcp_install_policy,
        enabled,
        StartRenderMode::Plain,
        false,
        None,
    )?
    .into_legacy_parts())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn run_with_home_and_registration_outcome<'a>(
    root: &Path,
    home: Option<&Path>,
    global: &GlobalArgs,
    register_worktree: impl FnOnce(&Path) -> WorktreeRegistration,
    mcp_install_policy: McpInstallPolicy,
    enabled: &BTreeSet<McpClientId>,
    render_mode: StartRenderMode,
    rotate_identity: bool,
    observer: Option<&'a mut ActivationEventObserver<'a>>,
) -> anyhow::Result<ActivationOutcome> {
    let mut activation_run = ActivationRunRecorder::new(observer);

    // DISTRIB-006 (ADR-060): under a non-default ANVIL_HOME without
    // `--touch-project-state`, activation runs in a read-only posture — it still
    // verifies, installs MCP entries into the candidate's own home, and produces
    // a diagnostic, but it does NOT seed durable per-project state into the real
    // repo (`.anvil.yaml`, `anvil/project-id`, `.gitattributes`, GitHub workflows,
    // baseline). These are state the production binary reads; an unreleased
    // candidate must not write them silently. On an already-activated repo every
    // one of these is a write-if-absent no-op anyway, so the gate only changes
    // behaviour on a fresh repo — exactly where silent seeding would be wrong.
    let project_writes_gated = crate::install_root::project_writes_gated();
    if project_writes_gated {
        log_or_eprintln(
            &mut activation_run,
            render_mode,
            "anvil: ANVIL_HOME override active without --touch-project-state — \
             activation runs read-only; project-id, .gitattributes, workflows, and \
             baseline will not be written to this project. Pass --touch-project-state \
             to persist.",
        );
    }

    // Step 1 — write the project config if absent.
    activation_run.start(ActivationStep::InitialProbe);
    let initial = verify_with_home(root, home);
    activation_run.complete(ActivationStep::InitialProbe);
    if matches!(initial.config, ConfigStatus::Absent)
        && !project_writes_gated
        && matches!(render_mode, StartRenderMode::Tui)
    {
        activation_run.defer(
            ActivationStep::InitConfig,
            "project config awaits activation TUI consent",
        );
    } else if matches!(initial.config, ConfigStatus::Absent) && !project_writes_gated {
        activation_run.start(ActivationStep::InitConfig);
        let args = init::InitArgs { force: false };
        // Init runs inline here as a composition step of `anvil start`; the
        // activation ending owns the single next step, so init must not print
        // its own "Next: run `anvil start`" line — that would tell the user to
        // re-run the command they are already inside (CIB-163).
        init::run_in(&args, global, root, init::InitInvocation::FromStart)
            .context("init step of `anvil start` failed")?;
        activation_run.complete(ActivationStep::InitConfig);
    } else if project_writes_gated {
        activation_run.skip(
            ActivationStep::InitConfig,
            "project writes are gated for this ANVIL_HOME",
        );
    } else {
        activation_run.skip(
            ActivationStep::InitConfig,
            INIT_CONFIG_ALREADY_PRESENT_DETAIL,
        );
    }

    // Step 1a — establish project identity (MLP-001 / A7.2).
    //
    // Writes `anvil/project-id` (UUID v7) if absent. Idempotent on
    // re-run. This is the foundation for the v1 multi-layer protection
    // architecture — every witness line, every cross-machine federation,
    // every fork relationship anchors on this UUID.
    //
    // Failures here MUST NOT propagate (orchestrator pattern). The
    // identity file is a future-architecture-positioning aid for the
    // current release; without it, existing protection paths (MCP,
    // daemon, watch) still work unchanged.
    //
    // Council C-3 / C-9: surface the failure to the user. The
    // `tracing::warn!` alone is invisible at default log levels. We
    // also emit a single noise-disciplined eprintln! so the user can
    // see something went wrong, AND attach the structured `path`
    // field for log consumers.
    let project_id_path = identity::project_id_path(root);
    if project_writes_gated {
        activation_run.skip(
            ActivationStep::ProjectIdentity,
            "project writes are gated for this ANVIL_HOME",
        );
    } else if matches!(render_mode, StartRenderMode::Tui)
        && (rotate_identity || !project_id_path.exists())
    {
        activation_run.defer(
            ActivationStep::ProjectIdentity,
            if rotate_identity {
                "project identity rotation awaits activation TUI consent"
            } else {
                "project identity awaits activation TUI consent"
            },
        );
    } else if matches!(render_mode, StartRenderMode::Tui) {
        activation_run.skip(
            ActivationStep::ProjectIdentity,
            "project identity already present",
        );
    } else {
        activation_run.start(ActivationStep::ProjectIdentity);
        if let Err(e) = identity::ensure_project_id(root, env!("CARGO_PKG_VERSION")) {
            tracing::warn!(
                error = %e,
                path = %project_id_path.display(),
                "orchestrator: failed to establish anvil/project-id; continuing without",
            );
            log_or_eprintln(
                &mut activation_run,
                render_mode,
                format!(
                    "anvil: could not write {} ({e}); future MLP features will be unavailable",
                    project_id_path.display()
                ),
            );
            activation_run.skip(
                ActivationStep::ProjectIdentity,
                format!("could not write {}", project_id_path.display()),
            );
        } else {
            activation_run.complete(ActivationStep::ProjectIdentity);
        }
    }

    // Step 1a-b — pre-position `.gitattributes` for v1 witness chain
    // (council C-7 / Pragmatic Finding 6 / spec §5.1).
    //
    // MLP-002 (witness chain) hard-depends on `merge=union -text` for
    // `anvil/witness/active.ndjson` and the manifest. Adding the attribute
    // line at adoption time means MLP-002 can ship without forcing a
    // separate `.gitattributes` migration. Idempotent — only appends
    // if the line is missing. Failures non-propagating, same pattern
    // as identity.
    if project_writes_gated {
        activation_run.skip(
            ActivationStep::WitnessAttributes,
            "project writes are gated for this ANVIL_HOME",
        );
    } else if matches!(render_mode, StartRenderMode::Tui)
        && witness_gitattributes_needs_update(root).unwrap_or(true)
    {
        activation_run.defer(
            ActivationStep::WitnessAttributes,
            "witness attributes await activation TUI consent",
        );
    } else if matches!(render_mode, StartRenderMode::Tui) {
        activation_run.skip(
            ActivationStep::WitnessAttributes,
            "witness attributes already present",
        );
    } else {
        activation_run.start(ActivationStep::WitnessAttributes);
        if let Err(e) = ensure_witness_gitattributes(root) {
            tracing::warn!(
                error = %e,
                "orchestrator: failed to update .gitattributes for witness chain; continuing without",
            );
            activation_run.skip(
                ActivationStep::WitnessAttributes,
                "could not update .gitattributes for witness chain",
            );
        } else {
            activation_run.complete(ActivationStep::WitnessAttributes);
        }
    }

    let interactive = is_interactive(global);
    let demand_interactive = interactive && render_mode.allows_demand_pickers();

    // Step 1a-d — install ADR-038 commit/push hook coverage as part of the
    // MCP-optional activation spine (ACTMO-005). Hook install is durable
    // project state, so it follows the same gated-write posture as the rest of
    // activation. Failure is non-fatal: MCP and daemon-backed save-time
    // validation can still run, and the operator gets an explicit warning.
    //
    // CIB-164: capture whether the two hooks are *actually* anvil-managed after
    // the call so the first-run `verify:` block claims L3/L4 hook coverage only
    // when it is real. A write-gated posture, a failed install, or a
    // pre-existing unmanaged hook all yield `false` here — replacing the old
    // `.git`-exists heuristic that over-claimed in every one of those cases.
    let hooks_active = if project_writes_gated {
        activation_run.skip(
            ActivationStep::GitHooks,
            "project writes are gated for this ANVIL_HOME",
        );
        false
    } else if matches!(render_mode, StartRenderMode::Tui) {
        match hooks::activation_hooks_active(root) {
            Ok(true) => {
                activation_run.skip(ActivationStep::GitHooks, "activation hooks already active");
                true
            }
            Ok(false) if root.join(".git").exists() => {
                activation_run.defer(
                    ActivationStep::GitHooks,
                    "git hooks await activation TUI consent",
                );
                false
            }
            Ok(false) => {
                activation_run.skip(ActivationStep::GitHooks, "not a Git repository");
                false
            }
            Err(error) => {
                activation_run.fail(
                    ActivationStep::GitHooks,
                    format!("could not inspect git hooks: {error}"),
                );
                false
            }
        }
    } else {
        activation_run.start(ActivationStep::GitHooks);
        match hooks::install_activation_hooks_silent(root) {
            Ok(active) => {
                activation_run.complete(ActivationStep::GitHooks);
                active
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "orchestrator: failed to install activation git hooks; continuing without",
                );
                log_or_eprintln(
                    &mut activation_run,
                    render_mode,
                    format!("anvil: could not install git hooks ({e}); continuing"),
                );
                activation_run.fail(ActivationStep::GitHooks, "could not install git hooks");
                false
            }
        }
    };

    // Step 1b — write `.anvil/baseline.json` if absent (LAUNCH-010).
    // The baseline captures the set of antipattern + secret findings
    // present at first activation so future scans (post-LAUNCH-010
    // PRs across watch / check) can surface only NEW findings. We
    // write it only when absent — this is the activation-time
    // snapshot; subsequent `anvil start` runs are idempotent.
    //
    // Failures here MUST NOT propagate. The baseline is a future-
    // change-tracking aid, not a blocker for activation. A failed
    // write logs and continues; the diagnostic's
    // `baseline_present == false` is the honest signal.
    // DISTRIB-006 (ADR-060): the activation baseline write is part of the gated
    // read-only posture above — skipped under a non-default ANVIL_HOME without
    // `--touch-project-state` so a candidate cannot seed a real project's
    // baseline. `baseline_present == false` stays the honest signal.
    if project_writes_gated {
        activation_run.skip(
            ActivationStep::BaselineSample,
            "project writes are gated for this ANVIL_HOME",
        );
    } else if baseline::baseline_exists(root) {
        activation_run.skip(
            ActivationStep::BaselineSample,
            "activation baseline already present",
        );
    } else if matches!(render_mode, StartRenderMode::Tui) {
        activation_run.defer(
            ActivationStep::BaselineSample,
            "activation baseline awaits activation TUI consent",
        );
    } else if let Some(scan) = sample_analyser::run_baseline_scan(root) {
        activation_run.start(ActivationStep::BaselineSample);
        let new_baseline = baseline::build_baseline(&scan.warnings, &scan.secrets);
        if let Err(e) = baseline::write_baseline(root, &new_baseline) {
            tracing::warn!(
                error = %e,
                "orchestrator: failed to write activation baseline; continuing without",
            );
            activation_run.skip(
                ActivationStep::BaselineSample,
                "could not write activation baseline",
            );
        } else {
            activation_run.complete(ActivationStep::BaselineSample);
        }
    } else {
        activation_run.skip(
            ActivationStep::BaselineSample,
            "no analysable files for baseline",
        );
    }

    // ACTMO-016 (ADR-094 decision 4): only register cwd when it is a
    // registerable Git worktree. Outside one (a bare repo, inside `.git`, or
    // not a repo at all) `anvil start` stays honest — it does not register a
    // junk session keyed to e.g. $HOME; the daemon is still ensured by the
    // caller, and `start.rs` surfaces the "no worktree registered" guidance.
    if let Err(reason) = registration::registerable_worktree(root) {
        tracing::info!(
            error = %reason,
            "orchestrator: cwd is not a registerable worktree; daemon ensured, cwd not registered",
        );
        activation_run.skip(
            ActivationStep::WorktreeRegistration,
            format!("cwd is not a registerable worktree: {reason}"),
        );
    } else {
        activation_run.start(ActivationStep::WorktreeRegistration);
        match register_worktree(root) {
            WorktreeRegistration::Registered | WorktreeRegistration::Refreshed => {
                activation_run.complete(ActivationStep::WorktreeRegistration);
            }
            WorktreeRegistration::DaemonUnavailable => {
                tracing::debug!(
                    "orchestrator: daemon unavailable for activation worktree registration; continuing",
                );
                activation_run.skip(
                    ActivationStep::WorktreeRegistration,
                    "daemon unavailable for worktree registration",
                );
            }
            WorktreeRegistration::Fenced(message) | WorktreeRegistration::CapExceeded(message) => {
                tracing::warn!(
                    error = %message,
                    "orchestrator: activation worktree registration refused; continuing",
                );
                activation_run.skip(
                    ActivationStep::WorktreeRegistration,
                    format!("registration refused: {message}"),
                );
            }
            WorktreeRegistration::Rejected(error) => {
                tracing::warn!(
                    error = %error,
                    "orchestrator: activation worktree registration rejected; continuing",
                );
                activation_run.skip(
                    ActivationStep::WorktreeRegistration,
                    format!("registration rejected: {error}"),
                );
            }
        }
    }

    // Step 1c — offer GitHub Actions workflow installation (MLP2-043 /
    // MLP2-053) as the first Consent-phase sub-step.
    //
    // GitHub Actions workflows change repo behaviour and may consume customer
    // CI minutes. Interactive plain activation uses the existing `demand`
    // picker; the TUI path records a deferred consent event for ACTTUI-004
    // instead of invoking `demand` before the surface opens. This ordering is
    // intentional: all preceding steps are `Working`, while workflow + MCP
    // consent are the `Consent` phase before the final `Verdict`.
    if project_writes_gated {
        activation_run.skip(
            ActivationStep::WorkflowConsent,
            "project writes are gated for this ANVIL_HOME",
        );
    } else if matches!(render_mode, StartRenderMode::Tui) && !pending_workflows(root).is_empty() {
        activation_run.defer(
            ActivationStep::WorkflowConsent,
            "deferred to activation TUI consent surface",
        );
    } else {
        activation_run.start(ActivationStep::WorkflowConsent);
        if let Err(e) = ensure_github_actions_workflows(root, demand_interactive) {
            tracing::warn!(
                error = %e,
                "orchestrator: failed to install GitHub Actions workflows; continuing without",
            );
            log_or_eprintln(
                &mut activation_run,
                render_mode,
                format!("anvil: could not install GitHub Actions workflows ({e}); continuing"),
            );
            activation_run.skip(
                ActivationStep::WorkflowConsent,
                "could not install GitHub Actions workflows",
            );
        } else {
            activation_run.complete(ActivationStep::WorkflowConsent);
        }
    }

    // Step 2 — install MCP entries for the user-selected (or auto-
    // selected) clients. The install module handles drift, picker UX,
    // and atomic writes; failures are folded into the report rather
    // than propagated, so the orchestrator always returns a final
    // diagnostic the user can act on.
    let mut install_report = match mcp_install_policy {
        McpInstallPolicy::Skip => {
            activation_run.skip(ActivationStep::McpConsent, "MCP installation disabled");
            InstallReport::default()
        }
        McpInstallPolicy::Install => {
            let fresh = crate::activation::mcp_client::AnvilEntry::preferred_stdio();
            if matches!(render_mode, StartRenderMode::Tui) {
                // Mirrors the WorkflowConsent deferral above: no legacy
                // picker is shown, so the step is explicitly Deferred
                // rather than Started/Completed, which would
                // otherwise misreport a "Passed" consent step in the TUI
                // progress panel before the surface's own consent widget
                // has run.
                if tui_mcp_offer_available(root, home, &fresh, enabled) {
                    activation_run.defer(
                        ActivationStep::McpConsent,
                        "deferred to activation TUI consent surface",
                    );
                } else {
                    activation_run.skip(
                        ActivationStep::McpConsent,
                        "no MCP changes available for consent",
                    );
                }
                install::install_for_clients_with_consent_mode(
                    root,
                    home,
                    &fresh,
                    install::InstallConsentMode::DeferToTui,
                    enabled,
                )
            } else {
                activation_run.start(ActivationStep::McpConsent);
                let report =
                    install::install_for_clients(root, home, &fresh, demand_interactive, enabled);
                activation_run.complete(ActivationStep::McpConsent);
                report
            }
        }
    };
    // CIB-164: carry the honest hook-coverage bool alongside the MCP report so
    // the render path claims L3/L4 only when the hooks are really installed.
    install_report.hooks_active = hooks_active;

    // Step 3 — final probe. The diagnostic absorbs the install side
    // effects (e.g. tiers should now read `RestartRequired` for the
    // clients we just wrote) so the caller can render a single source
    // of truth.
    activation_run.start(ActivationStep::FinalProbe);
    let mut diagnostic = verify_with_home(root, home);
    activation_run.complete(ActivationStep::FinalProbe);
    record_daemon_attestation_log(&mut activation_run, &diagnostic);

    // Surface every install failure on the diagnostic so
    // `protection_state()` collapses to `Error` and JSON consumers
    // see all simultaneous failures, not just the first one.
    if let Some(err) = install_report.aggregated_failure() {
        diagnostic.last_error = Some(format!("MCP install failed: {err}"));
    }

    activation_run.start(ActivationStep::Verdict);
    activation_run.complete(ActivationStep::Verdict);
    let activation_run = activation_run.finish()?;

    Ok(ActivationOutcome {
        diagnostic,
        install_report,
        run: activation_run,
    })
}

fn tui_mcp_offer_available(
    root: &Path,
    home: Option<&Path>,
    fresh: &crate::activation::mcp_client::AnvilEntry,
    enabled: &BTreeSet<McpClientId>,
) -> bool {
    install::collect_candidates(root, home, fresh)
        .into_iter()
        .any(|candidate| {
            (enabled.contains(&candidate.id)
                || !matches!(
                    candidate.drift,
                    crate::activation::mcp_client::DriftClass::NotPresent
                ))
                && !matches!(
                    candidate.drift,
                    crate::activation::mcp_client::DriftClass::UpToDate
                )
        })
}

fn log_or_eprintln(
    activation_run: &mut ActivationRun,
    render_mode: StartRenderMode,
    line: impl Into<String>,
) {
    let line = line.into();
    if matches!(render_mode, StartRenderMode::Tui) {
        activation_run.log(line.clone());
    }
    eprintln!("{line}");
}

fn record_daemon_attestation_log(
    activation_run: &mut ActivationRun,
    diagnostic: &ActivationDiagnostic,
) {
    use crate::activation::daemon_evidence::DaemonAttestation;

    let detail = match diagnostic.daemon_attestation {
        DaemonAttestation::NotProbed => None,
        DaemonAttestation::Unreachable => Some(
            "daemon attestation skipped: daemon IPC was unavailable; start the intercept daemon if MCP restart is complete",
        ),
        DaemonAttestation::Unenforced => Some(
            "daemon attestation skipped: this worktree is not currently enforced by the daemon",
        ),
        DaemonAttestation::StaleHeartbeat => {
            Some("daemon attestation skipped: daemon heartbeat evidence is stale")
        }
        DaemonAttestation::AllSurfacesQuarantined => Some(
            "daemon attestation skipped: all daemon surfaces for this worktree are quarantined",
        ),
        DaemonAttestation::Warming => {
            Some("daemon attestation skipped: daemon worktree evidence is still warming")
        }
        DaemonAttestation::NoParticipatingSurface => Some(
            "daemon attestation skipped: no participating daemon surface is attached to this worktree",
        ),
        DaemonAttestation::Enforced => {
            Some("daemon attestation: worktree is enforced; no MCP client was promoted")
        }
        DaemonAttestation::Promoted => {
            Some("daemon attestation: promoted MCP client to live validation")
        }
    };

    if let Some(detail) = detail {
        activation_run.log(detail);
    }
}

/// Pre-position `.gitattributes` lines for the v1 witness chain
/// (council C-7 / spec §5.1).
///
/// Adds `merge=union -text` for `anvil/witness/active.ndjson` and the
/// manifest if not already present. Idempotent — searches for the
/// exact line before appending so re-running `anvil start` doesn't
/// duplicate. Creates `.gitattributes` if it doesn't exist.
///
/// The orchestrator writes the attribute at adoption time so the
/// shipped witness chain (MLP-002 / MLP2-005 — commits emit
/// `active.ndjson` and the manifest via the git hooks and the intercept
/// daemon) union-merges across parallel branches instead of producing
/// conflicts.
const WITNESS_GITATTRIBUTE_LINES: &[&str] = &[
    "anvil/witness/active.ndjson merge=union -text",
    "anvil/witness/manifest/chain.ndjson merge=union -text",
];

fn witness_gitattributes_needs_update(root: &Path) -> std::io::Result<bool> {
    let path = root.join(".gitattributes");
    let existing = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };
    Ok(WITNESS_GITATTRIBUTE_LINES.iter().any(|line| {
        !existing
            .lines()
            .any(|existing_line| existing_line.trim() == *line)
    }))
}

fn ensure_witness_gitattributes(root: &Path) -> std::io::Result<()> {
    // Per spec §5.1 + ADR-037 §D-3, the active witness file lives at
    // `anvil/witness/active.ndjson` (not the deprecated top-level
    // `anvil/witnessed.ndjson` shorthand that appeared in early drafts).
    // Set `merge=union -text` on both the active file and the manifest
    // so the shipped witness chain (MLP-002 / MLP2-005) union-merges
    // across parallel branches without a separate `.gitattributes`
    // migration.
    let path = root.join(".gitattributes");
    let existing = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let mut to_append = String::new();
    for line in WITNESS_GITATTRIBUTE_LINES {
        if !existing
            .lines()
            .any(|existing_line| existing_line.trim() == *line)
        {
            if to_append.is_empty() && !existing.is_empty() && !existing.ends_with('\n') {
                to_append.push('\n');
            }
            to_append.push_str(line);
            to_append.push('\n');
        }
    }

    if to_append.is_empty() {
        return Ok(()); // Idempotent — nothing to do.
    }

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    f.write_all(to_append.as_bytes())?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum WorkflowTemplate {
    PrValidation,
    Audit,
}

impl WorkflowTemplate {
    fn consent_id(self) -> &'static str {
        match self {
            Self::PrValidation => "workflow:pr-validation",
            Self::Audit => "workflow:audit",
        }
    }

    fn target_path(self, root: &Path) -> PathBuf {
        let workflows_dir = root.join(".github").join("workflows");
        match self {
            Self::PrValidation => workflows_dir.join("anvil.yml"),
            Self::Audit => workflows_dir.join("anvil-audit.yml"),
        }
    }

    fn label(self, root: &Path) -> String {
        let target = workflow_display_path(root, &self.target_path(root));
        match self {
            Self::PrValidation => format!("PR validation ({target}) [pull_request]"),
            Self::Audit => format!("Nightly audit ({target}) [schedule]"),
        }
    }

    /// CIB-245: plain-language "what is this" for the consent row. Owned here
    /// beside the template so the TUI cannot invent its own wording.
    fn blurb(self) -> &'static str {
        match self {
            Self::PrValidation => {
                "A GitHub Actions job that runs anvil's checks on every pull \
                 request, so problems are visible in review. Skip it and \
                 nothing runs anvil in CI."
            }
            Self::Audit => {
                "A nightly GitHub Actions job that re-runs anvil across the \
                 whole repo and records the result. Skip it and you only see \
                 the files each change touches."
            }
        }
    }

    fn contents(self) -> &'static str {
        match self {
            Self::PrValidation => crate::commands::anvil_action::anvil_workflow_template(),
            Self::Audit => crate::commands::audit_chain::audit_workflow_template(),
        }
    }
}

impl std::fmt::Display for WorkflowTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrValidation => f.write_str("PR validation"),
            Self::Audit => f.write_str("Nightly audit"),
        }
    }
}

fn workflow_display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Offer GitHub Actions workflow installation.
///
/// Interactive sessions show an unticked list of absent workflow files. Every
/// entry starts unselected (CIB-165), so a plain Enter writes nothing; the
/// operator must tick a workflow to consent to installing it. In
/// non-interactive sessions we skip entirely so customer repos are not modified
/// without an operator seeing the list.
fn ensure_github_actions_workflows(
    root: &Path,
    interactive: bool,
) -> std::io::Result<Vec<PathBuf>> {
    let candidates = pending_workflows(root);
    if candidates.is_empty() || !interactive {
        return Ok(Vec::new());
    }

    eprintln!("anvil: press Enter to skip GitHub Actions workflow install");
    let selected = show_workflow_picker(root, &candidates)?;
    let written = install_selected_workflows(root, &selected)?;
    for path in &written {
        eprintln!(
            "anvil: installed GitHub Actions workflow {}",
            workflow_display_path(root, path),
        );
    }
    Ok(written)
}

fn pending_workflows(root: &Path) -> Vec<WorkflowTemplate> {
    [WorkflowTemplate::PrValidation, WorkflowTemplate::Audit]
        .into_iter()
        .filter(|workflow| !workflow.target_path(root).exists())
        .collect()
}

/// Build the `(workflow, label, selected)` tuples backing the interactive
/// picker.
///
/// Every candidate defaults to `selected = false` (CIB-165): a plain
/// Enter-through therefore selects nothing and writes no CI files, so a hurried
/// operator never silently adds workflows to a shared repo. Ticking a workflow
/// is the explicit consent.
fn workflow_picker_options(
    root: &Path,
    candidates: &[WorkflowTemplate],
) -> Vec<(WorkflowTemplate, String, bool)> {
    candidates
        .iter()
        .map(|workflow| (*workflow, workflow.label(root), false))
        .collect()
}

fn show_workflow_picker(
    root: &Path,
    candidates: &[WorkflowTemplate],
) -> std::io::Result<Vec<WorkflowTemplate>> {
    use demand::{DemandOption, MultiSelect};

    let mut picker = MultiSelect::new("Install or enable GitHub Actions workflows?")
        .description(
            "Nothing is selected by default — press Enter to skip. Space ticks a workflow; \
             Enter installs your ticked selection (writing each only if absent).",
        )
        .filterable(false)
        .min(0)
        .max(candidates.len());

    for (workflow, label, selected) in workflow_picker_options(root, candidates) {
        picker = picker.option(DemandOption::new(workflow).label(&label).selected(selected));
    }

    let _raw_guard = WorkflowRawModeCleanupGuard;
    match picker.run() {
        Ok(workflows) => Ok(workflows),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

struct WorkflowRawModeCleanupGuard;
impl Drop for WorkflowRawModeCleanupGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Drop selected GitHub Actions workflow templates into `.github/workflows/`.
///
/// Write-if-absent semantics — once a file exists we never touch it, so an
/// operator who edits triggers or swaps the install step keeps their changes
/// across re-runs of `anvil start` / `anvil baseline`. The
/// `.github/workflows/` parent is created if missing.
///
/// Errors propagate to the caller so the orchestrator can decide
/// whether to log + continue. Operators without write access to `.github/`
/// should not have activation hard-fail on this step.
fn install_selected_workflows(
    root: &Path,
    selected: &[WorkflowTemplate],
) -> std::io::Result<Vec<PathBuf>> {
    let workflows_dir = root.join(".github").join("workflows");
    let mut written = Vec::new();
    let mut workflows_dir_created = false;

    for workflow in selected {
        let target = workflow.target_path(root);
        if existing_workflow_target(&target)? {
            continue; // Idempotent — never clobber an existing file.
        }
        refuse_workflow_parent_symlinks(root)?;
        if !workflows_dir_created {
            std::fs::create_dir_all(&workflows_dir)?;
            refuse_workflow_parent_symlinks(root)?;
            workflows_dir_created = true;
        }
        if existing_workflow_target(&target)? {
            continue;
        }
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        file.write_all(workflow.contents().as_bytes())?;
        written.push(target);
    }
    Ok(written)
}

fn existing_workflow_target(target: &Path) -> std::io::Result<bool> {
    match std::fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "refusing to write workflow through symlink: {}",
                target.display()
            ),
        )),
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

fn refuse_workflow_parent_symlinks(root: &Path) -> std::io::Result<()> {
    for path in [root.join(".github"), root.join(".github/workflows")] {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "refusing to write workflow through symlink: {}",
                        path.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Decide whether to surface the interactive picker. We require:
/// - not `--json` (defensive: in practice `commands/start.rs` short-
///   circuits to read-only verify on `--json` so the orchestrator
///   never runs in that mode, but the gate stays here so any future
///   caller of `run_with_home` under `--json` cannot accidentally
///   prompt)
/// - not `--no-tui` (explicit user opt-out)
/// - stdin is a TTY (`demand` reads keystrokes from stdin)
/// - stderr is a TTY (`demand` renders the prompt to stderr; piping
///   stderr to a file would render the prompt invisibly while still
///   consuming keystrokes)
/// - not running under a known non-interactive shell context
///   (`CI=true`, `GIT_DIR` set, `ANVIL_NO_PROMPT`, etc. — see
///   [`crate::is_non_interactive_env`])
///
/// Council remediation: previously checked `stdout.is_terminal()`, which
/// misclassified `anvil start | tee log.txt` (auto-installs silently)
/// and `echo "" | anvil start` (picker hangs on closed stdin). The new
/// check matches the convention in `commands/tutorial.rs:41` and the
/// auth-prompt gate in `main.rs:413`.
fn is_interactive(global: &GlobalArgs) -> bool {
    !global.json
        && !global.no_tui
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal()
        && !crate::is_non_interactive_env()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GlobalArgs;
    use crate::activation::diagnostic::McpClientId;
    use tempfile::TempDir;

    fn default_global() -> GlobalArgs {
        // `--no-tui` forces the non-interactive auto-install branch
        // so unit tests don't try to summon a picker.
        GlobalArgs {
            no_tui: true,
            ..Default::default()
        }
    }

    /// Minimal [`DetectionEnv`] stub for the enabled-client resolution
    /// tests — only `has_binary` matters here.
    struct StubDetectionEnv {
        binaries: std::collections::HashSet<String>,
    }
    impl StubDetectionEnv {
        fn with_binary(name: &str) -> Self {
            let mut binaries = std::collections::HashSet::new();
            binaries.insert(name.to_string());
            Self { binaries }
        }
    }
    impl DetectionEnv for StubDetectionEnv {
        fn has_binary(&self, name: &str) -> bool {
            self.binaries.contains(name)
        }
        fn path_exists(&self, _path: &str) -> bool {
            false
        }
        fn env(&self, _name: &str) -> Option<String> {
            None
        }
        fn home_dir(&self) -> Option<String> {
            None
        }
    }

    #[test]
    fn activation_run_happy_path_order_maps_working_consent_verdict() {
        // ACTTUI-002 golden: workflow consent is explicitly ordered before MCP
        // consent so the TUI can render the real spine as
        // Working -> Consent -> Verdict instead of scraping plain strings.
        let mut run = ActivationRun::default();
        for step in [
            ActivationStep::InitialProbe,
            ActivationStep::InitConfig,
            ActivationStep::ProjectIdentity,
            ActivationStep::WitnessAttributes,
            ActivationStep::GitHooks,
            ActivationStep::BaselineSample,
            ActivationStep::WorktreeRegistration,
            ActivationStep::WorkflowConsent,
            ActivationStep::McpConsent,
            ActivationStep::FinalProbe,
            ActivationStep::Verdict,
        ] {
            run.start(step);
            run.complete(step);
        }

        let rendered = run
            .events()
            .iter()
            .map(ActivationStepEvent::render_line)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(
            rendered,
            "\
initial-probe: started
initial-probe: completed
init-config: started
init-config: completed
project-identity: started
project-identity: completed
witness-attributes: started
witness-attributes: completed
git-hooks: started
git-hooks: completed
baseline-sample: started
baseline-sample: completed
worktree-registration: started
worktree-registration: completed
workflow-consent: started
workflow-consent: completed
mcp-consent: started
mcp-consent: completed
final-probe: started
final-probe: completed
verdict: started
verdict: completed"
        );
    }

    #[test]
    fn activation_run_streams_each_typed_event_as_it_is_recorded() {
        let mut streamed_events = Vec::new();
        let mut observer = |event: &ActivationStepEvent| {
            streamed_events.push(event.clone());
            Ok(())
        };
        let mut run = ActivationRunRecorder::new(Some(&mut observer));

        run.start(ActivationStep::InitialProbe);
        run.complete(ActivationStep::InitialProbe);
        let completed = run.finish().expect("observer stays healthy");

        assert_eq!(streamed_events, completed.events());
        assert_eq!(
            streamed_events[0].lifecycle,
            ActivationStepLifecycle::Started
        );
        assert_eq!(
            streamed_events[1].lifecycle,
            ActivationStepLifecycle::Completed
        );
    }

    #[test]
    fn tui_render_mode_suppresses_demand_pickers() {
        assert!(StartRenderMode::Plain.allows_demand_pickers());
        assert!(!StartRenderMode::Tui.allows_demand_pickers());
    }

    #[test]
    fn orchestrator_tui_mode_defers_consent_without_mcp_writes() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let outcome = run_with_home_and_registration_outcome(
            dir.path(),
            Some(home.path()),
            &global,
            |_| WorktreeRegistration::DaemonUnavailable,
            McpInstallPolicy::Install,
            &crate::activation::mcp_client::all_client_ids(),
            StartRenderMode::Tui,
            false,
            None,
        )
        .expect("orchestrator should succeed in TUI mode");

        assert!(
            outcome.run.events().iter().any(|event| {
                event.step == ActivationStep::WorkflowConsent
                    && event.lifecycle == ActivationStepLifecycle::Deferred
                    && event.detail.as_deref() == Some("deferred to activation TUI consent surface")
            }),
            "workflow consent must be deferred instead of invoking demand: {:?}",
            outcome.run.events(),
        );
        assert!(
            outcome.run.events().iter().any(|event| {
                event.step == ActivationStep::McpConsent
                    && event.lifecycle == ActivationStepLifecycle::Deferred
                    && event.detail.as_deref() == Some("deferred to activation TUI consent surface")
            }),
            "MCP consent must be deferred instead of recording a false Started/Completed pass: {:?}",
            outcome.run.events(),
        );
        assert!(
            !home.path().join(".cursor/mcp.json").exists(),
            "TUI mode must not silently auto-install Cursor MCP while consent widgets are deferred",
        );
        assert!(
            !home.path().join(".claude.json").exists(),
            "TUI mode must not silently auto-install Claude MCP while consent widgets are deferred",
        );
        assert!(!dir.path().join(".anvilrc").exists());
        assert!(!dir.path().join(".anvil.yaml").exists());
        assert!(!dir.path().join("anvil/project-id").exists());
        assert!(!dir.path().join(".gitattributes").exists());
        assert!(!dir.path().join(".anvil/baseline.json").exists());
        assert!(matches!(
            outcome.install_report.per_client.get(&McpClientId::Cursor),
            Some(InstallOutcome::Skipped {
                reason: SkipReason::ConsentDeferredToTui,
            })
        ));
    }

    #[test]
    fn tui_consent_plan_applies_only_ticked_workflow_and_mcp_offers() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let fresh = crate::activation::mcp_client::AnvilEntry::local_stdio(PathBuf::from(
            "/usr/local/bin/anvil",
        ));
        let enabled = crate::activation::mcp_client::all_client_ids();
        let plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Install,
            &enabled,
            Some(fresh),
            false,
        );
        assert_eq!(plan.root, dir.path());
        assert_eq!(
            plan.project_actions
                .iter()
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "project:init-config",
                "project:identity",
                "project:witness-attributes",
                "project:baseline",
            ]
        );

        let offer_ids = plan
            .offers()
            .iter()
            .map(|offer| offer.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(offer_ids.contains("workflow:pr-validation"));
        assert!(offer_ids.contains("workflow:audit"));
        assert!(offer_ids.contains("mcp:cursor"));
        assert!(offer_ids.contains("mcp:claude-code"));

        let applied = plan.apply(&["workflow:audit".to_string(), "mcp:cursor".to_string()]);

        assert!(applied.workflow_error.is_none());
        assert!(
            dir.path()
                .join(".github/workflows/anvil-audit.yml")
                .exists()
        );
        assert!(!dir.path().join(".github/workflows/anvil.yml").exists());
        assert!(home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
    }

    #[test]
    fn tui_consent_plan_empty_selection_is_a_no_write_decision() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let fresh = crate::activation::mcp_client::AnvilEntry::local_stdio(PathBuf::from(
            "/usr/local/bin/anvil",
        ));
        let enabled = crate::activation::mcp_client::all_client_ids();
        let plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Install,
            &enabled,
            Some(fresh),
            false,
        );

        let applied = plan.apply(&[]);

        assert!(applied.workflow_error.is_none());
        assert!(!dir.path().join(".github").exists());
        assert!(!home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
    }

    #[test]
    fn tui_consent_plan_offers_and_applies_first_wave_registry_clients() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let mut plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
        );

        plan.add_registry_mcp_offers(true, InstallScope::Global, &[]);

        let offer_ids = plan
            .offers()
            .iter()
            .map(|offer| offer.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(offer_ids.contains("mcp:codex"));
        assert!(offer_ids.contains("mcp:opencode"));
        assert!(!offer_ids.contains("mcp:zed"));

        let applied = plan.apply(&["mcp:codex".to_string()]);

        assert!(applied.first_wave_mcp_errors.is_empty());
        assert_eq!(applied.first_wave_mcp_lines.len(), 1);
        assert!(home.path().join(".codex/config.toml").exists());
        assert!(!home.path().join(".config/opencode/opencode.json").exists());
    }

    #[test]
    fn tui_consent_plan_default_offers_all_registry_clients_without_detection() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let mut plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
        );

        // force_all=true mirrors the TUI default: offer everyone even when
        // no editor is detected on the host.
        plan.add_registry_mcp_offers(true, InstallScope::Global, &[]);
        plan.add_project_only_registry_mcp_offers(&[]);

        let offer_ids = plan
            .offers()
            .iter()
            .map(|offer| offer.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(offer_ids.contains("mcp:codex"));
        assert!(offer_ids.contains("mcp:opencode"));
        assert!(offer_ids.contains("mcp:gemini-cli"));
        assert!(offer_ids.contains("mcp:grok"));
        assert!(
            offer_ids.contains("mcp:vscode"),
            "project-only VS Code should appear"
        );
        assert!(
            offer_ids.contains("mcp:zed"),
            "project-only Zed should appear"
        );
        // Global Claude/Cursor remain on the legacy offer path, not registry.
        assert!(!offer_ids.contains("mcp:claude-code"));
        assert!(!offer_ids.contains("mcp:cursor"));
    }

    #[test]
    fn tui_consent_plan_skips_settled_registry_mcp_offers() {
        // ACTTUI-018: pre-install Codex so dry-run reports !changed; re-plan
        // must not re-offer mcp:codex, and must record a settled row.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        crate::commands::mcp_installer::install(
            AgentClientId::Codex,
            InstallScope::Global,
            home.path(),
            crate::activation::mcp_client::PREFERRED_MCP_COMMAND,
            false,
            false,
        )
        .expect("install codex for settled fixture");

        let mut plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
        );
        plan.add_registry_mcp_offers(true, InstallScope::Global, &[]);

        let offer_ids = plan
            .offers()
            .iter()
            .map(|offer| offer.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(
            !offer_ids.contains("mcp:codex"),
            "settled Codex must not re-appear as consent: {offer_ids:?}"
        );
        assert!(
            plan.settled_mcp()
                .iter()
                .any(|row| row.contains("Codex") && row.contains("already configured")),
            "settled list should mention Codex: {:?}",
            plan.settled_mcp()
        );
        // Other unset clients still offered.
        assert!(offer_ids.contains("mcp:opencode") || offer_ids.contains("mcp:grok"));
    }

    #[test]
    fn tui_consent_plan_honours_explicit_project_scope() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let zed_settings = dir.path().join(".zed").join("settings.json");
        let mut plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
        );

        plan.add_registry_mcp_offers(false, InstallScope::Project, &[AgentClientId::Zed]);

        let offer = plan
            .offers()
            .iter()
            .find(|offer| offer.id == "mcp:zed")
            .expect("explicit project-scoped Zed offer");
        assert!(offer.repo_scoped);
        assert!(
            offer
                .description
                .contains(zed_settings.to_string_lossy().as_ref())
        );

        let applied = plan.apply(&["mcp:zed".to_string()]);

        assert!(applied.first_wave_mcp_errors.is_empty());
        assert!(zed_settings.exists());
        assert!(!home.path().join(".zed").join("settings.json").exists());
    }

    #[test]
    fn tui_project_scope_routes_legacy_clients_to_project_registry_paths() {
        let project = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let fresh = crate::activation::mcp_client::AnvilEntry::local_stdio(PathBuf::from(
            "/usr/local/bin/anvil",
        ));
        let enabled = crate::activation::mcp_client::all_client_ids();
        let mut plan = build_tui_consent_plan_with_home(
            project.path(),
            Some(home.path()),
            McpInstallPolicy::Install,
            &enabled,
            Some(fresh),
            false,
        );

        plan.add_registry_mcp_offers(
            false,
            InstallScope::Project,
            &[AgentClientId::ClaudeCode, AgentClientId::Cursor],
        );

        let claude = plan
            .offers()
            .iter()
            .find(|offer| offer.id == "mcp:claude-code")
            .expect("project-scoped Claude offer");
        let cursor = plan
            .offers()
            .iter()
            .find(|offer| offer.id == "mcp:cursor")
            .expect("project-scoped Cursor offer");
        assert!(
            claude
                .description
                .contains(project.path().to_str().unwrap())
        );
        assert!(
            cursor
                .description
                .contains(project.path().to_str().unwrap())
        );

        let applied = plan.apply(&["mcp:claude-code".to_string(), "mcp:cursor".to_string()]);

        assert!(applied.first_wave_mcp_errors.is_empty());
        assert!(project.path().join(".mcp.json").exists());
        assert!(project.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
        assert!(!home.path().join(".cursor/mcp.json").exists());
    }

    #[test]
    fn tui_project_registry_install_does_not_require_a_home_directory() {
        let project = TempDir::new().unwrap();
        let mut plan = build_tui_consent_plan_with_home(
            project.path(),
            None,
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
        );
        plan.add_registry_mcp_offers(false, InstallScope::Project, &[AgentClientId::Codex]);

        let applied = plan.apply(&["mcp:codex".to_string()]);

        assert!(applied.first_wave_mcp_errors.is_empty());
        assert!(project.path().join(".codex/config.toml").exists());
    }

    #[test]
    fn config_consent_discloses_the_write_set_for_each_init_path() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        let default_plan = build_tui_consent_plan_with_project_options(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
            None,
            false,
        );
        let default_offer = default_plan
            .offers()
            .iter()
            .find(|offer| offer.id == "project:init-config")
            .unwrap();
        assert!(default_offer.description.contains("support files"));

        let formatted_plan = build_tui_consent_plan_with_project_options(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
            Some(anvil_config::ConfigFormat::Json),
            false,
        );
        let formatted_offer = formatted_plan
            .offers()
            .iter()
            .find(|offer| offer.id == "project:init-config")
            .unwrap();
        assert_eq!(
            formatted_offer.description,
            format!("Create {}", dir.path().join(".anvil.json").display())
        );
    }

    #[test]
    fn explicit_claude_consent_writes_only_the_disclosed_mcp_target() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let fresh = crate::activation::mcp_client::AnvilEntry::local_stdio(PathBuf::from(
            "/usr/local/bin/anvil",
        ));
        let enabled = crate::activation::mcp_client::all_client_ids();
        let plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Install,
            &enabled,
            Some(fresh),
            false,
        );

        let offer = plan
            .offers()
            .iter()
            .find(|offer| offer.id == "mcp:claude-code")
            .unwrap();
        assert!(offer.description.contains(".claude.json"));
        assert!(!offer.description.contains("settings.json"));

        plan.apply(&["mcp:claude-code".to_string()]);

        assert!(home.path().join(".claude.json").exists());
        assert!(!home.path().join(".claude/settings.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn selected_git_hook_consent_reports_only_active_managed_hooks_as_applied() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        assert!(
            Command::new("git")
                .arg("init")
                .arg(dir.path())
                .status()
                .unwrap()
                .success()
        );
        let plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
        );

        let applied = plan.apply(&["project:git-hooks".to_string()]);

        assert!(applied.project_applied.contains(&ActivationStep::GitHooks));
        assert!(applied.install_report.hooks_active);

        std::fs::write(
            dir.path().join(".git/hooks/pre-commit"),
            "#!/bin/sh\nexit 0\n",
        )
        .unwrap();
        let plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
        );
        let applied = plan.apply(&["project:git-hooks".to_string()]);
        assert!(!applied.project_applied.contains(&ActivationStep::GitHooks));
        assert!(
            applied
                .project_skipped
                .contains_key(&ActivationStep::GitHooks)
        );
        assert!(!applied.install_report.hooks_active);
    }

    #[test]
    fn identity_rotation_is_bound_to_consent_and_changes_the_existing_id() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let original = identity::ensure_project_id(dir.path(), env!("CARGO_PKG_VERSION")).unwrap();
        let lifecycle = run_with_home_and_registration_outcome(
            dir.path(),
            Some(home.path()),
            &default_global(),
            |_| WorktreeRegistration::DaemonUnavailable,
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            StartRenderMode::Tui,
            true,
            None,
        )
        .unwrap();
        assert!(lifecycle.run.events().iter().any(|event| {
            event.step == ActivationStep::ProjectIdentity
                && event.lifecycle == ActivationStepLifecycle::Deferred
        }));
        let plan = build_tui_consent_plan_with_project_options(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Skip,
            &BTreeSet::new(),
            None,
            false,
            None,
            true,
        );

        let applied = plan.apply(&["project:identity".to_string()]);
        let rotated = identity::read_project_id(dir.path()).unwrap().unwrap();

        assert_ne!(rotated.project_uuid, original.project_uuid);
        assert!(
            applied
                .project_applied
                .contains(&ActivationStep::ProjectIdentity)
        );
    }

    #[test]
    fn tui_mcp_lifecycle_skips_when_no_client_is_offerable() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let outcome = run_with_home_and_registration_outcome(
            dir.path(),
            Some(home.path()),
            &default_global(),
            |_| WorktreeRegistration::DaemonUnavailable,
            McpInstallPolicy::Install,
            &BTreeSet::new(),
            StartRenderMode::Tui,
            false,
            None,
        )
        .unwrap();

        assert!(outcome.run.events().iter().any(|event| {
            event.step == ActivationStep::McpConsent
                && event.lifecycle == ActivationStepLifecycle::Skipped
                && event.detail.as_deref() == Some("no MCP changes available for consent")
        }));
    }

    #[test]
    fn gated_tui_consent_plan_marks_and_rejects_workspace_mcp_writes() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let cursor_config = dir.path().join(".cursor/mcp.json");
        std::fs::create_dir_all(cursor_config.parent().unwrap()).unwrap();
        std::fs::write(&cursor_config, "{\"mcpServers\":{}}\n").unwrap();
        let fresh = crate::activation::mcp_client::AnvilEntry::local_stdio(PathBuf::from(
            "/usr/local/bin/anvil",
        ));
        let enabled = crate::activation::mcp_client::all_client_ids();
        let plan = build_tui_consent_plan_with_home(
            dir.path(),
            Some(home.path()),
            McpInstallPolicy::Install,
            &enabled,
            Some(fresh),
            true,
        );

        let cursor_offer = plan
            .offers()
            .iter()
            .find(|offer| offer.id == "mcp:cursor")
            .unwrap();
        assert!(cursor_offer.repo_scoped);

        plan.apply(&["mcp:cursor".to_string()]);

        assert_eq!(
            std::fs::read_to_string(cursor_config).unwrap(),
            "{\"mcpServers\":{}}\n"
        );
        assert!(!home.path().join(".cursor/mcp.json").exists());
    }

    #[test]
    fn activation_step_failed_lifecycle_is_distinct_from_skip() {
        let mut run = ActivationRun::default();
        run.start(ActivationStep::GitHooks);
        run.fail(ActivationStep::GitHooks, "could not install git hooks");

        assert!(run.events().iter().any(|event| {
            event.step == ActivationStep::GitHooks
                && event.lifecycle == ActivationStepLifecycle::Failed
                && event.render_line().contains("failed")
        }));
    }

    #[test]
    fn activation_run_logs_daemon_attestation_skip_detail() {
        let mut run = ActivationRun::default();
        let mut diagnostic = verify_with_home(Path::new("."), None);
        diagnostic.daemon_attestation =
            crate::activation::daemon_evidence::DaemonAttestation::Unreachable;

        record_daemon_attestation_log(&mut run, &diagnostic);

        assert!(
            run.log_lines()
                .iter()
                .any(|line| line.contains("daemon attestation skipped: daemon IPC")),
            "daemon attestation skip detail should be routed into the activation log buffer: {:?}",
            run.log_lines(),
        );
    }

    #[test]
    fn agent_to_mcp_client_maps_only_clients_with_impls() {
        assert_eq!(
            agent_to_mcp_client(AgentKind::ClaudeCode),
            Some(McpClientId::ClaudeCode)
        );
        assert_eq!(
            agent_to_mcp_client(AgentKind::Cursor),
            Some(McpClientId::Cursor)
        );
        assert_eq!(
            agent_to_mcp_client(AgentKind::Codex),
            Some(McpClientId::Codex)
        );
        // Detected for the "AI tools detected" line, but no MCP adapter.
        assert_eq!(agent_to_mcp_client(AgentKind::Aider), None);
        assert_eq!(agent_to_mcp_client(AgentKind::Windsurf), None);
    }

    #[test]
    fn resolve_enabled_clients_force_all_returns_every_client() {
        // `force_all` short-circuits before any detection or env read.
        let env = StubDetectionEnv {
            binaries: std::collections::HashSet::new(),
        };
        let enabled = resolve_enabled_clients(&env, /* force_all */ true);
        assert_eq!(enabled, crate::activation::mcp_client::all_client_ids());
    }

    #[test]
    fn resolve_enabled_clients_scopes_to_detected_editor() {
        // Hermetic: the stub's `env()` returns `None`, so the
        // `ANVIL_ALL_MCP_CLIENTS` opt-in never fires regardless of the
        // real process environment. Only the editor whose binary is on
        // PATH is enabled.
        let env = StubDetectionEnv::with_binary("claude");
        let enabled = resolve_enabled_clients(&env, /* force_all */ false);
        assert!(enabled.contains(&McpClientId::ClaudeCode));
        assert!(
            !enabled.contains(&McpClientId::Cursor),
            "undetected Cursor must not be enabled"
        );
    }

    #[test]
    fn explicit_legacy_client_is_enabled_without_detection() {
        let mut enabled = BTreeSet::new();

        extend_enabled_with_explicit_clients(&mut enabled, &[AgentClientId::Cursor]);

        assert_eq!(enabled, BTreeSet::from([McpClientId::Cursor]));
    }

    #[test]
    fn skipped_tui_policy_does_not_offer_first_wave_mcp_writes() {
        let dir = TempDir::new().unwrap();
        let plan = build_tui_consent_plan(
            dir.path(),
            McpInstallPolicy::Skip,
            false,
            None,
            false,
            RegistryMcpSelection {
                scope: InstallScope::Global,
                explicit_clients: &[],
            },
        );

        assert!(
            plan.offers()
                .iter()
                .all(|offer| offer.kind != TuiConsentOfferKind::Mcp)
        );
    }

    fn run_in_isolated(
        root: &Path,
        home: &Path,
        global: &GlobalArgs,
    ) -> (ActivationDiagnostic, InstallReport) {
        run_with_home_for_test(root, Some(home), global).expect("orchestrator should succeed")
    }

    fn run_with_home_for_test(
        root: &Path,
        home: Option<&Path>,
        global: &GlobalArgs,
    ) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
        run_with_home_and_registration(
            root,
            home,
            global,
            |_| WorktreeRegistration::DaemonUnavailable,
            McpInstallPolicy::Install,
            &crate::activation::mcp_client::all_client_ids(),
        )
    }

    #[test]
    fn orchestrator_writes_config_when_absent() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Pre-condition: no config.
        let pre = verify_with_home(dir.path(), Some(home.path()));
        assert!(matches!(pre.config, ConfigStatus::Absent));

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);

        assert!(
            dir.path().join(".anvil.yaml").exists(),
            "orchestrator should write the canonical config on a fresh repo"
        );
        assert!(matches!(diag.config, ConfigStatus::Valid));
    }

    #[test]
    fn orchestrator_writes_project_id_when_absent() {
        // A7.2 / MLP-001 — ensure orchestrator establishes project
        // identity on first run. The file is foundation for v1
        // multi-layer protection but does not affect current-release
        // behaviour beyond writing the tracked file.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);

        let project_id_path = dir.path().join("anvil/project-id");
        assert!(
            project_id_path.exists(),
            "orchestrator should write anvil/project-id on a fresh repo"
        );
        let contents = std::fs::read_to_string(&project_id_path).unwrap();
        assert!(contents.contains("project_uuid:"));
    }

    #[test]
    fn orchestrator_project_id_is_idempotent() {
        // A7.2 / MLP-001 — re-running anvil start must not mint a new
        // UUID; the existing project-id is the stable identity.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);
        let first = std::fs::read_to_string(dir.path().join("anvil/project-id")).unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let second = std::fs::read_to_string(dir.path().join("anvil/project-id")).unwrap();

        assert_eq!(
            first, second,
            "orchestrator must not rewrite anvil/project-id on re-run"
        );
    }

    #[test]
    fn orchestrator_writes_witness_gitattributes() {
        // Council C-7 — `.gitattributes` must include the witness file
        // merge=union lines so MLP-002 can ship without forcing a
        // separate migration.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);

        let attrs = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert!(
            attrs.contains("anvil/witness/active.ndjson merge=union -text"),
            ".gitattributes must include witness file merge=union line. got:\n{attrs}"
        );
        assert!(
            attrs.contains("anvil/witness/manifest/chain.ndjson merge=union -text"),
            ".gitattributes must include manifest merge=union line. got:\n{attrs}"
        );
    }

    #[test]
    fn orchestrator_gitattributes_is_idempotent() {
        // Re-running `anvil start` must not duplicate lines in
        // .gitattributes.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);
        let first = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let second = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();

        assert_eq!(
            first, second,
            "orchestrator must not duplicate .gitattributes lines on re-run"
        );
    }

    #[test]
    fn orchestrator_gitattributes_preserves_user_lines() {
        // Pre-existing `.gitattributes` content must survive; we
        // append, never overwrite.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::write(dir.path().join(".gitattributes"), "*.txt text\n").unwrap();

        run_in_isolated(dir.path(), home.path(), &global);

        let attrs = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert!(
            attrs.starts_with("*.txt text\n"),
            "user's existing .gitattributes lines must be preserved"
        );
        assert!(attrs.contains("anvil/witness/active.ndjson merge=union -text"));
    }

    #[test]
    fn orchestrator_installs_managed_git_hooks_when_repo_present() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        init_git_repo(dir.path());

        run_in_isolated(dir.path(), home.path(), &global);

        for hook in ["pre-commit", "pre-push"] {
            let path = dir.path().join(".git/hooks").join(hook);
            let raw = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read installed hook {}: {e}", path.display()));
            assert!(
                raw.contains("# @anvil-managed"),
                "{hook} must be installed as an anvil-managed hook; got:\n{raw}",
            );
        }
    }

    /// MLP2-038 — end-to-end proof that the `merge=union -text` line the
    /// orchestrator writes actually causes git to union-merge witness file
    /// appends from parallel branches without producing conflict markers.
    /// The existing tests at this site cover the **file content** the
    /// orchestrator writes; this one drives a real `git merge` to confirm
    /// the validation requirement in `plans/modules/multilayer-protection-v2.aps.md`
    /// (Group H, MLP2-038) holds end-to-end.
    #[test]
    fn orchestrator_gitattributes_unions_parallel_witness_appends() {
        use std::process::Command;

        // Skip when the test runner has no `git` on PATH; the rest of the
        // workspace requires git for normal operation so a missing binary
        // means the host is mis-configured rather than a CI signal we want
        // to fail on.
        let git_probe = Command::new("git").arg("--version").output();
        if !matches!(&git_probe, Ok(out) if out.status.success()) {
            eprintln!("skipping MLP2-038 union-merge test: `git --version` failed ({git_probe:?})");
            return;
        }

        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Run the same `.gitattributes` writer the orchestrator runs.
        // Going through the full `run_with_home` would also write
        // `.anvil.yaml`, `anvil/project-id`, `.anvil/baseline.json`, etc.,
        // which we'd then have to stage; the union-merge property is a
        // property of the `.gitattributes` content only, so we call the
        // narrow writer directly.
        ensure_witness_gitattributes(root).expect("write .gitattributes");

        // Bring up a minimal commit-capable git repo. The committer
        // identity is local-only so the test can't accidentally pick up
        // the dev's real `user.name` / `user.email`.
        let run_git = |args: &[&str]| -> std::process::Output {
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap_or_else(|e| panic!("git {args:?} failed to spawn: {e}"))
        };
        let must = |args: &[&str]| {
            let out = run_git(args);
            assert!(
                out.status.success(),
                "git {args:?} failed: stdout={:?} stderr={:?}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        };

        // `-b main` keeps the default-branch name deterministic across
        // host git defaults (`master` on older git, `main` on newer).
        must(&["init", "-q", "-b", "main"]);
        must(&["config", "user.email", "mlp2-038@anvil.test"]);
        must(&["config", "user.name", "MLP2-038 fixture"]);
        // Disable signing so the test passes on hosts with commit.gpgsign=true.
        must(&["config", "commit.gpgsign", "false"]);

        // Stage the .gitattributes plus an empty witness file as the
        // shared ancestor commit.
        let witness_rel = "anvil/witness/active.ndjson";
        let witness_path = root.join(witness_rel);
        std::fs::create_dir_all(witness_path.parent().unwrap()).unwrap();
        std::fs::write(&witness_path, "").unwrap();
        must(&["add", ".gitattributes", witness_rel]);
        must(&["commit", "-q", "-m", "base"]);

        // Branch A: append a row attributed to attribution "a".
        must(&["checkout", "-q", "-b", "branch-a"]);
        append_line(&witness_path, "{\"who\":\"a\",\"n\":1}\n");
        must(&["commit", "-q", "-am", "branch-a row"]);

        // Branch B (off main, not off A): append a different row.
        must(&["checkout", "-q", "main"]);
        must(&["checkout", "-q", "-b", "branch-b"]);
        append_line(&witness_path, "{\"who\":\"b\",\"n\":2}\n");
        must(&["commit", "-q", "-am", "branch-b row"]);

        // Merge A then B back into main. Each merge exercises the
        // `merge=union -text` attribute on a real divergent append.
        must(&["checkout", "-q", "main"]);
        must(&["merge", "-q", "--no-edit", "branch-a"]);
        let merge_out = run_git(&["merge", "--no-edit", "branch-b"]);
        assert!(
            merge_out.status.success(),
            "merge of branch-b into main must succeed under `merge=union -text`. \
             stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&merge_out.stdout),
            String::from_utf8_lossy(&merge_out.stderr)
        );

        let merged = std::fs::read_to_string(&witness_path).unwrap();
        assert!(
            !merged.contains("<<<<<<<")
                && !merged.contains("=======")
                && !merged.contains(">>>>>>>"),
            "merged witness file must not contain conflict markers:\n{merged}"
        );
        assert!(
            merged.contains("{\"who\":\"a\",\"n\":1}"),
            "merged file must retain branch-a row:\n{merged}"
        );
        assert!(
            merged.contains("{\"who\":\"b\",\"n\":2}"),
            "merged file must retain branch-b row:\n{merged}"
        );
    }

    fn append_line(path: &Path, line: &str) {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap_or_else(|e| panic!("open {} for append: {e}", path.display()));
        f.write_all(line.as_bytes())
            .unwrap_or_else(|e| panic!("append to {}: {e}", path.display()));
    }

    fn init_git_repo(root: &Path) {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q", "-b", "main"])
            .output()
            .unwrap_or_else(|e| panic!("git init failed to spawn: {e}"));
        assert!(
            out.status.success(),
            "git init failed: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }

    #[test]
    fn orchestrator_continues_when_project_id_write_fails() {
        // A7.2 — failures to establish project-id MUST NOT propagate.
        // Simulate by pre-creating `anvil/project-id` as a directory,
        // which makes both write-as-file and parse impossible. The
        // orchestrator should log a warning and finish successfully so
        // the user still gets MCP install + diagnostic.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::create_dir_all(dir.path().join("anvil/project-id")).unwrap();

        let result = run_with_home_for_test(dir.path(), Some(home.path()), &global);
        assert!(
            result.is_ok(),
            "orchestrator must not fail when anvil/project-id is unwritable: {result:?}"
        );
    }

    #[test]
    fn orchestrator_attempts_daemon_worktree_registration() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();
        let called = std::cell::Cell::new(false);

        // ACTMO-016: registration is gated on a registerable Git worktree, so
        // the closure only fires when cwd is one. Make the dir a real worktree.
        git_init(dir.path());

        run_with_home_and_registration(
            dir.path(),
            Some(home.path()),
            &global,
            |_root| {
                called.set(true);
                WorktreeRegistration::Registered
            },
            McpInstallPolicy::Install,
            &crate::activation::mcp_client::all_client_ids(),
        )
        .expect("orchestrator should continue after registration");

        assert!(
            called.get(),
            "orchestrator must register the activation worktree"
        );
    }

    /// ACTMO-016: outside a registerable worktree, the orchestrator does not
    /// invoke the registration closure (no junk session keyed to e.g. $HOME),
    /// yet still completes successfully.
    #[test]
    fn orchestrator_skips_registration_outside_a_worktree() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();
        let called = std::cell::Cell::new(false);

        run_with_home_and_registration(
            dir.path(),
            Some(home.path()),
            &global,
            |_root| {
                called.set(true);
                WorktreeRegistration::Registered
            },
            McpInstallPolicy::Install,
            &crate::activation::mcp_client::all_client_ids(),
        )
        .expect("orchestrator should continue without registering");

        assert!(!called.get(), "a non-worktree dir must not be registered");
    }

    /// Initialise a minimal Git worktree so the registerable-worktree gate
    /// (ACTMO-016) treats the directory as registerable.
    fn git_init(dir: &Path) {
        for args in [
            ["init", "-q"].as_slice(),
            ["config", "user.email", "t@t"].as_slice(),
            ["config", "user.name", "t"].as_slice(),
        ] {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .output()
                .expect("run git")
                .status
                .success();
            assert!(ok, "git {args:?} failed");
        }
    }

    #[test]
    fn orchestrator_skips_init_when_config_valid() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Run once to write config + install.
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_before = std::fs::metadata(dir.path().join(".anvil.yaml"))
            .unwrap()
            .modified()
            .unwrap();

        // Idempotency check: file mtime must not change on a re-run.
        // Sleep a beat to make any rewrite detectable across filesystems
        // with one-second mtime granularity (e.g. HFS+).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_after = std::fs::metadata(dir.path().join(".anvil.yaml"))
            .unwrap()
            .modified()
            .unwrap();

        assert_eq!(
            mtime_before, mtime_after,
            "orchestrator must not rewrite the config on idempotent re-run"
        );
    }

    #[test]
    fn orchestrator_auto_installs_in_no_tui_mode() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (_diag, report) = run_in_isolated(dir.path(), home.path(), &global);

        // Both clients should have been auto-installed at home scope.
        assert!(
            matches!(
                report.per_client.get(&McpClientId::Cursor),
                Some(InstallOutcome::Installed { .. })
            ),
            "Cursor must auto-install in --no-tui mode"
        );
        assert!(
            matches!(
                report.per_client.get(&McpClientId::ClaudeCode),
                Some(InstallOutcome::Installed { .. })
            ),
            "Claude Code must auto-install in --no-tui mode"
        );
        assert!(home.path().join(".cursor/mcp.json").exists());
        assert!(home.path().join(".claude.json").exists());
        let cursor_raw = std::fs::read_to_string(home.path().join(".cursor/mcp.json")).unwrap();
        let cursor: serde_json::Value = serde_json::from_str(&cursor_raw).unwrap();
        assert_eq!(
            cursor["mcpServers"]["anvil"]["command"], "anvil",
            "default managed install must write PATH-stable anvil, not current_exe"
        );
        assert!(
            !cursor_raw.contains("Cellar"),
            "default managed install must not pin a Cellar path: {cursor_raw}"
        );
    }

    #[test]
    fn orchestrator_skips_mcp_install_when_policy_skip() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (_diag, report) = run_with_home_and_registration(
            dir.path(),
            Some(home.path()),
            &global,
            |_| WorktreeRegistration::DaemonUnavailable,
            McpInstallPolicy::Skip,
            &crate::activation::mcp_client::all_client_ids(),
        )
        .expect("orchestrator should succeed with MCP install skipped");

        assert!(
            report.per_client.is_empty(),
            "skip policy must not report per-client MCP writes"
        );
        assert!(
            !home.path().join(".cursor/mcp.json").exists(),
            "skip policy must not write Cursor MCP config"
        );
        assert!(
            !home.path().join(".claude.json").exists(),
            "skip policy must not write Claude Code MCP config"
        );
    }

    #[test]
    fn orchestrator_diagnostic_reflects_post_install_state() {
        // After install, the diagnostic re-probe must show the
        // RestartRequired tier — that's the whole point of the install
        // step. Without it, `anvil start` would land on NeedsAction
        // even though we just wired both clients.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);
        let cursor_tier = diag.mcp.get(&McpClientId::Cursor).map(|r| r.tier);
        let claude_tier = diag.mcp.get(&McpClientId::ClaudeCode).map(|r| r.tier);

        assert!(
            matches!(
                cursor_tier,
                Some(
                    crate::activation::diagnostic::McpTier::RestartRequired
                        | crate::activation::diagnostic::McpTier::RestartHandshakeVerified
                )
            ),
            "Cursor tier should advance to RestartRequired after install, got {cursor_tier:?}"
        );
        assert!(
            matches!(
                claude_tier,
                Some(
                    crate::activation::diagnostic::McpTier::RestartRequired
                        | crate::activation::diagnostic::McpTier::RestartHandshakeVerified
                )
            ),
            "Claude Code tier should advance to RestartRequired after install, got {claude_tier:?}"
        );
    }

    #[test]
    fn orchestrator_returns_ready_restart_required_after_install() {
        // The composed flow's headline outcome: a fresh repo with no
        // editor wired ends at `ReadyRestartRequired` once both clients
        // have an entry on disk. (Was `NeedsAction` before LAUNCH-009
        // part 2 — that test moved into `*_skips_install_when_*` cases.)
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);
        let state = diag.protection_state();
        assert!(
            matches!(
                state,
                crate::activation::state::ProtectionState::ReadyRestartRequired
                    | crate::activation::state::ProtectionState::Unsupported
            ),
            "post-install fresh repo should land at ready_restart_required \
             (or unsupported when no covered languages), got {state:?}"
        );
    }

    #[test]
    fn orchestrator_writes_baseline_when_absent() {
        // LAUNCH-010: a fresh repo with at least one analysable file
        // must end with `.anvil/baseline.json` on disk, populated
        // with whatever findings the activation scan saw.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Plant a `.ts` file so the antipattern scanner has something
        // to scan; even if it produces zero findings, the baseline
        // writer still runs and writes an empty fingerprint set.
        std::fs::write(
            dir.path().join("hello.ts"),
            "export const greet = () => console.log('hi');\n",
        )
        .unwrap();

        let baseline_path = crate::activation::baseline::baseline_path(dir.path());
        assert!(
            !baseline_path.exists(),
            "precondition: baseline must be absent on a fresh repo"
        );

        let (diag, _) = run_in_isolated(dir.path(), home.path(), &global);

        assert!(
            baseline_path.exists(),
            "orchestrator must write baseline.json on first activation"
        );
        assert!(
            diag.baseline_present,
            "diagnostic must report baseline_present after first activation"
        );
        assert!(
            diag.baseline_summary.is_some(),
            "diagnostic must carry a parsed baseline summary"
        );
    }

    #[test]
    fn orchestrator_baseline_write_is_idempotent() {
        // LAUNCH-010: re-running activation must NOT rewrite an
        // existing baseline. The activation snapshot is captured once
        // — refreshing requires the user to delete the file and re-
        // run start.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::write(
            dir.path().join("hello.ts"),
            "export const greet = () => console.log('hi');\n",
        )
        .unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let baseline_path = crate::activation::baseline::baseline_path(dir.path());
        let mtime_before = std::fs::metadata(&baseline_path)
            .unwrap()
            .modified()
            .unwrap();

        // Sleep a beat so any rewrite would be detectable on filesystems
        // with one-second mtime granularity (mirrors the existing
        // `orchestrator_skips_init_when_config_valid` pattern).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_after = std::fs::metadata(&baseline_path)
            .unwrap()
            .modified()
            .unwrap();

        assert_eq!(
            mtime_before, mtime_after,
            "orchestrator must not rewrite baseline.json on re-run"
        );
    }

    #[test]
    fn orchestrator_records_findings_in_baseline() {
        // LAUNCH-010 spec: a fixture repo with a finding-shaped line
        // must produce a baseline whose total > 0.
        //
        // PR #1293 review fix (Copilot): the test relies on
        // antipattern findings, not secret findings. `@ts-ignore` is
        // AP-004 in the compiled registry and `: any` is AP-003 —
        // both are TS-shape and predate recent registry churn. The
        // earlier comment incorrectly named an "AWS access key"
        // approach; the actual fixture deliberately avoids the
        // secret-scanner allowlist (which captures `EXAMPLE`
        // patterns) so the assertion stays deterministic regardless
        // of allowlist evolution.
        use crate::activation::baseline;

        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::write(
            dir.path().join("leak.ts"),
            "// @ts-ignore\nconst x: any = 5;\n",
        )
        .unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let b = baseline::read_baseline(dir.path())
            .expect("baseline read must succeed")
            .expect("baseline must be present");
        assert!(
            b.total() > 0,
            "baseline must contain at least one fingerprint, got: {b:?}"
        );
    }

    #[test]
    fn orchestrator_does_not_overwrite_unsafe_drift() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Pre-populate Cursor with a foreign-tool entry that uses our
        // server name.
        std::fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/bin/bash", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        std::fs::write(home.path().join(".cursor/mcp.json"), cfg).unwrap();
        let bytes_before = std::fs::read(home.path().join(".cursor/mcp.json")).unwrap();

        let (_diag, report) = run_in_isolated(dir.path(), home.path(), &global);

        match report.per_client.get(&McpClientId::Cursor) {
            Some(InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(_),
            }) => {}
            other => panic!("expected Cursor UnsafeDrift skip, got {other:?}"),
        }

        let bytes_after = std::fs::read(home.path().join(".cursor/mcp.json")).unwrap();
        assert_eq!(bytes_before, bytes_after, "must not overwrite UnsafeDrift");
    }

    // ---- MLP2-053: audit-chain workflow installation -------------------

    #[test]
    fn orchestrator_does_not_write_github_actions_without_interactive_consent() {
        // MLP2-043 / MLP2-053 — GitHub Actions workflows change repo
        // behaviour and consume customer CI minutes, so non-interactive
        // activation must never add them silently.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);

        let action_target = dir.path().join(".github/workflows/anvil.yml");
        let audit_target = dir.path().join(".github/workflows/anvil-audit.yml");
        assert!(
            !action_target.exists(),
            "orchestrator must not write .github/workflows/anvil.yml without consent"
        );
        assert!(
            !audit_target.exists(),
            "orchestrator must not write .github/workflows/anvil-audit.yml without consent"
        );
    }

    #[test]
    fn workflow_install_is_idempotent() {
        // MLP2-053 — re-running activation must not rewrite an existing
        // `.github/workflows/anvil-audit.yml`. Operators are expected to
        // edit the file in-place (e.g. comment out the `schedule` block);
        // we must never clobber that. Asserting content equality (not
        // mtime) lets the test run in microseconds — the sibling
        // `orchestrator_audit_workflow_preserves_user_edits` test
        // proves the same property for a user-edited file; this one
        // pins it for the orchestrator's own template.
        let dir = TempDir::new().unwrap();

        install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit]).unwrap();
        let target = dir.path().join(".github/workflows/anvil-audit.yml");
        let before = std::fs::read_to_string(&target).unwrap();

        install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit]).unwrap();
        let after = std::fs::read_to_string(&target).unwrap();

        assert_eq!(
            before, after,
            "orchestrator must not rewrite anvil-audit.yml on re-run"
        );
    }

    #[test]
    fn workflow_install_writes_selected_templates() {
        let dir = TempDir::new().unwrap();

        let written = install_selected_workflows(
            dir.path(),
            &[WorkflowTemplate::PrValidation, WorkflowTemplate::Audit],
        )
        .unwrap();

        let action_target = dir.path().join(".github/workflows/anvil.yml");
        let audit_target = dir.path().join(".github/workflows/anvil-audit.yml");
        assert_eq!(written, vec![action_target.clone(), audit_target.clone()]);
        assert_eq!(
            std::fs::read_to_string(&action_target).unwrap(),
            crate::commands::anvil_action::anvil_workflow_template(),
        );
        assert_eq!(
            std::fs::read_to_string(&audit_target).unwrap(),
            crate::commands::audit_chain::audit_workflow_template(),
        );
    }

    #[test]
    fn workflow_picker_options_default_every_candidate_unticked() {
        // CIB-165 — both picker options must start unselected, so a hurried
        // Enter-through selects nothing and writes no CI files to a shared
        // repo. Ticking a workflow is the explicit consent.
        let dir = TempDir::new().unwrap();
        let candidates = [WorkflowTemplate::PrValidation, WorkflowTemplate::Audit];

        let options = workflow_picker_options(dir.path(), &candidates);

        // The returned options must correspond 1:1 to the input candidates, in
        // order — otherwise a helper that duplicated or dropped a workflow could
        // still pass the unticked check below.
        assert_eq!(options.len(), candidates.len());
        for ((workflow, _label, selected), expected) in options.iter().zip(candidates.iter()) {
            assert_eq!(
                workflow, expected,
                "picker options must match the input candidates 1:1 and in order",
            );
            assert!(
                !selected,
                "picker option for {workflow} must default to unticked (CIB-165)",
            );
        }
    }

    #[test]
    fn workflow_install_with_empty_selection_writes_nothing() {
        // CIB-165 — an Enter-through leaves the selection empty; installing an
        // empty selection must not create `.github/` or any workflow file.
        let dir = TempDir::new().unwrap();

        let written = install_selected_workflows(dir.path(), &[]).unwrap();

        assert!(
            written.is_empty(),
            "empty selection must write no workflow files"
        );
        assert!(
            !dir.path().join(".github").exists(),
            "empty selection must not create the .github directory"
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_install_refuses_symlinked_workflows_dir() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".github")).unwrap();
        symlink(outside.path(), dir.path().join(".github/workflows")).unwrap();

        let err = install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit])
            .expect_err("workflow install must reject symlinked workflow directory");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            !outside.path().join("anvil-audit.yml").exists(),
            "must not write outside repo through a symlinked workflow directory",
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_install_refuses_symlinked_target() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let workflows = dir.path().join(".github/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        symlink(
            outside.path().join("anvil-audit.yml"),
            workflows.join("anvil-audit.yml"),
        )
        .unwrap();

        let err = install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit])
            .expect_err("workflow install must reject symlinked workflow target");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            !outside.path().join("anvil-audit.yml").exists(),
            "must not write outside repo through a symlinked workflow target",
        );
    }

    #[test]
    fn orchestrator_audit_workflow_preserves_user_edits() {
        // MLP2-053 — operators routinely customise the workflow (e.g.
        // bump the schedule, swap the install step). Re-running `anvil
        // start` must leave their edits intact.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let workflows = dir.path().join(".github/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        let target = workflows.join("anvil-audit.yml");
        let user_content = "# customised by operator\nname: anvil-audit-custom\n";
        std::fs::write(&target, user_content).unwrap();

        run_in_isolated(dir.path(), home.path(), &global);

        let after = std::fs::read_to_string(&target).unwrap();
        assert_eq!(
            after, user_content,
            "orchestrator must not overwrite user-edited anvil-audit.yml"
        );
    }
}
