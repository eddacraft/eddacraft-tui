use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::GlobalArgs;
use crate::activation::agent_registry::{AgentClientId, InstallScope};
use crate::commands::{mcp_config, mcp_installer};
use crate::mcp::protocol::{self, handle_message};
use crate::output::AlreadyReported;

// Keep the stdio frame ceiling comfortably above the largest accepted tool
// payload. validate-write caps `proposedContent` at 1 MiB of UTF-8 source.
// JSON string escaping can grow that almost 2x in the worst case (every byte
// is `"` or `\\`), and the JSON-RPC / MCP envelope adds further overhead, so
// allow up to 4 MiB on the wire to keep valid requests from being rejected
// at the framing layer before tool-level validation runs.
pub(crate) const MAX_STDIO_FRAME_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    command: McpCommand,
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    /// Install anvil MCP configuration for an editor.
    Install(McpInstallArgs),
    /// Serve anvil MCP tools over stdin/stdout for editor and agent clients.
    Serve(McpServeArgs),
    /// Rewrite owned MCP configs, recycle a skewed daemon, and poke live heal.
    Refresh(super::mcp_refresh::McpRefreshArgs),
}

#[derive(Debug, Args)]
struct McpInstallArgs {
    /// Client to configure.
    #[arg(long, value_enum)]
    client: AgentClientId,

    /// Installation scope. Global is the beta default; project is explicit.
    #[arg(long, value_enum, default_value_t = InstallScope::Global)]
    scope: InstallScope,

    /// Verify the existing client config instead of writing it.
    #[arg(long)]
    verify: bool,

    /// Override the command path written into stdio configs. Defaults to `anvil`.
    #[arg(long)]
    command: Option<String>,

    /// Override the client config root. Defaults to the user's home directory.
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Preview the resolved path and entry without writing.
    #[arg(long, conflicts_with = "verify")]
    dry_run: bool,
}

#[derive(Debug, Args)]
#[command(override_usage = "anvil mcp serve --stdio")]
struct McpServeArgs {
    /// Required. Serve MCP over stdin/stdout. Editors launch
    /// `anvil mcp serve --stdio`.
    #[arg(long)]
    stdio: bool,
}

pub fn run(args: &McpArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        McpCommand::Install(install) => run_install(install, global),
        McpCommand::Serve(serve) => run_serve(serve),
        McpCommand::Refresh(refresh) => super::mcp_refresh::run(refresh, global),
    }
}

pub fn auth_gate_name(args: &McpArgs) -> &'static str {
    match &args.command {
        McpCommand::Install(_) | McpCommand::Refresh(_) => "mcp-install",
        McpCommand::Serve(_) => "mcp-serve",
    }
}

fn run_install(args: &McpInstallArgs, global: &GlobalArgs) -> Result<()> {
    if args.client == AgentClientId::VsCode && args.scope == InstallScope::Global {
        return run_vscode_profile_install(args, global);
    }
    let config_root = match &args.workspace {
        Some(path) => path.clone(),
        None if args.scope == InstallScope::Global => mcp_config::default_client_config_root()?,
        None => std::env::current_dir()?,
    };
    if args
        .command
        .as_deref()
        .is_some_and(|command| command.trim().is_empty())
    {
        bail!("--command must not be empty");
    }

    let command = crate::activation::mcp_client::preferred_mcp_command(
        args.command.as_deref().map(str::trim),
    );
    let install = match mcp_installer::install(
        args.client,
        args.scope,
        &config_root,
        command,
        args.verify,
        args.dry_run,
    ) {
        Ok(report) => report,
        Err(error) if global.json && args.verify => {
            eprintln!(
                "{}",
                json!({
                    "client": args.client.label(),
                    "error": "malformed-entry",
                    "message": error.to_string(),
                    "expected": {
                        "command": command,
                        "args": ["mcp", "serve", "--stdio"],
                        "type": "stdio",
                        "typeRequired": matches!(
                            args.client,
                            AgentClientId::ClaudeCode | AgentClientId::CopilotCli
                        ),
                    },
                })
            );
            return Err(AlreadyReported.into());
        }
        Err(error) => return Err(error),
    };
    if global.json {
        println!(
            "{}",
            json!({
                "client": args.client.label(),
                "path": install.path.display().to_string(),
                "wrote": install.wrote,
                "changed": install.changed,
                "drifted": install.drifted,
                "scope": args.scope.label(),
                "dryRun": args.dry_run,
                "command": command,
                "args": ["mcp", "serve", "--stdio"],
                "entry": install.entry,
                "ok": true,
            })
        );
    } else {
        println!(
            "Client: {} ({} config: {})",
            args.client.label(),
            args.scope.label(),
            install.path.display()
        );
        if install.drifted {
            println!("Existing entry drifted; rewrote the anvil MCP server entry.");
        }
        let status = if args.verify {
            "verified"
        } else if args.dry_run && install.changed {
            "would update"
        } else if args.dry_run {
            "already configured"
        } else if install.wrote {
            "ok"
        } else {
            "already configured"
        };
        println!("Installing anvil MCP server entry ... {status}");
        if !args.verify && !args.dry_run {
            println!("{}", install.reload_hint);
        }
    }
    Ok(())
}

