use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{Value, json};

use crate::GlobalArgs;
use crate::commands::mcp_config::{self, Target};
use crate::mcp::tools::validate_write;

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
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
    /// Install Anvil MCP configuration for an editor.
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

    /// Override the client config root. Defaults to the user's home directory.
    #[arg(long)]
    workspace: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum McpClient {
    /// Cursor (`.cursor/mcp.json`).
    Cursor,
    /// Anthropic Claude Code (`.claude/mcp.json`).
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

fn run_install(args: &McpInstallArgs, global: &GlobalArgs) -> Result<()> {
    let config_root = match &args.workspace {
        Some(path) => path.clone(),
        None => mcp_config::default_client_config_root()?,
    };
    let target = args.client.target();

    if args.verify {
        let (path, entry) = mcp_config::verify_rust_stdio_target(target, &config_root, global)?;
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

    let install = mcp_config::install_rust_stdio_target(target, &config_root, global)?;
    if global.json {
        println!(
            "{}",
            json!({
                "client": args.client.label(),
                "path": install.path.display().to_string(),
                "wrote": install.wrote,
                "drifted": install.drifted,
                "command": "anvil",
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
        Some("initialize") => id.map(|id| initialize_response(id, message)),
        Some("notifications/initialized") => None,
        Some("exit") if id.is_none() => None,
        Some("exit") => id.map(|id| error_response(id, -32600, "Invalid Request")),
        Some("shutdown") => id.map(|id| success_response(id, &Value::Null)),
        Some("ping") => id.map(|id| success_response(id, &json!({}))),
        Some("tools/list") => id.map(tools_list_response),
        Some("tools/call") => id.map(|id| tools_call_response(id, message)),
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
            "tools": {}
        },
        "serverInfo": {
            "name": "anvil",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    success_response(id, &result)
}

fn tools_list_response(id: &Value) -> Value {
    success_response(
        id,
        &json!({
            "tools": [validate_write_tool_descriptor()]
        }),
    )
}

fn validate_write_tool_descriptor() -> Value {
    validate_write::descriptor()
}

fn tools_call_response(id: &Value, message: &Value) -> Value {
    let Some(params) = message.get("params").and_then(Value::as_object) else {
        return error_response(id, -32602, "Invalid params");
    };

    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, -32602, "Invalid params");
    };

    if name != validate_write::TOOL_NAME {
        return error_response_with_data(
            id,
            -32602,
            "Invalid params",
            &json!({
                "reason": "unknown-tool",
                "tool": name
            }),
        );
    }

    let empty_arguments = json!({});
    let arguments = params.get("arguments").unwrap_or(&empty_arguments);

    success_response(id, &validate_write::call(arguments))
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

    use super::{Frame, MAX_STDIO_FRAME_BYTES, read_frame};

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
}
