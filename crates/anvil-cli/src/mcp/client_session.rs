//! JREL-002 / ADR-141: the live client session `anvil mcp serve --stdio`
//! registers with the intercept daemon.
//!
//! Before this lane existed the daemon could not tell an open editor from a
//! closed one: `anvil mcp serve` was invisible to it, live attributed leases
//! came only from `anvil run`, and `anvil start`'s durable activation-spine
//! record is byte-identical whether the editor is running or was closed hours
//! ago. Protection reporting therefore promoted clients to `LiveValidation`
//! on evidence that never meant "a client is attached".
//!
//! The session identifies itself from the MCP handshake's `clientInfo.name`
//! and registers as a live lease tagged `anvil-mcp/<client>#<starttime>`,
//! heartbeating until the process exits. A client that never identifies
//! itself is never registered — no attribution means no promotion, by design.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::{Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use anvil_intercept_proto::SessionId;
use serde_json::Value;

use crate::activation::agent_registry::AgentClientId;

/// Heartbeat cadence. The daemon registry evicts a live lease that has not
/// beaten within its 30 s TTL, so this leaves room for two missed beats.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

/// A registered live MCP client session. Dropping it stops the heartbeat
/// thread and unregisters, so a closed editor stops attesting promptly
/// rather than waiting out the registry TTL.
pub(crate) struct McpClientSession {
    session_id: SessionId,
    stop: Arc<(Mutex<bool>, Condvar)>,
    join: Option<JoinHandle<()>>,
}

impl McpClientSession {
    /// Register `client` as attached to `worktree` and start heartbeating.
    ///
    /// Best-effort: returns `None` when the daemon is unavailable or refuses.
    /// MCP serve never fails a session over protection bookkeeping.
    pub(crate) fn register(worktree: &Path, client: AgentClientId) -> Option<Self> {
        let starttime = process_starttime();
        let session_id = mcp_session_id(client, starttime);
        if !crate::registration::register_live_mcp_session(
            &session_id,
            worktree,
            client.label(),
            starttime,
        ) {
            return None;
        }

        let stop = Arc::new((Mutex::new(false), Condvar::new()));
        let thread_stop = Arc::clone(&stop);
        let beat_id = session_id.clone();
        let join = std::thread::Builder::new()
            .name("anvil-mcp-heartbeat".into())
            .spawn(move || {
                let (lock, cvar) = &*thread_stop;
                loop {
                    // Condvar wait rather than a sleep-poll: one wakeup per
                    // interval, and shutdown is still immediate. At ~100
                    // concurrent MCP sessions a 50 ms poll would burn ~2000
                    // wakeups/second across the machine for nothing.
                    let guard = lock.lock().unwrap_or_else(PoisonError::into_inner);
                    let (guard, _) = cvar
                        .wait_timeout(guard, HEARTBEAT_INTERVAL)
                        .unwrap_or_else(PoisonError::into_inner);
                    if *guard {
                        return;
                    }
                    drop(guard);
                    crate::registration::heartbeat_live_mcp_session(&beat_id);
                }
            })
            .ok()?;

        Some(Self {
            session_id,
            stop,
            join: Some(join),
        })
    }
}

impl Drop for McpClientSession {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.stop;
            let mut stopped = lock.lock().unwrap_or_else(PoisonError::into_inner);
            *stopped = true;
            cvar.notify_all();
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        crate::registration::unregister_live_mcp_session(&self.session_id);
    }
}

/// Session id for this MCP process. Derived from the client and the
/// process start time so a re-register from the same process is idempotent
/// while two editors attached to one worktree stay distinct.
fn mcp_session_id(client: AgentClientId, starttime: u64) -> SessionId {
    SessionId::new(format!(
        "sess_mcp_{}_{}_{}",
        client.label().replace('-', "_"),
        std::process::id(),
        starttime,
    ))
}

/// Process start time, used as the `AgentTag` disambiguator. Falls back to
/// the pid when the platform does not expose it — the pair only has to be
/// stable within this process, not globally meaningful.
fn process_starttime() -> u64 {
    #[cfg(target_os = "linux")]
    {
        // Field 22 (1-indexed) is starttime; the comm field can contain
        // spaces and parentheses, so parse after the final ')'.
        if let Ok(stat) = std::fs::read_to_string("/proc/self/stat")
            && let Some((_, rest)) = stat.rsplit_once(')')
            && let Some(value) = rest.split_whitespace().nth(19)
            && let Ok(parsed) = value.parse::<u64>()
        {
            return parsed;
        }
    }
    u64::from(std::process::id())
}

