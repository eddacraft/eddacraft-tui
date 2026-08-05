use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use anvil_kernel_types::WatchEventEnvelope;
use anvil_kernel_types::watch_event::{WatchActionResult, WatchDaemonVerdict};
use anyhow::{Context, Result, bail};
use clap::Args;

use crate::GlobalArgs;
use crate::warmup_cache::load_watch_warmup_cache;

#[derive(Debug, Args)]
// CLI flags are naturally booleans; the >3-bool shape is intentional and
// matches the sibling arg structs (StartArgs, CheckArgs, …).
#[allow(clippy::struct_excessive_bools)]
#[command(
    about = "Watch files and report save-time anvil findings.",
    after_help = "Behaviour:\n  - The initial scan builds baseline/readiness state; existing repo contents are not reported as new save-time violations.\n  - Watch and audit skip local tool state, agent worktrees, generated folders, and common caches by default.\n  - The TUI opens only when stdin and stdout are terminals; otherwise watch falls back to plain output.\n\nSave-time daemon:\n  - Save-time validation is served by the anvil daemon. When none is running, an interactive `anvil watch` offers to start one (run `anvil start` to auto-start it instead); pass --no-daemon to decline and use the scoped fallback. Headless, --json, CI, hook, and piped runs never start or offer a daemon and fall back deterministically. A daemon already running is reused without prompting.\n  - ANVIL_WATCH_DAEMON controls routing: unset routes through a live daemon when one answers (default), ANVIL_WATCH_DAEMON=0 opts out (no reuse, no start, no offer), ANVIL_WATCH_DAEMON=1 forces daemon routing (falls back to a scoped check with a warning when no daemon answers).\n  - When daemon routing is engaged but the daemon stops answering (forced-on start, or a mid-session disconnect), watch falls back to a scoped check and reports assurance unavailable{daemon-absent}."
)]
pub struct WatchArgs {
    /// File or directory to scope the watcher (when a file is given, its
    /// parent directory is watched; other files there may also trigger events)
    #[arg(long, short = 'f')]
    file: Option<String>,

    /// Action to run on each change: check (default), gate, or none for an
    /// architecture/dependency-only watch with no code-quality scan.
    #[arg(long, short = 'a')]
    action: Option<String>,

    /// Watch planning documents
    #[arg(long)]
    plans: bool,

    /// Watch source files. Built-in local-noise directories are ignored.
    #[arg(long)]
    source: bool,

    /// Watch everything except built-in local-noise/generated/cache directories.
    #[arg(long)]
    all: bool,

    /// Glob patterns to watch (comma-separated, e.g. "src/**/*.ts,lib/**/*.ts").
    /// Empty = watch everything that passes the built-in denylist.
    #[arg(long)]
    patterns: Option<String>,

    /// Glob patterns to exclude (comma-separated, e.g. "vendor/**,**/*.test.ts").
    /// Bare directory names like "vendor" only match the directory itself;
    /// use "vendor/**" to exclude contents.
    #[arg(long)]
    exclude: Option<String>,

    /// Debounce interval in milliseconds
    #[arg(long)]
    debounce: Option<u64>,

    /// Skip starting (or offering to start) the per-user save-time daemon.
    /// With no daemon answering, an interactive `anvil watch` offers to start
    /// one so save-time validation is daemon-backed; pass `--no-daemon` to
    /// disable daemon contact and rely on the scoped fallback, even when a
    /// daemon is already running. `ANVIL_WATCH_DAEMON=0` is the equivalent
    /// environment opt-out. No daemon is ever started or offered in `--json`,
    /// headless, CI, hook, or piped contexts — those fall back
    /// deterministically.
    #[arg(long = "no-daemon")]
    no_daemon: bool,

    // DSV-048 / ADR-101: the supervisor spawn contract. Provenance ids live
    // in this code comment, not the doc comment — CLIC-010 keeps plan/ADR
    // identifiers out of user-visible help text.
    /// Internal: run as the headless save-time driver the intercept daemon's
    /// supervisor spawns per registered worktree. Forces plain headless
    /// output (no TUI, no stdout banners), never offers or starts a daemon,
    /// routes save-time checks through the already-running daemon, and
    /// appends findings to the driver log (`ANVIL_SAVE_TIME_DRIVER_LOG`)
    /// instead of stdout. Exits non-zero only on unrecoverable setup
    /// failure — never on findings. The scope flags are rejected so the
    /// supervisor argv contract stays canonical.
    #[arg(
        long = "save-time-driver",
        requires = "worktree",
        conflicts_with_all = ["file", "action", "plans", "source", "all", "patterns", "exclude"]
    )]
    save_time_driver: bool,

    /// Canonical worktree root to drive (save-time driver mode only).
    #[arg(long, requires = "save_time_driver", value_name = "PATH")]
    worktree: Option<PathBuf>,
}

impl WatchArgs {
    /// Build the args used by `anvil start --watch` to enter the
    /// watch fallback (LAUNCH-011). Scopes the watcher to the
    /// workspace root (no `--file` override) and accepts the
    /// `FileFilter` denylist as the only scope filter.
    ///
    /// `action: none` keeps this path architecture/dependency-only. GH #1913
    /// changed bare `anvil watch` to run `check` by default, but the LAUNCH-011
    /// fallback is a lightweight save-time watcher whose honesty contract
    /// claims `state: watching` — it must not silently start spawning per-save
    /// `anvil check` runs. Users who want code-quality scanning run
    /// `anvil watch` directly.
    pub fn fallback_for_repo() -> Self {
        Self {
            file: None,
            action: Some("none".to_string()),
            plans: false,
            source: false,
            all: false,
            patterns: None,
            exclude: None,
            debounce: None,
            // The LAUNCH-011 fallback watcher is architecture-only
            // (`action: none`), so it never engages the save-time daemon
            // lifecycle. `anvil start` already owns daemon startup for that
            // path (DLIFE-003), so the offer must stay off here.
            no_daemon: true,
            save_time_driver: false,
            worktree: None,
        }
    }
}

const DEFAULT_WATCH_PATTERNS: &[&str] = &[
    "**/*.md",
    "**/*.aps.md",
    "**/prd.*",
    "**/plan.*",
    "**/spec.*",
];

const SOURCE_PATTERNS: &[&str] = &["src/**/*.ts", "src/**/*.tsx", "lib/**/*.ts"];

// FileFilter owns the hardcoded internal denylist (node_modules, .git, …).
// User --patterns / --exclude are glob filters applied separately by the
// kernel's WatchPatternFilter — they no longer extend FileFilter.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchOutputMode {
    Json,
    Plain {
        reason: PlainWatchReason,
    },
    Tui,
    /// DSV-048: headless save-time driver — no TUI, no stdout banners, no
    /// per-event lines; findings go to the driver log, breadcrumbs to stderr
    /// (the supervisor's crash-capture channel).
    Driver,
}

impl WatchOutputMode {
    /// True when the watch surface should print human-readable banners
    /// (scope, warm-up cache, fallback notices) onto the parent's
    /// channels. False in JSON mode (where stdout is reserved for the
    /// v1 NDJSON event stream) and TUI mode (rendered through the
    /// alt-screen instead). WOUT-003: centralises the rule so a new
    /// mode variant cannot silently start emitting banners on stdout.
    const fn writes_human_banners(self) -> bool {
        matches!(self, Self::Plain { .. })
    }

    const fn writes_json_events(self) -> bool {
        matches!(self, Self::Json)
    }
}

/// WOUT-003: per-line warning channel for advice that is *not* part of
/// the v1 event stream. Stdout is reserved for NDJSON event records in
/// JSON mode; everything else routes to stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WarningChannel {
    Stdout,
    Stderr,
}

impl WarningChannel {
    /// Bare-exclude / scope-mismatch / setup warnings route to stderr in
    /// JSON mode so stdout stays parseable, and to stdout in plain mode
    /// where the user reads them inline with the rest of the watch
    /// surface.
    const fn for_advisory(json_mode: bool) -> Self {
        if json_mode {
            Self::Stderr
        } else {
            Self::Stdout
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlainWatchReason {
    NoTuiFlag,
    StdoutNotTerminal,
    StdinNotTerminal,
}

impl PlainWatchReason {
    fn message(self) -> &'static str {
        match self {
            Self::NoTuiFlag => "--no-tui was passed",
            Self::StdoutNotTerminal => "stdout is not a terminal",
            Self::StdinNotTerminal => "stdin is not a terminal",
        }
    }
}

fn watch_output_mode(
    global: &GlobalArgs,
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> WatchOutputMode {
    if global.json {
        WatchOutputMode::Json
    } else if global.no_tui {
        WatchOutputMode::Plain {
            reason: PlainWatchReason::NoTuiFlag,
        }
    } else if !stdout_is_terminal {
        WatchOutputMode::Plain {
            reason: PlainWatchReason::StdoutNotTerminal,
        }
    } else if !stdin_is_terminal {
        WatchOutputMode::Plain {
            reason: PlainWatchReason::StdinNotTerminal,
        }
    } else {
        WatchOutputMode::Tui
    }
}

/// Normalise a path by canonicalising the longest existing ancestor, then
/// re-appending the remaining suffix. This resolves `..` traversal even when
/// the full path doesn't exist on disk.
fn normalise_path_via_ancestors(path: &std::path::Path) -> PathBuf {
    let mut ancestors: Vec<&std::path::Path> = path.ancestors().collect();
    ancestors.reverse(); // root first

    for ancestor in &ancestors {
        if let Ok(canon) = ancestor.canonicalize()
            && let Ok(suffix) = path.strip_prefix(ancestor)
        {
            // Re-append the remaining components and clean up any remaining ..
            let mut result = canon;
            for component in suffix.components() {
                match component {
                    std::path::Component::ParentDir => {
                        result.pop();
                    }
                    std::path::Component::Normal(c) => {
                        result.push(c);
                    }
                    _ => {}
                }
            }
            return result;
        }
    }
    // Absolute fallback: just return the original
    path.to_path_buf()
}

/// Resolve the effective watch root: if `--file` is given, scope to that path.
/// Returns an error if the resolved path escapes the workspace boundary.
fn resolve_watch_root(workspace_root: &std::path::Path, file_arg: Option<&str>) -> Result<PathBuf> {
    match file_arg {
        Some(f) => {
            let p = std::path::Path::new(f);
            let abs = if p.is_absolute() {
                p.to_path_buf()
            } else {
                workspace_root.join(p)
            };
            // Canonicalise to resolve .. traversal. For non-existent paths,
            // canonicalise the longest existing ancestor then re-append the rest.
            let resolved = abs
                .canonicalize()
                .unwrap_or_else(|_| normalise_path_via_ancestors(&abs));

            // Validate the resolved path is within the workspace
            let canon_ws = workspace_root
                .canonicalize()
                .unwrap_or_else(|_| workspace_root.to_path_buf());
            if !resolved.starts_with(&canon_ws) {
                bail!(
                    "Watch path '{}' escapes workspace root '{}'",
                    resolved.display(),
                    canon_ws.display()
                );
            }

            if resolved.is_dir() {
                Ok(resolved)
            } else {
                // Single file — watch its parent directory
                Ok(resolved
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .map_or_else(|| workspace_root.to_path_buf(), PathBuf::from))
            }
        }
        None => Ok(workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.to_path_buf())),
    }
}

/// The internal `FileFilter` owns the hardcoded denylist and the
/// parseable-extension gate. When the user has supplied their own scoping
/// criterion (e.g. `--patterns '**/*.rs'`), the parseable gate must yield
/// — otherwise events for non-JS files are dropped before the user's
/// pattern matcher ever sees them.
fn build_filter(user_supplied_patterns: bool) -> anvil_kernel::watcher::filter::FileFilter {
    anvil_kernel::watcher::filter::FileFilter::default()
        .with_respect_extensions(!user_supplied_patterns)
}

/// Format the bare-exclude warning line for a single pattern. Pulled out
/// so the message text is testable without spawning processes.
fn format_bare_exclude_warning(pattern: &str) -> String {
    // ASCII-only so it renders cleanly on Windows terminals that are
    // not configured for full Unicode (cmd.exe with a legacy code page,
    // log capture pipelines, dumb TERM environments).
    format!(
        "[warn] --exclude {pattern} matches only a path named exactly \"{pattern}\"; \
         to exclude its contents use --exclude {pattern}/**"
    )
}

/// `--exclude` switched from "directory names" to glob patterns in
/// LAUNCH-001. A user who previously ran `--exclude vendor` will now
/// find their vendor tree silently watched, because the bare name
/// matches only a path equal to "vendor". Detect that shape at parse
/// time and warn with the corrected form.
///
/// WOUT-003: routes through stderr in `--json` mode so the NDJSON event
/// stream on stdout stays parseable; otherwise stdout, alongside the
/// rest of the watch surface. The channel decision goes through
/// [`WarningChannel::for_advisory`] so the policy is testable.
fn warn_on_bare_exclude_patterns(patterns: &[String], json_mode: bool) {
    let channel = WarningChannel::for_advisory(json_mode);
    for pattern in patterns {
        if is_likely_bare_directory_name(pattern) {
            let line = format_bare_exclude_warning(pattern);
            match channel {
                WarningChannel::Stderr => eprintln!("{line}"),
                WarningChannel::Stdout => println!("{line}"),
            }
        }
    }
}

/// A pattern is a "likely bare directory name" if it contains no glob
/// metacharacters and no path separator — i.e. exactly the shape that
/// the previous denylist-based `--exclude` accepted. Empty strings and
/// patterns that look like glob expressions are not warned on.
fn is_likely_bare_directory_name(pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    !pattern.contains(['/', '\\', '*', '?', '[', '{', '!'])
}

/// Print the active include/exclude scope so a viewer can see the LAUNCH-001
/// glob filter is doing something. Silent in JSON mode (where the v1
/// NDJSON event stream owns stdout) and TUI mode (rendered separately).
/// WOUT-003: short-circuit is delegated to [`WatchOutputMode::writes_human_banners`]
/// so a new mode variant cannot accidentally start emitting banners on
/// stdout.
fn print_active_scope(include: &[String], exclude: &[String], mode: WatchOutputMode) {
    if !mode.writes_human_banners() {
        return;
    }
    // ASCII-only so it renders cleanly on Windows terminals without full
    // Unicode support; the watch banner is the first thing a piped or
    // recorded session captures and emoji mojibake at that exact moment
    // is the kind of papercut a hype-builder demo can't afford.
    if include.is_empty() {
        println!("[watching] everything (denylist still applies)");
    } else {
        println!("[watching] {}", include.join(", "));
    }
    if !exclude.is_empty() {
        println!("[excluding] {}", exclude.join(", "));
    }
}

fn print_warmup_cache_status(paths: Option<&Vec<String>>, mode: WatchOutputMode) {
    if !mode.writes_human_banners() {
        return;
    }
    if let Some(paths) = paths {
        println!(
            "[warm-up cache] {} files validated; reconciling with filesystem",
            paths.len()
        );
    }
}

fn warn_if_tui_fell_back(mode: WatchOutputMode) {
    let WatchOutputMode::Plain { reason } = mode else {
        return;
    };
    if reason == PlainWatchReason::NoTuiFlag {
        return;
    }
    eprintln!(
        "[watching] TUI disabled because {}; run anvil watch from an interactive terminal for the watch pane, or pass --no-tui for plain output.",
        reason.message()
    );
}

fn print_tui_startup_message(mode: WatchOutputMode) {
    if !matches!(mode, WatchOutputMode::Tui) {
        return;
    }
    eprintln!(
        "[watching] starting watcher; large repos may take a moment before the watch pane appears."
    );
}

/// Resolve the on-change action (GH #1913).
///
/// Bare `anvil watch` (no `--action`) now defaults to `check` so save-time
/// code-quality scanning (AP-*/GS-*) runs by default — previously the absent
/// case watched architecture/dependency edges only and ran no code checks,
/// which read as protection it was not providing. `--action none` restores the
/// architecture-only watch for users who want just that.
fn resolve_action(action: Option<&str>) -> Result<Option<&str>> {
    match action {
        None | Some("check") => Ok(Some("check")),
        Some("gate") => Ok(Some("gate")),
        Some("none") => Ok(None),
        Some(other) => {
            bail!("Unsupported action: {other}. Supported: check, gate, none (architecture-only)")
        }
    }
}

/// WOUT-003: action-child stdio policy. JSON mode discards the child's
/// stdout so its bytes cannot interleave with the parent's NDJSON event
/// stream; child stderr inherits the parent's stderr (the diagnostic
/// channel). TUI mode discards both because the parent owns the
/// alt-screen.
///
/// `Inherit` forwards directly to the parent's descriptors. `Null`
/// discards. We deliberately do NOT use piped: piped pipes that nobody
/// reads will block the child once the OS pipe buffer fills (~64 KiB on
/// Linux), which would deadlock long-running gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildStdio {
    Inherit,
    Null,
}

impl ChildStdio {
    fn apply(self) -> std::process::Stdio {
        match self {
            Self::Inherit => std::process::Stdio::inherit(),
            Self::Null => std::process::Stdio::null(),
        }
    }
}

/// Resolve the (stdout, stderr) policy for an action child in a given
/// parent context. Pulled out as a pure function so the WOUT-003
/// invariant "JSON parent → child stdout null" is testable without
/// spawning processes (`std::process::Command` does not expose its
/// configured stdio for inspection).
const fn child_stdio_policy(json: bool, tui_parent: bool) -> (ChildStdio, ChildStdio) {
    if tui_parent {
        (ChildStdio::Null, ChildStdio::Null)
    } else if json {
        (ChildStdio::Null, ChildStdio::Inherit)
    } else {
        (ChildStdio::Inherit, ChildStdio::Inherit)
    }
}

/// The one-line plain-surface advisory for a daemon-absent fallback. Names
/// `anvil start` so the watch surface itself teaches recovery (UJ-006).
/// ASCII-only, matching the watch banner policy for Windows terminals.
fn fallback_advisory_line(
    assurance: &anvil_intercept_proto::protocol::WorkspaceAssurance,
) -> String {
    format!(
        "[warn] anvil watch: save-time daemon unavailable -- falling back to a scoped check ({}); run `anvil start` for daemon-backed validation",
        crate::commands::watch_save_time::assurance_label(assurance),
    )
}

// ---------------------------------------------------------------------------
// DLIFE-004: daemon lifecycle wiring for `anvil watch`
// ---------------------------------------------------------------------------

/// The lifecycle decision `anvil watch` takes for the per-user save-time
/// daemon this run, before the watcher starts. Pure so every branch is
/// unit-tested; the orchestration ([`ensure_watch_daemon`]) supplies the
/// flags, env, TTY, and liveness probe.
///
/// Unlike `anvil start` — which auto-starts at the activation moment
/// (DLIFE-003) — `anvil watch` follows the ADR-082 tiered posture: it
/// *prompts* in an interactive TTY when no daemon answers and *falls back*
/// deterministically everywhere else (headless / `--json` / CI / hook /
/// piped), never prompting or hanging automation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchDaemonPlan {
    /// A daemon already answers the worktree — reuse it; no prompt, no spawn.
    ReuseLive,
    /// Do not start or prompt this run; the scoped fallback is preserved.
    /// The reason drives honest copy that names only the surface the user
    /// actually engaged (Council: don't blame a flag/env they didn't set).
    FallBack(WatchFallbackReason),
    /// Interactive TTY with no live daemon — offer to start one.
    Prompt,
}

