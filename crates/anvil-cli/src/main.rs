mod auth;
mod commands;
mod output;
mod services;
mod tui;
mod util;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Exit codes for structured error reporting.
pub const EXIT_OK: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_GATE_FAIL: u8 = 2;
pub const EXIT_AUTH_REQUIRED: u8 = 3;
pub const EXIT_CONFIG_ERROR: u8 = 4;

/// Global arguments available to every subcommand.
#[derive(Debug, Parser)]
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
#[command(name = "anvil", version, about, long_about = None)]
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
    /// Analyse files for architecture violations and anti-patterns (planless mode).
    Check(commands::check::CheckArgs),
    /// Run diagnostic checks on your environment.
    Doctor(commands::doctor::DoctorArgs),
    /// Track architecture drift over time.
    Drift(commands::drift::DriftArgs),
    /// Show project status and health.
    Status(commands::status::StatusArgs),
    /// Interactive guided tutorial.
    Tutorial(commands::tutorial::TutorialArgs),
    /// Show the welcome screen with quick-start options.
    #[command(alias = "start")]
    Welcome(commands::welcome::WelcomeArgs),
    /// Initialise Anvil configuration for a project.
    Init(commands::init::InitArgs),
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
    /// Start file-watching mode with live gate checks.
    Watch(commands::watch::WatchArgs),
    /// Export constraints and configuration.
    Export(commands::export::ExportArgs),
    /// Install and manage git hooks.
    Hooks(commands::hooks::HooksArgs),
    /// Manage architecture boundary definitions.
    Architecture(commands::architecture::ArchitectureArgs),
    /// Authenticate with the Anvil service.
    Auth(commands::auth::AuthArgs),
    /// Manage and evaluate policies.
    Policy(commands::policy::PolicyArgs),
    /// Update anvil to the latest version.
    Update(commands::update::UpdateArgs),
    /// Validate an APS plan file (structure, task format, hash integrity).
    Validate(commands::validate::ValidateArgs),
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

/// Returns `true` for commands that require a valid auth session.
fn requires_auth(cmd: &Commands) -> bool {
    use commands::auth::AuthCommand;

    match cmd {
        // Auth-gated commands
        Commands::Audit(_)
        | Commands::Check(_)
        | Commands::Drift(_)
        | Commands::Status(_)
        | Commands::Admin(_)
        | Commands::Gate(_)
        | Commands::GateConfig(_)
        | Commands::Watch(_)
        | Commands::Export(_)
        | Commands::Architecture(_)
        | Commands::Policy(_)
        | Commands::Whoami(_) => true,

        // Auth subcommands: only whoami needs credentials
        Commands::Auth(args) => matches!(args.command, AuthCommand::Whoami),

        // Bypass: onboarding, help, and auth flow commands
        Commands::Doctor(_)
        | Commands::Tutorial(_)
        | Commands::Welcome(_)
        | Commands::Init(_)
        | Commands::New(_)
        | Commands::Wizard(_)
        | Commands::Hooks(_)
        | Commands::Update(_)
        | Commands::Validate(_)
        | Commands::Login(_)
        | Commands::Logout(_) => false,
    }
}

/// Evaluate a credential-load result and return the appropriate exit code.
///
/// Separated from I/O so tests can call it with synthetic inputs.
fn evaluate_auth(
    loaded: &anyhow::Result<Option<auth::credentials::Credentials>>,
) -> Result<(), u8> {
    match loaded {
        Ok(Some(creds)) if auth::credentials::is_expired(creds) => {
            eprintln!("Session expired. Run `anvil auth login` to re-authenticate.");
            Err(EXIT_AUTH_REQUIRED)
        }
        Ok(Some(_)) => Ok(()),
        Ok(None) | Err(_) => {
            eprintln!("Authentication required. Run `anvil auth login` to authenticate.");
            Err(EXIT_AUTH_REQUIRED)
        }
    }
}

/// Validate that usable credentials exist.
///
/// Returns `Ok(())` when valid credentials are found, `Ok(())` when
/// `ANVIL_DEV=1` is set (local dev bypass — never active in production
/// builds), or `Err(exit_code)` with `EXIT_AUTH_REQUIRED` otherwise.
fn check_auth() -> Result<(), u8> {
    // Local dev bypass: ANVIL_DEV=1 skips auth entirely.
    // Safe because:
    //   - All API calls still require a real token server-side.
    //   - This only bypasses the local credential pre-check.
    //   - Commands that call the API will fail with a 401 anyway.
    //   - Intended for CLI UX testing without a live token.
    if std::env::var("ANVIL_DEV").as_deref() == Ok("1") {
        return Ok(());
    }
    evaluate_auth(&auth::credentials::load())
}

/// Check whether `--json` appears in raw args before clap parses them.
/// This lets us emit JSON errors even when clap rejects the input.
fn wants_json() -> bool {
    std::env::args().any(|a| a == "--json")
}

