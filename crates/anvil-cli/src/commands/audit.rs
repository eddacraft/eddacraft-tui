use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct AuditArgs {}

pub fn run(_args: &AuditArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    bail!("not yet implemented")
}
