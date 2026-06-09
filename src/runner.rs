//! Small fallback CLI shell for consumers without their own argument parser.
//!
//! The runner is a convenience layer, not a CLI framework. It owns the
//! undifferentiated plumbing a small terminal tool would otherwise rewrite:
//! shared global flags, first-level subcommand selection, and a config-path
//! handoff. Domain command semantics, parsing of command-specific arguments,
//! and the render loop stay with the consumer.
//!
//! # What the runner parses
//!
//! Global envelope only — handled by a zero-dependency parser ([`lexopt`]):
//!
//! - `--help` / `-h`, `--version` / `-V`
//! - `--theme <name>`, `--no-tui`, `--config <path>`
//! - one first-level command name; everything after it is handed to the
//!   consumer's [`TerminalCli::parse_command`] verbatim
//!
//! # When you've outgrown it
//!
//! The runner deliberately does **not** do nested command trees, typed
//! argument validation, `--help` generated from your command internals,
//! environment-variable binding, or shell completions. A serious CLI will
//! want those — that is the expected outcome, not a gap.
//!
//! When you reach that point, **bring your own parser** ([`clap`], `argh`,
//! hand-rolled — your choice) and hand the runner pre-parsed options via
//! [`launch_with`]. You keep full control of your CLI surface and still get
//! the runner's lifecycle/theme integration; the runner never owns or
//! re-exports your parser, so your `clap` major version is yours alone.
//!
//! ```ignore
//! // Cargo.toml — your crate brings the parser it needs:
//! //   clap = { version = "4", features = ["derive"] }
//! //   eddacraft-tui = { version = "0.3", features = ["runner"] }
//!
//! use clap::Parser;
//! use eddacraft_tui::runner::{launch_with, RunnerMode, RunnerOptions};
//!
//! #[derive(Parser)]
//! #[command(name = "mytool", version)]
//! struct Cli {
//!     command: String,
//!     #[arg(long)]
//!     theme: Option<String>,
//!     #[arg(long = "no-tui")]
//!     no_tui: bool,
//!     #[arg(long)]
//!     config: Option<std::path::PathBuf>,
//! }
//!
//! fn main() -> std::process::ExitCode {
//!     let cli = Cli::parse(); // clap owns help, validation, completions, …
//!     let options = RunnerOptions {
//!         command: Some(cli.command),
//!         config_path: cli.config,
//!         theme: cli.theme,
//!         mode: if cli.no_tui { RunnerMode::Plain } else { RunnerMode::Tui },
//!         ..RunnerOptions::default()
//!     };
//!     launch_with(MyApp::new(), options)
//! }
//! ```
//!
//! [`lexopt`]: https://docs.rs/lexopt
//! [`clap`]: https://docs.rs/clap

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Shared config source selected by the fallback CLI shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource<'a> {
    None,
    Path(&'a Path),
}

/// Whether the selected command should use an interactive TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunnerMode {
    #[default]
    Tui,
    Plain,
}

/// Parsed global options and first-level command envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerOptions {
    pub command: Option<String>,
    pub command_args: Vec<OsString>,
    pub config_path: Option<PathBuf>,
    pub theme: Option<String>,
    pub mode: RunnerMode,
}

impl Default for RunnerOptions {
    fn default() -> Self {
        Self {
            command: None,
            command_args: Vec::new(),
            config_path: None,
            theme: None,
            mode: RunnerMode::Tui,
        }
    }
}

/// A first-level command exposed by a consumer CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: String,
    pub about: String,
}

/// Consumer-declared first-level commands for help and dispatch validation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandSet {
    commands: Vec<CommandSpec>,
}