fn main() -> ExitCode {
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

    if requires_auth(&cli.command)
        && let Err(code) = check_auth()
    {
        if cli.global.json {
            eprintln!(
                "{}",
                serde_json::json!({"error": "authentication_required"})
            );
        }
        return ExitCode::from(code);
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
        Commands::Check(args) => commands::check::run(args, &cli.global),
        Commands::Doctor(args) => commands::doctor::run(args, &cli.global),
        Commands::Drift(args) => commands::drift::run(args, &cli.global),
        Commands::Status(args) => commands::status::run(args, &cli.global),
        Commands::Tutorial(args) => commands::tutorial::run(args, &cli.global),
        Commands::Welcome(args) => commands::welcome::run(args, &cli.global),
        Commands::Init(args) => commands::init::run(args, &cli.global),
        Commands::New(args) => commands::new::run(args, &cli.global),
        Commands::Wizard(args) => commands::wizard::run(args, &cli.global),
        Commands::Admin(args) => commands::admin::run(args, &cli.global),
        Commands::Auth(args) => commands::auth::run(args, &cli.global),
        Commands::Update(_) | Commands::Gate(_) => unreachable!("handled above"),
        Commands::GateConfig(args) => commands::gate_config::run(args, &cli.global),
        Commands::Watch(args) => commands::watch::run(args, &cli.global),
        Commands::Export(args) => commands::export::run(args, &cli.global),
        Commands::Hooks(args) => commands::hooks::run(args, &cli.global),
        Commands::Architecture(args) => commands::architecture::run(args, &cli.global),
        Commands::Policy(args) => commands::policy::run(args, &cli.global),
        Commands::Validate(args) => commands::validate::run(args, &cli.global),
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
    fn requires_auth_admin() {
        assert!(requires_auth(&parse_command(&[
            "admin", "approve", "--batch", "1"
        ])));
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
    fn bypass_auth_welcome() {
        assert!(!requires_auth(&parse_command(&["welcome"])));
    }

    #[test]
    fn bypass_auth_init() {
        assert!(!requires_auth(&parse_command(&["init"])));
    }

    #[test]
    fn bypass_auth_new() {
        assert!(!requires_auth(&parse_command(&["new"])));
    }

    #[test]
    fn bypass_auth_wizard() {
        assert!(!requires_auth(&parse_command(&["wizard"])));
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
    fn bypass_auth_validate() {
        assert!(!requires_auth(&parse_command(&["validate", "plan.aps.md"])));
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

    // ── evaluate_auth ────────────────────────────────────────────

    use crate::auth::credentials::Credentials;

    fn valid_creds() -> Credentials {
        Credentials {
            license: "tok".into(),
            refresh_token: None,
            email: None,
            expires_at: Some("2099-01-01T00:00:00Z".into()),
        }
    }

    fn expired_creds() -> Credentials {
        Credentials {
            license: "tok".into(),
            refresh_token: None,
            email: None,
            expires_at: Some("2000-01-01T00:00:00Z".into()),
        }
    }

    fn no_expiry_creds() -> Credentials {
        Credentials {
            license: "tok".into(),
            refresh_token: None,
            email: None,
            expires_at: None,
        }
    }

    #[test]
    fn evaluate_auth_returns_err_when_no_credentials() {
        assert_eq!(evaluate_auth(&Ok(None)), Err(EXIT_AUTH_REQUIRED));
    }

    #[test]
    fn evaluate_auth_returns_err_when_expired() {
        assert_eq!(
            evaluate_auth(&Ok(Some(expired_creds()))),
            Err(EXIT_AUTH_REQUIRED),
        );
    }

    #[test]
    fn evaluate_auth_returns_err_on_load_error() {
        assert_eq!(
            evaluate_auth(&Err(anyhow::anyhow!("disk failure"))),
            Err(EXIT_AUTH_REQUIRED),
        );
    }

    #[test]
    fn evaluate_auth_returns_ok_when_valid() {
        assert!(evaluate_auth(&Ok(Some(valid_creds()))).is_ok());
    }

    #[test]
    fn evaluate_auth_returns_ok_when_no_expiry() {
        assert!(evaluate_auth(&Ok(Some(no_expiry_creds()))).is_ok());
    }

    #[test]
    fn check_auth_bypasses_when_anvil_dev_set() {
        // ANVIL_DEV=1 should allow unauthenticated access for local testing.
        // Without credentials, auth normally fails — but not in dev mode.
        temp_env::with_var("ANVIL_DEV", Some("1"), || {
            assert!(check_auth().is_ok(), "ANVIL_DEV=1 should bypass auth check");
        });
    }

    #[test]
    fn check_auth_does_not_bypass_without_anvil_dev() {
        // Env var absent — auth still required without credentials.
        temp_env::with_vars(
            [
                ("ANVIL_DEV", None),
                ("ANVIL_LICENSE", None),
                ("XDG_CONFIG_HOME", Some("/nonexistent/path")),
            ],
            || {
                assert_eq!(check_auth(), Err(EXIT_AUTH_REQUIRED));
            },
        );
    }
}
