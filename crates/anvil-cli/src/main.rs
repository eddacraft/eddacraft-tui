mod activation;
mod auth;
mod capacity;
mod commands;
mod config_summary;
mod config_view;
mod feature_flags;
mod insights;
mod l4_engine;
mod mcp;
mod output;
mod plan_dashboard;
mod services;
mod tui;
mod update_hint;
mod util;
mod warmup_cache;

use std::io::IsTerminal;
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};

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
/// CI / scripts that gate on Anvil exit codes can rely on this map:
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

/// Global arguments available to every subcommand.
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
}

/// Anvil — structural governance for AI-assisted development.
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

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a full project audit.
    Audit(commands::audit::AuditArgs),
    /// L5 witness-chain audit — walk the branch and report commits
    /// that lack an L3 witness. Catches bypassed protection (admin
    /// overrides, force-push manipulation). Nightly via the
    /// `anvil-audit` workflow template; on-demand from the CLI.
    AuditChain(commands::audit_chain::AuditChainArgs),
    /// Scan files for anti-patterns and hardcoded secrets (planless mode).
    ///
    /// Honours `.anvilrc#checks` (and `.anvil.<ext>`) for the
    /// planless-eligible subset: `antipattern-scan` and `secret-detection`.
    /// Profile-based or config-heavy checks (`architecture`, `policy`,
    /// `import-boundaries`, `command-safety`, `lint`, `test`, `coverage`,
    /// `dependency`) live under `anvil gate`.
    Check(commands::check::CheckArgs),
    /// Run diagnostic checks on your environment.
    Doctor(commands::doctor::DoctorArgs),
    /// Show, set, and convert Anvil project config.
    Config(commands::config::ConfigArgs),
    /// Track architecture drift over time.
    Drift(commands::drift::DriftArgs),
    /// List, show, and trace Edda canonical memories.
    Edda(commands::edda::EddaArgs),
    /// Show project status and health.
    Status(commands::status::StatusArgs),
    /// Activate Anvil in this repository. Writes `.anvilrc` if missing
    /// and installs MCP config entries for Cursor and Claude Code into
    /// your home directory (`~/.cursor/mcp.json`, `~/.claude.json`).
    /// Pass `--verify` to run a read-only probe instead.
    Start(commands::start::StartArgs),
    /// Interactive guided tutorial.
    Tutorial(commands::tutorial::TutorialArgs),
    /// Show the welcome screen with quick-start options.
    Welcome(commands::welcome::WelcomeArgs),
    /// Initialise Anvil configuration for a project.
    Init(commands::init::InitArgs),
    /// Show local-only weekly activity insights.
    Insights(commands::insights::InsightsArgs),
    /// MLP2-040 — migrate a legacy `.anvilrc` to the multi-format
    /// `.anvil.<ext>` surface from MLP-011. Existing `.anvilrc` projects
    /// keep working through gate's fallback; this command is the one-shot
    /// bridge for operators who want to land on the new format without
    /// hand-editing.
    Migrate(commands::migrate::MigrateArgs),
    /// Manage the Anvil intercept daemon.
    Intercept(commands::intercept::InterceptArgs),
    /// MLP2-046: validate one or more commits against `anvil/policy.yml`
    /// using the L4 rule engine. Dedicated binary surface for CI /
    /// Marketplace lanes that don't sit inside git's pre-push hook.
    #[command(name = "l4-validate")]
    L4Validate(commands::l4_validate::L4ValidateArgs),
    /// Show Anvil's acknowledgements and third-party licence attribution.
    Licenses(commands::licenses::LicensesArgs),
    /// Generate MCP server configuration for AI editors (claude-code, cursor, windsurf, vscode).
    #[command(name = "mcp-config")]
    McpConfig(commands::mcp_config::McpConfigArgs),
    /// Manage and serve MCP integrations.
    Mcp(commands::mcp::McpArgs),
    /// Inspect APS planning state.
    Plan(commands::plan::PlanArgs),
    /// Open a native read-only dashboard over local Anvil state.
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
    /// Manage architecture boundary definitions.
    Architecture(commands::architecture::ArchitectureArgs),
    /// Authenticate with the Anvil service.
    Auth(commands::auth::AuthArgs),
    /// Manage and evaluate policies.
    Policy(commands::policy::PolicyArgs),
    /// Update anvil to the latest version.
    Update(commands::update::UpdateArgs),
    /// Remove project Anvil state; use `--global` for user state and daemon.
    Uninstall(commands::uninstall::UninstallArgs),
    /// Validate an APS plan file (structure, task format, hash integrity).
    Validate(commands::validate::ValidateArgs),
    /// Show install-method-aware version + upgrade guidance.
    Version(commands::version::VersionArgs),
    /// Log in to Anvil (alias for `auth login`).
    #[command(hide = true)]
    Login(commands::auth::LoginArgs),
    /// Log out of Anvil (alias for `auth logout`).
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
        Commands::Doctor(_) => "doctor",
        Commands::Config(_) => "config",
        Commands::Drift(_) => "drift",
        Commands::Edda(_) => "edda",
        Commands::Start(_) => "start",
        Commands::Status(_) => "status",
        Commands::Tutorial(_) => "tutorial",
        Commands::Welcome(_) => "welcome",
        Commands::Init(_) => "init",
        Commands::Insights(_) => "insights",
        Commands::Migrate(_) => "migrate",
        Commands::Intercept(_) => "intercept",
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
        Commands::Architecture(_) => "architecture",
        Commands::Policy(_) => "policy",
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

