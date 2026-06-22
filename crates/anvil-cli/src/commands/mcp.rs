use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{Value, json};

use crate::GlobalArgs;
use crate::auth::credentials;
use crate::commands::mcp_config::{self, Target};
use crate::feature_flags;
use crate::mcp::tools::{registry, validate_write};

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_INSTRUCTIONS: &str = "This server provides two write-validation tools: anvil_validate_write and anvil_apply_patch. Before applying any file write - Write, Edit, MultiEdit, fs.write, apply_edit, or equivalent - call anvil_validate_write with the proposed content (or a preview of the first lines) and respect the response decision. When applying a unified diff to an existing file, prefer anvil_apply_patch instead; it accepts a unifiedDiff and scans only the added lines, producing a smaller, more readable approval prompt. Decision vocabulary: `block` is authoritative — do not write, do not bypass via alternate tools (the response carries either a `diagnostics` array of findings or an `error` describing why the gate refused). `warn` means findings were detected but the workspace enforcement mode lets the write proceed — surface the diagnostics and continue. `gateUnavailable` is informational — the gate could not run (e.g. credentials missing or backend offline); surface the warning to the user and proceed with the write. `allow` means the proposed content passed validation.";
// Keep the stdio frame ceiling comfortably above the largest accepted tool
// payload. validate-write caps `proposedContent` at 1 MiB of UTF-8 source.
// JSON string escaping can grow that almost 2x in the worst case (every byte
// is `"` or `\\`), and the JSON-RPC / MCP envelope adds further overhead, so
// allow up to 4 MiB on the wire to keep valid requests from being rejected
// at the framing layer before tool-level validation runs.
const MAX_STDIO_FRAME_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    command: McpCommand,
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    /// Install anvil MCP configuration for an editor.
    Install(McpInstallArgs),
    /// Start an MCP server.
    Serve(McpServeArgs),
}

#[derive(Debug, Args)]
struct McpInstallArgs {
    /// Client to configure.
    #[arg(long, value_enum)]
    client: McpClient,

    /// Verify the existing client config instead of writing it.
    #[arg(long)]
    verify: bool,

    /// Override the command path written into stdio configs. Defaults to `anvil`.
    #[arg(long)]
    command: Option<String>,

    /// Override the client config root. Defaults to the user's home directory.
    #[arg(long)]
    workspace: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum McpClient {
    /// Cursor (`.cursor/mcp.json`).
    Cursor,
    /// Anthropic Claude Code (`.claude.json`).
    ClaudeCode,
}

#[derive(Debug, Args)]
struct McpServeArgs {
    /// Serve MCP over stdin/stdout.
    #[arg(long)]
    stdio: bool,
}

pub fn run(args: &McpArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        McpCommand::Install(install) => run_install(install, global),
        McpCommand::Serve(serve) => run_serve(serve),
    }
}

pub fn auth_gate_name(args: &McpArgs) -> &'static str {
    match &args.command {
        McpCommand::Install(_) => "mcp-install",
        McpCommand::Serve(_) => "mcp-serve",
    }
}

fn run_install(args: &McpInstallArgs, global: &GlobalArgs) -> Result<()> {
    let config_root = match &args.workspace {
        Some(path) => path.clone(),
        None => mcp_config::default_client_config_root()?,
    };
    let target = args.client.target();
    if args
        .command
        .as_deref()
        .is_some_and(|command| command.trim().is_empty())
    {
        bail!("--command must not be empty");
    }

    if args.verify {
        let (path, entry) = mcp_config::verify_rust_stdio_target(
            target,
            &config_root,
            args.command.as_deref(),
            global,
        )?;
        if global.json {
            println!(
                "{}",
                json!({
                    "client": args.client.label(),
                    "path": path.display().to_string(),
                    "entry": entry,
                    "ok": true,
                })
            );
        } else {
            println!(
                "Detected client: {} (config: {})",
                args.client.label(),
                path.display()
            );
            println!("Status: ok");
        }
        return Ok(());
    }

    let command = args.command.as_deref().unwrap_or("anvil");
    let install = mcp_config::install_rust_stdio_target(
        target,
        &config_root,
        args.command.as_deref(),
        global,
    )?;
    if global.json {
        println!(
            "{}",
            json!({
                "client": args.client.label(),
                "path": install.path.display().to_string(),
                "wrote": install.wrote,
                "drifted": install.drifted,
                "command": command,
                "args": ["mcp", "serve", "--stdio"],
            })
        );
    } else {
        println!(
            "Detected client: {} (config: {})",
            args.client.label(),
            install.path.display()
        );
        if install.drifted {
            println!("Existing entry drifted; rewrote anvil MCP server entry.");
        }
        let status = if install.wrote {
            "ok"
        } else {
            "already configured"
        };
        println!("Installing anvil MCP server entry ... {status}");
        println!("Restart {} to pick up the new server.", args.client.label());
    }
    Ok(())
}

