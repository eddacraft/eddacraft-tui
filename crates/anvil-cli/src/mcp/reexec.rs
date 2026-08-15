//! Live-heal re-exec for long-lived `anvil mcp serve --stdio` (MCPLH-002).
//!
//! Between JSON-RPC messages, detect skew versus the preferred Anvil binary
//! (first `anvil` on `PATH`, or `ANVIL_MCP_PREFERRED`) and replace this
//! process via `execve`, keeping stdin/stdout/stderr. Unix first; Windows
//! demotes to honest skew reporting.
//!
//! Re-exec is never attempted mid-frame or after partial JSON-RPC stdout.
//! At most one attempt per process (`ANVIL_MCP_REEXECED`). Kill-switch:
//! `ANVIL_MCP_NO_REEXEC`.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde_json::Value;

use crate::activation::mcp_client::PREFERRED_MCP_COMMAND;

/// Set on the replacement image so a still-skewed child does not loop.
pub(crate) const REEXECED_ENV: &str = "ANVIL_MCP_REEXECED";
/// Disables re-exec; the process stays on this image and reports skew.
pub(crate) const NO_REEXEC_ENV: &str = "ANVIL_MCP_NO_REEXEC";
/// Explicit preferred executable (spec §6 override). Not `current_exe()`.
pub(crate) const PREFERRED_ENV: &str = "ANVIL_MCP_PREFERRED";

/// Process-local anti-loop if `exec` returns (crate forbids `set_var`).
static REEXEC_ATTEMPTED: AtomicBool = AtomicBool::new(false);
/// Whether this process has observed the install-scoped generation.
/// First observation is a baseline, not a poke — otherwise a replacement
/// image with `LAST_SEEN=0` would treat any existing generation file as
/// a new bump and bypass `ANVIL_MCP_REEXECED`.
static GENERATION_SEEN: AtomicBool = AtomicBool::new(false);
/// Last consumed install-scoped refresh generation (MCPLH-003).
static LAST_SEEN_GENERATION: AtomicU64 = AtomicU64::new(0);

const TRIGGER_METHODS: &[&str] = &["initialize", "tools/list", "tools/call"];

/// Framing position relative to the current JSON-RPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FramePhase {
    /// A complete frame has been read; no response bytes have been written.
    BetweenMessages,
    /// Inside a handler or after a partial stdout write.
    #[allow(dead_code)] // constructed by unit tests that prove the between-message gate
    MidHandler,
}

/// Outcome of a re-exec check. [`ReexecDecision::Reexec`] is Unix-only in
/// production; tests may synthesise it on any platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReexecDecision {
    Reexec { preferred: PathBuf },
    Stay { reason: StayReason, skewed: bool },
}

/// Why this process stayed on its current image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StayReason {
    NotATrigger,
    MidFrame,
    KillSwitch,
    AlreadyReexeced,
    NotSkewed,
    PreferredUnresolved,
    PlatformDemoted,
    Pinned,
}

impl StayReason {
    /// Operator/agent hint when skew remains and we cannot (or will not)
    /// recycle. Never leads with “restart your editor”.
    #[must_use]
    pub(crate) fn recovery_hint(self) -> Option<&'static str> {
        match self {
            Self::KillSwitch => Some(
                "This MCP process is not the preferred anvil binary. \
                 Re-exec is disabled (ANVIL_MCP_NO_REEXEC). \
                 Retry a tool call after unsetting that variable, \
                 or reconnect MCP for this client.",
            ),
            Self::AlreadyReexeced => Some(
                "Anvil tried to recycle this MCP process in place. \
                 The session still runs a stale image. \
                 Retry a tool call after the preferred anvil is first on PATH, \
                 or reconnect MCP for this client.",
            ),
            Self::PlatformDemoted => Some(
                "This MCP process is not the preferred anvil binary. \
                 In-place recycle is not available on this platform. \
                 Retry a tool call after launching via PATH anvil, \
                 or reconnect MCP for this client.",
            ),
            Self::PreferredUnresolved => Some(
                "This MCP process could not resolve the preferred anvil binary. \
                 Retry a tool call after anvil is first on PATH, \
                 or reconnect MCP for this client.",
            ),
            Self::Pinned => Some(
                "MCP auto-heal is pinned, so this process will not recycle \
                 to the preferred anvil binary. Run `anvil mcp unpin` \
                 (or unset ANVIL_MCP_PIN) and retry a tool call, \
                 or reconnect MCP for this client.",
            ),
            Self::NotATrigger | Self::MidFrame | Self::NotSkewed => None,
        }
    }
}

