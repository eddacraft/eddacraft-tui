mod activation;
mod auth;
mod capacity;
mod commands;
mod config_summary;
mod config_view;
mod feature_flags;
mod help_layout;
mod insights;
mod install_root;
#[cfg(unix)]
mod intercept_symbol_parser;
mod kindling_daemon_sink;
mod l4_engine;
mod mcp;
mod output;
mod plan_dashboard;
mod services;
#[cfg(test)]
mod test_support;
mod tui;
mod update_hint;
mod usage;
mod usage_views;
mod util;
mod warmup_cache;
mod whats_new;

use std::io::IsTerminal;
use std::process::ExitCode;

use anyhow::Context;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

/// Exit codes for structured error reporting.
///
/// Codes 0–4 are the established surface (`EXIT_OK`, `EXIT_ERROR`,
/// `EXIT_GATE_FAIL`, `EXIT_AUTH_REQUIRED`, `EXIT_CONFIG_ERROR`).
///
/// Codes 5, 6, 7, 10 are pre-positioned for the v1 multi-layer
/// protection architecture per
/// [CLI surface coherence spec](../../../plans/specs/2026-05-07-cli-surface-coherence.md)
/// §3 (CLIC-001 / A7.3). They are declared here so future MLP /
/// DLIFE work items emit them via constants rather than magic
/// numbers; no current code path emits them yet.
///
/// CI / scripts that gate on anvil exit codes can rely on this map:
/// fail-fast on `2` (gate failure), `5` (cross-boundary detected),
/// `7` (version mismatch), `10` (discovery failed); treat `1`, `3`,
/// `4`, `6` as recoverable user-action conditions.
pub const EXIT_OK: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_GATE_FAIL: u8 = 2;
pub const EXIT_AUTH_REQUIRED: u8 = 3;
pub const EXIT_CONFIG_ERROR: u8 = 4;

/// Surface and daemon were on different OS instances (per ADR-036
/// `os_locality_token` mismatch) — surface refused to attach, OR
/// `anvil doctor --explain-boundary` detected a `cross-boundary-mixed`
/// configuration. Reserved for future emission by MLP / DLIFE
/// boundary-detection code paths.
pub const EXIT_CROSS_BOUNDARY: u8 = 5;

/// Daemon is not running and embedded fallback is unavailable. Reserved
/// for future emission by `anvil doctor` / `anvil intercept ensure` /
/// hooks that strictly require the daemon.
pub const EXIT_DAEMON_DOWN: u8 = 6;

/// `proto-version-mismatch` between this CLI / hook and the running
/// daemon (per ADR-036 §D-3). Reserved for future emission by
/// `anvil intercept ensure` / hooks when the daemon's
/// `proto_version` is outside the surface's supported range.
pub const EXIT_VERSION_MISMATCH: u8 = 7;

/// Discovery failed — runtime dir untrusted (lstat-ladder violation
/// per ADR-036 §D-3) or `info.json` ownership / mode invalid.
/// Reserved for future emission by `anvil doctor` / `anvil intercept
/// ensure` / hooks that read the runtime sidecar.
///
/// Note: codes 8 and 9 are intentionally reserved. The CLI surface
/// spec leaves them for future expansion (e.g., per-platform-specific
/// errors). Future contributors should not claim 8 or 9 without an
/// ADR amendment.
pub const EXIT_DISCOVERY_FAILED: u8 = 10;

/// Early-access request channel surfaced on the not-logged-in auth gate
/// (CIB-060). Matches README ("Early access at eddacraft.ai").
const EARLY_ACCESS_URL: &str = "https://eddacraft.ai";

const AUTH_NOT_AUTHENTICATED_MESSAGE: &str = "Authentication required. Run `anvil auth login` to authenticate. Early access: https://eddacraft.ai";

const AUTH_SESSION_EXPIRED_MESSAGE: &str =
    "Session expired. Run `anvil auth login` to re-authenticate.";

const AUTH_INVALID_EDICT_MESSAGE: &str =
    "Early-access edict is invalid or revoked. Run `anvil auth login --edict` to authenticate.";

/// Which `EXIT_AUTH_REQUIRED` path fired — drives JSON envelope copy while
/// keeping expired-session and invalid-edict messages unchanged (CIB-060).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthRequiredKind {
    NotAuthenticated,
    SessionExpired,
    InvalidEdict,
}

type AuthCheckFailure = (u8, Option<AuthRequiredKind>);
type AuthCheckResult = Result<(), AuthCheckFailure>;

/// Global arguments available to every subcommand.
// Each field is an independent CLI flag, not coupled state — a state machine /
// two-variant-enum refactor would obscure, not clarify, the clap surface.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Parser)]
pub struct GlobalArgs {
    /// Output results as JSON instead of human-readable text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable TUI rendering; use plain text output.
    #[arg(long, global = true)]
    pub no_tui: bool,

    /// Enable verbose logging.
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Re-root install-owned state (user state, daemon socket/PID, kernel
    /// cache/logs) under this prefix so a pre-release candidate can run
    /// side-by-side with the production install. Takes precedence over the
    /// `ANVIL_HOME` env var. Per-project `.anvil/` state stays at the project
    /// root; durable project mutations are gated unless
    /// `--touch-project-state` is also set.
    #[arg(long, global = true, value_name = "PATH")]
    pub anvil_home: Option<std::path::PathBuf>,

    /// Permit durable per-project mutations (baseline refresh, witness append,
    /// cutoff pinning) while running under a non-default `--anvil-home` /
    /// `ANVIL_HOME`. Without it such writes run read-only / dry-run so an
    /// unreleased candidate cannot silently corrupt a real project.
    #[arg(long, global = true)]
    pub touch_project_state: bool,
}

/// anvil — structural governance for AI-assisted development.
#[derive(Debug, Parser)]
#[command(
    name = "anvil",
    version,
    about,
    long_about = None,
    after_help = "\
EXIT CODES:
  0  Success (incl. pre-dispatch auth-required on action commands)
  1  General error (incl. failed `anvil auth login` attempt)
  2  Gate check failed (one or more checks did not pass)
  3  Authentication required:
       - pre-dispatch on `whoami` / `auth whoami` (state probe)
       - post-dispatch on any command (server-rejected token mid-call)
  4  Configuration error (invalid config file or options)"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,

    #[command(subcommand)]
    command: Commands,
}

fn augmented_cli_command() -> clap::Command {
    help_layout::augment_clic_010_help(Cli::command())
}

fn try_parse_cli() -> Result<Cli, clap::Error> {
    try_parse_cli_from(std::env::args_os())
}

fn try_parse_cli_from<I, T>(itr: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let mut command = augmented_cli_command();
    let mut matches = command.try_get_matches_from_mut(itr)?;
    Cli::from_arg_matches_mut(&mut matches)
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a full project audit.
    Audit(commands::audit::AuditArgs),
    /// Audit the witness chain for commits that bypassed protection.
    ///
    /// Walks the branch and reports commits that lack a witness record.
    /// Catches bypassed protection (admin overrides, force-push
    /// manipulation). Run nightly via the `anvil-audit` workflow
    /// template or on-demand from the CLI.
    AuditChain(commands::audit_chain::AuditChainArgs),
    /// Scan files for anti-patterns and hardcoded secrets (planless mode).
    ///
    /// Honours `.anvilrc#checks` (and `.anvil.<ext>`) for the
    /// planless-eligible subset: `antipattern-scan` and `secret-detection`.
    /// Profile-based or config-heavy checks (`architecture`, `policy`,
    /// `import-boundaries`, `command-safety`, `lint`, `test`, `coverage`,
    /// `dependency`) live under `anvil gate`.
    Check(commands::check::CheckArgs),
    /// Report a false positive against a check.
    ///
    /// Records `anvil report-fp <check-id> <file:line>` to the local
    /// Kindling record only, or lists local reports with `--list` — nothing
    /// leaves the machine. The file path is recorded and listed as a
    /// one-way salted hash, never plaintext, and source content is never
    /// included unless `--include-snippet` is set. The check identifier is
    /// validated against the stable-ID registry.
    #[command(name = "report-fp")]
    ReportFp(commands::report_fp::ReportFpArgs),
    /// Run diagnostic checks on your environment.
    Doctor(commands::doctor::DoctorArgs),
    /// Show, set, and convert anvil project config.
    Config(commands::config::ConfigArgs),
    /// Track architecture drift over time.
    Drift(commands::drift::DriftArgs),
    /// List, show, and trace Edda canonical memories.
    Edda(commands::edda::EddaArgs),
    /// List Ember proposals awaiting promotion to Edda.
    Ember(commands::ember::EmberArgs),
    /// Show project status and health.
    Status(commands::status::StatusArgs),
    /// Activate anvil in this repository. Writes `.anvilrc` if missing
    /// and installs MCP config entries for Cursor and Claude Code into
    /// your home directory (`~/.cursor/mcp.json`, `~/.claude.json`).
    /// Pass `--verify` to run a read-only probe instead.
    Start(commands::start::StartArgs),
    /// Interactive guided tutorial.
    Tutorial(commands::tutorial::TutorialArgs),
    /// Show the welcome screen with quick-start options.
    Welcome(commands::welcome::WelcomeArgs),
    /// Initialise anvil configuration for a project.
    Init(commands::init::InitArgs),
    /// Show local-only weekly activity insights.
    Insights(commands::insights::InsightsArgs),
    /// Query the local command-invocation usage log (dev-investment views).
    ///
    /// `anvil kindling usage <view>` answers "what is being used and what
    /// is not" over the user-scoped usage sidecar — top commands, never
    /// invoked, flag-dependent paths, principals by activity. Local-only;
    /// no authentication required. The views are signal, not evidence.
    Kindling(commands::kindling::KindlingArgs),
    /// Migrate anvil config to a new format or schema version.
    ///
    /// `format` converts a legacy `.anvilrc` to the multi-format
    /// `.anvil.<ext>` surface; `schema` reconciles an existing config
    /// across anvil versions. Bare `anvil migrate` runs `format` for
    /// back-compat.
    Migrate(commands::migrate::MigrateArgs),
    /// Manage the anvil intercept daemon.
    Intercept(commands::intercept::InterceptArgs),
    /// Manage operator workspace confinement for the intercept daemon.
    ///
    /// Switches the daemon's admission boundary from the default
    /// first-touch-adopt (`open`) to an operator `allowlist`, and manages
    /// the allow entries. Sits above the `SO_PEERCRED` same-uid trust floor.
    Workspace(commands::workspace::WorkspaceArgs),
    /// Validate commits against policy using the L4 rule engine.
    ///
    /// Dedicated surface for CI lanes that don't sit inside git's
    /// pre-push hook. Accepts an explicit commit range instead of
    /// reading git's pre-push stdin.
    #[command(name = "l4-validate")]
    L4Validate(commands::l4_validate::L4ValidateArgs),
    /// Show anvil's acknowledgements and third-party licence attribution.
    Licenses(commands::licenses::LicensesArgs),
    /// Generate MCP server configuration for AI editors (claude-code, cursor, windsurf, vscode).
    #[command(name = "mcp-config")]
    McpConfig(commands::mcp_config::McpConfigArgs),
    /// Manage and serve MCP integrations.
    Mcp(commands::mcp::McpArgs),
    /// Inspect APS planning state.
    Plan(commands::plan::PlanArgs),
    /// Open a native read-only dashboard over local anvil state.
    Dashboard(commands::dashboard::DashboardArgs),
    /// Scaffold a new project from a template.
    New(commands::new::NewArgs),
    /// Guided project setup wizard.
    Wizard(commands::wizard::WizardArgs),
    /// Administrative commands (approvals, user management).
    Admin(commands::admin::AdminArgs),
    /// Run gate checks against the current project.
    Gate(commands::gate::GateArgs),
    /// Configure gate check settings and thresholds.
    #[command(name = "gate-config")]
    GateConfig(commands::gate_config::GateConfigArgs),
    /// Watch files and report save-time findings after the baseline scan.
    Watch(commands::watch::WatchArgs),
    /// Export constraints and configuration.
    Export(commands::export::ExportArgs),
    /// Install and manage git hooks.
    Hooks(commands::hooks::HooksArgs),
    /// Runtime hook subcommands (pre-commit, post-commit, post-merge,
    /// post-rewrite, bootstrap) — invoked by the shell wrapper.
    Hook(commands::hook::HookArgs),
    /// Manage the `anvil/baseline.json` adoption record.
    Baseline(commands::baseline::BaselineArgs),
    /// Package a commit range's governance evidence into a portable,
    /// locally verifiable review capsule.
    Capsule(commands::capsule::CapsuleArgs),
    /// Manage architecture boundary definitions.
    Architecture(commands::architecture::ArchitectureArgs),
    /// Authenticate with the anvil service.
    Auth(commands::auth::AuthArgs),
    /// Manage and evaluate policies.
    Policy(commands::policy::PolicyArgs),
    /// Graph-context operator settings (snippet-egress opt-in).
    Gctx(commands::gctx::GctxArgs),
    /// Update anvil to the latest version.
    Update(commands::update::UpdateArgs),
    /// Remove project anvil state; use `--global` for user state and daemon.
    Uninstall(commands::uninstall::UninstallArgs),
    /// Validate an APS plan file (structure, task format, hash integrity).
    Validate(commands::validate::ValidateArgs),
    /// Show install-method-aware version + upgrade guidance.
    Version(commands::version::VersionArgs),
    /// Log in to anvil (alias for `auth login`).
    #[command(hide = true)]
    Login(commands::auth::LoginArgs),
    /// Log out of anvil (alias for `auth logout`).
    #[command(hide = true)]
    Logout(commands::auth::LogoutArgs),
    /// Show current identity (alias for `auth whoami`).
    #[command(hide = true)]
    Whoami(commands::auth::WhoamiArgs),
}

