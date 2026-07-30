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
//! ## Legacy activation adapters
//!
//! This trait remains the richer probe/install adapter for Cursor and Claude
//! Code. MCPX first-wave clients, including VS Code, project-scoped Zed, and
//! `OpenCode`, use the typed agent registry and managed installer; their absence
//! from this trait is not a statement that support is deferred.
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
#[allow(dead_code)] // returned by trait method; consumed by orchestrator install path (LAUNCH-006 follow-up)
pub enum DriftClass {
    /// No existing anvil entry in the parsed config. The install path
    /// should write a fresh entry. Distinct from `UpToDate` because
    /// `UpToDate` implies the entry is already present and correct;
    /// `NotPresent` is a clear "nothing to drift from" signal so the
    /// orchestrator's install gate can be `matches!(NotPresent | SafeDrift)`.
    NotPresent,
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
#[allow(dead_code)] // raw is read by merge_and_render via the trait — used by orchestrator install path
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

impl ParseError {
    /// Human-readable reason for `tracing::warn!` and the diagnostic's
    /// `last_error` field.
    pub fn reason(&self) -> String {
        match self {
            ParseError::Empty => "config file is empty".to_string(),
            ParseError::Invalid(s) | ParseError::UnexpectedShape(s) => s.clone(),
        }
    }
}

/// Render-failure modes returned by `merge_and_render` and `render_new`.
/// Typed (rather than `String`) so the install-path follow-up doesn't have
/// to string-match error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // variants are returned by trait methods; consumed by orchestrator install path
pub enum RenderError {
    /// Existing config root is not a JSON object.
    BadRoot,
    /// `mcpServers` (or equivalent) is present but not an object.
    BadServersKey,
    /// Claude Code `permissions` is present but not an object.
    BadPermissionsKey,
    /// Claude Code `permissions.allow` is present but not an array of strings.
    BadAllowKey,
    /// Claude Code settings JSON could not be parsed.
    BadSettingsJson(String),
    /// `serde_json::to_string_pretty` failed — payload carries the
    /// underlying reason (rare; usually I/O or invalid Value content).
    Serialise(String),
    /// `AnvilEntry` could not be serialised because the binary path
    /// is not valid UTF-8 (Windows non-ANSI paths). Surfaces as a
    /// loud error instead of silently writing a U+FFFD-corrupted path.
    InvalidCommandPath,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::BadRoot => write!(f, "config root is not a JSON object"),
            RenderError::BadServersKey => write!(f, "`mcpServers` is present but not an object"),
            RenderError::BadPermissionsKey => {
                write!(f, "`permissions` is present but not an object")
            }
            RenderError::BadAllowKey => {
                write!(
                    f,
                    "`permissions.allow` is present but not an array of strings"
                )
            }
            RenderError::BadSettingsJson(s) => write!(f, "settings JSON parse error: {s}"),
            RenderError::Serialise(s) => write!(f, "serialise: {s}"),
            RenderError::InvalidCommandPath => {
                write!(f, "anvil binary path is not valid UTF-8")
            }
        }
    }
}

/// Probe candidate config path with platform-aware resolution.
#[derive(Debug, Clone)]
#[allow(dead_code)] // scope is consumed by the orchestrator install path (LAUNCH-006 follow-up)
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
#[allow(dead_code)] // classify_drift / merge_and_render / render_new are called by the orchestrator install path (LAUNCH-006 follow-up)
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
    /// write based on the returned `DriftClass`. Returns `NotPresent`
    /// when no anvil entry exists in the parsed config.
    fn classify_drift(&self, parsed: &ParsedConfig, fresh: &AnvilEntry) -> DriftClass;

    /// Merge `fresh` into `parsed.raw` and return the rendered config
    /// text ready to write atomically. Caller is responsible for the
    /// drift check before calling — this method always installs.
    fn merge_and_render(
        &self,
        parsed: &ParsedConfig,
        fresh: &AnvilEntry,
    ) -> Result<String, RenderError>;

    /// Render the freshly-built config when no existing file is present.
    /// Distinct from `merge_and_render` because there's nothing to merge.
    fn render_new(&self, fresh: &AnvilEntry) -> Result<String, RenderError>;

    /// Determine the [`McpTier`] reached for `fresh` given the parsed
    /// config (or absence). Tier transitions are documented in the
    /// LAUNCH-009 task body:
    ///
    /// - `ConfigAbsent → ConfigPresent`: anvil entry parses cleanly in
    ///   the file.
    /// - `ConfigPresent → RestartRequired`: ALWAYS emit `RestartRequired`
    ///   on a fresh write — we cannot observe restart without IPC.
    /// - `RestartRequired → RestartHandshakeVerified`: caller spawns
    ///   the installed entry and observes a clean MCP handshake.
    ///   (Implemented by the diagnostic probe, not here, because the
    ///   spawn probe is shared across impls.)
    /// - `RestartHandshakeVerified → LiveValidation`: promoted by
    ///   `crate::activation::daemon_evidence::promote_to_live_validation_when_daemon_attests`
    ///   when the intercept daemon attests live enforcement for the
    ///   current worktree (MLP2-051f). The daemon is the canonical
    ///   evidence source; without it the activation diagnostic caps
    ///   at `RestartHandshakeVerified`.
    ///
    /// This method returns the tier based on the on-disk evidence
    /// (config absent / present / matches fresh entry). The promotion
    /// to `RestartHandshakeVerified` happens later in the diagnostic
    /// verification probe — that probe runs [`probe_startable`] against
    /// the installed entry and applies the promotion in
    /// `activation::diagnostic`, not here.
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

/// The set of every MCP client id that ships in v1.
///
/// Used by the orchestrator's `--all-mcp-clients` / `ANVIL_ALL_MCP_CLIENTS`
/// opt-out from editor-aware install gating (ACTMO-012), and by tests that
/// want the pre-gating "consider every client" behaviour.
pub fn all_client_ids() -> std::collections::BTreeSet<McpClientId> {
    all_clients().iter().map(|c| c.id()).collect()
}

// ---------------------------------------------------------------------------
// Shared helpers used by every (current and future) JSON-based MCP client
// impl. Reduces ~80% duplication between cursor.rs and claude_code.rs;
// each impl now wraps these with its server-name + entry-builder.
// ---------------------------------------------------------------------------

/// Shared `parse` for the JSON-with-`mcpServers`-key shape Cursor and
/// Claude Code (and any future MCP-spec-compliant client) all use.
pub(crate) fn parse_json_mcp(raw: &str, server_name: &str) -> Result<ParsedConfig, ParseError> {
    let trimmed = raw.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| ParseError::Invalid(format!("JSON parse error: {e}")))?;
    if !value.is_object() {
        return Err(ParseError::UnexpectedShape(
            "top-level value must be a JSON object".to_string(),
        ));
    }
    // Council finding (copilot): `{"mcpServers": null}` or `[]` would
    // otherwise silently report `ConfigAbsent` for a structurally
    // broken config because `get(...).and_then(...)` returns `None`.
    // Surface it as a parse error so the orchestrator engages drift
    // handling rather than installing over the malformed file.
    if let Some(servers) = value.get("mcpServers")
        && !servers.is_object()
    {
        return Err(ParseError::UnexpectedShape(format!(
            "`mcpServers` must be a JSON object; found {}",
            shape_label(servers)
        )));
    }
    let existing = value
        .get("mcpServers")
        .and_then(|m| m.get(server_name))
        .cloned();
    Ok(ParsedConfig {
        raw: value,
        existing_entry: existing,
    })
}