/// When the probe runs relative to the stdio loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckKind {
    /// Before the first stdin read (new attach).
    Startup,
    /// After a complete JSON-RPC frame was parsed.
    RpcMethod,
}

/// Policy gates evaluated before identity skew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReexecGate {
    Allowed,
    KillSwitch,
    AlreadyAttempted,
    PlatformDemoted,
    Pinned,
}

/// Injectable inputs for [`decide`]. Production builds these from the
/// process environment via [`probe_from_process`].
#[derive(Debug, Clone)]
pub(crate) struct ReexecProbe {
    pub method: Option<String>,
    pub phase: FramePhase,
    pub check: CheckKind,
    pub gate: ReexecGate,
    pub current_exe: Option<PathBuf>,
    pub preferred: Option<PathBuf>,
    /// True when the install-scoped refresh generation is newer than last seen.
    pub generation_bumped: bool,
}

#[must_use]
pub(crate) fn is_trigger_method(method: &str) -> bool {
    TRIGGER_METHODS.contains(&method)
}

/// Decide whether to replace this image. Pure: never execs.
#[must_use]
pub(crate) fn decide(probe: &ReexecProbe) -> ReexecDecision {
    if probe.check == CheckKind::RpcMethod
        && !probe.method.as_deref().is_some_and(is_trigger_method)
    {
        return stay(StayReason::NotATrigger, false);
    }
    if probe.phase != FramePhase::BetweenMessages {
        return stay(StayReason::MidFrame, false);
    }

    let skewed = is_skewed(probe.current_exe.as_deref(), probe.preferred.as_deref());

    // A refresh generation bump is an operator poke: re-check preferred
    // and allow one more recycle even if this image already attempted.
    let gate = match (probe.generation_bumped, probe.gate) {
        (true, ReexecGate::AlreadyAttempted) => ReexecGate::Allowed,
        (_, gate) => gate,
    };

    match gate {
        ReexecGate::KillSwitch => return stay(StayReason::KillSwitch, skewed),
        ReexecGate::AlreadyAttempted => return stay(StayReason::AlreadyReexeced, skewed),
        ReexecGate::PlatformDemoted => return stay(StayReason::PlatformDemoted, skewed),
        ReexecGate::Pinned => return stay(StayReason::Pinned, skewed),
        ReexecGate::Allowed => {}
    }
    let Some(preferred) = probe.preferred.as_ref() else {
        return stay(StayReason::PreferredUnresolved, true);
    };
    if !skewed {
        return stay(StayReason::NotSkewed, false);
    }
    ReexecDecision::Reexec {
        preferred: preferred.clone(),
    }
}

fn stay(reason: StayReason, skewed: bool) -> ReexecDecision {
    ReexecDecision::Stay { reason, skewed }
}

fn is_skewed(current: Option<&Path>, preferred: Option<&Path>) -> bool {
    match (current, preferred) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(current), Some(preferred)) => identities_differ(current, preferred),
    }
}

fn identities_differ(current: &Path, preferred: &Path) -> bool {
    identity(current) != identity(preferred)
}

fn identity(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve the preferred executable (spec §6).
///
/// An explicit override wins; otherwise the first `anvil` on `PATH`.
/// Never defaults to `current_exe()` (Cellar or otherwise).
#[must_use]
pub(crate) fn resolve_preferred_executable(
    override_path: Option<&OsStr>,
    path_var: Option<&OsStr>,
) -> Option<PathBuf> {
    if let Some(raw) = override_path.filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(raw));
    }
    find_command_on_path(PREFERRED_MCP_COMMAND, path_var)
}

