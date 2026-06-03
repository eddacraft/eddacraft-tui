//! `anvil workspace` — DSV-008 (ADR-061 §7): manage the operator-level
//! workspace **confinement** config the intercept daemon reads.
//!
//! The daemon's trust floor is `SO_PEERCRED` same-uid. Operators who want a
//! tighter boundary switch admission to `allowlist` and list the roots the
//! daemon may serve. This command is a thin caller of the read/modify/write
//! helpers in [`anvil_intercept::confinement`] — the config shape, owner-only
//! posture, and `ANVIL_HOME`/XDG path resolution all live daemon-side so the
//! CLI and daemon cannot drift (the dependency runs `anvil-cli` →
//! `anvil-intercept`, never the reverse).
//!
//! Subcommands:
//!
//! - `mode <open|allowlist>` — set the admission mode.
//! - `allow <PATH> [--prefix]` — add an allow entry (exact, or a `--prefix`
//!   subtree). Only meaningful in `allowlist` mode.
//! - `deny <PATH>` — remove an allow entry.
//! - `list` — show the current mode and allow entries.
//!
//! Changes take effect for **new** daemon connections (the daemon reads the
//! confinement config per connection); no restart is required.

use anvil_intercept::confinement::{self, AdmissionModeFile, MatchKind};
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    command: WorkspaceCommand,
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    /// Set the daemon admission mode: `open` (first-touch adopt, the default)
    /// or `allowlist` (only allow-listed roots, plus the primary check-in
    /// root, are served).
    Mode(ModeArgs),
    /// Add an allow entry. Exact by default; `--prefix` confines a whole
    /// subtree. Only consulted in `allowlist` mode.
    Allow(AllowArgs),
    /// Remove an allow entry by path.
    Deny(DenyArgs),
    /// Show the current admission mode and allow entries.
    List,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModeValue {
    Open,
    Allowlist,
}

impl From<ModeValue> for AdmissionModeFile {
    fn from(value: ModeValue) -> Self {
        match value {
            ModeValue::Open => AdmissionModeFile::Open,
            ModeValue::Allowlist => AdmissionModeFile::Allowlist,
        }
    }
}

#[derive(Debug, Args)]
struct ModeArgs {
    /// The admission mode to set.
    #[arg(value_enum)]
    mode: ModeValue,
}

#[derive(Debug, Args)]
struct AllowArgs {
    /// The workspace root to admit.
    #[arg(value_name = "PATH")]
    path: std::path::PathBuf,
    /// Admit the entire subtree beneath PATH, not just PATH exactly.
    #[arg(long)]
    prefix: bool,
}

#[derive(Debug, Args)]
struct DenyArgs {
    /// The allow entry to remove (matched as stored).
    #[arg(value_name = "PATH")]
    path: std::path::PathBuf,
}

pub fn run(args: &WorkspaceArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        WorkspaceCommand::Mode(mode_args) => run_mode(mode_args),
        WorkspaceCommand::Allow(allow_args) => run_allow(allow_args),
        WorkspaceCommand::Deny(deny_args) => run_deny(deny_args),
        WorkspaceCommand::List => run_list(),
    }
}

/// Absolutise a CLI path so the stored entry is stable regardless of the
/// invoking cwd, without requiring it to exist yet (a `--prefix` root may be
/// created later). Canonicalisation to a real path happens daemon-side.
///
/// The only failure is an unavailable working directory; propagate it rather
/// than silently storing a relative path the daemon would later drop.
fn absolutise(path: &std::path::Path) -> Result<std::path::PathBuf> {
    std::path::absolute(path).with_context(|| {
        format!(
            "could not absolutise {} (is the working directory available?)",
            path.display()
        )
    })
}

fn run_mode(args: &ModeArgs) -> Result<()> {
    let mut file = confinement::read_config_file().context("read workspace confinement config")?;
    file.admission = args.mode.into();
    let written = confinement::write_config_file(&file).context("write confinement config")?;
    println!(
        "Admission mode set to {} ({}).",
        mode_label(file.admission),
        written.display()
    );
    if file.admission == AdmissionModeFile::Allowlist {
        match file.allow.len() {
            0 => println!(
                "Allowlist is empty — only each connection's primary check-in root \
                 will be served."
            ),
            n => println!(
                "{n} allow {} in effect (plus each connection's primary root).",
                if n == 1 { "entry" } else { "entries" }
            ),
        }
    }
    print_takes_effect_note();
    Ok(())
}