/// Why `anvil watch` did not start (or offer) a daemon this run. Kept
/// distinct from [`anvil_intercept::ensure::NoStartReason`] so the rendered
/// copy can separate the two opt-out *surfaces* — they have different reuse
/// semantics and naming both when only one was used misleads the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchFallbackReason {
    /// `ANVIL_WATCH_DAEMON=0`: the hard opt-out — disables all daemon
    /// contact for this run (no reuse, no start, no offer).
    RoutingDisabled,
    /// `--no-daemon`: explicit per-run opt-out — no reuse, start, or offer.
    NoDaemonFlag,
    /// Headless / `--json` / CI / hook / piped: a deterministic context with
    /// no interactive consent surface.
    NonInteractive,
    /// This platform cannot background-launch a daemon yet, so there is
    /// nothing to offer even in a TTY.
    PlatformUnsupported,
}

/// The decision inputs for [`watch_daemon_plan`]. A named-field record (not
/// positional bools) so call sites and the truth-table tests cannot transpose
/// two flags silently. Each field is sourced once in [`ensure_watch_daemon`].
// All fields are independent runtime signals; the >3-bool shape is the data,
// not an abstraction smell.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WatchDaemonContext {
    /// `ANVIL_WATCH_DAEMON=0`: the hard opt-out — no daemon contact at all
    /// (no reuse, no start, no offer).
    routing_disabled: bool,
    /// `--no-daemon`: no daemon contact for this run.
    no_daemon: bool,
    /// `--json`: never prompt; fold into the deterministic, parse-safe fallback.
    json: bool,
    /// A terminal on stdin+stdout with no non-interactive env signal.
    interactive: bool,
    /// A live daemon already answered the worktree probe this run.
    daemon_live: bool,
    /// This platform can actually background-launch a daemon (Unix and Windows
    /// since CIB-072). When false, the ensure primitive returns
    /// `PlatformUnsupported` regardless, so offering to start one would ask a
    /// question that cannot change the outcome — the planner skips the prompt.
    platform_can_spawn: bool,
}

/// DSV-048: whether this run engages the daemon lifecycle (reuse / offer /
/// fallback line) at all. Only a `check` watch routes through the save-time
/// daemon, and driver mode never offers or starts one — the supervisor that
/// spawned the driver owns the daemon lifecycle (ADR-101); the driver only
/// talks to a daemon that already answers via the dispatcher's routing probe.
fn watch_daemon_offer_applies(action: Option<&str>, save_time_driver: bool) -> bool {
    action == Some("check") && !save_time_driver
}

/// Decide the watch daemon lifecycle plan. Pure; see [`WatchDaemonPlan`].
///
/// Precedence (ADR-082 §"Settled startup mode"):
/// 1. `routing_disabled` (`ANVIL_WATCH_DAEMON=0`) → `FallBack(RoutingDisabled)`:
///    the hard opt-out disables daemon contact entirely (no reuse, no spawn),
///    so liveness is moot.
/// 2. `no_daemon` (`--no-daemon`) → `FallBack(NoDaemonFlag)`: the per-run
///    opt-out also disables reuse, so liveness is moot.
/// 3. `daemon_live` → `ReuseLive`: a live daemon is used through the existing
///    routing probe; starting or offering a second one is pointless ceremony.
/// 4. `json` or non-interactive context → `FallBack(NonInteractive)`: never
///    prompt or hang automation; `--json` stays parse-safe.
/// 5. `!platform_can_spawn` (interactive, no live daemon, would otherwise
///    prompt) → `FallBack(PlatformUnsupported)`: on a platform where the ensure
///    primitive can never start a daemon, asking for consent is a question that
///    cannot change the outcome, so skip the prompt and say so honestly.
/// 6. Interactive TTY, no live daemon, can spawn → `Prompt`. Note
///    `ANVIL_WATCH_DAEMON=1` (`ForcedOn`) is *not* `routing_disabled`, so it
///    reaches this branch when no daemon answers in a TTY — offering to start
///    one is consistent with the user having forced routing on (pinned by the
///    planner tests).
fn watch_daemon_plan(ctx: WatchDaemonContext) -> WatchDaemonPlan {
    if ctx.routing_disabled {
        return WatchDaemonPlan::FallBack(WatchFallbackReason::RoutingDisabled);
    }
    if ctx.no_daemon {
        return WatchDaemonPlan::FallBack(WatchFallbackReason::NoDaemonFlag);
    }
    if ctx.daemon_live {
        return WatchDaemonPlan::ReuseLive;
    }
    if ctx.json || !ctx.interactive {
        return WatchDaemonPlan::FallBack(WatchFallbackReason::NonInteractive);
    }
    if !ctx.platform_can_spawn {
        return WatchDaemonPlan::FallBack(WatchFallbackReason::PlatformUnsupported);
    }
    WatchDaemonPlan::Prompt
}

/// Whether `anvil watch` has an interactive consent surface for offering
/// daemon startup. Requires a terminal on *both* stdin (to read the answer)
/// and stdout, and no CI / hook / explicit non-interactive signal
/// ([`crate::is_non_interactive_env`]).
///
/// Deliberately stricter than `anvil start`'s `start_is_interactive`, which
/// gates on stdout only: `start` never reads a reply, but `watch` reads the
/// y/n answer from stdin, so a session with a redirected stdin (e.g. a
/// `script`-driven harness presenting a PTY stdout) must still be classified
/// non-interactive here rather than blocking on an unreadable prompt.
fn watch_is_interactive() -> bool {
    use std::io::IsTerminal as _;
    !crate::is_non_interactive_env()
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
}

/// Render the one-line outcome of an *attempted* daemon ensure (the consented
/// [`WatchDaemonPlan::Prompt`] path) or a reuse. Deterministic fallbacks have
/// their own copy in [`render_watch_fallback_line`].
///
/// Honesty contract (module risk): the line reports the lifecycle ACTION
/// only — it never claims protection is active, and every non-started path
/// names the preserved scoped fallback so the user is never left thinking
/// save-time validation silently vanished.
fn render_watch_daemon_line(outcome: &anvil_intercept::ensure::EnsureOutcome) -> String {
    use anvil_intercept::ensure::{EnsureOutcome, NoStartReason};
    let body = match outcome {
        // Mirrors `anvil start`'s deferred-attestation phrasing: the daemon has
        // bound, but it only attests this worktree once the editor's MCP client
        // (re)connects — so this must not claim active protection.
        EnsureOutcome::Started => "started the per-user save-time daemon; \
             save-time checks route through it once your editor's MCP client connects."
            .to_owned(),
        EnsureOutcome::Reused => {
            "reusing the per-user save-time daemon already running.".to_owned()
        }
        EnsureOutcome::NoStart {
            reason: NoStartReason::PlatformUnsupported,
        } => "background start is not yet available on this platform; \
             save-time validation uses the scoped fallback."
            .to_owned(),
        // The `MaySpawn` ensure path only ever yields `PlatformUnsupported`
        // among the `NoStart` reasons (OptOut / NonInteractive are decided
        // earlier by the planner and rendered by `render_watch_fallback_line`).
        // Keep the arm total and defensive rather than `unreachable!`.
        EnsureOutcome::NoStart { .. } => {
            "not started; save-time validation uses the scoped fallback.".to_owned()
        }
        EnsureOutcome::Failed { recovery } => format!(
            "could not start the daemon — {recovery} \
             Save-time validation uses the scoped fallback until then."
        ),
    };
    format!("anvil watch: daemon: {body}")
}

/// Render the one-line copy for a deterministic, non-started fallback. Names
/// only the surface the user actually engaged so the message never blames a
/// flag or env var they did not set (Council finding), and always names the
/// preserved scoped fallback.
fn render_watch_fallback_line(reason: WatchFallbackReason) -> String {
    let body = match reason {
        WatchFallbackReason::RoutingDisabled => {
            "not started (ANVIL_WATCH_DAEMON=0 disables daemon contact); \
             save-time validation uses the scoped fallback."
        }
        WatchFallbackReason::NoDaemonFlag => {
            "not started (--no-daemon); save-time validation uses the scoped fallback."
        }
        WatchFallbackReason::NonInteractive => {
            "not started (non-interactive: CI, hook, piped output, or --json); \
             run `anvil watch` in a terminal to start it, or `anvil start` to \
             auto-start it; save-time validation uses the scoped fallback."
        }
        WatchFallbackReason::PlatformUnsupported => {
            "background start is not yet available on this platform; \
             save-time validation uses the scoped fallback."
        }
    };
    format!("anvil watch: daemon: {body}")
}

/// The lifecycle line shown when the user declines the interactive offer.
/// Distinct from a deliberate `--no-daemon` opt-out (it points at how to
/// start one later) and from a non-interactive fallback (the user *was*
/// asked).
fn watch_daemon_declined_line() -> String {
    "anvil watch: daemon: not started (declined); save-time validation uses \
     the scoped fallback. Run `anvil start` to start it later."
        .to_owned()
}

fn watch_daemon_probe_applies(routing_disabled: bool, no_daemon: bool, json: bool) -> bool {
    !routing_disabled && !no_daemon && !json
}

/// Resolve and act on the watch daemon lifecycle for a `check` watch, then
/// return the human-readable lifecycle line to surface on stderr (or `None`
/// in `--json` mode, where the run is deterministic and the stdout NDJSON
/// stream must stay the sole document).
///
/// Side effects are confined to the interactive [`WatchDaemonPlan::Prompt`]
/// branch: it asks on stderr and, on consent, drives the DLIFE-002 ensure
/// primitive through [`crate::commands::intercept::ensure_save_time_daemon`].
/// Reuse / fallback branches never touch a launcher. `--json` callers never
/// reach the prompt (the planner folds `json` into the non-interactive
/// fallback), so this can never block a machine-readable run.
fn ensure_watch_daemon(
    no_daemon: bool,
    json: bool,
    workspace_root: &std::path::Path,
) -> Option<String> {
    use crate::commands::watch_save_time::{DaemonRoutingMode, daemon_routing_mode};
    use anvil_intercept::ensure::{EnsureOutcome, StartCapability};

    let routing_disabled = daemon_routing_mode() == DaemonRoutingMode::Disabled;
    // Probe for a live daemon only when the answer can change the outcome:
    // both opt-outs short-circuit without any daemon contact, and `--json`
    // suppresses the line entirely below (so reuse vs fallback both render to
    // nothing — no point paying a socket round-trip on every headless run).
    // NOTE: this is *not* the same probe the dispatcher makes when it builds
    // its save-time client. That second probe must stay: a daemon the prompt
    // path just *started* was not live at this point, and only the later probe
    // observes it. The two are intentionally independent, not redundant.
    let daemon_live = watch_daemon_probe_applies(routing_disabled, no_daemon, json)
        && crate::commands::watch_save_time::query_workspace_status(workspace_root).is_some();

    let plan = watch_daemon_plan(WatchDaemonContext {
        routing_disabled,
        no_daemon,
        json,
        interactive: watch_is_interactive(),
        daemon_live,
        // Unix and Windows can background-launch the daemon; on other platforms
        // the ensure primitive returns `PlatformUnsupported`, so the planner
        // renders that honestly instead of prompting for nothing.
        platform_can_spawn: cfg!(any(unix, windows)),
    });

    let line = match plan {
        WatchDaemonPlan::ReuseLive => render_watch_daemon_line(&EnsureOutcome::Reused),
        WatchDaemonPlan::FallBack(reason) => render_watch_fallback_line(reason),
        WatchDaemonPlan::Prompt => {
            let consented = crate::prompt_yes_no(
                "No save-time daemon is running. Start one now for daemon-backed validation?",
                true,
            )
            .unwrap_or(false);
            if consented {
                eprintln!("anvil watch: ensuring the per-user save-time daemon is running…");
                let outcome =
                    crate::commands::intercept::ensure_save_time_daemon(StartCapability::MaySpawn);
                render_watch_daemon_line(&outcome)
            } else {
                watch_daemon_declined_line()
            }
        }
    };

    // Structured trace so a fleet operator reading the JSON tracing sink sees
    // the lifecycle decision even when the user's stderr is discarded — in
    // particular a fallback or a failed spawn that quietly drops to the scoped
    // check. The `line` is the user-facing copy; this is the machine record.
    tracing::debug!(outcome = %line, "anvil watch daemon lifecycle");

    // `--json` keeps stdout reserved for the v1 NDJSON event stream; the
    // lifecycle line is informational, so suppress it there entirely rather
    // than relying on stderr alone to keep the run deterministic.
    if json { None } else { Some(line) }
}

/// Build the Command for action dispatch (extracted for testability).
///
/// `tui_parent` (LAUNCH-002): when true, the parent is in TUI mode. The child
/// receives `--no-tui` regardless of the parent's `--no-tui` flag (otherwise
/// two Ratatui sessions would fight over the same alternate-screen).
fn build_action_command(
    exe: &std::path::Path,
    action: &str,
    check_paths: &[String],
    workspace_root: &std::path::Path,
    json: bool,
    no_tui: bool,
    tui_parent: bool,
) -> std::process::Command {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg(action);
    // `anvil check` requires an explicit file scope or it exits with
    // "No files specified" — so a bare `check` dispatch would fail every
    // cycle and never scan.
    //
    // RLB-007 (GH #2156): when the kernel surfaces the files that changed this
    // cycle, scope `check` to exactly those paths instead of re-walking the
    // whole repo. The full-repo `check --all` per save was ~100% of the
    // watch CPU cost the beta tester reported (one agent ≈ 7 of 16 cores).
    // `check_paths` are absolute, so they resolve regardless of the child's
    // cwd and honour any `--file`/`--patterns` watch scope.
    //
    // Empty `check_paths` falls back to `--all`: that covers the delete-driven
    // and initial dispatches (a delete can break imports in *other* files, so
    // a full walk is the safe default), and preserves the #1913 contract that
    // a bare `check` dispatch always scans untracked/new files and works
    // without git (unlike `--changed`). `gate` self-scopes via git status, so
    // it ignores `check_paths` and needs no flag.
    let scoped_check = action == "check" && !check_paths.is_empty();
    if action == "check" && check_paths.is_empty() {
        cmd.arg("--all");
    }
    if json {
        cmd.arg("--json");
    }
    if no_tui || tui_parent {
        cmd.arg("--no-tui");
    }
    // Scoped paths go LAST, behind a `--` end-of-options separator, so a file
    // whose absolute path could ever be mistaken for a flag is parsed as a
    // positional. (Absolute paths start with `/` today, so this is defence in
    // depth, but it keeps the contract robust if the path form ever changes.)
    // Must follow `--json`/`--no-tui`: everything after `--` is positional.
    if scoped_check {
        cmd.arg("--");
        cmd.args(check_paths);
    }
    cmd.current_dir(workspace_root);
    let (stdout_policy, stderr_policy) = child_stdio_policy(json, tui_parent);
    cmd.stdout(stdout_policy.apply());
    cmd.stderr(stderr_policy.apply());
    cmd
}

struct WatchJsonEmitterState {
    next_seq: u64,
    writer: Box<dyn std::io::Write + Send>,
}

#[derive(Clone)]
struct WatchJsonTerminalError {
    kind: std::io::ErrorKind,
    message: String,
}

#[derive(Clone)]
struct WatchJsonEmitter {
    inner: Arc<std::sync::Mutex<WatchJsonEmitterState>>,
    terminal_error: Arc<std::sync::Mutex<Option<WatchJsonTerminalError>>>,
}

impl WatchJsonEmitter {
    fn new(writer: Box<dyn std::io::Write + Send>) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(WatchJsonEmitterState {
                next_seq: 0,
                writer,
            })),
            terminal_error: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    fn stdout() -> Self {
        Self::new(Box::new(std::io::stdout()))
    }

    fn emit_envelope(&self, mut envelope: WatchEventEnvelope) -> std::io::Result<()> {
        let mut state = recover(self.inner.lock());
        envelope.seq = state.next_seq;
        state.next_seq = state.next_seq.saturating_add(1);
        let result = (|| {
            serde_json::to_writer(&mut state.writer, &envelope)?;
            state.writer.write_all(b"\n")?;
            state.writer.flush()
        })();
        if let Err(err) = &result {
            let mut terminal = recover(self.terminal_error.lock());
            if terminal.is_none() {
                *terminal = Some(WatchJsonTerminalError {
                    kind: err.kind(),
                    message: err.to_string(),
                });
            }
        }
        result
    }

    fn terminal_error(&self) -> Option<std::io::Error> {
        recover(self.terminal_error.lock())
            .clone()
            .map(|error| std::io::Error::new(error.kind, error.message))
    }

    fn emit_kernel_event(&self, event: &anvil_kernel_types::EngineEvent) -> std::io::Result<()> {
        self.emit_envelope(WatchEventEnvelope::from_engine_event(event))
    }

    fn emit_action_result(
        &self,
        timestamp: impl Into<String>,
        result: WatchActionResult,
    ) -> std::io::Result<()> {
        self.emit_envelope(WatchEventEnvelope::from_action_result(0, timestamp, result))
    }
}

fn continue_after_watch_json_emit(result: std::io::Result<()>) -> std::io::Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => Ok(false),
        Err(err) => Err(err),
    }
}

fn wire_enum_token<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("wire enums are infallibly serialisable")
        .as_str()
        .expect("wire enums serialise as strings")
        .to_owned()
}