/// Map an MCP `clientInfo.name` to a known client.
///
/// Clients spell themselves inconsistently (`claude-code`, `Claude Code`,
/// `Visual Studio Code`), so matching is case-insensitive over a
/// non-alphanumeric-normalised form and reuses each client's stable label
/// plus the aliases real clients actually send. An unrecognised name maps
/// to `None` and the session is never registered.
pub(crate) fn client_from_client_info_name(name: &str) -> Option<AgentClientId> {
    let normalised = normalise(name);
    if normalised.is_empty() {
        return None;
    }
    AgentClientId::all()
        .iter()
        .map(|entry| entry.id)
        .find(|id| {
            identity_aliases(*id)
                .iter()
                .any(|alias| *alias == normalised)
        })
}

/// Normalise a client name to lowercase alphanumerics plus `-`.
fn normalise(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// Accepted spellings per client, normalised. The client's own stable
/// label is always accepted; the extras are what real clients send in
/// `clientInfo.name`.
fn identity_aliases(client: AgentClientId) -> &'static [&'static str] {
    match client {
        AgentClientId::ClaudeCode => &["claude-code", "claude"],
        AgentClientId::Cursor => &["cursor"],
        AgentClientId::Codex => &["codex", "codex-cli"],
        AgentClientId::OpenCode => &["opencode", "open-code"],
        AgentClientId::GeminiCli => &["gemini-cli", "gemini"],
        AgentClientId::Antigravity => &["antigravity"],
        AgentClientId::OpenClaw => &["openclaw", "open-claw"],
        AgentClientId::VsCode => &["vscode", "vs-code", "visual-studio-code"],
        AgentClientId::CopilotCli => &["copilot-cli", "copilot"],
        AgentClientId::Grok => &["grok", "grok-cli"],
        AgentClientId::Warp => &["warp"],
        AgentClientId::Zed => &["zed"],
    }
}

/// Extract `clientInfo.name` from a handshake frame, covering both eras:
/// legacy `initialize` carries `params.clientInfo`, modern frames carry
/// `params._meta["io.modelcontextprotocol/clientInfo"]`.
pub(crate) fn client_info_name(message: &Value) -> Option<String> {
    let params = message.get("params")?;
    let legacy = params
        .get("clientInfo")
        .and_then(|info| info.get("name"))
        .and_then(Value::as_str);
    let modern = params
        .get("_meta")
        .and_then(|meta| meta.get(super::protocol::meta::META_CLIENT_INFO))
        .and_then(|info| info.get("name"))
        .and_then(Value::as_str);
    legacy.or(modern).map(str::to_owned)
}

/// The worktree this MCP server speaks for: its own working directory.
pub(crate) fn server_worktree() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn client_info_names_map_to_known_clients_across_spellings() {
        for (name, expected) in [
            ("claude-code", AgentClientId::ClaudeCode),
            ("Claude Code", AgentClientId::ClaudeCode),
            ("Cursor", AgentClientId::Cursor),
            ("Visual Studio Code", AgentClientId::VsCode),
            ("vscode", AgentClientId::VsCode),
            ("gemini-cli", AgentClientId::GeminiCli),
            ("Zed", AgentClientId::Zed),
        ] {
            assert_eq!(
                client_from_client_info_name(name),
                Some(expected),
                "{name} must attribute to {expected:?}",
            );
        }
    }

    /// An unknown or absent client is never registered: unattributed live
    /// evidence is exactly what JREL-002 stops counting as protection.
    #[test]
    fn unknown_client_names_are_not_attributed() {
        for name in ["", "   ", "some-unknown-editor", "anvil"] {
            assert_eq!(
                client_from_client_info_name(name),
                None,
                "{name:?} must not attribute to a known client",
            );
        }
    }

    #[test]
    fn client_info_name_reads_legacy_and_modern_handshakes() {
        let legacy = json!({
            "method": "initialize",
            "params": { "clientInfo": { "name": "claude-code", "version": "1" } }
        });
        assert_eq!(client_info_name(&legacy).as_deref(), Some("claude-code"));

        let modern = json!({
            "method": "tools/list",
            "params": {
                "_meta": {
                    super::super::protocol::meta::META_CLIENT_INFO: {
                        "name": "Cursor", "version": "2"
                    }
                }
            }
        });
        assert_eq!(client_info_name(&modern).as_deref(), Some("Cursor"));

        assert_eq!(client_info_name(&json!({ "method": "ping" })), None);
    }

    #[test]
    fn mcp_session_ids_are_distinct_per_client() {
        let claude = mcp_session_id(AgentClientId::ClaudeCode, 42);
        let cursor = mcp_session_id(AgentClientId::Cursor, 42);
        assert_ne!(claude.as_str(), cursor.as_str());
        assert!(claude.as_str().starts_with("sess_mcp_"));
    }
}