fn run_allow(args: &AllowArgs) -> Result<()> {
    let path = absolutise(&args.path)?;
    let kind = if args.prefix {
        MatchKind::Prefix
    } else {
        MatchKind::Exact
    };
    let mut file = confinement::read_config_file().context("read workspace confinement config")?;
    file.upsert_allow(path.clone(), kind);
    confinement::write_config_file(&file).context("write confinement config")?;
    println!("Allowed {} ({}).", path.display(), kind_label(kind));
    if file.admission == AdmissionModeFile::Open {
        println!(
            "Note: admission mode is `open`, so allow entries are not yet enforced. \
             Run `anvil workspace mode allowlist` to confine the daemon."
        );
    }
    print_takes_effect_note();
    Ok(())
}

fn run_deny(args: &DenyArgs) -> Result<()> {
    let path = absolutise(&args.path)?;
    let mut file = confinement::read_config_file().context("read workspace confinement config")?;
    if file.remove_allow(&path) {
        confinement::write_config_file(&file).context("write confinement config")?;
        println!("Removed allow entry {}.", path.display());
        print_takes_effect_note();
    } else {
        println!(
            "No allow entry matched {} — nothing to remove.",
            path.display()
        );
    }
    Ok(())
}

fn run_list() -> Result<()> {
    let file = confinement::read_config_file().context("read workspace confinement config")?;
    println!("Admission mode: {}", mode_label(file.admission));
    if file.allow.is_empty() {
        println!("Allow entries: (none)");
        if file.admission == AdmissionModeFile::Allowlist {
            println!("  Only the primary check-in root of each connection is admitted.");
        }
    } else {
        println!("Allow entries:");
        for entry in &file.allow {
            println!("  {} ({})", entry.path.display(), kind_label(entry.kind));
        }
    }
    Ok(())
}

fn mode_label(mode: AdmissionModeFile) -> &'static str {
    match mode {
        AdmissionModeFile::Open => "open",
        AdmissionModeFile::Allowlist => "allowlist",
    }
}

fn kind_label(kind: MatchKind) -> &'static str {
    match kind {
        MatchKind::Exact => "exact",
        MatchKind::Prefix => "prefix",
    }
}

fn print_takes_effect_note() {
    println!("Takes effect for new daemon connections; no restart required.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Minimal parser wrapper so the `workspace` subcommand surface can be
    /// exercised in isolation without standing up the whole CLI.
    #[derive(Debug, Parser)]
    struct Harness {
        #[command(subcommand)]
        command: WorkspaceCommand,
    }

    #[test]
    fn workspace_allow_prefix_parses() {
        let parsed = Harness::try_parse_from(["anvil-workspace", "allow", "/srv/proj", "--prefix"])
            .expect("parse allow --prefix");
        match parsed.command {
            WorkspaceCommand::Allow(args) => {
                assert_eq!(args.path, std::path::Path::new("/srv/proj"));
                assert!(args.prefix, "--prefix sets the subtree flag");
            }
            other => panic!("expected Allow, got {other:?}"),
        }
    }

    #[test]
    fn workspace_allow_defaults_to_exact() {
        let parsed = Harness::try_parse_from(["anvil-workspace", "allow", "/srv/proj"])
            .expect("parse allow");
        match parsed.command {
            WorkspaceCommand::Allow(args) => assert!(!args.prefix, "exact is the default"),
            other => panic!("expected Allow, got {other:?}"),
        }
    }

    #[test]
    fn workspace_mode_accepts_allowlist_and_rejects_garbage() {
        let parsed = Harness::try_parse_from(["anvil-workspace", "mode", "allowlist"])
            .expect("parse mode allowlist");
        match parsed.command {
            WorkspaceCommand::Mode(args) => {
                assert_eq!(
                    AdmissionModeFile::from(args.mode),
                    AdmissionModeFile::Allowlist
                );
            }
            other => panic!("expected Mode, got {other:?}"),
        }
        assert!(
            Harness::try_parse_from(["anvil-workspace", "mode", "loose"]).is_err(),
            "an unknown mode value is rejected by clap"
        );
    }

    #[test]
    fn absolutise_makes_relative_paths_absolute() {
        let abs = absolutise(std::path::Path::new("relative/dir")).expect("absolutise");
        assert!(abs.is_absolute(), "a relative path is absolutised: {abs:?}");
        // An already-absolute path is returned unchanged.
        assert_eq!(
            absolutise(std::path::Path::new("/srv/proj")).expect("absolutise"),
            std::path::PathBuf::from("/srv/proj")
        );
    }
}