impl CommandSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn command(mut self, name: impl Into<String>, about: impl Into<String>) -> Self {
        self.commands.push(CommandSpec {
            name: name.into(),
            about: about.into(),
        });
        self
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.commands.iter().any(|command| command.name == name)
    }

    #[must_use]
    pub fn first(&self) -> Option<&CommandSpec> {
        self.commands.first()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CommandSpec> {
        self.commands.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerError {
    MissingValue(&'static str),
    UnknownGlobal(String),
    UnknownCommand(String),
    NoCommand,
    InvalidUnicode(String),
    Consumer(String),
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue(flag) => write!(f, "missing value for {flag}"),
            Self::UnknownGlobal(flag) => write!(f, "unknown global flag {flag}"),
            Self::UnknownCommand(command) => write!(f, "unknown command {command}"),
            Self::NoCommand => f.write_str("no command selected"),
            Self::InvalidUnicode(value) => write!(f, "invalid unicode in {value}"),
            Self::Consumer(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RunnerError {}

/// Consumer-supplied CLI semantics for the fallback runner shell.
pub trait TerminalCli {
    type Command;
    type Config;

    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn commands(&self) -> CommandSet;
    fn parse_command(&self, command: &str, args: &[OsString])
    -> Result<Self::Command, RunnerError>;
    fn load_config(&self, source: ConfigSource<'_>) -> Result<Self::Config, RunnerError>;
    fn run_command(
        &mut self,
        command: Self::Command,
        config: Self::Config,
        options: &RunnerOptions,
    ) -> Result<ExitCode, RunnerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RunnerAction {
    Help,
    Version,
    Run(RunnerOptions),
}

/// Run a consumer CLI from the current process arguments using the built-in
/// fallback parser.
///
/// This is the ~3-line entry point for a small tool: the runner parses the
/// [global envelope](self#what-the-runner-parses) and selects a first-level
/// command for you. Need nested commands, argument validation, generated
/// `--help`, env binding, or completions? You've outgrown the fallback —
/// parse with your own parser (e.g. [`clap`](https://docs.rs/clap)) and call
/// [`launch_with`] instead. See the [module docs](self) for the pattern.
pub fn launch_cli<C: TerminalCli>(cli: C) -> ExitCode {
    launch_with_args(cli, std::env::args_os().skip(1))
}

/// Run a consumer CLI with pre-parsed runner options — the bring-your-own-parser
/// entry point.
///
/// Use this when you parse arguments yourself (with [`clap`](https://docs.rs/clap),
/// `argh`, or by hand) and only want the runner's command dispatch, config
/// handoff, and lifecycle/theme integration. The runner does not own or
/// re-export your parser, so its public API stays independent of your parser's
/// version. For the trivial case where the built-in parser suffices, use
/// [`launch_cli`].
#[allow(clippy::needless_pass_by_value)]
pub fn launch_with<C: TerminalCli>(mut cli: C, options: RunnerOptions) -> ExitCode {
    match execute(&mut cli, &options) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Run a consumer CLI from an explicit argument iterator.
///
/// This is public so consumers can test their own runner integration without
/// mutating process-global `argv`.
pub fn launch_with_args<C, I>(cli: C, args: I) -> ExitCode
where
    C: TerminalCli,
    I: IntoIterator<Item = OsString>,
{
    match parse_args(args) {
        Ok(RunnerAction::Help) => {
            print_help(&cli);
            ExitCode::SUCCESS
        }
        Ok(RunnerAction::Version) => {
            println!("{} {}", cli.name(), cli.version());
            ExitCode::SUCCESS
        }
        Ok(RunnerAction::Run(options)) => launch_with(cli, options),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn execute<C: TerminalCli>(cli: &mut C, options: &RunnerOptions) -> Result<ExitCode, RunnerError> {
    let command = selected_command(cli, options)?;
    let parsed_command = cli.parse_command(&command, &options.command_args)?;
    let config_source = match options.config_path.as_deref() {
        Some(path) => ConfigSource::Path(path),
        None => ConfigSource::None,
    };
    let config = cli.load_config(config_source)?;
    cli.run_command(parsed_command, config, options)
}

fn selected_command<C: TerminalCli>(
    cli: &C,
    options: &RunnerOptions,
) -> Result<String, RunnerError> {
    let commands = cli.commands();
    let command = match &options.command {
        Some(command) => command.clone(),
        None => commands
            .first()
            .map(|command| command.name.clone())
            .ok_or(RunnerError::NoCommand)?,
    };

    if commands.contains(&command) {
        Ok(command)
    } else {
        Err(RunnerError::UnknownCommand(command))
    }
}

fn parse_args<I>(args: I) -> Result<RunnerAction, RunnerError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut options = RunnerOptions::default();
    let mut parser = lexopt::Parser::from_args(args);

    while let Some(arg) = parser.next().map_err(map_lexopt_error)? {
        match arg {
            lexopt::Arg::Short('h') | lexopt::Arg::Long("help") => return Ok(RunnerAction::Help),
            lexopt::Arg::Short('V') | lexopt::Arg::Long("version") => {
                return Ok(RunnerAction::Version);
            }
            lexopt::Arg::Long("no-tui") => options.mode = RunnerMode::Plain,
            lexopt::Arg::Long("theme") => {
                options.theme = Some(os_to_string(
                    parser.value().map_err(map_lexopt_error)?,
                    "--theme",
                )?);
            }
            lexopt::Arg::Long("config") => {
                options.config_path =
                    Some(PathBuf::from(parser.value().map_err(map_lexopt_error)?));
            }
            lexopt::Arg::Short(flag) => return Err(RunnerError::UnknownGlobal(format!("-{flag}"))),
            lexopt::Arg::Long(flag) => return Err(RunnerError::UnknownGlobal(format!("--{flag}"))),
            lexopt::Arg::Value(command) => {
                options.command = Some(os_to_string(command, "command")?);
                options.command_args = parser
                    .raw_args()
                    .map_err(map_lexopt_error)?
                    .collect::<Vec<_>>();
                return Ok(RunnerAction::Run(options));
            }
        }
    }

    Ok(RunnerAction::Run(options))
}

fn map_lexopt_error(error: lexopt::Error) -> RunnerError {
    match error {
        lexopt::Error::MissingValue { option } => {
            RunnerError::MissingValue(match option.as_deref() {
                Some("--theme") => "--theme",
                Some("--config") => "--config",
                _ => "option",
            })
        }
        other => RunnerError::Consumer(other.to_string()),
    }
}

fn os_to_string(value: OsString, label: &str) -> Result<String, RunnerError> {
    value
        .into_string()
        .map_err(|_| RunnerError::InvalidUnicode(label.to_string()))
}

fn print_help<C: TerminalCli>(cli: &C) {
    println!("{} {}", cli.name(), cli.version());
    println!("\nGlobal flags:");
    println!("  --help");
    println!("  --version");
    println!("  --theme <name>");
    println!("  --no-tui");
    println!("  --config <path>");

    let commands = cli.commands();
    if commands.iter().next().is_some() {
        println!("\nCommands:");
        for command in commands.iter() {
            println!("  {:<16} {}", command.name, command.about);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubCli {
        config: Option<PathBuf>,
        args: Vec<OsString>,
        mode: RunnerMode,
        theme: Option<String>,
    }

    impl Default for StubCli {
        fn default() -> Self {
            Self {
                config: None,
                args: Vec::new(),
                mode: RunnerMode::Tui,
                theme: None,
            }
        }
    }

    #[allow(clippy::unnecessary_literal_bound)]
    impl TerminalCli for StubCli {
        type Command = String;
        type Config = Option<PathBuf>;

        fn name(&self) -> &str {
            "stub"
        }

        fn version(&self) -> &str {
            "1.2.3"
        }

        fn commands(&self) -> CommandSet {
            CommandSet::new()
                .command("run", "Run the tool")
                .command("doctor", "Check setup")
        }

        fn parse_command(
            &self,
            command: &str,
            args: &[OsString],
        ) -> Result<Self::Command, RunnerError> {
            assert_eq!(args, self.args.as_slice());
            Ok(command.to_string())
        }

        fn load_config(&self, source: ConfigSource<'_>) -> Result<Self::Config, RunnerError> {
            Ok(match source {
                ConfigSource::None => None,
                ConfigSource::Path(path) => Some(path.to_path_buf()),
            })
        }

        fn run_command(
            &mut self,
            command: Self::Command,
            config: Self::Config,
            options: &RunnerOptions,
        ) -> Result<ExitCode, RunnerError> {
            assert_eq!(command, "doctor");
            assert_eq!(config.as_deref(), Some(Path::new("config.toml")));
            assert_eq!(options.mode, RunnerMode::Plain);
            assert_eq!(options.theme.as_deref(), Some("ember"));
            self.config = config;
            self.mode = options.mode;
            self.theme = options.theme.clone();
            Ok(ExitCode::SUCCESS)
        }
    }

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn parse_shared_globals_and_passes_command_args_through() {
        let action = parse_args(os_args(&[
            "--theme",
            "ember",
            "--config",
            "config.toml",
            "--no-tui",
            "doctor",
            "--domain-flag",
            "value",
        ]))
        .unwrap();

        let RunnerAction::Run(options) = action else {
            panic!("expected run action");
        };
        assert_eq!(options.command.as_deref(), Some("doctor"));
        assert_eq!(options.theme.as_deref(), Some("ember"));
        assert_eq!(
            options.config_path.as_deref(),
            Some(Path::new("config.toml"))
        );
        assert_eq!(options.mode, RunnerMode::Plain);
        assert_eq!(options.command_args, os_args(&["--domain-flag", "value"]));
    }

    #[test]
    fn launch_with_hands_config_mode_and_theme_to_consumer() {
        let cli = StubCli {
            args: os_args(&["--domain-flag", "value"]),
            ..StubCli::default()
        };
        let code = launch_with_args(
            cli,
            os_args(&[
                "--theme",
                "ember",
                "--config=config.toml",
                "--no-tui",
                "doctor",
                "--domain-flag",
                "value",
            ]),
        );

        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn unknown_global_before_command_is_an_error() {
        let error = parse_args(os_args(&["--verbose", "doctor"])).unwrap_err();
        assert_eq!(error, RunnerError::UnknownGlobal("--verbose".to_string()));
    }

    #[test]
    fn unknown_command_is_an_error() {
        let code = launch_with_args(StubCli::default(), os_args(&["missing"]));
        assert_eq!(code, ExitCode::FAILURE);
    }
}
