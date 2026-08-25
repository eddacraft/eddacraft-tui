//! `anvil policy members` — list and toggle overlay-selected pack members.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use anvil_policy_engine::pack::{
    is_safe_pack_id, load_manifest, load_overlay, overlay_path, save_overlay,
};

use crate::GlobalArgs;
use crate::output;

use super::install::resolve_workspace;

#[derive(Debug, Args)]
pub struct MembersArgs {
    /// Installed pack id (directory name under `.anvil/policies/`).
    pack_id: String,
    /// Disable these members in the overlay.
    #[arg(long = "off", value_name = "MEMBER", action = clap::ArgAction::Append)]
    off: Vec<String>,
    /// Enable these members in the overlay.
    #[arg(long = "on", value_name = "MEMBER", action = clap::ArgAction::Append)]
    on: Vec<String>,
    /// Workspace root (defaults to the current workspace).
    #[arg(long)]
    workspace: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct MemberView {
    id: String,
    title: String,
    enabled: bool,
}

pub fn run(args: &MembersArgs, global: &GlobalArgs) -> Result<()> {
    if !is_safe_pack_id(&args.pack_id) {
        bail!(
            "pack id `{}` is not a safe directory name; use a single path component with no `/` or `..`",
            args.pack_id
        );
    }
    let workspace = resolve_workspace(args.workspace.as_deref())?;
    let pack_dir = workspace.join(".anvil/policies").join(&args.pack_id);
    let manifest_path = pack_dir.join("pack.yaml");
    if !manifest_path.is_file() {
        bail!(
            "pack `{}` is not installed under {}; run `anvil policy install {}` first",
            args.pack_id,
            pack_dir.display(),
            args.pack_id
        );
    }
    let manifest = load_manifest(&manifest_path)
        .with_context(|| format!("loading {}", manifest_path.display()))?;
    let known: Vec<String> = manifest
        .policies
        .iter()
        .map(|entry| entry.metadata.id.clone())
        .collect();
    for id in args.off.iter().chain(args.on.iter()) {
        if !known.iter().any(|known_id| known_id == id) {
            bail!(
                "unknown member `{id}` in pack `{}`; known members: {}",
                args.pack_id,
                known.join(", ")
            );
        }
    }

    let policies_dir = workspace.join(".anvil/policies");
    let mut overlay = load_overlay(&policies_dir, &args.pack_id)
        .with_context(|| format!("loading overlay for pack `{}`", args.pack_id))?;

    if !args.off.is_empty() || !args.on.is_empty() {
        crate::install_root::ensure_project_write_allowed("policy members")?;
        for id in &args.off {
            overlay.disable(id);
        }
        for id in &args.on {
            overlay.enable(id);
        }
        save_overlay(&policies_dir, &args.pack_id, &overlay)
            .with_context(|| format!("writing overlay for pack `{}`", args.pack_id))?;
    }

    let views: Vec<MemberView> = manifest
        .policies
        .iter()
        .map(|entry| MemberView {
            id: entry.metadata.id.clone(),
            title: entry.metadata.title.clone(),
            enabled: overlay.is_enabled(&entry.metadata.id),
        })
        .collect();

    if global.json {
        output::json::print(&views)?;
    } else {
        output::plain::blank();
        output::plain::section(&format!("Pack `{}` members", args.pack_id));
        for view in &views {
            let mark = if view.enabled { "on " } else { "off" };
            println!(
                "  {mark}  {id:<24} {title}",
                id = view.id,
                title = view.title
            );
        }
        output::plain::blank();
        println!(
            "Overlay: {}",
            overlay_path(&policies_dir, &args.pack_id)?.display()
        );
    }
    Ok(())
}
