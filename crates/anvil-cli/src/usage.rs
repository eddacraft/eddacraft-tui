//! USAGE-001: command-invocation usage observations.
//!
//! Records one `command.invoked` Kindling row per user-initiated CLI
//! invocation so the founder can answer "who is using what" for
//! dev-investment decisions (module: `plans/modules/usage-analytics.aps.md`).
//!
//! ## What is captured
//!
//! Per the privacy contract published at
//! `docs/observability/usage-analytics.md`: the command name, an
//! anonymised principal (one-way hash of the user's email with a
//! per-deployment salt — or the literal `anonymous` when unauthenticated),
//! a timestamp, the *redacted shape* of each argument (name plus value
//! type, coarse length bucket, and presence — never the value), an
//! inline `flag_set`
//! (empty in USAGE-001; USAGE-002 populates it per ADR-041), and the
//! incoming W3C `traceparent` when one is bound.
//!
//! ## What is NOT captured
//!
//! Raw argument values, command results/output, file contents, and
//! anything about the *value* of a sensitive-named argument. Argument
//! redaction defers to [`anvil_observability::redaction`] — the same
//! `SENSITIVE_FIELDS` deny-list the tracing pipe uses.
//!
//! ## Storage
//!
//! Usage is a cross-cutting, user-scoped signal (not per-repository),
//! so rows are appended to `<credentials_dir>/kindling/usage.ndjson` —
//! the user/deployment state directory, which re-roots under a gated
//! `ANVIL_HOME` (DISTRIB-006 / ADR-060) exactly like credentials. This
//! mirrors the audit-chain NDJSON sidecar pattern; the
//! Kindling-integration consumer tails the file. The Rust→TS `SQLite`
//! bridge remains a stack-wide deferred follow-up and is out of scope
//! here.
//!
//! ## Producer wiring
//!
//! [`record_invocation`] is called once from `main`, **after the
//! auth/routing phase** (so `flag_set` carries the flags resolved while
//! authorising — USAGE-002) but **before command dispatch**, on both the
//! auth-pass and auth-fail paths. It fires uniformly for *every* command:
//! there is no per-command wiring to forget — adding a new subcommand
//! cannot bypass the producer (R2 mitigation). Emission is strictly
//! best-effort: a failure is logged and dropped, never surfaced to the
//! exit code.
//!
//! ## JSON-RPC daemon producer (USAGE-004)
//!
//! USAGE-001 surfaced that the JSON-RPC daemon dispatch boundary
//! (`anvil-intercept::ipc`) carries no user principal and no flag
//! resolver. USAGE-004 resolves that: the client now attaches its
//! salted-hash principal on the JSON-RPC envelope, and the daemon emits
//! a `command.invoked` row for an explicit allowlist of user-initiated
//! methods (the GCTX query tools + the operator `unblock-*` verbs) to
//! this *same* sidecar via [`daemon_usage_emitter`]. `flag_set` stays
//! empty on the daemon path (no resolver there). The path is resolved
//! here ([`default_usage_log_path`]) so the daemon and CLI never diverge
//! on the credentials/`ANVIL_HOME` re-rooting.

use std::env;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anvil_intercept::kindling_observation::{
    CommandInvocationContext, CommandInvokedEmitter, CommandInvokedObservation,
    ConstraintAppliedObservation, DEFAULT_SAVE_TIME_PASS_CAPACITY, DEFAULT_SAVE_TIME_PASS_WINDOW,
    FalsePositiveReportContext, FlagSetEntry, GateEvaluatedObservation, KindlingObservationSink,
    KindlingSinkError, NonBlockingObservationSink, SAVE_TIME_GATE_ID, SaveTimeObservationEmitter,
    from_command_invocation, from_fp_report,
};
use anvil_intercept::rate_window::RateWindow;
use anvil_kernel::feature_flags::{CapturedResolution, ResolutionReason, take_captured_flags};
use anvil_observability::TraceContext;
use anvil_observability::redaction::{ArgShape, redact_arg};
use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::credentials;

/// NDJSON sidecar filename for usage rows.
const USAGE_NDJSON: &str = "usage.ndjson";
/// Per-deployment salt filename (sibling of the kindling sidecar dir).
const SALT_FILE: &str = "usage.salt";
/// Principal recorded when no identity is on the call path.
const ANONYMOUS_PRINCIPAL: &str = "anonymous";

/// One-way anonymise a principal (the user's email) with a
/// per-deployment `salt`.
///
/// Returns the literal [`ANONYMOUS_PRINCIPAL`] when no email is on the
/// call path. The raw `email` is consumed only to feed the hash and is
/// never returned — assert this invariant in the contract test.
#[must_use]
pub fn anonymise_principal(email: Option<&str>, salt: &[u8]) -> String {
    match email {
        None => ANONYMOUS_PRINCIPAL.to_string(),
        Some(email) => {
            let mut hasher = Sha256::new();
            hasher.update(salt);
            hasher.update(b":");
            hasher.update(email.as_bytes());
            hex::encode(hasher.finalize())
        }
    }
}

/// Load the per-deployment usage salt from `dir`, creating it on first
/// use.
///
/// Stored as hex at `<dir>/usage.salt`, mode `0600` on Unix. 256 bits
/// of entropy sourced from two v4 UUIDs (getrandom-backed). Rotating
/// the salt is a deliberate privacy reset — every historical principal
/// hash becomes unjoinable — not a routine operation. A corrupt or
/// empty salt file is regenerated rather than failing the invocation
/// record.
fn load_or_create_salt_in(dir: &Path) -> Result<Vec<u8>> {
    let path = dir.join(SALT_FILE);
    if let Some(bytes) = read_salt(&path) {
        return Ok(bytes);
    }
    create_private_dir(dir).with_context(|| format!("create salt dir {}", dir.display()))?;
    let mut salt = Vec::with_capacity(32);
    salt.extend_from_slice(Uuid::new_v4().as_bytes());
    salt.extend_from_slice(Uuid::new_v4().as_bytes());
    match write_salt_exclusive(&path, &salt) {
        Ok(()) => Ok(salt),
        // Lost a first-run race with a sibling process: it created the
        // salt first. Use the winner's salt so both processes hash to
        // the same principal — never overwrite an existing salt.
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            read_salt(&path).ok_or_else(|| {
                anyhow::anyhow!("salt file {} exists but is unreadable", path.display())
            })
        }
        Err(err) => Err(err).with_context(|| format!("write salt file {}", path.display())),
    }
}

/// Read and hex-decode a non-empty salt, or `None` if absent/corrupt
/// (a corrupt salt is regenerated rather than failing the record).
fn read_salt(path: &Path) -> Option<Vec<u8>> {
    let contents = fs::read_to_string(path).ok()?;
    let bytes = hex::decode(contents.trim()).ok()?;
    (!bytes.is_empty()).then_some(bytes)
}

/// Atomically create the salt file as hex (fails if it already exists),
/// mode `0600` on Unix. On non-Unix platforms no permission restriction
/// is applied (the Windows state-hardening gap is tracked separately,
/// alongside DSV-010/011).
fn write_salt_exclusive(path: &Path, salt: &[u8]) -> io::Result<()> {
    let encoded = hex::encode(salt);
    let mut opts = fs::OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(encoded.as_bytes())
}

/// Whether the next token looks like an option's value rather than the
/// next option. A token is a value when it does not start with `-`, or
/// when it is a negative number (`-5`, `-3.14`) — the latter is a value,
/// not a flag.
fn next_is_value(next: Option<&&String>) -> bool {
    matches!(next, Some(tok) if is_value_token(tok))
}

fn is_value_token(tok: &str) -> bool {
    // A bare token is a value; a `-`-prefixed token is a value only when
    // the remainder parses as a number (so `-5`/`-3.14` are values but
    // `-.`, `-x`, and `--flag` are not).
    tok.strip_prefix('-')
        .is_none_or(|rest| rest.parse::<f64>().is_ok())
}

