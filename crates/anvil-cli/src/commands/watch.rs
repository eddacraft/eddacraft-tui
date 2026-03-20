use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct WatchArgs {}

#[allow(dead_code)]
pub fn run(_args: &WatchArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    bail!("not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn default_global() -> GlobalArgs {
        GlobalArgs {
            json: false,
            no_tui: true,
            verbose: false,
        }
    }

    #[test]
    fn watch_args_parses_empty() {
        #[derive(Parser)]
        struct Wrapper {
            #[command(flatten)]
            watch: WatchArgs,
        }
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.watch);
    }

    #[test]
    fn run_returns_not_implemented() {
        let args = WatchArgs {};
        let global = default_global();
        let result = run(&args, &global);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not yet implemented"), "got: {err}");
    }

    #[test]
    fn run_errors_regardless_of_json_flag() {
        let args = WatchArgs {};
        let global = GlobalArgs {
            json: true,
            no_tui: true,
            verbose: false,
        };
        let result = run(&args, &global);
        assert!(result.is_err());
    }

    #[test]
    fn run_errors_regardless_of_verbose_flag() {
        let args = WatchArgs {};
        let global = GlobalArgs {
            json: false,
            no_tui: false,
            verbose: true,
        };
        let result = run(&args, &global);
        assert!(result.is_err());
    }
}