fn run_vscode_profile_install(args: &McpInstallArgs, global: &GlobalArgs) -> Result<()> {
    let command = crate::activation::mcp_client::preferred_mcp_command(
        args.command.as_deref().map(str::trim),
    );
    if command.is_empty() {
        bail!("--command must not be empty");
    }
    if args.verify {
        bail!(
            "VS Code global MCP configuration is profile-owned; verify the `anvil` server in VS Code's MCP UI, or use --scope project for file verification"
        );
    }
    let payload = json!({
        "name": "anvil",
        "command": command,
        "args": ["mcp", "serve", "--stdio"],
    })
    .to_string();
    if !args.dry_run {
        let status = std::process::Command::new("code")
            .arg("--add-mcp")
            .arg(&payload)
            .status()
            .context("running `code --add-mcp`; install the VS Code CLI or use --scope project")?;
        if !status.success() {
            bail!("`code --add-mcp` exited with {status}");
        }
    }

    if global.json {
        println!(
            "{}",
            json!({
                "client": "vscode",
                "scope": "global",
                "delegatedTo": "code --add-mcp",
                "dryRun": args.dry_run,
                "payload": serde_json::from_str::<Value>(&payload)?,
                "ok": true,
            })
        );
    } else if args.dry_run {
        println!("Would delegate VS Code profile installation to `code --add-mcp {payload}`");
    } else {
        println!("Delegated VS Code profile installation to `code --add-mcp`.");
        println!("Trust and start the anvil server in VS Code's MCP UI.");
    }
    Ok(())
}

fn run_serve(args: &McpServeArgs) -> Result<()> {
    if !args.stdio {
        bail!("`anvil mcp serve` currently requires --stdio");
    }

    run_stdio_server()
}