/// Derive redacted argument shapes from a raw `argv` vector.
///
/// `argv[0]` (the program) is skipped. The first bare (non-`-`) token is
/// the subcommand — already captured separately as the command name — so
/// it is skipped too; this is located dynamically rather than by fixed
/// position, so global flags placed *before* the subcommand
/// (`anvil --json version`) are still recorded correctly. The `--`
/// end-of-options sentinel switches the remaining tokens to positionals.
///
/// Each `--name=value` / `--name value` / `-x value` option records its
/// name plus value shape; a bare option records the name with no value;
/// further bare tokens record `positional` with shape only. Every value
/// passes through [`redact_arg`], so no raw value is ever retained and
/// sensitive-named options are elided to the `<redacted>` marker.
///
/// Known fidelity limitations (no value ever leaks): short-flag clusters
/// (`-vj`) record the raw cluster as one name; nested subcommand tokens
/// (`login` in `anvil auth login`) surface as a `positional`.
#[must_use]
pub fn arg_shapes_from_argv(argv: &[String]) -> Vec<ArgShape> {
    let mut tokens = argv.iter().skip(1).peekable();
    let mut shapes = Vec::new();
    let mut seen_subcommand = false;
    let mut past_separator = false;
    while let Some(tok) = tokens.next() {
        if past_separator {
            shapes.push(redact_arg("positional", Some(tok)));
            continue;
        }
        if tok == "--" {
            past_separator = true;
            continue;
        }
        if let Some(rest) = tok.strip_prefix("--") {
            if let Some((name, value)) = rest.split_once('=') {
                shapes.push(redact_arg(name, Some(value)));
            } else if next_is_value(tokens.peek()) {
                let value = tokens.next().expect("peeked a value");
                shapes.push(redact_arg(rest, Some(value)));
            } else {
                shapes.push(redact_arg(rest, None));
            }
        } else if let Some(rest) = tok.strip_prefix('-')
            && !rest.is_empty()
        {
            if next_is_value(tokens.peek()) {
                let value = tokens.next().expect("peeked a value");
                shapes.push(redact_arg(rest, Some(value)));
            } else {
                shapes.push(redact_arg(rest, None));
            }
        } else if seen_subcommand {
            // A bare token after the subcommand is a positional value.
            shapes.push(redact_arg("positional", Some(tok)));
        } else {
            // The first bare token is the subcommand (captured as the
            // command name); drop it.
            seen_subcommand = true;
        }
    }
    shapes
}

/// Append one usage observation as a single NDJSON line to `path`,
/// creating the parent directory if needed.
///
/// The sidecar holds per-invocation principals and argument metadata, so
/// it is created owner-only (`0600`) under an owner-only parent (`0700`)
/// on Unix — matching the salt's posture so a shared host can't read the
/// usage history. A symlinked target is refused (no `O_NOFOLLOW` dep
/// needed) so a pre-planted symlink can't redirect the append.
fn append_usage_observation_to(path: &Path, obs: &CommandInvokedObservation) -> Result<()> {
    append_observation_to(path, obs)
}

/// DPO-001: append a `gate_evaluated` row to the shared usage sidecar.
/// Reuses the same private-dir + symlink-refusal + `0600` + retention
/// pattern as [`append_usage_observation_to`]; only `gate_evaluated`
/// rows whose `gate_id` is `save-time` reach this helper (the
/// [`DaemonObservationSink`] gate keeps mid-edit / audit rows out).
fn append_gate_evaluated_to(path: &Path, obs: &GateEvaluatedObservation) -> Result<()> {
    append_observation_to(path, obs)
}

/// DPO-002: append a `constraint_applied` (fence-engage) row to the
/// shared usage sidecar, with the same write posture + retention.
fn append_constraint_applied_to(path: &Path, obs: &ConstraintAppliedObservation) -> Result<()> {
    append_observation_to(path, obs)
}

/// Append one observation as a single NDJSON line to `path`, creating the
/// parent directory if needed, generic over any serialisable row so the
/// three kinds (`command.invoked`, `gate_evaluated(save-time)`,
/// `constraint_applied`) share one write path.
///
/// The sidecar holds per-invocation principals and argument metadata, so
/// it is created owner-only (`0600`) under an owner-only parent (`0700`)
/// on Unix — matching the salt's posture so a shared host can't read the
/// usage history. A symlinked target is refused (no `O_NOFOLLOW` dep
/// needed) so a pre-planted symlink can't redirect the append. Before the
/// append the sidecar is lazily trimmed (see [`trim_usage_sidecar`]) to
/// the 7-day / 64 MiB retention bounds (council T5).
fn append_observation_to<T: serde::Serialize>(path: &Path, obs: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_private_dir(parent)
            .with_context(|| format!("create kindling dir {}", parent.display()))?;
    }
    // Refuse to follow a symlink at the target path.
    if let Ok(meta) = fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        anyhow::bail!(
            "usage sidecar {} is a symlink; refusing to write",
            path.display()
        );
    }
    // Retention (council T5): trim before the append so the sidecar stays
    // bounded. Best-effort — a trim failure must not block the write.
    trim_usage_sidecar(path);
    let serialised = serde_json::to_string(obs).context("serialise usage observation")?;
    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts
        .open(path)
        .with_context(|| format!("open usage sidecar {}", path.display()))?;
    // `OpenOptions::mode` only applies when the file is *created*. Enforce
    // `0600` on an already-existing sidecar too (best-effort), so a file
    // left world-readable by a previous run/version is tightened.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
    }
    writeln!(f, "{serialised}")
        .with_context(|| format!("append usage row to {}", path.display()))?;
    Ok(())
}

/// DPO retention (council T5): the maximum age a usage sidecar row is
/// kept. Rows whose ISO-8601 `timestamp` is older than this are dropped
/// from the front of the file on the next append.
///
/// Built from a seconds constant rather than `Duration::from_days` —
/// `from_days` is still unstable on Rust 1.95 (same workaround as
/// `commands::status`); routing through the named constant also keeps the
/// literal away from the `clippy::duration_suboptimal_units` lint.
const USAGE_SIDECAR_MAX_AGE: Duration = {
    const DAY_SECS: u64 = 86_400;
    Duration::from_secs(7 * DAY_SECS)
};

/// DPO retention (council T5): the maximum size the usage sidecar may
/// reach before the oldest lines are dropped. 64 MiB.
const USAGE_SIDECAR_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Env escape hatch: any non-empty value disables sidecar trimming.
const USAGE_SIDECAR_NO_TRIM_ENV: &str = "ANVIL_USAGE_SIDECAR_NO_TRIM";

/// DPO retention (council T5): lazily trim the usage sidecar to the
/// [`USAGE_SIDECAR_MAX_AGE`] (7-day) and [`USAGE_SIDECAR_MAX_BYTES`]
/// (64 MiB) bounds. Best-effort and deterministic:
///
/// - Drops leading lines whose parsed `timestamp` is older than the
///   max-age cut-off (computed from `now`).
/// - If the file still exceeds the byte cap, drops further oldest lines
///   until it is under.
/// - A line without a parseable `timestamp` is treated as recent and
///   KEPT (never crashed on, never silently dropped on a parse miss) so a
///   malformed row can't trigger data loss.
///
/// Disabled entirely when `ANVIL_USAGE_SIDECAR_NO_TRIM=1` is set. Any IO
/// error is swallowed — retention is housekeeping, never a write blocker.
fn trim_usage_sidecar(path: &Path) {
    if env::var_os(USAGE_SIDECAR_NO_TRIM_ENV).is_some_and(|v| !v.is_empty()) {
        return;
    }
    trim_usage_sidecar_at(path, Utc::now());
}