fn daemon_watch_action_result(
    action: &str,
    elapsed: std::time::Duration,
    response: &anvil_intercept_proto::protocol::ValidatePathsResponse,
) -> WatchActionResult {
    WatchActionResult {
        action: action.to_owned(),
        exit_code: Some(crate::commands::watch_save_time::verdict_exit_code(
            response,
        )),
        duration_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        error_detail: None,
        daemon_verdict: Some(WatchDaemonVerdict {
            assurance_state: wire_enum_token(&response.workspace_assurance.state),
            assurance_reason: response
                .workspace_assurance
                .reason
                .map(|reason| wire_enum_token(&reason)),
            coverage: wire_enum_token(&response.coverage),
            check_families: response
                .check_families
                .iter()
                .map(wire_enum_token)
                .collect(),
            finding_count: u64::try_from(response.diagnostics.len()).unwrap_or(u64::MAX),
            diagnostics: response.diagnostics.clone(),
        }),
    }
}

/// Action dispatcher (LAUNCH-002).
///
/// Owns the worker thread, the in-flight `Child`, the rerun atomics, and a
/// cancellation flag. `Drop` cancels, kills any in-flight child, and joins
/// the worker — fixing a pre-existing leak where the previous fire-and-forget
/// `thread::spawn` worker held child stdio descriptors and rerun atomics
/// across the parent's exit.
///
/// Both watch.rs branches (TUI and non-TUI) use this. In TUI mode, a
/// `SyncSender<ActionResultLine>` is provided and child stdio is discarded
/// (the parent owns the alt-screen). In non-TUI mode, the sender is `None`
/// and child stdio is inherited (bit-for-bit identical to the previous
/// behaviour).
pub(crate) struct ActionDispatcher(std::sync::Arc<DispatcherInner>);

/// Bundle the TUI-side action plumbing so signatures don't grow two
/// `Option<…>` parameters in lockstep.
pub(crate) struct WatchActionLink<'a> {
    pub action_rx: &'a std::sync::mpsc::Receiver<anvil_tui::surfaces::watch::ActionResultLine>,
    pub dispatcher: &'a ActionDispatcher,
}

/// Outcome of waiting on an in-flight child. `wait_for_completion` returns
/// this so the caller can populate `ActionResultLine.error_detail` with a
/// cause-specific string instead of the generic "spawn failed: …" the
/// footer used to print for cancellations and signal-kills (#1279 review).
enum WaitOutcome {
    /// Child exited; payload is the OS-reported exit code (None if the
    /// child was terminated by a signal).
    Exited(Option<i32>),
    /// Cancellation (Ctrl-C / `shutdown()`) terminated the child.
    Cancelled,
    /// `try_wait()` returned `Err`. Payload is the human-readable reason.
    WaitFailed(String),
}

impl WaitOutcome {
    /// Convert a wait outcome into the `(exit_code, error_detail)` pair
    /// `send_result` needs. Pulled out as a pure function so the
    /// `Cancelled → "cancelled"` and `WaitFailed → "wait failed: …"`
    /// invariants (#1279 review: cause-specific `error_detail`) can be
    /// asserted without spawning child processes or racing the worker
    /// thread. Adversarial regression: if a future refactor restored
    /// the pre-#1279 "spawn failed" string for cancellation outcomes,
    /// the unit test below catches it deterministically.
    fn to_send_args(&self) -> (Option<i32>, Option<String>) {
        match self {
            Self::Exited(code) => (*code, None),
            Self::Cancelled => (None, Some("cancelled".to_string())),
            Self::WaitFailed(reason) => (None, Some(format!("wait failed: {reason}"))),
        }
    }
}

struct DispatcherInner {
    action: String,
    workspace_root: PathBuf,
    json: bool,
    no_tui_arg: bool,
    /// Parent is in TUI mode → force `--no-tui` on the child and discard
    /// child stdio. See `build_action_command` for the rationale.
    tui_parent: bool,
    sender: Option<std::sync::mpsc::SyncSender<anvil_tui::surfaces::watch::ActionResultLine>>,
    running: AtomicBool,
    pending: AtomicBool,
    cancel: AtomicBool,
    /// RLB-007: absolute paths of files changed since the last dispatch
    /// drained them. `on_snapshot` accumulates here so coalesced reruns scope
    /// `anvil check` to every file that changed while a prior scan was in
    /// flight, not just the most recent. A `BTreeSet` dedups repeated saves of
    /// the same file and gives the child a deterministic argument order.
    /// Drained (taken) at the start of each `check` dispatch.
    pending_paths: std::sync::Mutex<std::collections::BTreeSet<String>>,
    /// In-flight child process. Held in a mutex so `shutdown()` can kill
    /// it from another thread while the worker is polling `try_wait()`.
    in_flight: std::sync::Mutex<Option<std::process::Child>>,
    worker: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    /// DSV-007 / v0.8: the save-time daemon client. `Some` only for a `check`
    /// watch when routing is explicitly forced on, or when default-on routing
    /// finds a live daemon serving the save-time status verb. `None` ⇒ the
    /// subprocess-only path runs unchanged (no daemon-absent WARN). Held behind
    /// a `Mutex` because the worker thread owns the connection-lifecycle latch
    /// across coalesced dispatches.
    save_time: Option<std::sync::Mutex<crate::commands::watch_save_time::WatchSaveTimeClient>>,
    /// DSV-048: the driver-mode findings log. `Some` ⇒ daemon verdicts append
    /// to this child-owned log (never stdout) with a one-line stderr summary
    /// when a batch produces findings.
    driver_log: Option<std::sync::Arc<crate::commands::watch_driver::DriverLog>>,
    json_emitter: Option<WatchJsonEmitter>,
    /// Test-only override for `current_exe()`.
    #[cfg(test)]
    exe_override: Option<PathBuf>,
}

/// Build the save-time daemon client for a dispatcher, or `None` to keep the
/// subprocess-only path. `None` when routing is disabled, the action is not
/// `check`, or default-on routing cannot prove a live daemon is serving the
/// save-time status verb (treated as "no daemon infrastructure" ⇒ behave exactly
/// as pre-DSV, no WARN). Unix uses the socket transport; Windows the named-pipe
/// transport (DSV-011).
fn save_time_client_applies(
    action: &str,
    mode: crate::commands::watch_save_time::DaemonRoutingMode,
    no_daemon: bool,
) -> bool {
    action == "check"
        && mode != crate::commands::watch_save_time::DaemonRoutingMode::Disabled
        && !no_daemon
}

#[cfg(unix)]
fn build_save_time_client(
    action: &str,
    workspace_root: &std::path::Path,
    no_daemon: bool,
) -> Option<std::sync::Mutex<crate::commands::watch_save_time::WatchSaveTimeClient>> {
    use crate::commands::watch_save_time::{
        DaemonRoutingMode, SaveTimeTransport, SocketSaveTimeTransport, WatchSaveTimeClient,
        daemon_routing_mode,
    };
    let mode = daemon_routing_mode();
    if !save_time_client_applies(action, mode, no_daemon) {
        return None;
    }
    let transport = SocketSaveTimeTransport::resolve()?;
    if mode == DaemonRoutingMode::DefaultOnWhenLive
        && transport.workspace_status(workspace_root).is_err()
    {
        return None;
    }
    Some(std::sync::Mutex::new(WatchSaveTimeClient::new(
        Box::new(transport),
        workspace_root.to_path_buf(),
    )))
}

#[cfg(windows)]
fn build_save_time_client(
    action: &str,
    workspace_root: &std::path::Path,
    no_daemon: bool,
) -> Option<std::sync::Mutex<crate::commands::watch_save_time::WatchSaveTimeClient>> {
    use crate::commands::watch_save_time::{
        DaemonRoutingMode, SaveTimeTransport, WatchSaveTimeClient, WindowsPipeSaveTimeTransport,
        daemon_routing_mode,
    };
    let mode = daemon_routing_mode();
    if !save_time_client_applies(action, mode, no_daemon) {
        return None;
    }
    let transport = WindowsPipeSaveTimeTransport::resolve()?;
    if mode == DaemonRoutingMode::DefaultOnWhenLive
        && transport.workspace_status(workspace_root).is_err()
    {
        return None;
    }
    Some(std::sync::Mutex::new(WatchSaveTimeClient::new(
        Box::new(transport),
        workspace_root.to_path_buf(),
    )))
}

#[cfg(not(any(unix, windows)))]
fn build_save_time_client(
    _action: &str,
    _workspace_root: &std::path::Path,
    _no_daemon: bool,
) -> Option<std::sync::Mutex<crate::commands::watch_save_time::WatchSaveTimeClient>> {
    None
}

/// Recover from a poisoned mutex by extracting the inner guard. The mutex
/// protects only transient state (the in-flight `Child` or the worker
/// `JoinHandle`); a poison from an unrelated panic must not leak the child
/// or strand the worker. Council finding: kernel-maintainer.
fn recover<'a, T>(
    result: Result<
        std::sync::MutexGuard<'a, T>,
        std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
    >,
) -> std::sync::MutexGuard<'a, T> {
    result.unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl ActionDispatcher {
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn new(
        action: String,
        workspace_root: PathBuf,
        json: bool,
        no_tui_arg: bool,
        tui_parent: bool,
        sender: Option<std::sync::mpsc::SyncSender<anvil_tui::surfaces::watch::ActionResultLine>>,
        driver_log: Option<std::sync::Arc<crate::commands::watch_driver::DriverLog>>,
        no_daemon: bool,
        json_emitter: Option<WatchJsonEmitter>,
    ) -> Self {
        let save_time = build_save_time_client(&action, &workspace_root, no_daemon);
        Self(std::sync::Arc::new(DispatcherInner {
            action,
            workspace_root,
            json,
            no_tui_arg,
            tui_parent,
            sender,
            running: AtomicBool::new(false),
            pending: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            pending_paths: std::sync::Mutex::new(std::collections::BTreeSet::new()),
            in_flight: std::sync::Mutex::new(None),
            worker: std::sync::Mutex::new(None),
            save_time,
            driver_log,
            json_emitter,
            #[cfg(test)]
            exe_override: None,
        }))
    }

    /// Trigger a dispatch (or mark a pending rerun if one is in flight).
    /// Called from the watch loop on each post-initial Snapshot event.
    ///
    /// `changed_path` (RLB-007) is the absolute path of the file whose save
    /// produced this snapshot, or `None` for delete-driven snapshots. It is
    /// recorded *before* the trigger dance so that a path arriving while a scan
    /// is in flight is guaranteed to be in `pending_paths` when the coalesced
    /// rerun drains it — no save can be silently dropped from the next scan's
    /// scope.
    ///
    /// Race repair (council finding: kernel-maintainer): if the worker is
    /// in the narrow window between `pending.swap(false)` and
    /// `running.store(false)`, a `pending=true` write here would otherwise
    /// be lost. The worker re-checks `pending` after releasing `running` —
    /// see the worker loop below — so the pending bit is recovered.
    pub(crate) fn on_snapshot(&self, changed_path: Option<&str>) {
        // Only `check` consumes scoped paths; `gate` self-scopes via git status,
        // so skip the accumulation entirely for it rather than growing a set
        // that `run_one_action` would drain and discard (Copilot review #2184).
        if let Some(path) = changed_path
            && self.0.action == "check"
        {
            recover(self.0.pending_paths.lock()).insert(path.to_string());
        }
        if self.0.running.swap(true, Ordering::SeqCst) {
            self.0.pending.store(true, Ordering::SeqCst);
            return;
        }
        let inner = std::sync::Arc::clone(&self.0);
        let handle = std::thread::spawn(move || worker_loop(&inner));
        // Replace any prior handle. When `running.swap(true)` returned
        // `false`, the previous worker has executed `running.store(false)`
        // (the last line of worker_loop) but the closure may not have
        // returned yet. Join the prior handle: cost is microseconds in
        // the common case, and joining captures any panic in the prior
        // worker rather than silently detaching it (#1279 review:
        // copilot flagged the lost panic propagation).
        let mut slot = recover(self.0.worker.lock());
        if let Some(prior) = slot.replace(handle) {
            // Discard the panic payload — in TUI mode we can't render it
            // without corrupting the alt-screen, and in non-TUI mode
            // panics already write to stderr via the default hook.
            let _ = prior.join();
        }
    }

    /// Cancel any in-flight action and join the worker. Idempotent.
    fn shutdown(&self) {
        self.0.cancel.store(true, Ordering::SeqCst);
        // Don't try to coalesce more reruns after a cancel — the worker
        // checks `cancel` before re-iterating.
        self.0.pending.store(false, Ordering::SeqCst);
        if let Some(mut child) = recover(self.0.in_flight.lock()).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let handle_opt = recover(self.0.worker.lock()).take();
        if let Some(handle) = handle_opt {
            let _ = handle.join();
        }
    }
}

impl Drop for ActionDispatcher {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Worker thread main loop. Lives outside `impl ActionDispatcher` because the
/// worker holds an `Arc<DispatcherInner>` and the loop body is read-only —
/// keeping it as a free function makes the lifetime obvious.
///
/// Implements the double-check pattern that closes the lost-pending-rerun
/// race the kernel-maintainer flagged.
fn worker_loop(inner: &DispatcherInner) {
    loop {
        inner.run_one_action();
        if inner.cancel.load(Ordering::SeqCst) {
            break;
        }
        if inner.pending.swap(false, Ordering::SeqCst) {
            continue;
        }
        // No pending. Try to release `running`. Then re-check `pending` in
        // case a caller raced in between our swap and our store.
        inner.running.store(false, Ordering::SeqCst);
        if !inner.pending.load(Ordering::SeqCst) {
            return;
        }
        // A caller set `pending` after we cleared it. Reclaim `running`.
        if inner.running.swap(true, Ordering::SeqCst) {
            // Another worker has already been spawned; defer to it.
            return;
        }
        // We have ownership again. Loop and run the pending action.
    }
    inner.running.store(false, Ordering::SeqCst);
}

impl DispatcherInner {
    fn resolve_exe(&self) -> Option<PathBuf> {
        #[cfg(test)]
        if let Some(p) = self.exe_override.as_ref() {
            return Some(p.clone());
        }
        match std::env::current_exe() {
            Ok(p) => Some(p),
            Err(e) => {
                if !self.tui_parent {
                    tracing::error!(error = %e, "cannot resolve current executable for action dispatch");
                }
                None
            }
        }
    }

    fn run_one_action(&self) {
        let start = std::time::Instant::now();
        let Some(exe) = self.resolve_exe() else {
            self.send_result(
                None,
                start.elapsed(),
                Some("cannot resolve current executable".to_string()),
                None,
            );
            return;
        };
        let mut daemon_notice: Option<anvil_tui::surfaces::watch::DaemonNotice> = None;
        // RLB-007: drain the paths accumulated since the last dispatch and
        // scope `check` to them. Taking (clearing) here means a save that
        // arrives after this point lands in `pending_paths` *and* sets the
        // `pending` bit, so the worker's coalescing rerun picks it up — a path
        // is at worst scanned twice, never skipped.
        //
        // Only `check` accumulates paths (see `on_snapshot`), so skip the lock
        // entirely for `gate` rather than draining an always-empty set
        // (Copilot review #2184).
        let mut check_paths: Vec<String> = if self.action == "check" {
            let mut guard = recover(self.pending_paths.lock());
            std::mem::take(&mut *guard).into_iter().collect()
        } else {
            Vec::new()
        };
        // Observability for the "why didn't watch catch X" support case: record
        // whether this dispatch was scoped to changed paths or fell back to a
        // full `--all` walk, and how many paths it covered.
        if self.action == "check" {
            tracing::debug!(
                scoped = !check_paths.is_empty(),
                path_count = check_paths.len(),
                "watch check dispatch scope"
            );
        }

        // DSV-007 Task 12: route the save-time check through the daemon when
        // enabled. Engaged only for a `check` dispatch that carries changed
        // paths — an empty set is a delete-/initial-driven cycle with nothing to
        // send to `validate_paths` or to scope a fallback to, so it keeps the
        // pre-DSV `--all` safety net below. A daemon verdict skips the
        // subprocess entirely (ADR-061 §3 "never double-scans"); a daemon-absent
        // / mid-session-death fallback warns once and drops through to the
        // existing scoped subprocess (`check_paths` is non-empty here ⇒ scoped,
        // never `--all`).
        if self.action == "check"
            && !check_paths.is_empty()
            && let Some(client) = self.save_time.as_ref()
        {
            use crate::commands::watch_save_time::SaveTimeDecision;
            match recover(client.lock()).validate(check_paths.clone()) {
                SaveTimeDecision::Validated(response) => {
                    self.report_daemon_verdict(&response, start.elapsed());
                    return;
                }
                SaveTimeDecision::FellBack {
                    assurance,
                    scoped_paths,
                    warned,
                } => {
                    self.handle_daemon_fallback_warning(&assurance, warned, &mut daemon_notice);
                    // Fall through to the subprocess scoped to exactly the
                    // client-returned changed paths (non-empty here ⇒ never
                    // `--all`).
                    check_paths = scoped_paths;
                }
            }
        }

        let mut cmd = build_action_command(
            &exe,
            &self.action,
            &check_paths,
            &self.workspace_root,
            self.json,
            self.no_tui_arg,
            self.tui_parent,
        );

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                if !self.tui_parent {
                    // Non-TUI: stderr is inherited (visible to the user)
                    // and tracing's stderr-bound JSON layer is safe to
                    // emit. In TUI mode both would corrupt the alt-screen,
                    // so the footer's `error_detail` is the only surface.
                    tracing::warn!(
                        action = %self.action,
                        error = %e,
                        "failed to spawn action child",
                    );
                    eprintln!("[error] Failed to run action '{}': {e}", self.action);
                }
                self.send_result(
                    None,
                    start.elapsed(),
                    Some(format!("spawn failed: {e}")),
                    daemon_notice,
                );
                return;
            }
        };

        // Park the child so shutdown() can kill it from another thread.
        // Council finding: a shutdown() arriving between spawn and park
        // would skip the kill; check `cancel` immediately after parking
        // to close that window.
        *recover(self.in_flight.lock()) = Some(child);
        if self.cancel.load(Ordering::SeqCst) {
            if let Some(mut child) = recover(self.in_flight.lock()).take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            // Route through `to_send_args` so the "cancelled" string lives
            // in exactly one place — same helper the post-wait branch uses.
            let (exit_code, error_detail) = WaitOutcome::Cancelled.to_send_args();
            self.send_result(exit_code, start.elapsed(), error_detail, daemon_notice);
            return;
        }

