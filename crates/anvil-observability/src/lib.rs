//! TRACE-001: Anvil's cross-cutting tracing baseline.
//!
//! Implements ADR-035's three-pipe rule on the tracing pipe:
//!
//! - [`TraceContext`] models a W3C `traceparent` header (version 00) with
//!   parse / generate / round-trip helpers.
//! - [`BinaryKind`] names the binary doing the initialisation so a single
//!   call to [`init_tracing`] sets the global subscriber for `anvil-cli`,
//!   the `anvil-intercept` daemon, and any future Rust binary.
//! - [`init_tracing`] installs a `tracing-subscriber` JSON formatter,
//!   an `EnvFilter` honouring `RUST_LOG` and `ANVIL_LOG`, and a
//!   sensible per-binary default directive.
//! - [`redaction`] holds the default deny-list of sensitive field names
//!   and the JSON field formatter used by [`init_tracing`] to replace
//!   matching span/event values with the canonical redaction marker.
//!
//! Per ADR-035 spans are **never** source-of-truth; consumers that need
//! durable governance facts go through Kindling, and live state belongs
//! on the notification envelope. `traceparent` is the cross-pipe
//! correlation key so a notification can be joined to its underlying
//! spans.
//!
//! Library crates emit spans via the global `tracing` macros and MUST NOT
//! call [`init_tracing`] themselves. Only binary entrypoints (`main`)
//! initialise the subscriber.

#![forbid(unsafe_code)]

pub mod redaction;
pub mod traceparent;

pub use traceparent::{TraceContext, TraceContextError};

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

use tracing::Span;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

const TRACE_SINK_ENV: &str = "ANVIL_TRACE_SINK";

/// Identifies the binary calling [`init_tracing`]. The variant drives the
/// default `EnvFilter` directive when neither `ANVIL_LOG` nor `RUST_LOG`
/// is set, and is recorded as a `binary` field on every formatted line so
/// multi-binary trace exports stay attributable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryKind {
    /// `anvil` CLI (interactive command surface).
    Cli,
    /// `anvil-intercept` daemon (long-running JSON-RPC server).
    InterceptDaemon,
}

impl BinaryKind {
    /// Stable name written into the `binary` JSON field.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "anvil-cli",
            Self::InterceptDaemon => "anvil-intercept",
        }
    }

    /// Default filter directive used when no env override is provided.
    fn default_filter(self) -> &'static str {
        match self {
            // The CLI prints its own user-facing output — keep tracing
            // quiet by default to avoid mixing diagnostic spans with
            // command output. Operators opt in via `ANVIL_LOG=info`.
            Self::Cli => "warn",
            // The daemon's logs are diagnostic-first; INFO captures
            // lifecycle events without flooding on per-request work.
            Self::InterceptDaemon => "info",
        }
    }
}

/// Errors returned by [`init_tracing`].
#[derive(Debug, thiserror::Error)]
pub enum InitTracingError {
    /// Subscriber install failed because another global subscriber was
    /// already registered. This typically means a binary called
    /// `init_tracing` twice or a library crate (incorrectly) tried to
    /// initialise the global subscriber.
    #[error("global tracing subscriber already installed")]
    AlreadyInstalled,
    /// The requested local development sink could not be opened or was not
    /// recognised.
    #[error("invalid {TRACE_SINK_ENV}: {0}")]
    TraceSink(String),
}

/// Record a parsed `traceparent` onto the current span.
///
/// The target span must declare `trace_id`, `parent_id`, and `trace_flags`
/// fields (usually as [`tracing::field::Empty`]); `tracing` ignores fields that
/// do not exist on the span. This helper intentionally records correlation
/// fields only and does not install an OpenTelemetry parent relationship;
/// exporter wiring and true parent propagation are owned by the EXPORT module.
pub fn bind_traceparent_to_current_span(context: &TraceContext) {
    bind_traceparent_to_span(&Span::current(), context);
}

/// Record a parsed `traceparent` onto an explicit span.
///
/// See [`bind_traceparent_to_current_span`] for the field declaration contract.
pub fn bind_traceparent_to_span(span: &Span, context: &TraceContext) {
    span.record("trace_id", context.trace_id());
    span.record("parent_id", context.parent_id());
    span.record("trace_flags", format_args!("{:02x}", context.flags()));
}

