//! MCP client abstraction for the activation flow (LAUNCH-009).
//!
//! Each supported editor exposes a [`McpClient`] implementation that knows:
//!
//! 1. Where its config file lives (per-platform paths).
//! 2. How to parse the file (JSON for v1; JSONC reserved for clients
//!    whose config preserves comments).
//! 3. How to merge an [`AnvilEntry`] into the existing config without
//!    clobbering user keys, classifying drift as `UpToDate`, `SafeDrift`,
//!    or `UnsafeDrift`.
//! 4. How to render the merged config back to disk text.
//! 5. How to verify which [`McpTier`] the client has reached for a given
//!    `AnvilEntry`.
//! 6. What restart hint to show the user when the tier caps at
//!    `RestartRequired`.
//!
//! ## Hosted-MCP-server pre-investments
//!
//! `AnvilEntry` is an enum with one variant today (`Stdio`); `RemoteSse`
//! and `RemoteHttp` variants are reserved for the future hosted-MCP-server
//! workstream. The trait signature uses the enum from day one so adding a
//! variant is a per-impl extension, not a contract change.
//!
//! `McpTransport` is a sibling enum that carries the transport tag through
//! the diagnostic JSON output (`{"tier": "config_present", "transport":
//! "stdio"}`). The schema is reserved now so the diagnostic doesn't need
//! a v2 migration when hosted lands.
//!
//! v1 ships only `Stdio`; impls may panic on other variants until the
//! hosted-server workstream extends them.
//!
//! ## V1 scope
//!
//! Cursor and Claude Code only, per the 2026-05-03 activation council.
//! `VsCode`, `Zed`, `OpenCode` are deferred; see LAUNCH-009 task body in
//! `plans/modules/launch-flow-readiness.aps.md` for the per-editor
//! rationale.
//!
//! ## Future install paths (out of v1 scope, but verified)
//!
//! For the **no-repo / user-global** case (user runs `anvil mcp install
//! --global` outside a workspace, or wants to register a hosted endpoint
//! against Claude Code without anvil touching `~/.claude.json` directly),
//! a viable alternative is to delegate to the editor's own CLI:
//!
//! ```sh
//! claude -p 'Run this shell command: claude mcp add --transport http anvil <url>' \
//!   --allowedTools 'Bash'
//! ```
//!
//! This invokes Claude Code in headless mode to run `claude mcp add` —
//! Claude Code owns the config mutation, anvil just orchestrates. Useful
//! when the file-based merge would risk corrupting an upstream invariant
//! we don't fully model. Verified working empirically. Not implemented in
//! v1; will resurface when the hosted-MCP-server workstream lands.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::diagnostic::{McpClientId, McpTier};

pub mod claude_code;
pub mod cursor;

/// What we install into the editor's config. Today only `Stdio` is
/// constructed; `RemoteSse` / `RemoteHttp` are reserved for the future
/// hosted-MCP-server workstream — see module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnvilEntry {
    /// Local stdio transport: editor spawns `anvil mcp serve --stdio`.
    Stdio {
        /// Path to the anvil binary (typically `current_exe()` resolved
        /// at orchestrator level).
        command: PathBuf,
        /// Arguments after the command — usually
        /// `["mcp", "serve", "--stdio"]`.
        args: Vec<String>,
        /// Environment variables to pass to the child. v1 leaves this
        /// empty; future hosted-server impls may set auth tokens here.
        env: BTreeMap<String, String>,
    },
    // RemoteSse { url, auth } — reserved
    // RemoteHttp { url, auth } — reserved
}

impl AnvilEntry {
    /// Construct the canonical local-stdio entry for an anvil binary at
    /// `command`. Used by the orchestrator after resolving `current_exe()`.
    pub fn local_stdio(command: PathBuf) -> Self {
        Self::Stdio {
            command,
            args: vec![
                "mcp".to_string(),
                "serve".to_string(),
                "--stdio".to_string(),
            ],
            env: BTreeMap::new(),
        }
    }

    /// Transport tag for diagnostic JSON output.
    pub fn transport(&self) -> McpTransport {
        match self {
            AnvilEntry::Stdio { .. } => McpTransport::Stdio,
        }
    }
}

