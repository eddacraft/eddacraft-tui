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
//! - `register [PATH] [--all] [--persist]` — register a worktree for durable
//!   daemon protection (ACTMO-015/018); `--persist` also records it under the
//!   `register_on_start` key so the daemon re-registers it on every startup
//!   (ACTMO-019).
//! - `unregister [PATH] [--persist]` — drop a worktree's durable protection
//!   (idempotent); `--persist` also removes it from `register_on_start`.
//! - `list` — show the admission mode, allow entries, the live registry, and the
//!   `register_on_start` set.
//!
//! Confinement (`mode`/`allow`/`deny`) changes take effect for **new** daemon
//! connections (the daemon reads the config per connection); no restart is
//! required. Registration (`register`/`unregister`) is a live daemon RPC.

use std::collections::BTreeSet;

use anvil_intercept::confinement::{self, AdmissionModeFile, MatchKind};
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};

use crate::GlobalArgs;
use crate::registration::{self, WorktreeRegistration, WorktreeUnregistration};

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
    /// Register a worktree for durable daemon protection (ACTMO-015). With no
    /// PATH, registers the current worktree.
    Register(RegisterArgs),
    /// Unregister a worktree's durable protection (ACTMO-015). Idempotent.
    Unregister(UnregisterArgs),
    /// Show the admission mode, allow entries, and registered worktrees.
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

#[derive(Debug, Args)]
struct RegisterArgs {
    /// The worktree to register. Defaults to the current directory.
    #[arg(value_name = "PATH")]
    path: Option<std::path::PathBuf>,
    /// Register every exact allowlist entry that is a live, unfenced worktree
    /// (ACTMO-018). Mutually exclusive with PATH.
    #[arg(long, conflicts_with = "path")]
    all: bool,
    /// Also record the worktree in `register_on_start` so the daemon
    /// re-registers it automatically on every startup (ACTMO-019). Persisted to
    /// `workspace.yaml`; survives daemon restarts without re-running this.
    #[arg(long)]
    persist: bool,
}

#[derive(Debug, Args)]
struct UnregisterArgs {
    /// The worktree to unregister. Defaults to the current directory.
    #[arg(value_name = "PATH")]
    path: Option<std::path::PathBuf>,
    /// Also remove the worktree from `register_on_start` so the daemon no longer
    /// re-registers it on startup (ACTMO-019).
    #[arg(long)]
    persist: bool,
}

pub fn run(args: &WorkspaceArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        WorkspaceCommand::Mode(mode_args) => run_mode(mode_args),
        WorkspaceCommand::Allow(allow_args) => run_allow(allow_args),
        WorkspaceCommand::Deny(deny_args) => run_deny(deny_args),
        WorkspaceCommand::Register(register_args) => run_register(register_args),
        WorkspaceCommand::Unregister(unregister_args) => run_unregister(unregister_args),
        WorkspaceCommand::List => run_list(),
    }
}

/// Resolve a `[PATH]` argument to a worktree, defaulting to the current
/// directory. The daemon is server-authoritative for canonical identity, so we
/// pass the path through and let the registration client canonicalise.
fn resolve_target(path: Option<&std::path::PathBuf>) -> Result<std::path::PathBuf> {
    match path {
        Some(path) => Ok(path.clone()),
        None => std::env::current_dir().context("could not resolve the current directory"),
    }
}

fn run_register(args: &RegisterArgs) -> Result<()> {
    if args.all {
        return run_register_all(args.persist);
    }
    let target = resolve_target(args.path.as_ref())?;
    // Resolve to the worktree ROOT (like `anvil start`) so registering from a
    // subdirectory registers the worktree, not the subdir — keeping the
    // session-id keying consistent across surfaces.
    match registration::registerable_worktree(&target) {
        Ok(root) => {
            report_registration(&root, registration::register_worktree_with_daemon(&root));
            // ACTMO-019: `--persist` records the worktree in `register_on_start`
            // independent of the live outcome — it captures the *intent* to
            // protect this worktree on every startup (e.g. the daemon may be
            // down right now). The startup path skips a fenced/gone entry.
            if args.persist {
                persist_register_on_start(&root)?;
            }
        }
        Err(reason) => {
            println!("Cannot register {}: {reason}.", target.display());
        }
    }
    Ok(())
}