/// Testable core of [`trim_usage_sidecar`] with an injected clock. `now`
/// drives the age cut-off so tests can age rows deterministically.
///
/// Best-effort observability housekeeping (council A): the
/// read→write-tmp→rename rewrite races with concurrent `O_APPEND` writers
/// (the non-blocking drain thread and any separate CLI process). A row
/// appended during the rewrite window can be lost. This is ACCEPTED — the
/// usage sidecar holds best-effort observability rows, not
/// billing/audit-critical state; the authoritative records live elsewhere
/// (the kindling store once KDS lands, and the persistent fence-state file
/// for fence engages). No file lock is taken (no new dependency); council
/// B's fast-path gate keeps the rewrite rare (only near the cap or with a
/// stale head), shrinking the race window further.
fn trim_usage_sidecar_at(path: &Path, now: chrono::DateTime<Utc>) {
    let Ok(meta) = fs::metadata(path) else {
        return; // No file yet — nothing to trim.
    };
    let size = meta.len();
    if size == 0 {
        return;
    }
    let cutoff = now - chrono::Duration::from_std(USAGE_SIDECAR_MAX_AGE).unwrap_or_default();

    // Council B fast-path: avoid reading + rewriting the whole file on every
    // append. The full read is only needed when EITHER a size trim is due
    // (file at/over the byte cap) OR an age trim might be due (the FIRST,
    // oldest line is stale). Reading just the first line is cheap; if the
    // file is under the cap AND the head line is not stale, there is nothing
    // to trim, so return without the full read. A malformed/unreadable first
    // line falls through to the full read (correctness over the fast path).
    let needs_size_trim = size >= USAGE_SIDECAR_MAX_BYTES;
    if !needs_size_trim && !first_line_is_stale(path, cutoff) {
        return;
    }

    let Ok(contents) = fs::read_to_string(path) else {
        return; // Unreadable (or non-UTF8) — leave it untouched.
    };
    let lines: Vec<&str> = contents.lines().collect();

    // Index of the first line to KEEP after dropping stale leading rows.
    let mut start = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        if line_is_older_than(line, cutoff) {
            start = idx + 1;
        } else {
            // A kept (recent or unparseable) line ends the leading-stale
            // run — the file is append-ordered so nothing older follows.
            break;
        }
    }

    // If still over the byte cap, drop further oldest lines until under.
    // Each line costs its bytes plus the newline.
    let mut remaining: u64 = lines[start..].iter().map(|l| l.len() as u64 + 1).sum();
    while remaining > USAGE_SIDECAR_MAX_BYTES && start < lines.len() {
        remaining -= lines[start].len() as u64 + 1;
        start += 1;
    }

    if start == 0 {
        return; // Nothing trimmed — avoid a needless rewrite.
    }

    let kept = &lines[start..];
    let mut rewritten = kept.join("\n");
    if !rewritten.is_empty() {
        rewritten.push('\n');
    }
    // Best-effort atomic-ish replace: write a temp sibling then rename.
    // A failure leaves the original intact (retention is housekeeping).
    let tmp = path.with_extension("ndjson.trim.tmp");
    // Council A: apply the same symlink-refusal guard the sidecar itself
    // uses to the trim temp path — a pre-planted symlink at the `.trim.tmp`
    // location must not be allowed to redirect the truncating write to an
    // attacker-chosen target. Refuse and bail (best-effort housekeeping
    // never escalates to a write through a symlink).
    if let Ok(meta) = fs::symlink_metadata(&tmp)
        && meta.file_type().is_symlink()
    {
        return;
    }
    if write_private_file(&tmp, rewritten.as_bytes()).is_ok() {
        // `fs::rename` replaces an existing destination on both Unix and
        // Windows (the latter via `MoveFileExW` + `REPLACE_EXISTING`). If it
        // still fails (e.g. a transient Windows sharing violation), drop the
        // temp so it cannot accumulate — retention is best-effort housekeeping.
        if fs::rename(&tmp, path).is_err() {
            let _ = fs::remove_file(&tmp);
        }
    } else {
        let _ = fs::remove_file(&tmp);
    }
}

/// Council B fast-path helper: cheaply test whether the FIRST (oldest) line
/// of the sidecar is older than `cutoff`, reading only that line rather
/// than the whole file. Returns `false` when the file cannot be opened, the
/// first line cannot be read, or the line is not stale — and `true` only
/// when the head line parses to a `timestamp` strictly older than `cutoff`.
///
/// A malformed/unreadable first line returns `false`; the caller treats
/// that as "fast-path inconclusive" and falls through to the full read,
/// which has the authoritative malformed-line-keeps semantics. So a
/// malformed head never causes data loss here either.
fn first_line_is_stale(path: &Path, cutoff: chrono::DateTime<Utc>) -> bool {
    use std::io::BufRead as _;

    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let mut reader = io::BufReader::new(file);
    let mut first = String::new();
    match reader.read_line(&mut first) {
        Ok(n) if n > 0 => line_is_older_than(first.trim_end_matches('\n'), cutoff),
        // Empty file (0 bytes) or read error: inconclusive — fall through.
        _ => false,
    }
}

/// Whether an NDJSON line's parsed `timestamp` is strictly older than
/// `cutoff`. A line that does not parse, or has no `timestamp`, or has an
/// unparseable timestamp, returns `false` (KEEP it) so a malformed row is
/// never the cause of data loss.
fn line_is_older_than(line: &str, cutoff: chrono::DateTime<Utc>) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    let Some(ts) = value.get("timestamp").and_then(serde_json::Value::as_str) else {
        return false;
    };
    match chrono::DateTime::parse_from_rfc3339(ts) {
        Ok(parsed) => parsed.with_timezone(&Utc) < cutoff,
        Err(_) => false,
    }
}

/// Write `bytes` to `path` owner-only (`0600` on Unix), truncating any
/// existing file. Used for the retention temp file.
fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(bytes)
}

/// Create a directory (and parents) owner-only (`0700`) on Unix.
fn create_private_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(dir)
    }
}

/// Read and validate the incoming W3C `traceparent` from the
/// environment.
///
/// Returns the canonical header form when the `TRACEPARENT` env var
/// holds a valid context, `None` otherwise. TRACE-004 will bind this
/// onto the active span; until then the env var is the propagation
/// channel and an absent/invalid value is simply not recorded.
fn incoming_traceparent() -> Option<String> {
    let raw = env::var("TRACEPARENT").ok()?;
    TraceContext::parse(raw.trim())
        .ok()
        .map(|tc| tc.as_header())
}

/// Record one `command.invoked` usage observation for the current
/// invocation.
///
/// Best-effort and side-effect-only: callers MUST log-and-drop the
/// `Result` so a usage-write failure never changes the command's
/// behaviour or exit code. Writes to the user-scoped state directory
/// (`credentials_dir`), which already honours a gated `ANVIL_HOME`, so
/// no separate project-write gating is needed.
/// USAGE-004: resolve the current invocation's salted-hash principal —
/// the same value [`record_invocation`] stamps on CLI rows.
///
/// JSON-RPC clients (the MCP query tools, the `intercept` operator verbs)
/// call this and attach the result on the request envelope so the
/// daemon-side producer records an attributable row instead of a
/// principal-less one. An absent/unreadable credential resolves to the
/// `anonymous` principal (parity with an unauthenticated CLI run); the
/// raw email is never returned.
#[cfg_attr(windows, allow(dead_code))]
pub fn current_principal() -> Result<String> {
    let state_dir = credentials::credentials_dir().context("resolve usage state directory")?;
    resolve_principal_in(&state_dir)
}

