use std::io::{self, BufRead, BufReader, Read, Write};

use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::GlobalArgs;

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const MAX_STDIO_FRAME_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    command: McpCommand,
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    /// Start an MCP server.
    Serve(McpServeArgs),
}

#[derive(Debug, Args)]
struct McpServeArgs {
    /// Serve MCP over stdin/stdout.
    #[arg(long)]
    stdio: bool,
}

pub fn run(args: &McpArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        McpCommand::Serve(serve) => run_serve(serve),
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

    if frame.len() as u64 > MAX_STDIO_FRAME_BYTES {
        discard_line_tail(reader)?;
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
    let protocol_version = message
        .get("params")
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);

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
}