/// Human-readable label for non-object `mcpServers` values in the
/// "unexpected shape" parse error.
fn shape_label(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Shared `merge_and_render` for the JSON-with-`mcpServers`-key shape.
///
/// **Preservation contract (semantic):**
/// - Every JSON value OUTSIDE `mcpServers.<server_name>` is preserved
///   semantically: other server entries (`mcpServers.other`),
///   top-level keys (`profile`, `unrelatedKey`), and any other
///   sub-trees survive intact. The output goes through
///   `serde_json::to_string_pretty`, which **may** rewrite whitespace,
///   indentation, and (for non-`Object` values) byte-level layout.
///   `serde_json::Map` preserves insertion order, so object key order
///   is also preserved in practice; document comments, trailing
///   commas, or other JSONC artefacts would not be (we use strict
///   JSON parsing).
/// - The `mcpServers.<server_name>` value is **replaced wholesale** with
///   the freshly-built `entry`. Any keys the user added inside their
///   anvil entry (e.g. a custom `timeout`, `disabled`, `description`)
///   are dropped on a `SafeDrift` rewrite.
///
/// The wholesale-replacement policy is deliberate (LAUNCH-009.5): the
/// drift classifier (`classify_drift_by_args`, `entries_equivalent`)
/// only treats the entry as anvil's when `args` and `command` match the
/// canonical shape, so we are confident the entry was anvil-installed
/// in the first place. A per-key merge would also need a schema for
/// "anvil-owned vs user-owned keys", which we do not have. If you
/// observe real-world configs that need preserved fields inside the
/// anvil entry, revisit via LAUNCH-009.5 follow-up.
#[allow(dead_code)] // called by trait merge_and_render impls; orchestrator-driven (LAUNCH-006 follow-up)
pub(crate) fn merge_json_mcp(
    parsed: &ParsedConfig,
    server_name: &str,
    entry: serde_json::Value,
) -> Result<String, RenderError> {
    let mut root = parsed.raw.clone();
    let obj = root.as_object_mut().ok_or(RenderError::BadRoot)?;
    let servers = obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let map = servers.as_object_mut().ok_or(RenderError::BadServersKey)?;
    map.insert(server_name.to_string(), entry);
    serde_json::to_string_pretty(&root).map_err(|e| RenderError::Serialise(e.to_string()))
}

/// Shared `render_new` for the JSON-with-`mcpServers`-key shape.
#[allow(dead_code)] // called by trait render_new impls; orchestrator-driven (LAUNCH-006 follow-up)
pub(crate) fn render_new_json_mcp(
    server_name: &str,
    entry: serde_json::Value,
) -> Result<String, RenderError> {
    let mut servers = serde_json::Map::new();
    servers.insert(server_name.to_string(), entry);
    let mut root = serde_json::Map::new();
    root.insert("mcpServers".to_string(), serde_json::Value::Object(servers));
    serde_json::to_string_pretty(&serde_json::Value::Object(root))
        .map_err(|e| RenderError::Serialise(e.to_string()))
}

/// Shared drift classifier: same args + anvil-shaped command path = `SafeDrift`;
/// same args + foreign command = `UnsafeDrift`;
/// different args = `UnsafeDrift`; non-object existing = `UnsafeDrift`.
/// Caller is responsible for the byte-for-byte equality check that produces
/// `UpToDate`.
///
/// Council finding (copilot): a foreign command like `/bin/bash` with our
/// args list would previously have been classified as `SafeDrift` and
/// eligible for overwrite. The `looks_like_anvil` check below is the
/// "foreign tool using our key" guardrail.
#[allow(dead_code)] // called by trait classify_drift impls
pub(crate) fn classify_drift_by_args(
    existing: &serde_json::Value,
    fresh: &AnvilEntry,
) -> DriftClass {
    let Some(obj) = existing.as_object() else {
        return DriftClass::UnsafeDrift {
            reason: "existing entry is not an object".to_string(),
        };
    };
    let existing_args: Vec<String> = obj
        .get("args")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let existing_cmd = obj.get("command").and_then(|c| c.as_str()).unwrap_or("");

    let AnvilEntry::Stdio {
        command: fresh_cmd,
        args: fresh_args,
        ..
    } = fresh;

    if existing_args != *fresh_args {
        return DriftClass::UnsafeDrift {
            reason: format!(
                "existing entry's args do not match anvil's launch shape (existing: {existing_args:?}, fresh: {fresh_args:?})"
            ),
        };
    }

    // Args match. Check the command's basename — if it doesn't look
    // like anvil, this is a foreign tool using our key, not a version
    // drift.
    if !looks_like_anvil(existing_cmd) {
        return DriftClass::UnsafeDrift {
            reason: format!(
                "existing entry's command `{existing_cmd}` does not look like anvil (basename must be `anvil` or `anvil.exe`)"
            ),
        };
    }

    DriftClass::SafeDrift {
        reason: format!(
            "version drift: existing command `{existing_cmd}` differs from fresh `{}`",
            fresh_cmd.display()
        ),
    }
}

/// True if `cmd` looks like an anvil binary path. Recognises bare
/// `"anvil"` (PATH-resolved), full paths ending in `/anvil`, Windows
/// backslash paths, and the `.exe` form.
///
/// We split on both `/` and `\` because the existing entry might be a
/// Windows path written by Cursor / Claude Code on Windows, which we'd
/// then probe on Unix in a CI matrix or smoke test.
pub(crate) fn looks_like_anvil(cmd: &str) -> bool {
    if cmd.is_empty() {
        return false;
    }
    let basename = cmd.rsplit(['/', '\\']).next().unwrap_or(cmd);
    matches!(basename, "anvil" | "anvil.exe")
}

/// True if `existing` (the on-disk entry) is byte-equivalent to `fresh`
/// (what we'd write), allowing for the bare `"anvil"`-vs-full-path
/// equivalence the standalone `anvil mcp-config` CLI introduced.
///
/// Council finding (copilot): users who installed via `anvil mcp-config`
/// have `"command": "anvil"` (bare, PATH-resolved). The probe builds
/// fresh from `current_exe()` (full path). Strict byte equality reports
/// these users as `ConfigPresent` not `RestartRequired`. Equivalence
/// here treats bare-`anvil` as matching when fresh's basename is
/// `anvil` / `anvil.exe`. `args`, `env`, and `type` (if present) must
/// still match exactly.
#[allow(dead_code)] // called by trait verify_config_tier / classify_drift impls
pub(crate) fn entries_equivalent(existing: &serde_json::Value, fresh: &serde_json::Value) -> bool {
    let (Some(eo), Some(fo)) = (existing.as_object(), fresh.as_object()) else {
        return existing == fresh;
    };
    // args / env / type must match exactly.
    if eo.get("args") != fo.get("args") {
        return false;
    }
    if eo.get("env") != fo.get("env") {
        return false;
    }
    if eo.get("type") != fo.get("type") {
        return false;
    }
    // Command: byte-equal OR bare-vs-full-path equivalence.
    let ec = eo.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let fc = fo.get("command").and_then(|v| v.as_str()).unwrap_or("");
    if ec == fc {
        return true;
    }
    // If existing is bare `"anvil"` (or `"anvil.exe"`) and fresh's
    // basename matches, treat as equivalent. Conversely, if fresh is
    // bare and existing's basename matches. Cross-platform basename
    // (split on both `/` and `\`) so a Windows-pathed existing entry
    // probed from a Unix smoke test still resolves correctly.
    let e_basename = ec.rsplit(['/', '\\']).next().unwrap_or(ec);
    let f_basename = fc.rsplit(['/', '\\']).next().unwrap_or(fc);
    let e_is_bare = e_basename == ec;
    let f_is_bare = f_basename == fc;
    // Only equivalence when at least one side is bare and basenames
    // match. Two full paths with the same basename but different
    // prefixes are version drift, not equivalence.
    if (e_is_bare || f_is_bare) && !e_basename.is_empty() && e_basename == f_basename {
        return true;
    }
    false
}

/// Convert the canonical command path to a UTF-8 `String` for inclusion
/// in JSON. Returns an explicit error on non-UTF-8 paths instead of
/// silently substituting U+FFFD via `to_string_lossy` — closes the
/// kernel-maintainer's Windows-non-UTF-8 footgun.
pub(crate) fn command_to_string(command: &Path) -> Result<String, RenderError> {
    command
        .to_str()
        .map(str::to_string)
        .ok_or(RenderError::InvalidCommandPath)
}

/// Per-client probe result emitted by [`probe_all`].
///
/// Carries the transport tag alongside the tier so the diagnostic JSON
/// renderer can emit `{"tier": "...", "transport": "..."}` from a single
/// source of truth — closes the council finding that the previous
/// renderer hardcoded `"stdio"` regardless of the entry's actual
/// transport. v1 always reports `Stdio`; future hosted-MCP-server
/// variants populate `RemoteSse` / `RemoteHttp` here.
/// Protocol era observed by the spawn probe (MCP26-007 diagnostic evidence).
///
/// CamelCase JSON keys (`protocolEra`) are emitted via serde rename on
/// [`McpProbeResult`]. The public tier label
/// (`restart_handshake_verified`) is intentionally unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolEraEvidence {
    Modern,
    Legacy,
}