/// Canonical stable name for a `Commands` variant.
///
/// Used to map dispatch-time variants onto the gated-command list carried
/// as metadata on the `cli.licence-gate` flag. Kept separate from
/// `clap`'s display names so that hidden aliases (`login`, `logout`,
/// `whoami`) and their real subcommands (`auth login`, …) map onto
/// distinct canonical identifiers where needed.
fn command_canonical_name(cmd: &Commands) -> &'static str {
    use commands::auth::AuthCommand;
    match cmd {
        Commands::Audit(_) => "audit",
        Commands::AuditChain(_) => "audit-chain",
        Commands::Check(_) => "check",
        Commands::ReportFp(_) => "report-fp",
        Commands::Doctor(_) => "doctor",
        Commands::Config(_) => "config",
        Commands::Drift(_) => "drift",
        Commands::Edda(_) => "edda",
        Commands::Ember(_) => "ember",
        Commands::Start(_) => "start",
        Commands::Status(_) => "status",
        Commands::Tutorial(_) => "tutorial",
        Commands::Welcome(_) => "welcome",
        Commands::Init(_) => "init",
        Commands::Insights(_) => "insights",
        Commands::Kindling(_) => "kindling",
        Commands::Migrate(_) => "migrate",
        Commands::Intercept(_) => "intercept",
        Commands::Workspace(_) => "workspace",
        Commands::L4Validate(_) => "l4-validate",
        Commands::Licenses(_) => "licenses",
        Commands::McpConfig(_) => "mcp-config",
        Commands::Mcp(args) => commands::mcp::auth_gate_name(args),
        Commands::Plan(_) => "plan",
        Commands::Dashboard(_) => "dashboard",
        Commands::New(_) => "new",
        Commands::Wizard(_) => "wizard",
        Commands::Admin(_) => "admin",
        Commands::Gate(_) => "gate",
        Commands::GateConfig(_) => "gate-config",
        Commands::Watch(_) => "watch",
        Commands::Export(_) => "export",
        Commands::Hooks(_) => "hooks",
        Commands::Hook(_) => "hook",
        Commands::Baseline(_) => "baseline",
        Commands::Capsule(_) => "capsule",
        Commands::Architecture(_) => "architecture",
        Commands::Policy(_) => "policy",
        Commands::Gctx(_) => "gctx",
        Commands::Update(_) => "update",
        Commands::Uninstall(_) => "uninstall",
        Commands::Validate(_) => "validate",
        Commands::Version(_) => "version",
        Commands::Login(_) => "login",
        Commands::Logout(_) => "logout",
        Commands::Whoami(_) => "whoami",
        Commands::Auth(args) => match args.command {
            AuthCommand::Login { .. } => "auth-login",
            AuthCommand::Logout => "auth-logout",
            AuthCommand::Whoami => "auth-whoami",
            AuthCommand::Refresh => "auth-refresh",
        },
    }
}

/// The canonical, user-visible top-level command names, derived from
/// clap's own subcommand registry.
///
/// Used by the USAGE-003 `kindling usage unused` view to compute
/// "registered minus seen" without a hand-maintained list that could
/// drift. Hidden aliases (`login` / `logout` / `whoami`) are excluded so
/// the view reports canonical surfaces only. Sorted for a stable view.
///
/// Caveat (documented in the runbook): a command recorded under a
/// finer-grained canonical name than its clap name — `auth` runs as
/// `auth-login` — can still appear here even though a subcommand ran.
pub fn registered_command_names() -> Vec<String> {
    use clap::CommandFactory;
    let mut names: Vec<String> = Cli::command()
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set())
        .map(|sub| sub.get_name().to_owned())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Returns `true` for commands that require a valid auth session.
///
/// Delegates to the `cli.licence-gate` flag's gated-command metadata via
/// [`feature_flags::command_needs_licence_gate`]. FLAGM-006 retired the
/// legacy hard-coded match and its parity-test scaffolding; the flag is
/// now the sole source of truth.
fn requires_auth(cmd: &Commands) -> bool {
    feature_flags::command_needs_licence_gate(command_canonical_name(cmd))
}

/// Whether the command requested machine-readable output via its own
/// `--format json|sarif` flag (distinct from the global `--json`).
///
/// The pre-dispatch auth gate uses this so a structured consumer of, e.g.,
/// `anvil check --format json` receives a JSON auth envelope rather than a
/// human-readable line — the global `--json` flag alone did not cover the
/// per-command `--format` surface. Only the finding-emitting commands
/// (`check` / `audit` / `gate`) expose `--format`.
fn command_requests_structured_output(cmd: &Commands) -> bool {
    match cmd {
        Commands::Check(args) => args.wants_structured_output(),
        Commands::Audit(args) => args.wants_structured_output(),
        Commands::Gate(args) => args.wants_structured_output(),
        _ => false,
    }
}

/// Read-only activation probes that must work unauthenticated (and
/// air-gapped): `status --verify` and its documented sibling
/// `start --verify` (CIB-049). Full (mutating) `start` and plain
/// `status` stay auth-gated.
fn skips_auth_for_local_probe(cmd: &Commands) -> bool {
    match cmd {
        Commands::Status(args) => args.verify,
        Commands::Start(args) => args.verify,
        _ => false,
    }
}

/// Returns `true` for commands whose entire purpose is to report the
/// current auth state — the canonical programmatic preflight. For these,
/// auth-required is the substantive answer the caller is asking for, so
/// the exit code carries the signal (`EXIT_AUTH_REQUIRED`).
///
/// All other gated commands treat auth-required as an *expected state*
/// (you haven't logged in yet) and exit `0` with an informational
/// message — see issue #1822.
fn is_auth_state_probe(cmd: &Commands) -> bool {
    use commands::auth::AuthCommand;
    match cmd {
        Commands::Whoami(_) => true,
        Commands::Auth(args) => matches!(args.command, AuthCommand::Whoami),
        _ => false,
    }
}

/// Decide the exit code and (optional) JSON envelope for the
/// pre-dispatch auth-required branch.
///
/// Issue #1822: action commands treat auth-required as an *expected
/// state* (the user hasn't logged in yet) and exit `0`; the stderr
/// message stays loud so humans see what to do next. Only the dedicated
/// auth-state probes (`whoami`, `auth whoami`) carry the auth signal in
/// the exit code so scripts have a stable preflight.
///
/// The exit-code coercion to `0` is gated on the incoming code being
/// exactly `EXIT_AUTH_REQUIRED`. Any other failure from `check_auth`
/// (e.g. a failed interactive `anvil auth login` attempt, which now
/// returns `EXIT_ERROR`) is a real runtime failure and passes through
/// unchanged — scripts must be able to distinguish "user hasn't logged
/// in yet" from "user tried to log in and it failed".
///
/// Pure so it can be unit-tested without depending on credential I/O.
/// Returns `(exit_code, Some(json_envelope))` when `--json` is set —
/// the caller prints the envelope to **stdout** (structured data, per
/// the stream policy in `docs/guides/cli-output-streams.md`) — or
/// `(exit_code, None)` in text mode, where `check_auth` already
/// emitted the human-readable message to stderr.
fn auth_required_message(kind: AuthRequiredKind) -> &'static str {
    match kind {
        AuthRequiredKind::NotAuthenticated => AUTH_NOT_AUTHENTICATED_MESSAGE,
        AuthRequiredKind::SessionExpired => AUTH_SESSION_EXPIRED_MESSAGE,
        AuthRequiredKind::InvalidEdict => AUTH_INVALID_EDICT_MESSAGE,
    }
}

fn auth_required_response(
    cmd: &Commands,
    code: u8,
    json_mode: bool,
    kind: Option<AuthRequiredKind>,
) -> (u8, Option<serde_json::Value>) {
    // Anything other than EXIT_AUTH_REQUIRED is a real failure that
    // happened to surface from `check_auth` (today: a failed login
    // attempt). Pass it through with a generic error envelope under
    // `--json`; the stderr message is already on the wire.
    if code != EXIT_AUTH_REQUIRED {
        let envelope = json_mode.then(|| serde_json::json!({"error": "auth_check_failed"}));
        return (code, envelope);
    }
    let is_probe = is_auth_state_probe(cmd);
    let exit_code = if is_probe { code } else { EXIT_OK };
    let envelope = if !json_mode {
        None
    } else if is_probe {
        Some(serde_json::json!({"error": "authentication_required"}))
    } else {
        let (message, early_access_url) = match kind {
            Some(AuthRequiredKind::NotAuthenticated) => {
                (AUTH_NOT_AUTHENTICATED_MESSAGE, Some(EARLY_ACCESS_URL))
            }
            Some(other) => (auth_required_message(other), None),
            None => (
                "Authentication required. Run `anvil auth login` to authenticate.",
                None,
            ),
        };
        let mut envelope = serde_json::json!({
            "state": "authRequired",
            "message": message,
            "next": "anvil auth login",
        });
        if let Some(url) = early_access_url {
            envelope["earlyAccessUrl"] = serde_json::Value::String(url.to_owned());
        }
        Some(envelope)
    };
    (exit_code, envelope)
}