/// ACTMO-019: record `root` in the `register_on_start` config so the daemon
/// re-registers it on every startup. Idempotent. `root` is the canonicalised
/// worktree root the daemon will re-derive the same activation id from.
fn persist_register_on_start(root: &std::path::Path) -> Result<()> {
    let mut file = confinement::read_config_file().context("read workspace confinement config")?;
    if file.add_register_on_start(root.to_path_buf()) {
        let written = confinement::write_config_file(&file).context("write confinement config")?;
        println!(
            "Recorded {} in register_on_start ({}); the daemon re-registers it on startup.",
            root.display(),
            written.display()
        );
        warn_if_unadmitted(&file, root);
    } else {
        println!(
            "{} is already in register_on_start; nothing to add.",
            root.display()
        );
    }
    Ok(())
}

/// ACTMO-019 (Council m-5): in `allowlist` mode, a `register_on_start` worktree
/// that the allowlist does not admit becomes a phantom registration — the daemon
/// registers it but every connection to it is refused by admission. Warn so the
/// operator knows to `anvil workspace allow` it. Best-effort, advisory only;
/// `open` mode admits everything so there is nothing to warn about.
fn warn_if_unadmitted(file: &confinement::ConfinementConfigFile, root: &std::path::Path) {
    if file.admission != AdmissionModeFile::Allowlist {
        return;
    }
    let canonical = dunce_canonical(root);
    let admitted = file.allow.iter().any(|entry| {
        let allowed = dunce_canonical(&entry.path);
        match entry.kind {
            MatchKind::Exact => canonical == allowed,
            MatchKind::Prefix => canonical.starts_with(&allowed),
        }
    });
    if !admitted {
        println!(
            "Note: admission mode is `allowlist` and {} is not admitted, so it will not be \
             served until you `anvil workspace allow {}`.",
            root.display(),
            root.display()
        );
    }
}

/// ACTMO-019: remove `root` from the `register_on_start` config.
fn unpersist_register_on_start(root: &std::path::Path) -> Result<()> {
    let mut file = confinement::read_config_file().context("read workspace confinement config")?;
    if file.remove_register_on_start(root) {
        confinement::write_config_file(&file).context("write confinement config")?;
        println!("Removed {} from register_on_start.", root.display());
    } else {
        println!(
            "{} was not in register_on_start; nothing to remove.",
            root.display()
        );
    }
    Ok(())
}

/// Print the outcome of a single registration attempt.
fn report_registration(target: &std::path::Path, outcome: WorktreeRegistration) {
    let shown = target.display();
    match outcome {
        WorktreeRegistration::Registered => println!("Registered {shown}."),
        WorktreeRegistration::Refreshed => println!("Refreshed {shown} (already registered)."),
        WorktreeRegistration::DaemonUnavailable => println!(
            "Daemon unavailable — {shown} not registered. Start it with `anvil start` or \
             `anvil intercept start`."
        ),
        WorktreeRegistration::Fenced(message)
        | WorktreeRegistration::CapExceeded(message)
        | WorktreeRegistration::Rejected(message) => {
            println!("Could not register {shown}: {message}");
        }
    }
}