fn run_stdio_server() -> Result<()> {
    // Recycle before the first read so a skewed image does not consume
    // `initialize` (the replacement process then handles the handshake).
    crate::mcp::reexec::maybe_reexec_at_startup();

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::stdout().lock();

    while let Some(frame) = read_frame(&mut reader)? {
        let Frame::Message(frame) = frame else {
            write_message(
                &mut stdout,
                &protocol::render::error_response(
                    &Value::Null,
                    protocol::versions::ERR_INVALID_REQUEST,
                    "Invalid Request",
                ),
            )?;
            continue;
        };

        if frame.iter().all(u8::is_ascii_whitespace) {
            continue;
        }

        let Ok(message) = serde_json::from_slice::<Value>(&frame) else {
            write_message(&mut stdout, &protocol::render::parse_error_response())?;
            continue;
        };

        // Between complete frames only — never after partial JSON-RPC stdout.
        crate::mcp::reexec::maybe_reexec_between_messages(&message);

        if let Some(response) = handle_message(&message) {
            write_message(&mut stdout, &response)?;
        }

        if protocol::dispatch::is_exit_notification(&message) {
            break;
        }
    }

    Ok(())
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

    use super::{Frame, MAX_STDIO_FRAME_BYTES, read_frame};
    use crate::mcp::protocol::domain::{
        EdictAuthCacheEntry, edict_auth_cache, edict_verify_cache_ttl, gctx_quota_exceeded_result,
        gctx_tool_result_is_error, mcp_auth_required_result,
    };
    use crate::mcp::protocol::handle_message;
    use std::time::{Duration, Instant};

    fn modern_meta() -> serde_json::Value {
        json!({
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": {}
        })
    }

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
        let ttl = edict_verify_cache_ttl();
        assert!(
            ttl <= Duration::from_mins(5),
            "edict verify cache TTL is too long: {ttl:?}"
        );
    }

    #[test]
    fn edict_auth_cache_entry_invalidates_on_license_change() {
        // Cache is keyed on (license, checked_at). A different license must
        // be treated as a miss even within the TTL window — credential
        // changes during a long-lived MCP process must not be served stale.
        let now = Instant::now();
        let entry = EdictAuthCacheEntry {
            license: "lic-a".to_string(),
            checked_at: now,
            ok: true,
        };
        // Same license + within TTL → hit.
        assert_eq!(entry.license, "lic-a");
        assert!(entry.checked_at.elapsed() < edict_verify_cache_ttl());
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
                        "_meta": modern_meta(),
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
                        "_meta": modern_meta(),
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
        let payload = mcp_auth_required_result(&json!({"path": "src/x.ts"}));
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
        let s = crate::mcp::protocol::domain::SERVER_INSTRUCTIONS;
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
        // CIB-091d: a GCTX `tools/call` charges the SAME process-local graph://
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
                "_meta": modern_meta(),
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
                "_meta": modern_meta(),
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

    /// CIB-144: assert a `tools/call` response is the shared auth-required
    /// envelope (the non-`validate_write` shape emitted by
    /// `mcp_tool_auth_required_result`) for `tool`, proving the auth gate
    /// short-circuited before the tool ran.
    fn assert_auth_required_envelope(response: &serde_json::Value, tool: &str) {
        let result = &response["result"];
        assert_eq!(
            result["isError"], false,
            "auth-required is not a tool error (agents must not abort)"
        );
        let text = result["content"][0]["text"]
            .as_str()
            .expect("tool content text");
        let payload: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(payload["schemaVersion"], "anvil.mcp.auth-required.v1");
        assert_eq!(payload["decision"], "gateUnavailable");
        assert_eq!(payload["safeDefault"], "allow-with-warning");
        assert_eq!(payload["tool"], tool);
        // The tool's own success payload keys must be absent — the auth gate
        // returned INSTEAD of invoking the tool, so no gate/fix/suppress result.
        assert!(
            payload.get("fixed").is_none()
                && payload.get("mode").is_none()
                && payload.get("hasBlockingWarnings").is_none()
                && payload.get("suppressed").is_none(),
            "auth-required envelope must not carry a tool result payload for {tool}: {payload}"
        );
    }

    #[test]
    fn anvil_fix_tool_call_requires_auth_and_leaves_file_untouched() {
        // CIB-144: an unauthenticated `anvil_fix` call must return the
        // auth-required envelope and must NOT mutate the target file. The
        // fixture carries a genuinely fixable AP-003 occurrence, so an
        // authenticated (or dev-bypass) call WOULD rewrite it — proving the
        // untouched assertion is load-bearing, not vacuous.
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let file = workspace.path().join("src/a.ts");
        std::fs::create_dir_all(file.parent().unwrap()).expect("parent dirs");
        let original = "const x: any = 1;\n";
        std::fs::write(&file, original).expect("fixture written");

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
                        "_meta": modern_meta(),
                        "name": "anvil_fix",
                        "arguments": {
                            "filePath": "src/a.ts",
                            "warningId": "AP-003",
                            "line": 1,
                            "workspaceRoot": workspace.path()
                        }
                    }
                }))
                .expect("request should produce a response");

                assert_auth_required_envelope(&response, "anvil_fix");
            },
        );

        let on_disk = std::fs::read_to_string(&file).expect("file readable");
        assert_eq!(
            on_disk, original,
            "CIB-144: unauthenticated anvil_fix must not rewrite the file"
        );
    }

    #[test]
    fn anvil_suppress_tool_call_requires_auth_and_leaves_file_untouched() {
        // CIB-144: an unauthenticated `anvil_suppress` call must return the
        // auth-required envelope and must NOT insert a suppression comment.
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let file = workspace.path().join("src/a.ts");
        std::fs::create_dir_all(file.parent().unwrap()).expect("parent dirs");
        let original = "const x: any = 1;\n";
        std::fs::write(&file, original).expect("fixture written");

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
                        "_meta": modern_meta(),
                        "name": "anvil_suppress",
                        "arguments": {
                            "filePath": "src/a.ts",
                            "warningId": "AP-003",
                            "line": 1,
                            "reason": "triaging in follow-up",
                            "workspaceRoot": workspace.path()
                        }
                    }
                }))
                .expect("request should produce a response");

                assert_auth_required_envelope(&response, "anvil_suppress");
            },
        );

        let on_disk = std::fs::read_to_string(&file).expect("file readable");
        assert_eq!(
            on_disk, original,
            "CIB-144: unauthenticated anvil_suppress must not insert a suppression comment"
        );
    }

    #[test]
    fn anvil_gate_tool_call_requires_auth_and_does_not_run() {
        // CIB-144: an unauthenticated `anvil_gate` call must return the
        // auth-required envelope and must NOT run the gate. The planless
        // fixture carries a blocking AP-003 warning, so an executed gate WOULD
        // report `hasBlockingWarnings: true`; the envelope's absence of any
        // gate-result keys proves the antipattern scan never ran.
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let file = workspace.path().join("src/a.ts");
        std::fs::create_dir_all(file.parent().unwrap()).expect("parent dirs");
        std::fs::write(&file, "const x: any = 1;\n").expect("fixture written");

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
                        "_meta": modern_meta(),
                        "name": "anvil_gate",
                        "arguments": {
                            "workspaceRoot": workspace.path(),
                            "targetFiles": ["src/a.ts"]
                        }
                    }
                }))
                .expect("request should produce a response");

                assert_auth_required_envelope(&response, "anvil_gate");
            },
        );
    }
}