/// How the spawn probe verified the installed anvil MCP entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    /// Modern `server/discover` succeeded.
    ServerDiscover,
    /// Legacy `initialize` handshake succeeded (fresh child after modern miss).
    Initialize,
}

/// Evidence returned by a successful [`probe_startable`] attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeEvidence {
    pub protocol_era: ProtocolEraEvidence,
    pub protocol_version: String,
    pub verification_method: VerificationMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpProbeResult {
    pub tier: McpTier,
    pub transport: McpTransport,
    /// Set after a successful spawn probe (MCP26-007). Omitted from JSON
    /// when unset so existing consumers stay compatible.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "protocolEra"
    )]
    pub protocol_era: Option<ProtocolEraEvidence>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "protocolVersion"
    )]
    pub protocol_version: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "verificationMethod"
    )]
    pub verification_method: Option<VerificationMethod>,
}

impl McpProbeResult {
    /// Convenience constructor for the local-stdio v1 case. Tests
    /// (and the rare orchestrator path that needs to fabricate a
    /// result) can use this without naming the transport explicitly.
    pub fn stdio(tier: McpTier) -> Self {
        Self {
            tier,
            transport: McpTransport::Stdio,
            protocol_era: None,
            protocol_version: None,
            verification_method: None,
        }
    }

    /// Attach MCP26-007 verification evidence without changing tier/transport.
    pub fn with_probe_evidence(mut self, evidence: ProbeEvidence) -> Self {
        self.protocol_era = Some(evidence.protocol_era);
        self.protocol_version = Some(evidence.protocol_version);
        self.verification_method = Some(evidence.verification_method);
        self
    }
}

impl From<McpTier> for McpProbeResult {
    fn from(tier: McpTier) -> Self {
        Self::stdio(tier)
    }
}