fn run_unregister(args: &UnregisterArgs) -> Result<()> {
    let target = resolve_target(args.path.as_ref())?;
    // Resolve to the worktree root so unregistering from a subdirectory targets
    // the same session id `register` keyed on. A removed worktree no longer
    // resolves, so fall back to the raw path (best-effort — the reaper already
    // drops gone worktrees).
    let target = registration::registerable_worktree(&target).unwrap_or(target);
    {
        let shown = target.display();
        match registration::unregister_worktree_with_daemon(&target) {
            WorktreeUnregistration::Unregistered => println!("Unregistered {shown}."),
            WorktreeUnregistration::NotRegistered => {
                println!("{shown} was not registered — nothing to do.");
            }
            WorktreeUnregistration::DaemonUnavailable => {
                println!("Daemon unavailable — nothing to unregister.");
            }
            WorktreeUnregistration::Rejected(message) => {
                println!("Could not unregister {shown}: {message}");
            }
        }
    }
    // ACTMO-019: `--persist` also drops it from `register_on_start`, so the
    // daemon stops re-registering it on startup.
    if args.persist {
        unpersist_register_on_start(&target)?;
    }
    Ok(())
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

    // Registry half first (ACTMO-015): the set of durably-registered worktrees,
    // canonicalised so the config half can flag which allow entries are live.
    // Degraded behaviour when the daemon is down: the config half still renders.
    let registered = registered_worktrees();

    println!("Admission mode: {}", mode_label(file.admission));
    if file.allow.is_empty() {
        println!("Allow entries: (none)");
        if file.admission == AdmissionModeFile::Allowlist {
            println!("  Only the primary check-in root of each connection is admitted.");
        }
    } else {
        println!("Allow entries:");
        for entry in &file.allow {
            let mark = match &registered {
                RegisteredSet::Known(set) if is_registered(&entry.path, set) => " [registered]",
                _ => "",
            };
            println!(
                "  {} ({}){mark}",
                entry.path.display(),
                kind_label(entry.kind)
            );
        }
    }

    match &registered {
        RegisteredSet::Unavailable => {
            println!("Registered worktrees: (daemon unavailable)");
        }
        RegisteredSet::Known(set) if set.is_empty() => {
            println!("Registered worktrees: (none)");
        }
        RegisteredSet::Known(set) => {
            println!("Registered worktrees:");
            for worktree in set {
                println!("  {}", worktree.display());
            }
        }
    }

    // ACTMO-019: the persistent auto-registration set. Distinct from the live
    // registry above (which may differ if the daemon is down or a worktree was
    // unregistered this session) — these re-register on every daemon startup.
    if file.register_on_start.is_empty() {
        println!("Register on start: (none)");
    } else {
        println!("Register on start:");
        for worktree in &file.register_on_start {
            let mark = match &registered {
                RegisteredSet::Known(set) if is_registered(worktree, set) => "",
                RegisteredSet::Known(_) => " [not currently registered]",
                RegisteredSet::Unavailable => "",
            };
            println!("  {}{mark}", worktree.display());
        }
    }
    Ok(())
}

/// The durable membership set as seen by `list`, distinguishing "daemon down"
/// (degraded) from "daemon up, nothing registered".
enum RegisteredSet {
    Known(BTreeSet<std::path::PathBuf>),
    Unavailable,
}

/// Query the daemon for the durably-registered worktrees (activation-spine
/// sessions). Canonicalised with `dunce` so they compare equal to the
/// canonicalised allow entries in the join.
fn registered_worktrees() -> RegisteredSet {
    match crate::commands::intercept::query_daemon_status() {
        Ok(status) => {
            // Use the shared accessor (ACTMO-017) so the durable-membership
            // predicate lives in exactly one place.
            let set = status
                .registered_worktrees()
                .iter()
                .map(|worktree| dunce_canonical(worktree))
                .collect();
            RegisteredSet::Known(set)
        }
        Err(_) => RegisteredSet::Unavailable,
    }
}

/// `true` when `path` (canonicalised) is in the registered set.
fn is_registered(path: &std::path::Path, set: &BTreeSet<std::path::PathBuf>) -> bool {
    set.contains(&dunce_canonical(path))
}