/// Resolve the salted-hash principal under an explicit state directory:
/// load (or create) the per-deployment salt and hash the loaded email,
/// or `anonymous` when no credential is present. Shared by
/// [`current_principal`] (client/daemon-attach path) and
/// [`record_invocation`] (CLI producer) so the salt-load + hashing logic
/// has one home.
fn resolve_principal_in(state_dir: &Path) -> Result<String> {
    let salt = load_or_create_salt_in(state_dir)?;
    // Credential load failure or absence is fine — an unauthenticated
    // invocation records the `anonymous` principal.
    let email = credentials::load().ok().flatten().and_then(|c| c.email);
    Ok(anonymise_principal(email.as_deref(), &salt))
}

/// USAGE-004: attach the current salted-hash principal to a JSON-RPC
/// request `frame` so the daemon-side producer can attribute the row to
/// the same principal CLI rows carry.
///
/// Best-effort: if the principal cannot be resolved, or `frame` is not a
/// JSON object, the frame is left unchanged and the daemon records the
/// `anonymous` principal (parity with an unauthenticated run). Never puts
/// a raw email on the wire — only the one-way hash.
#[cfg_attr(windows, allow(dead_code))]
pub fn attach_principal(frame: &mut serde_json::Value) {
    if let Ok(principal) = current_principal()
        && let Some(obj) = frame.as_object_mut()
    {
        obj.insert("principal".to_owned(), serde_json::Value::String(principal));
    }
}

pub fn record_invocation(command_name: &str) -> Result<()> {
    let state_dir = credentials::credentials_dir().context("resolve usage state directory")?;
    let principal = resolve_principal_in(&state_dir)?;

    let argv: Vec<String> = env::args().collect();
    let arg_shapes = arg_shapes_from_argv(&argv);
    let traceparent = incoming_traceparent();

    let session_id = Uuid::new_v4().to_string();
    // `Z`-suffixed RFC 3339 (not the `+00:00` offset form) so the TS-side
    // Zod `datetime()` validator accepts it — matches the convention used
    // across the codebase (ipc, telemetry, hook, insights).
    let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let ctx = CommandInvocationContext {
        session_id: &session_id,
        timestamp: &timestamp,
        command: command_name,
        principal: &principal,
        traceparent: traceparent.as_deref(),
    };
    // USAGE-002: populate `flag_set` from the flags resolved during the
    // auth/routing phase (the capture window opened by `main` before the
    // auth gate). Drains the kernel sink; empty when no flags resolved.
    let flag_set = flag_set_from_captured(take_captured_flags());
    let obs = from_command_invocation(&ctx, arg_shapes, flag_set);

    let path = usage_log_path(&state_dir);
    append_usage_observation_to(&path, &obs)
}

/// Build the inline `flag_set` (ADR-041) from the flags captured during
/// the auth/routing phase (USAGE-002).
///
/// Maps each capture's resolution reason to the ADR-041 `source`
/// vocabulary (`override` / `snapshot` / `default`), drops errored
/// resolutions (not a clean context fact), deduplicates by canonical
/// `key` (last write wins), and sorts by `key` so query fixtures and
/// diffs are stable.
fn flag_set_from_captured(captured: Vec<CapturedResolution>) -> Vec<FlagSetEntry> {
    let mut by_key: std::collections::BTreeMap<String, FlagSetEntry> =
        std::collections::BTreeMap::new();
    for cap in captured {
        let Some(source) = adr041_source(&cap.reason) else {
            continue;
        };
        by_key.insert(
            cap.key.clone(),
            FlagSetEntry {
                key: cap.key,
                variant: cap.variant,
                source: source.to_owned(),
                gate_affecting: cap.gate_affecting,
            },
        );
    }
    by_key.into_values().collect()
}

/// Map a resolver reason to the ADR-041 `source` vocabulary, or `None`
/// for an errored resolution (not recorded as context).
fn adr041_source(reason: &ResolutionReason) -> Option<&'static str> {
    match reason {
        ResolutionReason::EmergencyOverride | ResolutionReason::LocalOverride => Some("override"),
        ResolutionReason::TargetingMatch => Some("snapshot"),
        ResolutionReason::Default | ResolutionReason::Disabled => Some("default"),
        ResolutionReason::Error => None,
    }
}

/// Resolve the usage NDJSON path under the user-scoped state directory.
fn usage_log_path(state_dir: &Path) -> PathBuf {
    state_dir.join("kindling").join(USAGE_NDJSON)
}

/// Default path to the usage NDJSON sidecar under the user-scoped state
/// directory — the read-side counterpart used by the USAGE-003 query
/// views (`anvil kindling usage <view>`). Resolves the same
/// `credentials_dir` (honouring a gated `ANVIL_HOME`) as the producer, so
/// the views read exactly what [`record_invocation`] wrote.
pub fn default_usage_log_path() -> Result<PathBuf> {
    let state_dir = credentials::credentials_dir().context("resolve usage state directory")?;
    Ok(usage_log_path(&state_dir))
}

// ── False-positive reporting (OPSUP-007 / ADR-089) ──────────────────────

/// NDJSON sidecar filename for false-positive reports — a sibling of
/// `usage.ndjson` under the same `kindling/` dir.
const FALSE_POSITIVE_NDJSON: &str = "false-positives.ndjson";

/// One-way, salted hash of a file path for a false-positive report. Uses
/// the per-deployment usage salt so the digest is unjoinable across
/// deployments and the plaintext path is never recorded (OPSUP-007). A
/// distinct domain separator from the principal hash keeps the two
/// keyspaces independent.
fn hash_file_path(path: &str, salt: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(b":fp-path:");
    hasher.update(path.as_bytes());
    hex::encode(hasher.finalize())
}

/// Resolve the false-positive NDJSON path under the user-scoped state dir.
fn fp_log_path(state_dir: &Path) -> PathBuf {
    state_dir.join("kindling").join(FALSE_POSITIVE_NDJSON)
}

/// OPSUP-007 / ADR-089: record a false-positive report to the local
/// Kindling sidecar — the destination is the local record; nothing leaves
/// the machine (no network call, air-gap-safe).
///
/// Mirrors [`record_invocation`]'s anonymisation posture: a salted-hash
/// principal and a **salted-hash file path** (never the plaintext path),
/// and **no source content** unless the caller supplies an opt-in
/// `snippet` (fail-closed: `None` by default). `check_id` is expected to be
/// an already-resolved stable `ANV-*` ID (the command validates it against
/// the OPSUP-001 registry before calling).
pub fn record_false_positive(
    check_id: &str,
    path: &str,
    line: u32,
    snippet: Option<String>,
) -> Result<()> {
    let state_dir = credentials::credentials_dir().context("resolve usage state directory")?;
    record_false_positive_in(&state_dir, check_id, path, line, snippet)
}

/// Testable core of [`record_false_positive`] under an explicit state dir
/// (mirrors the `*_in` split used for the principal/salt logic).
fn record_false_positive_in(
    state_dir: &Path,
    check_id: &str,
    path: &str,
    line: u32,
    snippet: Option<String>,
) -> Result<()> {
    let salt = load_or_create_salt_in(state_dir)?;
    let principal = resolve_principal_in(state_dir)?;
    let hashed_path = hash_file_path(path, &salt);

    let session_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let traceparent = incoming_traceparent();

    let ctx = FalsePositiveReportContext {
        session_id: &session_id,
        timestamp: &timestamp,
        check_id,
        hashed_path: &hashed_path,
        line,
        principal: &principal,
        traceparent: traceparent.as_deref(),
    };
    let obs = from_fp_report(&ctx, snippet);
    append_observation_to(&fp_log_path(state_dir), &obs)
}