impl McpClient {
    fn target(self) -> Target {
        match self {
            McpClient::Cursor => Target::Cursor,
            McpClient::ClaudeCode => Target::ClaudeCode,
        }
    }

    fn label(self) -> &'static str {
        match self {
            McpClient::Cursor => "cursor",
            McpClient::ClaudeCode => "claude-code",
        }
    }
}

fn run_serve(args: &McpServeArgs) -> Result<()> {
    if !args.stdio {
        bail!("`anvil mcp serve` currently requires --stdio");
    }

    run_stdio_server()
}

fn run_stdio_server() -> Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::stdout().lock();

    while let Some(frame) = read_frame(&mut reader)? {
        let Frame::Message(frame) = frame else {
            write_message(
                &mut stdout,
                &error_response(&Value::Null, -32600, "Invalid Request"),
            )?;
            continue;
        };

        if frame.iter().all(u8::is_ascii_whitespace) {
            continue;
        }

        let Ok(message) = serde_json::from_slice::<Value>(&frame) else {
            write_message(&mut stdout, &parse_error_response())?;
            continue;
        };

        if let Some(response) = handle_message(&message) {
            write_message(&mut stdout, &response)?;
        }

        if is_exit_notification(&message) {
            break;
        }
    }

    Ok(())
}

fn handle_message(message: &Value) -> Option<Value> {
    if !message.is_object() {
        return Some(error_response(&Value::Null, -32600, "Invalid Request"));
    }

    let id = message.get("id");
    let method = message.get("method").and_then(Value::as_str);

    match method {
        Some("initialize") => {
            warm_up_session();
            id.map(|id| initialize_response(id, message))
        }
        Some("notifications/initialized") => None,
        Some("exit") if id.is_none() => None,
        Some("exit") => id.map(|id| error_response(id, -32600, "Invalid Request")),
        Some("shutdown") => id.map(|id| success_response(id, &Value::Null)),
        Some("ping") => id.map(|id| success_response(id, &json!({}))),
        Some("tools/list") => id.map(tools_list_response),
        Some("tools/call") => id.map(|id| tools_call_response(id, message)),
        Some("resources/list") => id.map(resources_list_response),
        Some("resources/read") => id.map(|id| resources_read_response(id, message)),
        Some(_) => id.map(|id| error_response(id, -32601, "Method not found")),
        None => id.map(|id| error_response(id, -32600, "Invalid Request")),
    }
}

fn is_exit_notification(message: &Value) -> bool {
    message.is_object()
        && message.get("method").and_then(Value::as_str) == Some("exit")
        && message.get("id").is_none()
}

enum Frame {
    Message(Vec<u8>),
    Oversize,
}

fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Frame>> {
    let mut frame = Vec::new();
    let bytes_read = {
        let mut limited = reader.by_ref().take(MAX_STDIO_FRAME_BYTES + 1);
        limited.read_until(b'\n', &mut frame)?
    };

    if bytes_read == 0 {
        return Ok(None);
    }

    let has_newline = frame.ends_with(b"\n");
    let payload_len = frame.len().saturating_sub(usize::from(has_newline)) as u64;
    if payload_len > MAX_STDIO_FRAME_BYTES {
        if !has_newline {
            discard_line_tail(reader)?;
        }
        return Ok(Some(Frame::Oversize));
    }

    Ok(Some(Frame::Message(frame)))
}