/// Evaluate a credential-load result and return the appropriate exit code.
///
/// Separated from I/O so tests can call it with synthetic inputs.
/// The underlying error from a failed credential load is always printed
/// so that system faults (I/O errors, corrupt files) are distinguishable
/// from a simple "not logged in" state in CI logs. When `verbose` is true,
/// the full error chain is shown; otherwise only a short summary.
fn evaluate_auth(
    loaded: &anyhow::Result<Option<auth::credentials::Credentials>>,
    verbose: bool,
    emit_human_messages: bool,
) -> AuthCheckResult {
    match loaded {
        Ok(Some(creds)) if auth::credentials::is_expired(creds) => {
            if emit_human_messages {
                eprintln!("{AUTH_SESSION_EXPIRED_MESSAGE}");
            }
            Err((EXIT_AUTH_REQUIRED, Some(AuthRequiredKind::SessionExpired)))
        }
        Ok(Some(creds)) if auth::credentials::is_edict(creds) => {
            if verify_edict_auth(creds, verbose, emit_human_messages) {
                Ok(())
            } else {
                if emit_human_messages {
                    eprintln!("{AUTH_INVALID_EDICT_MESSAGE}");
                }
                Err((EXIT_AUTH_REQUIRED, Some(AuthRequiredKind::InvalidEdict)))
            }
        }
        Ok(Some(_)) => Ok(()),
        Ok(None) => {
            if emit_human_messages {
                eprintln!("{AUTH_NOT_AUTHENTICATED_MESSAGE}");
            }
            Err((EXIT_AUTH_REQUIRED, Some(AuthRequiredKind::NotAuthenticated)))
        }
        Err(err) => {
            let msg = if verbose {
                format!("{err:#}")
            } else {
                format!("{err}")
            };
            // Redact home directory to avoid leaking paths in CI logs.
            let redacted = crate::util::user_home_dir()
                .map(|h| msg.replace(h.to_string_lossy().as_ref(), "~"))
                .unwrap_or(msg);
            if emit_human_messages {
                eprintln!("[auth] credential load failed: {redacted}");
                eprintln!(
                    "Could not read stored credentials. The file may be corrupt or unreadable; \
                     `anvil auth login` will overwrite it."
                );
            }
            // A genuine load fault (I/O error, corrupt/unparseable file) is a
            // configuration error, NOT the "not logged in yet" state. Coercing
            // it to EXIT_AUTH_REQUIRED is the silent-degrade-to-default class
            // called out in PR #1721 — it hides a real system fault behind the
            // normal exit-0/auth-required path. `load()` already folds a
            // missing file into `Ok(None)`, so reaching this arm means the
            // credential store exists but could not be read.
            Err((EXIT_CONFIG_ERROR, None))
        }
    }
}

/// Outcome of attempting a refresh-token exchange at startup.
enum SilentRefreshOutcome {
    /// Fresh licence saved; caller should reload from disk and proceed.
    Refreshed,
    /// Server gave a definitive reason the refresh cannot succeed
    /// (token expired, revoked, family theft, inactive account). The
    /// reason has already been printed to stderr, so the caller should
    /// skip its own generic "Session expired" line.
    PermanentFailure,
    /// Network / save / parse error. Caller should continue with the
    /// existing expired-session path so a transient blip doesn't mask
    /// the user's actual auth state.
    TransientFailure,
}

/// Exchange a stored refresh token for a fresh licence and persist the
/// result. Permanent failures print an actionable reason to stderr;
/// transient failures stay silent unless `verbose` is set.
fn try_silent_refresh(
    creds: &auth::credentials::Credentials,
    verbose: bool,
    emit_human_messages: bool,
) -> SilentRefreshOutcome {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            if verbose && emit_human_messages {
                eprintln!("[auth] could not create refresh runtime: {err:#}");
            }
            return SilentRefreshOutcome::TransientFailure;
        }
    };

    match rt.block_on(auth::device_flow::try_refresh_credentials(creds)) {
        Ok(new_creds) => {
            if let Err(err) = auth::credentials::save(&new_creds) {
                if verbose && emit_human_messages {
                    eprintln!("[auth] saving refreshed credentials failed: {err:#}");
                }
                return SilentRefreshOutcome::TransientFailure;
            }
            if verbose && emit_human_messages {
                eprintln!("[auth] refreshed expired session via stored refresh token");
            }
            SilentRefreshOutcome::Refreshed
        }
        Err(err) => {
            if auth::device_flow::is_permanent_refresh_failure(&err) {
                if emit_human_messages {
                    eprintln!("{err}");
                }
                SilentRefreshOutcome::PermanentFailure
            } else {
                if verbose && emit_human_messages {
                    eprintln!("[auth] silent refresh failed: {err:#}");
                }
                SilentRefreshOutcome::TransientFailure
            }
        }
    }
}

fn verify_edict_auth(
    creds: &auth::credentials::Credentials,
    verbose: bool,
    emit_human_messages: bool,
) -> bool {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            if verbose && emit_human_messages {
                eprintln!("[auth] could not create edict verification runtime: {err:#}");
            }
            return false;
        }
    };

    let client = match auth::client::AnvilClient::with_token(creds.license.clone()) {
        Ok(client) => client,
        Err(err) => {
            if verbose && emit_human_messages {
                eprintln!("[auth] could not create edict verification client: {err:#}");
            }
            return false;
        }
    };

    match rt.block_on(client.whoami()) {
        Ok(_) => true,
        Err(err) => {
            if verbose && emit_human_messages {
                eprintln!("[auth] edict verification failed: {err:#}");
            }
            false
        }
    }
}

/// Decide whether to offer an interactive login prompt instead of erroring
/// out with `EXIT_AUTH_REQUIRED`. Pure so it can be unit-tested.
///
/// - `suppress_interactive`: caller has reason to skip prompting —
///   `--json`/`--no-tui`, a CI/git-hook env signal, or a command like
///   `whoami` that should report state rather than launch flows.
/// - `tty_ok`: both stdin AND stderr are TTYs — required for prompting and
///   displaying the device-flow code. `stdout` is deliberately not checked;
///   the prompt goes to stderr so piping stdout (`anvil status | less`)
///   must not suppress it.
/// - `loaded`: the current credential-load result. Only missing/expired
///   trigger a prompt; a load error is treated as a systemic fault the
///   user needs to investigate, not re-prompt through.
fn should_offer_interactive_login(
    suppress_interactive: bool,
    tty_ok: bool,
    loaded: &anyhow::Result<Option<auth::credentials::Credentials>>,
) -> bool {
    if suppress_interactive || !tty_ok {
        return false;
    }
    match loaded {
        Ok(None) => true,
        Ok(Some(creds)) => auth::credentials::is_expired(creds),
        Err(_) => false,
    }
}

/// Detect environments where launching an interactive prompt would hang or
/// corrupt the host process:
///
/// - `ANVIL_NO_PROMPT` / `NONINTERACTIVE` — explicit opt-outs.
/// - `CI=true`/`CI=1` — GitHub Actions, Buildkite, `CircleCI`, etc. Some of
///   these allocate a PTY (`script -qfc`, `pty: true`) so TTY detection
///   alone is not enough.
/// - `GIT_DIR` / `GIT_INDEX_FILE` — reliably set by git when it invokes a
///   hook. Prompting from a commit hook would hold git's index lock.
pub(crate) fn is_non_interactive_env() -> bool {
    // Presence-only: matches the common shell convention that
    // `export FOO=` is still "set". Empty-string should count as opt-out.
    let is_set = |k: &str| std::env::var_os(k).is_some();
    if is_set("ANVIL_NO_PROMPT") || is_set("NONINTERACTIVE") {
        return true;
    }
    if matches!(
        std::env::var("CI").ok().as_deref(),
        Some("true" | "1" | "TRUE" | "True")
    ) {
        return true;
    }
    if is_set("GIT_DIR") || is_set("GIT_INDEX_FILE") {
        return true;
    }
    false
}

/// Returns `false` for commands that should never trigger an interactive
/// login flow even when the user is missing credentials — e.g. `whoami`,
/// whose job is to report identity state, not mutate it, and `auth refresh`,
/// which operates on stale credentials by design.
fn allows_interactive_auth_prompt(cmd: &Commands) -> bool {
    use commands::auth::AuthCommand;
    match cmd {
        Commands::Whoami(_) => false,
        Commands::Auth(args) => !matches!(args.command, AuthCommand::Whoami | AuthCommand::Refresh),
        _ => true,
    }
}

/// Prompt for a yes/no answer on stderr, reading from stdin.
///
/// Returns `Ok(false)` on EOF (`read_line` returning 0 bytes) so a closed
/// stdin fails closed rather than fail-open into launching device flow.
///
/// Defensively restores cooked terminal mode before reading. A previous
/// TUI in the same shell session (e.g. the MCP picker via `demand` →
/// `console::Term`) can leave the terminal in raw mode after an
/// abnormal exit; in raw mode the kernel never line-terminates stdin,
/// so `read_line` would block indefinitely even though the user is
/// typing `y` / `n` + Enter. `disable_raw_mode` is a no-op when the
/// terminal is already cooked, so this is safe to run unconditionally.
fn prompt_yes_no(message: &str, default_yes: bool) -> std::io::Result<bool> {
    use std::io::{BufRead, Write};
    let _ = crossterm::terminal::disable_raw_mode();

    let mut stderr = std::io::stderr();
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    write!(stderr, "{message} {hint} ")?;
    stderr.flush()?;

    let mut line = String::new();
    let stdin = std::io::stdin();
    let mut locked = stdin.lock();
    let n = locked.read_line(&mut line)?;
    if n == 0 {
        return Ok(false);
    }
    Ok(match line.trim().to_ascii_lowercase().as_str() {
        "" => default_yes,
        "y" | "yes" => true,
        _ => false,
    })
}

/// Run the device-code login flow on a fresh tokio runtime.
///
/// Uses a current-thread runtime since the device flow is pure I/O and
/// doesn't benefit from a work-stealing thread pool.
fn run_interactive_login() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating tokio runtime for login")?;
    rt.block_on(auth::device_flow::login_device_flow())
}

