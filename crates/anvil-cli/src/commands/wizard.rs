use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct WizardArgs {}

pub fn run(_args: &WizardArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    bail!("not yet implemented")
}
