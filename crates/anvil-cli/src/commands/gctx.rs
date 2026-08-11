//! `anvil gctx` — graph-context operator commands.
//!
//! Today this is the snippet-egress opt-in (GCTX-024). Identity-only graph
//! context is always available; **source-text** snippets ride only when the
//! operator consents (PV-9 CE-1 keeps the default identity-only). Consent is
//! recorded per-workspace as operator-owned state (`ANVIL_HOME` /
//! `XDG_STATE_HOME` / `~/.local/state/anvil`, never a worktree path) and
//! read by the daemon on the snippet path. The `ANVIL_GCTX_EGRESS` env var still
//! overrides this per process (`1` forces on, `0` is the kill-switch).

use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde_json::json;

use anvil_gctx_types::{EgressSource, GCTX_EGRESS_ENV, SnippetEgress, resolve_snippet_egress};
use anvil_intercept::egress_consent::{
    disable_snippet_consent, enable_snippet_consent, read_snippet_consent,
};

use crate::GlobalArgs;
use crate::util::workspace_root;

/// The CE-12 consequence statement shown before consent is recorded.
const CONSENT_STATEMENT: &str = "Enabling snippet egress means source text from matched symbols — \
secret-scanned and path-filtered — will be sent to the connected assistant / LLM provider whenever \
it requests context with `include_source`.";

#[derive(Debug, Args)]
pub struct GctxArgs {
    #[command(subcommand)]
    pub command: GctxCommand,
}

#[derive(Debug, Subcommand)]
pub enum GctxCommand {
    /// Manage source-text snippet egress to AI assistants.
    Egress(EgressArgs),
}

#[derive(Debug, Args)]
pub struct EgressArgs {
    #[command(subcommand)]
    pub action: EgressAction,
}

#[derive(Debug, Subcommand)]
pub enum EgressAction {
    /// Consent to sending source-text snippets for this workspace.
    Enable(EnableArgs),
    /// Revoke snippet-egress consent for this workspace (revert to identity-only).
    Disable,
    /// Show the effective snippet-egress state and where it comes from.
    Status,
}

#[derive(Debug, Args)]
pub struct EnableArgs {
    /// Acknowledge the egress consequence without an interactive prompt
    /// (required in non-interactive environments).
    #[arg(long, visible_alias = "consent")]
    pub yes: bool,
}

pub fn run(args: &GctxArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        GctxCommand::Egress(egress) => {
            // Use the canonical workspace root (git top-level) so consent is
            // written where the daemon reads it — never the raw CWD, which would
            // silently mis-place the record when run from a subdirectory.
            let root = workspace_root().context("resolve workspace root")?;
            match &egress.action {
                EgressAction::Status => status(&root, global),
                EgressAction::Enable(enable) => enable_egress(&root, enable.yes),
                EgressAction::Disable => disable_egress(&root),
            }
        }
    }
}

/// Report the effective snippet-egress state and its source, so a persisted
/// opt-in is never invisible. Emits JSON under the global `--json` flag for
/// scripting (e.g. multi-workspace consent audits).
fn status(root: &Path, global: &GlobalArgs) -> Result<()> {
    let env_raw = std::env::var(GCTX_EGRESS_ENV).ok();
    let (env_decision, env_source) = resolve_snippet_egress(env_raw.as_deref(), None);
    let (decision, source) = if matches!(env_source, EgressSource::Env) {
        (env_decision, env_source)
    } else {
        let persisted =
            read_snippet_consent(root).context("read persisted snippet-egress consent")?;
        resolve_snippet_egress(env_raw.as_deref(), persisted)
    };

    let enabled = matches!(decision, SnippetEgress::Enabled);
    let source_key = match source {
        EgressSource::Env => "env",
        EgressSource::Config => "config",
        EgressSource::Default => "default",
    };

    if global.json {
        let doc = json!({
            "egress": if enabled { "enabled" } else { "identity-only" },
            "source": source_key,
        });
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }

    let state = if enabled {
        "enabled (source-text snippets)"
    } else {
        "identity-only"
    };
    let source_label = match source {
        EgressSource::Env => "environment (ANVIL_GCTX_EGRESS)",
        EgressSource::Config => "workspace consent (operator state)",
        EgressSource::Default => "default (no opt-in)",
    };
    println!("Snippet egress: {state}");
    println!("Source:         {source_label}");

    if !enabled {
        println!();
        if matches!(source, EgressSource::Env) {
            // The kill-switch (ANVIL_GCTX_EGRESS=0) is suppressing egress —
            // enabling consent alone would not take effect.
            println!(
                "Snippet egress is held off by ANVIL_GCTX_EGRESS=0 (kill-switch). Unset that \
                 variable, then run `anvil gctx egress enable` to opt in."
            );
        } else {
            println!(
                "Identity-only graph context is always available. To also send source-text \
                 snippets, run:"
            );
            println!("  anvil gctx egress enable");
        }
    }
    Ok(())
}

/// Record the operator's consent to snippet egress for this workspace, behind the
/// CE-12 consent gate. Never auto-enables: a non-interactive run without `--yes`
/// fails closed.
fn enable_egress(root: &Path, yes: bool) -> Result<()> {
    crate::install_root::ensure_project_write_allowed("gctx egress enable")?;

    if !yes {
        if crate::is_non_interactive_env() || !std::io::stdin().is_terminal() {
            bail!(
                "snippet egress requires explicit consent; re-run with `--yes` to acknowledge in \
                 a non-interactive environment.\n\n{CONSENT_STATEMENT}"
            );
        }
        eprintln!("{CONSENT_STATEMENT}\n");
        if !confirm("Enable snippet egress for this workspace?")? {
            println!("Snippet egress unchanged (still identity-only).");
            return Ok(());
        }
    }

    enable_snippet_consent(root).context("persist snippet-egress consent")?;
    println!("Snippet egress enabled for this workspace.");
    println!(
        "Revoke any time with `anvil gctx egress disable`. `ANVIL_GCTX_EGRESS` still overrides \
         this per process (1 = on, 0 = kill-switch)."
    );
    Ok(())
}

/// Revoke snippet-egress consent — a clean revert to the CE-1 identity-only
/// default. Idempotent.
fn disable_egress(root: &Path) -> Result<()> {
    crate::install_root::ensure_project_write_allowed("gctx egress disable")?;
    disable_snippet_consent(root).context("revoke snippet-egress consent")?;
    println!("Snippet egress disabled for this workspace (identity-only).");
    Ok(())
}

/// Fail-closed yes/no prompt: prints to stderr, reads a line from stdin, and
/// treats EOF / anything but an explicit yes as "no" (so a closed stdin never
/// fail-opens into consent).
fn confirm(question: &str) -> Result<bool> {
    eprint!("{question} [y/N] ");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    let read = std::io::stdin()
        .lock()
        .read_line(&mut line)
        .context("read consent response")?;
    if read == 0 {
        return Ok(false);
    }
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