/// USAGE-004: a [`KindlingObservationSink`] that appends `command.invoked`
/// rows produced by the JSON-RPC daemon dispatch to the *same*
/// user-scoped `usage.ndjson` sidecar the CLI producer writes (resolved
/// via the CLI-owned [`default_usage_log_path`], so the daemon and CLI
/// never diverge on the path — the credentials/`ANVIL_HOME` re-rooting
/// logic lives only here). Only `command.invoked` is consumed by this
/// sink; the `gate_evaluated(save-time)` and `constraint_applied` kinds
/// are persisted to the SAME sidecar through the DPO sink
/// [`DaemonObservationSink`].
///
/// The path is resolved once at construction so a per-call resolution
/// failure cannot occur on the dispatch hot path; a write failure is
/// surfaced as [`KindlingSinkError::Unavailable`] and swallowed by the
/// emitter (logged, row dropped — dispatch is never coupled to sink
/// health).
#[derive(Debug)]
struct DaemonUsageSink {
    path: PathBuf,
}

impl KindlingObservationSink for DaemonUsageSink {
    fn try_emit(&self, _observation: GateEvaluatedObservation) -> Result<(), KindlingSinkError> {
        // This sink owns command.invoked only; gate_evaluated is persisted
        // through `DaemonObservationSink` (DPO-001).
        Ok(())
    }

    fn try_emit_command_invoked(
        &self,
        observation: CommandInvokedObservation,
    ) -> Result<(), KindlingSinkError> {
        append_usage_observation_to(&self.path, &observation)
            .map_err(|err| KindlingSinkError::Unavailable(err.to_string()))
    }
}

/// DPO-001 / DPO-002: a [`KindlingObservationSink`] that persists the two
/// newly-activated producer kinds — `gate_evaluated(save-time)` and
/// `constraint_applied` — to the SAME user-scoped `usage.ndjson` sidecar
/// the CLI / USAGE-004 producers write, with the same private-dir +
/// symlink-refusal + `0600` + retention posture.
///
/// `try_emit` persists a `gate_evaluated` row ONLY when its `gate_id`
/// equals [`SAVE_TIME_GATE_ID`] (`save-time`); rows from other gates
/// (mid-edit, audit-chain) are silently ignored so this sink never
/// scoops up rows from surfaces DPO does not own. `command.invoked` rows
/// are NOT consumed here (the [`DaemonUsageSink`] owns those) so a sink
/// shared between both producers does not double-write usage rows.
///
/// A write failure is surfaced as [`KindlingSinkError::Unavailable`];
/// behind the [`NonBlockingObservationSink`] decorator it is logged on
/// the drain thread and dropped, never coupling the verdict / engage hot
/// path to sink health.
// KDS-005: retire alongside DaemonUsageSink when the kindling daemon store
// lands (the NDJSON sidecar is the interim transport until the SQLite
// bridge exists).
#[derive(Debug)]
struct DaemonObservationSink {
    path: PathBuf,
}

impl KindlingObservationSink for DaemonObservationSink {
    fn try_emit(&self, observation: GateEvaluatedObservation) -> Result<(), KindlingSinkError> {
        // Only the save-time gate is DPO's to persist here; ignore every
        // other gate_id so mid-edit / audit rows are never silently
        // grabbed onto the usage sidecar.
        if observation.gate_id != SAVE_TIME_GATE_ID {
            return Ok(());
        }
        append_gate_evaluated_to(&self.path, &observation)
            .map_err(|err| KindlingSinkError::Unavailable(err.to_string()))
    }

    fn try_emit_constraint_applied(
        &self,
        observation: ConstraintAppliedObservation,
    ) -> Result<(), KindlingSinkError> {
        append_constraint_applied_to(&self.path, &observation)
            .map_err(|err| KindlingSinkError::Unavailable(err.to_string()))
    }
}

/// USAGE-004: build the daemon-side command-invocation usage emitter,
/// wired to the shared `usage.ndjson` sink. Returns `None` (usage export
/// off) when the usage path cannot be resolved, so a daemon on a host
/// without a resolvable state dir still starts. The per-startup
/// `daemon_session_id` stamps every daemon row; individual calls are
/// correlated by `traceparent`, not this id.
#[must_use]
pub fn daemon_usage_emitter() -> Option<Arc<CommandInvokedEmitter>> {
    let path = default_usage_log_path()
        .map_err(|err| {
            tracing::warn!(
                target: "anvil::usage",
                error = %err,
                "usage export disabled: could not resolve usage sidecar path",
            );
        })
        .ok()?;
    let sink = Arc::new(DaemonUsageSink { path }) as Arc<dyn KindlingObservationSink>;
    Some(Arc::new(CommandInvokedEmitter::new(
        sink,
        Uuid::new_v4().to_string(),
    )))
}

/// DPO-001 / DPO-002: build the daemon-side save-time + fence observation
/// producers, both fanning into ONE shared non-blocking sink over the
/// shared `usage.ndjson` sidecar.
///
/// Returns `(save_time_emitter, shared_sink)`:
///
/// - The save-time emitter (DPO-001) the daemon attaches to its
///   `validate_paths` verdict path so each verdict (pass and fail)
///   produces a `gate_evaluated(save-time)` row.
/// - The shared sink (DPO-002) the daemon also hands to the fence surface
///   so each successful engage produces a `constraint_applied` row.
///
/// Both share ONE [`DaemonObservationSink`] wrapped in ONE
/// [`NonBlockingObservationSink`] (council T2: the producer hot path is
/// never back-pressured by the sidecar's blocking IO — a single drain
/// thread owns the write). Both also share ONE per-startup
/// `daemon_session_id` (a fresh v4 UUID) so a save-time row and a fence
/// row from the same daemon process carry an identical `session_id`.
///
/// `include_paths` is OFF unless `ANVIL_OBSERVATION_INCLUDE_PATHS=1`
/// (default-off: a clean verdict records only the path count, not the
/// validated paths) and is returned as the third tuple element so the
/// fence surface can apply the SAME posture to its `constraint_applied`
/// worktree field (council C). On an unresolvable usage path returns
/// `(None, None, false)` so a daemon on a host without a resolvable state
/// dir still starts.
///
/// Whole-DPO kill-switch (council J): when
/// `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1` is set, this returns
/// `(None, None, false)` with a `tracing::warn!` — no producers are wired
/// at all, mirroring the `ANVIL_INTERCEPT_DISABLE_SYMBOL_PARSER`
/// break-glass for the verdict path. This is the single env toggle that
/// silences both the save-time `gate_evaluated` and the fence
/// `constraint_applied` producers without a redeploy.
#[must_use]
pub fn daemon_observation_producers() -> (
    Option<Arc<SaveTimeObservationEmitter>>,
    Option<Arc<dyn KindlingObservationSink>>,
    bool,
) {
    // Whole-DPO kill-switch (council J): one env toggle disables every DPO
    // producer. Read fresh so an operator can flip it per daemon start.
    if env::var_os("ANVIL_INTERCEPT_DISABLE_OBSERVATION").is_some_and(|v| v == "1") {
        tracing::warn!(
            target: "anvil::usage",
            "ANVIL_INTERCEPT_DISABLE_OBSERVATION=1 — save-time + fence observation \
             producers disabled (break-glass)",
        );
        return (None, None, false);
    }

    let Some(path) = default_usage_log_path()
        .map_err(|err| {
            tracing::warn!(
                target: "anvil::usage",
                error = %err,
                "save-time/fence observation export disabled: could not resolve usage sidecar path",
            );
        })
        .ok()
    else {
        return (None, None, false);
    };

    let include_paths = env::var_os("ANVIL_OBSERVATION_INCLUDE_PATHS").is_some_and(|v| v == "1");
    let daemon_session_id = Uuid::new_v4().to_string();

    // ONE inner sink, ONE non-blocking decorator, shared by both producers.
    let inner = Arc::new(DaemonObservationSink { path }) as Arc<dyn KindlingObservationSink>;
    // The drain thread spawn is fallible (council G): a `None` sink means
    // the host could not start the drain thread, so degrade to no
    // observation export rather than crashing the daemon at startup.
    let Some(non_blocking) =
        NonBlockingObservationSink::new(inner, DEFAULT_OBSERVATION_CHANNEL_CAPACITY)
    else {
        tracing::warn!(
            target: "anvil::usage",
            "save-time/fence observation export disabled: could not start the \
             non-blocking observation drain thread",
        );
        return (None, None, false);
    };
    let shared_sink = Arc::new(non_blocking) as Arc<dyn KindlingObservationSink>;

    let emitter = Arc::new(SaveTimeObservationEmitter::new(
        Arc::clone(&shared_sink),
        RateWindow::new(
            DEFAULT_SAVE_TIME_PASS_CAPACITY,
            DEFAULT_SAVE_TIME_PASS_WINDOW,
        ),
        daemon_session_id,
        include_paths,
    ));

    (Some(emitter), Some(shared_sink), include_paths)
}