fn find_command_on_path(name: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let path_var = path_var?;
    let names = command_names(name);
    for dir in env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        for candidate_name in &names {
            let candidate = dir.join(candidate_name);
            if is_runnable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn command_names(name: &str) -> Vec<String> {
    if cfg!(windows) {
        if Path::new(name).extension().is_some() {
            vec![name.to_string()]
        } else {
            vec![format!("{name}.exe"), name.to_string()]
        }
    } else {
        vec![name.to_string()]
    }
}

fn is_runnable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn env_flag_set(name: &str) -> bool {
    match env::var_os(name) {
        None => false,
        Some(value) => {
            let text = value.to_string_lossy();
            let text = text.trim();
            !text.is_empty()
                && !matches!(
                    text.to_ascii_lowercase().as_str(),
                    "0" | "false" | "no" | "off"
                )
        }
    }
}

#[must_use]
pub(crate) fn probe_from_process(method: Option<&str>, phase: FramePhase) -> ReexecProbe {
    let override_path = env::var_os(PREFERRED_ENV);
    let path_var = env::var_os("PATH");
    ReexecProbe {
        method: method.map(str::to_owned),
        phase,
        check: CheckKind::RpcMethod,
        gate: gate_from_process(),
        current_exe: env::current_exe().ok(),
        preferred: resolve_preferred_executable(override_path.as_deref(), path_var.as_deref()),
        generation_bumped: consume_generation_bump(),
    }
}

/// Treat generation greater than last seen as a preferred-binary re-check.
///
/// The first observation in a process is a baseline, not a poke. Only a
/// later increase (operator `anvil mcp refresh` while this image is still
/// running) retries after `AlreadyAttempted`.
fn consume_generation_bump() -> bool {
    let current = crate::commands::mcp_generation::current_generation();
    consume_generation_bump_from(current, &GENERATION_SEEN, &LAST_SEEN_GENERATION)
}

fn consume_generation_bump_from(current: u64, seen: &AtomicBool, last_seen: &AtomicU64) -> bool {
    if !seen.swap(true, Ordering::SeqCst) {
        last_seen.store(current, Ordering::SeqCst);
        return false;
    }
    let last = last_seen.load(Ordering::SeqCst);
    if current > last {
        last_seen.store(current, Ordering::SeqCst);
        true
    } else {
        false
    }
}

fn gate_from_process() -> ReexecGate {
    if env_flag_set(NO_REEXEC_ENV) {
        ReexecGate::KillSwitch
    } else if crate::commands::mcp_heal::heal_policy().is_pinned() {
        ReexecGate::Pinned
    } else if env_flag_set(REEXECED_ENV) || REEXEC_ATTEMPTED.load(Ordering::SeqCst) {
        ReexecGate::AlreadyAttempted
    } else if cfg!(unix) {
        ReexecGate::Allowed
    } else {
        ReexecGate::PlatformDemoted
    }
}

/// Recycle before the first stdin read so `initialize` is not consumed
/// by a stale image. Does not return if re-exec succeeds.
pub(crate) fn maybe_reexec_at_startup() {
    let mut probe = probe_from_process(None, FramePhase::BetweenMessages);
    probe.check = CheckKind::Startup;
    apply_decision(&decide(&probe));
}

/// Check at a between-message boundary. Does not return if re-exec succeeds.
pub(crate) fn maybe_reexec_between_messages(message: &Value) {
    let method = message.get("method").and_then(Value::as_str);
    if !method.is_some_and(is_trigger_method) {
        return;
    }
    apply_decision(&decide(&probe_from_process(
        method,
        FramePhase::BetweenMessages,
    )));
}

fn apply_decision(decision: &ReexecDecision) {
    match decision {
        ReexecDecision::Reexec { preferred } => exec_preferred(preferred),
        ReexecDecision::Stay { reason, skewed } if *skewed => {
            if let Some(hint) = reason.recovery_hint() {
                eprintln!("anvil mcp serve: {hint}");
            }
        }
        ReexecDecision::Stay { .. } => {}
    }
}

fn exec_preferred(preferred: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        let mut cmd = Command::new(preferred);
        // spec §7.1: argv shape `["anvil", "mcp", "serve", "--stdio", …]`
        cmd.arg0(PREFERRED_MCP_COMMAND);
        cmd.args(env::args_os().skip(1));
        cmd.env(REEXECED_ENV, "1");
        REEXEC_ATTEMPTED.store(true, Ordering::SeqCst);
        let err = cmd.exec();
        eprintln!(
            "anvil mcp serve: failed to re-exec {}: {err}. {}",
            preferred.display(),
            StayReason::AlreadyReexeced
                .recovery_hint()
                .unwrap_or("Retry a tool call, or reconnect MCP for this client.")
        );
    }
    #[cfg(not(unix))]
    {
        let _ = preferred;
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    use super::{
        CheckKind, FramePhase, ReexecDecision, ReexecGate, ReexecProbe, StayReason,
        consume_generation_bump_from, decide, resolve_preferred_executable,
    };

    // These imports serve only the `#[cfg(unix)]` test below. On windows-msvc
    // that test does not exist, so unconditional imports here are unused and
    // `-D warnings` reds the cross-target Clippy leg (caught on the first two
    // PRs to rebase past ecb07bd6f). Scope them to the test.
    #[cfg(unix)]
    #[test]
    fn exec_failure_marks_process_already_reexeced() {
        use std::sync::atomic::Ordering;

        use super::{REEXEC_ATTEMPTED, exec_preferred};

        exec_preferred(std::path::Path::new("/nonexistent-anvil-reexec"));
        assert!(
            REEXEC_ATTEMPTED.load(Ordering::SeqCst),
            "failed exec must still consume the one-shot anti-loop marker"
        );
        REEXEC_ATTEMPTED.store(false, Ordering::SeqCst);
    }

    fn skewed_probe() -> ReexecProbe {
        ReexecProbe {
            method: Some("tools/call".into()),
            phase: FramePhase::BetweenMessages,
            check: CheckKind::RpcMethod,
            gate: ReexecGate::Allowed,
            current_exe: Some(PathBuf::from(
                "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil",
            )),
            preferred: Some(PathBuf::from("/opt/homebrew/bin/anvil")),
            generation_bumped: false,
        }
    }

    #[test]
    fn mcp_reexec_generation_bump_rechecks_preferred_and_reexecs_if_skewed() {
        let mut probe = skewed_probe();
        probe.gate = ReexecGate::AlreadyAttempted;
        probe.generation_bumped = true;

        match decide(&probe) {
            ReexecDecision::Reexec { preferred } => {
                assert_eq!(preferred, PathBuf::from("/opt/homebrew/bin/anvil"));
            }
            stay @ ReexecDecision::Stay { .. } => {
                panic!(
                    "generation bump must re-check preferred and re-exec when skewed, got {stay:?}"
                )
            }
        }
    }

    #[test]
    fn mcp_reexec_generation_bump_stays_when_preferred_matches() {
        let mut probe = skewed_probe();
        probe.current_exe = probe.preferred.clone();
        probe.gate = ReexecGate::AlreadyAttempted;
        probe.generation_bumped = true;
        assert_eq!(
            decide(&probe),
            ReexecDecision::Stay {
                reason: StayReason::NotSkewed,
                skewed: false,
            }
        );
    }

    #[test]
    fn mcp_reexec_anti_loop_does_not_reexec_when_already_reexeced_and_skewed() {
        let mut probe = skewed_probe();
        probe.gate = ReexecGate::AlreadyAttempted;

        let decision = decide(&probe);

        assert_eq!(
            decision,
            ReexecDecision::Stay {
                reason: StayReason::AlreadyReexeced,
                skewed: true,
            }
        );
    }

    #[test]
    fn mcp_reexec_first_generation_observation_is_baseline_not_a_poke() {
        let seen = AtomicBool::new(false);
        let last = AtomicU64::new(0);
        assert!(
            !consume_generation_bump_from(3, &seen, &last),
            "existing generation file must not look like a fresh bump on a new image"
        );
        assert_eq!(last.load(Ordering::SeqCst), 3);
        assert!(
            !consume_generation_bump_from(3, &seen, &last),
            "unchanged generation must not poke again"
        );
        assert!(
            consume_generation_bump_from(4, &seen, &last),
            "a later bump while this image is running is a poke"
        );
        assert_eq!(last.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn mcp_reexec_missing_generation_then_first_bump_is_a_poke() {
        let seen = AtomicBool::new(false);
        let last = AtomicU64::new(0);
        assert!(!consume_generation_bump_from(0, &seen, &last));
        assert!(consume_generation_bump_from(1, &seen, &last));
    }

    #[test]
    fn mcp_reexec_between_message_gate_skips_mid_handler() {
        let mut probe = skewed_probe();
        probe.phase = FramePhase::MidHandler;

        let decision = decide(&probe);

        assert_eq!(
            decision,
            ReexecDecision::Stay {
                reason: StayReason::MidFrame,
                skewed: false,
            }
        );
    }

    #[test]
    fn mcp_reexec_kill_switch_disables_reexec() {
        let mut probe = skewed_probe();
        probe.gate = ReexecGate::KillSwitch;

        let decision = decide(&probe);

        assert_eq!(
            decision,
            ReexecDecision::Stay {
                reason: StayReason::KillSwitch,
                skewed: true,
            }
        );
    }

    #[test]
    fn mcp_reexec_unix_reexecs_when_skewed_between_messages() {
        let probe = skewed_probe();
        match decide(&probe) {
            ReexecDecision::Reexec { preferred } => {
                assert_eq!(preferred, PathBuf::from("/opt/homebrew/bin/anvil"));
            }
            stay @ ReexecDecision::Stay { .. } => panic!("expected re-exec, got {stay:?}"),
        }
    }

    #[test]
    fn mcp_reexec_windows_demotes_to_honest_skew() {
        let mut probe = skewed_probe();
        probe.gate = ReexecGate::PlatformDemoted;

        let decision = decide(&probe);

        assert_eq!(
            decision,
            ReexecDecision::Stay {
                reason: StayReason::PlatformDemoted,
                skewed: true,
            }
        );
        let hint = StayReason::PlatformDemoted
            .recovery_hint()
            .expect("windows demotion must surface a recovery hint");
        assert!(
            !hint.to_ascii_lowercase().contains("restart your editor"),
            "demotion hint must not lead with editor restart: {hint}"
        );
    }

    #[test]
    fn mcp_reexec_recovery_hint_does_not_lead_with_restart_editor() {
        for reason in [
            StayReason::KillSwitch,
            StayReason::AlreadyReexeced,
            StayReason::PlatformDemoted,
            StayReason::PreferredUnresolved,
            StayReason::Pinned,
        ] {
            let hint = reason
                .recovery_hint()
                .unwrap_or_else(|| panic!("{reason:?} must publish a recovery hint"));
            let lower = hint.to_ascii_lowercase();
            assert!(
                !lower.starts_with("restart"),
                "{reason:?} hint must not lead with restart: {hint}"
            );
            assert!(
                !lower.contains("restart your editor"),
                "{reason:?} hint must not tell agents to restart the editor: {hint}"
            );
        }
    }

    #[test]
    fn mcp_reexec_skips_unrelated_methods() {
        let mut probe = skewed_probe();
        probe.method = Some("ping".into());
        assert_eq!(
            decide(&probe),
            ReexecDecision::Stay {
                reason: StayReason::NotATrigger,
                skewed: false,
            }
        );
    }

    #[test]
    fn mcp_reexec_stays_when_current_matches_preferred() {
        let mut probe = skewed_probe();
        probe.current_exe = probe.preferred.clone();
        assert_eq!(
            decide(&probe),
            ReexecDecision::Stay {
                reason: StayReason::NotSkewed,
                skewed: false,
            }
        );
    }

    #[test]
    fn mcp_reexec_startup_check_reexecs_when_skewed() {
        let mut probe = skewed_probe();
        probe.method = None;
        probe.check = CheckKind::Startup;
        assert!(
            matches!(decide(&probe), ReexecDecision::Reexec { .. }),
            "startup must recycle before the first frame is read"
        );
    }

    #[test]
    fn mcp_reexec_checks_initialize_and_tools_list() {
        for method in ["initialize", "tools/list"] {
            let mut probe = skewed_probe();
            probe.method = Some(method.into());
            assert!(
                matches!(decide(&probe), ReexecDecision::Reexec { .. }),
                "{method} must be a re-exec trigger"
            );
        }
    }

    #[test]
    fn mcp_reexec_preferred_override_wins_over_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let override_bin = write_fake_anvil(dir.path(), "override-anvil");
        let path_dir = tempfile::tempdir().expect("path dir");
        write_fake_anvil(path_dir.path(), "anvil");

        let resolved = resolve_preferred_executable(
            Some(override_bin.as_os_str()),
            Some(path_dir.path().as_os_str()),
        )
        .expect("override must resolve");

        assert_eq!(resolved, override_bin);
    }

    #[test]
    fn mcp_reexec_preferred_is_first_anvil_on_path_not_current_exe() {
        let path_dir = tempfile::tempdir().expect("path dir");
        let path_anvil = write_fake_anvil(path_dir.path(), "anvil");
        let cellar = tempfile::tempdir().expect("cellar");
        write_fake_anvil(cellar.path(), "anvil");

        let resolved = resolve_preferred_executable(None, Some(path_dir.path().as_os_str()))
            .expect("PATH anvil");

        assert_eq!(resolved, path_anvil);
        assert_ne!(
            resolved,
            cellar.path().join("anvil"),
            "must not treat a Cellar current_exe as preferred"
        );
    }

    #[test]
    fn mcp_reexec_preferred_skips_empty_path_components() {
        let path_dir = tempfile::tempdir().expect("path dir");
        let path_anvil = write_fake_anvil(path_dir.path(), "anvil");
        let mut path = OsString::from(":");
        path.push(path_dir.path());

        let resolved = resolve_preferred_executable(None, Some(path.as_os_str())).expect("PATH");
        assert_eq!(resolved, path_anvil);
    }

    fn write_fake_anvil(dir: &std::path::Path, name: &str) -> PathBuf {
        let file_name = if cfg!(windows) && std::path::Path::new(name).extension().is_none() {
            format!("{name}.exe")
        } else {
            name.to_string()
        };
        let path = dir.join(file_name);
        fs::write(&path, b"#!/bin/sh\nexit 0\n").expect("write fake anvil");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).expect("meta").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).expect("chmod");
        }
        path
    }
}
