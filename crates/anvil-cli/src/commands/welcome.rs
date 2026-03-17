use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct WelcomeArgs {}

pub fn run(_args: &WelcomeArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    bail!("not yet implemented")
}
