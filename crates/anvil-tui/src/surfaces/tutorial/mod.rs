pub mod discovery;
mod discovery_render;
pub(crate) mod executor;

/// The in-process autoplay check runner supplied by `anvil-cli` (CIB-248), its
/// diagnostic thread name, and the explicit caught-panic containment signal.
pub use executor::{
    AUTOPLAY_WORKER_THREAD, AutoplayRunner, catch_autoplay_panic, is_autoplay_panic_contained,
};

/// CLI-injected probe: whether a tutorial command string would hit the
/// licence gate if executed as a child `anvil` process (CIB-349).
pub type LicenceGateProbe = std::sync::Arc<dyn Fn(&str) -> bool + Send + Sync + 'static>;
pub mod first_win;
mod first_win_render;
pub mod fix;
mod fix_render;
pub mod paths;
pub mod render;
pub mod showcase;
pub mod verify;
pub mod watch_demo;
pub mod watch_demo_render;

use anvil_kernel_types::{Notification, NotificationClass, NotificationPriority};
use discovery::{FindingSeverity, ScanResults};
use eddacraft_tui::keyboard::Action;
use verify::{Verify, VerifyResult};

use crate::surfaces::fix_request::FixRequest;
use crate::surfaces::notifications::{NotificationSource, surface_notification};

/// Notice rendered when the file watcher can't be started and the tutorial
/// falls back to static mode. Shared between `anvil tutorial` and
/// `anvil welcome` so both entry points surface the same cause.
pub const STATIC_MODE_WATCHER_UNAVAILABLE: &str =
    "Live file watcher unavailable \u{2014} file saves won't retrigger checks.";

/// Title of the tutorial path picker, and the single source of truth for how
/// that surface is named. CIB-246: the return-visit welcome hub used to coin
/// its own name for the tutorial ("Learn the anvil model"), so the same
/// mental object carried two names across first run and return visit. The hub
/// entry is now named after this constant and a test in the welcome surface
/// pins the two together.
pub const PATH_PICKER_TITLE: &str = "Choose a Learning Path";

pub const AUTOPLAY_DEMO_LABEL: &str = "Watch anvil work (demo)";
const AUTOPLAY_DEMO_DESCRIPTION: &str =
    "A hands-free sandbox demonstration of anvil's protection loop";

/// Available tutorial paths.
///
/// LAUNCH-014 introduced [`TutorialPath::ProtectionLoop`] as the
/// default first path: a short repo-local value walk that explains
/// anvil's protection loop, simulates a high-signal check on safe
/// fixture content, and points the user at `anvil start --verify` as
/// the next step. The remaining four paths are the deeper-learning
/// track for users who want the full taxonomy walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialPath {
    /// LAUNCH-014: the value-first default. Demonstrates the loop in
    /// 60 seconds without claiming pre-write protection.
    ProtectionLoop,
    /// The AI-assisted development loop: wiring an MCP-capable editor,
    /// pre-write validation for the agent, the fast save-time loop, and the
    /// graph context anvil exposes to the agent.
    DeveloperAcceleration,
    Policy,
    Architecture,
    Drift,
    CI,
}

impl TutorialPath {
    pub fn label(self) -> &'static str {
        match self {
            Self::ProtectionLoop => "anvil's protection loop",
            Self::DeveloperAcceleration => "Developer acceleration",
            Self::Policy => "Policy checks",
            Self::Architecture => "Boundary findings",
            Self::Drift => "Configuration drift",
            Self::CI => "CI gate integration",
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        // Legacy labels ("Policy", "Architecture", "Drift", "CI Integration")
        // are kept so progress files written by older builds still round-trip
        // into the correct enum variant after the onboarding rename.
        // LAUNCH-014's "anvil's protection loop" is new — no legacy
        // alias is required.
        match s {
            "anvil's protection loop" => Some(Self::ProtectionLoop),
            "Developer acceleration" => Some(Self::DeveloperAcceleration),
            "Policy checks" | "Policy" => Some(Self::Policy),
            "Boundary findings" | "Architecture" => Some(Self::Architecture),
            "Configuration drift" | "Drift" => Some(Self::Drift),
            "CI gate integration" | "CI Integration" => Some(Self::CI),
            _ => None,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::ProtectionLoop => {
                "60-second walk: see what anvil checks, then verify protection in this repo"
            }
            Self::DeveloperAcceleration => {
                "Wire your AI coding agent, validate its edits, and feed it graph context"
            }
            Self::Policy => "Define checks that produce findings and influence the gate",
            Self::Architecture => "See how boundary checks turn imports into actionable findings",
            Self::Drift => "Capture state changes and review the findings between snapshots",
            Self::CI => "Carry checks, findings, and gate outcomes into your delivery workflow",
        }
    }
}

/// Current phase of the tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialPhase {
    PathSelect,
    Running,
    Complete,
}

/// WOW-001: declared execution effect of a command step, authored alongside
/// the command in `paths.rs`. Drives the runs-in-your-repo / read-only badge
/// and the step-aware footer help so the user knows **before** pressing Enter
/// whether the step runs a real command and whether it mutates their repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEffect {
    /// The command only inspects state — it never writes to the repo.
    ReadOnly,
    /// The command creates or modifies files in the user's repo.
    MutatesRepo,
}

/// WOW-002: characters of the command revealed per TUI tick. The tutorial
/// loop ticks on its fixed 100ms poll timeout, so pacing is deterministic —
/// N ticks always show the same prefix — and snapshot-testable.
pub const REVEAL_CHARS_PER_TICK: usize = 3;

/// Canonicalise tutorial paths without retaining Windows verbatim prefixes.
///
/// Watch events and CLI workspace roots use ordinary path forms, so tutorial
/// containment and equality checks must use the same representation.
pub(crate) fn canonicalize_working_path(
    path: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    dunce::canonicalize(path)
}

/// Resolve a tutorial-owned target beneath a canonical session root.
pub fn resolve_working_path(
    root: &std::path::Path,
    target: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    let root = canonicalize_working_path(root)?;
    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "tutorial working root is not a directory",
        ));
    }
    if target.is_absolute()
        || target.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "tutorial target resolves outside working root",
        ));
    }
    let candidate = root.join(target);
    let mut probe = root.clone();
    for component in target.components() {
        if let std::path::Component::Normal(component) = component {
            probe.push(component);
            if let Ok(metadata) = std::fs::symlink_metadata(&probe)
                && metadata.file_type().is_symlink()
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "tutorial target contains a symlink",
                ));
            }
        }
    }
    let mut existing = candidate.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "tutorial target resolves outside working root",
            )
        })?;
    }
    let canonical_existing = canonicalize_working_path(existing)?;
    if !canonical_existing.starts_with(&root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "tutorial target resolves outside working root",
        ));
    }
    let suffix = candidate.strip_prefix(existing).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "tutorial target resolves outside working root",
        )
    })?;
    if suffix.as_os_str().is_empty() {
        Ok(canonical_existing)
    } else {
        Ok(canonical_existing.join(suffix))
    }
}

/// WOW-002: an in-flight typed-command reveal on the current step. On Enter
/// the command is "typed" into the step's prompt line at a fixed interval
/// before it executes, so running-for-real is unmistakable at the moment it
/// happens. Any keypress fast-forwards to the full command and executes it
/// immediately. Driven by [`TutorialState::reveal_tick`] from the TUI loop's
/// existing tick — no threads, no wall-clock in rendered content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReveal {
    command: String,
    /// Number of characters currently visible.
    shown: usize,
}

impl CommandReveal {
    fn new(command: String) -> Self {
        Self { command, shown: 0 }
    }

    /// The currently revealed prefix of the command.
    pub fn visible(&self) -> &str {
        match self.command.char_indices().nth(self.shown) {
            Some((idx, _)) => &self.command[..idx],
            None => &self.command,
        }
    }

    fn is_complete(&self) -> bool {
        self.shown >= self.command.chars().count()
    }
}

/// WOW-004: before/after findings count for the chosen domain, shown on the
/// completion screen. `before` is the session's opening scan; `after` comes
/// from a read-only re-scan run when the tutorial completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindingsDelta {
    pub before: usize,
    pub after: usize,
    /// CIB-247: how many of `after` sit under test or fixture paths. Reported
    /// beside the count so a repo whose secret rules fire on deliberate test
    /// data is not read as a repo with that many leaks.
    pub after_in_test_paths: usize,
}

/// Output captured after running a step's command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub exit_code: Option<i32>,
}

/// A single step in a tutorial path.
#[derive(Debug, Clone, Default)]
pub struct TutorialStep {
    pub title: String,
    pub description: String,
    pub instruction: String,
    /// Optional shell command to execute when the user presses Enter.
    pub command: Option<String>,
    /// WOW-001: declared effect of `command`. `Some` on every command step
    /// (pinned by a test in `paths.rs`); `None` on informational steps.
    pub effect: Option<CommandEffect>,
    pub completed: bool,
    /// Captured output from the last execution of `command`.
    pub output: Option<CommandOutput>,
    /// Optional verification check to run after command execution.
    pub verify: Option<Verify>,
    /// Result of the last verification check.
    pub verify_result: Option<VerifyResult>,
    /// Contextual hint shown when verification fails.
    pub verify_hint: Option<String>,
    /// Optional filesystem path to watch for changes. When set and a
    /// file watcher is available, changes to this path (or files within
    /// it) trigger automatic re-verification without pressing Enter.
    pub watch_path: Option<String>,
    /// When true, pressing Enter on this step triggers the watch mode
    /// demo instead of normal advancement. The TUI loop exits and the
    /// CLI command launches the demo surface.
    pub watch_demo: bool,
    /// Optional file the step's inline editor creates or edits. When set,
    /// pressing `e` opens an in-TUI editor seeded with the file's current
    /// contents (or [`seed_template`](Self::seed_template) when it does not
    /// exist yet); saving writes the file and runs the step's `verify`. This
    /// keeps the user inside the tutorial instead of dropping to an external
    /// editor in a second terminal.
    pub edit_target: Option<String>,
    /// Starting content for the inline editor when `edit_target` does not yet
    /// exist on disk. Ignored once the file exists (existing content wins so a
    /// re-entered step never clobbers the user's work).
    pub seed_template: Option<String>,
    /// CIB-349: this step named a licence-gated command while the session
    /// is unsigned-in. Enter must not run the command; the copy names
    /// `anvil auth login` first.
    pub sign_in_bridge: bool,
}

struct AutoplaySavedContext {
    working_root: Option<std::path::PathBuf>,
    scan_results: Option<ScanResults>,
    domain_findings: Option<ScanResults>,
    completion_rescan: Option<Box<dyn Fn() -> Option<ScanResults>>>,
    completion_baseline: Option<usize>,
    completion_delta: Option<FindingsDelta>,
}

/// State for the tutorial orchestrator surface.
#[allow(clippy::struct_excessive_bools)]
pub struct TutorialState {
    pub phase: TutorialPhase,
    pub paths: Vec<TutorialPath>,
    pub path_selected: usize,
    /// Session-scoped autoplay mode. This is deliberately separate from
    /// `TutorialPath`: the demo must never masquerade as persisted path
    /// completion.
    pub autoplay: bool,
    autoplay_session: bool,
    pub wants_autoplay_setup: bool,
    working_root: Option<std::path::PathBuf>,
    autoplay_ghost_offset: usize,
    autoplay_result_dwell: bool,
    autoplay_watch_dwell: bool,
    autoplay_command: Option<executor::AutoplayCommand>,
    autoplay_command_advance: bool,
    /// Supplied by `anvil-cli` (CIB-248). Runs the demo check in-process so
    /// autoplay never re-enters the licence-gated `anvil check` CLI.
    autoplay_runner: Option<executor::AutoplayRunner>,
    /// CIB-349: when true, gated tutorial commands become a sign-in
    /// bridge instead of a runnable check. Default false so isolated TUI
    /// tests keep their existing command-step behaviour.
    requires_sign_in: bool,
    /// CLI-injected probe keyed to `CLI_GATED_COMMANDS`. Absent, the
    /// executor fallback covers the CIB-349 class.
    licence_gated_command: Option<LicenceGateProbe>,
    autoplay_failure: Option<String>,
    autoplay_teardown_requested: bool,
    autoplay_saved_context: Option<AutoplaySavedContext>,
    pub chosen_path: Option<TutorialPath>,
    pub steps: Vec<TutorialStep>,
    pub current_step: usize,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Scan results from the discovery phase, threaded through to tutorials.
    pub scan_results: Option<ScanResults>,
    /// Findings filtered by the chosen tutorial domain.
    pub domain_findings: Option<ScanResults>,
    /// When true, command execution is disabled and all steps become
    /// informational (press-enter-to-continue). Set by the caller when the
    /// kernel watcher is unavailable.
    pub static_mode: bool,
    /// Notice displayed when static mode is active, explaining why interactive
    /// features are disabled.
    pub static_notice: Option<String>,
    /// Paths the user has previously completed (persisted across sessions).
    /// Used by the renderer to show checkmarks in the path selector.
    pub completed_paths: Vec<TutorialPath>,
    /// Transient notice shown when resuming an interrupted session.
    pub resuming_notice: Option<String>,
    /// Set to true when the tutorial wants to launch the watch mode demo.
    /// The TUI loop exits and the CLI command handles the transition.
    pub wants_watch_demo: bool,
    /// WOW-002: in-flight typed-command reveal on the current step. `Some`
    /// between Enter on a command step and the command's execution.
    pub reveal: Option<CommandReveal>,
    /// WOW-004: read-only re-scan hook supplied by the CLI entry point
    /// (reuses the discovery scanner). Called once, when the tutorial
    /// completes; absent — or returning `None` — the completion screen
    /// renders unchanged.
    pub completion_rescan: Option<Box<dyn Fn() -> Option<ScanResults>>>,
    /// WOW-004: the chosen domain's finding count at the moment the path was
    /// loaded. Captured eagerly because the fix flow prunes `scan_results`
    /// in place mid-walk — the delta must compare against what the user
    /// actually started with, or an applied fix erases its own win.
    pub completion_baseline: Option<usize>,
    /// WOW-004: computed findings delta for the chosen domain.
    pub completion_delta: Option<FindingsDelta>,
    /// Pending fix request emitted when the user presses `f`.
    pub pending_fix: Option<FixRequest>,
    /// Active inline editor, opened with `e` on a step that has an
    /// [`edit_target`](TutorialStep::edit_target). `None` when not editing.
    pub editor: Option<eddacraft_tui::widgets::editor::EditorState>,
    /// The relative file path the active editor writes to on save. Mirrors the
    /// current step's `edit_target`; kept separately so the write target is
    /// stable even if the step list is mutated mid-edit.
    pub edit_path: Option<String>,
    /// Set when the last inline-editor save failed to write to disk (e.g.
    /// permissions). The editor stays open so the user does not lose work; the
    /// renderer surfaces this message.
    pub edit_error: Option<String>,
    /// Editor viewport height (inner rows) recorded by the renderer each frame,
    /// so the key handler can keep the cursor visible after a move. `Cell`
    /// because rendering takes `&self` but needs to report the height it used.
    pub editor_viewport: std::cell::Cell<u16>,
}

impl TutorialState {
    pub fn new() -> Self {
        Self {
            phase: TutorialPhase::PathSelect,
            // LAUNCH-014: the value-first ProtectionLoop path is
            // listed first AND pre-selected so the default Enter
            // press lands the user on the concrete first-win walk
            // rather than the deeper-learning taxonomy paths.
            paths: vec![
                TutorialPath::ProtectionLoop,
                TutorialPath::DeveloperAcceleration,
                TutorialPath::Policy,
                TutorialPath::Architecture,
                TutorialPath::Drift,
                TutorialPath::CI,
            ],
            path_selected: 0,
            autoplay: false,
            autoplay_session: false,
            wants_autoplay_setup: false,
            working_root: None,
            autoplay_ghost_offset: 0,
            autoplay_result_dwell: false,
            autoplay_watch_dwell: false,
            autoplay_command: None,
            autoplay_command_advance: false,
            autoplay_runner: None,
            requires_sign_in: false,
            licence_gated_command: None,
            autoplay_failure: None,
            autoplay_teardown_requested: false,
            autoplay_saved_context: None,
            chosen_path: None,
            steps: Vec::new(),
            current_step: 0,
            should_quit: false,
            wants_back: false,
            scan_results: None,
            domain_findings: None,
            static_mode: false,
            static_notice: None,
            completed_paths: Vec::new(),
            resuming_notice: None,
            wants_watch_demo: false,
            reveal: None,
            completion_rescan: None,
            completion_baseline: None,
            completion_delta: None,
            pending_fix: None,
            editor: None,
            edit_path: None,
            edit_error: None,
            editor_viewport: std::cell::Cell::new(0),
        }
    }

    pub fn new_autoplay() -> Self {
        let mut state = Self::new();
        state.start_autoplay();
        state
    }

    pub fn new_autoplay_in(root: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let mut state = Self::new();
        state.start_autoplay_in(root)?;
        Ok(state)
    }

    /// Whether an inline editor is currently open. The CLI loop reads this to
    /// switch to a text-input key mapping (so ordinary letters — including
    /// `j`/`k`/`h`/`l`/`q`/space — are typed into the editor rather than
    /// consumed as navigation/quit commands).
    pub fn is_editing(&self) -> bool {
        self.editor.is_some()
    }

    /// Open the inline editor for the current step, if it declares an
    /// `edit_target`. Seeds from the file's current contents when it exists,
    /// otherwise from the step's `seed_template`. No-op if already editing, in
    /// static mode is still allowed (the save path writes + verifies directly
    /// and needs no watcher).
    pub fn open_step_editor(&mut self) {
        if self.editor.is_some() {
            return;
        }
        let Some(step) = self.steps.get(self.current_step) else {
            return;
        };
        let Some(target) = step.edit_target.clone() else {
            return;
        };
        if self.autoplay_session && self.working_root.is_none() {
            self.edit_error = Some("autoplay working root is unavailable".to_string());
            return;
        }
        let seed = step.seed_template.clone().unwrap_or_default();
        let declared_target = target.clone();
        let target = match self.resolve_session_target(&target) {
            Ok(target) => target,
            Err(error) => {
                self.edit_error = Some(error.to_string());
                return;
            }
        };
        let existing = std::fs::read_to_string(&target).ok();
        let content = existing.unwrap_or(seed);
        self.editor = Some(eddacraft_tui::widgets::editor::EditorState::from_string(
            &content,
        ));
        self.edit_path = Some(declared_target);
        self.edit_error = None;
    }

