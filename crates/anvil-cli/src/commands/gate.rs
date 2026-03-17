use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct GateArgs {}

pub fn run(_args: &GateArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    bail!("not yet implemented")
}