/// Transport tag for diagnostic JSON. Reserved enum: only `Stdio` is
/// emitted in v1; `RemoteSse` / `RemoteHttp` are reserved for the
/// hosted-MCP-server workstream so the JSON schema doesn't need a
/// migration when hosted lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    // RemoteSse — reserved
    // RemoteHttp — reserved
}

impl McpTransport {
    pub fn label(self) -> &'static str {
        match self {
            McpTransport::Stdio => "stdio",
        }
    }
}

/// Drift classification of an existing anvil entry against a freshly-built
/// `AnvilEntry`. Closes the council's adversarial finding that the previous
/// `bool drifted` flag conflated "old anvil version" with "foreign tool
/// using our key" and silently overwrote the latter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftClass {
    /// Existing entry matches our `AnvilEntry` byte-for-byte.
    UpToDate,
    /// Same shape (recognisable as anvil), different binary path. The
    /// orchestrator may rewrite if the new path resolves to anvil; if the
    /// new path is unreachable, the orchestrator should escalate to
    /// `UnsafeDrift`.
    SafeDrift {
        /// Human-readable reason — e.g. "version upgrade: /usr/local/bin/anvil
        /// → /home/user/.cargo/bin/anvil".
        reason: String,
    },
    /// Existing entry's `command` field doesn't resolve to anvil, OR the
    /// key shape is unrecognised. Don't write; surface as `state: error`
    /// in the diagnostic with the cause.
    UnsafeDrift {
        /// Human-readable reason — e.g. "existing entry points at
        /// /opt/foo/anvil-shim which is not anvil".
        reason: String,
    },
}

/// Parsed editor config. The trait owns the parse step so each impl can
/// pick the right parser (strict JSON for v1; JSONC reserved for Zed's
/// future support).
#[derive(Debug, Clone)]
pub struct ParsedConfig {
    /// Raw parsed value (kept generic so JSON / JSONC impls share a
    /// type). v1 always uses `serde_json::Value` under the hood.
    pub raw: serde_json::Value,
    /// The existing anvil entry, if any.
    pub existing_entry: Option<serde_json::Value>,
}

/// Parse failure modes the orchestrator distinguishes when classifying
/// the diagnostic.
#[derive(Debug)]
#[allow(dead_code)] // payloads are read by the orchestrator install path (LAUNCH-006 follow-up)
pub enum ParseError {
    /// File does not exist on disk.
    NotFound,
    /// File exists but is empty / whitespace only.
    Empty,
    /// Parser returned an error — payload carries the human-readable
    /// reason (e.g. "JSON parse error at line 12: unexpected token").
    Invalid(String),
    /// Top-level value is not the expected shape (typically not an
    /// object). Distinct from `Invalid` so the orchestrator can render
    /// a different next-step.
    UnexpectedShape(String),
}

/// Probe candidate config path with platform-aware resolution.
#[derive(Debug, Clone)]
pub struct ConfigCandidate {
    pub path: PathBuf,
    /// Workspace-local (`.anvil-relative`) vs user-global (`$HOME` /
    /// `$APPDATA`). Used by the orchestrator to prefer per-repo configs
    /// over per-user ones when both are present.
    pub scope: ConfigScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// `.cursor/mcp.json` etc. — relative to the current workspace.
    Workspace,
    /// `~/.cursor/mcp.json` etc. — per-user.
    Global,
}

/// Per-editor MCP integration contract.
///
/// Implementations live in submodules (`cursor`, `claude_code`). Each is
/// a zero-sized struct so the trait can be used as `&dyn McpClient` in a
/// static registry without lifetime gymnastics.
pub trait McpClient: Send + Sync {
    /// Stable identifier — matches the `McpClientId` enum.
    fn id(&self) -> McpClientId;

    /// Where the editor's MCP config might live, in priority order
    /// (first match wins). Different editors prefer workspace-local vs
    /// user-global; the orchestrator picks the first existing path.
    fn config_paths(&self, workspace: &Path, home: Option<&Path>) -> Vec<ConfigCandidate>;