        let outcome = self.wait_for_completion();
        if !self.tui_parent
            && let WaitOutcome::Exited(Some(c)) = &outcome
            && *c != 0
        {
            eprintln!("[warn] Action '{}' exited with code {c}", self.action);
            tracing::info!(action = %self.action, exit_code = *c, "action exited non-zero");
        }
        // Footer renders `error_detail` verbatim — `to_send_args` keeps
        // cancellation as "cancelled" and wait failures as "wait failed:
        // …", never "spawn failed" (#1279 review).
        let (exit_code, error_detail) = outcome.to_send_args();
        self.send_result(exit_code, start.elapsed(), error_detail, daemon_notice);
    }

    fn handle_daemon_fallback_warning(
        &self,
        assurance: &anvil_intercept_proto::protocol::WorkspaceAssurance,
        warned: bool,
        daemon_notice: &mut Option<anvil_tui::surfaces::watch::DaemonNotice>,
    ) {
        if !warned {
            return;
        }
        if self.tui_parent {
            *daemon_notice = Some(anvil_tui::surfaces::watch::DaemonNotice::Fallback {
                message: "daemon: unavailable -- scoped fallback".to_string(),
            });
            return;
        }
        tracing::warn!(
            target: "anvil::watch",
            reason = "daemon-absent",
            "save-time daemon unavailable; falling back to a scoped check",
        );
        // DSV-048: keep the findings log honest about the degraded window —
        // the subsequent subprocess findings land on the crash-capture
        // channel (inherited stderr/stdout), not in this log.
        if let Some(log) = self.driver_log.as_ref() {
            let _ = log.append(format!("{}\n", fallback_advisory_line(assurance)).as_bytes());
        }
        // The structured `tracing::warn!` above is the JSON-mode channel
        // (stderr, structured); the bare advisory line is for the human plain
        // surface only (WOUT-003 — do not mix unstructured text into JSON).
        if !self.json {
            eprintln!("{}", fallback_advisory_line(assurance));
        }
    }

    fn wait_for_completion(&self) -> WaitOutcome {
        loop {
            let mut slot = recover(self.in_flight.lock());
            let Some(child) = slot.as_mut() else {
                // Slot was emptied by `shutdown()` racing with us — the
                // child has been killed. Treat as cancelled so the
                // footer renders accurately.
                return WaitOutcome::Cancelled;
            };
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code();
                    slot.take();
                    return WaitOutcome::Exited(code);
                }
                Ok(None) => {
                    drop(slot);
                    if self.cancel.load(Ordering::SeqCst) {
                        if let Some(mut child) = recover(self.in_flight.lock()).take() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        return WaitOutcome::Cancelled;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    let reason = e.to_string();
                    if !self.tui_parent {
                        tracing::warn!(action = %self.action, error = %e, "child try_wait failed");
                    }
                    slot.take();
                    return WaitOutcome::WaitFailed(reason);
                }
            }
        }
    }

    /// DSV-007: surface a daemon `validate_paths` verdict on the watch output
    /// channel, mirroring how a subprocess `anvil check` would have reported.
    ///
    /// - TUI mode: send the action footer (the alt-screen owns the detail panel).
    /// - JSON mode: emit a typed `action_result` envelope through the shared
    ///   run-scoped allocator. Stdout remains pure WOUT-v1 NDJSON while carrying
    ///   the daemon's exact assurance, coverage, family, and diagnostic evidence.
    /// - Plain mode: render the findings + assurance to stdout.
    fn report_daemon_verdict(
        &self,
        response: &anvil_intercept_proto::protocol::ValidatePathsResponse,
        elapsed: std::time::Duration,
    ) {
        // DSV-048 driver mode: the findings log (child-owned) is the verdict
        // surface — never stdout, and findings never touch an exit code. A
        // non-empty batch also leaves a one-line stderr breadcrumb pointing
        // at the log (stderr is the supervisor's crash-capture channel).
        if let Some(log) = self.driver_log.as_ref() {
            let mut block = Vec::new();
            let render =
                crate::commands::watch_save_time::render_daemon_verdict_plain(&mut block, response)
                    .and_then(|()| log.append(&block));
            if let Err(err) = render {
                tracing::warn!(error = %err, path = %log.path().display(), "failed to append driver findings log");
            }
            if !response.diagnostics.is_empty() {
                eprintln!(
                    "{}",
                    crate::commands::watch_driver::driver_summary_line(
                        response.diagnostics.len(),
                        log.path(),
                    )
                );
            }
            return;
        }
        let exit_code = crate::commands::watch_save_time::verdict_exit_code(response);
        if self.sender.is_some() {
            let summary = crate::commands::watch_save_time::daemon_verdict_summary(response);
            self.send_result_with_assurance(
                Some(exit_code),
                elapsed,
                None,
                Some(anvil_tui::surfaces::watch::DaemonNotice::ClearFallback),
                Some(summary.detail),
                summary.degraded,
            );
        } else if let Some(emitter) = self.json_emitter.as_ref() {
            let result = daemon_watch_action_result(&self.action, elapsed, response);
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            if let Err(err) = emitter.emit_action_result(timestamp, result)
                && err.kind() != std::io::ErrorKind::BrokenPipe
            {
                tracing::warn!(error = %err, "failed to emit JSON daemon verdict");
            }
        } else {
            let mut stdout = std::io::stdout().lock();
            if let Err(err) =
                crate::commands::watch_save_time::render_daemon_verdict_plain(&mut stdout, response)
            {
                tracing::warn!(error = %err, "failed to render daemon verdict");
            }
        }
    }

    fn send_result(
        &self,
        exit_code: Option<i32>,
        elapsed: std::time::Duration,
        error_detail: Option<String>,
        daemon_notice: Option<anvil_tui::surfaces::watch::DaemonNotice>,
    ) {
        self.send_result_with_assurance(
            exit_code,
            elapsed,
            error_detail,
            daemon_notice,
            None,
            false,
        );
    }

    fn send_result_with_assurance(
        &self,
        exit_code: Option<i32>,
        elapsed: std::time::Duration,
        error_detail: Option<String>,
        daemon_notice: Option<anvil_tui::surfaces::watch::DaemonNotice>,
        assurance_detail: Option<String>,
        assurance_degraded: bool,
    ) {
        let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        if let Some(emitter) = self.json_emitter.as_ref() {
            let result = WatchActionResult {
                action: self.action.clone(),
                exit_code,
                duration_ms,
                error_detail,
                daemon_verdict: None,
            };
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            if let Err(err) = emitter.emit_action_result(timestamp, result)
                && err.kind() != std::io::ErrorKind::BrokenPipe
            {
                tracing::warn!(error = %err, "failed to emit JSON subprocess action result");
            }
            return;
        }

        let Some(sender) = self.sender.as_ref() else {
            return;
        };
        let line = anvil_tui::surfaces::watch::ActionResultLine {
            action: self.action.clone(),
            exit_code,
            duration_ms,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            error_detail,
            daemon_notice,
            assurance_detail,
            assurance_degraded,
        };
        // Cancel-aware send (council finding: shutdown deadlock).
        // SyncSender::send would block if the buffer is full while the
        // TUI loop has stopped draining; if shutdown is in progress that
        // would deadlock the worker on a `worker.join()` that depends on
        // it returning. Poll cancel between try_send attempts so the
        // worker can exit promptly during shutdown.
        loop {
            if self.cancel.load(Ordering::SeqCst) {
                return;
            }
            match sender.try_send(line.clone()) {
                Ok(()) | Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
        }
    }
}

#[cfg(all(test, unix))]
impl ActionDispatcher {
    /// Test-only constructor that overrides the resolved `exe`. Used for
    /// kill-on-shutdown tests that spawn `/bin/sleep` directly without
    /// going through the production `current_exe()` path.
    fn new_with_exe(
        action: String,
        workspace_root: PathBuf,
        json: bool,
        no_tui_arg: bool,
        tui_parent: bool,
        sender: Option<std::sync::mpsc::SyncSender<anvil_tui::surfaces::watch::ActionResultLine>>,
        exe_override: Option<PathBuf>,
    ) -> Self {
        Self(std::sync::Arc::new(DispatcherInner {
            action,
            workspace_root,
            json,
            no_tui_arg,
            tui_parent,
            sender,
            running: AtomicBool::new(false),
            pending: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            pending_paths: std::sync::Mutex::new(std::collections::BTreeSet::new()),
            in_flight: std::sync::Mutex::new(None),
            worker: std::sync::Mutex::new(None),
            // Subprocess-only by default in tests; the daemon-routing tests
            // exercise `WatchSaveTimeClient` directly in `watch_save_time`.
            save_time: None,
            driver_log: None,
            json_emitter: None,
            exe_override,
        }))
    }
}

