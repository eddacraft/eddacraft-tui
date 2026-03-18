mod commands;
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
    /// Interactive guided tutorial.
    Tutorial(commands::tutorial::TutorialArgs),
    /// Show project status and health.
    Status(commands::status::StatusArgs),
    /// Run diagnostic checks on your environment.
    Doctor(commands::doctor::DoctorArgs),
    /// Show the welcome screen with quick-start options.
    Welcome(commands::welcome::WelcomeArgs),
    /// Run a full project audit.
    Audit(commands::audit::AuditArgs),
    /// Initialise Anvil configuration for a project.
    Init(commands::init::InitArgs),
    /// Guided project setup wizard.
    Wizard(commands::wizard::WizardArgs),
    /// Scaffold a new project from a template.
    New(commands::new::NewArgs),
    /// Run gate checks against the current project.
    Gate(commands::gate::GateArgs),
    /// Start file-watching mode with live gate checks.
    Watch(commands::watch::WatchArgs),
    /// Authenticate with the Anvil service.
    Auth(commands::auth::AuthArgs),
    /// Administrative commands (approvals, user management).
    Admin(commands::admin::AdminArgs),
    /// Manage and evaluate policies.
    Policy(commands::policy::PolicyArgs),
    /// Manage architecture boundary definitions.
    Architecture(commands::architecture::ArchitectureArgs),
    /// Install and manage git hooks.
    Hooks(commands::hooks::HooksArgs),
    /// Export constraints and configuration.
    Export(commands::export::ExportArgs),
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Tutorial(args) => commands::tutorial::run(args, &cli.global),
        Commands::Status(args) => commands::status::run(args, &cli.global),
        Commands::Doctor(args) => commands::doctor::run(args, &cli.global),
        Commands::Welcome(args) => commands::welcome::run(args, &cli.global),
        Commands::Audit(args) => commands::audit::run(args, &cli.global),
        Commands::Init(args) => commands::init::run(args, &cli.global),
        Commands::Wizard(args) => commands::wizard::run(args, &cli.global),
        Commands::New(args) => commands::new::run(args, &cli.global),
        Commands::Gate(args) => commands::gate::run(args, &cli.global),
        Commands::Watch(args) => commands::watch::run(args, &cli.global),
        Commands::Auth(args) => commands::auth::run(args, &cli.global),
        Commands::Admin(args) => commands::admin::run(args, &cli.global),
        Commands::Policy(args) => commands::policy::run(args, &cli.global),
        Commands::Architecture(args) => commands::architecture::run(args, &cli.global),
        Commands::Hooks(args) => commands::hooks::run(args, &cli.global),
        Commands::Export(args) => commands::export::run(args, &cli.global),
    };

    match result {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(err) => {
            if cli.global.json {
                eprintln!(
                    "{}",
                    serde_json::json!({ "error": format!("{err:#}") })
                );
            } else {
                eprintln!("Error: {err:#}");
            }
            ExitCode::from(EXIT_ERROR)
        }
    }
}