    /// Parse the raw config text. v1 uses strict JSON for both impls;
    /// future Zed / Claude Code variants may switch to JSONC if their
    /// settings.json formats turn out to require comment preservation.
    fn parse(&self, raw: &str) -> Result<ParsedConfig, ParseError>;

    /// Classify drift between the existing entry (if any) and the
    /// freshly-built `AnvilEntry`. The orchestrator decides whether to
    /// write based on the returned `DriftClass`.
    fn classify_drift(&self, parsed: &ParsedConfig, fresh: &AnvilEntry) -> DriftClass;

    /// Merge `fresh` into `parsed.raw` and return the rendered config
    /// text ready to write atomically. Caller is responsible for the
    /// drift check before calling — this method always installs.
    fn merge_and_render(&self, parsed: &ParsedConfig, fresh: &AnvilEntry)
    -> Result<String, String>;

    /// Render the freshly-built config when no existing file is present.
    /// Distinct from `merge_and_render` because there's nothing to merge.
    fn render_new(&self, fresh: &AnvilEntry) -> Result<String, String>;

    /// Determine the [`McpTier`] reached for `fresh` given the parsed
    /// config (or absence). Tier transitions are documented in the
    /// LAUNCH-009 task body:
    ///
    /// - `ConfigAbsent → ConfigPresent`: anvil entry parses cleanly in
    ///   the file.
    /// - `ConfigPresent → RestartRequired`: ALWAYS emit `RestartRequired`
    ///   on a fresh write — we cannot observe restart without IPC.
    /// - `RestartRequired → ServerStartable`: caller spawns the entry
    ///   and observes a clean MCP handshake. (Implemented by the
    ///   orchestrator, not here, because the spawn probe is shared
    ///   across impls.)
    /// - `LiveValidation`: out-of-scope for v1. INTD-only.
    ///
    /// This method returns the tier based on the on-disk evidence
    /// (config absent / present / matches fresh entry). The orchestrator
    /// promotes to `ServerStartable` separately via [`probe_startable`].
    fn verify_config_tier(&self, parsed: Option<&ParsedConfig>, fresh: &AnvilEntry) -> McpTier;

    /// Human-readable hint shown when the tier caps at `RestartRequired`.
    /// Editor-specific because the user's restart action differs
    /// (Cursor: quit + reopen; Claude Code: terminal restart).
    /// Used by the orchestrator install path (LAUNCH-006 follow-up).
    #[allow(dead_code)]
    fn restart_hint(&self) -> &'static str;
}

/// Static registry of all clients that ship in v1.
///
/// Returns `&dyn McpClient` so callers can iterate over them generically.
/// Adding a client means: implement the trait in a new submodule and
/// add a `&` to this slice.
pub fn all_clients() -> &'static [&'static dyn McpClient] {
    &[&cursor::Cursor, &claude_code::ClaudeCode]
}