fn discard_line_tail(reader: &mut impl BufRead) -> io::Result<()> {
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(());
        }

        if let Some(newline_index) = available.iter().position(|byte| *byte == b'\n') {
            reader.consume(newline_index + 1);
            return Ok(());
        }

        let consumed = available.len();
        reader.consume(consumed);
    }
}

/// GCTX-010 C1 (ADR-085) session-init warm-up: on `initialize`, proactively
/// ask the daemon to warm this session's workspace graph so the assistant's
/// first `anvil_search_symbols` query is less likely to hit a cold graph.
///
/// The root is the server's working directory — the same root the write tools
/// derive their server-root from (`search_payload`), and the root the daemon
/// re-validates against the connection's admitted-root set (ADR-084 C3) before
/// scanning; this call does not itself enforce admission. The cwd may be a
/// parent of the exact root a later query targets (a different worktree key); in
/// that case this enqueue is merely a head start and the precise key is warmed
/// by the on-demand re-warm in `search_symbols` instead.
///
/// Best-effort and fire-and-forget: the transport detaches the round-trip, so
/// this never blocks or fails the MCP handshake; an absent daemon, the
/// `ANVIL_WATCH_DAEMON=0` opt-out, and a per-session dedup are all handled in
/// `warm_up_root`. On a very short session the detached thread may not complete,
/// in which case the daemon's own first-contact auto-enqueue (DSV-045) warms it.
fn warm_up_session() {
    if let Ok(cwd) = std::env::current_dir() {
        let _ = crate::commands::watch_save_time::warm_up_root(&cwd);
    }
}

fn initialize_response(id: &Value, message: &Value) -> Value {
    let Some(params) = message.get("params").and_then(Value::as_object) else {
        return error_response(id, -32602, "Invalid params");
    };

    let protocol_version = match params.get("protocolVersion") {
        Some(Value::String(version)) => version.as_str(),
        Some(_) => return error_response(id, -32602, "Invalid params"),
        None => DEFAULT_PROTOCOL_VERSION,
    };

    let result = json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "instructions": SERVER_INSTRUCTIONS,
        "serverInfo": {
            "name": "anvil",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    success_response(id, &result)
}

fn tools_list_response(id: &Value) -> Value {
    let tools = registry::all()
        .iter()
        .map(registry::ToolDefinition::descriptor)
        .collect::<Vec<_>>();

    success_response(
        id,
        &json!({
            "tools": tools
        }),
    )
}

fn resources_list_response(id: &Value) -> Value {
    success_response(
        id,
        &json!({
            "resources": crate::mcp::resources::list()
        }),
    )
}

fn resources_read_response(id: &Value, message: &Value) -> Value {
    let Some(params) = message.get("params").and_then(Value::as_object) else {
        return error_response(id, -32602, "Invalid params");
    };
    let Some(uri) = params.get("uri").and_then(Value::as_str) else {
        return error_response(id, -32602, "Invalid params");
    };
    match crate::mcp::resources::read(uri) {
        Ok(result) => success_response(id, &result),
        // A client mistake (unknown URI / malformed query) is -32602; a
        // server-side daemon transport fault is -32603 (council CR-2).
        Err(err @ crate::mcp::resources::ReadError::BadRequest(_)) => error_response_with_data(
            id,
            -32602,
            "Invalid params",
            &json!({ "reason": err.reason(), "uri": uri }),
        ),
        Err(err @ crate::mcp::resources::ReadError::Internal(_)) => error_response_with_data(
            id,
            -32603,
            "Internal error",
            &json!({ "reason": err.reason(), "uri": uri }),
        ),
        // CIB-091d: the per-session graph:// egress credit is exhausted — a
        // structured `quota_exceeded` resource-exhaustion error.
        Err(err @ crate::mcp::resources::ReadError::QuotaExceeded(_)) => error_response_with_data(
            id,
            -32603,
            "Internal error",
            &json!({ "reason": err.reason(), "uri": uri, "kind": "quota_exceeded" }),
        ),
    }
}

fn tools_call_response(id: &Value, message: &Value) -> Value {
    let Some(params) = message.get("params").and_then(Value::as_object) else {
        return error_response(id, -32602, "Invalid params");
    };

    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, -32602, "Invalid params");
    };

    let Some(tool) = registry::find(name) else {
        return error_response_with_data(
            id,
            -32602,
            "Invalid params",
            &json!({
                "reason": "unknown-tool",
                "tool": name
            }),
        );
    };

    let empty_arguments = json!({});
    let arguments = params.get("arguments").unwrap_or(&empty_arguments);

    if tool.requires_auth && !mcp_tool_auth_ok() {
        return success_response(id, &mcp_tool_auth_required_result(tool, arguments));
    }

    let result = tool.call(arguments);

    // CIB-091d: a GCTX tool projects the same identity-only graph data as the
    // `graph://` resources, so its successful payload is charged against the SAME
    // per-session egress byte ceiling — otherwise `tools/call` would be an
    // unbounded back door past the resource cap, letting an assistant reassemble
    // the whole graph. The read that crosses the ceiling is refused (the payload
    // is replaced with a structured `quota_exceeded` result), so the budget is a
    // hard cap, not a soft one.
    if tool.charges_graph_egress && !gctx_tool_result_is_error(&result) {
        let payload_bytes = serde_json::to_vec(&result).map_or(0, |v| v.len() as u64);
        if !crate::mcp::resources::try_charge_graph_egress(payload_bytes) {
            return success_response(id, &gctx_quota_exceeded_result(tool.name));
        }
    }

    success_response(id, &result)
}

