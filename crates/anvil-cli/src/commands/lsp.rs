//! RTAI-005 production LSP diagnostics surface (ADR-109).
//!
//! `anvil lsp --stdio` is a thin, advisory-only frontend over the daemon's
//! existing `scan_buffer(mode = "midEdit")` contract. Graph navigation belongs
//! to LSPNAV and is deliberately absent from this module.

use std::io::{self, BufReader, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use clap::Args;
use serde_json::{Value, json};

use crate::daemon_validation::{ScanMode, scan_buffer};

mod protocol;
mod state;

use protocol::{WorkspaceRoots, read_lsp_frame};
use state::{ChangeError, DocumentStore, ScanJob};

const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(80);
const EVENT_QUEUE_CAPACITY: usize = 32;
const SCAN_QUEUE_CAPACITY: usize = 8;
const SCAN_WORKERS: usize = 4;

#[derive(Debug, Args)]
pub struct LspArgs {
    /// Serve LSP over stdin/stdout.
    #[arg(long)]
    stdio: bool,
}

pub fn run(args: &LspArgs) -> anyhow::Result<()> {
    if !args.stdio {
        anyhow::bail!("`anvil lsp` currently requires --stdio");
    }
    run_stdio_server()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lifecycle {
    WaitingForInitialize,
    WaitingForInitialized,
    Running,
    Shutdown,
}

enum Event {
    Message(Value),
    ParseError,
    ProtocolError,
    ScanComplete {
        job: ScanJob,
        result:
            Result<Vec<anvil_kernel_types::Diagnostic>, crate::daemon_validation::ScanBufferError>,
    },
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanDispatchError {
    Disconnected,
}

fn run_stdio_server() -> anyhow::Result<()> {
    let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
    spawn_reader(sender.clone());
    let (scan_sender, scan_receiver) = mpsc::sync_channel(SCAN_QUEUE_CAPACITY);
    spawn_scan_workers(&sender, scan_receiver);

    let mut stdout = io::stdout().lock();
    let mut lifecycle = Lifecycle::WaitingForInitialize;
    let mut documents = DocumentStore::new(DEFAULT_DEBOUNCE);
    let mut workspace_roots = WorkspaceRoots::default();

    loop {
        for job in documents.take_due_bounded(Instant::now(), SCAN_QUEUE_CAPACITY) {
            match try_send_scan(&scan_sender, job) {
                Ok(Some(job)) => documents.retry(&job, Instant::now()),
                Ok(None) => {}
                Err(ScanDispatchError::Disconnected) => {
                    anyhow::bail!("LSP scan workers disconnected")
                }
            }
        }

        let event = receive_event(&receiver, documents.next_deadline());
        let Some(event) = event else {
            continue;
        };

        match event {
            Event::Eof => break,
            Event::ParseError => {
                write_message(
                    &mut stdout,
                    &error_response(&Value::Null, -32700, "Parse error"),
                )?;
            }
            Event::ProtocolError => {
                write_message(
                    &mut stdout,
                    &error_response(&Value::Null, -32700, "Parse error"),
                )?;
                anyhow::bail!("LSP protocol framing failed");
            }
            Event::ScanComplete { job, result } => {
                let successful = result.is_ok();
                let diagnostics = result.unwrap_or_else(|error| {
                    eprintln!("anvil-lsp: mid-edit scan unavailable: {error}");
                    Vec::new()
                });
                if documents.finish(&job, successful) {
                    write_message(
                        &mut stdout,
                        &publish_diagnostics_notification(
                            &job.uri,
                            job.version,
                            &job.text,
                            &diagnostics,
                        ),
                    )?;
                }
            }
            Event::Message(message) => {
                if !handle_message(
                    &mut stdout,
                    &message,
                    &mut lifecycle,
                    &mut documents,
                    &mut workspace_roots,
                )? {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn try_send_scan(
    sender: &SyncSender<ScanJob>,
    job: ScanJob,
) -> Result<Option<ScanJob>, ScanDispatchError> {
    match sender.try_send(job) {
        Ok(()) => Ok(None),
        Err(TrySendError::Full(job)) => Ok(Some(job)),
        Err(TrySendError::Disconnected(_)) => Err(ScanDispatchError::Disconnected),
    }
}

fn receive_event(receiver: &Receiver<Event>, deadline: Option<Instant>) -> Option<Event> {
    match deadline {
        Some(deadline) => {
            match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(event) => Some(event),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => Some(Event::Eof),
            }
        }
        None => match receiver.recv() {
            Ok(event) => Some(event),
            Err(_) => Some(Event::Eof),
        },
    }
}

fn spawn_reader(sender: SyncSender<Event>) {
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        loop {
            match read_lsp_frame(&mut reader) {
                Ok(Some(body)) => match serde_json::from_slice(&body) {
                    Ok(message) => {
                        if sender.send(Event::Message(message)).is_err() {
                            return;
                        }
                    }
                    Err(_) => {
                        if sender.send(Event::ParseError).is_err() {
                            return;
                        }
                    }
                },
                Ok(None) => {
                    let _ = sender.send(Event::Eof);
                    return;
                }
                Err(error) => {
                    eprintln!("anvil-lsp: rejected malformed protocol frame: {error}");
                    let _ = sender.send(Event::ProtocolError);
                    return;
                }
            }
        }
    });
}

fn spawn_scan_workers(sender: &SyncSender<Event>, receiver: mpsc::Receiver<ScanJob>) {
    let receiver = Arc::new(Mutex::new(receiver));
    for _ in 0..SCAN_WORKERS {
        let sender = sender.clone();
        let receiver = Arc::clone(&receiver);
        thread::spawn(move || {
            loop {
                let job = {
                    let Ok(receiver) = receiver.lock() else {
                        return;
                    };
                    let Ok(job) = receiver.recv() else { return };
                    job
                };
                let result = scan_buffer(
                    ScanMode::MidEdit,
                    &job.relative_path.to_string_lossy(),
                    &job.text,
                    &job.cancelled,
                );
                if sender.send(Event::ScanComplete { job, result }).is_err() {
                    return;
                }
            }
        });
    }
}

fn handle_message(
    stdout: &mut impl Write,
    message: &Value,
    lifecycle: &mut Lifecycle,
    documents: &mut DocumentStore,
    workspace_roots: &mut WorkspaceRoots,
) -> anyhow::Result<bool> {
    let method = message.get("method").and_then(Value::as_str);
    let id = message.get("id").cloned();

    if method == Some("exit") {
        if *lifecycle != Lifecycle::Shutdown {
            anyhow::bail!("LSP exit received before shutdown");
        }
        return Ok(false);
    }

    match (*lifecycle, method) {
        (Lifecycle::WaitingForInitialize, Some("initialize")) => {
            let Some(id) = id else {
                return Ok(true);
            };
            write_message(stdout, &initialize_response(&id))?;
            *workspace_roots = WorkspaceRoots::from_initialize(message);
            *lifecycle = Lifecycle::WaitingForInitialized;
        }
        (Lifecycle::WaitingForInitialize, _) => {
            if let Some(id) = id {
                write_message(
                    stdout,
                    &error_response(&id, -32002, "Server not initialized"),
                )?;
            }
        }
        (Lifecycle::WaitingForInitialized, Some("initialized")) => {
            *lifecycle = Lifecycle::Running;
        }
        (Lifecycle::WaitingForInitialized | Lifecycle::Running, Some("shutdown")) => {
            if let Some(id) = id {
                write_message(stdout, &success_response(&id, &Value::Null))?;
            }
            *lifecycle = Lifecycle::Shutdown;
            documents.close_all();
        }
        (Lifecycle::Running, Some("textDocument/didOpen")) => {
            if let (Some(uri), Some(version), Some(text)) = (
                message
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str),
                message
                    .pointer("/params/textDocument/version")
                    .and_then(Value::as_i64),
                message
                    .pointer("/params/textDocument/text")
                    .and_then(Value::as_str),
            ) {
                match workspace_roots.relative_path(uri) {
                    Ok(relative_path) => {
                        if documents
                            .open(uri, relative_path, version, text, Instant::now())
                            .is_err()
                        {
                            eprintln!("anvil-lsp: document capacity reached");
                            // Fail closed like didChange CapacityExceeded: drop any
                            // retained document and clear published diagnostics so a
                            // refused re-open cannot leave stale client state.
                            documents.close(uri);
                            write_message(stdout, &clear_diagnostics_notification(uri))?;
                        }
                    }
                    Err(error) => eprintln!("anvil-lsp: refused document URI: {error}"),
                }
            }
        }
        (Lifecycle::Running, Some("textDocument/didChange")) => {
            if let (Some(uri), Some(version), Some(text)) = (
                message
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str),
                message
                    .pointer("/params/textDocument/version")
                    .and_then(Value::as_i64),
                message
                    .pointer("/params/contentChanges/0/text")
                    .and_then(Value::as_str),
            ) {
                match documents.change(uri, version, text, Instant::now()) {
                    Ok(()) | Err(ChangeError::StaleVersion) => {}
                    Err(ChangeError::CapacityExceeded) => {
                        eprintln!("anvil-lsp: document capacity reached during change");
                        documents.close(uri);
                        write_message(stdout, &clear_diagnostics_notification(uri))?;
                    }
                }
            }
        }
        (Lifecycle::Running, Some("textDocument/didClose")) => {
            if let Some(uri) = message
                .pointer("/params/textDocument/uri")
                .and_then(Value::as_str)
            {
                documents.close(uri);
                write_message(stdout, &clear_diagnostics_notification(uri))?;
            }
        }
        (Lifecycle::Shutdown, _) => {}
        (Lifecycle::WaitingForInitialized | Lifecycle::Running, _) => {
            if let Some(id) = id {
                write_message(stdout, &error_response(&id, -32601, "Method not found"))?;
            }
        }
    }

    Ok(true)
}

fn write_message(stdout: &mut impl Write, message: &Value) -> anyhow::Result<()> {
    let body = serde_json::to_vec(message)?;
    write!(stdout, "Content-Length: {}\r\n\r\n", body.len())?;
    stdout.write_all(&body)?;
    stdout.flush()?;
    Ok(())
}

fn success_response(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn initialize_response(id: &Value) -> Value {
    success_response(
        id,
        &json!({
            "capabilities": { "textDocumentSync": 1 },
            "serverInfo": {
                "name": "anvil-lsp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn publish_diagnostics_notification(
    uri: &str,
    version: i64,
    text: &str,
    diagnostics: &[anvil_kernel_types::Diagnostic],
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "version": version,
            "diagnostics": diagnostics
                .iter()
                .map(|diagnostic| to_lsp_diagnostic(diagnostic, text))
                .collect::<Vec<_>>()
        }
    })
}

fn clear_diagnostics_notification(uri: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": [] }
    })
}

fn to_lsp_diagnostic(diagnostic: &anvil_kernel_types::Diagnostic, text: &str) -> Value {
    let start_line = diagnostic.location.line.unwrap_or(1).saturating_sub(1);
    let end_line = diagnostic
        .location
        .end_line
        .unwrap_or(diagnostic.location.line.unwrap_or(1))
        .saturating_sub(1);
    let start_col = diagnostic.location.column.unwrap_or(1).saturating_sub(1);
    let end_col = diagnostic
        .location
        .end_column
        .unwrap_or(diagnostic.location.column.unwrap_or(1))
        .saturating_sub(1);

    let severity = match diagnostic.severity {
        anvil_kernel_types::Severity::Error => 1,
        anvil_kernel_types::Severity::Warning | anvil_kernel_types::Severity::Unknown => 2,
        anvil_kernel_types::Severity::Info => 3,
    };

    json!({
        "range": {
            "start": {
                "line": start_line,
                "character": byte_column_to_utf16(text, start_line, start_col)
            },
            "end": {
                "line": end_line,
                "character": byte_column_to_utf16(text, end_line, end_col)
            }
        },
        "severity": severity,
        "code": diagnostic.source.rule_id,
        "source": "anvil",
        "message": diagnostic.summary,
        "data": { "phase": "midEdit" }
    })
}

fn byte_column_to_utf16(text: &str, line: u32, byte_column: u32) -> u32 {
    let Some(line_text) = text.lines().nth(line as usize) else {
        return 0;
    };
    let mut end = usize::try_from(byte_column)
        .unwrap_or(usize::MAX)
        .min(line_text.len());
    while !line_text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    u32::try_from(line_text[..end].encode_utf16().count()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use serde_json::json;

    use super::{
        DocumentStore, Event, Lifecycle, ScanDispatchError, WorkspaceRoots, byte_column_to_utf16,
        handle_message, receive_event, try_send_scan,
    };

    #[test]
    fn disconnected_event_channel_is_treated_as_eof() {
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(sender);

        assert!(matches!(receive_event(&receiver, None), Some(Event::Eof)));
    }

    #[test]
    fn disconnected_scan_workers_are_terminal() {
        let started = Instant::now();
        let mut documents = DocumentStore::new(Duration::ZERO);
        documents
            .open("file:///src/main.rs", "main.rs".into(), 1, "one", started)
            .expect("document capacity");
        let job = documents.take_due(started).pop().expect("due scan job");
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(receiver);

        assert!(matches!(
            try_send_scan(&sender, job),
            Err(ScanDispatchError::Disconnected)
        ));
    }

    #[test]
    fn unicode_columns_are_projected_as_utf16_code_units() {
        assert_eq!(byte_column_to_utf16("a😀z", 0, 5), 3);
        assert_eq!(byte_column_to_utf16("first\r\na😀e\u{301}z", 1, 8), 5);
        assert_eq!(byte_column_to_utf16("plain", 0, 3), 3);
    }

    #[test]
    fn requests_before_initialize_return_server_not_initialized() {
        let mut output = Vec::new();
        let mut lifecycle = Lifecycle::WaitingForInitialize;
        let mut documents = DocumentStore::new(Duration::from_millis(80));
        let mut roots = WorkspaceRoots::default();

        handle_message(
            &mut output,
            &json!({"jsonrpc":"2.0","id":1,"method":"shutdown"}),
            &mut lifecycle,
            &mut documents,
            &mut roots,
        )
        .expect("handle request");

        let rendered = String::from_utf8(output).expect("utf8 output");
        assert!(rendered.contains("-32002"));
        assert_eq!(lifecycle, Lifecycle::WaitingForInitialize);
        assert!(documents.take_due(Instant::now()).is_empty());
    }

    #[test]
    fn capacity_rejected_change_closes_document_and_clears_diagnostics() {
        let uri = "file:///src/main.rs";
        let mut output = Vec::new();
        let mut lifecycle = Lifecycle::Running;
        let mut documents = DocumentStore::new(Duration::ZERO);
        documents
            .open(uri, "main.rs".into(), 1, "one", Instant::now())
            .expect("document capacity");
        let mut roots = WorkspaceRoots::default();

        handle_message(
            &mut output,
            &json!({
                "jsonrpc":"2.0",
                "method":"textDocument/didChange",
                "params":{
                    "textDocument":{"uri":uri,"version":2},
                    "contentChanges":[{"text":"x".repeat(super::protocol::MAX_DOCUMENT_BYTES + 1)}]
                }
            }),
            &mut lifecycle,
            &mut documents,
            &mut roots,
        )
        .expect("handle capacity-rejected change");

        let rendered = String::from_utf8(output).expect("utf8 output");
        assert!(rendered.contains("textDocument/publishDiagnostics"));
        assert!(rendered.contains("\"diagnostics\":[]"));
        assert!(rendered.contains(uri));
        assert!(documents.take_due(Instant::now()).is_empty());
    }

    #[test]
    fn capacity_rejected_open_closes_retained_document_and_clears_diagnostics() {
        let uri = "file:///src/main.rs";
        let mut output = Vec::new();
        let mut lifecycle = Lifecycle::Running;
        let mut documents = DocumentStore::new(Duration::ZERO);
        documents
            .open(uri, "main.rs".into(), 1, "one", Instant::now())
            .expect("document capacity");
        let mut roots = WorkspaceRoots::from_initialize(&json!({
            "params": {
                "rootUri": "file:///"
            }
        }));

        handle_message(
            &mut output,
            &json!({
                "jsonrpc":"2.0",
                "method":"textDocument/didOpen",
                "params":{
                    "textDocument":{
                        "uri":uri,
                        "languageId":"rust",
                        "version":2,
                        "text":"x".repeat(super::protocol::MAX_DOCUMENT_BYTES + 1)
                    }
                }
            }),
            &mut lifecycle,
            &mut documents,
            &mut roots,
        )
        .expect("handle capacity-rejected open");

        let rendered = String::from_utf8(output).expect("utf8 output");
        assert!(rendered.contains("textDocument/publishDiagnostics"));
        assert!(rendered.contains("\"diagnostics\":[]"));
        assert!(rendered.contains(uri));
        assert!(documents.take_due(Instant::now()).is_empty());
    }
}