/// DPO-001: bound on the shared non-blocking observation channel. Past
/// this many queued rows the drain is far enough behind that dropping is
/// the right call (the next verdict / engage produces a fresh row); the
/// producer hot path never blocks. Sized generously so only a genuinely
/// stuck sidecar trips it.
const DEFAULT_OBSERVATION_CHANNEL_CAPACITY: usize = 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use tempfile::tempdir;

    // ── False-positive reporting (OPSUP-007 / ADR-089) ──────────────

    #[test]
    fn record_false_positive_hashes_path_and_omits_source_by_default() {
        let dir = tempdir().expect("tempdir");
        let plaintext_path = "src/secrets/loader.rs";

        record_false_positive_in(dir.path(), "ANV-CORE-001", plaintext_path, 42, None)
            .expect("record fp");

        let sidecar = fp_log_path(dir.path());
        let content = std::fs::read_to_string(&sidecar).expect("read sidecar");
        let row: serde_json::Value = serde_json::from_str(content.trim()).expect("one ndjson row");

        assert_eq!(row["kind"], "false_positive_reported");
        assert_eq!(row["check_id"], "ANV-CORE-001");
        assert_eq!(row["line"], 42);
        // The plaintext path must never appear; the hash must be present.
        assert!(
            !content.contains(plaintext_path),
            "plaintext path leaked into the record: {content}"
        );
        assert_eq!(
            row["hashed_path"],
            hash_file_path(plaintext_path, &load_or_create_salt_in(dir.path()).unwrap())
        );
        // No source content by default — the snippet field is omitted.
        assert!(
            row.get("snippet").is_none(),
            "source snippet must be omitted under the default config: {content}"
        );
    }

    #[test]
    fn record_false_positive_includes_snippet_only_when_opted_in() {
        let dir = tempdir().expect("tempdir");
        record_false_positive_in(
            dir.path(),
            "ANV-CORE-002",
            "a/b.rs",
            7,
            Some("let x = 1;".to_string()),
        )
        .expect("record fp");

        let content = std::fs::read_to_string(fp_log_path(dir.path())).expect("read");
        let row: serde_json::Value = serde_json::from_str(content.trim()).expect("row");
        assert_eq!(row["snippet"], "let x = 1;");
    }

    #[test]
    fn fp_path_hash_is_salted_and_deterministic() {
        let dir = tempdir().expect("tempdir");
        let salt = load_or_create_salt_in(dir.path()).expect("salt");
        let a = hash_file_path("src/x.rs", &salt);
        let b = hash_file_path("src/x.rs", &salt);
        let other = hash_file_path("src/y.rs", &salt);
        assert_eq!(a, b, "same path + salt hashes identically");
        assert_ne!(a, other, "different paths hash differently");
        assert!(!a.contains("src/x.rs"), "hash must not embed the path");
        // Distinct domain from the principal hash for the same input bytes.
        assert_ne!(a, anonymise_principal(Some("src/x.rs"), &salt));
    }

    #[test]
    fn anonymise_principal_is_anonymous_without_email() {
        assert_eq!(
            anonymise_principal(None, Uuid::new_v4().as_bytes()),
            ANONYMOUS_PRINCIPAL
        );
    }

    #[test]
    fn anonymise_principal_hashes_email_and_never_returns_it() {
        let salt = Uuid::new_v4();
        let principal = anonymise_principal(Some("josh@arkahna.io"), salt.as_bytes());
        // 32-byte SHA-256 digest rendered as hex.
        assert_eq!(principal.len(), 64);
        assert!(principal.chars().all(|c| c.is_ascii_hexdigit()));
        // The raw identity must never survive into the hash output.
        assert!(!principal.contains("josh"));
        assert!(!principal.contains('@'));
    }

    #[test]
    fn anonymise_principal_is_deterministic_per_salt() {
        let salt = Uuid::new_v4();
        let other_salt = Uuid::new_v4();
        let a = anonymise_principal(Some("user@example.com"), salt.as_bytes());
        let again = anonymise_principal(Some("user@example.com"), salt.as_bytes());
        let other_salt = anonymise_principal(Some("user@example.com"), other_salt.as_bytes());
        assert_eq!(a, again, "same salt + email must be stable");
        assert_ne!(a, other_salt, "rotating the salt must change the hash");
    }

    #[test]
    fn salt_is_created_then_reused_and_restricted() {
        let dir = tempdir().expect("tempdir");
        let first = load_or_create_salt_in(dir.path()).expect("create salt");
        assert_eq!(first.len(), 32, "256-bit salt");
        let second = load_or_create_salt_in(dir.path()).expect("reuse salt");
        assert_eq!(first, second, "salt must be stable across calls");

        let salt_path = dir.path().join(SALT_FILE);
        assert!(salt_path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&salt_path).expect("stat").permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "salt file must be owner-only");
        }
    }

    #[test]
    fn argv_shapes_skip_program_and_subcommand_and_redact() {
        let argv: Vec<String> = [
            "anvil",
            "check",
            "--path",
            "/home/me/secret-repo",
            "--json",
            "--token",
            "super-secret",
            "extra-positional",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();

        let shapes = arg_shapes_from_argv(&argv);
        let by_name = |n: &str| shapes.iter().find(|s| s.name == n).cloned();

        let path = by_name("path").expect("path arg");
        assert!(path.present);
        assert_eq!(path.redacted, None);

        let json = by_name("json").expect("json flag");
        assert!(!json.present, "bare flag has no value");

        let token = by_name("token").expect("token arg");
        assert_eq!(token.redacted.as_deref(), Some("<redacted>"));
        assert_eq!(token.length, None, "sensitive length must not leak");

        // `check` (the subcommand) is dropped, not recorded as a
        // positional; only the trailing `extra-positional` remains.
        assert!(
            by_name("positional").is_some(),
            "trailing positional recorded"
        );
        assert!(
            by_name("check").is_none(),
            "the subcommand token must not be recorded as an arg"
        );

        // No raw value may appear anywhere in the serialised shapes.
        let json_blob = serde_json::to_string(&shapes).expect("serialise");
        assert!(
            !json_blob.contains("secret-repo"),
            "raw value leaked: {json_blob}"
        );
        assert!(
            !json_blob.contains("super-secret"),
            "sensitive value leaked: {json_blob}"
        );
    }

    #[test]
    fn append_writes_one_ndjson_row_without_raw_values() {
        let dir = tempdir().expect("tempdir");
        let path = usage_log_path(dir.path());

        let salt = Uuid::new_v4();
        let principal = anonymise_principal(Some("user@example.com"), salt.as_bytes());
        let args = arg_shapes_from_argv(
            &[
                "anvil",
                "check",
                "--path",
                "/secret/place",
                "--token",
                "zzsecretzz",
            ]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        );
        let ctx = CommandInvocationContext {
            session_id: "33333333-3333-4333-8333-333333333333",
            timestamp: "2099-06-14T10:00:00Z",
            command: "check",
            principal: &principal,
            traceparent: None,
        };
        let obs = from_command_invocation(&ctx, args, Vec::new());

        append_usage_observation_to(&path, &obs).expect("append once");
        append_usage_observation_to(&path, &obs).expect("append twice");

        let contents = fs::read_to_string(&path).expect("read log");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "one NDJSON row per append");
        for line in &lines {
            let parsed: CommandInvokedObservation =
                serde_json::from_str(line).expect("valid NDJSON row");
            assert_eq!(parsed.kind, "command.invoked");
            assert_eq!(parsed.command, "check");
        }
        assert!(!contents.contains("user@example.com"), "raw email leaked");
        assert!(!contents.contains("/secret/place"), "raw value leaked");
        assert!(!contents.contains("zzsecretzz"), "sensitive value leaked");
        assert!(contents.contains("<redacted>"), "redaction marker expected");
    }

    /// USAGE-004: `attach_principal` never panics on a non-object frame
    /// and leaves it unchanged (the daemon then records `anonymous`).
    #[test]
    fn attach_principal_leaves_non_object_unchanged() {
        let mut scalar = serde_json::json!("not-an-object");
        attach_principal(&mut scalar);
        assert_eq!(scalar, serde_json::json!("not-an-object"));

        let mut arr = serde_json::json!([1, 2, 3]);
        attach_principal(&mut arr);
        assert_eq!(arr, serde_json::json!([1, 2, 3]));
    }

    /// USAGE-004: the daemon sink appends `command.invoked` rows to the
    /// configured path (the JSON-RPC dispatch surface's entry point into
    /// the shared usage sidecar), and never routes `gate_evaluated` rows.
    #[test]
    fn daemon_usage_sink_appends_command_invoked_row() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("kindling").join(USAGE_NDJSON);
        let sink = DaemonUsageSink { path: path.clone() };

        let ctx = CommandInvocationContext {
            session_id: "daemon-startup-1",
            timestamp: "2026-06-18T11:00:00Z",
            command: "anvil/gctx/search_symbols",
            principal: "deadbeefcafe",
            traceparent: Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        };
        let obs = from_command_invocation(&ctx, vec![redact_arg("query", Some("Foo"))], Vec::new());
        sink.try_emit_command_invoked(obs)
            .expect("daemon sink appends the row");

        let contents = fs::read_to_string(&path).expect("read usage log");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 1, "one NDJSON row per emit");
        let parsed: CommandInvokedObservation =
            serde_json::from_str(lines[0]).expect("valid NDJSON row");
        assert_eq!(parsed.kind, "command.invoked");
        assert_eq!(parsed.command, "anvil/gctx/search_symbols");
        assert_eq!(parsed.principal, "deadbeefcafe");
    }

    #[test]
    fn argv_shapes_capture_global_flag_before_subcommand() {
        // `anvil --json version`: the global flag precedes the
        // subcommand. The flag must still be captured, and `version`
        // (the subcommand) dropped — not mis-skipped.
        let shapes = arg_shapes_from_argv(
            &["anvil", "--json", "version"]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        );
        assert!(
            shapes.iter().any(|s| s.name == "json"),
            "global --json captured"
        );
        assert!(
            !shapes
                .iter()
                .any(|s| s.name == "version" || s.name == "positional"),
            "subcommand must not surface as an arg: {shapes:?}"
        );
    }

    #[test]
    fn argv_shapes_honour_end_of_options_sentinel() {
        // After `--`, dash-prefixed tokens are positionals, not flags.
        let shapes = arg_shapes_from_argv(
            &["anvil", "check", "--", "--not-a-flag"]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        );
        assert!(
            shapes.iter().all(|s| s.name == "positional"),
            "post-separator tokens must be positionals: {shapes:?}"
        );
        assert!(
            !shapes.iter().any(|s| s.name.is_empty()),
            "no empty-name shape"
        );
    }

    #[test]
    fn argv_shapes_treat_negative_number_as_value() {
        // `--threshold -5`: the negative number is the option's value,
        // not a separate flag.
        let shapes = arg_shapes_from_argv(
            &["anvil", "check", "--threshold", "-5"]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        );
        let threshold = shapes
            .iter()
            .find(|s| s.name == "threshold")
            .expect("threshold");
        assert!(threshold.present, "negative number consumed as value");
        assert!(
            !shapes.iter().any(|s| s.name == "5"),
            "negative number must not become a synthetic flag: {shapes:?}"
        );
    }

    #[test]
    fn salt_race_uses_the_winner_not_a_fresh_salt() {
        // Simulate losing the create race: a salt already exists. A
        // second creator must adopt it, never overwrite it.
        let dir = tempdir().expect("tempdir");
        let winner = load_or_create_salt_in(dir.path()).expect("first salt");
        let loser = load_or_create_salt_in(dir.path()).expect("second salt");
        assert_eq!(
            winner, loser,
            "must adopt the existing salt, not regenerate"
        );
    }

    // --- USAGE-002 flag_set population ---

    fn cap(key: &str, variant: &str, reason: ResolutionReason, gate: bool) -> CapturedResolution {
        CapturedResolution {
            key: key.to_owned(),
            variant: variant.to_owned(),
            reason,
            gate_affecting: gate,
        }
    }

    #[test]
    fn flag_set_maps_sources_and_sorts_by_key() {
        let captured = vec![
            cap("z.rollout", "on", ResolutionReason::TargetingMatch, false),
            cap("a.gate", "enabled", ResolutionReason::Default, true),
            cap("m.over", "x", ResolutionReason::LocalOverride, true),
        ];
        let fs = flag_set_from_captured(captured);
        // Sorted by key.
        assert_eq!(
            fs.iter().map(|e| e.key.as_str()).collect::<Vec<_>>(),
            ["a.gate", "m.over", "z.rollout"]
        );
        let by = |k: &str| fs.iter().find(|e| e.key == k).cloned().unwrap();
        assert_eq!(by("a.gate").source, "default");
        assert!(by("a.gate").gate_affecting);
        assert_eq!(by("m.over").source, "override");
        assert_eq!(by("z.rollout").source, "snapshot");
        assert!(!by("z.rollout").gate_affecting);
    }

    #[test]
    fn flag_set_skips_errored_resolution() {
        let fs = flag_set_from_captured(vec![
            cap("good", "v", ResolutionReason::Default, true),
            cap("bad", "__fail_closed", ResolutionReason::Error, true),
        ]);
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].key, "good");
    }

    #[test]
    fn flag_set_dedups_by_key_last_wins() {
        let fs = flag_set_from_captured(vec![
            cap("dup", "first", ResolutionReason::Default, true),
            cap("dup", "second", ResolutionReason::LocalOverride, true),
        ]);
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].variant, "second");
        assert_eq!(fs[0].source, "override");
    }

    #[test]
    fn flag_set_empty_when_nothing_captured() {
        assert!(flag_set_from_captured(Vec::new()).is_empty());
    }

    #[test]
    fn disabled_reason_maps_to_default_source() {
        let fs = flag_set_from_captured(vec![cap("d", "off", ResolutionReason::Disabled, false)]);
        assert_eq!(fs[0].source, "default");
    }

    // --- DPO-001 / DPO-002: DaemonObservationSink + retention ---

    use anvil_intercept::kindling_observation::{from_fence, from_validate_paths};

    fn save_time_row(paths: &[String]) -> GateEvaluatedObservation {
        // Build a save-time `gate_evaluated` row (gate_id = save-time).
        let ctx = anvil_intercept::kindling_observation::ObservationContext {
            session_id: "00000000-0000-4000-8000-000000000000",
            timestamp: "2026-06-19T10:00:00.000Z",
            gate_eval_id: "gate-eval-1",
            file_path: "",
            duration_ms: 10,
        };
        from_validate_paths(&ctx, &[], paths.len(), paths, false)
    }

    #[test]
    fn daemon_observation_sink_persists_save_time_gate_rows() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("kindling").join(USAGE_NDJSON);
        let sink = DaemonObservationSink { path: path.clone() };

        sink.try_emit(save_time_row(&["src/lib.rs".to_string()]))
            .expect("persist save-time row");

        let contents = fs::read_to_string(&path).expect("read sidecar");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 1, "one save-time row appended");
        let parsed: GateEvaluatedObservation = serde_json::from_str(lines[0]).expect("valid row");
        assert_eq!(parsed.gate_id, SAVE_TIME_GATE_ID);
        assert_eq!(parsed.kind, "gate_evaluated");
    }

    #[test]
    fn daemon_observation_sink_ignores_non_save_time_gate_rows() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("kindling").join(USAGE_NDJSON);
        let sink = DaemonObservationSink { path: path.clone() };

        // A mid-edit gate row must NOT be persisted by this sink.
        let mut row = save_time_row(&["src/lib.rs".to_string()]);
        row.gate_id = "midEdit".to_string();
        sink.try_emit(row).expect("ignored, returns Ok");

        assert!(
            !path.exists() || fs::read_to_string(&path).unwrap().is_empty(),
            "non-save-time gate rows must not be persisted to the usage sidecar",
        );
    }

    #[test]
    fn daemon_observation_sink_persists_constraint_rows() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("kindling").join(USAGE_NDJSON);
        let sink = DaemonObservationSink { path: path.clone() };

        let row = from_fence(
            "00000000-0000-4000-8000-000000000000",
            "2026-06-19T10:00:00.000Z",
            "/work/tree",
            "operator",
            false,
            true,
        );
        sink.try_emit_constraint_applied(row)
            .expect("persist constraint row");

        let contents = fs::read_to_string(&path).expect("read sidecar");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 1, "one constraint row appended");
        let parsed: ConstraintAppliedObservation =
            serde_json::from_str(lines[0]).expect("valid row");
        assert_eq!(parsed.kind, "constraint_applied");
    }

    /// Retention: a file over the byte cap is trimmed to under it; the
    /// oldest lines are dropped and recent lines survive.
    #[test]
    fn trim_drops_oldest_lines_when_over_byte_cap() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("usage.ndjson");

        // Recent timestamp so the age cut-off does not fire — isolate the
        // byte-cap behaviour. Each line is padded to a known size.
        let now = Utc::now();
        let recent = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let pad = "x".repeat(1024);
        let mut buf = String::new();
        // Write ~80 MiB so we're comfortably over the 64 MiB cap.
        let line_count = (USAGE_SIDECAR_MAX_BYTES / 1024) + 16_000;
        for i in 0..line_count {
            writeln!(
                buf,
                "{{\"timestamp\":\"{recent}\",\"i\":{i},\"pad\":\"{pad}\"}}"
            )
            .expect("write line");
        }
        fs::write(&path, &buf).expect("seed oversized sidecar");
        assert!(
            fs::metadata(&path).unwrap().len() > USAGE_SIDECAR_MAX_BYTES,
            "precondition: file starts over the byte cap"
        );

        trim_usage_sidecar_at(&path, now);

        let after = fs::metadata(&path).unwrap().len();
        assert!(
            after <= USAGE_SIDECAR_MAX_BYTES,
            "trim must bring the file under the byte cap; got {after}"
        );
        // The newest line (highest `i`) must survive.
        let contents = fs::read_to_string(&path).expect("read trimmed");
        let last: serde_json::Value =
            serde_json::from_str(contents.lines().last().expect("a surviving line"))
                .expect("valid json");
        assert_eq!(
            last["i"].as_u64(),
            Some(line_count - 1),
            "the most recent row must survive the byte-cap trim"
        );
    }

    /// Retention: lines older than the max age are dropped while recent
    /// lines survive; a malformed line is kept (never crashed on).
    #[test]
    fn trim_drops_lines_older_than_max_age_and_keeps_malformed() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("usage.ndjson");

        let now = Utc::now();
        let old =
            (now - chrono::Duration::days(30)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let recent =
            (now - chrono::Duration::hours(1)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let mut buf = String::new();
        writeln!(buf, "{{\"timestamp\":\"{old}\",\"tag\":\"old1\"}}").expect("w");
        writeln!(buf, "{{\"timestamp\":\"{old}\",\"tag\":\"old2\"}}").expect("w");
        writeln!(buf, "{{\"timestamp\":\"{recent}\",\"tag\":\"recent1\"}}").expect("w");
        writeln!(buf, "{{\"timestamp\":\"{recent}\",\"tag\":\"recent2\"}}").expect("w");
        fs::write(&path, &buf).expect("seed aged sidecar");

        trim_usage_sidecar_at(&path, now);

        let contents = fs::read_to_string(&path).expect("read trimmed");
        assert!(
            !contents.contains("old1"),
            "stale leading row must be dropped"
        );
        assert!(
            !contents.contains("old2"),
            "stale leading row must be dropped"
        );
        assert!(contents.contains("recent1"), "recent row must survive");
        assert!(contents.contains("recent2"), "recent row must survive");
    }

    #[test]
    fn trim_keeps_malformed_leading_line() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("usage.ndjson");
        let now = Utc::now();
        let recent = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // A malformed leading line (not JSON) followed by a recent row.
        // The malformed line parses as "not older" → KEPT, and ends the
        // leading-stale scan, so nothing is dropped.
        let mut buf = String::new();
        buf.push_str("this is not json\n");
        writeln!(buf, "{{\"timestamp\":\"{recent}\",\"tag\":\"recent\"}}").expect("w");
        fs::write(&path, &buf).expect("seed");

        trim_usage_sidecar_at(&path, now);

        let contents = fs::read_to_string(&path).expect("read");
        assert!(
            contents.contains("this is not json"),
            "a malformed line must be kept, not crashed on or dropped"
        );
        assert!(contents.contains("recent"));
    }

    #[test]
    fn trim_is_noop_on_missing_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("does-not-exist.ndjson");
        // Must not panic or create the file.
        trim_usage_sidecar_at(&path, Utc::now());
        assert!(!path.exists());
    }

    /// Council B fast-path: a small file whose head line is fresh and which
    /// is well under the byte cap is NOT rewritten — the trim returns before
    /// the full read+rewrite. Proven by asserting the file's mtime is
    /// unchanged across the call (a rewrite would replace the file via
    /// rename, changing the mtime), and that no `.trim.tmp` sibling is left.
    #[test]
    fn trim_does_not_rewrite_small_fresh_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("usage.ndjson");
        let now = Utc::now();
        let recent = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let mut buf = String::new();
        writeln!(buf, "{{\"timestamp\":\"{recent}\",\"tag\":\"a\"}}").expect("w");
        writeln!(buf, "{{\"timestamp\":\"{recent}\",\"tag\":\"b\"}}").expect("w");
        fs::write(&path, &buf).expect("seed");

        let before = fs::metadata(&path)
            .expect("stat")
            .modified()
            .expect("mtime");
        let before_contents = fs::read_to_string(&path).expect("read");

        // Sleep a touch so a rewrite would produce a distinguishable mtime.
        std::thread::sleep(std::time::Duration::from_millis(20));
        trim_usage_sidecar_at(&path, now);

        let after = fs::metadata(&path)
            .expect("stat")
            .modified()
            .expect("mtime");
        let after_contents = fs::read_to_string(&path).expect("read");
        assert_eq!(
            before, after,
            "a small fresh file must not be rewritten (mtime changed)",
        );
        assert_eq!(
            before_contents, after_contents,
            "a small fresh file's content must be untouched",
        );
        assert!(
            !path.with_extension("ndjson.trim.tmp").exists(),
            "no trim temp sibling should be created on the fast path",
        );
    }

    #[test]
    fn emergency_override_maps_to_override_source() {
        let fs = flag_set_from_captured(vec![cap(
            "e",
            "on",
            ResolutionReason::EmergencyOverride,
            true,
        )]);
        assert_eq!(fs[0].source, "override");
    }
}
