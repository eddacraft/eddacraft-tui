#[cfg(feature = "runner")]
fn main() -> std::process::ExitCode {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::process::ExitCode;

    use eddacraft_tui::runner::{
        CommandSet, ConfigSource, RunnerError, RunnerOptions, TerminalCli,
    };

    struct DemoCli;

    #[allow(clippy::unnecessary_literal_bound)]
    impl TerminalCli for DemoCli {
        type Command = String;
        type Config = Option<PathBuf>;

        fn name(&self) -> &str {
            "runner-shell"
        }

        fn version(&self) -> &str {
            env!("CARGO_PKG_VERSION")
        }

        fn commands(&self) -> CommandSet {
            CommandSet::new().command("doctor", "Show the selected config path")
        }

        fn parse_command(
            &self,
            command: &str,
            _args: &[OsString],
        ) -> Result<Self::Command, RunnerError> {
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
            _options: &RunnerOptions,
        ) -> Result<ExitCode, RunnerError> {
            println!(
                "command={command} config={}",
                config
                    .as_ref()
                    .map_or("<none>".into(), |path| path.display().to_string())
            );
            Ok(ExitCode::SUCCESS)
        }
    }

    eddacraft_tui::runner::launch_cli(DemoCli)
}

#[cfg(not(feature = "runner"))]
fn main() {
    eprintln!("run with `--features runner`");
}