/// Validate that usable credentials exist.
///
/// Returns `Ok(())` when valid credentials are found or when
/// `ANVIL_DEV=1` is set (local dev bypass), or `Err(exit_code)` with
/// `EXIT_AUTH_REQUIRED` otherwise.
///
/// When running interactively (TTY on stdin+stderr, not `--json`/`--no-tui`,
/// no CI/git-hook env signals, command allows prompting) and the only
/// problem is missing or expired credentials, offers to launch the
/// device-code login flow inline so first-time users don't bounce off a
/// terse "Run `anvil auth login`" error.
fn check_auth(global: &GlobalArgs, allow_interactive: bool, wants_json: bool) -> AuthCheckResult {
    // `wants_json` folds the global `--json` flag together with a
    // per-command `--format json|sarif` request: both mean a machine is
    // reading the output, so human-readable auth chatter must be
    // suppressed and the interactive login prompt skipped. Resolving it
    // here (rather than reading only `global.json`) is what keeps
    // `anvil check --format json` from leaking a human line onto the
    // stream instead of the structured auth envelope.
    let json_mode = global.json || wants_json;
    // Local dev bypass: ANVIL_DEV=1 resolves through the shared resolver's
    // local-override precedence on `cli.licence-gate`. Routing via the
    // resolver (rather than an inline env-var read) means override
    // telemetry, reason codes, and future override sources all share one
    // code path. Safety rationale is unchanged from the legacy bypass:
    //   - All API calls still require a real token server-side.
    //   - This only bypasses the local credential pre-check.
    //   - Commands that call the API will fail with a 401 anyway.
    //   - Intended for CLI UX testing without a live token.
    // USAGE-002: resolve `cli.licence-gate` once here so the gating policy
    // is consulted — and recorded as auth context on the usage row via the
    // open capture window — on every gated invocation, in production as
    // well as under `ANVIL_DEV`.
    //
    // USAGE-005: the resolved gate now *drives* the local credential
    // pre-check. The decision table lives in
    // `feature_flags::local_auth_precheck` (dev-bypass → Skip; `disabled`
    // variant → Skip; `enabled` → Enforce). A Skip runs the command without
    // a local credential check; for the local-only gated commands (which
    // never call the server) that means they run fully ungated — the
    // intended meaning of a `disabled` licence gate. The network-touching
    // commands (`auth`, `mcp`) still require a valid server token, so the
    // server backstops those even on a Skip. The manifest default is
    // `enabled`, so production behaviour is unchanged unless an
    // operator/targeting rule disables the gate.
    let licence_gate = feature_flags::resolve_cli_licence_gate();
    if let feature_flags::LocalAuthPrecheck::Skip(reason) =
        feature_flags::local_auth_precheck(&licence_gate)
    {
        if !json_mode {
            let note = match reason {
                feature_flags::LocalAuthSkipReason::DevBypass => "ANVIL_DEV=1 local override",
                feature_flags::LocalAuthSkipReason::GateDisabled => "licence gate disabled",
            };
            eprintln!(
                "[licence-gate] {note}: {}={} (reason={:?}) — skipping local auth pre-check",
                licence_gate.flag_key, licence_gate.variant, licence_gate.reason
            );
        }
        return Ok(());
    }

    let mut loaded = auth::credentials::load();

    // Silent refresh: if the licence expired locally but we have a refresh
    // token, exchange it before deciding to prompt or error. The 7-day JWT
    // lapses long before the 90-day refresh token, so without this every
    // expired session forced a full re-login through the device flow.
    let mut refresh_reason_already_printed = false;
    if let Ok(Some(creds)) = &loaded
        && auth::credentials::is_expired(creds)
        && creds.refresh_token.is_some()
    {
        match try_silent_refresh(creds, global.verbose, !json_mode) {
            SilentRefreshOutcome::Refreshed => loaded = auth::credentials::load(),
            SilentRefreshOutcome::PermanentFailure => {
                refresh_reason_already_printed = true;
            }
            SilentRefreshOutcome::TransientFailure => {}
        }
    }

    let suppress_interactive =
        json_mode || global.no_tui || !allow_interactive || is_non_interactive_env();
    let tty_ok = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();

    if should_offer_interactive_login(suppress_interactive, tty_ok, &loaded) {
        let expired = matches!(&loaded, Ok(Some(c)) if auth::credentials::is_expired(c));
        if !refresh_reason_already_printed {
            if expired {
                eprintln!("Your anvil session has expired.");
            } else {
                eprintln!("This command requires authentication with anvil.");
            }
        }
        match prompt_yes_no("Log in now?", true) {
            Ok(true) => match run_interactive_login() {
                Ok(()) => {
                    // Re-validate freshly-written credentials before
                    // handing off to the command — guards against clock
                    // skew or partial writes that would otherwise silently
                    // pass the local gate and fail server-side.
                    return evaluate_auth(&auth::credentials::load(), global.verbose, !json_mode);
                }
                Err(err) => {
                    // Distinct from EXIT_AUTH_REQUIRED: the user
                    // explicitly opted into the interactive login flow
                    // and it *failed* (device-flow error, network,
                    // credential save, etc.). This is a real runtime
                    // failure, not the "you haven't logged in yet"
                    // state — issue #1822 / PR #1824 review feedback.
                    eprintln!("Login failed: {err:#}");
                    return Err((EXIT_ERROR, None));
                }
            },
            Ok(false) => {
                eprintln!("Run `anvil auth login` when you're ready.");
                return Err((
                    EXIT_AUTH_REQUIRED,
                    Some(if expired {
                        AuthRequiredKind::SessionExpired
                    } else {
                        AuthRequiredKind::NotAuthenticated
                    }),
                ));
            }
            Err(err) => {
                // Fall through to the non-interactive error below.
                if global.verbose {
                    eprintln!("Could not read response: {err}");
                } else {
                    eprintln!("Could not read response.");
                }
            }
        }
    }

    if refresh_reason_already_printed {
        // Silent refresh already explained the failure; no need for
        // `evaluate_auth` to repeat itself with the generic "Session
        // expired" line.
        return Err((EXIT_AUTH_REQUIRED, Some(AuthRequiredKind::SessionExpired)));
    }

    evaluate_auth(&loaded, global.verbose, !json_mode)
}

/// Check whether `--json` appears in raw args before clap parses them.
/// This lets us emit JSON errors even when clap rejects the input.
fn wants_json() -> bool {
    std::env::args().any(|a| a == "--json")
}

/// Make the DISTRIB-006 `--anvil-home` / `--touch-project-state` flags behave as
/// the canonical `ANVIL_HOME` / `ANVIL_TOUCH_PROJECT_STATE` environment override.
///
/// The crate forbids `unsafe_code`, so we cannot `std::env::set_var` the flag into
/// the current process — and the env is the only channel that reaches every
/// consumer coherently: `anvil-intercept`'s socket/PID resolver reads the
/// environment, and a spawned daemon inherits it. So when a flag would change the
/// effective environment, re-exec this same binary **once** with the variable set
/// in the child's environment and forward the child's exit code. The child then
/// sees a normal inherited `ANVIL_HOME`, giving every downstream resolver and the
/// daemon one source of truth. When the environment already reflects the flags
/// (the common case — env-var usage, or the re-exec'd child itself) this returns
/// `None` and we proceed in-process, so at most one re-exec happens and it always
/// terminates.
///
/// Returns `Some(exit_code)` when a re-exec was performed (the caller must return
/// it), `None` when execution should continue in this process.
fn reexec_for_install_root(global: &GlobalArgs) -> Option<ExitCode> {
    use std::ffi::OsString;
    use std::path::Path;

    let mut overrides: Vec<(&'static str, OsString)> = Vec::new();

    // Whether ANVIL_HOME is in play at all (via flag or pre-existing env) — the
    // `--touch-project-state` flag only matters under a non-default install root,
    // so without one a touch-only re-exec would be a wasted fork.
    let mut anvil_home_active =
        std::env::var_os(install_root::ANVIL_HOME_ENV).is_some_and(|v| !v.is_empty());

    // Treat an empty / whitespace-only `--anvil-home` as unset, matching the
    // `ANVIL_HOME` env semantics in `install_root::resolve_install_root_from` — so
    // `--anvil-home ""` does not resolve to the cwd and silently enable the
    // override + write-guard.
    let anvil_home_flag = global
        .anvil_home
        .as_ref()
        .filter(|p| !p.as_os_str().is_empty() && !p.to_str().is_some_and(|s| s.trim().is_empty()));

    if let Some(path) = anvil_home_flag {
        anvil_home_active = true;
        let abs = if path.is_absolute() {
            path.clone()
        } else {
            std::env::current_dir().map_or_else(|_| path.clone(), |cwd| cwd.join(path))
        };
        // Compare as paths so a trailing-slash difference (`/opt/x` vs `/opt/x/`)
        // between the flag and a pre-existing env var does not trigger a spurious
        // re-exec. Equal paths → environment already reflects the flag.
        let env_matches = std::env::var_os(install_root::ANVIL_HOME_ENV)
            .is_some_and(|cur| Path::new(&cur) == abs);
        if !env_matches {
            overrides.push((install_root::ANVIL_HOME_ENV, abs.into_os_string()));
        }
    }
    if global.touch_project_state && anvil_home_active && !install_root::env_touch_is_truthy() {
        overrides.push((install_root::TOUCH_PROJECT_STATE_ENV, OsString::from("1")));
    }

    if overrides.is_empty() {
        return None; // environment already reflects the flags — run in-process
    }

    let Ok(exe) = std::env::current_exe() else {
        eprintln!("anvil: --anvil-home unavailable: cannot resolve current executable");
        return Some(ExitCode::from(EXIT_ERROR));
    };
    let mut cmd = std::process::Command::new(exe);
    cmd.args(std::env::args_os().skip(1));
    for (key, value) in &overrides {
        cmd.env(key, value);
    }
    match cmd.status() {
        Ok(status) => Some(ExitCode::from(exit_status_to_code(status))),
        Err(err) => {
            eprintln!("anvil: failed to apply --anvil-home override: {err}");
            Some(ExitCode::from(EXIT_ERROR))
        }
    }
}

/// Map a re-exec'd child's `ExitStatus` to this process's exit code. A clean exit
/// forwards the child's code; on Unix a signal death is translated to the
/// conventional `128 + signum` (e.g. 130 for SIGINT) so a Ctrl-C on a long
/// `anvil watch --anvil-home …` reports the usual code rather than a bare 1.
fn exit_status_to_code(status: std::process::ExitStatus) -> u8 {
    if let Some(code) = status.code() {
        return u8::try_from(code).unwrap_or(EXIT_ERROR);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return u8::try_from(128 + signal).unwrap_or(EXIT_ERROR);
        }
    }
    EXIT_ERROR
}