fn dunce_canonical(path: &std::path::Path) -> std::path::PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `true` when daemon use is bypassed via `ANVIL_NO_DAEMON` (ACTMO-018).
fn daemon_bypassed() -> bool {
    std::env::var_os("ANVIL_NO_DAEMON").is_some_and(|value| !value.is_empty())
}

/// ACTMO-018: register every **exact** allowlist entry that is a live, unfenced
/// worktree. Prefix entries are skipped with a warning (walking them would be
/// the forbidden filesystem scan); all skips are reported. Bounded strictly by
/// the operator's curated allowlist — never a scan.
fn run_register_all(persist: bool) -> Result<()> {
    if daemon_bypassed() {
        println!("Registration skipped (ANVIL_NO_DAEMON set).");
        return Ok(());
    }
    let file = confinement::read_config_file().context("read workspace confinement config")?;
    if file.admission == AdmissionModeFile::Open {
        println!("No allowlist entries (confinement mode: open).");
        return Ok(());
    }
    if file.allow.is_empty() {
        println!("No allowlist entries to register.");
        return Ok(());
    }

    let mut registered = 0usize;
    let mut prefix_skipped = 0usize;
    let mut skips: Vec<String> = Vec::new();
    // ACTMO-019: roots to add to `register_on_start` when `--persist` is set —
    // the exact entries that resolved to a live worktree (the same set `--all`
    // registered this run).
    let mut persist_roots: Vec<std::path::PathBuf> = Vec::new();
    for entry in &file.allow {
        if matches!(entry.kind, MatchKind::Prefix) {
            prefix_skipped += 1;
            continue;
        }
        // Per-entry progress so a large allowlist never reads as a hang.
        println!("Registering {} ...", entry.path.display());
        // Register the resolved worktree ROOT, not the raw allow entry, so an
        // entry pointing inside a worktree still keys on the worktree.
        let root = match registration::registerable_worktree(&entry.path) {
            Ok(root) => root,
            Err(reason) => {
                skips.push(format!("{} — {reason}", entry.path.display()));
                continue;
            }
        };
        match registration::register_worktree_with_daemon(&root) {
            WorktreeRegistration::Registered | WorktreeRegistration::Refreshed => {
                registered += 1;
                persist_roots.push(root);
            }
            WorktreeRegistration::DaemonUnavailable => {
                println!("Daemon unavailable — stopping. Start it with `anvil start` and retry.");
                // Council m-1: still persist the intent captured before the
                // daemon went away, so `--persist` does not silently drop the
                // worktrees that DID register this run.
                if persist && !persist_roots.is_empty() {
                    persist_register_on_start_all(&persist_roots)?;
                }
                return Ok(());
            }
            WorktreeRegistration::Fenced(message)
            | WorktreeRegistration::CapExceeded(message)
            | WorktreeRegistration::Rejected(message) => {
                skips.push(format!("{} — {message}", entry.path.display()));
            }
        }
    }

    println!(
        "Registered {registered} worktree{}.",
        if registered == 1 { "" } else { "s" }
    );
    if prefix_skipped > 0 {
        println!(
            "{prefix_skipped} prefix entr{} skipped — only exact entries can be registered with --all.",
            if prefix_skipped == 1 { "y" } else { "ies" }
        );
    }
    if !skips.is_empty() {
        println!("Skipped:");
        for skip in &skips {
            println!("  {skip}");
        }
    }
    if persist && !persist_roots.is_empty() {
        persist_register_on_start_all(&persist_roots)?;
    }
    Ok(())
}