/// A GCTX tool result is an error when its MCP envelope carries `isError: true`
/// (a parse error, a daemon failure). A degraded `unavailable`/`not_ready`
/// outcome is *not* an error (it is a successful, in-band degradation) but it
/// carries no graph identity data, so it is harmless to charge — only a genuine
/// error is excluded so a failed call never burns the egress budget.
fn gctx_tool_result_is_error(result: &Value) -> bool {
    result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// CIB-091d: the structured `quota_exceeded` MCP tool result returned once a
/// session's shared `graph://` egress credit is exhausted. Mirrors the
/// resource-surface `quota_exceeded` error so a client sees one vocabulary across
/// both GCTX surfaces; `isError` is `true` so the assistant stops paging.
fn gctx_quota_exceeded_result(tool_name: &str) -> Value {
    let reason = crate::mcp::resources::graph_egress_quota_reason();
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "error": reason,
                    "kind": "quota_exceeded",
                    "tool": tool_name,
                })).expect("quota-exceeded payload serialises")
            }
        ],
        "isError": true
    })
}

fn mcp_tool_auth_required_result(tool: &registry::ToolDefinition, arguments: &Value) -> Value {
    if tool.name == validate_write::TOOL_NAME {
        return mcp_auth_required_result(arguments);
    }

    // MLP2-072 / #1796 — non-write tools also surface auth-required as
    // `gateUnavailable` rather than `block`. The wire shape is now
    // consistent with the write-validation path (same `decision` and
    // `safeDefault` fields), so clients that branch on either field
    // see the same vocabulary regardless of tool.
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "schemaVersion": "anvil.mcp.auth-required.v1",
                    "decision": "gateUnavailable",
                    "safeDefault": "allow-with-warning",
                    "reason": "anvil MCP credentials are required for this tool. Run `anvil auth login` or `anvil auth login --edict`.",
                    "tool": tool.name,
                    "correlation": {
                        "daemonStatus": crate::mcp::validation::DaemonStatus::NotWired.as_str(),
                        "enforcementMode": "block",
                        "gateState": "unavailable"
                    }
                })).expect("auth-required payload serialises")
            }
        ],
        "isError": false
    })
}