#[allow(clippy::too_many_lines)] // dispatch table; splitting harms readability
fn main() -> ExitCode {
    let cli = match try_parse_cli() {
        Ok(cli) => cli,
        Err(err) => {
            let code = err.exit_code();
            if wants_json() && code != 0 {
                eprintln!("{}", serde_json::json!({ "error": err.to_string() }));
            } else {
                let _ = err.print();
            }
            return ExitCode::from(u8::try_from(code).unwrap_or(EXIT_ERROR));
        }
    };

    // DISTRIB-006: apply the `--anvil-home` / `--touch-project-state` override
    // before any heavy init or daemon spawn. The crate forbids `unsafe_code`
    // (no `set_var`), so a flag that changes the effective environment re-execs
    // this binary once with the variable set in the child's env — the env is the
    // single channel every consumer (in-process resolvers, the inherited daemon)
    // reads. The flag takes precedence over a pre-existing `ANVIL_HOME`.
    if let Some(exit_code) = reexec_for_install_root(&cli.global) {
        return exit_code;
    }

    // V050F-007: cap rayon's global pool at half available cores
    // BEFORE any subcommand can dispatch to a rayon-using path
    // (`anvil check`, `anvil watch`, the secret/antipattern scanners,
    // `scan_artifact`, etc.). Pre-V050F-007 the kernel's defensive
    // `POOL_INIT.call_once` blocks were no-ops if a non-kernel path
    // (e.g. `scan_artifact` from `anvil-checks`) drove rayon's first
    // `par_iter` — rayon defaulted to `num_cpus::get()` and the cap
    // was silently absent. Calling it from `main` first guarantees
    // the cap is always in force.
    anvil_rayon_init::init_global();

    // TRACE-001: install the cross-cutting tracing subscriber once at
    // process start. `Err` means a global subscriber was already
    // registered (test harness, parent context, or a misbehaving
    // dependency); the CLI continues on that subscriber but surfaces
    // the condition to stderr so an operator can diagnose missing
    // spans rather than silently losing observability.
    if let Err(err) = anvil_observability::init_tracing(anvil_observability::BinaryKind::Cli) {
        eprintln!("anvil: tracing subscriber init skipped: {err}");
    }

    let command_name = command_canonical_name(&cli.command);
    let cli_span = tracing::info_span!(
        target: "anvil_cli",
        "cli.command",
        command = command_name,
        json = cli.global.json,
        no_tui = cli.global.no_tui,
        verbose = cli.global.verbose,
    );
    let _cli_span_guard = cli_span.enter();
    tracing::info!(target: "anvil_cli", "cli command parsed");

    // USAGE-002: open the flag-capture window before the auth/routing
    // phase so the usage row records the flags resolved while authorising
    // (e.g. `cli.licence-gate`). The daemon never opens a window, so this
    // is a no-op off the CLI path.
    anvil_kernel::feature_flags::begin_flag_capture();

    let wants_json = cli.global.json || command_requests_structured_output(&cli.command);
    let auth_outcome = if requires_auth(&cli.command) && !skips_auth_for_local_probe(&cli.command) {
        check_auth(
            &cli.global,
            allows_interactive_auth_prompt(&cli.command),
            wants_json,
        )
    } else {
        Ok(())
    };

    // USAGE-001/-002: record one durable `command.invoked` row per
    // user-initiated invocation, AFTER the auth/routing phase so
    // `flag_set` carries the flags resolved while authorising. Emitted on
    // both the auth-pass and auth-fail paths so every invocation gets
    // exactly one row, and before dispatch so the command's own
    // `process::exit` paths cannot drop it. Strictly best-effort: a
    // usage-write failure is logged and dropped so it never changes the
    // command's behaviour or exit code.
    // USAGE-004: a non-dry-run `intercept unblock` records its
    // authoritative row from the daemon dispatch path; suppress the
    // generic CLI-side `intercept` row for it so the operator action is
    // counted once (a dry-run unblock contacts no daemon, so its CLI row
    // is kept — see InterceptArgs::suppresses_cli_usage_row).
    let suppress_cli_usage_row =
        matches!(&cli.command, Commands::Intercept(args) if args.suppresses_cli_usage_row());
    if !suppress_cli_usage_row && let Err(err) = usage::record_invocation(command_name) {
        tracing::warn!(
            target: "anvil_cli",
            error = %err,
            "usage: failed to record command-invocation observation; continuing",
        );
    }

    if let Err((code, kind)) = auth_outcome {
        if code == EXIT_AUTH_REQUIRED {
            // CIB-061: `info`, not `warn` — auth-required is an expected
            // state (issue #1822) and `check_auth` already put the human
            // message on stderr; at `warn` the event passed the CLI's
            // default filter and leaked a raw JSON line under that message.
            // `ANVIL_LOG=info` still surfaces it for operators.
            tracing::info!(target: "anvil_cli", "cli command authentication required");
        } else {
            // Real runtime failures from `check_auth` (failed interactive
            // login, credential-store read fault) — distinct from the
            // expected "not logged in yet" state.
            tracing::warn!(
                target: "anvil_cli",
                exit_code = code,
                "cli auth check failed"
            );
        }
        let (exit_code, json_envelope) =
            auth_required_response(&cli.command, code, wants_json, kind);
        if let Some(envelope) = json_envelope {
            // CIB-049: the envelope only exists under `--json` / `--format
            // json`, and structured output belongs on stdout (stream policy,
            // `docs/guides/cli-output-streams.md`) — a JSON consumer piping
            // stdout must receive it. Exit-code routing is unchanged.
            println!("{envelope}");
        }
        return ExitCode::from(exit_code);
    }

    // Update --check returns UpdateAvailable error when an update exists (exit 1).
    if let Commands::Update(args) = &cli.command {
        return match commands::update::run(args, &cli.global) {
            Ok(()) => ExitCode::from(EXIT_OK),
            Err(err) if err.is::<commands::update::UpdateAvailable>() => ExitCode::from(EXIT_ERROR),
            Err(err) => {
                if wants_json {
                    eprintln!("{}", serde_json::json!({ "error": format!("{err:#}") }));
                } else {
                    eprintln!("Error: {err:#}");
                }
                ExitCode::from(EXIT_ERROR)
            }
        };
    }

    // Gate returns Result<bool> (false = gate failed); all others return Result<()>.
    if let Commands::Gate(args) = &cli.command {
        return match commands::gate::run(args, &cli.global) {
            Ok(true) => ExitCode::from(EXIT_OK),
            Ok(false) => ExitCode::from(EXIT_GATE_FAIL),
            Err(err) => {
                // Match the auth gate: a `--format json|sarif` consumer gets a
                // structured error envelope, not a human `Error:` line.
                if wants_json {
                    eprintln!("{}", serde_json::json!({ "error": format!("{err:#}") }));
                } else {
                    eprintln!("Error: {err:#}");
                }
                ExitCode::from(EXIT_ERROR)
            }
        };
    }

    let result = match &cli.command {
        Commands::Audit(args) => commands::audit::run(args, &cli.global),
        Commands::AuditChain(args) => commands::audit_chain::run(args, &cli.global),
        Commands::Check(args) => commands::check::run(args, &cli.global),
        Commands::ReportFp(args) => commands::report_fp::run(args, &cli.global),
        Commands::Doctor(args) => commands::doctor::run(args, &cli.global),
        Commands::Config(args) => commands::config::run(args, &cli.global),
        Commands::Drift(args) => commands::drift::run(args, &cli.global),
        Commands::Edda(args) => commands::edda::run(args, &cli.global),
        Commands::Ember(args) => commands::ember::run(args, &cli.global),
        Commands::Start(args) => commands::start::run(args, &cli.global),
        Commands::Status(args) => commands::status::run(args, &cli.global),
        Commands::Tutorial(args) => commands::tutorial::run(args, &cli.global),
        Commands::Welcome(args) => commands::welcome::run(args, &cli.global),
        Commands::Init(args) => commands::init::run(args, &cli.global),
        Commands::Insights(args) => commands::insights::run(args, &cli.global),
        Commands::Kindling(args) => commands::kindling::run(args, &cli.global),
        Commands::Migrate(args) => commands::migrate::run(args, &cli.global),
        Commands::Intercept(args) => commands::intercept::run(args, &cli.global),
        Commands::Workspace(args) => commands::workspace::run(args, &cli.global),
        Commands::L4Validate(args) => commands::l4_validate::run(args, &cli.global),
        Commands::Licenses(args) => commands::licenses::run(args, &cli.global),
        Commands::McpConfig(args) => commands::mcp_config::run(args, &cli.global),
        Commands::Mcp(args) => commands::mcp::run(args, &cli.global),
        Commands::Plan(args) => commands::plan::run(args, &cli.global),
        Commands::Dashboard(args) => commands::dashboard::run(args, &cli.global),
        Commands::New(args) => commands::new::run(args, &cli.global),
        Commands::Wizard(args) => commands::wizard::run(args, &cli.global),
        Commands::Admin(args) => commands::admin::run(args, &cli.global),
        Commands::Auth(args) => commands::auth::run(args, &cli.global),
        Commands::Update(_) | Commands::Gate(_) => unreachable!("handled above"),
        Commands::GateConfig(args) => commands::gate_config::run(args, &cli.global),
        Commands::Watch(args) => commands::watch::run(args, &cli.global),
        Commands::Export(args) => commands::export::run(args, &cli.global),
        Commands::Hooks(args) => commands::hooks::run(args, &cli.global),
        Commands::Hook(args) => commands::hook::run(args, &cli.global),
        Commands::Uninstall(args) => commands::uninstall::run(args, &cli.global),
        Commands::Baseline(args) => commands::baseline::run(args, &cli.global),
        Commands::Capsule(args) => commands::capsule::run(args, &cli.global),
        Commands::Architecture(args) => commands::architecture::run(args, &cli.global),
        Commands::Policy(args) => commands::policy::run(args, &cli.global),
        Commands::Gctx(args) => commands::gctx::run(args, &cli.global),
        Commands::Validate(args) => commands::validate::run(args, &cli.global),
        Commands::Version(args) => commands::version::run(args, &cli.global),
        Commands::Login(args) => commands::auth::run_login(args, &cli.global),
        Commands::Logout(args) => commands::auth::run_logout(args, &cli.global),
        Commands::Whoami(args) => commands::auth::run_whoami(args, &cli.global),
    };

    match result {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(err) => {
            if err.is::<output::AlreadyReported>() {
                return ExitCode::from(EXIT_ERROR);
            }
            if err.is::<output::AuthRequired>() {
                return ExitCode::from(EXIT_AUTH_REQUIRED);
            }
            // Universal runtime-error envelope for action commands. Honour a
            // per-command `--format json|sarif` (not just global `--json`) so
            // a structured consumer of `check`/`audit` gets a JSON error
            // envelope rather than a human `Error:` line — same contract as
            // the pre-dispatch auth gate above.
            if wants_json {
                eprintln!("{}", serde_json::json!({ "error": format!("{err:#}") }));
            } else {
                eprintln!("Error: {err:#}");
            }
            ExitCode::from(EXIT_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a `Commands` variant from CLI-style tokens.
    fn parse_command(args: &[&str]) -> Commands {
        let mut tokens = vec!["anvil"];
        tokens.extend_from_slice(args);
        Cli::try_parse_from(tokens).unwrap().command
    }

    // ── exit-code constants (CLIC-001 / A7.3) ────────────────────────
    //
    // Pin the numeric values so silent renumbering can't break CI /
    // tooling that gates on specific exit codes. The contract is
    // documented in plans/specs/2026-05-07-cli-surface-coherence.md §3.

    #[test]
    fn exit_code_constants_pin_canonical_values() {
        assert_eq!(EXIT_OK, 0);
        assert_eq!(EXIT_ERROR, 1);
        assert_eq!(EXIT_GATE_FAIL, 2);
        assert_eq!(EXIT_AUTH_REQUIRED, 3);
        assert_eq!(EXIT_CONFIG_ERROR, 4);
        assert_eq!(EXIT_CROSS_BOUNDARY, 5);
        assert_eq!(EXIT_DAEMON_DOWN, 6);
        assert_eq!(EXIT_VERSION_MISMATCH, 7);
        assert_eq!(EXIT_DISCOVERY_FAILED, 10);
    }

    #[test]
    fn exit_code_constants_are_distinct() {
        // Defense-in-depth: detect accidental aliasing if any two
        // constants ever drift to the same value.
        let codes = [
            EXIT_OK,
            EXIT_ERROR,
            EXIT_GATE_FAIL,
            EXIT_AUTH_REQUIRED,
            EXIT_CONFIG_ERROR,
            EXIT_CROSS_BOUNDARY,
            EXIT_DAEMON_DOWN,
            EXIT_VERSION_MISMATCH,
            EXIT_DISCOVERY_FAILED,
        ];
        for (i, a) in codes.iter().enumerate() {
            for b in codes.iter().skip(i + 1) {
                assert_ne!(
                    a, b,
                    "exit code constants must be distinct: collision on value {a}"
                );
            }
        }
    }

    #[test]
    fn help_layout_inventory_lists_visible_command_paths() {
        use clap::CommandFactory;

        let paths = help_layout::visible_command_paths(&Cli::command());

        assert!(help_layout::contains_path(&paths, &["anvil", "check"]));
        assert!(help_layout::contains_path(
            &paths,
            &["anvil", "auth", "login"]
        ));
        assert!(help_layout::contains_path(
            &paths,
            &["anvil", "policy", "eval"]
        ));
        assert!(!help_layout::contains_path(&paths, &["anvil", "login"]));
        assert!(paths.len() > registered_command_names().len());
    }

    #[test]
    fn clic_010_lint_covers_visible_command_paths() {
        use clap::CommandFactory;

        let findings = help_layout::lint_clic_010_layout(&Cli::command());

        assert!(
            findings.is_empty(),
            "CLIC-010 help layout drift:\n{}",
            findings
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn clic_010_help_excludes_internal_identifiers() {
        let findings = help_layout::lint_internal_identifiers(&augmented_cli_command());
        assert!(
            findings.is_empty(),
            "user-visible help must not leak internal identifiers (CLIC-010):\n{}",
            findings.join("\n")
        );
    }

    #[test]
    fn augmented_help_adds_clic_010_sections_to_check() {
        let mut command = augmented_cli_command();
        let help = command
            .find_subcommand_mut("check")
            .expect("check command exists")
            .render_long_help()
            .to_string();
        assert!(
            help.contains("WHEN TO USE:"),
            "missing when-to-use:\n{help}"
        );
        assert!(
            help.contains("COMMON FLAGS:"),
            "missing common flags:\n{help}"
        );
        assert!(help.contains("LEARN MORE:"), "missing learn more:\n{help}");
        assert!(
            help.contains("docs/runbooks/cli-surface.md#anvil-check"),
            "missing docs pointer:\n{help}"
        );
    }

    #[test]
    fn augmented_help_preserves_existing_watch_footer() {
        let mut command = augmented_cli_command();
        let help = command
            .find_subcommand_mut("watch")
            .expect("watch command exists")
            .render_long_help()
            .to_string();
        assert!(
            help.contains("ANVIL_WATCH_DAEMON"),
            "existing watch footer lost:\n{help}"
        );
        assert!(
            help.contains("WHEN TO USE:"),
            "missing when-to-use:\n{help}"
        );
    }

    #[test]
    fn augmented_parse_preserves_command_dispatch() {
        for args in [
            vec!["anvil", "check"],
            vec!["anvil", "config", "show"],
            vec!["anvil", "gate"],
        ] {
            let raw = Cli::try_parse_from(&args).expect("raw parse");
            let augmented = try_parse_cli_from(args.iter().copied()).expect("augmented parse");
            assert_eq!(
                command_canonical_name(&raw.command),
                command_canonical_name(&augmented.command),
                "dispatch drift for {args:?}"
            );
        }
    }

    // ── 094c: full-registry usage-producer coverage ─────────────────
    //
    // The USAGE-001 dossier claims the conformance test iterates the
    // registered command list so that adding a command without an
    // observation fails CI. The end-to-end test
    // (`every_runnable_sampled_command_emits_exactly_one_row` in
    // `tests/usage_observation.rs`) can only cover the commands that parse
    // and run with no extra args from an out-of-process spawn. This
    // in-process test closes the gap by iterating the WHOLE registry: it
    // asserts that every registered top-level command maps to a non-empty
    // canonical name via `command_canonical_name` — the value the
    // unconditional `record_invocation` producer stamps on the row. A new
    // subcommand added to `Commands` without a parse recipe here fails the
    // completeness assertion, forcing a deliberate decision (the spec's
    // "adding a command without an observation fails the test" guarantee).

    /// Minimal CLI tokens that make each registered top-level command parse
    /// to a `Commands` variant. Parent commands that require a subcommand
    /// get one; commands that need a positional get a throwaway value.
    /// `command_canonical_name` keys off the top-level variant only, so the
    /// specific subcommand/positional does not affect the recorded name.
    fn parse_recipe(command: &str) -> Vec<&'static str> {
        match command {
            "audit" => vec!["audit"],
            "audit-chain" => vec!["audit-chain"],
            "check" => vec!["check", "--all"],
            "report-fp" => vec!["report-fp", "ANV-CORE-001", "src/x.rs:1"],
            "doctor" => vec!["doctor"],
            "config" => vec!["config", "show"],
            "drift" => vec!["drift", "snapshot"],
            "edda" => vec!["edda", "list"],
            "ember" => vec!["ember", "list"],
            "start" => vec!["start"],
            "status" => vec!["status"],
            "tutorial" => vec!["tutorial"],
            "welcome" => vec!["welcome"],
            "init" => vec!["init"],
            "insights" => vec!["insights"],
            "kindling" => vec!["kindling", "usage", "top"],
            "migrate" => vec!["migrate", "format"],
            "intercept" => vec!["intercept", "status"],
            "workspace" => vec!["workspace", "mode", "open"],
            "l4-validate" => vec!["l4-validate", "HEAD~1..HEAD"],
            "licenses" => vec!["licenses"],
            "mcp-config" => vec!["mcp-config", "--target", "claude-code"],
            "mcp" => vec!["mcp", "serve"],
            "plan" => vec!["plan", "dashboard"],
            "dashboard" => vec!["dashboard"],
            "new" => vec!["new", "demo"],
            "wizard" => vec!["wizard"],
            "admin" => vec!["admin", "list"],
            "gate" => vec!["gate"],
            "gate-config" => vec!["gate-config"],
            "watch" => vec!["watch"],
            "export" => vec!["export"],
            "hooks" => vec!["hooks", "install"],
            "hook" => vec!["hook", "pre-commit"],
            "baseline" => vec!["baseline", "verify"],
            "capsule" => vec![
                "capsule",
                "create",
                "--range",
                "HEAD~1..HEAD",
                "--out",
                "cap.tar",
            ],
            "architecture" => vec!["architecture", "validate"],
            "auth" => vec!["auth", "login"],
            "policy" => vec!["policy", "eval", "policy.yaml"],
            "gctx" => vec!["gctx", "egress", "status"],
            "update" => vec!["update"],
            "uninstall" => vec!["uninstall"],
            "validate" => vec!["validate", "plan.aps.md"],
            "version" => vec!["version"],
            // Unknown command: no recipe — the completeness assertion below
            // turns this into a hard failure so a new command cannot be
            // added without confirming the usage producer covers it.
            _ => Vec::new(),
        }
    }

    #[test]
    fn registered_commands_all_have_canonical_names() {
        for command in registered_command_names() {
            let recipe = parse_recipe(&command);
            assert!(
                !recipe.is_empty(),
                "new registered command {command:?} has no parse recipe in \
                 `parse_recipe`: add one so the usage-producer coverage test \
                 (094c) confirms `record_invocation` records a row for it",
            );
            let mut tokens = vec!["anvil"];
            tokens.extend_from_slice(&recipe);
            let cli = Cli::try_parse_from(&tokens)
                .unwrap_or_else(|e| panic!("recipe for {command:?} must parse: {e}"));
            let canonical = command_canonical_name(&cli.command);
            assert!(
                !canonical.is_empty(),
                "command {command:?} resolved to an empty canonical name; the \
                 usage producer would record a nameless row",
            );
        }
    }

    // ── requires_auth: commands that MUST require auth ──────────────

    #[test]
    fn requires_auth_check() {
        assert!(requires_auth(&parse_command(&["check", "--all"])));
    }

    #[test]
    fn requires_auth_drift() {
        assert!(requires_auth(&parse_command(&["drift", "list"])));
    }

    #[test]
    fn requires_auth_gate_config() {
        assert!(requires_auth(&parse_command(&["gate-config", "--list"])));
    }

    #[test]
    fn requires_auth_gate() {
        assert!(requires_auth(&parse_command(&["gate"])));
    }

    #[test]
    fn requires_auth_watch() {
        assert!(requires_auth(&parse_command(&["watch"])));
    }

    #[test]
    fn requires_auth_status() {
        assert!(requires_auth(&parse_command(&["status"])));
    }

    #[test]
    fn requires_auth_export() {
        assert!(requires_auth(&parse_command(&["export"])));
    }

    #[test]
    fn requires_auth_audit() {
        assert!(requires_auth(&parse_command(&["audit"])));
    }

    #[test]
    fn requires_auth_architecture() {
        assert!(requires_auth(&parse_command(&["architecture", "validate"])));
    }

    #[test]
    fn requires_auth_policy() {
        assert!(requires_auth(&parse_command(&["policy", "list"])));
    }

    #[test]
    fn requires_auth_whoami_alias() {
        assert!(requires_auth(&parse_command(&["whoami"])));
    }

    #[test]
    fn requires_auth_auth_whoami() {
        assert!(requires_auth(&parse_command(&["auth", "whoami"])));
    }

    // ── requires_auth: commands that bypass auth ────────────────────

    #[test]
    fn bypass_auth_doctor() {
        assert!(!requires_auth(&parse_command(&["doctor"])));
    }

    #[test]
    fn bypass_auth_tutorial() {
        assert!(!requires_auth(&parse_command(&["tutorial"])));
    }

    #[test]
    fn requires_auth_welcome() {
        // ADR-080 (UJ-004): `welcome` is the ungated beta demo surface — the
        // discovery path shows value before the licence gate; durable
        // surfaces (`init`, `start`, `watch`) stay gated.
        assert!(!requires_auth(&parse_command(&["welcome"])));
    }

    #[test]
    fn requires_auth_start() {
        // LAUNCH-006: `start` is its own command; gated like
        // `init` / `status` / `watch`. Pre-LAUNCH-006 this test was
        // `requires_auth_start_alias` and asserted the alias behaviour.
        assert!(requires_auth(&parse_command(&["start"])));
    }

    #[test]
    fn requires_auth_init() {
        assert!(requires_auth(&parse_command(&["init"])));
    }

    #[test]
    fn bypass_auth_intercept() {
        // INTD-001 scaffold: `anvil intercept start` is a daemon
        // launcher and must not be gated behind the licence-gate
        // flag's auth list. If a future flag-config change accidentally
        // enrols `intercept`, this test pins the regression.
        assert!(!requires_auth(&parse_command(&[
            "intercept",
            "start",
            "--foreground",
        ])));
    }

    #[test]
    fn bypass_auth_licenses() {
        assert!(!requires_auth(&parse_command(&["licenses"])));
    }

    #[test]
    fn bypass_auth_insights() {
        // INSIGHTS-001 is explicitly local-only and reads only in-repo
        // witness evidence, so users can check value signals without
        // a network/auth dependency.
        assert!(!requires_auth(&parse_command(&["insights"])));
    }

    #[test]
    fn bypass_auth_kindling() {
        // USAGE-003: `anvil kindling usage <view>` is local-only and reads
        // only the user-scoped usage sidecar, like `insights`. It must not
        // be gated; this pins the regression if a future flag-config
        // change accidentally enrols `kindling` in the licence-gate list.
        assert!(!requires_auth(&parse_command(&[
            "kindling", "usage", "top"
        ])));
    }

    #[test]
    fn requires_auth_new() {
        assert!(requires_auth(&parse_command(&["new"])));
    }

    #[test]
    fn requires_auth_wizard() {
        assert!(requires_auth(&parse_command(&["wizard"])));
    }

    #[test]
    fn requires_auth_mcp_config() {
        assert!(requires_auth(&parse_command(&[
            "mcp-config",
            "--target",
            "cursor",
        ])));
    }

    #[test]
    fn requires_auth_mcp_install() {
        assert!(requires_auth(&parse_command(&[
            "mcp", "install", "--client", "cursor",
        ])));
    }

    #[test]
    fn bypass_auth_mcp_serve() {
        assert!(!requires_auth(&parse_command(
            &["mcp", "serve", "--stdio",]
        )));
    }

    #[test]
    fn bypass_auth_hooks() {
        assert!(!requires_auth(&parse_command(&["hooks", "install"])));
    }

    #[test]
    fn bypass_auth_update() {
        assert!(!requires_auth(&parse_command(&["update"])));
    }

    #[test]
    fn bypass_auth_update_check() {
        assert!(!requires_auth(&parse_command(&["update", "--check"])));
    }

    #[test]
    fn bypass_auth_uninstall() {
        // Uninstall is a recovery command. A user with broken or
        // expired credentials must still be able to clean up before
        // reinstalling. Pin this in tests so a future change to
        // `CLI_GATED_COMMANDS` or the canonical name cannot
        // accidentally regress it.
        assert!(!requires_auth(&parse_command(&["uninstall"])));
        assert!(!requires_auth(&parse_command(&["uninstall", "--global"])));
        assert!(!requires_auth(&parse_command(&["uninstall", "--dry-run",])));
    }

    #[test]
    fn bypass_auth_validate() {
        assert!(!requires_auth(&parse_command(&["validate", "plan.aps.md"])));
    }

    #[test]
    fn bypass_auth_plan_dashboard() {
        // The licence gate (customer plan tiers) does not apply to the APS
        // dashboard — it is an internal-developer surface. CIB-046 gates it
        // separately at dispatch via `feature_flags::aps_dashboard_access_allowed`
        // (default-disabled flag + ANVIL_DEV / ANVIL_ADMIN_KEY escape hatches),
        // not through `requires_auth`, so this stays `false`.
        assert!(!requires_auth(&parse_command(&["plan", "dashboard"])));
    }

    #[test]
    fn canonical_name_plan_dashboard() {
        assert_eq!(
            command_canonical_name(&parse_command(&["plan", "dashboard"])),
            "plan"
        );
    }

    #[test]
    fn bypass_auth_dashboard() {
        assert!(!requires_auth(&parse_command(&["dashboard"])));
        assert!(!requires_auth(&parse_command(&[
            "dashboard",
            "architecture"
        ])));
    }

    #[test]
    fn canonical_name_dashboard() {
        assert_eq!(
            command_canonical_name(&parse_command(&["dashboard"])),
            "dashboard"
        );
    }

    #[test]
    fn bypass_auth_login_alias() {
        assert!(!requires_auth(&parse_command(&["login"])));
    }

    #[test]
    fn bypass_auth_logout_alias() {
        assert!(!requires_auth(&parse_command(&["logout"])));
    }

    #[test]
    fn bypass_auth_auth_login() {
        assert!(!requires_auth(&parse_command(&["auth", "login"])));
    }

    #[test]
    fn bypass_auth_auth_logout() {
        assert!(!requires_auth(&parse_command(&["auth", "logout"])));
    }

    #[test]
    fn bypass_auth_admin() {
        // Admin authenticates via ANVIL_ADMIN_KEY, not personal credentials,
        // so the pre-action auth check is skipped; admin::run checks the
        // env var itself and exits with EXIT_AUTH_REQUIRED if missing.
        assert!(!requires_auth(&parse_command(&[
            "admin", "approve", "--batch", "1"
        ])));
    }

    // ── skips_auth_for_local_probe (CIB-049) ────────────────────────

    #[test]
    fn local_probe_skip_matches_status_verify() {
        assert!(skips_auth_for_local_probe(&parse_command(&[
            "status", "--verify"
        ])));
    }

    #[test]
    fn local_probe_skip_matches_start_verify() {
        // CIB-049: `start --verify` is the documented read-only sibling
        // of `status --verify` and must not hit the auth wall.
        assert!(skips_auth_for_local_probe(&parse_command(&[
            "start", "--verify"
        ])));
    }

    #[test]
    fn local_probe_skip_excludes_full_start_and_status() {
        // Full (mutating) `start` and plain `status` stay auth-gated.
        assert!(!skips_auth_for_local_probe(&parse_command(&["start"])));
        assert!(!skips_auth_for_local_probe(&parse_command(&["status"])));
    }

    // ── is_auth_state_probe / auth_required_response (#1822) ────────

    #[test]
    fn auth_state_probe_matches_whoami_alias() {
        assert!(is_auth_state_probe(&parse_command(&["whoami"])));
    }

    #[test]
    fn auth_state_probe_matches_auth_whoami() {
        assert!(is_auth_state_probe(&parse_command(&["auth", "whoami"])));
    }

    #[test]
    fn auth_state_probe_excludes_other_auth_subcommands() {
        assert!(!is_auth_state_probe(&parse_command(&["auth", "logout"])));
        assert!(!is_auth_state_probe(&parse_command(&["auth", "refresh"])));
    }

    #[test]
    fn auth_state_probe_excludes_action_commands() {
        // Regression pin: action commands must not be classified as
        // probes, or they'd inherit the exit-3 surface.
        for tokens in [
            &["welcome"][..],
            &["status"][..],
            &["start"][..],
            &["init"][..],
            &["gate"][..],
            &["audit"][..],
            &["watch"][..],
            &["check", "--all"][..],
            &["architecture", "validate"][..],
            &["drift", "list"][..],
            &["policy", "list"][..],
        ] {
            assert!(
                !is_auth_state_probe(&parse_command(tokens)),
                "action command {tokens:?} must not be an auth-state probe"
            );
        }
    }

    #[test]
    fn auth_required_response_action_command_exits_zero() {
        // Issue #1822: gated action commands treat auth-required as an
        // expected state and exit 0 so new users don't see what looks
        // like a crash.
        for tokens in [
            &["welcome"][..],
            &["status"][..],
            &["start"][..],
            &["init"][..],
            &["gate"][..],
            &["audit"][..],
            &["watch"][..],
        ] {
            let (code, envelope) = auth_required_response(
                &parse_command(tokens),
                EXIT_AUTH_REQUIRED,
                false,
                Some(AuthRequiredKind::NotAuthenticated),
            );
            assert_eq!(
                code, EXIT_OK,
                "{tokens:?} should exit 0 on auth-required (informational)"
            );
            assert!(
                envelope.is_none(),
                "text mode must not emit a JSON envelope"
            );
        }
    }

    #[test]
    fn auth_required_response_probe_keeps_exit_three() {
        // The canonical preflight: `whoami` / `auth whoami` carry the
        // auth signal in the exit code so scripts have a stable check.
        for tokens in [&["whoami"][..], &["auth", "whoami"][..]] {
            let (code, _) = auth_required_response(
                &parse_command(tokens),
                EXIT_AUTH_REQUIRED,
                false,
                Some(AuthRequiredKind::NotAuthenticated),
            );
            assert_eq!(
                code, EXIT_AUTH_REQUIRED,
                "{tokens:?} is an auth-state probe and must exit 3"
            );
        }
    }

    #[test]
    fn auth_required_response_action_json_envelope_shape() {
        let (code, envelope) = auth_required_response(
            &parse_command(&["start"]),
            EXIT_AUTH_REQUIRED,
            true,
            Some(AuthRequiredKind::NotAuthenticated),
        );
        assert_eq!(code, EXIT_OK);
        let envelope = envelope.expect("--json mode must emit an envelope");
        assert_eq!(envelope["state"], "authRequired");
        assert_eq!(envelope["next"], "anvil auth login");
        assert_eq!(envelope["earlyAccessUrl"], EARLY_ACCESS_URL);
        assert!(
            envelope["message"].as_str().is_some_and(
                |m| m.contains("Authentication required") && m.contains(EARLY_ACCESS_URL)
            ),
            "envelope must carry the human-readable message with early-access pointer"
        );
        // No `error` key on the informational envelope — distinguishes
        // the informational shape from the probe's error shape so
        // structured consumers can tell them apart.
        assert!(envelope.get("error").is_none());
    }

    #[test]
    fn auth_required_response_action_json_omits_early_access_for_expired() {
        let (_, envelope) = auth_required_response(
            &parse_command(&["start"]),
            EXIT_AUTH_REQUIRED,
            true,
            Some(AuthRequiredKind::SessionExpired),
        );
        let envelope = envelope.expect("--json mode must emit an envelope");
        assert_eq!(envelope["message"], AUTH_SESSION_EXPIRED_MESSAGE);
        assert!(envelope.get("earlyAccessUrl").is_none());
    }

    #[test]
    fn auth_required_response_probe_json_envelope_shape() {
        let (code, envelope) = auth_required_response(
            &parse_command(&["whoami"]),
            EXIT_AUTH_REQUIRED,
            true,
            Some(AuthRequiredKind::NotAuthenticated),
        );
        assert_eq!(code, EXIT_AUTH_REQUIRED);
        let envelope = envelope.expect("--json mode must emit an envelope");
        // Probe keeps the existing error-shaped envelope for backward
        // compatibility with whoami callers.
        assert_eq!(envelope["error"], "authentication_required");
    }

    #[test]
    fn auth_required_response_passes_through_non_auth_code() {
        // PR #1824 review feedback: a failed interactive login attempt
        // returns EXIT_ERROR from check_auth. The dispatcher must not
        // coerce that to 0 — it's a real runtime failure, distinct from
        // "user hasn't logged in yet". Pin the pass-through for every
        // non-EXIT_AUTH_REQUIRED code on both action commands and probes.
        for cmd_tokens in [&["start"][..], &["whoami"][..]] {
            for incoming in [EXIT_ERROR, EXIT_GATE_FAIL, EXIT_CONFIG_ERROR] {
                let (code, envelope) =
                    auth_required_response(&parse_command(cmd_tokens), incoming, false, None);
                assert_eq!(
                    code, incoming,
                    "{cmd_tokens:?} with incoming {incoming} must pass through"
                );
                assert!(envelope.is_none(), "text mode emits no envelope");
            }
        }
    }

    #[test]
    fn auth_required_response_non_auth_code_json_envelope_is_generic() {
        // Under --json the pass-through path emits a distinct error
        // envelope so structured consumers can tell a check failure
        // apart from the informational `authRequired` state.
        let (code, envelope) =
            auth_required_response(&parse_command(&["start"]), EXIT_ERROR, true, None);
        assert_eq!(code, EXIT_ERROR);
        let envelope = envelope.expect("--json mode must emit an envelope");
        assert_eq!(envelope["error"], "auth_check_failed");
        assert!(envelope.get("state").is_none());
    }

    // ── command_requests_structured_output ───────────────────────

    #[test]
    fn structured_output_detected_for_format_json_commands() {
        // The pre-dispatch auth gate must treat `--format json|sarif` on a
        // finding-emitting command as machine output, so the auth envelope
        // is structured rather than a human line on the wrong stream.
        for cmd in [
            &["check", "--all", "--format", "json"][..],
            &["check", "--all", "--format", "sarif"][..],
            &["audit", "--format", "json"][..],
            &["gate", "--format", "json"][..],
        ] {
            assert!(
                command_requests_structured_output(&parse_command(cmd)),
                "{cmd:?} should be detected as structured output"
            );
        }
    }

    #[test]
    fn structured_output_not_detected_for_human_or_absent_format() {
        for cmd in [
            &["check", "--all"][..],
            &["check", "--all", "--format", "plain"][..],
            &["check", "--all", "--format", "tui"][..],
            &["audit"][..],
            // A command with no `--format` surface is never structured here.
            &["status"][..],
        ] {
            assert!(
                !command_requests_structured_output(&parse_command(cmd)),
                "{cmd:?} should NOT be detected as structured output"
            );
        }
    }

    // ── evaluate_auth ────────────────────────────────────────────

    use crate::auth::credentials::Credentials;

    fn valid_creds() -> Credentials {
        Credentials {
            license: "tok".into(),
            refresh_token: None,
            email: None,
            expires_at: Some("2099-01-01T00:00:00Z".into()),
            is_edict: None,
        }
    }

    fn expired_creds() -> Credentials {
        Credentials {
            license: "tok".into(),
            refresh_token: None,
            email: None,
            expires_at: Some("2000-01-01T00:00:00Z".into()),
            is_edict: None,
        }
    }

    fn no_expiry_creds() -> Credentials {
        Credentials {
            license: "tok".into(),
            refresh_token: None,
            email: None,
            expires_at: None,
            is_edict: None,
        }
    }

    #[test]
    fn evaluate_auth_returns_err_when_no_credentials() {
        assert_eq!(
            evaluate_auth(&Ok(None), false, true),
            Err((EXIT_AUTH_REQUIRED, Some(AuthRequiredKind::NotAuthenticated),))
        );
    }

    #[test]
    fn auth_not_authenticated_message_names_early_access_channel() {
        assert!(AUTH_NOT_AUTHENTICATED_MESSAGE.contains(EARLY_ACCESS_URL));
        assert!(!AUTH_SESSION_EXPIRED_MESSAGE.contains(EARLY_ACCESS_URL));
        assert!(!AUTH_INVALID_EDICT_MESSAGE.contains(EARLY_ACCESS_URL));
    }

    #[test]
    fn evaluate_auth_returns_err_when_expired() {
        assert_eq!(
            evaluate_auth(&Ok(Some(expired_creds())), false, true),
            Err((EXIT_AUTH_REQUIRED, Some(AuthRequiredKind::SessionExpired))),
        );
    }

    #[test]
    fn evaluate_auth_returns_config_error_on_load_error() {
        // A genuine credential load fault (I/O error, corrupt file) must
        // surface as EXIT_CONFIG_ERROR — not be flattened into the normal
        // EXIT_AUTH_REQUIRED "not logged in" path (silent-degrade class,
        // PR #1721). `load()` folds a missing file into `Ok(None)`, so the
        // `Err` arm only ever represents a real fault.
        assert_eq!(
            evaluate_auth(&Err(anyhow::anyhow!("disk failure")), false, true),
            Err((EXIT_CONFIG_ERROR, None)),
        );
    }

    #[test]
    fn evaluate_auth_returns_ok_when_valid() {
        assert!(evaluate_auth(&Ok(Some(valid_creds())), false, true).is_ok());
    }

    #[test]
    fn evaluate_auth_returns_ok_when_no_expiry() {
        assert!(evaluate_auth(&Ok(Some(no_expiry_creds())), false, true).is_ok());
    }

    #[test]
    fn check_auth_bypasses_when_anvil_dev_set() {
        // ANVIL_DEV=1 should allow unauthenticated access for local testing.
        // Without credentials, auth normally fails — but not in dev mode.
        temp_env::with_var("ANVIL_DEV", Some("1"), || {
            assert!(
                check_auth(&GlobalArgs::default(), true, false).is_ok(),
                "ANVIL_DEV=1 should bypass auth check"
            );
        });
    }

    #[test]
    fn check_auth_does_not_bypass_without_anvil_dev() {
        // Env var absent — auth still required without credentials.
        // Tests run under cargo without a TTY on stdin, so the interactive
        // prompt is suppressed and we fall straight through to the error.
        temp_env::with_vars(
            [
                ("ANVIL_DEV", None),
                ("ANVIL_LICENSE", None),
                ("XDG_CONFIG_HOME", Some("/nonexistent/path")),
            ],
            || {
                assert_eq!(
                    check_auth(&GlobalArgs::default(), true, false),
                    Err((EXIT_AUTH_REQUIRED, Some(AuthRequiredKind::NotAuthenticated),))
                );
            },
        );
    }

    // ── should_offer_interactive_login ──────────────────────────────

    #[test]
    fn offer_login_true_when_missing_and_interactive() {
        let loaded: anyhow::Result<Option<Credentials>> = Ok(None);
        assert!(should_offer_interactive_login(
            /* machine_output */ false, /* tty_ok */ true, &loaded,
        ));
    }

    #[test]
    fn offer_login_true_when_expired_and_interactive() {
        let loaded: anyhow::Result<Option<Credentials>> = Ok(Some(expired_creds()));
        assert!(should_offer_interactive_login(false, true, &loaded));
    }

    #[test]
    fn offer_login_false_when_valid_creds() {
        let loaded: anyhow::Result<Option<Credentials>> = Ok(Some(valid_creds()));
        assert!(!should_offer_interactive_login(false, true, &loaded));
    }

    #[test]
    fn offer_login_false_when_machine_output_requested() {
        // covers both --json and --no-tui (caller OR's them).
        let loaded: anyhow::Result<Option<Credentials>> = Ok(None);
        assert!(!should_offer_interactive_login(true, true, &loaded));
    }

    #[test]
    fn offer_login_false_when_not_a_tty() {
        let loaded: anyhow::Result<Option<Credentials>> = Ok(None);
        assert!(!should_offer_interactive_login(false, false, &loaded));
    }

    #[test]
    fn offer_login_false_on_load_error() {
        let loaded: anyhow::Result<Option<Credentials>> = Err(anyhow::anyhow!("disk fault"));
        assert!(!should_offer_interactive_login(false, true, &loaded));
    }

    // ── is_non_interactive_env ──────────────────────────────────────

    #[test]
    fn non_interactive_env_detects_ci_true() {
        temp_env::with_vars(
            [
                ("ANVIL_NO_PROMPT", None),
                ("NONINTERACTIVE", None),
                ("CI", Some("true")),
                ("GIT_DIR", None),
                ("GIT_INDEX_FILE", None),
            ],
            || assert!(is_non_interactive_env()),
        );
    }

    #[test]
    fn non_interactive_env_detects_anvil_no_prompt() {
        temp_env::with_vars(
            [
                ("ANVIL_NO_PROMPT", Some("1")),
                ("CI", None),
                ("GIT_DIR", None),
                ("GIT_INDEX_FILE", None),
            ],
            || assert!(is_non_interactive_env()),
        );
    }

    #[test]
    fn non_interactive_env_detects_git_hook_signals() {
        temp_env::with_vars(
            [
                ("ANVIL_NO_PROMPT", None),
                ("CI", None),
                ("GIT_DIR", Some(".git")),
                ("GIT_INDEX_FILE", None),
            ],
            || assert!(is_non_interactive_env()),
        );
    }

    #[test]
    fn non_interactive_env_false_when_clean() {
        temp_env::with_vars(
            [
                ("ANVIL_NO_PROMPT", None::<&str>),
                ("NONINTERACTIVE", None),
                ("CI", None),
                ("GIT_DIR", None),
                ("GIT_INDEX_FILE", None),
            ],
            || assert!(!is_non_interactive_env()),
        );
    }

    #[test]
    fn non_interactive_env_detects_empty_string_opt_out() {
        // `export ANVIL_NO_PROMPT=` should still count as opt-out —
        // presence of the variable is the signal, not its value.
        temp_env::with_vars(
            [
                ("ANVIL_NO_PROMPT", Some("")),
                ("NONINTERACTIVE", None),
                ("CI", None),
                ("GIT_DIR", None),
                ("GIT_INDEX_FILE", None),
            ],
            || assert!(is_non_interactive_env()),
        );
    }

    #[test]
    fn non_interactive_env_ignores_ci_false() {
        temp_env::with_vars(
            [
                ("ANVIL_NO_PROMPT", None),
                ("NONINTERACTIVE", None),
                ("CI", Some("false")),
                ("GIT_DIR", None),
                ("GIT_INDEX_FILE", None),
            ],
            || assert!(!is_non_interactive_env()),
        );
    }

    // ── allows_interactive_auth_prompt ──────────────────────────────

    #[test]
    fn whoami_alias_does_not_allow_interactive_prompt() {
        assert!(!allows_interactive_auth_prompt(&parse_command(&["whoami"])));
    }

    #[test]
    fn auth_whoami_does_not_allow_interactive_prompt() {
        assert!(!allows_interactive_auth_prompt(&parse_command(&[
            "auth", "whoami"
        ])));
    }

    #[test]
    fn other_commands_allow_interactive_prompt() {
        assert!(allows_interactive_auth_prompt(&parse_command(&[
            "check", "--all"
        ])));
        assert!(allows_interactive_auth_prompt(&parse_command(&["status"])));
    }
}
