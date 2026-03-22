mod auth;
mod commands;
mod output;
mod tui;

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
    /// Run diagnostic checks on your environment.
    Doctor(commands::doctor::DoctorArgs),
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
    /// Run gate checks against the current project.
    Gate(commands::gate::GateArgs),
    /// Start file-watching mode with live gate checks.
    Watch(commands::watch::WatchArgs),
    /// Export constraints and configuration.
    Export(commands::export::ExportArgs),
    /// Install and manage git hooks.
    Hooks(commands::hooks::HooksArgs),
    // Not yet implemented — uncomment as each command ships:
    // /// Administrative commands (approvals, user management).
    // Admin(commands::admin::AdminArgs),
    // /// Manage architecture boundary definitions.
    // Architecture(commands::architecture::ArchitectureArgs),
    // /// Authenticate with the Anvil service.
    // Auth(commands::auth::AuthArgs),
    // /// Manage and evaluate policies.
    // Policy(commands::policy::PolicyArgs),
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

    let result = match &cli.command {
        Commands::Audit(args) => commands::audit::run(args, &cli.global),
        Commands::Doctor(args) => commands::doctor::run(args, &cli.global),
        Commands::Status(args) => commands::status::run(args, &cli.global),
        Commands::Tutorial(args) => commands::tutorial::run(args, &cli.global),
        Commands::Welcome(args) => commands::welcome::run(args, &cli.global),
        Commands::Init(args) => commands::init::run(args, &cli.global),
        Commands::New(args) => commands::new::run(args, &cli.global),
        Commands::Wizard(args) => commands::wizard::run(args, &cli.global),
        Commands::Gate(args) => commands::gate::run(args, &cli.global),
        Commands::Watch(args) => commands::watch::run(args, &cli.global),
        Commands::Export(args) => commands::export::run(args, &cli.global),
        Commands::Hooks(args) => commands::hooks::run(args, &cli.global),
    };

    match result {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(err) => {
            if cli.global.json {
                eprintln!("{}", serde_json::json!({ "error": format!("{err:#}") }));
            } else {
                eprintln!("Error: {err:#}");
            }
            ExitCode::from(EXIT_ERROR)
        }
    }
}