/// Map a kernel [`anvil_kernel::watch::WatchError`] to actionable,
/// platform-aware start-failure guidance (CIB-175). Returns `None` for
/// non-watcher failures (config parse, missing root, …), which already carry
/// their own descriptive message.
fn watcher_start_guidance(err: &anvil_kernel::watch::WatchError) -> Option<String> {
    use anvil_kernel::watch::WatchError;
    use anvil_kernel::watcher::WatcherError;
    match err {
        WatchError::Watcher(WatcherError::Notify(notify_err)) => {
            Some(anvil_kernel::watcher::failure_guidance(notify_err))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_lines)]
pub fn run(args: &WatchArgs, global: &GlobalArgs) -> Result<()> {
    // DSV-048: driver mode pins the workspace root to the supervisor-supplied
    // canonical worktree (never cwd discovery) and builds the findings log the
    // child owns. A worktree that does not resolve is the one "unrecoverable
    // setup failure" exit the driver contract allows. Canonicalise BEFORE
    // deriving the default log path so the path-hashed log id is identical
    // whether the caller passed the canonical root (the supervisor) or a
    // relative/symlinked spelling (a manual run).
    let workspace_root = match args.worktree.as_ref().filter(|_| args.save_time_driver) {
        Some(worktree) => worktree.canonicalize().with_context(|| {
            format!(
                "save-time driver worktree does not resolve: {}",
                worktree.display()
            )
        })?,
        None => crate::util::workspace_root()?,
    };
    let driver_log = if args.save_time_driver {
        let path = crate::commands::watch_driver::resolve_driver_log_path(&workspace_root)?;
        Some(std::sync::Arc::new(
            crate::commands::watch_driver::DriverLog::new(path),
        ))
    } else {
        None
    };
    // Driver mode always runs the save-time `check` action (`--action`
    // conflicts with `--save-time-driver` at the clap layer).
    let action = if args.save_time_driver {
        Some("check")
    } else {
        resolve_action(args.action.as_deref())?
    };

    // DLIFE-004 (ADR-082): only the `check` watch routes through the save-time
    // daemon, so the daemon lifecycle offer applies only there — and never in
    // driver mode (DSV-048). Resolve it before the watcher starts so the
    // (interactive) offer and any bounded start wait happen up front, not
    // interleaved with file events. The line goes to stderr so the stdout
    // NDJSON / plain event stream stays intact; `--json` suppresses it
    // entirely. Reuse / fallback are non-interactive and side-effect-free; the
    // offer can only appear in an interactive TTY.
    if watch_daemon_offer_applies(action, args.save_time_driver)
        && let Some(line) = ensure_watch_daemon(args.no_daemon, global.json, &workspace_root)
    {
        eprintln!("{line}");
    }

    // LAUNCH-002: --action is now allowed in TUI mode. The dispatcher forces
    // --no-tui on the child and discards child stdio so two Ratatui sessions
    // can't fight over the same alternate-screen.

    // Resolve watch root — if --file is given, scope to that path
    let watch_root = resolve_watch_root(&workspace_root, args.file.as_deref())?;

    // Build include patterns passed to the kernel's WatchPatternFilter.
    //
    // The defaults assume the user wants the broadest reasonable scope
    // unless they opt into a narrower one:
    //   no flags            → empty (let the FileFilter denylist define scope)
    //   --all               → empty (same — FileFilter is the only gate)
    //   --plans only        → DEFAULT_WATCH_PATTERNS (planning docs)
    //   --source only       → SOURCE_PATTERNS (parseable sources)
    //   --plans + --source  → both
    //   --patterns "..."    → use those verbatim
    //
    // --all is checked first so it stays "watch everything" even when
    // combined with the narrower flags — without this, `--all --plans`
    // would silently scope to planning docs.
    //
    // Previously the no-flag and bare --plans cases both sent
    // DEFAULT_WATCH_PATTERNS, which silently restricted `anvil watch`
    // to planning docs and dropped every source-file event before it
    // ever reached the policy engine.
    let patterns: Vec<String> = if args.all {
        Vec::new()
    } else if let Some(ref p) = args.patterns {
        p.split(',').map(|s| s.trim().to_string()).collect()
    } else if args.source && args.plans {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .chain(SOURCE_PATTERNS.iter())
            .map(ToString::to_string)
            .collect()
    } else if args.source {
        SOURCE_PATTERNS.iter().map(ToString::to_string).collect()
    } else if args.plans {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect()
    } else {
        Vec::new()
    };

    // Exclude globs are applied by the kernel's WatchPatternFilter — they
    // no longer extend the internal FileFilter denylist. The internal
    // denylist (node_modules, .git, target, …) stays in place via
    // build_filter(); user-supplied excludes are passed through as
    // WatchConfig.exclude_patterns below.
    let exclude: Vec<String> = args.exclude.as_ref().map_or_else(Vec::new, |s| {
        s.split(',').map(|s| s.trim().to_string()).collect()
    });
    warn_on_bare_exclude_patterns(&exclude, global.json);
    // When the user has supplied an explicit scoped pattern (--patterns,
    // --source, --plans), the FileFilter must not additionally enforce
    // its hardcoded ts/js extension gate — that would silently drop events
    // for file types the user explicitly asked us to watch.
    //
    // `--all` is deliberately *not* in this set: it widens scope to "watch
    // everything that passes the denylist", but the kernel's parser still
    // only handles TS/JS today, so forwarding non-JS files to it produces
    // UnsupportedLanguage errors and noisy snapshots. Keep the extension
    // gate enabled for `--all` until the kernel supports more languages.
    let user_supplied_patterns = args.patterns.is_some() || args.source || args.plans;
    let filter = build_filter(user_supplied_patterns);

    let output_mode = if args.save_time_driver {
        // DSV-048: headless regardless of TTY state — the driver must never
        // open a TUI or print banners even when spawned from a terminal.
        WatchOutputMode::Driver
    } else {
        watch_output_mode(
            global,
            std::io::stdin().is_terminal(),
            std::io::stdout().is_terminal(),
        )
    };
    let warmup_paths = if should_use_warmup_cache(&watch_root, &workspace_root, &patterns) {
        load_watch_warmup_cache(&workspace_root).unwrap_or(None)
    } else {
        None
    };
    let warmup_path_bufs: Vec<PathBuf> = warmup_paths
        .as_ref()
        .map(|paths| paths.iter().map(PathBuf::from).collect())
        .unwrap_or_default();

    print_active_scope(&patterns, &exclude, output_mode);
    print_warmup_cache_status(warmup_paths.as_ref(), output_mode);
    print_tui_startup_message(output_mode);

    let arch_config_path = workspace_root.join(".anvil").join("architecture.yaml");
    let arch_config = if arch_config_path.exists() {
        Some(arch_config_path)
    } else {
        None
    };

    let watcher_config = anvil_kernel::watcher::WatcherConfig {
        root: watch_root.clone(),
        debounce_window: std::time::Duration::from_millis(args.debounce.unwrap_or(300)),
        filter: Some(filter),
        ..Default::default()
    };

    let watch_config = anvil_kernel::watch::WatchConfig {
        root: watch_root.clone(),
        architecture_config: arch_config.clone(),
        watcher: watcher_config,
        include_patterns: patterns,
        exclude_patterns: exclude.clone(),
        warmup_paths: warmup_path_bufs,
    };

    let (event_tx, event_rx) = mpsc::channel();

    let handle = anvil_kernel::watch::run_watch(&watch_config, event_tx).map_err(|err| {
        // CIB-175: surface an actionable, platform-aware guidance line for a
        // watcher-start failure instead of the raw
        // "starting kernel watcher: notify error: …" chain. The original
        // `WatchError` stays as the anyhow source, so `--json`/debug keep the
        // full chain (`{err:#}` renders "<guidance>: watcher error: notify
        // error: …").
        match watcher_start_guidance(&err) {
            Some(guidance) => anyhow::Error::new(err).context(guidance),
            None => anyhow::Error::new(err).context("starting kernel watcher"),
        }
    })?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_flag = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        shutdown_flag.store(true, Ordering::SeqCst);
    })
    .context("setting Ctrl-C handler")?;

    let non_tui = !matches!(output_mode, WatchOutputMode::Tui);
    let json_emitter = output_mode
        .writes_json_events()
        .then(WatchJsonEmitter::stdout);

    // LAUNCH-002: in TUI mode, the dispatcher emits ActionResultLine records
    // through a sync_channel(1) into the watch loop. The bound is intentional
    // back-pressure: if the TUI hasn't drained the most recent result, the
    // worker blocks on `send` until it does, naturally rate-limiting reruns.
    let (action_tx, action_rx) = if action.is_some() && !non_tui {
        let (tx, rx) = mpsc::sync_channel::<anvil_tui::surfaces::watch::ActionResultLine>(1);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let dispatcher = action.map(|act| {
        ActionDispatcher::new(
            act.to_string(),
            workspace_root.clone(),
            global.json,
            global.no_tui,
            !non_tui,
            action_tx,
            driver_log,
            args.no_daemon,
            json_emitter.clone(),
        )
    });
    let mut background_json_error = None;

    if non_tui {
        warn_if_tui_fell_back(output_mode);
        let mut snapshot_count: u64 = 0;

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            if let Some(error) = json_emitter
                .as_ref()
                .and_then(WatchJsonEmitter::terminal_error)
            {
                background_json_error = Some(error);
                break;
            }
            match event_rx.recv_timeout(std::time::Duration::from_millis(250)) {
                Ok(event) => {
                    if matches!(output_mode, WatchOutputMode::Driver) {
                        // DSV-048: no per-event stdout in driver mode — the
                        // findings log carries verdicts, and event noise must
                        // stay off the supervisor's crash-capture channel.
                    } else if global.json {
                        let emitted = continue_after_watch_json_emit(
                            json_emitter
                                .as_ref()
                                .expect("JSON runs create the public-stream emitter")
                                .emit_kernel_event(&event),
                        )
                        .context("emitting watch JSON event")?;
                        if !emitted {
                            break;
                        }
                    } else {
                        print_event_plain(&event);
                    }

                    // Dispatch action on snapshot events (skip initial scan).
                    // Concurrency / rerun guarding lives in ActionDispatcher.
                    if matches!(event.event_type, anvil_kernel_types::EventType::Snapshot) {
                        snapshot_count += 1;
                        if snapshot_count == 1
                            && !global.json
                            && matches!(output_mode, WatchOutputMode::Plain { .. })
                            && let anvil_kernel_types::EventPayload::Snapshot {
                                files_watched, ..
                            } = &event.payload
                        {
                            println!(
                                "[watching] ready — {files_watched} files watched (initial scan complete)"
                            );
                        }
                        if let Some(d) = dispatcher.as_ref()
                            && snapshot_count > 1
                        {
                            d.on_snapshot(snapshot_changed_path(&event));
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    } else {
        // DISTRIB-002: probe before launching the TUI loop so the hint
        // is rendered on the first frame. Network call is gated by the
        // existing 3s timeout in `fetch_latest_version_quiet`; the
        // 24h rate-limit keeps repeated `anvil watch` invocations from
        // re-printing the same line every restart. Opt-out via
        // `ANVIL_DISABLE_UPDATE_HINT=1`.
        //
        // `include_advisories: true` so the hint names any
        // `Security-Advisory: GHSA-…` tag attached to the running
        // version per the DISTRIB-002 spec. Long-running watch
        // sessions keep the first-frame value — the hint does not
        // re-probe mid-session.
        let update_hint = if std::env::var_os("ANVIL_DISABLE_UPDATE_HINT").is_none() {
            crate::commands::version::compute_update_hint(true)
        } else {
            None
        };
        let state =
            anvil_tui::surfaces::watch::WatchState::new(anvil_tui::surfaces::watch::WatchData {
                status: anvil_tui::surfaces::watch::WatchStatus::Idle,
                queue: std::collections::VecDeque::new(),
                history: Vec::new(),
                stats: anvil_tui::surfaces::watch::WatchStats {
                    total_runs: 0,
                    pass_rate: 0.0,
                    avg_duration_ms: 0,
                    files_watched: 0,
                },
                warmup: None,
                last_action: None,
                update_hint,
                insights_hint: if std::env::var_os("ANVIL_DISABLE_UPDATE_HINT").is_none() {
                    // Reuse the env gate for simplicity; first-week is even
                    // quieter than update hint. In practice the 14d + weekly
                    // gates already keep it rare.
                    use chrono::Utc;
                    crate::insights::first_week_hint::first_week_insights_hint(
                        &workspace_root,
                        Utc::now(),
                        crate::install_root::project_writes_gated(),
                    )
                } else {
                    None
                },
                daemon_fallback_notice: None,
            });
        let link = action_rx
            .as_ref()
            .zip(dispatcher.as_ref())
            .map(|(rx, d)| WatchActionLink {
                action_rx: rx,
                dispatcher: d,
            });
        crate::tui::run_watch(state, &event_rx, link.as_ref(), Some(&shutdown))?;
    }

    // Tear down in a deterministic order:
    //   1. Drop the action receiver first so any worker mid-send sees a
    //      Disconnected error immediately rather than blocking on a buffer
    //      that nobody is draining (defence in depth — the dispatcher's
    //      send_result is also cancel-aware).
    //   2. Drop the dispatcher: cancel + kill in-flight child + join worker.
    //   3. Stop the kernel watcher.
    drop(action_rx);
    drop(dispatcher);
    if background_json_error.is_none() {
        background_json_error = json_emitter
            .as_ref()
            .and_then(WatchJsonEmitter::terminal_error);
    }
    handle.stop().context("stopping watcher")?;
    if let Some(error) = background_json_error {
        continue_after_watch_json_emit(Err(error)).context("emitting watch JSON output")?;
    }
    Ok(())
}

fn should_use_warmup_cache(
    watch_root: &std::path::Path,
    workspace_root: &std::path::Path,
    include_patterns: &[String],
) -> bool {
    watch_root == workspace_root && include_patterns.is_empty()
}

/// RLB-007: extract the per-save changed-path dispatch hint from an event.
///
/// Returns the absolute path of the file whose save produced a snapshot, or
/// `None` for non-snapshot events and for snapshots without a single changed
/// file (the initial scan and delete-driven snapshots). The CLI watch loop
/// feeds this to `ActionDispatcher::on_snapshot` so `anvil check` scopes to
/// the changed file instead of re-walking the whole repo.
///
/// Called from both watch loops: the non-TUI loop in this module and the TUI
/// loop in `crate::tui::run_watch`.
pub(crate) fn snapshot_changed_path(event: &anvil_kernel_types::EngineEvent) -> Option<&str> {
    match &event.payload {
        anvil_kernel_types::EventPayload::Snapshot { changed_path, .. } => changed_path.as_deref(),
        _ => None,
    }
}

fn print_event_plain(event: &anvil_kernel_types::EngineEvent) {
    use anvil_kernel_types::{EventPayload, EventType};

    // ASCII-only labels so per-event watch output renders cleanly on
    // Windows terminals and CI log captures that lack full Unicode. The
    // banner and bare-exclude warning were previously fixed; this is the
    // hot path during a demo and was missed in that round.
    let prefix = match event.event_type {
        EventType::Progress => "[progress]",
        EventType::Snapshot => "[snapshot]",
        EventType::Violation => "[violation]",
        EventType::Error => "[error]",
    };

    match &event.payload {
        EventPayload::Progress {
            phase,
            current,
            total,
        } => {
            println!("{prefix} {phase}: {current}/{total}");
        }
        EventPayload::Snapshot {
            node_count,
            edge_count,
            files_watched,
            ..
        } => {
            println!(
                "{prefix} Snapshot: {node_count} nodes, {edge_count} edges, {files_watched} files"
            );
        }
        EventPayload::Violation {
            policy_id,
            file,
            message,
            ..
        } => {
            println!("{prefix} [{policy_id}] {file}: {message}");
        }
        EventPayload::Error(err) => {
            eprintln!("{prefix} Error: {}", err.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{
        EngineEvent, EngineId, ErrorCode, ErrorPayload, EventPayload, EventType,
        WATCH_EVENT_SCHEMA_VERSION,
    };
    use clap::Parser;
    use serde_json::Value;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: WatchArgs,
    }

    // ----- DLIFE-004 daemon lifecycle planner + render tests -----

    /// The default decision context: no opt-out, plain (non-JSON) output, a
    /// non-interactive shell, no live daemon. Tests flip exactly the fields
    /// they exercise via struct-update syntax, so each case reads as a delta.
    fn base_ctx() -> WatchDaemonContext {
        WatchDaemonContext {
            routing_disabled: false,
            no_daemon: false,
            json: false,
            interactive: false,
            daemon_live: false,
            platform_can_spawn: true,
        }
    }

    /// The hard opt-out (`ANVIL_WATCH_DAEMON=0`) falls back without any
    /// daemon contact — regardless of liveness, flag, or interactivity — so
    /// the planner never reuses or offers under it.
    #[test]
    fn watch_plan_routing_disabled_always_falls_back_opt_out() {
        for no_daemon in [false, true] {
            for json in [false, true] {
                for interactive in [false, true] {
                    for daemon_live in [false, true] {
                        assert_eq!(
                            watch_daemon_plan(WatchDaemonContext {
                                routing_disabled: true,
                                no_daemon,
                                json,
                                interactive,
                                daemon_live,
                                ..base_ctx()
                            }),
                            WatchDaemonPlan::FallBack(WatchFallbackReason::RoutingDisabled),
                            "ANVIL_WATCH_DAEMON=0 must always fall back \
                             (no_daemon={no_daemon}, json={json}, \
                             interactive={interactive}, daemon_live={daemon_live})",
                        );
                    }
                }
            }
        }
    }

    /// `--no-daemon` is an explicit no-contact choice even when a daemon is
    /// live, so the planner must choose the scoped fallback in every UI mode.
    #[test]
    fn watch_plan_no_daemon_wins_over_live_daemon() {
        for json in [false, true] {
            for interactive in [false, true] {
                assert_eq!(
                    watch_daemon_plan(WatchDaemonContext {
                        no_daemon: true,
                        json,
                        interactive,
                        daemon_live: true,
                        ..base_ctx()
                    }),
                    WatchDaemonPlan::FallBack(WatchFallbackReason::NoDaemonFlag),
                    "--no-daemon must prevent reuse (json={json}, interactive={interactive})",
                );
            }
        }
    }

    /// The dispatcher construction gate must honour `--no-daemon` before any
    /// transport resolution or liveness probe, even under forced-on routing.
    #[test]
    fn no_daemon_prevents_save_time_client_construction() {
        use crate::commands::watch_save_time::DaemonRoutingMode;

        assert!(!save_time_client_applies(
            "check",
            DaemonRoutingMode::ForcedOn,
            true,
        ));
    }

    /// No-contact opt-outs short-circuit before the lifecycle liveness probe,
    /// so neither the environment nor the CLI flag opens a daemon transport.
    #[test]
    fn daemon_opt_outs_skip_the_liveness_probe() {
        assert!(!watch_daemon_probe_applies(true, false, false));
        assert!(!watch_daemon_probe_applies(false, true, false));
        assert!(watch_daemon_probe_applies(false, false, false));
    }

    /// `--no-daemon` with nothing live suppresses the offer and falls back,
    /// with copy distinct from both the hard env opt-out and the
    /// non-interactive context.
    #[test]
    fn watch_plan_no_daemon_falls_back_opt_out_when_nothing_live() {
        assert_eq!(
            watch_daemon_plan(WatchDaemonContext {
                no_daemon: true,
                interactive: true,
                ..base_ctx()
            }),
            WatchDaemonPlan::FallBack(WatchFallbackReason::NoDaemonFlag),
        );
    }

    /// `--json` never prompts even on an interactive TTY: it folds into the
    /// deterministic non-interactive fallback so the run stays parse-safe.
    #[test]
    fn watch_plan_json_never_prompts() {
        assert_eq!(
            watch_daemon_plan(WatchDaemonContext {
                json: true,
                interactive: true,
                ..base_ctx()
            }),
            WatchDaemonPlan::FallBack(WatchFallbackReason::NonInteractive),
        );
    }

    /// A non-interactive context (CI / hook / piped) with nothing live falls
    /// back deterministically — never prompting or hanging automation. This is
    /// also the `ANVIL_WATCH_DAEMON=1` (`ForcedOn`) headless case: `ForcedOn`
    /// is not `routing_disabled`, so it lands here, not at an opt-out.
    #[test]
    fn watch_plan_non_interactive_falls_back_without_prompting() {
        assert_eq!(
            watch_daemon_plan(base_ctx()),
            WatchDaemonPlan::FallBack(WatchFallbackReason::NonInteractive),
        );
    }

    /// Interactive TTY, no live daemon, no opt-out → the offer is made. This
    /// also pins the `ANVIL_WATCH_DAEMON=1` (`ForcedOn`) interactive case:
    /// `ForcedOn` maps to `routing_disabled = false`, so a forced-routing user
    /// whose daemon is absent is offered a start rather than silently
    /// dropped to the scoped fallback.
    #[test]
    fn watch_plan_interactive_no_live_daemon_prompts() {
        assert_eq!(
            watch_daemon_plan(WatchDaemonContext {
                interactive: true,
                ..base_ctx()
            }),
            WatchDaemonPlan::Prompt,
        );
    }

    /// On a platform that cannot background-launch a daemon, an interactive
    /// session that would otherwise prompt instead falls back as
    /// platform-unsupported — asking for consent it cannot honour is pointless
    /// ceremony (Copilot finding). The opt-outs still take precedence.
    #[test]
    fn watch_plan_no_spawn_platform_skips_the_prompt() {
        assert_eq!(
            watch_daemon_plan(WatchDaemonContext {
                interactive: true,
                platform_can_spawn: false,
                ..base_ctx()
            }),
            WatchDaemonPlan::FallBack(WatchFallbackReason::PlatformUnsupported),
        );
        // A live daemon is still reused even where we cannot spawn one.
        assert_eq!(
            watch_daemon_plan(WatchDaemonContext {
                interactive: true,
                platform_can_spawn: false,
                daemon_live: true,
                ..base_ctx()
            }),
            WatchDaemonPlan::ReuseLive,
        );
    }

    /// Started: the line reports the action, defers attestation to the MCP
    /// (re)connect (mirroring `anvil start`), and never claims protection
    /// state (the activation diagnostic owns it).
    #[test]
    fn watch_line_for_started_is_honest() {
        use anvil_intercept::ensure::EnsureOutcome;
        let line = render_watch_daemon_line(&EnsureOutcome::Started);
        assert!(line.contains("started"), "got: {line}");
        assert!(
            !line.to_lowercase().contains("protect"),
            "the lifecycle line must not claim protection: {line}",
        );
        // Must not over-claim active daemon-backed validation before the
        // editor reconnects — the attestation is deferred.
        assert!(
            line.contains("connects") || line.contains("once"),
            "started copy must defer attestation to MCP reconnect: {line}",
        );
    }

    /// Reuse names the running daemon and makes no protection claim.
    #[test]
    fn watch_line_for_reused_names_the_running_daemon() {
        use anvil_intercept::ensure::EnsureOutcome;
        let line = render_watch_daemon_line(&EnsureOutcome::Reused);
        assert!(line.contains("reusing"), "got: {line}");
        assert!(!line.to_lowercase().contains("protect"), "got: {line}");
    }

    /// Failure surfaces the recovery hint verbatim and names the preserved
    /// scoped fallback so the user is not stranded.
    #[test]
    fn watch_line_for_failure_surfaces_recovery_and_fallback() {
        use anvil_intercept::ensure::EnsureOutcome;
        let recovery = "the daemon did not become ready within 12s. See the daemon log at /run/x.";
        let line = render_watch_daemon_line(&EnsureOutcome::Failed {
            recovery: recovery.to_owned(),
        });
        assert!(line.contains(recovery), "got: {line}");
        assert!(line.contains("scoped fallback"), "got: {line}");
    }

    /// Platform-unsupported (the only `NoStart` reason the `MaySpawn` ensure
    /// path yields) must not blame the opt-out flag and must preserve the
    /// scoped fallback.
    #[test]
    fn watch_line_for_platform_unsupported_does_not_blame_opt_out() {
        use anvil_intercept::ensure::{EnsureOutcome, NoStartReason};
        let unsupported = render_watch_daemon_line(&EnsureOutcome::NoStart {
            reason: NoStartReason::PlatformUnsupported,
        });
        assert!(unsupported.contains("platform"), "got: {unsupported}");
        assert!(
            !unsupported.contains("--no-daemon"),
            "platform-unsupported must not blame the opt-out flag: {unsupported}",
        );
        assert!(
            unsupported.contains("scoped fallback"),
            "got: {unsupported}"
        );
    }

    /// The three deterministic fallback reasons render distinct, honest copy:
    /// each names ONLY the surface the user actually engaged (Council: don't
    /// blame a flag/env they didn't set), and all preserve the scoped fallback.
    #[test]
    fn watch_fallback_lines_name_only_the_engaged_surface() {
        let routing = render_watch_fallback_line(WatchFallbackReason::RoutingDisabled);
        let flag = render_watch_fallback_line(WatchFallbackReason::NoDaemonFlag);
        let non_interactive = render_watch_fallback_line(WatchFallbackReason::NonInteractive);
        let platform = render_watch_fallback_line(WatchFallbackReason::PlatformUnsupported);

        // Hard env opt-out: names the env var, not the flag.
        assert!(routing.contains("ANVIL_WATCH_DAEMON=0"), "got: {routing}");
        assert!(!routing.contains("--no-daemon"), "got: {routing}");

        // Soft flag opt-out: names the flag, not the env var.
        assert!(flag.contains("--no-daemon"), "got: {flag}");
        assert!(!flag.contains("ANVIL_WATCH_DAEMON"), "got: {flag}");

        // Context fallback: blames neither opt-out surface, explains why, and
        // points at both recovery commands.
        assert!(
            non_interactive.contains("non-interactive"),
            "got: {non_interactive}"
        );
        assert!(
            !non_interactive.contains("--no-daemon"),
            "got: {non_interactive}"
        );
        assert!(
            non_interactive.contains("anvil start"),
            "got: {non_interactive}"
        );

        // Platform-unsupported: blames the platform, not an opt-out surface.
        assert!(platform.contains("platform"), "got: {platform}");
        assert!(!platform.contains("--no-daemon"), "got: {platform}");

        // All preserve the scoped fallback.
        for line in [&routing, &flag, &non_interactive, &platform] {
            assert!(line.contains("scoped fallback"), "got: {line}");
        }
    }

    /// The declined line is distinct from a deliberate opt-out (the user was
    /// asked) and points at how to start one later, preserving the fallback.
    #[test]
    fn watch_declined_line_is_distinct_and_preserves_fallback() {
        let line = watch_daemon_declined_line();
        assert!(line.contains("declined"), "got: {line}");
        assert!(line.contains("scoped fallback"), "got: {line}");
        assert!(line.contains("anvil start"), "got: {line}");
        assert!(!line.contains("--no-daemon"), "got: {line}");
    }

    /// `--no-daemon` parses onto `WatchArgs` and the `anvil start --watch`
    /// fallback constructor disables the offer (architecture-only watch must
    /// never engage the save-time daemon lifecycle).
    #[test]
    fn no_daemon_flag_parses_and_fallback_constructor_opts_out() {
        let parsed = Wrapper::parse_from(["watch", "--no-daemon"]);
        assert!(parsed.inner.no_daemon);
        let default = Wrapper::parse_from(["watch"]);
        assert!(!default.inner.no_daemon);
        assert!(WatchArgs::fallback_for_repo().no_daemon);
    }

    /// Assert the v1 envelope invariants the wire contract guarantees for
    /// every variant: pinned `schema_version`, `seq`/`timestamp` propagated
    /// from the kernel event, the documented lower-case `event_type`
    /// discriminator, no internal `engine` field on the wire, and a
    /// single-line serialisation.
    fn assert_envelope_invariants(event: &EngineEvent, expected_type: &str) -> Value {
        let envelope = WatchEventEnvelope::from_engine_event(event);
        let line = serde_json::to_string(&envelope).expect("serialise");
        let value: Value = serde_json::from_str(&line).expect("parse own output");

        assert_eq!(value["schema_version"], WATCH_EVENT_SCHEMA_VERSION);
        assert_eq!(value["seq"], event.seq);
        assert_eq!(value["timestamp"], event.timestamp);
        assert_eq!(value["event_type"], expected_type);
        assert!(
            value.get("engine").is_none(),
            "engine field must not appear on wire envelope: {value}"
        );
        assert!(
            !line.contains('\n'),
            "envelope must serialise as a single NDJSON line: {line}"
        );
        value
    }

    /// WOUT-002 validation: an `EngineEvent` from the kernel must serialise
    /// through the v1 wire envelope as documented in
    /// `docs/specs/watch-output-contract.md`. The previous behaviour was
    /// `WatchEvent { timestamp, event_type: "Debug(...)", detail: "Debug(...)" }`
    /// — a structured payload replaces the debug-formatted `detail` string,
    /// and `schema_version` + `seq` are now required fields on the wire.
    /// Covers one of each variant so a future regression that flattens
    /// payloads incorrectly is caught by the most common shapes the
    /// consumer guide documents.
    #[test]
    fn watch_event_serialises_to_json() {
        let progress = EngineEvent {
            event_type: EventType::Progress,
            seq: 0,
            timestamp: "2026-05-14T10:21:30Z".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Progress {
                phase: "initial-scan".into(),
                current: 12,
                total: 100,
            },
        };
        let v = assert_envelope_invariants(&progress, "progress");
        assert_eq!(v["payload"]["phase"], "initial-scan");
        assert_eq!(v["payload"]["current"], 12);
        assert_eq!(v["payload"]["total"], 100);

        let snapshot = EngineEvent {
            event_type: EventType::Snapshot,
            seq: 3,
            timestamp: "2026-05-14T10:21:30Z".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Snapshot {
                node_count: 312,
                edge_count: 845,
                files_watched: 64,
                changed_path: None,
            },
        };
        let v = assert_envelope_invariants(&snapshot, "snapshot");
        assert_eq!(v["payload"]["node_count"], 312);
        assert_eq!(v["payload"]["edge_count"], 845);
        assert_eq!(v["payload"]["files_watched"], 64);

        let violation = EngineEvent {
            event_type: EventType::Violation,
            seq: 7,
            timestamp: "2026-05-14T10:21:31Z".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Violation {
                policy_id: "no-circular-deps".into(),
                file: "src/main.ts".into(),
                symbol: "App".into(),
                message: "Circular dependency detected".into(),
            },
        };
        let v = assert_envelope_invariants(&violation, "violation");
        assert_eq!(v["payload"]["policy_id"], "no-circular-deps");
        assert_eq!(v["payload"]["file"], "src/main.ts");
        assert_eq!(v["payload"]["symbol"], "App");
        assert_eq!(v["payload"]["message"], "Circular dependency detected");

        let error = EngineEvent {
            event_type: EventType::Error,
            seq: 9,
            timestamp: "2026-05-14T10:21:31Z".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Error(ErrorPayload {
                code: ErrorCode::ParseError,
                file: Some("src/broken.ts".into()),
                message: "Unexpected token".into(),
                recoverable: true,
            }),
        };
        let v = assert_envelope_invariants(&error, "error");
        assert_eq!(v["payload"]["code"], "ParseError");
        assert_eq!(v["payload"]["file"], "src/broken.ts");
        assert_eq!(v["payload"]["message"], "Unexpected token");
        assert_eq!(v["payload"]["recoverable"], true);
    }

    #[test]
    fn args_parses_empty() {
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_source() {
        let w = Wrapper::try_parse_from(["test", "--source"]).unwrap();
        assert!(w.inner.source);
    }

    #[test]
    fn args_parses_all() {
        let w = Wrapper::try_parse_from(["test", "--all"]).unwrap();
        assert!(w.inner.all);
    }

    #[test]
    fn args_parses_patterns() {
        let w = Wrapper::try_parse_from(["test", "--patterns", "**/*.ts,**/*.tsx"]).unwrap();
        assert!(w.inner.patterns.is_some());
    }

    #[test]
    fn args_parses_file_and_action() {
        let w = Wrapper::try_parse_from([
            "test",
            "--file",
            "src/",
            "--action",
            "gate",
            "--exclude",
            "vendor/",
        ])
        .unwrap();
        assert_eq!(w.inner.file.as_deref(), Some("src/"));
        assert_eq!(w.inner.action.as_deref(), Some("gate"));
        assert_eq!(w.inner.exclude.as_deref(), Some("vendor/"));
    }

    #[test]
    fn resolve_action_defaults_to_check_when_absent() {
        // GH #1913: bare `anvil watch` now runs the code-quality scanners by
        // default instead of silently watching architecture only.
        assert_eq!(resolve_action(None).unwrap(), Some("check"));
    }

    #[test]
    fn resolve_action_accepts_gate_and_check() {
        assert_eq!(resolve_action(Some("gate")).unwrap(), Some("gate"));
        assert_eq!(resolve_action(Some("check")).unwrap(), Some("check"));
    }

    #[test]
    fn resolve_action_none_opts_out_to_architecture_only() {
        // The explicit opt-out restores the pre-#1913 architecture-only watch.
        assert_eq!(resolve_action(Some("none")).unwrap(), None);
    }

    #[test]
    fn resolve_action_rejects_unknown() {
        assert!(resolve_action(Some("deploy")).is_err());
    }

    // --- UJ-006: the CLI itself teaches the daemon story ---

    #[test]
    fn watch_long_help_names_daemon_start_and_routing_values() {
        use clap::CommandFactory;
        let mut cli = crate::Cli::command();
        let watch = cli
            .find_subcommand_mut("watch")
            .expect("watch subcommand exists");
        let help = watch.render_long_help().to_string();
        assert!(
            help.contains("daemon"),
            "long help must name the daemon, got:\n{help}",
        );
        assert!(
            help.contains("anvil start"),
            "long help must name the `anvil start` prerequisite, got:\n{help}",
        );
        assert!(
            help.contains("ANVIL_WATCH_DAEMON"),
            "long help must name the routing env var, got:\n{help}",
        );
        for value in ["unset", "ANVIL_WATCH_DAEMON=0", "ANVIL_WATCH_DAEMON=1"] {
            assert!(
                help.contains(value),
                "long help must document the {value} routing value, got:\n{help}",
            );
        }
    }

    #[test]
    fn fallback_advisory_names_anvil_start() {
        let line =
            fallback_advisory_line(&crate::commands::watch_save_time::daemon_absent_assurance());
        assert!(
            line.contains("daemon unavailable"),
            "advisory must say the daemon is unavailable, got: {line}",
        );
        assert!(
            line.contains("anvil start"),
            "advisory must name `anvil start` so the surface teaches recovery, got: {line}",
        );
        assert!(
            line.contains("unavailable{daemon-absent}"),
            "advisory keeps the assurance label, got: {line}",
        );
        assert!(
            line.is_ascii(),
            "advisory is ASCII-only per the watch banner policy, got: {line}",
        );
    }

    #[test]
    fn tui_fallback_warning_sets_daemon_notice() {
        let dispatcher = ActionDispatcher::new(
            "check".to_string(),
            PathBuf::from("/tmp"),
            false,
            false,
            true,
            None,
            None,
            false,
            None,
        );
        let mut notice = None;
        dispatcher.0.handle_daemon_fallback_warning(
            &crate::commands::watch_save_time::daemon_absent_assurance(),
            true,
            &mut notice,
        );

        assert_eq!(
            notice,
            Some(anvil_tui::surfaces::watch::DaemonNotice::Fallback {
                message: "daemon: unavailable -- scoped fallback".to_string(),
            })
        );
    }

    #[test]
    fn daemon_verdict_populates_tui_assurance_and_clears_fallback() {
        use anvil_intercept_proto::protocol::{
            AssuranceState, CheckFamily, Coverage, EvaluatedPath, ValidatePathsResponse,
            WorkspaceAssurance,
        };
        let response = ValidatePathsResponse {
            diagnostics: Vec::new(),
            evaluated: vec![EvaluatedPath {
                path: "src/a.ts".into(),
                content_hash: Some("hash".into()),
            }],
            workspace_assurance: WorkspaceAssurance {
                state: AssuranceState::Clean,
                reason: None,
                generation: 1,
                last_full_scan: None,
                scan_coverage: None,
            },
            coverage: Coverage::Certified,
            check_families: vec![CheckFamily::Antipattern],
        };
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let dispatcher = ActionDispatcher::new(
            "check".to_string(),
            PathBuf::from("/tmp"),
            false,
            false,
            true,
            Some(tx),
            None,
            false,
            None,
        );

        dispatcher
            .0
            .report_daemon_verdict(&response, std::time::Duration::from_millis(1));
        let line = rx.try_recv().expect("daemon verdict line");
        assert_eq!(
            line.daemon_notice,
            Some(anvil_tui::surfaces::watch::DaemonNotice::ClearFallback)
        );
        assert_eq!(
            line.assurance_detail.as_deref(),
            Some("antipattern-only certified")
        );
        assert!(!line.assurance_degraded);

        let mut stale = response;
        stale.coverage = Coverage::Partial;
        stale.workspace_assurance.state = AssuranceState::Stale;
        stale.workspace_assurance.reason =
            Some(anvil_intercept_proto::protocol::StaleReason::CrossFileResolutionNeeded);
        dispatcher
            .0
            .report_daemon_verdict(&stale, std::time::Duration::from_millis(1));
        let line = rx.try_recv().expect("degraded daemon verdict line");
        assert_eq!(
            line.assurance_detail.as_deref(),
            Some("antipattern-only partial; stale{cross-file-resolution-needed}")
        );
        assert!(
            line.assurance_degraded,
            "partial/stale assurance must dominate an exit-zero result"
        );
    }

    #[test]
    fn json_emitter_allocates_sequence_in_stdout_order_under_concurrency() {
        #[derive(Clone, Default)]
        struct SharedWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

        impl std::io::Write for SharedWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                recover(self.0.lock()).extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let writer = SharedWriter::default();
        let emitter = WatchJsonEmitter::new(Box::new(writer.clone()));
        let mut threads = Vec::new();
        for index in 0..16 {
            let emitter = emitter.clone();
            threads.push(std::thread::spawn(move || {
                emitter
                    .emit_action_result(
                        format!("t{index}"),
                        anvil_kernel_types::watch_event::WatchActionResult {
                            action: "check".to_string(),
                            exit_code: Some(0),
                            duration_ms: index,
                            error_detail: None,
                            daemon_verdict: None,
                        },
                    )
                    .expect("emit action result");
            }));
        }
        for thread in threads {
            thread.join().expect("emitter thread");
        }

        let output = String::from_utf8(recover(writer.0.lock()).clone()).expect("UTF-8 NDJSON");
        let sequences = output
            .lines()
            .map(|line| {
                serde_json::from_str::<WatchEventEnvelope>(line)
                    .expect("parse emitted envelope")
                    .seq
            })
            .collect::<Vec<_>>();
        assert_eq!(sequences, (0..16).collect::<Vec<_>>());
    }

    #[test]
    fn json_emitter_preserves_terminal_error_detail_and_shared_stop_semantics() {
        struct ErrorWriter(std::io::ErrorKind, &'static str);

        impl std::io::Write for ErrorWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(self.0, self.1))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        for (kind, message, clean_stop) in [
            (std::io::ErrorKind::BrokenPipe, "consumer closed", true),
            (std::io::ErrorKind::Other, "output device failed", false),
        ] {
            let emitter = WatchJsonEmitter::new(Box::new(ErrorWriter(kind, message)));
            let err = emitter
                .emit_action_result(
                    "2026-08-05T00:00:00Z",
                    WatchActionResult {
                        action: "check".to_string(),
                        exit_code: Some(1),
                        duration_ms: 1,
                        error_detail: None,
                        daemon_verdict: None,
                    },
                )
                .expect_err("failing consumer must reject action_result");
            assert_eq!(err.kind(), kind);
            let terminal = emitter
                .terminal_error()
                .expect("worker output failure must reach the watch loop");
            assert_eq!(terminal.kind(), kind);
            assert_eq!(terminal.to_string(), message);
            let decision = continue_after_watch_json_emit(Err(terminal));
            if clean_stop {
                assert!(!decision.expect("BrokenPipe is a clean stop"));
            } else {
                assert_eq!(
                    decision
                        .expect_err("other output failures must propagate")
                        .kind(),
                    kind
                );
            }
        }
    }

    #[test]
    fn json_broken_pipe_stops_watch_cleanly_but_other_io_errors_propagate() {
        let broken_pipe = continue_after_watch_json_emit(Err(std::io::Error::from(
            std::io::ErrorKind::BrokenPipe,
        )))
        .expect("broken pipe is a normal consumer shutdown");
        assert!(!broken_pipe);

        let other = continue_after_watch_json_emit(Err(std::io::Error::other("write failed")))
            .expect_err("non-broken-pipe errors must propagate");
        assert_eq!(other.kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn daemon_action_result_preserves_exact_wire_verdict() {
        use anvil_intercept_proto::protocol::{
            AssuranceState, CheckFamily, Coverage, EvaluatedPath, StaleReason,
            ValidatePathsResponse, WorkspaceAssurance,
        };

        let response = ValidatePathsResponse {
            diagnostics: Vec::new(),
            evaluated: vec![EvaluatedPath {
                path: "src/a.ts".into(),
                content_hash: Some("hash".into()),
            }],
            workspace_assurance: WorkspaceAssurance {
                state: AssuranceState::Stale,
                reason: Some(StaleReason::CrossFileResolutionNeeded),
                generation: 7,
                last_full_scan: None,
                scan_coverage: None,
            },
            coverage: Coverage::Partial,
            check_families: vec![CheckFamily::Antipattern],
        };

        let result =
            daemon_watch_action_result("check", std::time::Duration::from_millis(42), &response);
        let verdict = result.daemon_verdict.expect("daemon verdict");
        assert_eq!(verdict.assurance_state, "stale");
        assert_eq!(
            verdict.assurance_reason.as_deref(),
            Some("cross-file-resolution-needed")
        );
        assert_eq!(verdict.coverage, "partial");
        assert_eq!(verdict.check_families, vec!["antipattern"]);
        assert_eq!(verdict.finding_count, 0);
        assert_eq!(verdict.diagnostics, response.diagnostics);
    }

    #[test]
    fn start_watch_fallback_stays_architecture_only() {
        // GH #1913: bare `anvil watch` runs `check`, but the `anvil start
        // --watch` fallback must remain architecture-only (it claims
        // `state: watching` and must not silently spawn per-save checks).
        let args = WatchArgs::fallback_for_repo();
        assert_eq!(resolve_action(args.action.as_deref()).unwrap(), None);
    }

    #[test]
    fn output_mode_uses_tui_only_when_stdin_and_stdout_are_terminal() {
        let global = GlobalArgs::default();

        assert_eq!(watch_output_mode(&global, true, true), WatchOutputMode::Tui);
        assert_eq!(
            watch_output_mode(&global, true, false),
            WatchOutputMode::Plain {
                reason: PlainWatchReason::StdoutNotTerminal,
            }
        );
        assert_eq!(
            watch_output_mode(&global, false, true),
            WatchOutputMode::Plain {
                reason: PlainWatchReason::StdinNotTerminal,
            }
        );
    }

    #[test]
    fn output_mode_json_and_no_tui_override_terminal_state() {
        let json = GlobalArgs {
            json: true,
            ..GlobalArgs::default()
        };
        assert_eq!(watch_output_mode(&json, true, true), WatchOutputMode::Json);

        let no_tui = GlobalArgs {
            no_tui: true,
            ..GlobalArgs::default()
        };
        assert_eq!(
            watch_output_mode(&no_tui, true, true),
            WatchOutputMode::Plain {
                reason: PlainWatchReason::NoTuiFlag,
            }
        );
    }

    #[test]
    fn resolve_watch_root_uses_workspace_for_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(
            resolve_watch_root(tmp.path(), None).unwrap(),
            tmp.path()
                .canonicalize()
                .unwrap_or_else(|_| tmp.path().to_path_buf())
        );
    }

    #[test]
    fn warmup_cache_only_applies_to_broad_workspace_scope() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();

        assert!(should_use_warmup_cache(root, root, &[]));
        assert!(!should_use_warmup_cache(&root.join("src"), root, &[]));
        assert!(!should_use_warmup_cache(
            root,
            root,
            &["src/**/*.ts".to_string()]
        ));
    }

    #[test]
    fn resolve_watch_root_joins_relative() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let result = resolve_watch_root(tmp.path(), Some("src")).unwrap();
        assert_eq!(result, src_dir.canonicalize().unwrap());
    }

    #[test]
    fn resolve_watch_root_file_uses_parent() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        let result = resolve_watch_root(tmp.path(), Some("main.rs")).unwrap();
        assert_eq!(result, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn resolve_watch_root_rejects_path_traversal() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_watch_root(tmp.path(), Some("../../etc"));
        assert!(result.is_err());
    }

    // The previous tests that exercised --exclude extending the internal
    // FileFilter denylist were removed in LAUNCH-001: --exclude is now a
    // user-glob path, applied by the kernel's WatchPatternFilter, and no
    // longer touches the internal denylist. Coverage moved to
    // crates/anvil-kernel/src/watcher/pattern.rs (unit) and
    // crates/anvil-kernel/tests/watch_pattern_filter.rs (integration).

    #[test]
    fn resolve_watch_root_rejects_nonexistent_traversal() {
        // Even when the target doesn't exist on disk, the ancestor-based
        // canonicalisation should still catch .. traversal
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_watch_root(tmp.path(), Some("../nonexistent-dir"));
        assert!(result.is_err());
    }

    #[test]
    fn normalise_path_via_ancestors_success_non_existent_target() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        std::fs::create_dir_all(base.join("a/b")).unwrap();
        // Path traverses up from a/b then into non-existent "c"
        let input = base.join("a/b/../../c");
        let result = normalise_path_via_ancestors(&input);
        let expected = base.join("c");
        assert_eq!(result, expected);
    }

    #[test]
    fn build_action_command_sets_correct_args() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        // `gate` self-scopes (git status) so it needs no file flag.
        let cmd = build_action_command(&exe, "gate", &[], &ws, false, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(args, vec![std::ffi::OsStr::new("gate")]);

        // `check` MUST carry a file scope (`--all`) or it bails with
        // "No files specified" and never scans (GH #1913 / council F-001).
        // `--all` (not `--changed`) so untracked/new files and non-git repos
        // are still scanned (Copilot review on #1933).
        let cmd = build_action_command(&exe, "check", &[], &ws, true, true, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(
            args,
            vec![
                std::ffi::OsStr::new("check"),
                std::ffi::OsStr::new("--all"),
                std::ffi::OsStr::new("--json"),
                std::ffi::OsStr::new("--no-tui"),
            ]
        );
    }

    #[test]
    fn build_action_command_scopes_check_to_all_even_plain() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");
        let cmd = build_action_command(&exe, "check", &[], &ws, false, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(
            args,
            vec![std::ffi::OsStr::new("check"), std::ffi::OsStr::new("--all")],
            "bare check dispatch must include --all so it scans (incl. untracked)"
        );
    }

    // --- RLB-007: per-save check is scoped to changed paths (GH #2156) ---

    #[test]
    fn watch_action_scope_check_uses_changed_paths_not_all() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");
        let paths = vec![
            "/project/src/a.ts".to_string(),
            "/project/src/b.ts".to_string(),
        ];
        let cmd = build_action_command(&exe, "check", &paths, &ws, false, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(
            args,
            vec![
                std::ffi::OsStr::new("check"),
                // `--` end-of-options guard, then the changed files positionally.
                std::ffi::OsStr::new("--"),
                std::ffi::OsStr::new("/project/src/a.ts"),
                std::ffi::OsStr::new("/project/src/b.ts"),
            ],
            "scoped check must pass the changed files positionally (after --), not --all"
        );
        assert!(
            !args.iter().any(|a| *a == std::ffi::OsStr::new("--all")),
            "scoped check must NOT re-walk the whole repo with --all, got {args:?}"
        );
    }

    #[test]
    fn watch_fallback_is_scoped_never_all() {
        // DSV-007 Task 12: a daemon-absent fallback must run a scoped check over
        // exactly the changed paths and never re-walk the repo with `--all`.
        use crate::commands::watch_save_time::{
            SaveTimeClientError, SaveTimeDecision, SaveTimeTransport, WatchSaveTimeClient,
        };
        use anvil_intercept_proto::protocol::{ChangeDescriptor, ValidatePathsResponse};

        struct AbsentTransport;
        impl SaveTimeTransport for AbsentTransport {
            fn validate_paths(
                &self,
                _root: &std::path::Path,
                _paths: &[ChangeDescriptor],
            ) -> Result<ValidatePathsResponse, SaveTimeClientError> {
                Err(SaveTimeClientError::Unavailable)
            }
            fn request_full_scan(
                &self,
                _root: &std::path::Path,
            ) -> Result<(), SaveTimeClientError> {
                Ok(())
            }
            fn workspace_status(
                &self,
                _root: &std::path::Path,
            ) -> Result<anvil_intercept_proto::protocol::WorkspaceAssurance, SaveTimeClientError>
            {
                Err(SaveTimeClientError::Unavailable)
            }
        }

        let mut client =
            WatchSaveTimeClient::new(Box::new(AbsentTransport), PathBuf::from("/project"));
        let changed = vec![
            "/project/src/a.ts".to_string(),
            "/project/src/b.ts".to_string(),
        ];
        let SaveTimeDecision::FellBack { scoped_paths, .. } = client.validate(changed.clone())
        else {
            panic!("an absent daemon must fall back");
        };
        assert_eq!(
            scoped_paths, changed,
            "fallback must scope to exactly the changed paths",
        );

        // The dispatcher feeds `scoped_paths` to the existing scoped builder;
        // non-empty paths guarantee no `--all`.
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");
        let cmd = build_action_command(&exe, "check", &scoped_paths, &ws, false, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(
            !args.iter().any(|a| *a == std::ffi::OsStr::new("--all")),
            "daemon-absent fallback must never use --all, got {args:?}",
        );
        assert!(
            args.iter()
                .any(|a| *a == std::ffi::OsStr::new("/project/src/a.ts")),
            "fallback must scope to the changed paths, got {args:?}",
        );
    }

    #[test]
    fn watch_action_scope_flags_precede_path_separator() {
        // The `--` end-of-options guard must come AFTER --json/--no-tui,
        // otherwise those flags would be parsed as positional file paths.
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");
        let paths = vec!["/project/src/a.ts".to_string()];
        let cmd = build_action_command(&exe, "check", &paths, &ws, true, true, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(
            args,
            vec![
                std::ffi::OsStr::new("check"),
                std::ffi::OsStr::new("--json"),
                std::ffi::OsStr::new("--no-tui"),
                std::ffi::OsStr::new("--"),
                std::ffi::OsStr::new("/project/src/a.ts"),
            ]
        );
    }

    #[test]
    fn watch_action_scope_check_empty_paths_falls_back_to_all() {
        // Delete-driven / initial dispatch: no changed file to scope to, so a
        // full walk stays the safe default (a delete can break imports
        // elsewhere; #1913 still requires untracked/new files are covered).
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");
        let cmd = build_action_command(&exe, "check", &[], &ws, false, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(
            args,
            vec![std::ffi::OsStr::new("check"), std::ffi::OsStr::new("--all")]
        );
    }

    #[test]
    fn watch_action_scope_gate_ignores_changed_paths() {
        // `gate` self-scopes via git status, so changed paths must not leak
        // onto its argv (and it must never carry --all).
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");
        let paths = vec!["/project/src/a.ts".to_string()];
        let cmd = build_action_command(&exe, "gate", &paths, &ws, false, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(args, vec![std::ffi::OsStr::new("gate")]);
    }

    #[test]
    fn watch_action_scope_extracts_changed_path_from_event() {
        use anvil_kernel_types::{EngineEvent, EngineId, EventPayload, EventType};

        let snap = |changed: Option<&str>| EngineEvent {
            event_type: EventType::Snapshot,
            seq: 1,
            timestamp: "t".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Snapshot {
                node_count: 1,
                edge_count: 0,
                files_watched: 1,
                changed_path: changed.map(str::to_string),
            },
        };
        assert_eq!(
            snapshot_changed_path(&snap(Some("/p/x.ts"))),
            Some("/p/x.ts")
        );
        // Delete / initial snapshot → no path to scope to.
        assert_eq!(snapshot_changed_path(&snap(None)), None);
        // Non-snapshot events never carry a scope hint.
        let progress = EngineEvent {
            event_type: EventType::Progress,
            seq: 2,
            timestamp: "t".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Progress {
                phase: "p".into(),
                current: 0,
                total: 1,
            },
        };
        assert_eq!(snapshot_changed_path(&progress), None);
    }

    /// While a scan is in flight, further saves accumulate (and dedup) in
    /// `pending_paths` so the coalesced rerun scopes to every file that
    /// changed meanwhile — none is dropped. Deterministic and subprocess-free:
    /// pre-marking `running` drives `on_snapshot` down its in-flight branch
    /// (record + set pending, no new worker) without spawning a child.
    #[test]
    fn watch_action_scope_accumulates_paths_while_in_flight() {
        let dispatcher = ActionDispatcher::new(
            "check".to_string(),
            PathBuf::from("/project"),
            false,
            false,
            false,
            None,
            None,
            false,
            None,
        );
        // Simulate a scan already running so on_snapshot records paths for the
        // coalesced rerun instead of spawning a worker.
        dispatcher.0.running.store(true, Ordering::SeqCst);

        dispatcher.on_snapshot(Some("/project/src/b.ts"));
        dispatcher.on_snapshot(Some("/project/src/b.ts")); // duplicate save
        dispatcher.on_snapshot(Some("/project/src/a.ts"));
        dispatcher.on_snapshot(None); // a delete contributes no path

        let pending: Vec<String> = recover(dispatcher.0.pending_paths.lock())
            .iter()
            .cloned()
            .collect();
        // BTreeSet → deduped and deterministically ordered.
        assert_eq!(
            pending,
            vec![
                "/project/src/a.ts".to_string(),
                "/project/src/b.ts".to_string(),
            ],
            "in-flight saves must accumulate + dedup for the coalesced rerun"
        );
        assert!(
            dispatcher.0.pending.load(Ordering::SeqCst),
            "an in-flight save must set the pending bit so the worker reruns"
        );

        // No worker was ever spawned; release the simulated running flag so
        // Drop's shutdown() is a clean no-op.
        dispatcher.0.running.store(false, Ordering::SeqCst);
    }

    #[test]
    fn build_action_command_sets_cwd() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/my/project");
        let cmd = build_action_command(&exe, "gate", &[], &ws, false, false, false);
        assert_eq!(
            cmd.get_current_dir(),
            Some(std::path::Path::new("/my/project"))
        );
    }

    // --- LAUNCH-002: --no-tui propagation in TUI parent context ---

    #[test]
    fn tui_parent_forces_no_tui_on_child_even_without_parent_flag() {
        // The foot-gun the original guard was hiding: with the parent in TUI
        // mode and no `--no-tui` flag set, a naive guard-drop would let the
        // child enter its own Ratatui alt-screen and fight the parent.
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        let cmd = build_action_command(
            &exe,
            "gate",
            &[],
            &ws,
            false,
            /* no_tui */ false,
            /* tui_parent */ true,
        );
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(
            args.iter().any(|a| *a == std::ffi::OsStr::new("--no-tui")),
            "child must receive --no-tui when parent is in TUI mode, got {args:?}"
        );
    }

    #[test]
    fn tui_parent_does_not_duplicate_no_tui_when_parent_flag_also_set() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        let cmd = build_action_command(
            &exe,
            "gate",
            &[],
            &ws,
            false,
            /* no_tui */ true,
            /* tui_parent */ true,
        );
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        let count = args
            .iter()
            .filter(|a| **a == std::ffi::OsStr::new("--no-tui"))
            .count();
        assert_eq!(
            count, 1,
            "--no-tui should appear exactly once, got {args:?}"
        );
    }

    #[test]
    fn non_tui_parent_does_not_force_no_tui() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        let cmd = build_action_command(
            &exe,
            "gate",
            &[],
            &ws,
            false,
            /* no_tui */ false,
            /* tui_parent */ false,
        );
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(
            !args.iter().any(|a| *a == std::ffi::OsStr::new("--no-tui")),
            "non-TUI parent without explicit --no-tui must not force it on child, got {args:?}"
        );
    }

    // --- LAUNCH-002: ActionDispatcher shutdown ---

    /// Spawns a real `/bin/sleep 30` child via the dispatcher's exe override,
    /// then calls `shutdown()` and asserts the worker joins promptly. Closes
    /// the pre-existing leak where Ctrl-C orphaned the dispatch worker.
    /// Unix-only because the test depends on `/bin/sleep`; Windows lacks an
    /// equivalent at a stable path.
    #[cfg(unix)]
    #[test]
    fn shutdown_kills_in_flight_child_and_joins_worker() {
        let dispatcher = ActionDispatcher::new_with_exe(
            "30".to_string(), // sleep 30 seconds
            PathBuf::from("/tmp"),
            false,
            false,
            false,
            None,
            Some(PathBuf::from("/bin/sleep")),
        );

        dispatcher.on_snapshot(None);

        // Wait briefly for the worker to spawn /bin/sleep and park it in
        // the in_flight slot. 250 ms is generous; the child usually appears
        // within a few ms.
        let parked = std::time::Instant::now();
        loop {
            if dispatcher
                .0
                .in_flight
                .lock()
                .ok()
                .is_some_and(|g| g.is_some())
            {
                break;
            }
            assert!(
                parked.elapsed() <= std::time::Duration::from_millis(500),
                "child did not park in in_flight slot within 500 ms"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let shutdown_started = std::time::Instant::now();
        dispatcher.shutdown();
        let shutdown_took = shutdown_started.elapsed();

        // /bin/sleep 30 would not have exited naturally. If shutdown
        // returned, the child was killed and the worker joined. The kill
        // path includes a 50 ms poll grace; allow up to 1 s for slow CI.
        assert!(
            shutdown_took < std::time::Duration::from_secs(1),
            "shutdown took {shutdown_took:?}; expected < 1 s — \
             worker did not join promptly, child may have leaked"
        );

        // No child remains in the slot.
        assert!(
            dispatcher
                .0
                .in_flight
                .lock()
                .ok()
                .is_some_and(|g| g.is_none()),
            "in_flight slot should be empty after shutdown"
        );

        // Falsifiability: the worker handle slot must be empty after
        // shutdown. If a future refactor removed the join() call, the
        // handle would still occupy this slot. Council finding:
        // adversarial M2.
        assert!(
            dispatcher.0.worker.lock().ok().is_some_and(|g| g.is_none()),
            "worker slot should be empty after shutdown — handle was not joined"
        );

        // Idempotent: a second shutdown is a no-op.
        dispatcher.shutdown();
    }

    /// **#1279 review: cause-specific `error_detail`.**
    ///
    /// `WaitOutcome::to_send_args` is the single source of truth for
    /// the `(exit_code, error_detail)` pair `run_one_action` ships to
    /// `send_result`. Asserting it directly catches a future refactor
    /// that restored the pre-#1279 "spawn failed" string for
    /// cancellation outcomes — without spawning child processes,
    /// without racing the worker thread, without nextest pinning.
    ///
    /// History: the previous version of this test drove `on_snapshot`
    /// → spawn `/bin/sleep` → poll the `in_flight` slot for up to 30 s
    /// waiting for the worker to park, then called `shutdown` and
    /// inspected the channel. It flaked under contended CI even after
    /// two timeout ratchets (500 ms → 5 s → 30 s) and a single-thread
    /// nextest group pin. The flake is structural: `send_result` is
    /// cancel-aware (drops the result when `cancel` is set) so once
    /// shutdown fires, the cancellation result almost always vanishes
    /// before the channel sees it. The polling barrier was an attempt
    /// to widen the window where the worker raced through
    /// `send_result` first; it could never be made deterministic.
    /// Routing the assertion through the pure helper closes the gap.
    #[test]
    fn cancellation_emits_cancelled_error_detail_not_spawn_failed() {
        let (exit_code, detail) = WaitOutcome::Cancelled.to_send_args();
        assert!(exit_code.is_none(), "cancellation has no exit code");
        let detail = detail.expect("cancellation must populate error_detail");
        assert!(
            !detail.contains("spawn failed"),
            "cancellation must not be reported as spawn failure, got: {detail:?}"
        );
        assert_eq!(detail, "cancelled");
    }

    /// Mirror coverage for the non-cancel outcomes so a refactor that
    /// changed *all* outcomes' strings can't slip past.
    #[test]
    fn wait_failed_outcome_uses_wait_failed_prefix_not_spawn_failed() {
        let (exit_code, detail) =
            WaitOutcome::WaitFailed("Permission denied".to_string()).to_send_args();
        assert!(exit_code.is_none());
        let detail = detail.expect("wait failure must populate error_detail");
        assert!(
            !detail.contains("spawn failed"),
            "wait failure must not be reported as spawn failure, got: {detail:?}"
        );
        assert_eq!(detail, "wait failed: Permission denied");
    }

    #[test]
    fn exited_outcome_carries_exit_code_and_no_error_detail() {
        let (exit_code, detail) = WaitOutcome::Exited(Some(0)).to_send_args();
        assert_eq!(exit_code, Some(0));
        assert!(detail.is_none(), "successful exit has no error_detail");

        let (exit_code, detail) = WaitOutcome::Exited(Some(2)).to_send_args();
        assert_eq!(exit_code, Some(2));
        assert!(detail.is_none(), "non-zero exit has no error_detail");

        let (exit_code, detail) = WaitOutcome::Exited(None).to_send_args();
        assert!(
            exit_code.is_none(),
            "signal-killed children report no exit code"
        );
        assert!(
            detail.is_none(),
            "signal-killed children have no error_detail"
        );
    }

    /// **Deadlock regression test (council finding: adversarial + ops).**
    ///
    /// Reproduces the shutdown deadlock that hung the watch process on
    /// Ctrl-C: with the channel buffer pre-filled and the receiver alive
    /// (mirroring how `run()` declared `action_rx` outside the dispatcher),
    /// a worker that produces a result faster than the TUI drains will
    /// block on `sender.send()`. The shutdown path then deadlocks on
    /// `worker.join()`. The fix (cancel-aware `try_send` in `send_result`)
    /// breaks the loop within ~20 ms of `cancel` being set.
    #[cfg(unix)]
    #[test]
    fn shutdown_does_not_deadlock_when_channel_buffer_full() {
        let (tx, _rx) =
            std::sync::mpsc::sync_channel::<anvil_tui::surfaces::watch::ActionResultLine>(1);

        // Pre-fill the buffer so the worker's send blocks immediately.
        tx.try_send(anvil_tui::surfaces::watch::ActionResultLine {
            action: "preload".to_string(),
            exit_code: Some(0),
            duration_ms: 0,
            timestamp: "00:00:00".to_string(),
            error_detail: None,
            daemon_notice: None,
            assurance_detail: None,
            assurance_degraded: false,
        })
        .expect("preload send");

        let dispatcher = ActionDispatcher::new_with_exe(
            "0".to_string(), // sleep 0 — child exits immediately, worker
            // immediately calls send_result and blocks on
            // the full buffer.
            PathBuf::from("/tmp"),
            false,
            false,
            true, // tui_parent — sender path is exercised
            Some(tx),
            Some(PathBuf::from("/bin/sleep")),
        );

        dispatcher.on_snapshot(None);

        // Give the worker time to spawn the child, complete it, and reach
        // the blocking send. 200 ms is plenty for /bin/sleep 0.
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Shutdown must return promptly even though the worker is parked
        // in send_result. Pre-fix this hung indefinitely; the cancel-aware
        // try_send loop polls cancel every 20 ms.
        let started = std::time::Instant::now();
        dispatcher.shutdown();
        let took = started.elapsed();

        assert!(
            took < std::time::Duration::from_secs(2),
            "shutdown took {took:?}; expected < 2 s — \
             worker is likely blocked on a non-cancel-aware sender.send()"
        );

        // _rx is still alive at this point (mirrors run()'s drop order
        // before `drop(action_rx)` is called explicitly). The worker
        // should have exited regardless.
    }

    /// `Drop` must call `shutdown` so a panic or early-return path doesn't
    /// leak the worker.
    #[cfg(unix)]
    #[test]
    fn drop_invokes_shutdown() {
        let parked = {
            let dispatcher = ActionDispatcher::new_with_exe(
                "30".to_string(),
                PathBuf::from("/tmp"),
                false,
                false,
                false,
                None,
                Some(PathBuf::from("/bin/sleep")),
            );
            dispatcher.on_snapshot(None);

            // Wait for the child to park.
            let waited = std::time::Instant::now();
            loop {
                if dispatcher
                    .0
                    .in_flight
                    .lock()
                    .ok()
                    .is_some_and(|g| g.is_some())
                {
                    break;
                }
                assert!(
                    waited.elapsed() <= std::time::Duration::from_millis(500),
                    "child did not park within 500 ms"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            std::time::Instant::now()
            // dispatcher dropped here; Drop -> shutdown -> kill+join
        };

        assert!(
            parked.elapsed() < std::time::Duration::from_secs(1),
            "Drop should kill child and join worker within 1 s"
        );
    }

    // The concurrency guard (action_running/action_pending AtomicBool pair) is
    // tested indirectly via the watch integration flow. Direct unit testing is
    // impractical because the guard is coupled to thread spawning and the
    // dispatch loop in run(). The previous test here only exercised
    // AtomicBool::swap semantics in isolation, which is tautological.

    // --- normalise_path_via_ancestors ---

    #[test]
    fn normalise_path_resolves_dotdot_traversal() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let sub = base.join("a").join("b");
        std::fs::create_dir_all(&sub).unwrap();

        // a/b/../../c should resolve to <tmp>/c
        let input = sub.join("..").join("..").join("c");
        let result = normalise_path_via_ancestors(&input);
        assert_eq!(result, base.join("c"));
    }

    #[test]
    fn normalise_path_handles_absolute_existing_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let file = base.join("exists.txt");
        std::fs::write(&file, "").unwrap();

        let result = normalise_path_via_ancestors(&file);
        assert_eq!(result, file.canonicalize().unwrap());
    }

    #[test]
    fn normalise_path_handles_relative_components() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let sub = base.join("deep");
        std::fs::create_dir(&sub).unwrap();

        // deep/../shallow should resolve to <tmp>/shallow
        let input = sub.join("..").join("shallow");
        let result = normalise_path_via_ancestors(&input);
        assert_eq!(result, base.join("shallow"));
    }

    // --- WOUT-003: JSON-mode stdout discipline ---

    #[test]
    fn watch_output_mode_writes_human_banners_only_for_plain() {
        assert!(
            WatchOutputMode::Plain {
                reason: PlainWatchReason::NoTuiFlag,
            }
            .writes_human_banners()
        );
        assert!(
            WatchOutputMode::Plain {
                reason: PlainWatchReason::StdoutNotTerminal,
            }
            .writes_human_banners()
        );
        assert!(
            WatchOutputMode::Plain {
                reason: PlainWatchReason::StdinNotTerminal,
            }
            .writes_human_banners()
        );
        // JSON suppresses banners — stdout owned by NDJSON. TUI renders
        // them inside the alt-screen.
        assert!(!WatchOutputMode::Json.writes_human_banners());
        assert!(!WatchOutputMode::Tui.writes_human_banners());
    }

    #[test]
    fn warning_channel_for_advisory_routes_to_stderr_when_json_mode() {
        assert_eq!(WarningChannel::for_advisory(true), WarningChannel::Stderr);
        assert_eq!(WarningChannel::for_advisory(false), WarningChannel::Stdout);
    }

    #[test]
    fn format_bare_exclude_warning_includes_pattern_and_corrected_glob() {
        let line = format_bare_exclude_warning("vendor");
        assert!(line.starts_with("[warn] "));
        assert!(line.contains("--exclude vendor"));
        assert!(line.contains("--exclude vendor/**"));
        // ASCII-only — no smart quotes, em-dashes, or emoji on the
        // diagnostic line.
        assert!(line.is_ascii(), "warning line must be ASCII: {line}");
    }

    #[test]
    fn child_stdio_policy_in_json_mode_discards_child_stdout_and_inherits_stderr() {
        // The whole point of WOUT-003: child stdout MUST be null so its
        // bytes cannot interleave with the parent's NDJSON event stream.
        let (out, err) = child_stdio_policy(/* json */ true, /* tui_parent */ false);
        assert_eq!(
            out,
            ChildStdio::Null,
            "JSON parent must discard child stdout"
        );
        assert_eq!(
            err,
            ChildStdio::Inherit,
            "JSON parent inherits child stderr (diagnostic channel)"
        );
    }

    #[test]
    fn child_stdio_policy_in_tui_mode_discards_both_streams() {
        let (out, err) = child_stdio_policy(/* json */ false, /* tui_parent */ true);
        assert_eq!(out, ChildStdio::Null);
        assert_eq!(err, ChildStdio::Null);

        // TUI parent wins over JSON: if both flags were set, child
        // stdio is still fully discarded so it cannot fight the
        // alt-screen.
        let (out, err) = child_stdio_policy(/* json */ true, /* tui_parent */ true);
        assert_eq!(out, ChildStdio::Null);
        assert_eq!(err, ChildStdio::Null);
    }

    #[test]
    fn child_stdio_policy_in_plain_mode_inherits_both_streams() {
        let (out, err) = child_stdio_policy(/* json */ false, /* tui_parent */ false);
        assert_eq!(out, ChildStdio::Inherit);
        assert_eq!(err, ChildStdio::Inherit);
    }

    // --- bare-exclude warning heuristic (M4) ---

    #[test]
    fn bare_directory_name_is_detected() {
        assert!(is_likely_bare_directory_name("vendor"));
        assert!(is_likely_bare_directory_name("tmp"));
        assert!(is_likely_bare_directory_name("node_modules"));
    }

    #[test]
    fn glob_patterns_are_not_treated_as_bare_names() {
        assert!(!is_likely_bare_directory_name("vendor/**"));
        assert!(!is_likely_bare_directory_name("**/*.test.ts"));
        assert!(!is_likely_bare_directory_name("src/foo"));
        assert!(!is_likely_bare_directory_name("*.log"));
        assert!(!is_likely_bare_directory_name("file?.ts"));
        assert!(!is_likely_bare_directory_name("[abc]/lib"));
        assert!(!is_likely_bare_directory_name("{a,b}/lib"));
        assert!(!is_likely_bare_directory_name("!skip"));
    }

    #[test]
    fn empty_string_does_not_trigger_bare_warning() {
        assert!(!is_likely_bare_directory_name(""));
    }

    // --- build_filter ---

    #[test]
    fn build_filter_returns_default_denylist() {
        let filter = build_filter(false);
        // Default filter still ignores standard dirs like node_modules
        assert!(filter.should_ignore(std::path::Path::new("node_modules/x.ts")));
        // But not arbitrary dirs
        assert!(!filter.should_ignore(std::path::Path::new("src/main.rs")));
    }

    #[test]
    fn build_filter_with_user_patterns_bypasses_extension_gate() {
        // The demo-killer regression: --patterns '**/*.rs' must not be
        // dropped by the FileFilter's hardcoded ts/js list before the
        // user's WatchPatternFilter ever sees the event.
        let filter = build_filter(true);
        assert!(filter.should_process(std::path::Path::new("src/main.rs")));
        assert!(filter.should_process(std::path::Path::new("lib.py")));
        // Denylist still applies.
        assert!(!filter.should_process(std::path::Path::new("node_modules/foo.rs")));
    }

    // --- Pattern selection logic ---
    //
    // The helper mirrors the include-pattern computation in `run()`.
    // Keep them in sync — a test-local duplicate that drifts from the
    // production logic was the gap that let the M2 default-pattern bug
    // ship in the original LAUNCH-001 commit.

    fn collect_patterns(args: &[&str]) -> Vec<String> {
        let w = Wrapper::try_parse_from(args).unwrap();
        if w.inner.all {
            Vec::new()
        } else if let Some(ref p) = w.inner.patterns {
            p.split(',').map(|s| s.trim().to_string()).collect()
        } else if w.inner.source && w.inner.plans {
            DEFAULT_WATCH_PATTERNS
                .iter()
                .chain(SOURCE_PATTERNS.iter())
                .map(ToString::to_string)
                .collect()
        } else if w.inner.source {
            SOURCE_PATTERNS.iter().map(ToString::to_string).collect()
        } else if w.inner.plans {
            DEFAULT_WATCH_PATTERNS
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            Vec::new()
        }
    }

    #[test]
    fn pattern_selection_source_picks_source_patterns() {
        let patterns = collect_patterns(&["test", "--source"]);
        let expected: Vec<String> = SOURCE_PATTERNS.iter().map(ToString::to_string).collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn source_patterns_only_include_kernel_parseable_extensions() {
        assert!(
            SOURCE_PATTERNS.iter().all(|pattern| {
                std::path::Path::new(pattern)
                    .extension()
                    .is_some_and(|ext| {
                        matches!(
                            ext.to_string_lossy().as_ref(),
                            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs"
                        )
                    })
            }),
            "--source patterns must not forward unsupported languages into the kernel parser"
        );
    }

    #[test]
    fn pattern_selection_all_returns_empty_for_broadest_scope() {
        // --all delegates scope to the FileFilter denylist; the kernel
        // pattern filter is intentionally noop.
        let patterns = collect_patterns(&["test", "--all"]);
        assert!(
            patterns.is_empty(),
            "--all should send empty include_patterns, got {patterns:?}"
        );
    }

    #[test]
    fn pattern_selection_source_and_plans_picks_both() {
        let patterns = collect_patterns(&["test", "--source", "--plans"]);
        let expected: Vec<String> = DEFAULT_WATCH_PATTERNS
            .iter()
            .chain(SOURCE_PATTERNS.iter())
            .map(ToString::to_string)
            .collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_default_returns_empty_for_broadest_scope() {
        // No flags = let the FileFilter denylist define scope; do not
        // silently restrict to plan files (the M2 regression).
        let patterns = collect_patterns(&["test"]);
        assert!(
            patterns.is_empty(),
            "no flags should send empty include_patterns, got {patterns:?}"
        );
    }

    #[test]
    fn pattern_selection_plans_alone_picks_plan_patterns() {
        // Bare --plans is now opt-in narrowing, not the default.
        let patterns = collect_patterns(&["test", "--plans"]);
        let expected: Vec<String> = DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_all_overrides_narrower_flags() {
        // --all is "watch everything" — combining it with --plans,
        // --source, or --patterns must not silently narrow scope.
        for combo in [
            vec!["test", "--all", "--plans"],
            vec!["test", "--all", "--source"],
            vec!["test", "--all", "--plans", "--source"],
            vec!["test", "--all", "--patterns", "src/**/*.ts"],
        ] {
            let patterns = collect_patterns(&combo);
            assert!(
                patterns.is_empty(),
                "{combo:?} should keep --all's broad scope, got {patterns:?}"
            );
        }
    }

    // ── CIB-133: gated-root suppression of the first-week nudge ─────────
    //
    // The watch surface resolves the hint via the same canonical
    // `first_week_insights_hint(root, now, project_writes_gated)` call the
    // TUI-state setup uses (watch.rs:~1703). Under a gated project root it
    // must neither emit the nudge nor read-and-record the real project's
    // hint state (DISTRIB-006 / ADR-060).
    #[test]
    fn first_week_hint_suppressed_when_project_writes_gated() {
        use chrono::{Duration, Utc};

        let tmp = tempfile::TempDir::with_prefix("anvil-watch-insights-").unwrap();
        let root = tmp.path().to_path_buf();
        let anvil = root.join("anvil");
        std::fs::create_dir_all(&anvil).unwrap();
        let created =
            (Utc::now() - Duration::days(3)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        std::fs::write(
            anvil.join("project-id"),
            format!("project_uuid: 01999999-0000-0000-0000-000000000133\ncreated_at: {created}\n"),
        )
        .unwrap();

        let hint =
            crate::insights::first_week_hint::first_week_insights_hint(&root, Utc::now(), true);
        assert!(
            hint.is_none(),
            "gated watch must not emit the first-week nudge"
        );
        assert!(
            !root.join(".anvil/insights-hint.json").exists(),
            "gated watch must not write the project hint state"
        );
    }

    // ----- DSV-048 save-time driver mode tests -----

    /// Driver mode never engages the daemon lifecycle (no ensure, no offer,
    /// no spawn): the supervisor that spawned the driver owns the daemon
    /// lifecycle (ADR-101), so `ensure_watch_daemon` must be unreachable.
    /// This predicate is the single gate in `run()` in front of it.
    #[test]
    fn watch_save_time_driver_never_offers_or_ensures_daemon() {
        assert!(watch_daemon_offer_applies(Some("check"), false));
        assert!(!watch_daemon_offer_applies(Some("check"), true));
        assert!(!watch_daemon_offer_applies(Some("gate"), false));
        assert!(!watch_daemon_offer_applies(None, false));
        assert!(!watch_daemon_offer_applies(None, true));
    }

    /// The supervisor argv contract (`anvil watch --save-time-driver
    /// --worktree <PATH>`) is canonical: the flag requires the worktree, the
    /// worktree requires the flag, and scope/action overrides are rejected
    /// loudly rather than silently reinterpreted.
    #[test]
    fn watch_save_time_driver_argv_contract() {
        assert!(
            Wrapper::try_parse_from(["test", "--save-time-driver", "--worktree", "/ws"]).is_ok()
        );
        assert!(
            Wrapper::try_parse_from(["test", "--save-time-driver"]).is_err(),
            "--save-time-driver without --worktree must be rejected"
        );
        assert!(
            Wrapper::try_parse_from(["test", "--worktree", "/ws"]).is_err(),
            "--worktree without --save-time-driver must be rejected"
        );
        for (flag, value) in [
            ("--action", Some("gate")),
            ("--file", Some("src/a.ts")),
            ("--patterns", Some("src/**")),
            ("--exclude", Some("vendor/**")),
            ("--plans", None),
            ("--source", None),
            ("--all", None),
        ] {
            let mut argv = vec!["test", "--save-time-driver", "--worktree", "/ws", flag];
            if let Some(v) = value {
                argv.push(v);
            }
            assert!(
                Wrapper::try_parse_from(argv).is_err(),
                "{flag} must conflict with --save-time-driver"
            );
        }
        // --no-daemon stays accepted: it is redundant in driver mode (the
        // driver never offers) but harmless, and rejecting it would break a
        // supervisor that passes it belt-and-braces.
        assert!(
            Wrapper::try_parse_from([
                "test",
                "--save-time-driver",
                "--worktree",
                "/ws",
                "--no-daemon"
            ])
            .is_ok()
        );
    }

    /// Driver mode is headless by definition — it never opens the TUI and
    /// never writes human banners, regardless of TTY state.
    #[test]
    fn watch_save_time_driver_mode_is_headless() {
        assert!(!WatchOutputMode::Driver.writes_human_banners());
        assert!(!WatchOutputMode::Driver.writes_json_events());
        assert!(
            !matches!(WatchOutputMode::Driver, WatchOutputMode::Tui),
            "driver mode must be non-TUI"
        );
        assert!(WatchOutputMode::Json.writes_json_events());
    }

    /// A daemon verdict in driver mode appends the rendered findings to the
    /// child-owned log — stdout stays silent and the TUI sender is unused.
    #[cfg(unix)]
    #[test]
    fn watch_save_time_driver_verdict_appends_to_log_not_stdout() {
        use anvil_intercept_proto::protocol::{
            AssuranceState, Coverage, EvaluatedPath, ValidatePathsResponse, WorkspaceAssurance,
        };
        use anvil_kernel_types::diagnostics::{
            Category, DiagnosticSource, KnownMode, Location, Mode, Severity,
        };

        let dir = tempfile::tempdir().expect("tempdir");
        let log_path = dir.path().join("wt.log");
        let inner_log = std::sync::Arc::new(crate::commands::watch_driver::DriverLog::new(
            log_path.clone(),
        ));
        let inner = DispatcherInner {
            action: "check".to_string(),
            workspace_root: dir.path().to_path_buf(),
            json: false,
            no_tui_arg: true,
            tui_parent: false,
            sender: None,
            running: AtomicBool::new(false),
            pending: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            pending_paths: std::sync::Mutex::new(std::collections::BTreeSet::new()),
            in_flight: std::sync::Mutex::new(None),
            worker: std::sync::Mutex::new(None),
            save_time: None,
            driver_log: Some(std::sync::Arc::clone(&inner_log)),
            json_emitter: None,
            exe_override: None,
        };

        let response = ValidatePathsResponse {
            diagnostics: vec![anvil_kernel_types::Diagnostic::new(
                "ap-driver-test",
                Severity::Warning,
                "planted driver finding",
                Location {
                    file: "src/a.ts".into(),
                    line: Some(7),
                    column: None,
                    end_line: None,
                    end_column: None,
                },
                Category::Antipattern,
                DiagnosticSource {
                    rule_id: "r".into(),
                    source_module: "m".into(),
                },
                Mode::known(KnownMode::SaveTime),
            )],
            evaluated: vec![EvaluatedPath {
                path: "src/a.ts".into(),
                content_hash: Some("hash".into()),
            }],
            workspace_assurance: WorkspaceAssurance {
                state: AssuranceState::Clean,
                reason: None,
                generation: 1,
                last_full_scan: None,
                scan_coverage: None,
            },
            coverage: Coverage::Certified,
            check_families: vec![anvil_intercept_proto::protocol::CheckFamily::Antipattern],
        };
        inner.report_daemon_verdict(&response, std::time::Duration::from_millis(1));

        let content = std::fs::read_to_string(&log_path).expect("driver log written");
        assert!(
            content.contains("ap-driver-test") && content.contains("src/a.ts:7"),
            "findings render into the child-owned log, got: {content}"
        );
        assert!(
            content.contains("finding(s)"),
            "assurance trailer present: {content}"
        );
    }
}