/// Returns `true` for commands that require a valid auth session.
///
/// Delegates to the `cli.licence-gate` flag's gated-command metadata via
/// [`feature_flags::command_needs_licence_gate`]. FLAGM-006 retired the
/// legacy hard-coded match and its parity-test scaffolding; the flag is
/// now the sole source of truth.
fn requires_auth(cmd: &Commands) -> bool {
    feature_flags::command_needs_licence_gate(command_canonical_name(cmd))
}

fn skips_auth_for_local_probe(cmd: &Commands) -> bool {
    matches!(cmd, Commands::Status(args) if args.verify)
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
/// Returns `(exit_code, Some(json_envelope))` when `--json` is set, or
/// `(exit_code, None)` in text mode (stderr message already emitted by
/// `check_auth`).
fn auth_required_response(
    cmd: &Commands,
    code: u8,
    json_mode: bool,
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
        Some(serde_json::json!({
            "state": "authRequired",
            "message": "Authentication required. Run `anvil auth login` to authenticate.",
            "next": "anvil auth login",
        }))
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
) -> Result<(), u8> {
    match loaded {
        Ok(Some(creds)) if auth::credentials::is_expired(creds) => {
            if emit_human_messages {
                eprintln!("Session expired. Run `anvil auth login` to re-authenticate.");
            }
            Err(EXIT_AUTH_REQUIRED)
        }
        Ok(Some(creds)) if auth::credentials::is_edict(creds) => {
            if verify_edict_auth(creds, verbose, emit_human_messages) {
                Ok(())
            } else {
                if emit_human_messages {
                    eprintln!(
                        "Early-access edict is invalid or revoked. Run `anvil auth login --edict` to authenticate."
                    );
                }
                Err(EXIT_AUTH_REQUIRED)
            }
        }
        Ok(Some(_)) => Ok(()),
        Ok(None) => {
            if emit_human_messages {
                eprintln!("Authentication required. Run `anvil auth login` to authenticate.");
            }
            Err(EXIT_AUTH_REQUIRED)
        }
        Err(err) => {
            let msg = if verbose {
                format!("{err:#}")
            } else {
                format!("{err}")
            };
            // Redact home directory to avoid leaking paths in CI logs.
            let redacted = dirs::home_dir()
                .map(|h| msg.replace(h.to_string_lossy().as_ref(), "~"))
                .unwrap_or(msg);
            if emit_human_messages {
                eprintln!("[auth] credential load failed: {redacted}");
                eprintln!("Authentication required. Run `anvil auth login` to authenticate.");
            }
            Err(EXIT_AUTH_REQUIRED)
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
fn check_auth(global: &GlobalArgs, allow_interactive: bool) -> Result<(), u8> {
    // Local dev bypass: ANVIL_DEV=1 resolves through the shared resolver's
    // local-override precedence on `cli.licence-gate`. Routing via the
    // resolver (rather than an inline env-var read) means override
    // telemetry, reason codes, and future override sources all share one
    // code path. Safety rationale is unchanged from the legacy bypass:
    //   - All API calls still require a real token server-side.
    //   - This only bypasses the local credential pre-check.
    //   - Commands that call the API will fail with a 401 anyway.
    //   - Intended for CLI UX testing without a live token.
    if let Some(details) = feature_flags::cli_dev_bypass_active() {
        if !global.json {
            eprintln!(
                "[dev] ANVIL_DEV=1: local override {}={} (reason={:?}) — skipping local auth check",
                details.flag_key, details.variant, details.reason
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
        match try_silent_refresh(creds, global.verbose, !global.json) {
            SilentRefreshOutcome::Refreshed => loaded = auth::credentials::load(),
            SilentRefreshOutcome::PermanentFailure => {
                refresh_reason_already_printed = true;
            }
            SilentRefreshOutcome::TransientFailure => {}
        }
    }

    let suppress_interactive =
        global.json || global.no_tui || !allow_interactive || is_non_interactive_env();
    let tty_ok = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();

    if should_offer_interactive_login(suppress_interactive, tty_ok, &loaded) {
        let expired = matches!(&loaded, Ok(Some(c)) if auth::credentials::is_expired(c));
        if !refresh_reason_already_printed {
            if expired {
                eprintln!("Your Anvil session has expired.");
            } else {
                eprintln!("This command requires authentication with Anvil.");
            }
        }
        match prompt_yes_no("Log in now?", true) {
            Ok(true) => match run_interactive_login() {
                Ok(()) => {
                    // Re-validate freshly-written credentials before
                    // handing off to the command — guards against clock
                    // skew or partial writes that would otherwise silently
                    // pass the local gate and fail server-side.
                    return evaluate_auth(&auth::credentials::load(), global.verbose, !global.json);
                }
                Err(err) => {
                    // Distinct from EXIT_AUTH_REQUIRED: the user
                    // explicitly opted into the interactive login flow
                    // and it *failed* (device-flow error, network,
                    // credential save, etc.). This is a real runtime
                    // failure, not the "you haven't logged in yet"
                    // state — issue #1822 / PR #1824 review feedback.
                    eprintln!("Login failed: {err:#}");
                    return Err(EXIT_ERROR);
                }
            },
            Ok(false) => {
                eprintln!("Run `anvil auth login` when you're ready.");
                return Err(EXIT_AUTH_REQUIRED);
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
        return Err(EXIT_AUTH_REQUIRED);
    }

    evaluate_auth(&loaded, global.verbose, !global.json)
}

/// Check whether `--json` appears in raw args before clap parses them.
/// This lets us emit JSON errors even when clap rejects the input.
fn wants_json() -> bool {
    std::env::args().any(|a| a == "--json")
}

#[allow(clippy::too_many_lines)] // dispatch table; splitting harms readability
fn main() -> ExitCode {
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

    let cli = match Cli::try_parse() {
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

    if requires_auth(&cli.command)
        && !skips_auth_for_local_probe(&cli.command)
        && let Err(code) = check_auth(&cli.global, allows_interactive_auth_prompt(&cli.command))
    {
        tracing::warn!(target: "anvil_cli", "cli command authentication required");
        let (exit_code, json_envelope) =
            auth_required_response(&cli.command, code, cli.global.json);
        if let Some(envelope) = json_envelope {
            eprintln!("{envelope}");
        }
        return ExitCode::from(exit_code);
    }

    // Update --check returns UpdateAvailable error when an update exists (exit 1).
    if let Commands::Update(args) = &cli.command {
        return match commands::update::run(args, &cli.global) {
            Ok(()) => ExitCode::from(EXIT_OK),
            Err(err) if err.is::<commands::update::UpdateAvailable>() => ExitCode::from(EXIT_ERROR),
            Err(err) => {
                if cli.global.json {
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
                if cli.global.json {
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
        Commands::Doctor(args) => commands::doctor::run(args, &cli.global),
        Commands::Config(args) => commands::config::run(args, &cli.global),
        Commands::Drift(args) => commands::drift::run(args, &cli.global),
        Commands::Edda(args) => commands::edda::run(args, &cli.global),
        Commands::Start(args) => commands::start::run(args, &cli.global),
        Commands::Status(args) => commands::status::run(args, &cli.global),
        Commands::Tutorial(args) => commands::tutorial::run(args, &cli.global),
        Commands::Welcome(args) => commands::welcome::run(args, &cli.global),
        Commands::Init(args) => commands::init::run(args, &cli.global),
        Commands::Insights(args) => commands::insights::run(args, &cli.global),
        Commands::Migrate(args) => commands::migrate::run(args, &cli.global),
        Commands::Intercept(args) => commands::intercept::run(args, &cli.global),
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
        Commands::Architecture(args) => commands::architecture::run(args, &cli.global),
        Commands::Policy(args) => commands::policy::run(args, &cli.global),
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
            if cli.global.json {
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
        assert!(requires_auth(&parse_command(&["welcome"])));
    }

    #[test]
    fn requires_auth_start() {
        // LAUNCH-006: `start` is its own command; gated like `welcome` /
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
            let (code, envelope) =
                auth_required_response(&parse_command(tokens), EXIT_AUTH_REQUIRED, false);
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
            let (code, _) =
                auth_required_response(&parse_command(tokens), EXIT_AUTH_REQUIRED, false);
            assert_eq!(
                code, EXIT_AUTH_REQUIRED,
                "{tokens:?} is an auth-state probe and must exit 3"
            );
        }
    }

    #[test]
    fn auth_required_response_action_json_envelope_shape() {
        let (code, envelope) =
            auth_required_response(&parse_command(&["start"]), EXIT_AUTH_REQUIRED, true);
        assert_eq!(code, EXIT_OK);
        let envelope = envelope.expect("--json mode must emit an envelope");
        assert_eq!(envelope["state"], "authRequired");
        assert_eq!(envelope["next"], "anvil auth login");
        assert!(
            envelope["message"]
                .as_str()
                .is_some_and(|m| m.contains("Authentication required")),
            "envelope must carry the human-readable message"
        );
        // No `error` key on the informational envelope — distinguishes
        // the informational shape from the probe's error shape so
        // structured consumers can tell them apart.
        assert!(envelope.get("error").is_none());
    }

    #[test]
    fn auth_required_response_probe_json_envelope_shape() {
        let (code, envelope) =
            auth_required_response(&parse_command(&["whoami"]), EXIT_AUTH_REQUIRED, true);
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
                    auth_required_response(&parse_command(cmd_tokens), incoming, false);
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
        let (code, envelope) = auth_required_response(&parse_command(&["start"]), EXIT_ERROR, true);
        assert_eq!(code, EXIT_ERROR);
        let envelope = envelope.expect("--json mode must emit an envelope");
        assert_eq!(envelope["error"], "auth_check_failed");
        assert!(envelope.get("state").is_none());
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
            Err(EXIT_AUTH_REQUIRED)
        );
    }

    #[test]
    fn evaluate_auth_returns_err_when_expired() {
        assert_eq!(
            evaluate_auth(&Ok(Some(expired_creds())), false, true),
            Err(EXIT_AUTH_REQUIRED),
        );
    }

    #[test]
    fn evaluate_auth_returns_err_on_load_error() {
        assert_eq!(
            evaluate_auth(&Err(anyhow::anyhow!("disk failure")), false, true),
            Err(EXIT_AUTH_REQUIRED),
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
                check_auth(&GlobalArgs::default(), true).is_ok(),
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
                    check_auth(&GlobalArgs::default(), true),
                    Err(EXIT_AUTH_REQUIRED)
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