/// Probe each registered client against the user's filesystem and return
/// the tier each has reached.
///
/// This is the function that replaces the
/// `BTreeMap::new()` stub at `activation/diagnostic.rs::verify` — when
/// activation runs, it walks the registry, parses each client's config
/// (if found), and reports the resulting tier.
///
/// **Read-only.** This function does not write any editor config; it
/// only reads. Install paths (`merge_and_render` + `atomic_write`)
/// are driven by the orchestrator separately.
///
/// `fresh` is the canonical anvil entry the activation flow would
/// install. It's required because tier classification depends on
/// matching against what we'd write — we can't tell `RestartRequired`
/// from `ConfigPresent` without comparing.
pub fn probe_all(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> BTreeMap<McpClientId, McpTier> {
    let mut out = BTreeMap::new();
    for client in all_clients() {
        let tier = probe_one(*client, workspace, home, fresh);
        out.insert(client.id(), tier);
    }
    out
}

fn probe_one(
    client: &dyn McpClient,
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> McpTier {
    // Walk the candidate paths; first existing+parseable wins.
    for candidate in client.config_paths(workspace, home) {
        match std::fs::read_to_string(&candidate.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                // I/O error other than NotFound — treat as no config so
                // the diagnostic doesn't blame the user for transient
                // permission issues. The orchestrator's install path
                // will surface a more specific error if writing fails.
            }
            Ok(raw) => match client.parse(&raw) {
                Ok(parsed) => return client.verify_config_tier(Some(&parsed), fresh),
                Err(_) => {
                    // Parse failure is not the same as no config — the
                    // file exists but is broken. Surface as
                    // `ConfigPresent` so the orchestrator knows to
                    // engage drift handling, not as `ConfigAbsent`
                    // which would silently install.
                    return McpTier::ConfigPresent;
                }
            },
        }
    }
    // No candidate path existed.
    McpTier::ConfigAbsent
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn fresh() -> AnvilEntry {
        AnvilEntry::local_stdio(std::path::PathBuf::from("/usr/local/bin/anvil"))
    }

    #[test]
    fn probe_all_returns_one_entry_per_registered_client() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let map = probe_all(ws.path(), Some(home.path()), &fresh());
        assert_eq!(map.len(), 2, "v1 has Cursor + ClaudeCode");
        assert!(map.contains_key(&McpClientId::Cursor));
        assert!(map.contains_key(&McpClientId::ClaudeCode));
    }

    #[test]
    fn probe_with_no_configs_anywhere_reports_config_absent() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let map = probe_all(ws.path(), Some(home.path()), &fresh());
        assert_eq!(map[&McpClientId::Cursor], McpTier::ConfigAbsent);
        assert_eq!(map[&McpClientId::ClaudeCode], McpTier::ConfigAbsent);
    }

    #[test]
    fn probe_with_workspace_cursor_config_promotes_tier() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // Write a workspace-scoped Cursor config that matches our fresh
        // entry exactly — should land at RestartRequired.
        fs::create_dir_all(ws.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        fs::write(ws.path().join(".cursor/mcp.json"), cfg).unwrap();
        let map = probe_all(ws.path(), Some(home.path()), &fresh());
        assert_eq!(map[&McpClientId::Cursor], McpTier::RestartRequired);
        // Claude Code still absent.
        assert_eq!(map[&McpClientId::ClaudeCode], McpTier::ConfigAbsent);
    }

    #[test]
    fn probe_with_malformed_config_reports_config_present_not_absent() {
        // The orchestrator distinguishes "no config" from "broken
        // config" so it doesn't silently install over a broken file.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(ws.path().join(".cursor")).unwrap();
        fs::write(
            ws.path().join(".cursor/mcp.json"),
            "{this is not valid JSON",
        )
        .unwrap();
        let map = probe_all(ws.path(), Some(home.path()), &fresh());
        assert_eq!(map[&McpClientId::Cursor], McpTier::ConfigPresent);
    }

    #[test]
    fn probe_workspace_takes_precedence_over_home() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // Workspace has the matching entry; home has a different command.
        fs::create_dir_all(ws.path().join(".cursor")).unwrap();
        let ws_cfg = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        fs::write(ws.path().join(".cursor/mcp.json"), ws_cfg).unwrap();

        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let home_cfg = r#"{"mcpServers": {"anvil": {"command": "/different/path", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        fs::write(home.path().join(".cursor/mcp.json"), home_cfg).unwrap();

        let map = probe_all(ws.path(), Some(home.path()), &fresh());
        // Workspace wins → RestartRequired (matching).
        assert_eq!(map[&McpClientId::Cursor], McpTier::RestartRequired);
    }

    #[test]
    fn local_stdio_constructor_uses_canonical_args() {
        let entry = AnvilEntry::local_stdio(std::path::PathBuf::from("/foo/bar"));
        match entry {
            AnvilEntry::Stdio { command, args, env } => {
                assert_eq!(command, std::path::PathBuf::from("/foo/bar"));
                assert_eq!(args, vec!["mcp", "serve", "--stdio"]);
                assert!(env.is_empty());
            }
        }
    }

    #[test]
    fn transport_tag_matches_variant() {
        let entry = AnvilEntry::local_stdio(std::path::PathBuf::from("/anvil"));
        assert_eq!(entry.transport(), McpTransport::Stdio);
        assert_eq!(McpTransport::Stdio.label(), "stdio");
    }
}