/// Install the global tracing subscriber for an Anvil binary.
///
/// Calling more than once returns [`InitTracingError::AlreadyInstalled`]
/// — every binary entrypoint MUST call this exactly once before spawning
/// the runtime that emits spans. Library crates MUST NOT call it.
///
/// `set_global_default` is the sole atomic guard; this function does not
/// keep a sentinel of its own (a separate flag would create a TOCTOU
/// window between guard check and install).
///
/// Filter precedence (highest first): `ANVIL_LOG`, `RUST_LOG`,
/// [`BinaryKind::default_filter`].
///
/// # Errors
///
/// Returns [`InitTracingError::AlreadyInstalled`] if a global subscriber
/// is already registered for the process.
pub fn init_tracing(kind: BinaryKind) -> Result<(), InitTracingError> {
    let resolved = EnvFilter::try_from_env("ANVIL_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new(kind.default_filter()));
    let filter_repr = resolved.to_string();

    let layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(false)
        .json()
        .fmt_fields(redaction::RedactingJsonFields)
        .event_format(redaction::RedactingJsonEventFormatter::default());

    match std::env::var(TRACE_SINK_ENV).ok().as_deref() {
        None | Some("") => {
            // CIB-024: the CLI reserves stdout for command output (notably
            // `--json`), so its diagnostics go to stderr — otherwise an enabled
            // log level, or any `warn!`/`error!` at the default filter,
            // interleaves log JSON with the command's own JSON. The daemon keeps
            // the default (stdout), where its host captures it.
            let registry = tracing_subscriber::registry().with(resolved);
            let install = if matches!(kind, BinaryKind::Cli) {
                tracing::subscriber::set_global_default(
                    registry.with(layer.with_writer(std::io::stderr)),
                )
            } else {
                tracing::subscriber::set_global_default(registry.with(layer))
            };
            install.map_err(|_| InitTracingError::AlreadyInstalled)?;
        }
        Some(value) if value.starts_with("file=") => {
            let path = value.strip_prefix("file=").expect("checked prefix");
            if path.is_empty() {
                return Err(InitTracingError::TraceSink(
                    "file sink path must not be empty".to_owned(),
                ));
            }
            let file = open_trace_file(path)?;
            let writer = SharedTraceWriter::new(file);
            let subscriber = tracing_subscriber::registry()
                .with(resolved)
                .with(layer.with_writer(move || writer.clone()));
            tracing::subscriber::set_global_default(subscriber)
                .map_err(|_| InitTracingError::AlreadyInstalled)?;
        }
        Some(value) if value.starts_with("otlp") => {
            return Err(InitTracingError::TraceSink(
                "otlp sink is deferred to EXPORT; use file=<path> locally".to_owned(),
            ));
        }
        Some(value) => {
            return Err(InitTracingError::TraceSink(format!(
                "unsupported sink {value:?}; expected file=<path>"
            )));
        }
    }

    tracing::info!(
        target: "anvil_observability",
        binary = kind.as_str(),
        filter = %filter_repr,
        "tracing subscriber installed",
    );
    Ok(())
}

fn open_trace_file(path: &str) -> Result<File, InitTracingError> {
    let path = Path::new(path);
    #[cfg(unix)]
    let existing_metadata = validate_existing_trace_file(path)?;

    #[cfg(unix)]
    let file = open_trace_file_after_validation(path, existing_metadata.is_none())?;

    #[cfg(not(unix))]
    let file = open_trace_file_after_validation(path, true)?;

    #[cfg(unix)]
    validate_opened_trace_file(path, &file, existing_metadata.as_ref())?;

    Ok(file)
}

fn open_trace_file_after_validation(
    path: &Path,
    create_missing: bool,
) -> Result<File, InitTracingError> {
    let mut options = OpenOptions::new();
    options.append(true);
    if create_missing {
        #[cfg(unix)]
        options.create_new(true);
        #[cfg(not(unix))]
        options.create(true);
    }
    #[cfg(unix)]
    options.mode(0o600);

    options
        .open(path)
        .map_err(|err| InitTracingError::TraceSink(format!("file={}: {err}", path.display())))
}

#[cfg(unix)]
fn validate_existing_trace_file(
    path: &Path,
) -> Result<Option<std::fs::Metadata>, InitTracingError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(InitTracingError::TraceSink(format!(
                "file={}: {err}",
                path.display()
            )));
        }
    };

    validate_trace_file_metadata(path, &metadata)?;

    Ok(Some(metadata))
}

#[cfg(unix)]
fn validate_trace_file_metadata(
    path: &Path,
    metadata: &std::fs::Metadata,
) -> Result<(), InitTracingError> {
    if metadata.file_type().is_symlink() {
        return Err(InitTracingError::TraceSink(format!(
            "file={} must not be a symlink",
            path.display()
        )));
    }

    if !metadata.file_type().is_file() {
        return Err(InitTracingError::TraceSink(format!(
            "file={} must be a regular file",
            path.display()
        )));
    }

    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        return Err(InitTracingError::TraceSink(format!(
            "file={} must not be readable or writable by group/other",
            path.display()
        )));
    }

    Ok(())
}