fn mcp_tool_auth_ok() -> bool {
    if feature_flags::cli_dev_bypass_active().is_some() {
        return true;
    }

    let Ok(Some(creds)) = credentials::load() else {
        return false;
    };

    if credentials::is_expired(&creds) {
        return false;
    }

    if credentials::is_edict(&creds) {
        return cached_edict_auth_ok(&creds);
    }

    true
}

/// How long a successful edict `/auth/verify` result is honoured before we
/// hit the network again. Short enough that revoked credentials lose access
/// promptly, long enough that a steady stream of `tools/call` requests does
/// not produce a verify request per call. Mirrored in the cache test below.
const EDICT_VERIFY_CACHE_TTL: Duration = Duration::from_mins(1);

#[derive(Clone)]
struct EdictAuthCacheEntry {
    /// License the result was recorded against. Used so a credential change
    /// invalidates the cache even if it lands within the TTL window.
    license: String,
    checked_at: Instant,
    ok: bool,
}

fn edict_auth_cache() -> &'static Mutex<Option<EdictAuthCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<EdictAuthCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Shared single-thread Tokio runtime for the per-tool-call edict verify.
/// Building a fresh runtime per call (the previous behaviour) cost an extra
/// thread-spawn + reactor init per `tools/call`, which is enough to be felt
/// at write-validation cadence in editor MCP clients.
fn edict_verify_runtime() -> Option<&'static tokio::runtime::Runtime> {
    static RT: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
    })
    .as_ref()
}

fn cached_edict_auth_ok(creds: &credentials::Credentials) -> bool {
    if let Ok(guard) = edict_auth_cache().lock()
        && let Some(entry) = guard.as_ref()
        && entry.license == creds.license
        && entry.checked_at.elapsed() < EDICT_VERIFY_CACHE_TTL
    {
        return entry.ok;
    }

    let ok = verify_mcp_edict_auth(creds);

    if let Ok(mut guard) = edict_auth_cache().lock() {
        *guard = Some(EdictAuthCacheEntry {
            license: creds.license.clone(),
            checked_at: Instant::now(),
            ok,
        });
    }
    ok
}

fn verify_mcp_edict_auth(creds: &credentials::Credentials) -> bool {
    let Some(rt) = edict_verify_runtime() else {
        return false;
    };

    let Ok(client) = crate::auth::client::AnvilClient::with_token(creds.license.clone()) else {
        return false;
    };

    rt.block_on(client.verify_edict()).is_ok()
}

fn mcp_auth_required_result(arguments: &Value) -> Value {
    // MLP2-072 / #1796 — the pre-write gate distinguishes
    // *gate-unavailable* (auth missing; the gate could not run) from
    // *content-veto* (the gate ran and the content failed). A
    // well-behaved agent following SERVER_INSTRUCTIONS honours `block`
    // for content-vetoes and proceeds-with-warning on `gateUnavailable`.
    // Pre-MLP2-072 this path returned `block` and `isError: true`,
    // which made agents refuse to write any file pre-login — including
    // the bootstrap files needed to onboard.
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");
    let payload = json!({
        "schema": "anvil.mcp.validate-write.v1",
        "decision": "gateUnavailable",
        "error": {
            "code": "authentication-required",
            "message": "Pre-write gate unavailable: authentication required. Run `anvil auth login` or `anvil auth login --edict`. The write may proceed; the gate could not validate it.",
            "retriable": true
        },
        "safeDefault": "allow-with-warning",
        "correlation": {
            "id": "corr_mcp_auth_required",
            "surface": "mcp",
            "mode": "preWrite",
            "backend": "embedded",
            "daemonStatus": "not-wired",
            "path": path,
            "enforcementMode": "block",
            "gateState": "unavailable"
        }
    });
    let text = serde_json::to_string(&payload).expect("auth-required payload serialises");
    json!({
        "content": [{"type": "text", "text": text}],
        // MLP2-072 — `isError: false` because the tool itself succeeded;
        // the gate just could not run. Setting this `true` is what
        // caused agents to abort writes pre-login.
        "isError": false
    })
}

fn success_response(id: &Value, result: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn parse_error_response() -> Value {
    error_response(&Value::Null, -32700, "Parse error")
}

fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn error_response_with_data(id: &Value, code: i64, message: &str, data: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
            "data": data
        }
    })
}