/// Probe each registered client against the user's filesystem and return
/// the per-client probe result.
///
/// **Read-only.** This function does not write any editor config; it
/// only reads. Install paths (`merge_and_render` + `atomic_write`)
/// are driven by the orchestrator separately.
///
/// `fresh` is the canonical anvil entry the activation flow would
/// install. Tier classification depends on matching against what we'd
/// write — we can't tell `RestartRequired` from `ConfigPresent` without
/// comparing.
///
/// Walks `config_paths` in priority order, and **continues past
/// candidates that have no anvil entry** — closing the council finding
/// that a workspace `.cursor/mcp.json` containing other servers (but no
/// anvil) silently shadowed a valid home install. The first candidate
/// whose anvil entry is `ConfigPresent` or higher wins; if every
/// candidate is `ConfigAbsent`, the result is `ConfigAbsent`.
pub fn probe_all(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> BTreeMap<McpClientId, McpProbeResult> {
    let mut out = BTreeMap::new();
    for client in all_clients() {
        let result = McpProbeResult {
            tier: probe_one(*client, workspace, home, fresh),
            transport: fresh.transport(),
            protocol_era: None,
            protocol_version: None,
            verification_method: None,
        };
        out.insert(client.id(), result);
    }
    out
}

fn probe_one(
    client: &dyn McpClient,
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> McpTier {
    // Walk the candidate paths in priority order. Stop on the first
    // candidate that produces a meaningful signal:
    //
    // - Tier > ConfigAbsent (anvil entry present, broken file, or
    //   I/O error): return immediately. This preserves the
    //   workspace-precedence rule the council flagged — a broken or
    //   permission-denied workspace config must not be silently
    //   shadowed by a valid home install.
    // - ConfigAbsent (file exists but no anvil entry, OR file
    //   doesn't exist): continue to the next candidate. This closes
    //   the original blind spot where a workspace `.cursor/mcp.json`
    //   with other servers (no anvil) hid a valid home install.
    for candidate in client.config_paths(workspace, home) {
        match std::fs::read_to_string(&candidate.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // No file at this scope — keep walking.
            }
            Err(e) => {
                // I/O error other than NotFound (e.g. permission
                // denied). Don't fall through to home — surface this
                // candidate's broken state so SREs see it.
                tracing::warn!(
                    client = %client.id().label(),
                    path = %candidate.path.display(),
                    error = %e,
                    "mcp probe: I/O error reading editor config",
                );
                return McpTier::ConfigPresent;
            }
            Ok(raw) => match client.parse(&raw) {
                Ok(parsed) => {
                    let tier = client.verify_config_tier(Some(&parsed), fresh);
                    if tier > McpTier::ConfigAbsent {
                        return tier;
                    }
                    // ConfigAbsent at this scope (no anvil entry).
                    // Keep walking — a higher-priority entry might
                    // exist at the home scope.
                }
                Err(e) => {
                    // Parse failure: file exists but is broken. Don't
                    // fall through — the orchestrator must engage
                    // drift handling on this specific file rather than
                    // silently installing over it.
                    tracing::warn!(
                        client = %client.id().label(),
                        path = %candidate.path.display(),
                        error = %e.reason(),
                        "mcp probe: parse error — reporting ConfigPresent so install path engages drift handling",
                    );
                    return McpTier::ConfigPresent;
                }
            },
        }
    }
    // No candidate yielded a tier > ConfigAbsent.
    McpTier::ConfigAbsent
}

// ---------------------------------------------------------------------------
// Spawn probe (LAUNCH-009.6): handshake against the installed entry. The
// diagnostic promotes `RestartRequired` to `RestartHandshakeVerified` on
// success so `ServerStartable` can keep its weaker no-client-wiring meaning.
// ---------------------------------------------------------------------------

/// Walk each registered client's config paths and return installed entries
/// whose tier is currently `RestartRequired`, keyed by client.
///
/// Used by the spawn probe so each handshake spawns the command that
/// specific editor would run (e.g. a bare `"anvil"` entry from
/// `anvil mcp-config` that PATH-resolves to a different binary than
/// `current_exe()`), not just `fresh`.
///
/// Skips a client when it is not at `RestartRequired`, when the installed
/// entry can't be re-parsed, or when the entry's `command` field is
/// missing/non-string. The caller may still probe `fresh` for
/// observability if this returns empty, but it must not promote a client
/// tier from fallback evidence because that is not the editor's actual
/// spawn target.
pub fn installed_restart_required_entries(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> BTreeMap<McpClientId, AnvilEntry> {
    let mut entries = BTreeMap::new();
    for client in all_clients() {
        for candidate in client.config_paths(workspace, home) {
            let Ok(raw) = std::fs::read_to_string(&candidate.path) else {
                continue;
            };
            let Ok(parsed) = client.parse(&raw) else {
                continue;
            };
            if client.verify_config_tier(Some(&parsed), fresh) != McpTier::RestartRequired {
                continue;
            }
            let Some(existing) = parsed.existing_entry.as_ref() else {
                continue;
            };
            if let Some(entry) = stdio_entry_from_value(existing) {
                entries.insert(client.id(), entry);
                break;
            }
        }
    }
    entries
}

/// Best-effort: parse an installed JSON entry back into an
/// [`AnvilEntry::Stdio`]. Returns `None` if the shape doesn't match
/// (missing or non-string `command`, args not a string array, etc.).
fn stdio_entry_from_value(v: &serde_json::Value) -> Option<AnvilEntry> {
    let obj = v.as_object()?;
    let command = obj.get("command")?.as_str()?;
    if command.is_empty() {
        return None;
    }
    let args: Vec<String> = obj
        .get("args")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let env: BTreeMap<String, String> = obj
        .get("env")
        .and_then(|e| e.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Some(AnvilEntry::Stdio {
        command: PathBuf::from(command),
        args,
        env,
    })
}

/// Legacy initialise protocol version the fallback probe announces.
const PROBE_LEGACY_PROTOCOL_VERSION: &str = "2025-06-18";

/// Modern protocol version the discovery probe requests.
const PROBE_MODERN_PROTOCOL_VERSION: &str = "2026-07-28";

/// Maximum wall-clock time the probe waits for one child's first response
/// frame. Applies **per attempt**: modern discover, then (on fallback) a
/// fresh legacy initialise child. Worst-case wall clock is therefore
/// **2 × this timeout** (~2s) per `RestartRequired` client on
/// `anvil status --verify` / `anvil start` when both attempts time out.
const PROBE_HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

/// Why a [`probe_startable`] attempt did not promote the tier.
///
/// Carried into `tracing::warn!` for SREs. Returned variants are not
/// public: callers should treat the function as `Result<_, _>` and only
/// promote on `Ok`. Failure rendering goes through the renderer's
/// per-client outcome strings, which never include this enum's payload.
#[derive(Debug, Clone)]
#[allow(dead_code)] // payloads are read via Debug in tracing::warn!
pub enum ProbeError {
    /// `Command::spawn` failed (binary missing, ENOENT, permission denied).
    Spawn(String),
    /// Couldn't take the child's stdin or stdout pipe (extremely rare).
    NoPipes,
    /// Failed to write a probe request to the child's stdin.
    Write(String),
    /// First-line read from stdout produced no bytes before the child
    /// closed the pipe (process exited before responding).
    EmptyResponse,
    /// First-line read did not arrive within
    /// [`PROBE_HANDSHAKE_TIMEOUT`].
    Timeout,
    /// Child response exceeded the MCP stdio frame ceiling.
    OversizedFrame,
    /// Response was non-UTF-8 or could not be parsed as JSON.
    ParseResponse(String),
    /// Response parsed as JSON but does not look like a successful
    /// modern discovery or legacy initialise result for anvil.
    BadResponse(String),
    /// `current_exe()` resolution or the entry's command is non-UTF-8 in
    /// a way that prevents spawning.
    InvalidCommand(String),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::Spawn(s) => write!(f, "spawn: {s}"),
            ProbeError::NoPipes => write!(f, "child stdin/stdout pipes were not captured"),
            ProbeError::Write(s) => write!(f, "write probe request: {s}"),
            ProbeError::EmptyResponse => write!(f, "child closed stdout before responding"),
            ProbeError::Timeout => write!(
                f,
                "no response within {}s",
                PROBE_HANDSHAKE_TIMEOUT.as_secs_f32()
            ),
            ProbeError::OversizedFrame => write!(
                f,
                "response exceeded {} byte MCP stdio frame limit",
                crate::commands::mcp::MAX_STDIO_FRAME_BYTES
            ),
            ProbeError::ParseResponse(s) => write!(f, "parse response: {s}"),
            ProbeError::BadResponse(s) => write!(f, "bad response: {s}"),
            ProbeError::InvalidCommand(s) => write!(f, "invalid command: {s}"),
        }
    }
}

/// Drive a dual-era MCP verification probe against the child the editor would
/// spawn for `entry`, and return evidence when the server responds as anvil
/// within [`PROBE_HANDSHAKE_TIMEOUT`] per attempt.
///
/// **Algorithm (MCP26-007):**
/// 1. Spawn a **disposable** child and probe modern `server/discover`.
/// 2. On valid modern discovery (anvil identity in result `_meta`), return
///    modern evidence.
/// 3. On non-modern failure (timeout, early exit, method-not-found, malformed
///    frame, etc.), **reap** that child and spawn a **fresh** child for legacy
///    `initialize`.
/// 4. Never leave probe children behind.
///
/// **Caller contract:** call this only when [`McpClient::verify_config_tier`]
/// has already returned [`McpTier::RestartRequired`] for `entry`.
///
/// **Tier behaviour:** LAUNCH-009.6 maps `Ok(_)` to
/// `RestartHandshakeVerified`, not `ServerStartable`. The public tier label
/// is unchanged; era/method live on diagnostic evidence fields.
pub fn probe_startable(entry: &AnvilEntry) -> Result<ProbeEvidence, ProbeError> {
    let AnvilEntry::Stdio { command, args, env } = entry;
    probe_stdio(command, args, env)
}

fn probe_stdio(
    command: &Path,
    args: &[String],
    env: &BTreeMap<String, String>,
) -> Result<ProbeEvidence, ProbeError> {
    // Attempt 1: modern discovery on a disposable child.
    match probe_child_once(command, args, env, ProbeRequest::ModernDiscover) {
        Ok(evidence) => return Ok(evidence),
        Err(err) if modern_probe_should_fallback(&err) => {
            tracing::debug!(
                error = %err,
                "mcp probe: modern discover did not verify; trying legacy initialise on a fresh child"
            );
        }
        Err(err) => return Err(err),
    }

    // Attempt 2: fresh child for legacy initialise (never reuse the modern child).
    probe_child_once(command, args, env, ProbeRequest::LegacyInitialize)
}

/// Whether a modern-discover failure should fall through to legacy initialise.
fn modern_probe_should_fallback(err: &ProbeError) -> bool {
    match err {
        // Modern server answered but rejected our version — do not pretend
        // a legacy initialise will fix a version mismatch on a modern binary.
        ProbeError::BadResponse(s) if s.contains("unsupported protocol version") => false,
        ProbeError::BadResponse(s) if s.contains("modern protocol error") => false,
        // Everything else: timeout, empty, parse, method-not-found shaped
        // bad responses, spawn issues on first attempt shouldn't fallback
        // if we couldn't spawn at all.
        ProbeError::Spawn(_) | ProbeError::InvalidCommand(_) | ProbeError::NoPipes => false,
        ProbeError::Write(_)
        | ProbeError::EmptyResponse
        | ProbeError::Timeout
        | ProbeError::OversizedFrame
        | ProbeError::ParseResponse(_)
        | ProbeError::BadResponse(_) => true,
    }
}

#[derive(Clone, Copy)]
enum ProbeRequest {
    ModernDiscover,
    LegacyInitialize,
}

fn read_probe_frame<R: std::io::BufRead>(reader: &mut R) -> Result<Vec<u8>, ProbeError> {
    let mut buf = Vec::new();
    {
        let mut limited =
            std::io::Read::take(reader, crate::commands::mcp::MAX_STDIO_FRAME_BYTES + 1);
        std::io::BufRead::read_until(&mut limited, b'\n', &mut buf)
            .map_err(|err| ProbeError::ParseResponse(format!("I/O error reading stdout: {err}")))?;
    }
    if u64::try_from(buf.len()).unwrap_or(u64::MAX) > crate::commands::mcp::MAX_STDIO_FRAME_BYTES {
        return Err(ProbeError::OversizedFrame);
    }
    Ok(buf)
}

fn probe_child_once(
    command: &Path,
    args: &[String],
    env: &BTreeMap<String, String>,
    request: ProbeRequest,
) -> Result<ProbeEvidence, ProbeError> {
    use std::io::{BufReader, Write};
    use std::process::{Command, Stdio};

    let mut child = Command::new(command)
        .args(args)
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| ProbeError::Spawn(e.to_string()))?;

    let Some(mut stdin) = child.stdin.take() else {
        reap_child(&mut child);
        return Err(ProbeError::NoPipes);
    };
    let Some(stdout) = child.stdout.take() else {
        reap_child(&mut child);
        return Err(ProbeError::NoPipes);
    };

    let (tx, rx) = std::sync::mpsc::channel::<std::result::Result<Vec<u8>, ProbeError>>();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let result = read_probe_frame(&mut reader);
        let _ = tx.send(result);
    });

    let request_line = match request {
        ProbeRequest::ModernDiscover => format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{{"_meta":{{"io.modelcontextprotocol/protocolVersion":"{ver}","io.modelcontextprotocol/clientCapabilities":{{}},"io.modelcontextprotocol/clientInfo":{{"name":"anvil-probe","version":"{cli_version}"}}}}}}}}"#,
            ver = PROBE_MODERN_PROTOCOL_VERSION,
            cli_version = env!("CARGO_PKG_VERSION"),
        ),
        ProbeRequest::LegacyInitialize => format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"{ver}","capabilities":{{}},"clientInfo":{{"name":"anvil-probe","version":"{cli_version}"}}}}}}"#,
            ver = PROBE_LEGACY_PROTOCOL_VERSION,
            cli_version = env!("CARGO_PKG_VERSION"),
        ),
    };

    if let Err(e) = writeln!(stdin, "{request_line}") {
        reap_child(&mut child);
        return Err(ProbeError::Write(e.to_string()));
    }
    drop(stdin);

    let response_bytes = match rx.recv_timeout(PROBE_HANDSHAKE_TIMEOUT) {
        Ok(Ok(bytes)) if bytes.iter().all(u8::is_ascii_whitespace) => {
            reap_child(&mut child);
            return Err(ProbeError::EmptyResponse);
        }
        Ok(Ok(bytes)) => bytes,
        Ok(Err(err)) => {
            reap_child(&mut child);
            return Err(err);
        }
        Err(_) => {
            reap_child(&mut child);
            return Err(ProbeError::Timeout);
        }
    };
    // Always reap — do not leave probe processes behind (MCP26-007).
    reap_child(&mut child);

    let response = std::str::from_utf8(&response_bytes)
        .map_err(|e| ProbeError::ParseResponse(format!("response is not UTF-8: {e}")))?;
    match request {
        ProbeRequest::ModernDiscover => validate_discover_response(response.trim()),
        ProbeRequest::LegacyInitialize => validate_initialize_response(response.trim()),
    }
}