#[cfg(unix)]
fn validate_opened_trace_file(
    path: &Path,
    file: &File,
    existing_metadata: Option<&std::fs::Metadata>,
) -> Result<(), InitTracingError> {
    let opened_metadata = file
        .metadata()
        .map_err(|err| InitTracingError::TraceSink(format!("file={}: {err}", path.display())))?;
    if !opened_metadata.file_type().is_file() {
        return Err(InitTracingError::TraceSink(format!(
            "file={} must be a regular file",
            path.display()
        )));
    }

    let path_metadata = std::fs::symlink_metadata(path)
        .map_err(|err| InitTracingError::TraceSink(format!("file={}: {err}", path.display())))?;
    validate_trace_file_metadata(path, &path_metadata)?;

    if let Some(expected) = existing_metadata
        && (expected.dev(), expected.ino()) != (opened_metadata.dev(), opened_metadata.ino())
    {
        return Err(InitTracingError::TraceSink(format!(
            "file={} changed while opening",
            path.display()
        )));
    }

    if (path_metadata.dev(), path_metadata.ino()) != (opened_metadata.dev(), opened_metadata.ino())
    {
        return Err(InitTracingError::TraceSink(format!(
            "file={} changed while opening",
            path.display()
        )));
    }

    Ok(())
}

#[derive(Clone)]
struct SharedTraceWriter {
    file: Arc<Mutex<File>>,
}

impl SharedTraceWriter {
    fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

impl Write for SharedTraceWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("trace sink lock poisoned"))?
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("trace sink lock poisoned"))?
            .flush()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    #[cfg(unix)]
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    use tracing::{field, info_span};
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::registry::LookupSpan;

    use super::*;

    #[cfg(unix)]
    fn fresh_test_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "anvil-observability-{name}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("test dir");
        path
    }

    #[derive(Debug, Default, Clone)]
    struct RecordedFields(Arc<Mutex<HashMap<String, String>>>);

    impl RecordedFields {
        fn get(&self, key: &str) -> Option<String> {
            self.0.lock().expect("fields").get(key).cloned()
        }
    }

    struct RecordingLayer {
        fields: RecordedFields,
    }

    impl<S> Layer<S> for RecordingLayer
    where
        S: tracing::Subscriber,
        S: for<'lookup> LookupSpan<'lookup>,
    {
        fn on_record(
            &self,
            _span: &tracing::Id,
            values: &tracing::span::Record<'_>,
            _ctx: Context<'_, S>,
        ) {
            values.record(&mut FieldVisitor {
                fields: self.fields.clone(),
            });
        }
    }

    struct FieldVisitor {
        fields: RecordedFields,
    }

    impl field::Visit for FieldVisitor {
        fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
            self.fields
                .0
                .lock()
                .expect("fields")
                .insert(field.name().to_owned(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &field::Field, value: &str) {
            self.fields
                .0
                .lock()
                .expect("fields")
                .insert(field.name().to_owned(), value.to_owned());
        }

        fn record_u64(&mut self, field: &field::Field, value: u64) {
            self.fields
                .0
                .lock()
                .expect("fields")
                .insert(field.name().to_owned(), value.to_string());
        }
    }

    #[test]
    fn bind_traceparent_to_span_records_trace_fields() {
        let context =
            TraceContext::parse("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01")
                .expect("valid traceparent");
        let fields = RecordedFields::default();
        let subscriber = tracing_subscriber::registry().with(RecordingLayer {
            fields: fields.clone(),
        });

        tracing::subscriber::with_default(subscriber, || {
            let span = info_span!(
                "test.span",
                trace_id = field::Empty,
                parent_id = field::Empty,
                trace_flags = field::Empty,
            );
            super::bind_traceparent_to_span(&span, &context);
        });

        assert_eq!(
            fields.get("trace_id").as_deref(),
            Some("0af7651916cd43dd8448eb211c80319c")
        );
        assert_eq!(fields.get("parent_id").as_deref(), Some("b7ad6b7169203331"));
        assert_eq!(fields.get("trace_flags").as_deref(), Some("01"));
    }

    #[cfg(unix)]
    #[test]
    fn open_trace_file_refuses_non_regular_existing_path() {
        let tmp = fresh_test_dir("non-regular");
        let path = tmp.join("trace-dir");
        std::fs::create_dir(&path).expect("directory sink fixture");

        let err =
            open_trace_file(path.to_str().expect("utf8 path")).expect_err("directory refused");

        assert!(
            err.to_string().contains("must be a regular file"),
            "unexpected error: {err}"
        );

        std::fs::remove_dir_all(&tmp).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn open_trace_file_accepts_private_regular_file() {
        let tmp = fresh_test_dir("regular-file");
        let path = tmp.join("trace.jsonl");
        std::fs::write(&path, b"").expect("trace file fixture");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("private mode");

        open_trace_file(path.to_str().expect("utf8 path")).expect("regular file accepted");

        std::fs::remove_dir_all(&tmp).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn open_trace_file_missing_path_open_refuses_symlink_substitution() {
        let tmp = fresh_test_dir("symlink-substitution");
        let path = tmp.join("trace.jsonl");
        let target = tmp.join("target.jsonl");
        symlink(&target, &path).expect("symlink trace path");

        let err = open_trace_file_after_validation(&path, true)
            .expect_err("create-new open must reject symlink substitution");

        assert!(
            err.to_string().contains("File exists") || err.to_string().contains("exists"),
            "unexpected error: {err}"
        );
        assert!(
            !target.exists(),
            "create-new trace open must not create the symlink target"
        );

        std::fs::remove_dir_all(&tmp).expect("cleanup");
    }
}