    /// Cancel the inline editor without writing anything.
    pub fn cancel_step_editor(&mut self) {
        self.editor = None;
        self.edit_path = None;
        self.edit_error = None;
    }

    /// Save the inline editor's content to `edit_path`, then run the step's
    /// verify. Advances the step when verification passes. Returns the write
    /// error (if any) so the caller/renderer can surface it; the editor stays
    /// open on write failure so the user does not lose their work.
    pub fn save_step_editor(&mut self) -> std::io::Result<()> {
        self.save_step_editor_with_advance(true)
    }

    fn save_step_editor_with_advance(&mut self, advance: bool) -> std::io::Result<()> {
        let (Some(editor), Some(path)) = (self.editor.as_ref(), self.edit_path.clone()) else {
            return Ok(());
        };
        let content = editor.content();
        let path = self.resolve_session_target(&path)?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        // Editor content is committed — close it and run verification.
        self.editor = None;
        self.edit_path = None;
        // Record a synthetic successful output so command-less verify steps
        // (the common case for edit steps) evaluate against a Pass baseline.
        let placeholder = CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            exit_code: Some(0),
        };
        if let Some(step) = self.steps.get_mut(self.current_step)
            && step.command.is_none()
        {
            step.output = Some(placeholder);
        }
        if self.run_verify_current() && advance {
            self.advance_step();
        }
        Ok(())
    }

    /// Enable static mode, disabling command execution and showing a notice.
    /// All steps become informational (press-enter-to-continue) regardless of
    /// whether they have a `command` attached.
    pub fn enable_static_mode(&mut self) {
        self.static_mode = true;
        self.static_notice =
            Some("Interactive mode unavailable \u{2014} showing guided walkthrough.".to_string());
    }

    /// Enable static mode with a caller-supplied notice so the user sees the
    /// specific cause (e.g. watcher failed) instead of the generic fallback.
    pub fn enable_static_mode_with_reason(&mut self, reason: impl Into<String>) {
        self.static_mode = true;
        self.static_notice = Some(reason.into());
    }

    /// Set which paths the user has previously completed (loaded from
    /// persistent progress file). The renderer uses this to show checkmarks.
    pub fn set_completed_paths(&mut self, paths: Vec<TutorialPath>) {
        self.completed_paths = paths;
    }

    /// Resume an interrupted session: load the path's steps and jump to
    /// `step_index`, marking earlier steps as completed per `steps_completed`.
    /// If the saved step count doesn't match the current path definition
    /// (e.g. after a tool upgrade), the stale session is discarded and the
    /// path starts fresh.
    pub fn resume_path(&mut self, path: TutorialPath, step_index: usize, steps_completed: &[bool]) {
        self.load_steps(path);
        // Stale session: step count changed since the session was saved.
        if steps_completed.len() != self.steps.len() {
            return;
        }
        for (i, step) in self.steps.iter_mut().enumerate() {
            if steps_completed.get(i).copied().unwrap_or(false) {
                step.completed = true;
            }
        }
        self.current_step = step_index.min(self.steps.len().saturating_sub(1));
        self.resuming_notice = Some(format!(
            "Resuming from step {} of {}.",
            self.current_step + 1,
            self.steps.len(),
        ));
    }

    pub fn set_scan_results(&mut self, results: ScanResults) {
        self.scan_results = Some(results);
    }

    pub fn picker_len(&self) -> usize {
        self.paths.len() + 1
    }

    pub fn picker_label(&self, index: usize) -> Option<&'static str> {
        self.paths
            .get(index)
            .map(|path| path.label())
            .or((index == self.paths.len()).then_some(AUTOPLAY_DEMO_LABEL))
    }

    pub(crate) fn picker_description(&self, index: usize) -> Option<&'static str> {
        self.paths
            .get(index)
            .map(|path| path.description())
            .or((index == self.paths.len()).then_some(AUTOPLAY_DEMO_DESCRIPTION))
    }

    pub(crate) fn picker_path(&self, index: usize) -> Option<TutorialPath> {
        self.paths.get(index).copied()
    }

    /// WOW-003: per-domain finding count for the path picker. `Some(n)` only
    /// when real scan results are present and the domain has at least one
    /// finding. Zero-finding and no-scan cases fall back to the standard
    /// picker copy, and showcase-derived counts are never presented as real
    /// findings (CIB-170), so `is_showcase` results yield `None`.
    pub fn picker_finding_count(&self, path: TutorialPath) -> Option<usize> {
        let results = self.scan_results.as_ref()?;
        if results.is_showcase {
            return None;
        }
        let count = results.count_by_domain(path);
        (count > 0).then_some(count)
    }

    fn next_fix_request(&self) -> Option<FixRequest> {
        let mut best: Option<(FindingSeverity, FixRequest)> = None;
        for finding in &self.domain_findings.as_ref()?.findings {
            let Some(request) = finding.fix_request() else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(severity, _)| finding.severity > *severity)
            {
                best = Some((finding.severity, request));
            }
        }
        best.map(|(_, request)| request)
    }

    pub fn load_steps(&mut self, path: TutorialPath) {
        let leaving_autoplay = self.autoplay_session;
        if leaving_autoplay {
            self.abort_autoplay_session();
            self.restore_autoplay_context();
            self.autoplay_teardown_requested = true;
        }
        self.autoplay = false;
        self.wants_autoplay_setup = false;
        self.steps = match path {
            TutorialPath::ProtectionLoop => paths::protection_loop_steps(),
            TutorialPath::DeveloperAcceleration => paths::developer_acceleration_steps(),
            TutorialPath::Policy => paths::policy_steps(),
            TutorialPath::Architecture => paths::architecture_steps(),
            TutorialPath::Drift => paths::drift_steps(),
            TutorialPath::CI => paths::ci_steps(),
        };
        self.current_step = 0;
        self.chosen_path = Some(path);
        self.domain_findings = self.scan_results.as_ref().map(|r| r.filter_by_domain(path));
        // WOW-004: snapshot the opening count now — `scan_results` may be
        // pruned by the fix flow before the walk completes. Showcase data is
        // never a real baseline (CIB-170).
        self.completion_baseline = self
            .scan_results
            .as_ref()
            .filter(|r| !r.is_showcase)
            .map(|r| r.count_by_domain(path));
        if self.requires_sign_in {
            let probe = self.licence_gated_command.clone();
            executor::apply_sign_in_bridge(&mut self.steps, |cmd| match &probe {
                Some(probe) => probe(cmd),
                None => executor::command_needs_licence_gate_fallback(cmd),
            });
        }
        self.phase = TutorialPhase::Running;
    }

    /// Start the isolated demonstration while retaining the real
    /// `ProtectionLoop` identity for session flow and completion semantics.
    pub fn start_autoplay(&mut self) {
        if self.autoplay_session {
            self.abort_autoplay_session();
            self.restore_autoplay_context();
            self.autoplay_teardown_requested = true;
        }
        self.stash_autoplay_context();
        self.load_steps(TutorialPath::ProtectionLoop);
        self.steps = paths::autoplay_protection_loop_steps();
        self.autoplay_failure = None;
        self.autoplay_session = true;
        self.wants_autoplay_setup = true;
    }

    fn stash_autoplay_context(&mut self) {
        self.autoplay_saved_context = Some(AutoplaySavedContext {
            working_root: self.working_root.take(),
            scan_results: self.scan_results.take(),
            domain_findings: self.domain_findings.take(),
            completion_rescan: self.completion_rescan.take(),
            completion_baseline: self.completion_baseline.take(),
            completion_delta: self.completion_delta.take(),
        });
    }

    fn restore_autoplay_context(&mut self) {
        let Some(saved) = self.autoplay_saved_context.take() else {
            return;
        };
        self.working_root = saved.working_root;
        self.scan_results = saved.scan_results;
        self.domain_findings = saved.domain_findings;
        self.completion_rescan = saved.completion_rescan;
        self.completion_baseline = saved.completion_baseline;
        self.completion_delta = saved.completion_delta;
    }

    /// Bind an ordinary tutorial session to its canonical workspace root.
    pub fn bind_working_root(&mut self, root: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        if self.autoplay_session {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "autoplay uses its isolated working root",
            ));
        }
        let root = canonicalize_working_path(root.as_ref())?;
        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "tutorial working root is not a directory",
            ));
        }
        self.working_root = Some(root);
        Ok(())
    }

    pub fn start_autoplay_in(&mut self, root: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let root = canonicalize_working_path(root.as_ref())?;
        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "tutorial working root is not a directory",
            ));
        }
        if !self.autoplay_session || !self.wants_autoplay_setup {
            self.start_autoplay();
        }
        self.working_root = Some(root);
        self.wants_autoplay_setup = false;
        self.autoplay = true;
        Ok(())
    }

    pub fn hand_back_autoplay(&mut self) -> bool {
        if self.autoplay {
            if let Some(command) = self.autoplay_command.take() {
                command.cancel();
            }
            self.autoplay = false;
            self.reveal = None;
            self.autoplay_result_dwell = false;
            self.autoplay_watch_dwell = false;
            true
        } else {
            false
        }
    }

    pub fn abort_autoplay_session(&mut self) {
        if let Some(command) = self.autoplay_command.take() {
            command.cancel();
        }
        self.autoplay = false;
        self.autoplay_session = false;
        self.working_root = None;
        self.wants_watch_demo = false;
        self.reveal = None;
        self.editor = None;
    }

    /// Recover from an autoplay failure **without leaving the TUI** (CIB-248).
    ///
    /// The old behaviour returned `Err` out of the welcome tutorial loop, so a
    /// failed demo step dropped the user to scrollback with
    /// `Error: autoplay command failed: ...`. A demo that cannot finish is not
    /// a reason to end the session: tear the sandbox session down, return to
    /// the path picker, and say what happened.
    ///
    /// CIB-249's teardown criteria ride along here — the surface stays owned by
    /// the TUI, so the terminal is restored by the ordinary exit path rather
    /// than by unwinding through an error.
    pub fn recover_from_autoplay_failure(&mut self, message: impl Into<String>) {
        self.abort_autoplay_session();
        // `start_autoplay` stashes the pre-demo scan context so the sandbox
        // cannot masquerade as the user's repo. Recovery has to put it back,
        // or the picker returns stripped of its per-domain finding counts.
        self.restore_autoplay_context();
        self.autoplay_failure = None;
        self.phase = TutorialPhase::PathSelect;
        self.resuming_notice = Some(message.into());
    }

    pub fn autoplay_session_active(&self) -> bool {
        self.autoplay_session
    }

    pub fn autoplay_driver_active(&self) -> bool {
        self.autoplay
    }

    /// Supply the in-process check runner used by the autoplay demo (CIB-248).
    ///
    /// `anvil-cli` owns this because it already depends on the check crates;
    /// `anvil-tui` stays a presentation crate. Without a runner the demo
    /// reports itself unavailable rather than shelling out to a gated CLI.
    pub fn set_autoplay_runner(&mut self, runner: executor::AutoplayRunner) {
        self.autoplay_runner = Some(runner);
    }

    /// CIB-349: when `requires_sign_in` is true, gated tutorial commands
    /// are rewritten into a sign-in bridge on path load. `command_is_gated`
    /// should be the CLI's `tutorial_command_needs_licence_gate` so the
    /// probe stays aligned with `CLI_GATED_COMMANDS`.
    pub fn set_sign_in_bridge(
        &mut self,
        requires_sign_in: bool,
        command_is_gated: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) {
        self.requires_sign_in = requires_sign_in;
        self.licence_gated_command = Some(std::sync::Arc::new(command_is_gated));
    }

    /// Test helper: enable the sign-in bridge with the TUI fallback
    /// classifier (no CLI injection).
    pub fn set_requires_sign_in(&mut self, requires_sign_in: bool) {
        self.requires_sign_in = requires_sign_in;
    }

    fn command_is_gated(&self, command: &str) -> bool {
        match &self.licence_gated_command {
            Some(probe) => probe(command),
            None => executor::command_needs_licence_gate_fallback(command),
        }
    }

    pub fn autoplay_failure(&self) -> Option<&str> {
        self.autoplay_failure.as_deref()
    }

    pub fn take_autoplay_failure(&mut self) -> Option<String> {
        self.autoplay_failure.take()
    }

    pub fn take_autoplay_teardown_requested(&mut self) -> bool {
        std::mem::take(&mut self.autoplay_teardown_requested)
    }

    pub fn autoplay_teardown_requested(&self) -> bool {
        self.autoplay_teardown_requested
    }

    fn fail_autoplay(&mut self, message: String) {
        if self.autoplay_failure.is_some() {
            return;
        }
        if let Some(command) = self.autoplay_command.take() {
            command.cancel();
        }
        self.autoplay_failure = Some(message);
        self.autoplay = false;
        self.wants_watch_demo = false;
        self.reveal = None;
        self.editor = None;
    }

    fn resolve_session_target(
        &self,
        target: impl AsRef<std::path::Path>,
    ) -> std::io::Result<std::path::PathBuf> {
        if self.autoplay_session && self.working_root.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "autoplay working root is unavailable",
            ));
        }
        match self.working_root.as_deref() {
            Some(root) => resolve_working_path(root, target.as_ref()),
            None => Ok(target.as_ref().to_path_buf()),
        }
    }

    /// Called by the TUI loop when the file watcher detects changes.
    /// If the current step has a `watch_path` and verification, re-runs
    /// the verify check (and optionally the command). Returns `true` if
    /// the step was auto-advanced.
    pub fn handle_file_change(&mut self, changed_paths: &[std::path::PathBuf]) -> bool {
        if self.phase != TutorialPhase::Running || self.static_mode {
            return false;
        }
        let Some(step) = self.steps.get(self.current_step) else {
            return false;
        };
        let Some(ref watch_target) = step.watch_path else {
            return false;
        };
        // Skip if the step already completed or hasn't been attempted yet
        // when it has a command (user should press Enter first).
        if step.completed || (step.command.is_some() && step.output.is_none()) {
            return false;
        }

        // Normalise the watch target to an absolute path so it matches the
        // absolute paths emitted by the file watcher, while applying the same
        // containment guard as commands, verification, and inline edits.
        let watch_target_path = std::path::PathBuf::from(watch_target);
        let Ok(target) = self.resolve_session_target(&watch_target_path) else {
            return false;
        };
        let relevant = changed_paths
            .iter()
            .any(|p| p == &target || p.starts_with(&target));
        if !relevant {
            return false;
        }

        // For steps with a command, re-execute it then verify.
        // For steps without a command, verify directly (e.g. FileExists).
        if let Some(ref cmd) = step.command.clone() {
            if self.execute_current_command(cmd) {
                return true;
            }
        } else if let Some(ref verify) = step.verify {
            // No command — verify directly with a placeholder output.
            let placeholder = CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
                exit_code: Some(0),
            };
            let result = verify.check_in_root(&placeholder, self.working_root.as_deref());
            let passed = result == VerifyResult::Pass;
            self.steps[self.current_step].verify_result = Some(result);
            if passed {
                self.advance_step();
                return true;
            }
        }

        false
    }

    pub fn handle_key(&mut self, action: Action) {
        // WOW-006 hands-back invariant: the first routed action only converts
        // autoplay into the ordinary interactive session. It must not also
        // execute, advance, navigate, quit, or cancel any in-flight state.
        if self.autoplay {
            self.hand_back_autoplay();
            return;
        }

        match self.phase {
            TutorialPhase::PathSelect => self.handle_path_select(action),
            TutorialPhase::Running => self.handle_running(action),
            TutorialPhase::Complete => self.handle_complete(action),
        }
    }

    fn handle_path_select(&mut self, action: Action) {
        match action {
            Action::Up if self.path_selected > 0 => {
                self.path_selected -= 1;
            }
            Action::Down if self.path_selected < self.picker_len().saturating_sub(1) => {
                self.path_selected += 1;
            }
            Action::Select => {
                if let Some(path) = self.picker_path(self.path_selected) {
                    self.load_steps(path);
                } else if self.path_selected == self.paths.len() {
                    self.start_autoplay();
                }
            }
            Action::Back => self.wants_back = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    pub fn advance_step(&mut self) {
        // Clear the resume notice on first interaction.
        self.resuming_notice = None;
        self.autoplay_ghost_offset = 0;
        self.autoplay_result_dwell = false;
        self.autoplay_watch_dwell = false;
        if self.current_step < self.steps.len() {
            self.steps[self.current_step].completed = true;
            if self.current_step + 1 < self.steps.len() {
                self.current_step += 1;
            } else {
                self.phase = TutorialPhase::Complete;
                // WOW-004: re-scan once, at the moment the walk completes.
                self.compute_completion_delta();
            }
        }
    }

    /// Apply the terminal result returned by the watch-demo surface.
    ///
    /// This is the cross-crate state-transition boundary used by the CLI watch
    /// loop. Only a completed cycle may finish the active autoplay watch step;
    /// hand-back/continue results and repeated completion reports are no-ops.
    #[doc(hidden)]
    pub fn apply_watch_demo_outcome(&mut self, outcome: watch_demo::WatchDemoOutcome) {
        if outcome == watch_demo::WatchDemoOutcome::CycleComplete
            && self.phase == TutorialPhase::Running
            && self.autoplay_session
            && self
                .steps
                .get(self.current_step)
                .is_some_and(|step| step.watch_demo)
        {
            self.advance_step();
        }
    }

    /// WOW-004: register the read-only re-scan used to compute the
    /// completion findings delta. Supplied by the CLI so the state machine
    /// stays scanner-agnostic (and tests stay deterministic).
    pub fn set_completion_rescan(&mut self, rescan: impl Fn() -> Option<ScanResults> + 'static) {
        self.completion_rescan = Some(Box::new(rescan));
    }

    /// WOW-004: compute the before/after findings count for the chosen
    /// domain. No-op — leaving the completion screen unchanged — without a
    /// re-scan hook, without an opening baseline (no scan, or showcase data;
    /// CIB-170: example findings are never counted as real), or when the
    /// re-scan fails or returns showcase data.
    fn compute_completion_delta(&mut self) {
        let Some(rescan) = self.completion_rescan.as_ref() else {
            return;
        };
        let Some(before) = self.completion_baseline else {
            return;
        };
        let Some(path) = self.chosen_path else {
            return;
        };
        let Some(rescanned) = rescan() else {
            return;
        };
        if rescanned.is_showcase {
            return;
        }
        let after = rescanned.count_by_domain(path);
        self.completion_delta = Some(FindingsDelta {
            before,
            after,
            after_in_test_paths: rescanned.count_in_test_paths_by_domain(path),
        });
    }

    /// Returns true if the current step has failed command output or failed
    /// verification, waiting for retry/skip.
    pub fn current_step_failed(&self) -> bool {
        let Some(step) = self.steps.get(self.current_step) else {
            return false;
        };
        let command_failed = step.output.as_ref().is_some_and(|o| !o.success);
        let verify_failed = matches!(step.verify_result, Some(VerifyResult::Fail(_)));
        command_failed || verify_failed
    }

    /// Run the verification check for the current step against its stored
    /// output. Returns `true` if the step should advance (either no
    /// verification is configured, verification passed, or the step index
    /// is out of bounds).
    fn run_verify_current(&mut self) -> bool {
        let Some(step) = self.steps.get_mut(self.current_step) else {
            return true;
        };
        let Some(ref output) = step.output else {
            return true;
        };
        if let Some(ref verify) = step.verify {
            let result = verify.check_in_root(output, self.working_root.as_deref());
            let passed = result == VerifyResult::Pass;
            step.verify_result = Some(result);
            passed
        } else {
            true
        }
    }

    /// Route a key to the active inline editor. Save (Ctrl-S) writes+verifies;
    /// Esc cancels; everything else is text editing. Called only while
    /// `self.editor.is_some()`.
    fn handle_editor_key(&mut self, action: Action) {
        if let Action::Character('\x13') = action {
            match self.save_step_editor() {
                Ok(()) => self.edit_error = None,
                Err(e) => {
                    self.edit_error = Some(format!(
                        "Could not write {}: {e}",
                        self.edit_path.as_deref().unwrap_or("file")
                    ));
                }
            }
            return;
        }
        match action {
            Action::Back => self.cancel_step_editor(),
            Action::Quit => self.should_quit = true,
            other => {
                // Viewport rows the renderer last drew into (falls back to a
                // sane default before the first frame). Used both to page and
                // to keep the cursor visible: the renderer clones the editor to
                // draw it, so any scroll adjustment it makes is discarded —
                // scrolling has to happen on the authoritative state here.
                let viewport = usize::from(self.editor_viewport.get()).max(1);
                if let Some(ed) = self.editor.as_mut() {
                    match other {
                        Action::Character(c) => ed.insert(c),
                        Action::Backspace => ed.backspace(),
                        Action::Delete => ed.delete(),
                        Action::Up => ed.move_up(),
                        Action::Down => ed.move_down(),
                        Action::Left => ed.move_left(),
                        Action::Right => ed.move_right(),
                        Action::Home => ed.home(),
                        Action::End => ed.end(),
                        Action::PageUp => ed.page_up(viewport),
                        Action::PageDown => ed.page_down(viewport),
                        _ => {}
                    }
                    ed.ensure_cursor_visible(viewport);
                }
            }
        }
    }

    fn handle_running(&mut self, action: Action) {
        // Inline editor active: route keys to the editor. The CLI loop
        // switches to a text-input key map while `is_editing()`, so ordinary
        // letters (including j/k/h/l/q and space) arrive as Action::Character
        // and Enter arrives as Character('\n'). Ctrl-S (Character('\x13'))
        // saves; Esc cancels.
        if self.editor.is_some() {
            self.handle_editor_key(action);
            return;
        }

        // WOW-002: while a command reveal is in flight, keys fast-forward to
        // the fully revealed command and execute it immediately — except
        // Back/Quit. The reveal window is the user's last chance to back out
        // before the run (CIB-165 consent posture), and q must never become
        // a "run it now" key: Esc aborts the pending command, q aborts it
        // and quits.
        if self.reveal.is_some() {
            match action {
                Action::Back => self.reveal = None,
                Action::Quit => {
                    self.reveal = None;
                    self.should_quit = true;
                }
                _ => self.finish_reveal(),
            }
            return;
        }

        // When a command has failed or verification has failed, only retry
        // (r) and skip (s) are active; everything else is ignored except
        // Back and Quit.
        if self.current_step_failed() {
            match action {
                // 'r' — clear output/verify state and re-execute the command
                Action::Character('r') => {
                    let cmd = self.steps.get_mut(self.current_step).and_then(|step| {
                        step.output = None;
                        step.verify_result = None;
                        step.command.clone()
                    });
                    if let Some(cmd) = cmd {
                        self.execute_current_command(&cmd);
                    }
                }
                // 's' — skip: mark complete and advance without re-running
                Action::Character('s') => {
                    self.advance_step();
                }
                Action::Back => self.phase = TutorialPhase::PathSelect,
                Action::Quit => self.should_quit = true,
                _ => {}
            }
            return;
        }

        match action {
            Action::Select => {
                if self.static_mode {
                    // Static mode: all steps are informational — advance
                    // without executing commands.
                    self.advance_step();
                } else if let Some(step) = self.steps.get(self.current_step)
                    && step.watch_demo
                {
                    // Watch demo step: signal the TUI loop to launch the demo.
                    self.wants_watch_demo = true;
                } else if let Some(step) = self.steps.get(self.current_step) {
                    if step.sign_in_bridge {
                        // CIB-349: not a runnable check — Enter continues.
                        self.advance_step();
                    } else if let Some(cmd) = step.command.clone() {
                        if self.requires_sign_in && self.command_is_gated(&cmd) {
                            if let Some(step) = self.steps.get_mut(self.current_step) {
                                executor::bridge_command_step(step, &cmd);
                            }
                        } else {
                            // WOW-002: don't execute yet — start the typed
                            // reveal. The command runs when the reveal completes
                            // (via ticks or a fast-forwarding keypress).
                            self.reveal = Some(CommandReveal::new(cmd));
                        }
                    } else {
                        // No command — informational step, advance immediately.
                        self.advance_step();
                    }
                }
            }
            Action::Toggle => {
                // Toggle (space) advances every step WITHOUT executing its
                // command — the skip-without-running escape hatch (WOW-001,
                // CIB-165 consent posture). A user who declines a command
                // step must be able to move past it; execution stays
                // exclusively on Enter, so accidental shell invocation via
                // space remains impossible.
                self.advance_step();
            }
            Action::Character('f') => {
                if let Some(request) = self.next_fix_request() {
                    self.pending_fix = Some(request);
                }
            }
            // 'e' — open the inline editor for a create/edit step, keeping the
            // user inside the tutorial instead of a second terminal. No-op
            // unless the current step declares an `edit_target`.
            Action::Character('e') => self.open_step_editor(),
            Action::Back => self.phase = TutorialPhase::PathSelect,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    /// WOW-002: whether a typed-command reveal is in flight. The renderer
    /// swaps the command bar for the partially typed prompt line while true.
    pub fn is_revealing(&self) -> bool {
        self.reveal.is_some()
    }

    /// WOW-002: advance the in-flight reveal by one fixed interval. Called by
    /// the TUI loop on its existing poll tick; executes the command once the
    /// full command is visible. No-op when no reveal is active.
    pub fn reveal_tick(&mut self) {
        if self.autoplay_failure.is_some() || self.phase != TutorialPhase::Running {
            return;
        }
        if self.poll_autoplay_command() {
            return;
        }
        if self.autoplay {
            self.autoplay_tick();
            return;
        }
        let Some(reveal) = self.reveal.as_mut() else {
            return;
        };
        let total = reveal.command.chars().count();
        reveal.shown = (reveal.shown + REVEAL_CHARS_PER_TICK).min(total);
        if reveal.is_complete() {
            self.finish_reveal();
        }
    }

    fn poll_autoplay_command(&mut self) -> bool {
        let Some(command) = self.autoplay_command.as_mut() else {
            return false;
        };
        match command.is_finished() {
            Ok(false) => true,
            Ok(true) => {
                let output = self
                    .autoplay_command
                    .take()
                    .expect("checked above")
                    .finish();
                self.consume_autoplay_output(output);
                true
            }
            Err(error) => {
                let message = format!("autoplay command failed: {error}");
                if self.autoplay {
                    self.fail_autoplay(message);
                } else if let Some(step) = self.steps.get_mut(self.current_step) {
                    step.output = Some(CommandOutput {
                        stdout: String::new(),
                        stderr: message,
                        success: false,
                        exit_code: None,
                    });
                }
                true
            }
        }
    }

    fn consume_autoplay_output(&mut self, output: CommandOutput) {
        let accepted = output.success
            || self
                .steps
                .get(self.current_step)
                .is_some_and(|step| step.verify.is_some());
        if let Some(step) = self.steps.get_mut(self.current_step) {
            step.output = Some(output);
        }
        if accepted && self.run_verify_current() {
            if self.autoplay {
                self.autoplay_result_dwell = true;
            } else if self.autoplay_command_advance {
                self.advance_step();
            }
        } else if self.autoplay {
            let detail = self
                .steps
                .get(self.current_step)
                .and_then(|step| step.output.as_ref())
                .map(|output| output.stderr.trim())
                .filter(|message| !message.is_empty())
                .unwrap_or("verification failed");
            self.fail_autoplay(format!("autoplay command failed: {detail}"));
        }
    }

    fn autoplay_tick(&mut self) {
        if self.working_root.is_none() {
            self.fail_autoplay("autoplay working root is unavailable".to_string());
            return;
        }

        if self.autoplay_result_dwell {
            self.autoplay_result_dwell = false;
            self.advance_step();
            return;
        }

        if let Some(reveal) = self.reveal.as_mut() {
            let total = reveal.command.chars().count();
            reveal.shown = (reveal.shown + REVEAL_CHARS_PER_TICK).min(total);
            if reveal.is_complete() {
                self.finish_reveal();
            }
            return;
        }

        if self.editor.is_some() {
            let chars: Vec<char> = paths::AUTOPLAY_APP_REPAIRED.chars().collect();
            let end = (self.autoplay_ghost_offset + REVEAL_CHARS_PER_TICK).min(chars.len());
            if let Some(editor) = self.editor.as_mut() {
                for character in &chars[self.autoplay_ghost_offset..end] {
                    editor.insert(*character);
                }
            }
            self.autoplay_ghost_offset = end;
            if end == chars.len() {
                match self.save_step_editor_with_advance(false) {
                    Ok(())
                        if self.steps[self.current_step].verify_result
                            == Some(VerifyResult::Pass) =>
                    {
                        self.autoplay_result_dwell = true;
                    }
                    Ok(()) => self.fail_autoplay(
                        "autoplay fixture verification failed after edit".to_string(),
                    ),
                    Err(error) => {
                        self.fail_autoplay(format!("autoplay fixture edit failed: {error}"));
                    }
                }
            }
            return;
        }

        let Some(step) = self.steps.get(self.current_step) else {
            return;
        };
        if let Some(command) = step.command.clone() {
            self.reveal = Some(CommandReveal::new(command));
        } else if step.edit_target.is_some() {
            self.open_step_editor();
            if self.editor.is_some() {
                self.editor = Some(eddacraft_tui::widgets::editor::EditorState::from_string(""));
                self.autoplay_ghost_offset = 0;
            }
        } else if step.watch_demo {
            if self.autoplay_watch_dwell {
                self.wants_watch_demo = true;
            } else {
                self.autoplay_watch_dwell = true;
            }
        }
    }

    /// Complete the reveal and execute the revealed command — the same
    /// execute → verify → advance sequence Enter performed before WOW-002.
    fn finish_reveal(&mut self) {
        let Some(reveal) = self.reveal.take() else {
            return;
        };
        if self.autoplay {
            if self.execute_current_command_with_advance(&reveal.command, false) {
                self.autoplay_result_dwell = true;
            }
        } else {
            self.execute_current_command(&reveal.command);
        }
    }

    /// Execute `cmd` for the current step, store its output, verify, and
    /// advance on success. Returns whether the step advanced. Shared by
    /// reveal completion, failed-step retry, and watch-triggered re-runs so
    /// the execution contract lives in one place.
    fn execute_current_command(&mut self, cmd: &str) -> bool {
        self.execute_current_command_with_advance(cmd, true)
    }

    fn execute_current_command_with_advance(&mut self, cmd: &str, advance: bool) -> bool {
        if self.autoplay_session && self.working_root.is_none() {
            if let Some(step) = self.steps.get_mut(self.current_step) {
                step.output = Some(CommandOutput {
                    stdout: String::new(),
                    stderr: "autoplay working root is unavailable".to_string(),
                    success: false,
                    exit_code: None,
                });
            }
            return false;
        }
        // CIB-349: never spawn a licence-gated child from a signed-out
        // welcome/tutorial walk. Autoplay stays on its in-process runner
        // (CIB-248) and is not rewritten here.
        if !self.autoplay_session && self.requires_sign_in && self.command_is_gated(cmd) {
            if let Some(step) = self.steps.get_mut(self.current_step) {
                let gated = cmd.to_string();
                executor::bridge_command_step(step, &gated);
            }
            return false;
        }
        if self.autoplay_session {
            let Some(root) = self.working_root.as_deref() else {
                return false;
            };
            match executor::AutoplayCommand::spawn(cmd, root, self.autoplay_runner.as_ref()) {
                Ok(command) => {
                    self.autoplay_command = Some(command);
                    self.autoplay_command_advance = advance;
                }
                Err(error) => {
                    let message = format!("autoplay command failed: {error}");
                    if self.autoplay {
                        self.fail_autoplay(message);
                    } else if let Some(step) = self.steps.get_mut(self.current_step) {
                        step.output = Some(CommandOutput {
                            stdout: String::new(),
                            stderr: message,
                            success: false,
                            exit_code: None,
                        });
                    }
                }
            }
            return false;
        }
        let result = match self.working_root.as_deref() {
            Some(root) => executor::execute_command_in(cmd, root),
            None => executor::execute_command(cmd),
        };
        let succeeded = result.success
            || (self.autoplay_session
                && self
                    .steps
                    .get(self.current_step)
                    .is_some_and(|step| step.verify.is_some()));
        if let Some(step) = self.steps.get_mut(self.current_step) {
            step.output = Some(result);
        }
        if succeeded && self.run_verify_current() {
            if advance {
                self.advance_step();
            }
            return true;
        }
        // On command failure we stay on the same step (retry/skip take over).
        false
    }

    /// Whether the current step offers inline editing (`e`).
    pub fn current_step_is_editable(&self) -> bool {
        self.steps
            .get(self.current_step)
            .is_some_and(|s| s.edit_target.is_some())
    }

    /// WOW-001: whether the current step executes a shell command on Enter.
    /// Drives the command/informational split in the footer help.
    /// CIB-349: a sign-in bridge is not a runnable check.
    pub fn current_step_has_command(&self) -> bool {
        self.steps
            .get(self.current_step)
            .is_some_and(|s| s.command.is_some() && !s.sign_in_bridge)
    }

    /// CIB-349: whether the current step is a sign-in bridge rather than
    /// a runnable gated command.
    pub fn current_step_is_sign_in_bridge(&self) -> bool {
        self.steps
            .get(self.current_step)
            .is_some_and(|s| s.sign_in_bridge)
    }

    fn handle_complete(&mut self, action: Action) {
        match action {
            Action::Select | Action::Back => {
                let leaving_autoplay = self.autoplay_session;
                if leaving_autoplay {
                    self.abort_autoplay_session();
                    self.restore_autoplay_context();
                    self.autoplay_teardown_requested = true;
                }
                if !leaving_autoplay
                    && let Some(path) = self.chosen_path
                    && !self.completed_paths.contains(&path)
                {
                    self.completed_paths.push(path);
                }
                self.phase = TutorialPhase::PathSelect;
                self.steps.clear();
                self.current_step = 0;
                self.chosen_path = None;
                if !leaving_autoplay {
                    self.domain_findings = None;
                    self.completion_baseline = None;
                    self.completion_delta = None;
                }
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl crate::surface::Surface for TutorialState {
    fn surface_name(&self) -> &'static str {
        "Tutorial"
    }

    fn help_text(&self) -> &'static str {
        match self.phase {
            TutorialPhase::PathSelect => "j/k navigate  enter select  esc close tutorial  q quit",
            TutorialPhase::Running => {
                if self.is_editing() {
                    "type to edit  enter newline  ctrl-s save  esc cancel"
                } else if self.is_revealing() {
                    "any key run now  esc cancel  q quit"
                } else if self.current_step_failed() {
                    "r retry  s skip  esc paths  q quit"
                } else if self.current_step_is_editable() {
                    "e edit inline  space next  esc paths  q quit"
                } else if self.static_mode {
                    "enter next  esc paths  q quit"
                } else if self.current_step_is_sign_in_bridge() {
                    "enter next  run anvil auth login first  esc paths  q quit"
                } else if self.current_step_has_command() {
                    // WOW-001: command steps say what Enter really does and
                    // make the skip-without-running escape hatch visible.
                    if self.next_fix_request().is_some() {
                        "enter run command  space skip without running  f fix  esc paths  q quit"
                    } else {
                        "enter run command  space skip without running  esc paths  q quit"
                    }
                } else if self.next_fix_request().is_some() {
                    "enter next  space next  f fix  esc paths  q quit"
                } else {
                    "enter next  space next  esc paths  q quit"
                }
            }
            TutorialPhase::Complete => "enter choose another  esc paths  q quit",
        }
    }

    fn handle_key(&mut self, action: Action) {
        self.handle_key(action);
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.pending_fix.is_some()
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        // CIB-274: fold any pre-autoplay stash back into the live fields
        // before clearing, rather than dropping it. Five of the six fields
        // `AutoplaySavedContext` owns are cleared below anyway
        // (`working_root`, `scan_results`, `domain_findings`,
        // `completion_baseline`, `completion_delta`), but `completion_rescan`
        // is injected session capability that `reset` deliberately preserves
        // (same class as `autoplay_runner`, `static_mode`, `completed_paths`).
        // Discarding the stash would silently lose that hook for the rest of
        // the session, because during autoplay the live field is empty.
        // Invariant: every stash-owned field must be either cleared here or
        // preserved on purpose — see
        // `reset_leaves_no_stash_owned_field_orphaned`.
        self.restore_autoplay_context();
        self.should_quit = false;
        self.wants_back = false;
        self.reveal = None;
        self.autoplay = false;
        self.autoplay_session = false;
        self.wants_autoplay_setup = false;
        self.working_root = None;
        self.autoplay_ghost_offset = 0;
        self.autoplay_result_dwell = false;
        self.autoplay_watch_dwell = false;
        if let Some(command) = self.autoplay_command.take() {
            command.cancel();
        }
        self.autoplay_command_advance = false;
        self.autoplay_failure = None;
        self.autoplay_teardown_requested = false;
        self.pending_fix = None;
        self.editor = None;
        self.edit_path = None;
        self.edit_error = None;
        self.phase = TutorialPhase::PathSelect;
        self.path_selected = 0;
        self.steps.clear();
        self.current_step = 0;
        self.chosen_path = None;
        self.scan_results = None;
        self.domain_findings = None;
        self.completion_baseline = None;
        self.completion_delta = None;
        self.resuming_notice = None;
        // static_mode, static_notice, completed_paths, requires_sign_in, and
        // licence_gated_command are intentionally preserved — they represent
        // environment/session state, not transient.
    }

    fn render(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        theme: &eddacraft_tui::theme::EddaCraftTheme,
    ) {
        render::render(frame, area, self, theme);
    }
}

impl Default for TutorialState {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationSource for TutorialState {
    fn notifications(&self) -> Vec<Notification> {
        let mut out = Vec::new();

        if let Some(notice) = self.static_notice.as_ref() {
            out.push(surface_notification(
                "tutorial",
                NotificationClass::Warning,
                NotificationPriority::High,
                "Interactive mode unavailable",
                notice,
            ));
        }

        if let Some(notice) = self.resuming_notice.as_ref() {
            out.push(surface_notification(
                "tutorial",
                NotificationClass::Info,
                NotificationPriority::Normal,
                "Tutorial resumed",
                notice,
            ));
        }

        // Only emit step-level failures while the tutorial is actively
        // running on a non-completed step. Once a step is skipped or the
        // phase flips to Complete, its stored `output`/`verify_result` are
        // stale and must not re-surface as live failures (adversarial F-002).
        if self.phase == TutorialPhase::Running
            && let Some(step) = self.steps.get(self.current_step)
            && !step.completed
        {
            if let Some(output) = &step.output
                && !output.success
            {
                // Do NOT echo stderr into the notification message. stderr
                // from shell commands regularly contains absolute paths,
                // credential-helper output, and $HOME/username — shipping
                // it via NotificationSource would leak that to every
                // telemetry subscriber (CWE-209). Keep the raw stderr on
                // `step.output` for local TUI rendering only.
                let message = format!(
                    "{} failed with exit code {}",
                    step.title,
                    output.exit_code.unwrap_or(-1),
                );
                out.push(surface_notification(
                    "tutorial",
                    NotificationClass::Failure,
                    NotificationPriority::High,
                    "Tutorial step failed",
                    message,
                ));
            }
            if matches!(step.verify_result, Some(VerifyResult::Fail(_))) {
                // verify_hint is author-controlled (tutorial path definitions),
                // not user input, so it is safe to surface verbatim.
                let hint = step
                    .verify_hint
                    .as_deref()
                    .unwrap_or("Verification failed.");
                out.push(surface_notification(
                    "tutorial",
                    NotificationClass::Failure,
                    NotificationPriority::High,
                    "Verification failed",
                    hint.to_string(),
                ));
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a state pre-loaded with informational-only steps (no commands) so
    /// state-machine tests do not accidentally invoke real processes.
    fn state_with_plain_steps(count: usize) -> TutorialState {
        let mut state = TutorialState::new();
        state.steps = (0..count)
            .map(|i| TutorialStep {
                title: format!("Step {i}"),
                description: format!("Description {i}"),
                instruction: format!("Press enter to continue ({i})."),
                ..TutorialStep::default()
            })
            .collect();
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    /// Build a state pre-loaded with a single step that has a given command.
    fn state_with_command_step(command: &str) -> TutorialState {
        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Cmd Step".to_string(),
            description: "A step with a command.".to_string(),
            instruction: format!("Run: {command}"),
            command: Some(command.to_string()),
            effect: Some(CommandEffect::ReadOnly),
            ..TutorialStep::default()
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    /// Build a state with a command step that has verification attached.
    fn state_with_verified_step(command: &str, verify: Verify, hint: &str) -> TutorialState {
        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Verified Step".to_string(),
            description: "A step with verification.".to_string(),
            instruction: format!("Run: {command}"),
            command: Some(command.to_string()),
            effect: Some(CommandEffect::ReadOnly),
            verify: Some(verify),
            verify_hint: Some(hint.to_string()),
            ..TutorialStep::default()
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    /// Press Enter on a command step, then fast-forward the WOW-002 typed
    /// reveal with a second keypress so the command actually executes.
    /// Mirrors a user hitting a key mid-reveal. Only valid on command steps —
    /// on informational steps the second key would advance an extra step.
    fn select_and_run(state: &mut TutorialState) {
        state.handle_key(Action::Select);
        assert!(
            state.is_revealing(),
            "Enter on a command step must start the reveal"
        );
        state.handle_key(Action::Select);
    }

    #[test]
    fn starts_at_path_select() {
        let state = TutorialState::new();
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        // LAUNCH-014: paths include the value-first ProtectionLoop default
        // (index 0 / pre-selected), the developer-acceleration path, and the
        // four deeper-learning tracks.
        assert_eq!(state.paths.len(), 6);
        assert_eq!(state.paths[0], TutorialPath::ProtectionLoop);
        assert_eq!(state.paths[1], TutorialPath::DeveloperAcceleration);
        assert_eq!(state.path_selected, 0);
        assert!(state.chosen_path.is_none());
    }

    #[test]
    fn path_selection_advances_to_running() {
        // LAUNCH-014: hitting Enter from the default selection lands
        // the user on the ProtectionLoop walk, not the Policy
        // taxonomy path. The chosen_path assertion is the visible
        // pin against accidental reordering.
        let mut state = TutorialState::new();
        state.handle_key(Action::Select);
        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.chosen_path, Some(TutorialPath::ProtectionLoop));
        assert!(!state.steps.is_empty());
        assert_eq!(state.current_step, 0);
    }

    #[test]
    fn picker_adds_one_distinct_autoplay_entry_without_changing_paths() {
        let mut state = TutorialState::new();
        state.set_completed_paths(vec![TutorialPath::Policy]);

        assert_eq!(state.paths.len(), 6);
        assert_eq!(state.picker_len(), state.paths.len() + 1);
        assert_eq!(
            state.picker_label(state.paths.len()),
            Some("Watch anvil work (demo)")
        );
        assert_eq!(state.completed_paths, vec![TutorialPath::Policy]);
        assert!(!state.autoplay);
    }

    #[test]
    fn selecting_demo_starts_autoplay_protection_loop_session() {
        let mut state = TutorialState::new();
        for _ in 0..state.paths.len() {
            state.handle_key(Action::Down);
        }
        assert_eq!(state.path_selected, state.paths.len());

        state.handle_key(Action::Select);

        assert!(!state.autoplay);
        assert!(state.wants_autoplay_setup);
        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.chosen_path, Some(TutorialPath::ProtectionLoop));
    }

    #[test]
    fn autoplay_initialisers_load_only_the_authorised_demo_beats() {
        let direct = TutorialState::new_autoplay();
        let mut picker = TutorialState::new();
        for _ in 0..picker.paths.len() {
            picker.handle_key(Action::Down);
        }
        picker.handle_key(Action::Select);

        let signature = |state: &TutorialState| {
            state
                .steps
                .iter()
                .map(|step| {
                    (
                        step.title.clone(),
                        step.command.is_some(),
                        step.edit_target.is_some(),
                        step.verify.is_some(),
                        step.watch_demo,
                    )
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(signature(&direct), signature(&picker));
        assert!(!direct.autoplay);
        assert!(direct.wants_autoplay_setup);
        assert_eq!(direct.chosen_path, Some(TutorialPath::ProtectionLoop));
        assert!(direct.steps.iter().any(|step| step.command.is_some()));
        assert!(direct.steps.iter().any(|step| step.edit_target.is_some()));
        assert!(direct.steps.iter().any(|step| step.verify.is_some()));
        assert!(direct.steps.iter().any(|step| step.watch_demo));

        let body = direct
            .steps
            .iter()
            .map(|step| format!("{}\n{}", step.title, step.description))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(body.contains("AP-003"));
        assert!(body.contains("AP-004"));

        let ordinary = paths::protection_loop_steps();
        assert_eq!(ordinary.len(), 5);
        assert!(!ordinary.iter().any(|step| step.edit_target.is_some()));
        assert!(!ordinary.iter().any(|step| step.watch_demo));
    }

    #[test]
    fn binding_prepared_autoplay_to_root_does_not_request_teardown() {
        let root = tempfile::tempdir().expect("root");
        let mut state = TutorialState::new();
        state.start_autoplay();

        state
            .start_autoplay_in(root.path())
            .expect("bind autoplay root");

        assert!(state.autoplay_driver_active());
        assert!(state.autoplay_session_active());
        assert!(!state.autoplay_teardown_requested());
    }

    #[test]
    fn first_routed_action_hands_back_without_other_state_changes() {
        let mut path_select = TutorialState::new();
        path_select.autoplay = true;
        path_select.handle_key(Action::Down);
        assert!(!path_select.autoplay);
        assert_eq!(path_select.path_selected, 0);

        let running_root = tempfile::tempdir().expect("root");
        let mut running = TutorialState::new_autoplay_in(running_root.path()).expect("autoplay");
        running.reveal = Some(CommandReveal::new("echo must-not-run".to_string()));
        running.handle_key(Action::Character('x'));
        assert!(!running.autoplay);
        assert!(!running.is_revealing());
        assert!(running.steps[0].output.is_none());
        assert_eq!(running.current_step, 0);
        for _ in 0..8 {
            running.reveal_tick();
        }
        assert!(running.steps[0].output.is_none());

        let complete_root = tempfile::tempdir().expect("root");
        let mut complete = TutorialState::new_autoplay_in(complete_root.path()).expect("autoplay");
        complete.phase = TutorialPhase::Complete;
        complete.handle_key(Action::Select);
        assert!(!complete.autoplay);
        assert_eq!(complete.phase, TutorialPhase::Complete);
        assert_eq!(complete.chosen_path, Some(TutorialPath::ProtectionLoop));

        let quit_root = tempfile::tempdir().expect("root");
        let mut quit = TutorialState::new_autoplay_in(quit_root.path()).expect("autoplay");
        quit.handle_key(Action::Quit);
        assert!(!quit.autoplay);
        assert!(!quit.should_quit);
        assert!(!quit.wants_back);

        let back_root = tempfile::tempdir().expect("root");
        let mut back = TutorialState::new_autoplay_in(back_root.path()).expect("autoplay");
        back.handle_key(Action::Back);
        assert!(!back.autoplay);
        assert!(!back.wants_back);
        assert!(!back.should_quit);
    }

    #[test]
    fn ordinary_session_binding_survives_path_load_from_nested_launch_root() {
        let root = tempfile::tempdir().expect("workspace");
        let nested = root.path().join("nested/deeper");
        std::fs::create_dir_all(&nested).expect("nested launch directory");
        let canonical_root = canonicalize_working_path(root.path()).expect("canonical root");
        let mut state = TutorialState::new();
        state.working_root = Some(canonical_root.clone());

        state.load_steps(TutorialPath::Policy);

        assert_eq!(
            state.working_root.as_deref(),
            Some(canonical_root.as_path()),
            "selecting or resuming a path must retain the canonical workspace root"
        );
    }

    #[cfg(windows)]
    #[test]
    fn canonical_working_root_uses_ordinary_windows_path_form() {
        let root = tempfile::tempdir().expect("workspace");

        let canonical = canonicalize_working_path(root.path()).expect("canonical root");

        assert!(
            !canonical.as_os_str().to_string_lossy().starts_with(r"\\?\"),
            "tutorial roots must compare with ordinary watcher and CLI paths"
        );
    }

    #[cfg(unix)]
    #[test]
    fn ordinary_session_rejects_symlink_parent_escape() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        symlink(outside.path(), root.path().join("linked")).expect("symlink");
        let mut state = TutorialState::new();
        state.working_root = Some(canonicalize_working_path(root.path()).expect("canonical root"));
        state.load_steps(TutorialPath::Policy);
        state.steps[0] = TutorialStep {
            edit_target: Some("linked/escape.rego".to_string()),
            seed_template: Some("must not escape".to_string()),
            ..TutorialStep::default()
        };

        state.open_step_editor();

        assert!(!state.is_editing(), "symlink escape must fail closed");
        assert!(!outside.path().join("escape.rego").exists());
    }

    #[test]
    fn ordinary_watcher_target_uses_bound_root_not_process_cwd() {
        let root = tempfile::tempdir().expect("workspace");
        let canonical_root =
            canonicalize_working_path(root.path()).expect("canonical workspace fixture");
        let watched = canonical_root.join("watched");
        std::fs::create_dir_all(&watched).expect("watched directory");
        let marker = watched.join("marker.txt");
        std::fs::write(&marker, "ready").expect("marker");
        let mut state = TutorialState::new();
        state
            .bind_working_root(&canonical_root)
            .expect("bind workspace");
        state.phase = TutorialPhase::Running;
        state.steps = vec![TutorialStep {
            watch_path: Some("watched".to_string()),
            verify: Some(Verify::FileExists("watched/marker.txt".to_string())),
            ..TutorialStep::default()
        }];

        let advanced = state.handle_file_change(&[marker]);

        assert!(advanced);
        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    #[test]
    fn autoplay_commands_and_editor_are_confined_to_working_root() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join("marker.txt"), "sandbox").expect("fixture");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.steps[0].command = Some("cat marker.txt".to_string());
        state.steps[0].verify = None;
        assert!(!state.execute_current_command("cat marker.txt"));
        assert!(
            state
                .autoplay_failure()
                .is_some_and(|failure| failure.contains("must be exactly"))
        );

        state.phase = TutorialPhase::Running;
        state.current_step = 0;
        state.steps[0] = TutorialStep {
            edit_target: Some("../escape.txt".to_string()),
            seed_template: Some("must not escape".to_string()),
            ..TutorialStep::default()
        };
        state.open_step_editor();

        assert!(!state.is_editing());
        assert!(
            state
                .edit_error
                .as_deref()
                .is_some_and(|error| error.contains("outside"))
        );
        assert!(!root.path().parent().unwrap().join("escape.txt").exists());
        assert!(
            resolve_working_path(
                root.path(),
                std::path::Path::new("missing/../../outside.ts")
            )
            .is_err()
        );
    }

    #[test]
    fn rootless_autoplay_cannot_execute_or_open_editor_in_process_cwd() {
        let marker = format!("wow006-rootless-{}", std::process::id());
        let mut state = TutorialState::new_autoplay();
        assert!(!state.execute_current_command(&format!("touch {marker}")));
        assert!(!std::path::Path::new(&marker).exists());
        assert!(
            state.steps[0]
                .output
                .as_ref()
                .is_some_and(|output| output.stderr.contains("working root"))
        );

        state.handle_key(Action::Down);
        state.steps[0].output = None;
        assert!(!state.execute_current_command(&format!("touch {marker}")));
        assert!(!std::path::Path::new(&marker).exists());

        state.current_step = 1;
        state.open_step_editor();
        assert!(!state.is_editing());
        assert!(
            state
                .edit_error
                .as_deref()
                .is_some_and(|error| error.contains("working root"))
        );
    }

    #[test]
    fn autoplay_command_result_dwells_once_before_advancing() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.steps = vec![TutorialStep::default()];
        state.autoplay_command = Some(executor::AutoplayCommand::successful_for_test());

        for _ in 0..500 {
            state.reveal_tick();
            if state.steps[0].output.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(state.reveal.is_none());
        let output = state.steps[0].output.as_ref().expect("child output");
        assert!(output.success, "stderr: {}", output.stderr);
        assert_eq!(state.current_step, 0);
        assert_eq!(state.phase, TutorialPhase::Running);

        state.reveal_tick(); // advance after the explicit result dwell
        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    #[test]
    fn autoplay_watch_beat_requests_existing_transition() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.current_step = state.steps.len() - 1;

        state.reveal_tick();

        assert!(!state.wants_watch_demo);
        state.reveal_tick();

        assert!(state.wants_watch_demo);
        assert_eq!(state.phase, TutorialPhase::Running);
    }

    #[test]
    fn autoplay_cycle_complete_is_terminal_and_runs_completion_once() {
        let root = tempfile::tempdir().expect("tempdir");
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = std::sync::Arc::clone(&calls);
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.current_step = state.steps.len() - 1;
        state.completion_baseline = Some(0);
        state.set_completion_rescan(move || {
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(ScanResults::default())
        });
        state.wants_watch_demo = false;

        state.advance_step();
        for _ in 0..8 {
            state.reveal_tick();
        }

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(!state.wants_watch_demo);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn starting_autoplay_discards_user_scan_and_completion_state() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = std::sync::Arc::clone(&calls);
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.load_steps(TutorialPath::Policy);
        state.completion_delta = Some(FindingsDelta {
            before: 2,
            after: 1,
            after_in_test_paths: 0,
        });
        state.set_completion_rescan(move || {
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(ScanResults::default())
        });

        state.start_autoplay();
        assert!(state.scan_results.is_none());
        assert!(state.domain_findings.is_none());
        assert!(state.completion_baseline.is_none());
        assert!(state.completion_delta.is_none());
        assert!(state.completion_rescan.is_none());

        state.current_step = state.steps.len() - 1;
        state.advance_step();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn leaving_demo_restores_discovery_and_completion_context() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = std::sync::Arc::clone(&calls);
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.load_steps(TutorialPath::Policy);
        let baseline = state.completion_baseline;
        state.completion_delta = Some(FindingsDelta {
            before: 2,
            after: 1,
            after_in_test_paths: 0,
        });
        state.set_completion_rescan(move || {
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(ScanResults::default())
        });
        state.start_autoplay();
        state.hand_back_autoplay();
        state.current_step = state.steps.len() - 1;
        state.advance_step();
        state.handle_key(Action::Select);

        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(state.scan_results.is_some());
        assert!(state.domain_findings.is_some());
        assert_eq!(state.completion_baseline, baseline);
        assert_eq!(
            state.completion_delta,
            Some(FindingsDelta {
                before: 2,
                after: 1,
                after_in_test_paths: 0
            })
        );
        state.completion_rescan.as_ref().expect("rescan")();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    /// CIB-274: `reset` must not silently drop the pre-autoplay stash. Every
    /// field `AutoplaySavedContext` owns has to end up either cleared by
    /// `reset` or deliberately preserved by it — never orphaned inside a
    /// discarded stash. The exhaustive destructure below is the drift alarm:
    /// adding a field to the stash stops this test compiling until the new
    /// field is given a reset disposition in the assertions further down.
    #[test]
    fn reset_leaves_no_stash_owned_field_orphaned() {
        let root = tempfile::tempdir().expect("tempdir");
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = std::sync::Arc::clone(&calls);
        let mut state = TutorialState::new();
        state.bind_working_root(root.path()).expect("working root");
        state.set_scan_results(make_scan_results());
        state.load_steps(TutorialPath::Policy);
        state.completion_delta = Some(FindingsDelta {
            before: 2,
            after: 1,
            after_in_test_paths: 0,
        });
        state.set_completion_rescan(move || {
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(ScanResults::default())
        });

        state.stash_autoplay_context();
        let AutoplaySavedContext {
            working_root,
            scan_results,
            domain_findings,
            completion_rescan,
            completion_baseline,
            completion_delta,
        } = state.autoplay_saved_context.take().expect("stash");
        assert!(working_root.is_some(), "stash owns working_root");
        assert!(scan_results.is_some(), "stash owns scan_results");
        assert!(domain_findings.is_some(), "stash owns domain_findings");
        assert!(completion_rescan.is_some(), "stash owns completion_rescan");
        assert!(
            completion_baseline.is_some(),
            "stash owns completion_baseline"
        );
        assert!(completion_delta.is_some(), "stash owns completion_delta");

        // Hand the captured context back and re-stash, so `reset` sees a fully
        // populated stash exactly as it would mid-autoplay.
        state.working_root = working_root;
        state.scan_results = scan_results;
        state.domain_findings = domain_findings;
        state.completion_rescan = completion_rescan;
        state.completion_baseline = completion_baseline;
        state.completion_delta = completion_delta;
        state.stash_autoplay_context();

        <TutorialState as crate::surface::Surface>::reset(&mut state);

        assert!(
            state.autoplay_saved_context.is_none(),
            "reset must consume the stash"
        );
        assert!(state.working_root.is_none(), "reset clears working_root");
        assert!(state.scan_results.is_none(), "reset clears scan_results");
        assert!(
            state.domain_findings.is_none(),
            "reset clears domain_findings"
        );
        assert!(
            state.completion_baseline.is_none(),
            "reset clears completion_baseline"
        );
        assert!(
            state.completion_delta.is_none(),
            "reset clears completion_delta"
        );
        // The re-scan hook is injected session capability, not transient state
        // (same class as `autoplay_runner`, `static_mode`, `completed_paths`),
        // so `reset` preserves it. The stash therefore has to restore it —
        // dropping the stash would lose it for the rest of the session.
        state
            .completion_rescan
            .as_ref()
            .expect("reset preserves the injected rescan hook")();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_autoplay_command_is_terminal_and_taken_once() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.steps = vec![TutorialStep {
            command: Some("anvil check ../outside.ts".to_string()),
            effect: Some(CommandEffect::ReadOnly),
            ..TutorialStep::default()
        }];

        for _ in 0..32 {
            state.reveal_tick();
        }

        assert!(!state.autoplay);
        assert!(state.autoplay_failure().is_some());
        assert!(state.autoplay_command.is_none());
        let first = state.take_autoplay_failure().expect("terminal failure");
        assert!(first.contains("autoplay command failed"));
        assert!(state.take_autoplay_failure().is_none());
        for _ in 0..8 {
            state.reveal_tick();
        }
        assert!(state.autoplay_command.is_none());
        assert!(state.reveal.is_none());
    }

    #[test]
    fn autoplay_completion_does_not_mark_the_real_path_complete() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.current_step = state.steps.len() - 1;
        state.advance_step();
        assert_eq!(state.phase, TutorialPhase::Complete);

        assert!(state.hand_back_autoplay());
        state.handle_key(Action::Select);

        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.autoplay_session_active());
        assert!(
            !state
                .completed_paths
                .contains(&TutorialPath::ProtectionLoop)
        );
    }

    #[test]
    fn ordinary_path_selection_ends_handed_back_demo_session() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        assert!(state.autoplay_session_active());
        assert!(state.hand_back_autoplay());
        state.current_step = state.steps.len() - 1;
        state.advance_step();
        state.handle_key(Action::Select);
        state.handle_key(Action::Select);

        assert_eq!(state.chosen_path, Some(TutorialPath::ProtectionLoop));
        assert!(!state.autoplay_session_active());
        assert!(state.take_autoplay_teardown_requested());
        assert!(!state.take_autoplay_teardown_requested());
    }

    #[test]
    fn handed_back_autoplay_uses_ordinary_failure_semantics() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        assert!(state.hand_back_autoplay());
        assert!(!state.autoplay_driver_active());
        assert!(state.autoplay_session_active());
        state.steps = vec![TutorialStep {
            command: Some("anvil check ../outside.ts".to_string()),
            effect: Some(CommandEffect::ReadOnly),
            ..TutorialStep::default()
        }];

        assert!(!state.execute_current_command("anvil check ../outside.ts"));

        assert!(state.autoplay_failure().is_none());
        assert!(state.current_step_failed());
    }

    fn autoplay_check_output(root: &std::path::Path, command: &str) -> CommandOutput {
        let target = command
            .strip_prefix("anvil check ")
            .expect("structured check command");
        assert!(!target.contains(char::is_whitespace));
        let target = resolve_working_path(root, std::path::Path::new(target))
            .expect("contained check target");
        let source = std::fs::read_to_string(target).expect("check target");
        let mut findings = Vec::new();
        if source.contains(": any") {
            findings.push("AP-003");
        }
        if source.contains("@ts-ignore") {
            findings.push("AP-004");
        }
        CommandOutput {
            stdout: findings.join("\n"),
            stderr: String::new(),
            success: true,
            exit_code: Some(0),
        }
    }

    fn autoplay_snapshot(seq: u64) -> anvil_kernel_types::EngineEvent {
        anvil_kernel_types::EngineEvent {
            event_type: anvil_kernel_types::EventType::Snapshot,
            seq,
            timestamp: "now".to_string(),
            engine: anvil_kernel_types::EngineId::Rust,
            payload: anvil_kernel_types::EventPayload::Snapshot {
                node_count: 1,
                edge_count: 0,
                files_watched: 1,
                changed_path: None,
            },
        }
    }

    #[test]
    fn autoplay_full_state_executes_checks_edits_watches_and_completes() {
        use std::collections::VecDeque;

        let root = tempfile::tempdir().expect("root");
        std::fs::create_dir(root.path().join("src")).expect("src");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay");
        let fixture = state.steps[1]
            .seed_template
            .as_deref()
            .expect("pinned fixture seed");
        std::fs::write(root.path().join("src/app.ts"), fixture).expect("fixture");
        let mut commands = Vec::new();

        for expected_step in [0, 2] {
            assert_eq!(state.current_step, expected_step);
            let command = state.steps[state.current_step]
                .command
                .clone()
                .expect("structured check step");
            commands.push(command.clone());
            state.consume_autoplay_output(autoplay_check_output(root.path(), &command));
            state.reveal_tick();
            if expected_step == 0 {
                while state.current_step == 1 {
                    state.reveal_tick();
                }
                let repaired =
                    std::fs::read_to_string(root.path().join("src/app.ts")).expect("repair");
                assert!(!repaired.contains(": any"));
                assert!(!repaired.contains("@ts-ignore"));
            }
        }

        assert_eq!(state.current_step, 3);
        state.reveal_tick();
        state.reveal_tick();
        assert!(state.wants_watch_demo);

        let data = crate::surfaces::watch::WatchData {
            status: crate::surfaces::watch::WatchStatus::Idle,
            queue: VecDeque::new(),
            history: Vec::new(),
            stats: crate::surfaces::watch::WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
            daemon_fallback_notice: None,
        };
        let mut watch = watch_demo::WatchDemoState::new(data);
        let initial = watch.autoplay_engine_event(&autoplay_snapshot(1));
        assert_eq!(initial, watch_demo::WatchDemoOutcome::Continue);
        state.apply_watch_demo_outcome(initial);
        assert_eq!(state.current_step, 3);
        state.apply_watch_demo_outcome(watch_demo::WatchDemoOutcome::HandBack);
        assert_eq!(state.current_step, 3);
        let target = root.path().join("src/app.ts");
        let mut edited = std::fs::read_to_string(&target).expect("post-repair source");
        edited.push_str("\n// watch cycle edit\n");
        std::fs::write(&target, edited).expect("watch edit");
        let outcome = watch.autoplay_engine_event(&autoplay_snapshot(2));
        state.apply_watch_demo_outcome(outcome);
        state.apply_watch_demo_outcome(outcome);

        assert_eq!(
            commands,
            ["anvil check src/app.ts", "anvil check src/app.ts"]
        );
        assert_eq!(watch.snapshot_count, 2);
        assert!(
            std::fs::read_to_string(target)
                .expect("watched source")
                .contains("watch cycle edit")
        );
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps.iter().all(|step| step.completed));
    }

    #[test]
    fn autoplay_editor_ghost_types_repaired_fixture_then_advances() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.current_step = 1;

        for _ in 0..100 {
            state.reveal_tick();
            if state.current_step > 1 {
                break;
            }
        }

        let repaired = std::fs::read_to_string(root.path().join("src/app.ts")).expect("repair");
        assert!(
            !repaired.contains(": any"),
            "AP-003 must be repaired: {repaired}"
        );
        assert!(
            !repaired.contains("@ts-ignore"),
            "AP-004 must be repaired: {repaired}"
        );
        assert!(repaired.contains("name: string"));
        assert!(state.current_step > 1);
    }

    #[cfg(unix)]
    #[test]
    fn editor_save_rejects_parent_symlink_swap() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::create_dir(root.path().join("nested")).expect("nested");
        let mut state = TutorialState::new_autoplay_in(root.path()).expect("autoplay root");
        state.steps[0] = TutorialStep {
            edit_target: Some("nested/file.ts".to_string()),
            seed_template: Some("safe".to_string()),
            ..TutorialStep::default()
        };
        state.open_step_editor();
        std::fs::rename(root.path().join("nested"), root.path().join("old")).expect("rename");
        symlink(outside.path(), root.path().join("nested")).expect("swap");

        let error = state
            .save_step_editor()
            .expect_err("escape must fail closed");

        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(!outside.path().join("file.ts").exists());
    }

    #[test]
    fn path_navigation() {
        let mut state = TutorialState::new();

        state.handle_key(Action::Down);
        assert_eq!(state.path_selected, 1);

        state.handle_key(Action::Down);
        assert_eq!(state.path_selected, 2);

        state.handle_key(Action::Up);
        assert_eq!(state.path_selected, 1);

        state.handle_key(Action::Up);
        assert_eq!(state.path_selected, 0);

        state.handle_key(Action::Up); // at min
        assert_eq!(state.path_selected, 0);
    }

    #[test]
    fn step_progression_informational() {
        // LAUNCH-014: ProtectionLoop step 0 is "anvil's protection
        // loop in 60 seconds" — no command — so Select advances it.
        // (Was the Policy "Introduction" step before LAUNCH-014
        // reordered the default path; the assertion is identical
        // because both are informational.)
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose ProtectionLoop
        let total_steps = state.steps.len();
        assert!(total_steps > 1);

        state.handle_key(Action::Select); // advance informational step 0
        assert_eq!(state.current_step, 1);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn completing_all_plain_steps_transitions_to_complete() {
        let total = 4;
        let mut state = state_with_plain_steps(total);

        for _ in 0..total {
            state.handle_key(Action::Select);
        }

        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    #[test]
    fn esc_from_default_path_returns_to_picker() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose default ProtectionLoop
        assert_eq!(state.phase, TutorialPhase::Running);

        state.handle_key(Action::Back);
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.wants_back);
    }

    #[test]
    fn complete_returns_to_path_select() {
        let total = 3;
        let mut state = state_with_plain_steps(total);

        for _ in 0..total {
            state.handle_key(Action::Select);
        }
        assert_eq!(state.phase, TutorialPhase::Complete);

        state.handle_key(Action::Select); // return to path select
        assert_eq!(state.phase, TutorialPhase::PathSelect);
    }

    #[test]
    fn esc_from_running_returns_to_picker_without_requesting_exit() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select);
        assert_eq!(state.phase, TutorialPhase::Running);

        state.handle_key(Action::Back);

        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.wants_back);
        assert!(!state.should_quit);
    }

    #[test]
    fn esc_from_failed_and_static_running_states_returns_to_picker() {
        let mut failed = state_with_command_step("exit 1");
        select_and_run(&mut failed);
        assert!(failed.current_step_failed());

        failed.handle_key(Action::Back);

        assert_eq!(failed.phase, TutorialPhase::PathSelect);
        assert!(!failed.wants_back);

        let mut static_mode = state_with_command_step("echo test");
        static_mode.enable_static_mode();

        static_mode.handle_key(Action::Back);

        assert_eq!(static_mode.phase, TutorialPhase::PathSelect);
        assert!(!static_mode.wants_back);
    }

    #[test]
    fn esc_from_complete_returns_to_picker_without_requesting_exit() {
        let mut state = state_with_plain_steps(1);
        state.handle_key(Action::Select);
        assert_eq!(state.phase, TutorialPhase::Complete);

        state.handle_key(Action::Back);

        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.wants_back);
        assert!(!state.should_quit);
    }

    #[test]
    fn tutorial_escape_help_matches_navigation_and_exit_behaviour() {
        let mut state = TutorialState::new();
        assert_eq!(
            <TutorialState as crate::surface::Surface>::help_text(&state),
            "j/k navigate  enter select  esc close tutorial  q quit"
        );

        state.handle_key(Action::Select);
        assert!(
            <TutorialState as crate::surface::Surface>::help_text(&state).contains("esc paths")
        );

        state.phase = TutorialPhase::Complete;
        assert_eq!(
            <TutorialState as crate::surface::Surface>::help_text(&state),
            "enter choose another  esc paths  q quit"
        );
    }

    #[test]
    fn esc_from_non_default_path_returns_to_picker() {
        let mut state = TutorialState::new();

        // Path order: ProtectionLoop(0), DeveloperAcceleration(1), Policy(2),
        // Architecture(3), .... Press Down three times to land on Architecture
        // so the assertion still pins running-phase exit semantics rather than
        // path selection.
        state.handle_key(Action::Down);
        state.handle_key(Action::Down);
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen_path, Some(TutorialPath::Architecture));

        // Esc returns to the path picker without requesting surface exit.
        state.handle_key(Action::Back);
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.wants_back);
    }

    #[test]
    fn quit_from_any_phase() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);

        let mut state = TutorialState::new();
        state.handle_key(Action::Select);
        state.should_quit = false;
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn path_labels() {
        assert_eq!(
            TutorialPath::ProtectionLoop.label(),
            "anvil's protection loop"
        );
        assert_eq!(TutorialPath::Policy.label(), "Policy checks");
        assert_eq!(TutorialPath::Architecture.label(), "Boundary findings");
        assert_eq!(TutorialPath::Drift.label(), "Configuration drift");
        assert_eq!(TutorialPath::CI.label(), "CI gate integration");
    }

    #[test]
    fn protection_loop_picker_copy_promises_verification_not_activation() {
        let description = TutorialPath::ProtectionLoop.description().to_lowercase();

        assert!(description.contains("verify"));
        assert!(
            !description.contains("activate in this repo"),
            "the picker must not promise mutation for a read-only walkthrough"
        );
    }

    // --- LAUNCH-014: protection-loop path copy invariants ---

    #[test]
    fn protection_loop_path_is_default_first_path() {
        // The Enter key on a fresh tutorial must land the user on
        // the ProtectionLoop walk, not the Policy taxonomy. This is
        // the load-bearing invariant of LAUNCH-014.
        let state = TutorialState::new();
        assert_eq!(state.paths.first(), Some(&TutorialPath::ProtectionLoop));
        assert_eq!(state.path_selected, 0);
    }

    #[test]
    fn protection_loop_copy_uses_activation_state_vocabulary() {
        // The path body must reference the LAUNCH-008 activation
        // state literals so users recognise them when `anvil status
        // --verify` prints one. This pin protects the cross-surface
        // vocabulary contract.
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::ProtectionLoop);
        let body: String = state
            .steps
            .iter()
            .map(|s| format!("{}\n{}\n{}", s.title, s.description, s.instruction))
            .collect::<Vec<_>>()
            .join("\n");

        for state_word in [
            "protecting",
            "ready_restart_required",
            "watching",
            "needs_action",
            "unsupported",
        ] {
            assert!(
                body.contains(state_word),
                "ProtectionLoop copy must reference state `{state_word}` so it stays \
                 vocabulary-aligned with `anvil start --verify` / LAUNCH-008. body:\n{body}"
            );
        }
    }

    #[test]
    fn protection_loop_copy_does_not_claim_pre_write_protection() {
        // The tutorial does not have activation evidence, so its copy
        // must not promise pre-write protection or call the user's
        // repo "protected". The final step points at `anvil start
        // --verify` instead — the only surface that produces a
        // literal `ProtectionState`.
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::ProtectionLoop);
        let body: String = state
            .steps
            .iter()
            .map(|s| format!("{}\n{}\n{}", s.title, s.description, s.instruction))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();

        // Allow the literal state word `protecting` (referenced in
        // the vocabulary explainer) but reject phrasings that claim
        // present-tense protection of the user's repo. This pin
        // would catch copy edits like "you are now protected" or
        // "pre-write validation enabled".
        for forbidden in [
            "you are now protected",
            "you're now protected",
            "your repo is protected",
            "pre-write validation enabled",
            "pre-write validation active",
            "anvil is now intercepting",
        ] {
            assert!(
                !body.contains(forbidden),
                "ProtectionLoop copy must not include `{forbidden}` — only \
                 `anvil start --verify` is allowed to produce that claim. body:\n{body}"
            );
        }

        // The final step must point at `anvil start --verify` so the
        // user gets a real evidence-backed answer instead of trusting
        // the tutorial's word.
        assert!(
            body.contains("anvil start --verify"),
            "final ProtectionLoop step must direct users at `anvil start --verify`, body:\n{body}"
        );
    }

    #[test]
    fn no_path_claims_pre_write_protection() {
        // G-05: the LAUNCH-014 honesty pins only covered ProtectionLoop,
        // so the four legacy paths (Policy / Architecture / Drift / CI)
        // could re-introduce a present-tense "you are now protected"
        // claim without any test noticing. Extend the forbidden-phrase
        // guard across every path so copy drift on any of them fails CI.
        // Only `anvil start --verify` may produce that claim.
        for path in [
            TutorialPath::ProtectionLoop,
            TutorialPath::DeveloperAcceleration,
            TutorialPath::Policy,
            TutorialPath::Architecture,
            TutorialPath::Drift,
            TutorialPath::CI,
        ] {
            let mut state = TutorialState::new();
            state.load_steps(path);
            let body: String = state
                .steps
                .iter()
                .map(|s| format!("{}\n{}\n{}", s.title, s.description, s.instruction))
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();

            for forbidden in [
                "you are now protected",
                "you're now protected",
                "your repo is protected",
                "pre-write validation enabled",
                "pre-write validation active",
                "anvil is now intercepting",
            ] {
                assert!(
                    !body.contains(forbidden),
                    "{path:?} copy must not include `{forbidden}` — only \
                     `anvil start --verify` is allowed to produce that claim. body:\n{body}"
                );
            }
        }
    }

    #[test]
    fn protection_loop_round_trips_through_label() {
        // The progress-file label round-trip must work for the new
        // path so completed-state checkmarks survive a process
        // restart. (Mirrors `path_labels_round_trip` for the
        // existing four paths.)
        let path = TutorialPath::ProtectionLoop;
        assert_eq!(TutorialPath::from_label(path.label()), Some(path));
    }

    #[test]
    fn all_paths_round_trip_through_label() {
        // Every path's label must round-trip so progress-file checkmarks
        // survive a restart — including the developer-acceleration path.
        for path in [
            TutorialPath::ProtectionLoop,
            TutorialPath::DeveloperAcceleration,
            TutorialPath::Policy,
            TutorialPath::Architecture,
            TutorialPath::Drift,
            TutorialPath::CI,
        ] {
            assert_eq!(
                TutorialPath::from_label(path.label()),
                Some(path),
                "label round-trip failed for {path:?}"
            );
        }
    }

    // --- Command execution tests ---

    #[test]
    fn successful_command_step_advances() {
        let mut state = state_with_command_step("echo hello");
        assert_eq!(state.current_step, 0);

        select_and_run(&mut state);

        // Command succeeds — step is completed and phase moves to Complete
        // (only one step in this state)
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        let output = state.steps[0]
            .output
            .as_ref()
            .expect("output should be present");
        assert!(output.success);
    }

    #[test]
    fn failed_command_step_stays_on_step() {
        // Use a command that will always fail with exit code 1.
        let mut state = state_with_command_step("exit 1");

        select_and_run(&mut state);

        // Command fails — step stays current and is not completed.
        assert_eq!(state.current_step, 0);
        assert!(!state.steps[0].completed);
        assert_eq!(state.phase, TutorialPhase::Running);
        let output = state.steps[0]
            .output
            .as_ref()
            .expect("output should be present");
        assert!(!output.success);
    }

    #[test]
    fn failed_command_help_text_shows_retry_skip() {
        let mut state = state_with_command_step("exit 1");
        select_and_run(&mut state); // executes and fails

        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(help, "r retry  s skip  esc paths  q quit");
    }

    #[test]
    fn retry_after_failure_re_executes_command() {
        // First run fails; second run succeeds because we swap to "echo ok".
        // We can't swap the command at runtime, so test retry with a succeeding command:
        // Use a command that fails first time... but since we can't vary behaviour
        // per call, test that retry with a succeeding command advances.
        // This verifies the retry path clears output and re-executes.
        let mut state = state_with_command_step("echo retry_test");

        // Simulate failure by injecting failed output directly.
        state.steps[0].output = Some(CommandOutput {
            stdout: String::new(),
            stderr: "simulated failure".to_string(),
            success: false,
            exit_code: Some(1),
        });

        // Verify we're in the "failed" state.
        assert!(state.current_step_failed());

        // Press 'r' to retry — the actual command is "echo retry_test" which succeeds.
        state.handle_key(Action::Character('r'));

        // Should advance past the step.
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn skip_after_failure_advances_without_re_running() {
        let mut state = state_with_command_step("exit 1");
        select_and_run(&mut state); // fails

        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s')); // skip

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn esc_from_failed_command_returns_to_picker() {
        let mut state = state_with_command_step("exit 1");
        select_and_run(&mut state); // fails

        state.handle_key(Action::Back);

        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.wants_back);
    }

    #[test]
    fn toggle_skips_command_step_without_executing() {
        // WOW-001: space is the skip-without-running escape hatch (CIB-165
        // consent posture). It advances past a command step but must never
        // execute the command — execution stays exclusively on Enter.
        let mut state = state_with_command_step("echo should_not_run");
        state.handle_key(Action::Toggle);

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert!(
            state.steps[0].output.is_none(),
            "space must not execute the command"
        );
    }

    // --- WOW-001: command-step evidence affordance ---

    #[test]
    fn command_step_help_text_names_run_and_skip() {
        // Before pressing Enter on a command step, the footer must say that
        // Enter runs a command and that space skips without running it.
        let state = state_with_command_step("echo hello");
        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(
            help,
            "enter run command  space skip without running  esc paths  q quit"
        );
    }

    #[test]
    fn informational_step_help_text_says_next() {
        // Informational steps must not claim Enter "runs" anything.
        let state = state_with_plain_steps(1);
        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(help, "enter next  space next  esc paths  q quit");
    }

    #[test]
    fn command_step_help_text_with_fix_available() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        // Policy path: load steps and jump to the first command step.
        state.handle_key(Action::Down);
        state.handle_key(Action::Down);
        state.handle_key(Action::Select);
        assert_eq!(state.chosen_path, Some(TutorialPath::Policy));
        state.current_step = 1; // "Create Policy Directory" — a command step
        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(
            help,
            "enter run command  space skip without running  f fix  esc paths  q quit"
        );
    }

    #[test]
    fn every_command_step_declares_an_effect() {
        // WOW-001: the run/read-only badge is only honest if every command
        // step declares its effect where the command is authored.
        for (path, steps) in [
            (TutorialPath::ProtectionLoop, paths::protection_loop_steps()),
            (
                TutorialPath::DeveloperAcceleration,
                paths::developer_acceleration_steps(),
            ),
            (TutorialPath::Policy, paths::policy_steps()),
            (TutorialPath::Architecture, paths::architecture_steps()),
            (TutorialPath::Drift, paths::drift_steps()),
            (TutorialPath::CI, paths::ci_steps()),
        ] {
            for step in &steps {
                assert_eq!(
                    step.command.is_some(),
                    step.effect.is_some(),
                    "{path:?} step '{}' must declare an effect iff it has a command",
                    step.title
                );
            }
        }
    }

    #[test]
    fn toggle_advances_informational_step() {
        let mut state = state_with_plain_steps(2);
        state.handle_key(Action::Toggle);

        assert_eq!(state.current_step, 1);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn informational_step_advances_without_executing() {
        let mut state = state_with_plain_steps(2);
        state.handle_key(Action::Select);

        assert_eq!(state.current_step, 1);
        assert!(state.steps[0].completed);
        assert!(state.steps[0].output.is_none());
    }

    // --- WOW-002: typed-command execution presentation ---

    #[test]
    fn select_on_command_step_starts_reveal_without_executing() {
        let mut state = state_with_command_step("echo hello");
        state.handle_key(Action::Select);

        assert!(state.is_revealing());
        assert_eq!(state.reveal.as_ref().unwrap().visible(), "");
        assert!(
            state.steps[0].output.is_none(),
            "command must not run until the reveal completes"
        );
        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.current_step, 0);
    }

    #[test]
    fn reveal_ticks_show_fixed_prefixes_then_execute() {
        // "echo hello" is 10 chars; at 3 chars/tick the prefixes are
        // deterministic and the 4th tick completes and executes.
        let mut state = state_with_command_step("echo hello");
        state.handle_key(Action::Select);

        state.reveal_tick();
        assert_eq!(state.reveal.as_ref().unwrap().visible(), "ech");
        state.reveal_tick();
        assert_eq!(state.reveal.as_ref().unwrap().visible(), "echo h");
        state.reveal_tick();
        assert_eq!(state.reveal.as_ref().unwrap().visible(), "echo hell");
        state.reveal_tick(); // clamps to 10 → complete → executes

        assert!(!state.is_revealing());
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert!(state.steps[0].output.as_ref().unwrap().success);
    }

    #[test]
    fn any_key_completes_reveal_instantly_and_executes() {
        let mut state = state_with_command_step("echo hello");
        state.handle_key(Action::Select);
        assert!(state.is_revealing());

        // An arbitrary key — not Enter — fast-forwards and runs the command.
        state.handle_key(Action::Character('x'));

        assert!(!state.is_revealing());
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].output.as_ref().unwrap().success);
    }

    #[test]
    fn esc_during_reveal_cancels_without_executing() {
        // The reveal window is the user's last chance to back out before
        // the run (CIB-165): Esc aborts the pending command entirely.
        let mut state = state_with_command_step("echo should_not_run");
        state.handle_key(Action::Select);
        assert!(state.is_revealing());

        state.handle_key(Action::Back);

        assert!(!state.is_revealing());
        assert!(state.steps[0].output.is_none(), "command must not run");
        assert!(!state.steps[0].completed);
        assert_eq!(state.phase, TutorialPhase::Running);
        assert!(!state.wants_back, "esc mid-reveal cancels, not backs out");
    }

    #[test]
    fn quit_during_reveal_quits_without_executing() {
        // q must never become a "run it now" key: it aborts the pending
        // command and quits.
        let mut state = state_with_command_step("echo should_not_run");
        state.handle_key(Action::Select);
        assert!(state.is_revealing());

        state.handle_key(Action::Quit);

        assert!(!state.is_revealing());
        assert!(state.steps[0].output.is_none(), "command must not run");
        assert!(state.should_quit);
    }

    #[test]
    fn reveal_failed_command_lands_in_retry_skip_state() {
        // The failed-step contract is unchanged: a command that fails after
        // the reveal leaves the step in the retry/skip state.
        let mut state = state_with_command_step("exit 1");
        select_and_run(&mut state);

        assert!(!state.is_revealing());
        assert!(state.current_step_failed());
        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(help, "r retry  s skip  esc paths  q quit");
    }

    #[test]
    fn static_mode_select_does_not_reveal() {
        let mut state = state_with_command_step("echo should_not_run");
        state.enable_static_mode();

        state.handle_key(Action::Select);

        assert!(!state.is_revealing());
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].output.is_none());
    }

    #[test]
    fn retry_after_failure_executes_without_reveal() {
        // Failed-step retry behaviour is unchanged by WOW-002: 'r'
        // re-executes immediately, no animation.
        let mut state = state_with_command_step("echo retry_direct");
        state.steps[0].output = Some(CommandOutput {
            stdout: String::new(),
            stderr: "simulated failure".to_string(),
            success: false,
            exit_code: Some(1),
        });
        assert!(state.current_step_failed());

        state.handle_key(Action::Character('r'));

        assert!(!state.is_revealing());
        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    #[test]
    fn reveal_tick_is_noop_without_reveal() {
        let mut state = state_with_command_step("echo hello");
        state.reveal_tick();
        assert!(!state.is_revealing());
        assert!(state.steps[0].output.is_none());
        assert_eq!(state.current_step, 0);
    }

    #[test]
    fn revealing_help_text_offers_fast_forward() {
        let mut state = state_with_command_step("echo hello");
        state.handle_key(Action::Select);
        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(help, "any key run now  esc cancel  q quit");
    }

    #[test]
    fn reveal_cleared_on_reset() {
        let mut state = state_with_command_step("echo hello");
        state.handle_key(Action::Select);
        assert!(state.is_revealing());

        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert!(!state.is_revealing());
    }

    // --- Scan results threading tests ---

    use discovery::{Finding, FindingSeverity, FindingSource, ScanResults};

    fn make_scan_results() -> ScanResults {
        ScanResults {
            findings: vec![
                Finding {
                    file: "src/main.rs".to_string(),
                    line: Some(10),
                    severity: FindingSeverity::Error,
                    source: FindingSource::AntiPattern,
                    title: "anti-pattern".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: Some("AP-003".to_string()),
                },
                Finding {
                    file: "src/lib.rs".to_string(),
                    line: Some(20),
                    severity: FindingSeverity::Warning,
                    source: FindingSource::Architecture,
                    title: "boundary".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: None,
                },
            ],
            files_scanned: 100,
            duration_ms: 250,
            truncated: false,
            files_skipped_by_ignore: 0,
            is_showcase: false,
        }
    }

    #[test]
    fn scan_results_default_none() {
        let state = TutorialState::new();
        assert!(state.scan_results.is_none());
        assert!(state.domain_findings.is_none());
    }

    #[test]
    fn set_scan_results_stores_results() {
        let mut state = TutorialState::new();
        let results = make_scan_results();
        state.set_scan_results(results);
        assert!(state.scan_results.is_some());
        assert_eq!(state.scan_results.as_ref().unwrap().findings.len(), 2);
    }

    #[test]
    fn load_steps_computes_domain_findings() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.load_steps(TutorialPath::Policy);

        assert!(state.domain_findings.is_some());
        let domain = state.domain_findings.as_ref().unwrap();
        // Policy gets AntiPattern + Secret, so only the AntiPattern finding
        assert_eq!(domain.findings.len(), 1);
        assert_eq!(domain.findings[0].source, FindingSource::AntiPattern);
    }

    #[test]
    fn load_steps_without_scan_results_leaves_domain_none() {
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Architecture);
        assert!(state.domain_findings.is_none());
    }

    #[test]
    fn reset_clears_scan_and_domain_findings() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.load_steps(TutorialPath::Policy);
        assert!(state.scan_results.is_some());
        assert!(state.domain_findings.is_some());

        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert!(state.scan_results.is_none());
        assert!(state.domain_findings.is_none());
    }

    // --- WOW-003: personalized path picker ---

    #[test]
    fn picker_finding_count_none_without_scan() {
        let state = TutorialState::new();
        for path in &state.paths.clone() {
            assert_eq!(state.picker_finding_count(*path), None);
        }
    }

    #[test]
    fn picker_finding_count_per_domain() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        // make_scan_results: 1 AntiPattern (policy domain) + 1 Architecture.
        assert_eq!(state.picker_finding_count(TutorialPath::Policy), Some(1));
        assert_eq!(
            state.picker_finding_count(TutorialPath::Architecture),
            Some(1)
        );
        // Cross-cutting domains see all findings.
        assert_eq!(state.picker_finding_count(TutorialPath::Drift), Some(2));
        assert_eq!(
            state.picker_finding_count(TutorialPath::ProtectionLoop),
            Some(2)
        );
    }

    #[test]
    fn picker_finding_count_zero_findings_is_none() {
        // Clean repo: scan ran but found nothing — fall back to plain copy.
        let mut state = TutorialState::new();
        state.set_scan_results(ScanResults {
            findings: Vec::new(),
            files_scanned: 42,
            duration_ms: 10,
            truncated: false,
            files_skipped_by_ignore: 0,
            is_showcase: false,
        });
        for path in &state.paths.clone() {
            assert_eq!(state.picker_finding_count(*path), None);
        }
    }

    #[test]
    fn picker_finding_count_showcase_is_none() {
        // CIB-170: showcase examples must never be presented as real
        // findings — no counts on the picker.
        let mut state = TutorialState::new();
        let mut results = make_scan_results();
        results.is_showcase = true;
        state.set_scan_results(results);
        for path in &state.paths.clone() {
            assert_eq!(state.picker_finding_count(*path), None);
        }
    }

    // --- WOW-004: completion findings delta ---

    /// Real (non-showcase) scan results with `n` policy-domain findings.
    fn policy_scan_results(n: usize) -> ScanResults {
        ScanResults {
            findings: (0..n)
                .map(|i| Finding {
                    file: format!("src/file{i}.rs"),
                    line: Some(i + 1),
                    severity: FindingSeverity::Warning,
                    source: FindingSource::AntiPattern,
                    title: "anti-pattern".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: None,
                })
                .collect(),
            files_scanned: 10,
            duration_ms: 5,
            truncated: false,
            files_skipped_by_ignore: 0,
            is_showcase: false,
        }
    }

    /// Drive a plain-steps state to completion.
    fn complete_all_steps(state: &mut TutorialState) {
        while state.phase == TutorialPhase::Running {
            state.handle_key(Action::Select);
        }
        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    /// Build a Policy-path state through the real `load_steps` seam (so the
    /// WOW-004 baseline is captured), then swap in harmless plain steps so
    /// completing the walk never executes real commands.
    fn delta_state(opening: ScanResults) -> TutorialState {
        let mut state = TutorialState::new();
        state.set_scan_results(opening);
        state.load_steps(TutorialPath::Policy);
        state.steps = vec![TutorialStep {
            title: "Step".to_string(),
            description: "desc".to_string(),
            instruction: "enter".to_string(),
            ..TutorialStep::default()
        }];
        state.current_step = 0;
        state
    }

    #[test]
    fn completion_delta_improved() {
        let mut state = delta_state(policy_scan_results(3));
        state.set_completion_rescan(|| Some(policy_scan_results(1)));

        complete_all_steps(&mut state);

        assert_eq!(
            state.completion_delta,
            Some(FindingsDelta {
                before: 3,
                after: 1,
                after_in_test_paths: 0
            })
        );
    }

    #[test]
    fn completion_delta_unchanged() {
        let mut state = delta_state(policy_scan_results(2));
        state.set_completion_rescan(|| Some(policy_scan_results(2)));

        complete_all_steps(&mut state);

        assert_eq!(
            state.completion_delta,
            Some(FindingsDelta {
                before: 2,
                after: 2,
                after_in_test_paths: 0
            })
        );
    }

    #[test]
    fn completion_delta_regressed() {
        let mut state = delta_state(policy_scan_results(1));
        state.set_completion_rescan(|| Some(policy_scan_results(4)));

        complete_all_steps(&mut state);

        assert_eq!(
            state.completion_delta,
            Some(FindingsDelta {
                before: 1,
                after: 4,
                after_in_test_paths: 0
            })
        );
    }

    #[test]
    fn completion_delta_absent_without_rescan_hook() {
        let mut state = delta_state(policy_scan_results(2));

        complete_all_steps(&mut state);

        assert_eq!(state.completion_delta, None);
    }

    #[test]
    fn completion_delta_absent_without_opening_scan() {
        let mut state = state_with_plain_steps(1);
        state.set_completion_rescan(|| Some(policy_scan_results(0)));

        complete_all_steps(&mut state);

        assert_eq!(state.completion_delta, None);
    }

    #[test]
    fn completion_delta_baseline_survives_fix_pruning() {
        // The welcome fix flow prunes scan_results in place when a finding
        // is fixed mid-walk. The delta's 'before' must stay the opening
        // count, or an applied fix erases its own win from the summary.
        let mut state = delta_state(policy_scan_results(3));
        state.set_completion_rescan(|| Some(policy_scan_results(2)));

        // Simulate remove_fixed_finding: one finding pruned mid-walk.
        state.scan_results.as_mut().unwrap().findings.pop();

        complete_all_steps(&mut state);

        assert_eq!(
            state.completion_delta,
            Some(FindingsDelta {
                before: 3,
                after: 2,
                after_in_test_paths: 0
            })
        );
    }

    #[test]
    fn completion_delta_absent_when_opening_scan_is_showcase() {
        // CIB-170: showcase counts are examples, not a real baseline.
        let mut results = policy_scan_results(2);
        results.is_showcase = true;
        let mut state = delta_state(results);
        state.set_completion_rescan(|| Some(policy_scan_results(0)));

        complete_all_steps(&mut state);

        assert_eq!(state.completion_delta, None);
    }

    /// CIB-247: the re-scan carries the test/fixture split so the completion
    /// screen can say where a secret-noisy count actually lives. The total is
    /// untouched — the split explains it, it does not discount it.
    #[test]
    fn completion_delta_carries_the_test_path_split() {
        let mut state = delta_state(policy_scan_results(2));
        state.set_completion_rescan(|| {
            let mut results = policy_scan_results(1);
            results.findings.push(Finding {
                file: "tests/fixtures/creds.rs".to_string(),
                line: Some(1),
                severity: FindingSeverity::Error,
                source: FindingSource::Secret,
                title: "hardcoded secret".to_string(),
                message: "test".to_string(),
                suggestion: "fix".to_string(),
                warning_id: None,
            });
            Some(results)
        });

        complete_all_steps(&mut state);

        assert_eq!(
            state.completion_delta,
            Some(FindingsDelta {
                before: 2,
                after: 2,
                after_in_test_paths: 1
            })
        );
    }

    #[test]
    fn completion_delta_absent_when_rescan_fails() {
        let mut state = delta_state(policy_scan_results(2));
        state.set_completion_rescan(|| None);

        complete_all_steps(&mut state);

        assert_eq!(state.completion_delta, None);
    }

    #[test]
    fn completion_delta_cleared_when_choosing_another_path() {
        let mut state = delta_state(policy_scan_results(1));
        state.set_completion_rescan(|| Some(policy_scan_results(0)));
        complete_all_steps(&mut state);
        assert!(state.completion_delta.is_some());

        state.handle_key(Action::Select); // back to path select
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert_eq!(state.completion_delta, None);
    }

    #[test]
    fn completion_delta_cleared_on_reset() {
        let mut state = delta_state(policy_scan_results(1));
        state.set_completion_rescan(|| Some(policy_scan_results(0)));
        complete_all_steps(&mut state);
        assert!(state.completion_delta.is_some());

        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert_eq!(state.completion_delta, None);
    }

    // --- Verification integration tests ---

    #[test]
    fn verify_pass_advances_step() {
        // "echo hello" succeeds and stdout contains "hello" — should advance.
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("hello".to_string()),
            "Output should contain hello.",
        );
        select_and_run(&mut state);

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert_eq!(state.steps[0].verify_result, Some(VerifyResult::Pass));
    }

    #[test]
    fn verify_fail_stays_on_step() {
        // "echo hello" succeeds but stdout does NOT contain "world" — should stay.
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        select_and_run(&mut state);

        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.current_step, 0);
        assert!(!state.steps[0].completed);
        assert!(state.current_step_failed());
        assert!(matches!(
            state.steps[0].verify_result,
            Some(VerifyResult::Fail(_))
        ));
    }

    #[test]
    fn verify_fail_then_skip_advances() {
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        select_and_run(&mut state); // verify fails
        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s')); // skip
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn verify_fail_then_retry_clears_result() {
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("hello".to_string()),
            "Output should contain hello.",
        );

        // Inject a failed verify state to simulate prior failure.
        state.steps[0].output = Some(CommandOutput {
            stdout: "nope".to_string(),
            stderr: String::new(),
            success: true,
            exit_code: Some(0),
        });
        state.steps[0].verify_result = Some(VerifyResult::Fail(
            "Output did not contain expected text: hello".to_string(),
        ));
        assert!(state.current_step_failed());

        // Retry — the actual "echo hello" command succeeds and contains "hello".
        state.handle_key(Action::Character('r'));

        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert_eq!(state.steps[0].verify_result, Some(VerifyResult::Pass));
    }

    // --- Static mode tests ---

    #[test]
    fn static_mode_defaults_to_false() {
        let state = TutorialState::new();
        assert!(!state.static_mode);
        assert!(state.static_notice.is_none());
    }

    #[test]
    fn enable_static_mode_sets_flag_and_notice() {
        let mut state = TutorialState::new();
        state.enable_static_mode();
        assert!(state.static_mode);
        assert_eq!(
            state.static_notice.as_deref(),
            Some("Interactive mode unavailable \u{2014} showing guided walkthrough.")
        );
    }

    /// CIB-248: an autoplay failure used to unwind out of the welcome loop as
    /// an `Err`, dropping the user to scrollback. Recovery must keep the
    /// session alive and hand them back to the picker with an explanation.
    #[test]
    fn autoplay_failure_recovers_to_the_path_picker_instead_of_exiting() {
        let mut state = TutorialState::new_autoplay();
        state.fail_autoplay("autoplay command failed: boom".to_string());
        assert!(state.autoplay_failure().is_some());

        state.recover_from_autoplay_failure("The hands-free demo stopped: boom.");

        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.autoplay_session_active());
        assert!(!state.autoplay_driver_active());
        // The failure is consumed by the recovery, so the welcome loop cannot
        // observe it again and treat it as fatal on the next tick.
        assert!(state.autoplay_failure().is_none());
        assert_eq!(
            state.resuming_notice.as_deref(),
            Some("The hands-free demo stopped: boom.")
        );
    }

    /// `start_autoplay` stashes the pre-demo scan context. Recovery must
    /// restore it, or the picker comes back without its per-domain counts.
    #[test]
    fn autoplay_recovery_restores_the_pre_demo_scan_context() {
        let mut state = TutorialState::new();
        state.set_scan_results(discovery::ScanResults::default());
        assert!(state.scan_results.is_some());

        state.start_autoplay();
        assert!(
            state.scan_results.is_none(),
            "the demo must not present the sandbox as the user's repo"
        );

        state.recover_from_autoplay_failure("demo stopped");

        assert!(
            state.scan_results.is_some(),
            "recovery must restore the user's own scan results"
        );
        assert!(state.autoplay_saved_context.is_none());
    }

    #[test]
    fn autoplay_recovery_restores_the_ordinary_working_root() {
        let workspace = tempfile::tempdir().expect("workspace");
        let canonical_workspace =
            canonicalize_working_path(workspace.path()).expect("canonical workspace");
        let sandbox = tempfile::tempdir().expect("autoplay sandbox");
        let mut state = TutorialState::new();
        state
            .bind_working_root(&canonical_workspace)
            .expect("bind ordinary root");

        state
            .start_autoplay_in(sandbox.path())
            .expect("start autoplay");
        drop(workspace);
        state.recover_from_autoplay_failure("demo stopped");

        assert_eq!(
            state.working_root.as_deref(),
            Some(canonical_workspace.as_path()),
            "teardown must restore the pre-demo containment boundary"
        );
        assert!(
            state
                .resolve_session_target("must-not-use-ambient-cwd")
                .is_err(),
            "a disappeared workspace must fail closed against its saved boundary"
        );
    }

    #[test]
    fn enable_static_mode_with_reason_overrides_notice() {
        let mut state = TutorialState::new();
        state.enable_static_mode_with_reason("watcher failed: inotify limit reached");
        assert!(state.static_mode);
        assert_eq!(
            state.static_notice.as_deref(),
            Some("watcher failed: inotify limit reached")
        );
    }

    #[test]
    fn static_mode_select_advances_command_step_without_executing() {
        let mut state = state_with_command_step("echo should_not_run");
        state.enable_static_mode();

        state.handle_key(Action::Select);

        // Step should advance without executing — no output stored.
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert!(state.steps[0].output.is_none());
    }

    #[test]
    fn static_mode_toggle_advances_command_step() {
        let mut state = state_with_command_step("echo should_not_run");
        state.enable_static_mode();

        state.handle_key(Action::Toggle);

        // In static mode, Toggle advances even command steps.
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
        assert!(state.steps[0].output.is_none());
    }

    #[test]
    fn static_mode_informational_steps_still_advance() {
        let mut state = state_with_plain_steps(3);
        state.enable_static_mode();

        state.handle_key(Action::Select);
        assert_eq!(state.current_step, 1);

        state.handle_key(Action::Select);
        assert_eq!(state.current_step, 2);

        state.handle_key(Action::Select);
        assert_eq!(state.phase, TutorialPhase::Complete);
    }

    #[test]
    fn static_mode_help_text_shows_simplified() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert_eq!(help, "enter next  esc paths  q quit");
    }

    #[test]
    fn static_mode_preserved_across_reset() {
        let mut state = TutorialState::new();
        state.enable_static_mode();

        <TutorialState as crate::surface::Surface>::reset(&mut state);

        assert!(state.static_mode);
        assert!(state.static_notice.is_some());
    }

    #[test]
    fn static_mode_current_step_failed_always_false() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        // In static mode, commands never execute, so current_step_failed()
        // should always return false.
        assert!(!state.current_step_failed());
    }

    #[test]
    fn static_mode_skip_still_works_defensively() {
        // Even though current_step_failed() is unreachable in static mode,
        // if output were injected (defensively), skip should still advance.
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        // Inject failure output to simulate an edge case.
        state.steps[0].output = Some(CommandOutput {
            stdout: String::new(),
            stderr: "simulated".to_string(),
            success: false,
            exit_code: Some(1),
        });
        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s'));
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);
    }

    #[test]
    fn static_mode_esc_from_running_returns_to_picker() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        state.handle_key(Action::Back);
        assert_eq!(state.phase, TutorialPhase::PathSelect);
        assert!(!state.wants_back);
    }

    #[test]
    fn static_mode_quit_from_running() {
        let mut state = state_with_command_step("echo test");
        state.enable_static_mode();

        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    // --- Progress persistence / resumption tests ---

    #[test]
    fn from_label_roundtrips() {
        for path in &[
            // LAUNCH-014: include the new ProtectionLoop default in
            // the round-trip pin so a label rename can't silently
            // break the resumption seam.
            TutorialPath::ProtectionLoop,
            TutorialPath::Policy,
            TutorialPath::Architecture,
            TutorialPath::Drift,
            TutorialPath::CI,
        ] {
            assert_eq!(TutorialPath::from_label(path.label()), Some(*path));
        }
        assert_eq!(TutorialPath::from_label("Nonexistent"), None);
    }

    #[test]
    fn from_label_accepts_legacy_labels() {
        // Pre-rename progress files still need to resume into the matching
        // enum variant after the labels were reframed for onboarding clarity.
        assert_eq!(
            TutorialPath::from_label("Policy"),
            Some(TutorialPath::Policy)
        );
        assert_eq!(
            TutorialPath::from_label("Architecture"),
            Some(TutorialPath::Architecture)
        );
        assert_eq!(TutorialPath::from_label("Drift"), Some(TutorialPath::Drift));
        assert_eq!(
            TutorialPath::from_label("CI Integration"),
            Some(TutorialPath::CI)
        );
    }

    #[test]
    fn set_completed_paths_stored() {
        let mut state = TutorialState::new();
        state.set_completed_paths(vec![TutorialPath::Policy, TutorialPath::Drift]);
        assert_eq!(state.completed_paths.len(), 2);
        assert!(state.completed_paths.contains(&TutorialPath::Policy));
        assert!(state.completed_paths.contains(&TutorialPath::Drift));
    }

    #[test]
    fn completed_paths_preserved_across_reset() {
        let mut state = TutorialState::new();
        state.set_completed_paths(vec![TutorialPath::Architecture]);

        <TutorialState as crate::surface::Surface>::reset(&mut state);

        assert_eq!(state.completed_paths, vec![TutorialPath::Architecture]);
    }

    #[test]
    fn resume_path_jumps_to_step() {
        let mut state = TutorialState::new();
        // Policy has 6 steps — provide a matching-length vec.
        let completed = vec![true, true, false, false, false, false];
        state.resume_path(TutorialPath::Policy, 2, &completed);

        assert_eq!(state.phase, TutorialPhase::Running);
        assert_eq!(state.chosen_path, Some(TutorialPath::Policy));
        assert_eq!(state.current_step, 2);
        assert!(state.steps[0].completed);
        assert!(state.steps[1].completed);
        assert!(!state.steps[2].completed);
    }

    #[test]
    fn resume_path_sets_notice() {
        let mut state = TutorialState::new();
        // Policy has 6 steps.
        state.resume_path(
            TutorialPath::Policy,
            2,
            &[true, true, false, false, false, false],
        );

        assert!(state.resuming_notice.is_some());
        let notice = state.resuming_notice.as_ref().unwrap();
        assert!(notice.contains("Resuming from step 3"));
    }

    #[test]
    fn resume_notice_cleared_on_advance() {
        let mut state = state_with_plain_steps(3);
        state.resuming_notice = Some("Resuming from step 2 of 3.".to_string());
        state.current_step = 1;
        state.steps[0].completed = true;

        state.handle_key(Action::Select); // advance step 1
        assert!(state.resuming_notice.is_none());
    }

    #[test]
    fn resume_clears_on_reset() {
        let mut state = TutorialState::new();
        // Drift has 6 steps.
        state.resume_path(
            TutorialPath::Drift,
            1,
            &[true, false, false, false, false, false],
        );
        assert!(state.resuming_notice.is_some());

        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert!(state.resuming_notice.is_none());
    }

    #[test]
    fn resume_stale_session_discarded() {
        let mut state = TutorialState::new();
        // Provide wrong-length steps_completed — simulates a stale session.
        state.resume_path(TutorialPath::CI, 2, &[true, true]);

        // Stale session discarded: starts at step 0 with no notice.
        assert_eq!(state.current_step, 0);
        assert!(state.resuming_notice.is_none());
        assert!(!state.steps[0].completed);
    }

    // --- File watcher integration tests ---

    fn state_with_watched_step(watch_path: &str) -> TutorialState {
        let dir = std::env::temp_dir().join("anvil_watch_test");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("marker.txt");

        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Watched Step".to_string(),
            description: "A step with file watching.".to_string(),
            instruction: "Create the target file.".to_string(),
            verify: Some(Verify::FileExists(target.to_string_lossy().to_string())),
            verify_hint: Some("File not found.".to_string()),
            watch_path: Some(watch_path.to_string()),
            ..TutorialStep::default()
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    #[test]
    fn handle_file_change_ignores_non_running_phase() {
        let mut state = TutorialState::new();
        // Phase is PathSelect, not Running.
        let advanced = state.handle_file_change(&[std::path::PathBuf::from("test.txt")]);
        assert!(!advanced);
    }

    #[test]
    fn handle_file_change_ignores_step_without_watch_path() {
        let mut state = state_with_plain_steps(2);
        let advanced = state.handle_file_change(&[std::path::PathBuf::from("test.txt")]);
        assert!(!advanced);
    }

    #[test]
    fn handle_file_change_ignores_irrelevant_paths() {
        let mut state = state_with_watched_step("/tmp/watched_dir");
        let advanced =
            state.handle_file_change(&[std::path::PathBuf::from("/other/unrelated.txt")]);
        assert!(!advanced);
        assert_eq!(state.current_step, 0);
    }

    #[test]
    fn handle_file_change_auto_verifies_file_exists() {
        let dir = std::env::temp_dir().join("anvil_watch_autotest");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("marker.txt");

        // Create the file so FileExists passes.
        std::fs::write(&target, "ok").unwrap();

        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Watched".to_string(),
            description: "desc".to_string(),
            instruction: "inst".to_string(),
            verify: Some(Verify::FileExists(target.to_string_lossy().to_string())),
            watch_path: Some(dir.to_string_lossy().to_string()),
            ..TutorialStep::default()
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);

        let changed = dir.join("marker.txt");
        let advanced = state.handle_file_change(&[changed]);
        assert!(advanced);
        assert_eq!(state.phase, TutorialPhase::Complete);

        // Clean up.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn handle_file_change_stays_when_verify_fails() {
        let dir = std::env::temp_dir().join("anvil_watch_failtest");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("nonexistent.txt");

        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Watched".to_string(),
            description: "desc".to_string(),
            instruction: "inst".to_string(),
            verify: Some(Verify::FileExists(target.to_string_lossy().to_string())),
            watch_path: Some(dir.to_string_lossy().to_string()),
            ..TutorialStep::default()
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);

        // Trigger with a file in the watched dir, but the verify target doesn't exist.
        let changed = dir.join("other.txt");
        let advanced = state.handle_file_change(&[changed]);
        assert!(!advanced);
        assert_eq!(state.current_step, 0);
        assert!(matches!(
            state.steps[0].verify_result,
            Some(VerifyResult::Fail(_))
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn handle_file_change_skipped_in_static_mode() {
        let mut state = state_with_watched_step("/tmp/watched_dir");
        state.enable_static_mode();

        let advanced =
            state.handle_file_change(&[std::path::PathBuf::from("/tmp/watched_dir/file.txt")]);
        assert!(!advanced);
    }

    // --- Fix key tests ---

    #[test]
    fn f_key_sets_pending_fix_when_fixable_domain_finding_present() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.handle_key(Action::Select); // choose Policy path
        assert!(state.domain_findings.is_some());
        assert!(!state.domain_findings.as_ref().unwrap().findings.is_empty());

        state.handle_key(Action::Character('f'));
        assert_eq!(
            state.pending_fix,
            Some(FixRequest::AntiPatternWarning {
                file: "src/main.rs".to_string(),
                line: 10,
                warning_id: "AP-003".to_string(),
            })
        );
    }

    #[test]
    fn f_key_no_op_without_domain_findings() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy path, no scan results
        assert!(state.domain_findings.is_none());

        state.handle_key(Action::Character('f'));
        assert!(state.pending_fix.is_none());
    }

    #[test]
    fn pending_fix_causes_should_quit_true() {
        let mut state = TutorialState::new();
        state.set_scan_results(make_scan_results());
        state.handle_key(Action::Select);
        assert!(!crate::surface::Surface::should_quit(&state));

        state.handle_key(Action::Character('f'));
        assert!(crate::surface::Surface::should_quit(&state));
    }

    #[test]
    fn reset_clears_pending_fix() {
        let mut state = TutorialState::new();
        state.pending_fix = Some(FixRequest::AntiPatternWarning {
            file: "src/main.rs".to_string(),
            line: 10,
            warning_id: "AP-003".to_string(),
        });
        <TutorialState as crate::surface::Surface>::reset(&mut state);
        assert!(state.pending_fix.is_none());
    }

    // --- NotificationSource impl ---

    #[test]
    fn notifications_empty_without_notices() {
        let state = TutorialState::new();
        assert!(state.notifications().is_empty());
    }

    #[test]
    fn notifications_include_static_notice_as_warning() {
        let mut state = TutorialState::new();
        state.enable_static_mode_with_reason("watcher unavailable");
        let notifications = state.notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].class, NotificationClass::Warning);
        assert_eq!(notifications[0].priority, NotificationPriority::High);
        assert_eq!(
            notifications[0]
                .context
                .as_ref()
                .and_then(|c| c.source.as_deref()),
            Some("tutorial")
        );
    }

    #[test]
    fn notifications_include_resume_notice_as_info() {
        let mut state = TutorialState::new();
        state.resume_path(
            TutorialPath::Policy,
            1,
            &[true, false, false, false, false, false],
        );
        let notifications = state.notifications();
        let resume = notifications
            .iter()
            .find(|n| n.title == "Tutorial resumed")
            .expect("resume notification present");
        assert_eq!(resume.class, NotificationClass::Info);
        assert_eq!(resume.priority, NotificationPriority::Normal);
    }

    #[test]
    fn notifications_include_failure_when_command_fails() {
        let mut state = state_with_command_step("exit 1");
        select_and_run(&mut state);
        let notifications = state.notifications();
        let failure = notifications
            .iter()
            .find(|n| n.class == NotificationClass::Failure)
            .expect("failure notification present");
        assert_eq!(failure.priority, NotificationPriority::High);
        assert_eq!(failure.title, "Tutorial step failed");
    }

    #[test]
    fn notifications_include_failure_when_verify_fails() {
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        select_and_run(&mut state);
        let notifications = state.notifications();
        assert!(
            notifications
                .iter()
                .any(|n| n.title == "Verification failed" && n.class == NotificationClass::Failure),
            "expected verification failure notification, got {notifications:?}"
        );
    }

    #[test]
    fn notifications_never_echo_stderr_contents() {
        // Security regression (CWE-209): failed-command notifications must
        // never embed the step's stderr, which frequently contains absolute
        // paths, credential-helper output, or $HOME/username fragments.
        let mut state = state_with_command_step("/bin/sh -c 'exit 7'");
        state.steps[0].output = Some(CommandOutput {
            stdout: String::new(),
            stderr: "/home/secret-user/work/tokens/.env: permission denied".to_string(),
            success: false,
            exit_code: Some(7),
        });
        let notifications = state.notifications();
        for n in &notifications {
            assert!(
                !n.message.contains("secret-user"),
                "notification leaked $HOME fragment: {:?}",
                n.message
            );
            assert!(
                !n.message.contains("/home/"),
                "notification leaked absolute path: {:?}",
                n.message
            );
            assert!(
                !n.message.contains("permission denied"),
                "notification leaked stderr text: {:?}",
                n.message
            );
        }
        // And we still report a failure — with the sanitised message.
        assert!(
            notifications
                .iter()
                .any(|n| n.title == "Tutorial step failed" && n.message.contains("exit code 7")),
            "expected sanitised failure notification, got {notifications:?}"
        );
    }

    #[test]
    fn notifications_suppressed_after_verify_fail_skip_complete() {
        // Adversarial F-002: after verify-fail -> skip -> phase=Complete,
        // advance_step() doesn't clear step.verify_result, but notifications()
        // must not re-surface the stale failure because the tutorial is done.
        let mut state = state_with_verified_step(
            "echo hello",
            Verify::OutputContains("world".to_string()),
            "Output should contain world.",
        );
        select_and_run(&mut state); // command succeeds, verify fails
        assert!(state.current_step_failed());

        state.handle_key(Action::Character('s')); // skip
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(state.steps[0].completed);

        let notifications = state.notifications();
        assert!(
            !notifications
                .iter()
                .any(|n| n.class == NotificationClass::Failure),
            "completed tutorial must not emit Failure notifications: {notifications:?}",
        );
    }

    // ── Inline editor (edit steps) ──────────────────────────────────────

    /// Build a Running state with a single inline-editable step whose
    /// `edit_target`/`verify` point at `target` (an absolute path so the test
    /// never depends on the process working directory).
    fn editable_state(target: &std::path::Path, seed: &str) -> TutorialState {
        let target_str = target.to_string_lossy().to_string();
        let mut state = TutorialState::new();
        state.steps = vec![TutorialStep {
            title: "Edit step".to_string(),
            description: "desc".to_string(),
            instruction: "press e".to_string(),
            verify: Some(Verify::FileExists(target_str.clone())),
            edit_target: Some(target_str),
            seed_template: Some(seed.to_string()),
            ..TutorialStep::default()
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::Policy);
        state
    }

    fn unique_tmp(name: &str) -> std::path::PathBuf {
        // Distinct per test name to avoid collisions under the parallel runner.
        std::env::temp_dir().join(format!("anvil_tut_inline_{name}"))
    }

    #[test]
    fn pressing_e_opens_seeded_editor_when_file_absent() {
        let dir = unique_tmp("seeded");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("seed.rego");

        let mut state = editable_state(&target, "package seed\n");
        assert!(!state.is_editing());
        state.handle_key(Action::Character('e'));

        assert!(state.is_editing());
        let editor = state.editor.as_ref().unwrap();
        assert!(editor.content().contains("package seed"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_editor_prefers_existing_file_over_seed() {
        let dir = unique_tmp("existing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("keep.rego");
        std::fs::write(&target, "existing content\n").unwrap();

        let mut state = editable_state(&target, "SEED SHOULD NOT APPEAR");
        state.open_step_editor();

        let editor = state.editor.as_ref().unwrap();
        assert!(editor.content().contains("existing content"));
        assert!(!editor.content().contains("SEED"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn e_is_noop_on_non_editable_step() {
        let mut state = state_with_plain_steps(1);
        state.handle_key(Action::Character('e'));
        assert!(!state.is_editing());
    }

    #[test]
    fn save_writes_file_and_advances_when_verify_passes() {
        let dir = unique_tmp("save");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("out.rego");

        let mut state = editable_state(&target, "package seed\n");
        state.open_step_editor();
        // Save the (seeded) content with Ctrl-S.
        state.handle_key(Action::Character('\x13'));

        assert!(target.exists(), "file should be written on save");
        assert!(!state.is_editing(), "editor closes after successful save");
        // Single step → tutorial completes.
        assert_eq!(state.phase, TutorialPhase::Complete);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_persists_typed_content() {
        let dir = unique_tmp("typed");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("typed.txt");

        let mut state = editable_state(&target, "");
        state.open_step_editor();
        for c in "hi jkq".chars() {
            state.handle_key(Action::Character(c));
        }
        state.handle_key(Action::Character('\x13'));

        let written = std::fs::read_to_string(&target).unwrap();
        assert!(
            written.contains("hi jkq"),
            "typed text (including j/k/q/space) must be saved, got: {written:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn letters_that_are_normally_navigation_insert_while_editing() {
        // At the state-machine level, every Action::Character is inserted while
        // editing — the CLI's text keymap is what turns raw j/k/q into
        // Character; here we assert the surface does not treat them specially.
        let dir = unique_tmp("navletters");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("nav.txt");

        let mut state = editable_state(&target, "");
        state.open_step_editor();
        for c in ['j', 'k', 'q', ' '] {
            state.handle_key(Action::Character(c));
        }
        assert!(state.is_editing(), "q must not quit while editing");
        let editor = state.editor.as_ref().unwrap();
        assert!(editor.content().contains("jkq "));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paging_uses_the_recorded_editor_viewport() {
        // The renderer records the viewport height; the handler must page by
        // that height (not a hardcoded constant) and keep the cursor visible.
        let dir = unique_tmp("viewport");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("tall.txt");

        let seed = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut state = editable_state(&target, &seed);
        state.open_step_editor();
        // Simulate a render that measured a 5-row editor viewport.
        state.editor_viewport.set(5);

        state.handle_key(Action::PageDown);
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(
            editor.cursor_line(),
            5,
            "PageDown should advance by the recorded viewport height (5), not a constant"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn newline_action_splits_lines_in_editor() {
        let dir = unique_tmp("newline");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("nl.txt");

        let mut state = editable_state(&target, "");
        state.open_step_editor();
        state.handle_key(Action::Character('a'));
        state.handle_key(Action::Character('\n'));
        state.handle_key(Action::Character('b'));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.line_count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn esc_cancels_editor_without_writing() {
        let dir = unique_tmp("cancel");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("cancel.txt");

        let mut state = editable_state(&target, "seed\n");
        state.open_step_editor();
        state.handle_key(Action::Character('x'));
        state.handle_key(Action::Back); // Esc

        assert!(!state.is_editing());
        assert!(!target.exists(), "cancel must not write the file");
        assert_eq!(state.phase, TutorialPhase::Running, "step not advanced");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = unique_tmp("mkparents");
        let _ = std::fs::remove_dir_all(&dir);
        // Note: parent (.anvil/policies) does not exist yet.
        let target = dir.join("nested").join("deep").join("file.rego");

        let mut state = editable_state(&target, "seed\n");
        state.open_step_editor();
        state.handle_key(Action::Character('\x13'));

        assert!(target.exists(), "save must create parent directories");
        assert!(state.edit_error.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_help_text_shows_save_and_cancel() {
        let dir = unique_tmp("help");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut state = editable_state(&dir.join("h.txt"), "");
        // Editable-but-not-editing shows the inline-edit affordance.
        assert!(crate::surface::Surface::help_text(&state).contains("e edit"));
        state.open_step_editor();
        let help = crate::surface::Surface::help_text(&state);
        assert!(
            help.contains("save"),
            "editing help must mention save: {help}"
        );
        assert!(help.contains("cancel"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_and_architecture_edit_steps_are_inline_editable() {
        // The two converted legacy steps expose an edit_target + seed.
        let policy = paths::policy_steps();
        let write_step = &policy[2];
        assert_eq!(
            write_step.edit_target.as_deref(),
            Some(".anvil/policies/no-todos.rego")
        );
        assert!(write_step.seed_template.is_some());
        // Still verifiable via the external-editor path.
        assert!(write_step.watch_path.is_some());

        let arch = paths::architecture_steps();
        let layers_step = &arch[1];
        assert_eq!(
            layers_step.edit_target.as_deref(),
            Some(".anvil/architecture.yaml")
        );
        assert!(layers_step.seed_template.is_some());
    }

    /// CIB-349: a signed-out Policy walk must not present `anvil policy test`
    /// as a runnable check. The step names `anvil auth login` first and
    /// Enter advances without spawning.
    #[test]
    fn signed_out_policy_test_is_a_sign_in_bridge() {
        let mut state = TutorialState::new();
        state.set_requires_sign_in(true);
        state.load_steps(TutorialPath::Policy);

        let test = state
            .steps
            .iter()
            .find(|step| step.title == "Test the Policy")
            .expect("policy test step");
        assert!(test.sign_in_bridge);
        assert!(
            test.command.is_none(),
            "must not stay a runnable check: {:?}",
            test.command
        );
        assert!(
            test.instruction.contains("anvil auth login"),
            "bridge must name login first: {}",
            test.instruction
        );
        assert!(
            test.instruction.contains("anvil policy test"),
            "bridge must still name the deferred command: {}",
            test.instruction
        );

        let test_index = state
            .steps
            .iter()
            .position(|step| step.title == "Test the Policy")
            .expect("index");
        state.current_step = test_index;
        assert!(!state.current_step_has_command());
        assert!(state.current_step_is_sign_in_bridge());
        let help = <TutorialState as crate::surface::Surface>::help_text(&state);
        assert!(
            help.contains("anvil auth login"),
            "footer must name login before the command: {help}"
        );
        assert!(
            !help.contains("run command"),
            "footer must not offer Enter as run: {help}"
        );

        state.handle_key(Action::Select);
        assert!(
            state.steps[test_index].output.is_none(),
            "Enter must not execute the gated command"
        );
        assert_eq!(state.current_step, test_index + 1);
    }

    #[test]
    fn signed_out_policy_gate_copy_names_login_first() {
        let mut state = TutorialState::new();
        state.set_requires_sign_in(true);
        state.load_steps(TutorialPath::Policy);

        let fire = state
            .steps
            .iter()
            .find(|step| step.title == "See the Policy Fire")
            .expect("policy fire step");
        assert!(fire.sign_in_bridge);
        assert!(
            fire.instruction.contains("anvil auth login"),
            "run-now gate copy must name login first: {}",
            fire.instruction
        );

        let severity = state
            .steps
            .iter()
            .find(|step| step.title == "Customise Severity")
            .expect("severity step");
        assert!(
            severity.instruction.contains("anvil auth login"),
            "re-run gate copy must name login first: {}",
            severity.instruction
        );
    }

    #[test]
    fn signed_out_architecture_validate_is_a_sign_in_bridge() {
        let mut state = TutorialState::new();
        state.set_requires_sign_in(true);
        state.load_steps(TutorialPath::Architecture);

        let validate = state
            .steps
            .iter()
            .find(|step| step.title == "Validate the Architecture")
            .expect("architecture validate step");
        assert!(validate.sign_in_bridge);
        assert!(validate.command.is_none());
        assert!(
            validate.instruction.contains("anvil auth login"),
            "{}",
            validate.instruction
        );
        assert!(validate.instruction.contains("anvil architecture validate"));
    }

    #[test]
    fn signed_out_protection_loop_keeps_verify_probe_runnable() {
        let mut state = TutorialState::new();
        state.set_requires_sign_in(true);
        state.load_steps(TutorialPath::ProtectionLoop);

        let verifier = state.steps.last().expect("verifier");
        assert!(!verifier.sign_in_bridge);
        assert_eq!(
            verifier.command.as_deref(),
            Some("anvil start --verify"),
            "the free --verify probe must stay runnable unsigned-in"
        );
    }

    #[test]
    fn signed_in_policy_test_stays_runnable() {
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Policy);
        let test = state
            .steps
            .iter()
            .find(|step| step.title == "Test the Policy")
            .expect("policy test step");
        assert!(!test.sign_in_bridge);
        assert_eq!(test.command.as_deref(), Some("anvil policy test"));
    }

    #[test]
    fn execute_intercept_converts_leftover_gated_command() {
        let mut state = TutorialState::new();
        state.set_requires_sign_in(true);
        state.phase = TutorialPhase::Running;
        state.steps = vec![TutorialStep {
            title: "Test the Policy".to_string(),
            description: "run it".to_string(),
            instruction: "Run: anvil policy test".to_string(),
            command: Some("anvil policy test".to_string()),
            effect: Some(CommandEffect::ReadOnly),
            ..TutorialStep::default()
        }];
        assert!(!state.execute_current_command("anvil policy test"));
        assert!(state.steps[0].sign_in_bridge);
        assert!(state.steps[0].command.is_none());
        assert!(
            state.steps[0].output.is_none(),
            "must not surface a failed-command auth wall"
        );
    }
}
