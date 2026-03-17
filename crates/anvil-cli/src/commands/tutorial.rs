use std::io::IsTerminal;

use anvil_tui::surfaces::tutorial::TutorialState;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct TutorialArgs {}

pub fn run(_args: &TutorialArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    if global.no_tui || !std::io::stdout().is_terminal() {
        println!("Tutorial requires an interactive terminal. Run without --no-tui in a TTY.");
        return Ok(());
    }

    let state = TutorialState::new();
    crate::tui::run_surface(state)
}