fn reap_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Validate a modern `server/discover` success frame and extract anvil identity
/// from result `_meta` (MCP26-007).
fn validate_discover_response(raw: &str) -> Result<ProbeEvidence, ProbeError> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| ProbeError::ParseResponse(e.to_string()))?;
    if value.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Err(ProbeError::BadResponse("missing jsonrpc=2.0".to_string()));
    }
    if value.get("id").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(ProbeError::BadResponse(format!(
            "expected id=1, got {}",
            value
                .get("id")
                .map_or_else(|| "<missing>".to_string(), serde_json::Value::to_string)
        )));
    }
    if let Some(error) = value.get("error") {
        let code = error.get("code").and_then(serde_json::Value::as_i64);
        // Method not found → older anvil / non-modern; caller may fall back.
        if code == Some(-32601) {
            return Err(ProbeError::BadResponse(
                "method not found (non-modern server)".to_string(),
            ));
        }
        // Unsupported protocol version → modern server, wrong version.
        if code == Some(-32022) {
            return Err(ProbeError::BadResponse(format!(
                "unsupported protocol version (modern protocol error): {error}"
            )));
        }
        return Err(ProbeError::BadResponse(format!(
            "server returned JSON-RPC error: {error}"
        )));
    }
    let Some(result) = value.get("result") else {
        return Err(ProbeError::BadResponse("missing result".to_string()));
    };
    // Modern identity lives only in result _meta. Do not accept top-level
    // legacy `serverInfo` — that would false-promote stubs as modern discover
    // (Council 2026-07-28 full review).
    if result.get("resultType").and_then(serde_json::Value::as_str) != Some("complete") {
        return Err(ProbeError::BadResponse(
            "missing result.resultType=complete".to_string(),
        ));
    }
    let Some(server_info) = result
        .get("_meta")
        .and_then(|m| m.get("io.modelcontextprotocol/serverInfo"))
    else {
        return Err(ProbeError::BadResponse(
            "missing result._meta io.modelcontextprotocol/serverInfo".to_string(),
        ));
    };
    if server_info.get("name").and_then(|v| v.as_str()) != Some("anvil") {
        return Err(ProbeError::BadResponse(format!(
            "expected serverInfo.name=anvil, got {}",
            server_info
                .get("name")
                .map_or_else(|| "<missing>".to_string(), serde_json::Value::to_string)
        )));
    }

    let supported = result
        .get("supportedVersions")
        .and_then(serde_json::Value::as_array)
        .filter(|arr| !arr.is_empty());
    let Some(supported) = supported else {
        return Err(ProbeError::BadResponse(
            "missing non-empty result.supportedVersions".to_string(),
        ));
    };
    if !supported.iter().all(serde_json::Value::is_string) {
        return Err(ProbeError::BadResponse(
            "supportedVersions entries must be strings".to_string(),
        ));
    }
    if !supported
        .iter()
        .any(|v| v.as_str() == Some(PROBE_MODERN_PROTOCOL_VERSION))
    {
        return Err(ProbeError::BadResponse(format!(
            "supportedVersions must include {PROBE_MODERN_PROTOCOL_VERSION}"
        )));
    }

    Ok(ProbeEvidence {
        protocol_era: ProtocolEraEvidence::Modern,
        protocol_version: PROBE_MODERN_PROTOCOL_VERSION.to_string(),
        verification_method: VerificationMethod::ServerDiscover,
    })
}

