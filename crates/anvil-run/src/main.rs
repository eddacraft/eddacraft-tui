//! `anvil-run` binary entry point.
//!
//! Thin: argv → `cli::parse_from` → `run::run` → `process::exit`.
//! All meaningful logic lives in the library half so tests can drive
//! the orchestrator without spawning a child process.

use std::process::ExitCode;

use anvil_run::cli;
use anvil_run::exit_codes::EXIT_USAGE;
use anvil_run::run;

fn main() -> ExitCode {
    let cli = match cli::parse_from(std::env::args_os()) {
        Ok(cli) => cli,
        Err(err) => {
            // `clap` writes its own help / usage text; we just
            // need to forward the exit code. A successful
            // `--help` / `--version` invocation uses
            // `ErrorKind::DisplayHelp` / `DisplayVersion` — those
            // map to exit code 0.
            let _ = err.print();
            return match err.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    ExitCode::from(0)
                }
                _ => ExitCode::from(EXIT_USAGE as u8),
            };
        }
    };
    let code = run::run(cli);
    // `ExitCode` only carries the low byte. Match the documented
    // contract on `forward_child_status`: forward the OS-reported
    // status modulo 256 instead of collapsing everything outside
    // 0..=255 to a generic `1`. A wrapped tool returning a value
    // outside the BSD byte range (e.g. a Windows HRESULT) still
    // gives the parent shell the same low byte the OS would have.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let truncated = (code & 0xff) as u8;
    ExitCode::from(truncated)
}
