use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct ExportArgs {}

pub fn run(_args: &ExportArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    bail!("not yet implemented")
}