/// Validate that `raw` is a JSON-RPC 2.0 success response to our legacy
/// initialize request. Split out so unit tests can exercise the validator
/// without spawning a child.
fn validate_initialize_response(raw: &str) -> Result<ProbeEvidence, ProbeError> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| ProbeError::ParseResponse(e.to_string()))?;
    if value.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Err(ProbeError::BadResponse("missing jsonrpc=2.0".to_string()));
    }
    if value.get("id").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(ProbeError::BadResponse(format!(
            "expected id=1, got {}",
            value
                .get("id")
                .map_or_else(|| "<missing>".to_string(), serde_json::Value::to_string)
        )));
    }
    if value.get("error").is_some() {
        return Err(ProbeError::BadResponse(format!(
            "server returned JSON-RPC error: {}",
            value["error"]
        )));
    }
    let Some(result) = value.get("result") else {
        return Err(ProbeError::BadResponse("missing result".to_string()));
    };
    let Some(server_info) = result.get("serverInfo") else {
        return Err(ProbeError::BadResponse(
            "missing result.serverInfo".to_string(),
        ));
    };
    if server_info.get("name").and_then(|v| v.as_str()) != Some("anvil") {
        return Err(ProbeError::BadResponse(format!(
            "expected result.serverInfo.name=anvil, got {}",
            server_info
                .get("name")
                .map_or_else(|| "<missing>".to_string(), serde_json::Value::to_string)
        )));
    }
    let protocol_version = result
        .get("protocolVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(PROBE_LEGACY_PROTOCOL_VERSION)
        .to_string();
    Ok(ProbeEvidence {
        protocol_era: ProtocolEraEvidence::Legacy,
        protocol_version,
        verification_method: VerificationMethod::Initialize,
    })
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
        assert_eq!(map[&McpClientId::Cursor], McpTier::ConfigAbsent.into());
        assert_eq!(map[&McpClientId::ClaudeCode], McpTier::ConfigAbsent.into());
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
        assert_eq!(map[&McpClientId::Cursor], McpTier::RestartRequired.into());
        // Claude Code still absent.
        assert_eq!(map[&McpClientId::ClaudeCode], McpTier::ConfigAbsent.into());
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
        assert_eq!(map[&McpClientId::Cursor], McpTier::ConfigPresent.into());
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
        assert_eq!(map[&McpClientId::Cursor], McpTier::RestartRequired.into());
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

    // --- Council remediation: copilot review on PR #1283 ---

    #[test]
    fn parse_rejects_null_mcp_servers() {
        // Council finding: `{"mcpServers": null}` previously returned
        // existing_entry: None silently. Now returns UnexpectedShape so
        // the orchestrator engages drift handling.
        let err = parse_json_mcp(r#"{"mcpServers": null}"#, "anvil").unwrap_err();
        match err {
            ParseError::UnexpectedShape(s) => assert!(s.contains("null")),
            other => panic!("expected UnexpectedShape, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_array_mcp_servers() {
        let err = parse_json_mcp(r#"{"mcpServers": [1, 2, 3]}"#, "anvil").unwrap_err();
        match err {
            ParseError::UnexpectedShape(s) => assert!(s.contains("array")),
            other => panic!("expected UnexpectedShape, got {other:?}"),
        }
    }

    #[test]
    fn looks_like_anvil_recognises_bare_and_full_path() {
        assert!(looks_like_anvil("anvil"));
        assert!(looks_like_anvil("/usr/local/bin/anvil"));
        assert!(looks_like_anvil("/home/user/.cargo/bin/anvil"));
        assert!(looks_like_anvil("anvil.exe"));
        assert!(looks_like_anvil("C:\\Users\\u\\.cargo\\bin\\anvil.exe"));

        assert!(!looks_like_anvil(""));
        assert!(!looks_like_anvil("/bin/bash"));
        assert!(!looks_like_anvil("/usr/local/bin/anvil-shim"));
        assert!(!looks_like_anvil("not-anvil"));
    }

    #[test]
    fn entries_equivalent_recognises_bare_anvil_vs_full_path() {
        // Council finding: `anvil mcp-config` writes `"command": "anvil"`
        // (bare); the activation probe builds fresh from current_exe()
        // (full path). Strict byte equality misclassifies these users.
        let bare_existing = serde_json::json!({
            "command": "anvil",
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
        });
        let full_fresh = serde_json::json!({
            "command": "/usr/local/bin/anvil",
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
        });
        assert!(entries_equivalent(&bare_existing, &full_fresh));
        assert!(entries_equivalent(&full_fresh, &bare_existing));
    }

    #[test]
    fn entries_equivalent_rejects_two_full_paths_with_same_basename() {
        // Two full paths with the same `anvil` basename but different
        // prefixes are version drift, not equivalence.
        let a = serde_json::json!({
            "command": "/nix/store/abc/bin/anvil",
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
        });
        let b = serde_json::json!({
            "command": "/usr/local/bin/anvil",
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
        });
        assert!(!entries_equivalent(&a, &b));
    }

    #[test]
    fn entries_equivalent_rejects_different_args() {
        let a =
            serde_json::json!({"command": "anvil", "args": ["mcp", "serve", "--stdio"], "env": {}});
        let b = serde_json::json!({"command": "/usr/local/bin/anvil", "args": ["mcp", "serve"], "env": {}});
        assert!(!entries_equivalent(&a, &b));
    }

    #[test]
    fn entries_equivalent_rejects_different_env() {
        let a =
            serde_json::json!({"command": "anvil", "args": ["mcp", "serve", "--stdio"], "env": {}});
        let b = serde_json::json!({"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {"FOO": "bar"}});
        assert!(!entries_equivalent(&a, &b));
    }

    #[test]
    fn classify_drift_by_args_blocks_foreign_command_with_matching_args() {
        // Council finding: a foreign command like /bin/bash with our
        // canonical args was previously classified as SafeDrift and
        // would have been overwritten by the install path.
        let foreign = serde_json::json!({
            "command": "/bin/bash",
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
        });
        let fresh = AnvilEntry::local_stdio(std::path::PathBuf::from("/usr/local/bin/anvil"));
        match classify_drift_by_args(&foreign, &fresh) {
            DriftClass::UnsafeDrift { reason } => {
                assert!(reason.contains("/bin/bash"));
                assert!(reason.contains("does not look like anvil"));
            }
            other => panic!("expected UnsafeDrift, got {other:?}"),
        }
    }

    #[test]
    fn classify_drift_by_args_allows_same_args_with_anvil_basename() {
        // Same canonical args + anvil-shaped command (different prefix)
        // = SafeDrift (legitimate version upgrade).
        let drift = serde_json::json!({
            "command": "/nix/store/abc/bin/anvil",
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
        });
        let fresh = AnvilEntry::local_stdio(std::path::PathBuf::from("/usr/local/bin/anvil"));
        match classify_drift_by_args(&drift, &fresh) {
            DriftClass::SafeDrift { .. } => {}
            other => panic!("expected SafeDrift, got {other:?}"),
        }
    }

    #[test]
    fn probe_with_unreadable_workspace_does_not_fall_through_to_home() {
        // Council finding: previously, an I/O error at workspace scope
        // (e.g. permission denied) silently fell through to home,
        // hiding the broken workspace state. Now: workspace I/O error
        // returns ConfigPresent immediately, preserving precedence.
        // Simulate by creating an unreadable file. Note: we can't
        // actually chmod 0 in a portable way for tempdirs, so we
        // assert the structural property: a workspace file exists →
        // home is never reached.
        // (The I/O-error-specifically path is harder to test
        // portably; the parse-error path exercises the same
        // early-return logic and is covered below.)
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // Workspace has malformed JSON; home has a valid anvil entry.
        fs::create_dir_all(ws.path().join(".cursor")).unwrap();
        fs::write(ws.path().join(".cursor/mcp.json"), "{not json").unwrap();
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let home_cfg = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        fs::write(home.path().join(".cursor/mcp.json"), home_cfg).unwrap();

        let map = probe_all(ws.path(), Some(home.path()), &fresh());
        // Workspace parse-error MUST surface as ConfigPresent (not
        // home's RestartRequired) because workspace shadows home when
        // workspace exists.
        assert_eq!(map[&McpClientId::Cursor].tier, McpTier::ConfigPresent);
    }

    // ---------------------------------------------------------------
    // Spawn-probe handshake validator (LAUNCH-009.5)
    // ---------------------------------------------------------------

    #[test]
    fn validate_initialize_accepts_a_well_formed_success_response() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"anvil","version":"0.5.1"}}}"#;
        let evidence =
            validate_initialize_response(raw).expect("well-formed response should validate");
        assert_eq!(evidence.protocol_era, ProtocolEraEvidence::Legacy);
        assert_eq!(evidence.protocol_version, "2025-06-18");
        assert_eq!(evidence.verification_method, VerificationMethod::Initialize);
    }

    #[test]
    fn validate_discover_accepts_modern_anvil_discovery() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"resultType":"complete","supportedVersions":["2026-07-28"],"capabilities":{"tools":{},"resources":{}},"ttlMs":3600000,"cacheScope":"private","_meta":{"io.modelcontextprotocol/serverInfo":{"name":"anvil","version":"0.9.0-beta"}}}}"#;
        let evidence = validate_discover_response(raw).expect("modern discover should validate");
        assert_eq!(evidence.protocol_era, ProtocolEraEvidence::Modern);
        assert_eq!(evidence.protocol_version, "2026-07-28");
        assert_eq!(
            evidence.verification_method,
            VerificationMethod::ServerDiscover
        );
    }

    #[test]
    fn validate_discover_reports_the_requested_supported_version() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"resultType":"complete","supportedVersions":["2099-01-01","2026-07-28"],"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"anvil","version":"0.9.0-beta"}}}}"#;
        let evidence = validate_discover_response(raw).expect("modern discover should validate");
        assert_eq!(evidence.protocol_version, PROBE_MODERN_PROTOCOL_VERSION);
    }

    #[test]
    fn validate_discover_rejects_mixed_type_supported_versions() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"resultType":"complete","supportedVersions":["2026-07-28",7],"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"anvil","version":"0.9.0-beta"}}}}"#;
        let err = validate_discover_response(raw).expect_err("mixed types must fail");
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("entries must be strings")),
            other => panic!("expected BadResponse, got {other:?}"),
        }
    }

    #[test]
    fn validate_discover_rejects_legacy_shaped_server_info_only() {
        // Council: top-level serverInfo must not count as modern discover.
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"anvil","version":"0.9.0-beta"}}}"#;
        let err = validate_discover_response(raw).expect_err("legacy-shaped body must fail");
        match &err {
            ProbeError::BadResponse(s) => {
                assert!(s.contains("resultType") || s.contains("_meta"), "got {s}");
            }
            other => panic!("expected BadResponse, got {other:?}"),
        }
        // Non-modern shaped response should fall through to legacy initialise.
        assert!(modern_probe_should_fallback(&err));
    }

    #[test]
    fn validate_discover_rejects_non_anvil_server() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"resultType":"complete","supportedVersions":["2026-07-28"],"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"other","version":"1"}}}}"#;
        let err = validate_discover_response(raw).expect_err("non-anvil must fail");
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("anvil")),
            other => panic!("expected BadResponse, got {other:?}"),
        }
    }

    #[test]
    fn validate_discover_method_not_found_is_fallback_shaped() {
        let raw =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let err = validate_discover_response(raw).expect_err("method not found must fail");
        assert!(modern_probe_should_fallback(&err));
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("non-modern")),
            other => panic!("expected BadResponse, got {other:?}"),
        }
    }

    #[test]
    fn validate_discover_unsupported_version_does_not_fallback() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32022,"message":"Unsupported protocol version","data":{"supported":["2026-07-28"],"requested":"2099-01-01"}}}"#;
        let err = validate_discover_response(raw).expect_err("unsupported must fail");
        assert!(!modern_probe_should_fallback(&err));
    }

    #[test]
    fn validate_initialize_rejects_garbage_json() {
        let err =
            validate_initialize_response("not even json").expect_err("garbage must be rejected");
        assert!(matches!(err, ProbeError::ParseResponse(_)), "got {err:?}");
    }

    #[test]
    fn validate_initialize_rejects_missing_jsonrpc_version() {
        let raw = r#"{"id":1,"result":{"serverInfo":{}}}"#;
        let err = validate_initialize_response(raw).expect_err("missing jsonrpc must fail");
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("jsonrpc")),
            other => panic!("expected BadResponse(missing jsonrpc), got {other:?}"),
        }
    }

    #[test]
    fn validate_initialize_rejects_wrong_id() {
        let raw = r#"{"jsonrpc":"2.0","id":42,"result":{"serverInfo":{}}}"#;
        let err = validate_initialize_response(raw).expect_err("wrong id must fail");
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("id=1") || s.contains("expected id")),
            other => panic!("expected BadResponse(wrong id), got {other:?}"),
        }
    }

    #[test]
    fn validate_initialize_rejects_jsonrpc_error_response() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let err = validate_initialize_response(raw).expect_err("error response must fail");
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("error")),
            other => panic!("expected BadResponse(error), got {other:?}"),
        }
    }

    #[test]
    fn validate_initialize_rejects_missing_serverinfo() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let err = validate_initialize_response(raw).expect_err("missing serverInfo must fail");
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("serverInfo")),
            other => panic!("expected BadResponse(serverInfo), got {other:?}"),
        }
    }

    #[test]
    fn validate_initialize_rejects_non_anvil_server() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"other-mcp","version":"1.0.0"}}}"#;
        let err = validate_initialize_response(raw).expect_err("non-anvil server must fail");
        match err {
            ProbeError::BadResponse(s) => assert!(s.contains("serverInfo.name=anvil")),
            other => panic!("expected BadResponse(serverInfo.name), got {other:?}"),
        }
    }

    #[test]
    fn probe_frame_reader_rejects_frames_over_stdio_limit() {
        let oversized_len = usize::try_from(crate::commands::mcp::MAX_STDIO_FRAME_BYTES + 1)
            .expect("frame limit fits usize");
        let mut reader = std::io::Cursor::new(vec![b'x'; oversized_len]);

        let err = read_probe_frame(&mut reader).expect_err("oversized frame must fail closed");

        assert!(matches!(err, ProbeError::OversizedFrame));
    }

    #[cfg(unix)]
    #[test]
    fn probe_startable_fails_when_command_does_not_exist() {
        // ENOENT path: the binary does not exist, spawn fails.
        let entry = AnvilEntry::Stdio {
            command: std::path::PathBuf::from("/nonexistent/path/to/anvil-shim"),
            args: vec!["mcp".into(), "serve".into(), "--stdio".into()],
            env: BTreeMap::new(),
        };
        let err = probe_startable(&entry).expect_err("nonexistent command must fail");
        assert!(matches!(err, ProbeError::Spawn(_)), "got {err:?}");
    }

    #[cfg(unix)]
    #[test]
    fn probe_startable_times_out_when_child_does_not_respond() {
        // /bin/cat reads stdin and echoes — but it won't produce a
        // valid initialize response. Since cat echoes our request,
        // the response IS our request which is NOT a valid response,
        // so this exercises the BadResponse path. Use /bin/sleep for
        // a true timeout: it never reads or writes anything.
        let entry = AnvilEntry::Stdio {
            command: std::path::PathBuf::from("/bin/sleep"),
            args: vec!["10".into()],
            env: BTreeMap::new(),
        };
        let start = std::time::Instant::now();
        let err = probe_startable(&entry).expect_err("non-responsive child must fail");
        let elapsed = start.elapsed();
        assert!(matches!(err, ProbeError::Timeout), "got {err:?}");
        // Dual-era probe: modern timeout (1s) then fresh legacy timeout (1s).
        // Allow slack for slow CI; fail if far longer (kill/recv stuck).
        assert!(
            elapsed < std::time::Duration::from_secs(4),
            "probe should return within 2×1s timeout + slack, took {elapsed:?}",
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(1500),
            "expected both modern and legacy attempts (~2s), took {elapsed:?}",
        );
    }

    #[cfg(unix)]
    #[test]
    fn probe_startable_rejects_child_that_responds_with_garbage() {
        // /bin/echo prints its argument and exits — the line on
        // stdout is not a JSON-RPC response.
        let entry = AnvilEntry::Stdio {
            command: std::path::PathBuf::from("/bin/echo"),
            args: vec!["not-jsonrpc".into()],
            env: BTreeMap::new(),
        };
        let err = probe_startable(&entry).expect_err("garbage line must fail");
        // Could be Write if /bin/echo exits before stdin is written,
        // ParseResponse if echo prints "not-jsonrpc", or BadResponse
        // depending on how /bin/echo's output parses.
        assert!(
            matches!(
                err,
                ProbeError::Write(_) | ProbeError::ParseResponse(_) | ProbeError::BadResponse(_)
            ),
            "got {err:?}",
        );
    }

    #[cfg(unix)]
    #[test]
    fn probe_startable_rejects_child_that_exits_immediately() {
        // `true` exits with status 0 producing no output. The reader thread
        // sees EOF immediately, sending an empty line. Resolve the binary
        // through `which::which` rather than hardcoding `/bin/true` —
        // macOS GitHub Actions Cross runners surfaced `os error 2` when the
        // test pinned `/bin/true` because PATH-based resolution behaves
        // more predictably across OSes than absolute hard-coding. The
        // probe itself still spawns whatever `PathBuf` the resolver
        // returns; this test fixture change is purely about resilience to
        // the runner environment.
        let command = which::which("true").expect("`true` exists on PATH on every Unix host");
        let entry = AnvilEntry::Stdio {
            command,
            args: vec![],
            env: BTreeMap::new(),
        };
        let err = probe_startable(&entry).expect_err("immediate exit must fail");
        assert!(
            matches!(err, ProbeError::Write(_) | ProbeError::EmptyResponse),
            "got {err:?}",
        );
    }

    /// MCP26-007: the current anvil binary must verify via modern discovery.
    #[test]
    fn probe_startable_succeeds_against_current_anvil_binary() {
        let command = std::env::var_os("CARGO_BIN_EXE_anvil").map(std::path::PathBuf::from);
        let Some(command) = command else {
            // Integration coverage lives in tests/ when the env is absent.
            eprintln!("skipping: CARGO_BIN_EXE_anvil not set in this harness");
            return;
        };
        let entry = AnvilEntry::Stdio {
            command,
            args: vec!["mcp".into(), "serve".into(), "--stdio".into()],
            env: BTreeMap::new(),
        };
        let evidence = probe_startable(&entry).expect("current anvil must probe successfully");
        assert_eq!(evidence.protocol_era, ProtocolEraEvidence::Modern);
        assert_eq!(
            evidence.verification_method,
            VerificationMethod::ServerDiscover
        );
        assert_eq!(evidence.protocol_version, "2026-07-28");
    }
}
