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
//!   sensible per-binary default directive. **No redaction layer is
//!   wired in TRACE-001** — see below.
//! - [`redaction`] holds an **advisory-only** deny-list of sensitive
//!   field names. The runtime subscriber installed by [`init_tracing`]
//!   does NOT consult it; secret-bearing span attributes will appear in
//!   plain text in JSON output. TRACE-003 wires the actual layer
//!   against this same constant table (DA-OBS-004 risk acceptance per
//!   ADR-035 R1).
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

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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
        .with_current_span(true)
        .with_span_list(false);

    let subscriber = tracing_subscriber::registry().with(resolved).with(layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|_| InitTracingError::AlreadyInstalled)?;

    tracing::info!(
        target: "anvil_observability",
        binary = kind.as_str(),
        filter = %filter_repr,
        "tracing subscriber installed",
    );
    Ok(())
}