/// ACTMO-019: add several worktree roots to `register_on_start` in one
/// read/modify/write, reporting how many were newly recorded.
fn persist_register_on_start_all(roots: &[std::path::PathBuf]) -> Result<()> {
    let mut file = confinement::read_config_file().context("read workspace confinement config")?;
    let mut added = 0usize;
    for root in roots {
        if file.add_register_on_start(root.clone()) {
            added += 1;
        }
    }
    if added > 0 {
        let written = confinement::write_config_file(&file).context("write confinement config")?;
        println!(
            "Recorded {added} worktree{} in register_on_start ({}); the daemon re-registers them on startup.",
            if added == 1 { "" } else { "s" },
            written.display()
        );
    } else {
        println!("All registered worktrees were already in register_on_start.");
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
    fn workspace_register_defaults_path_and_accepts_all() {
        let bare =
            Harness::try_parse_from(["anvil-workspace", "register"]).expect("parse register");
        match bare.command {
            WorkspaceCommand::Register(args) => {
                assert!(args.path.is_none(), "no PATH defaults to cwd");
                assert!(!args.all);
            }
            other => panic!("expected Register, got {other:?}"),
        }

        let all = Harness::try_parse_from(["anvil-workspace", "register", "--all"])
            .expect("parse register --all");
        match all.command {
            WorkspaceCommand::Register(args) => assert!(args.all),
            other => panic!("expected Register, got {other:?}"),
        }

        // PATH and --all are mutually exclusive.
        assert!(
            Harness::try_parse_from(["anvil-workspace", "register", "/srv/p", "--all"]).is_err(),
            "PATH conflicts with --all",
        );
    }

    #[test]
    fn workspace_unregister_parses_optional_path() {
        let parsed = Harness::try_parse_from(["anvil-workspace", "unregister", "/srv/p"])
            .expect("parse unregister PATH");
        match parsed.command {
            WorkspaceCommand::Unregister(args) => {
                assert_eq!(args.path.as_deref(), Some(std::path::Path::new("/srv/p")));
                assert!(!args.persist, "--persist defaults off");
            }
            other => panic!("expected Unregister, got {other:?}"),
        }
    }

    #[test]
    fn workspace_register_persist_parses_with_path_and_all() {
        // ACTMO-019: `--persist` records the worktree in `register_on_start`.
        let with_path =
            Harness::try_parse_from(["anvil-workspace", "register", "/srv/p", "--persist"])
                .expect("parse register --persist");
        match with_path.command {
            WorkspaceCommand::Register(args) => {
                assert_eq!(args.path.as_deref(), Some(std::path::Path::new("/srv/p")));
                assert!(args.persist);
                assert!(!args.all);
            }
            other => panic!("expected Register, got {other:?}"),
        }

        // `--persist` composes with `--all` (populate register_on_start from the
        // allowlist in one shot).
        let with_all =
            Harness::try_parse_from(["anvil-workspace", "register", "--all", "--persist"])
                .expect("parse register --all --persist");
        match with_all.command {
            WorkspaceCommand::Register(args) => {
                assert!(args.all);
                assert!(args.persist);
            }
            other => panic!("expected Register, got {other:?}"),
        }

        let unregister =
            Harness::try_parse_from(["anvil-workspace", "unregister", "/srv/p", "--persist"])
                .expect("parse unregister --persist");
        match unregister.command {
            WorkspaceCommand::Unregister(args) => assert!(args.persist),
            other => panic!("expected Unregister, got {other:?}"),
        }
    }

    #[test]
    fn absolutise_makes_relative_paths_absolute() {
        let abs = absolutise(std::path::Path::new("relative/dir")).expect("absolutise");
        assert!(abs.is_absolute(), "a relative path is absolutised: {abs:?}");
        // An already-absolute path is returned unchanged. "Absolute" is
        // platform-specific: a leading-slash path is absolute on Unix but
        // drive-relative (not absolute) on Windows, which needs a drive prefix.
        #[cfg(unix)]
        let already = std::path::PathBuf::from("/srv/proj");
        #[cfg(windows)]
        let already = std::path::PathBuf::from(r"C:\srv\proj");
        assert_eq!(absolutise(&already).expect("absolutise"), already);
    }
}