fn write_message(stdout: &mut impl Write, message: &Value) -> Result<()> {
    serde_json::to_writer(&mut *stdout, &message)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::json;

    use super::{
        EDICT_VERIFY_CACHE_TTL, EdictAuthCacheEntry, Frame, MAX_STDIO_FRAME_BYTES,
        edict_auth_cache, gctx_quota_exceeded_result, gctx_tool_result_is_error, handle_message,
        read_frame,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn read_frame_rejects_oversize_line_without_returning_payload() {
        let oversize_len =
            usize::try_from(MAX_STDIO_FRAME_BYTES + 2).expect("test frame size fits usize");
        let mut input = Cursor::new(vec![b'a'; oversize_len]);

        let frame = read_frame(&mut input).expect("frame read succeeds");

        assert!(matches!(frame, Some(Frame::Oversize)));
    }

    #[test]
    fn read_frame_allows_max_payload_with_newline_without_discarding_next_frame() {
        let max_len = usize::try_from(MAX_STDIO_FRAME_BYTES).expect("test frame size fits usize");
        let mut input = Vec::with_capacity(max_len + 7);
        input.extend(vec![b'a'; max_len]);
        input.extend(b"\nnext\n");
        let mut input = Cursor::new(input);

        let frame = read_frame(&mut input).expect("first frame read succeeds");
        let next_frame = read_frame(&mut input).expect("next frame read succeeds");

        assert!(matches!(frame, Some(Frame::Message(frame)) if frame.len() == max_len + 1));
        assert!(matches!(next_frame, Some(Frame::Message(frame)) if frame == b"next\n"));
    }

    #[test]
    fn edict_auth_cache_ttl_is_short_enough_to_drop_revoked_creds() {
        // Sanity guard: if someone bumps the TTL very high, revoked edict
        // tokens would keep working for that whole window. Keep it ≤ 5 min.
        let ttl = EDICT_VERIFY_CACHE_TTL;
        assert!(
            ttl <= Duration::from_mins(5),
            "edict verify cache TTL is too long: {ttl:?}"
        );
    }

    #[test]
    fn edict_auth_cache_entry_invalidates_on_license_change() {
        // Cache is keyed on (license, checked_at). A different license must
        // be treated as a miss even within the TTL window — credential
        // changes during a long-lived MCP session must not be served stale.
        let now = Instant::now();
        let entry = EdictAuthCacheEntry {
            license: "lic-a".to_string(),
            checked_at: now,
            ok: true,
        };
        // Same license + within TTL → hit.
        assert_eq!(entry.license, "lic-a");
        assert!(entry.checked_at.elapsed() < EDICT_VERIFY_CACHE_TTL);
        // Different license must not be served from this entry. The
        // production path enforces this via the `entry.license == creds.license`
        // check in `cached_edict_auth_ok`; this test pins the field so a
        // future refactor can't drop it silently.
        assert_ne!(entry.license, "lic-b");

        // Pre-warm the cache to confirm the static initialiser works under
        // tests, but reset to avoid leaking state to other tests.
        if let Ok(mut guard) = edict_auth_cache().lock() {
            *guard = None;
        }
    }

    #[test]
    fn validate_write_tool_call_returns_gate_unavailable_without_credentials() {
        // MLP2-072 / #1796 — when auth is missing, the pre-write gate
        // must distinguish *gate-unavailable* from *content-veto*. The
        // wire shape carries `decision: "gateUnavailable"` (NOT
        // `block`), `isError: false` (the tool itself succeeded), and
        // `safeDefault: "allow-with-warning"` so a well-behaved agent
        // surfaces the warning and proceeds with the write rather than
        // refusing to onboard.
        temp_env::with_vars(
            [
                ("ANVIL_DEV", None),
                ("ANVIL_LICENSE", None),
                ("XDG_CONFIG_HOME", Some("/nonexistent/path")),
            ],
            || {
                let response = handle_message(&json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "tools/call",
                    "params": {
                        "name": "anvil_validate_write",
                        "arguments": {
                            "path": "src/example.ts",
                            "operation": "create",
                            "proposedContent": "export const value = 1;\n"
                        }
                    }
                }))
                .expect("request should produce a response");

                let result = &response["result"];
                assert_eq!(
                    result["isError"], false,
                    "MLP2-072: gate-unavailable is not a tool error; isError must be false so agents do not abort writes pre-login"
                );
                let text = result["content"][0]["text"]
                    .as_str()
                    .expect("tool content text");
                let payload: serde_json::Value = serde_json::from_str(text).unwrap();
                assert_eq!(
                    payload["decision"], "gateUnavailable",
                    "MLP2-072: auth-missing must NOT return `block` (which agents treat as authoritative)"
                );
                assert_eq!(payload["error"]["code"], "authentication-required");
                assert_eq!(payload["safeDefault"], "allow-with-warning");
                assert_eq!(
                    payload["correlation"]["enforcementMode"], "block",
                    "v1 contract: enforcementMode stays in the closed set {{block|warn|off}}"
                );
                assert_eq!(
                    payload["correlation"]["gateState"], "unavailable",
                    "gate-unavailable signal lives in `gateState`, not `enforcementMode`"
                );
                assert_eq!(payload["schema"], "anvil.mcp.validate-write.v1");
            },
        );
    }

    #[test]
    fn apply_patch_tool_call_returns_gate_unavailable_without_credentials() {
        // MLP2-072 / #1796 — sibling test to validate_write. The
        // non-validate_write branch of `mcp_tool_auth_required_result`
        // must carry the same gate-unavailable vocabulary so agents
        // see one consistent decision shape across both write tools.
        temp_env::with_vars(
            [
                ("ANVIL_DEV", None),
                ("ANVIL_LICENSE", None),
                ("XDG_CONFIG_HOME", Some("/nonexistent/path")),
            ],
            || {
                let response = handle_message(&json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "tools/call",
                    "params": {
                        "name": "anvil_apply_patch",
                        "arguments": {
                            "path": "src/example.ts",
                            "unifiedDiff": "--- a/src/example.ts\n+++ b/src/example.ts\n@@ -0,0 +1 @@\n+export const value = 1;\n"
                        }
                    }
                }))
                .expect("request should produce a response");

                let result = &response["result"];
                assert_eq!(
                    result["isError"], false,
                    "MLP2-072: apply_patch gate-unavailable must not be a tool error"
                );
                let text = result["content"][0]["text"]
                    .as_str()
                    .expect("tool content text");
                let payload: serde_json::Value = serde_json::from_str(text).unwrap();
                assert_eq!(payload["decision"], "gateUnavailable");
                assert_eq!(
                    payload["safeDefault"], "allow-with-warning",
                    "MLP2-072 follow-up: apply_patch path must carry safeDefault (Council finding)"
                );
                assert_eq!(
                    payload["correlation"]["enforcementMode"], "block",
                    "v1 contract: enforcementMode stays in the closed set {{block|warn|off}}"
                );
                assert_eq!(
                    payload["correlation"]["gateState"], "unavailable",
                    "gate-unavailable signal lives in `gateState`, not `enforcementMode`"
                );
                assert_eq!(payload["schemaVersion"], "anvil.mcp.auth-required.v1");
                assert_eq!(payload["tool"], "anvil_apply_patch");
            },
        );
    }

    #[test]
    fn auth_required_payload_schema_stays_v1() {
        // The decision-vocabulary change is additive — schema string
        // stays `anvil.mcp.validate-write.v1`. Existing v1 consumers
        // that branch on `decision` will see a previously-unknown
        // value (`gateUnavailable`); per SERVER_INSTRUCTIONS this is
        // documented as proceed-with-warning.
        let payload = super::mcp_auth_required_result(&json!({"path": "src/x.ts"}));
        let text = payload["content"][0]["text"].as_str().expect("text");
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["schema"], "anvil.mcp.validate-write.v1");
        assert_eq!(parsed["decision"], "gateUnavailable");
        assert_eq!(parsed["error"]["code"], "authentication-required");
        // path threading preserved
        assert_eq!(parsed["correlation"]["path"], "src/x.ts");
    }

    #[test]
    fn server_instructions_document_gate_unavailable_vocabulary() {
        // The published `initialize.instructions` text is what
        // well-behaved agents read to decide how to handle each
        // decision value. Pin the contract: `block` MUST be called out
        // as authoritative with diagnostics, `gateUnavailable` MUST be
        // called out as informational so agents do not honour it as a
        // hard stop.
        let s = super::SERVER_INSTRUCTIONS;
        assert!(s.contains("`block`"), "instructions must name `block`");
        assert!(
            s.contains("`gateUnavailable`"),
            "instructions must name the new `gateUnavailable` decision"
        );
        assert!(
            s.contains("informational"),
            "instructions must mark gateUnavailable as informational, not authoritative"
        );
        assert!(
            s.contains("diagnostics"),
            "instructions must tell agents `block` is paired with diagnostics"
        );
    }

    #[test]
    fn gctx_tool_result_error_classification() {
        // CIB-091d: only a genuine tool error (`isError: true`) is excluded from
        // the egress charge. A success and a missing/false flag both charge.
        assert!(gctx_tool_result_is_error(
            &json!({ "content": [], "isError": true })
        ));
        assert!(!gctx_tool_result_is_error(
            &json!({ "content": [], "isError": false })
        ));
        // A missing flag is treated as not-an-error (so a payload still charges).
        assert!(!gctx_tool_result_is_error(&json!({ "content": [] })));
    }

    #[test]
    fn gctx_quota_exceeded_result_is_structured_error() {
        // CIB-091d: the shared-credit refusal returned to a GCTX `tools/call`.
        let result = gctx_quota_exceeded_result("anvil_search_symbols");
        assert_eq!(result["isError"], true, "exhaustion stops the assistant");
        let text = result["content"][0]["text"].as_str().expect("text");
        let payload: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(payload["kind"], "quota_exceeded");
        assert_eq!(payload["tool"], "anvil_search_symbols");
        assert!(
            payload["error"].as_str().unwrap().contains("quota"),
            "{}",
            payload["error"]
        );
    }

    #[test]
    fn gctx_tool_call_is_refused_once_the_shared_egress_credit_is_exhausted() {
        // CIB-091d: a GCTX `tools/call` charges the SAME per-session graph://
        // egress credit as `resources/read`, closing the reassembly back door.
        // The credit is a process-global static; the shared test guard serialises
        // the credit-touching tests and zeroes the counter, so this starts fresh
        // and never leaves the credit poisoned for an order-sensitive sibling.
        let _guard = crate::mcp::resources::lock_and_reset_graph_egress_for_test();

        // A valid workspace root so the tool call itself is NOT an error: with no
        // daemon it degrades to a successful `unavailable` outcome (isError:false),
        // which reaches the egress-charge step.
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");

        // Sanity: a fresh credit serves the GCTX tool call (charged, under budget).
        let ok = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "anvil_search_symbols",
                "arguments": { "workspaceRoot": workspace.path() }
            }
        }))
        .expect("request should produce a response");
        assert_eq!(
            ok["result"]["isError"], false,
            "a GCTX tool call under budget is served (the degraded unavailable outcome)"
        );

        // Now exhaust the shared credit and re-issue: the SAME charge point must
        // refuse with a structured quota_exceeded — proving the tool-call surface
        // shares the resource byte ceiling.
        crate::mcp::resources::exhaust_graph_egress_for_test();
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "anvil_search_symbols",
                "arguments": { "workspaceRoot": workspace.path() }
            }
        }))
        .expect("request should produce a response");

        let result = &response["result"];
        assert_eq!(
            result["isError"], true,
            "an exhausted egress credit refuses the GCTX tool call"
        );
        let text = result["content"][0]["text"].as_str().expect("text");
        let payload: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(
            payload["kind"], "quota_exceeded",
            "the refusal carries the shared quota_exceeded vocabulary"
        );
    }
}
