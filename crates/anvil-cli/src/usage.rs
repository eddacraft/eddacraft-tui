//! USAGE-001: CLI `command.invoked` usage observations.
//!
//! Best-effort Kindling rows: command name, salted principal (or `anonymous`),
//! redacted arg shapes — never raw values. Written under credentials/Kindling
//! paths. Wired once from `main` for every command; failures never affect exit
//! code. Daemon path: USAGE-004 allowlisted methods via [`daemon_usage_emitter`].

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anvil_intercept::kindling_observation::{
    CommandInvocationContext, CommandInvokedEmitter, CommandInvokedObservation,
    ConstraintAppliedObservation, DEFAULT_SAVE_TIME_PASS_CAPACITY, DEFAULT_SAVE_TIME_PASS_WINDOW,
    FalsePositiveReportContext, FalsePositiveReportedObservation, FlagSetEntry,
    GateEvaluatedObservation, KindlingObservationSink, KindlingSinkError,
    NonBlockingObservationSink, SAVE_TIME_GATE_ID, SaveTimeObservationEmitter,
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

/// Append one usage observation as a single NDJSON line to `path`, creating the
/// parent directory if needed.
///
/// The sidecar holds per-invocation principals and argument metadata, so it is
/// created owner-only (`0600`) under an owner-only parent (`0700`) on Unix —
/// matching the salt's posture so a shared host can't read the usage history.
/// On Unix the parent is validated as a non-symlinked, current-user-owned
/// directory and the sidecar leaf is opened relative to that directory fd with
/// `O_NOFOLLOW`, closing the final-component check/open race.
// KDS-003: `pub(crate)` so the daemon-sink parity test can write a row through
// the real NDJSON path and compare it against the daemon-stored row.
pub(crate) fn append_usage_observation_to(
    path: &Path,
    obs: &CommandInvokedObservation,
) -> Result<()> {
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

/// Append one observation as a single NDJSON line to `path`, creating the parent
/// directory if needed, generic over any serialisable row so the three kinds
/// (`command.invoked`, `gate_evaluated(save-time)`, `constraint_applied`) share
/// one write path.
///
/// The sidecar holds per-invocation principals and argument metadata, so it is
/// created owner-only (`0600`) under an owner-only parent (`0700`) on Unix —
/// matching the salt's posture so a shared host can't read the usage history.
/// On Unix the append open is parent-fd anchored and leaf-`O_NOFOLLOW`; the
/// trim read path uses the same no-follow discipline. Before the append the
/// sidecar is lazily trimmed (see [`trim_usage_sidecar`]) to the 7-day / 64 MiB
/// retention bounds (council T5).
fn append_observation_to<T: serde::Serialize>(path: &Path, obs: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_private_sidecar_parent(parent)
            .with_context(|| format!("prepare kindling dir {}", parent.display()))?;
    }
    // Retention (council T5): trim before the append so the sidecar stays
    // bounded. Best-effort — a trim failure must not block the write.
    trim_usage_sidecar(path);
    let serialised = serde_json::to_string(obs).context("serialise usage observation")?;
    let mut f = open_sidecar_append(path)
        .with_context(|| format!("open usage sidecar {}", path.display()))?;
    // `OpenOptions::mode` only applies when the file is *created*. Enforce
    // `0600` on an already-existing sidecar too (best-effort), so a file
    // left world-readable by a previous run/version is tightened.
    tighten_file_mode(&f);
    writeln!(f, "{serialised}")
        .with_context(|| format!("append usage row to {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn sidecar_parent(path: &Path) -> io::Result<&Path> {
    path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage sidecar {} has no parent directory", path.display()),
        )
    })
}

fn sidecar_leaf(path: &Path) -> io::Result<&OsStr> {
    path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage sidecar {} has no file name", path.display()),
        )
    })
}

fn ensure_private_sidecar_parent(parent: &Path) -> io::Result<()> {
    create_private_dir(parent)?;
    #[cfg(unix)]
    ensure_private_sidecar_parent_unix(parent)?;
    Ok(())
}

#[cfg(unix)]
fn ensure_private_sidecar_parent_unix(parent: &Path) -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    let meta = fs::symlink_metadata(parent)?;
    if meta.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage sidecar parent {} is a symlink; refusing to write",
                parent.display()
            ),
        ));
    }
    if !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage sidecar parent {} is not a directory",
                parent.display()
            ),
        ));
    }
    if meta.uid() != nix::unistd::geteuid().as_raw() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "usage sidecar parent {} is not owned by the current user",
                parent.display()
            ),
        ));
    }
    if meta.permissions().mode() & 0o077 != 0 {
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(unix)]
fn open_sidecar_parent_dirfd(parent: &Path) -> io::Result<std::os::fd::OwnedFd> {
    use nix::fcntl::OFlag;
    use nix::sys::stat::{Mode, fstat};

    ensure_private_sidecar_parent(parent)?;
    let flags = OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let fd = nix::fcntl::open(parent, flags, Mode::empty()).map_err(io::Error::from)?;
    let st = fstat(&fd).map_err(io::Error::from)?;
    if st.st_uid != nix::unistd::geteuid().as_raw() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "usage sidecar parent fd is not owned by the current user",
        ));
    }
    if st.st_mode & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "usage sidecar parent fd is group/other-accessible",
        ));
    }
    Ok(fd)
}

#[cfg(unix)]
fn open_sidecar_append(path: &Path) -> io::Result<File> {
    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::{Mode, fchmod};
    use std::os::fd::AsFd as _;

    let parent = sidecar_parent(path)?;
    let leaf = sidecar_leaf(path)?;
    let dirfd = open_sidecar_parent_dirfd(parent)?;
    let flags =
        OFlag::O_CREAT | OFlag::O_APPEND | OFlag::O_WRONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let fd = openat(
        dirfd.as_fd(),
        Path::new(leaf),
        flags,
        Mode::from_bits_truncate(0o600),
    )
    .map_err(io::Error::from)?;
    fchmod(&fd, Mode::from_bits_truncate(0o600)).map_err(io::Error::from)?;
    Ok(File::from(fd))
}

#[cfg(not(unix))]
fn open_sidecar_append(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        create_private_dir(parent)?;
    }
    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    opts.open(path)
}

#[cfg(unix)]
fn open_existing_sidecar_read(path: &Path) -> io::Result<Option<File>> {
    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::{Mode, SFlag, fstat};
    use std::os::fd::AsFd as _;

    let parent = sidecar_parent(path)?;
    if !parent.exists() {
        return Ok(None);
    }
    let leaf = sidecar_leaf(path)?;
    let dirfd = open_sidecar_parent_dirfd(parent)?;
    let flags = OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let fd = match openat(dirfd.as_fd(), Path::new(leaf), flags, Mode::empty()) {
        Ok(fd) => fd,
        Err(nix::errno::Errno::ENOENT) => return Ok(None),
        Err(err) => return Err(io::Error::from(err)),
    };
    let st = fstat(&fd).map_err(io::Error::from)?;
    let kind = SFlag::from_bits_truncate(st.st_mode);
    if !kind.contains(SFlag::S_IFREG) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage sidecar {} is not a regular file", path.display()),
        ));
    }
    Ok(Some(File::from(fd)))
}

#[cfg(not(unix))]
fn open_existing_sidecar_read(path: &Path) -> io::Result<Option<File>> {
    match File::open(path) {
        Ok(file) => Ok(Some(file)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

#[cfg_attr(not(unix), allow(unused_variables))]
fn tighten_file_mode(file: &File) {
    #[cfg(unix)]
    {
        use nix::sys::stat::{Mode, fchmod};
        let _ = fchmod(file, Mode::from_bits_truncate(0o600));
    }
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
    let Ok(Some(file)) = open_existing_sidecar_read(path) else {
        return; // No file yet — nothing to trim.
    };
    let Ok(meta) = file.metadata() else {
        return;
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
    if !needs_size_trim && !first_line_is_stale(file, cutoff) {
        return;
    }

    // Read line-by-line through a `BufReader`, skipping non-UTF-8
    // (`InvalidData`) lines rather than aborting the whole trim on the first
    // torn byte. A single corrupt line (e.g. a write that split a multi-byte
    // codepoint) previously made `fs::read_to_string` return `Err`, which
    // bailed silently here — so the file could never be trimmed again and
    // grew past the 64 MiB cap. This mirrors `usage_views::load_rows`, which
    // already skips `InvalidData` lines on the read path.
    let lines: Vec<String> = {
        use std::io::BufRead as _;
        let Ok(Some(file)) = open_existing_sidecar_read(path) else {
            return; // Unreadable — leave it untouched.
        };
        let mut collected = Vec::new();
        for line in io::BufReader::new(file).lines() {
            match line {
                Ok(line) => collected.push(line),
                // A non-UTF-8 line is corrupt data for that line only — skip
                // it (it is dropped from the rewritten file) but keep
                // trimming the rest.
                Err(err) if err.kind() == io::ErrorKind::InvalidData => {}
                // A genuine I/O failure mid-read bails (best-effort
                // housekeeping never escalates).
                Err(_) => return,
            }
        }
        collected
    };
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();

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
    // Best-effort atomic-ish replace: write a unique temp sibling then rename.
    // A failure leaves the original intact (retention is housekeeping).
    let _ = rewrite_sidecar_via_unique_temp(path, rewritten.as_bytes());
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
fn first_line_is_stale(file: File, cutoff: chrono::DateTime<Utc>) -> bool {
    use std::io::BufRead as _;

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

/// Write `bytes` to a unique owner-only temp file next to `path`, then rename it
/// over `path`. Used for retention housekeeping only; callers keep it
/// best-effort.
fn rewrite_sidecar_via_unique_temp(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        rewrite_sidecar_via_unique_temp_unix(path, bytes)
    }
    #[cfg(not(unix))]
    {
        rewrite_sidecar_via_unique_temp_fallback(path, bytes)
    }
}

#[cfg(unix)]
fn rewrite_sidecar_via_unique_temp_unix(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use nix::fcntl::{OFlag, openat, renameat};
    use nix::sys::stat::{Mode, fchmod};
    use nix::unistd::{UnlinkatFlags, unlinkat};
    use std::os::fd::AsFd as _;

    let parent = sidecar_parent(path)?;
    let final_leaf = sidecar_leaf(path)?;
    let dirfd = open_sidecar_parent_dirfd(parent)?;
    let temp = unique_trim_temp_name(final_leaf);
    let flags =
        OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_WRONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let fd = openat(
        dirfd.as_fd(),
        Path::new(&temp),
        flags,
        Mode::from_bits_truncate(0o600),
    )
    .map_err(io::Error::from)?;
    fchmod(&fd, Mode::from_bits_truncate(0o600)).map_err(io::Error::from)?;
    let mut file = File::from(fd);
    if let Err(err) = file.write_all(bytes) {
        let _ = unlinkat(dirfd.as_fd(), Path::new(&temp), UnlinkatFlags::NoRemoveDir);
        return Err(err);
    }
    drop(file);
    if let Err(err) = renameat(
        dirfd.as_fd(),
        Path::new(&temp),
        dirfd.as_fd(),
        Path::new(final_leaf),
    ) {
        let _ = unlinkat(dirfd.as_fd(), Path::new(&temp), UnlinkatFlags::NoRemoveDir);
        return Err(io::Error::from(err));
    }
    Ok(())
}

#[cfg(not(unix))]
fn rewrite_sidecar_via_unique_temp_fallback(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = path.with_file_name(unique_trim_temp_name(sidecar_leaf(path)?));
    let mut opts = fs::OpenOptions::new();
    opts.create_new(true).write(true);
    let mut f = opts.open(&temp)?;
    if let Err(err) = f.write_all(bytes) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    drop(f);
    if let Err(err) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

fn unique_trim_temp_name(final_leaf: &OsStr) -> String {
    format!(
        ".{}.{}.trim.tmp",
        final_leaf.to_string_lossy(),
        Uuid::new_v4().as_simple()
    )
}

/// Create a directory (and parents) owner-only (`0700`) on Unix.
// KDS-001: `pub(crate)` so the daemon sink creates its spool's `kindling/`
// parent dir with the same owner-only (`0700`) posture before first write.
pub(crate) fn create_private_dir(dir: &Path) -> io::Result<()> {
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

/// Whether the operator has opted out of CLI usage collection.
///
/// 094a: the CLI `command.invoked` producer ([`record_invocation`]) is the
/// one usage producer with no kill-switch — the daemon DPO producers honour
/// `ANVIL_INTERCEPT_DISABLE_OBSERVATION`, but that never reached the CLI
/// path. This consults, in order, the dedicated CLI opt-out
/// (`ANVIL_USAGE_DISABLE`), the cross-cutting whole-observation break-glass
/// (`ANVIL_INTERCEPT_DISABLE_OBSERVATION`, so a single toggle silences both
/// the daemon and the CLI), and the cross-tool `DO_NOT_TRACK` consent
/// convention. Any of them set to `1` (the explicit opt-out value the
/// daemon kill-switch already uses) declines collection. Read fresh each
/// call so an operator can flip it without a code change.
#[must_use]
pub fn usage_collection_disabled() -> bool {
    const OPT_OUT_VARS: [&str; 3] = [
        "ANVIL_USAGE_DISABLE",
        "ANVIL_INTERCEPT_DISABLE_OBSERVATION",
        "DO_NOT_TRACK",
    ];
    OPT_OUT_VARS
        .iter()
        .any(|var| env::var_os(var).is_some_and(|v| v == "1"))
}

pub fn record_invocation(command_name: &str) -> Result<()> {
    // 094a operator kill-switch: decline CLI usage collection entirely when
    // opted out. Returns `Ok(())` (not an error) so the best-effort caller
    // path is unaffected — no row is written and nothing is logged as a
    // failure.
    if usage_collection_disabled() {
        return Ok(());
    }
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
        // CIB-197: stamp the producing binary's identity so every row
        // self-describes once any export or fleet surface exists. The
        // version is THIS binary's compile-time crate version; the
        // install method reuses the LAUNCH-013 detection behind
        // `anvil version` (process-cached — it now runs per command).
        version: env!("CARGO_PKG_VERSION"),
        install_method: crate::commands::version::detect_install_method_cached().label(),
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FalsePositiveReportSummary {
    pub check_id: String,
    pub hashed_path: String,
    pub line: u32,
    pub timestamp: String,
}

pub fn list_false_positive_reports() -> Result<Vec<FalsePositiveReportSummary>> {
    let state_dir = credentials::credentials_dir().context("resolve usage state directory")?;
    list_false_positive_reports_in(&state_dir)
}

fn list_false_positive_reports_in(state_dir: &Path) -> Result<Vec<FalsePositiveReportSummary>> {
    let path = fp_log_path(state_dir);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(err).with_context(|| format!("read {}", path.display()));
        }
    };

    let mut reports = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let row: FalsePositiveReportedObservation = match serde_json::from_str(line) {
            Ok(row) => row,
            Err(err) => {
                tracing::warn!(%err, "skipping corrupt false-positive sidecar row");
                continue;
            }
        };
        reports.push(FalsePositiveReportSummary {
            check_id: row.check_id,
            hashed_path: row.hashed_path,
            line: row.line,
            timestamp: row.timestamp,
        });
    }
    Ok(reports)
}

// KDS-005: the bespoke `DaemonUsageSink` NDJSON writer for the daemon
// `command.invoked` producer is retired — the producer now always routes through
// `KindlingDaemonSink` (the daemon, with a capped spool fallback). The CLI
// producer (`record_invocation`) still writes the sidecar, and the DPO
// `DaemonObservationSink` below still persists `gate_evaluated` / `constraint_applied`
// there, so `append_usage_observation_to` / the trim helpers remain in use.

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
/// are NOT consumed here (the daemon `command.invoked` producer routes them
/// through `KindlingDaemonSink` since KDS-005) so a sink shared between both
/// producers does not double-write usage rows.
///
/// A write failure is surfaced as [`KindlingSinkError::Unavailable`];
/// behind the [`NonBlockingObservationSink`] decorator it is logged on
/// the drain thread and dropped, never coupling the verdict / engage hot
/// path to sink health.
// DPO save-time / fence observations still use the NDJSON sidecar. KDS-005
// retired only the `command.invoked` `DaemonUsageSink`; routing these
// `gate_evaluated` / `constraint_applied` kinds to the daemon is a DPO-track
// follow-up (it needs the DPO read surface), so this writer stays for now.
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

/// KDS-002: env var selecting the daemon `command.invoked` sink backend.
const KINDLING_SINK_ENV: &str = "ANVIL_KINDLING_SINK";

/// KDS-002/-005: operator-selected backend for the daemon `command.invoked`
/// producer, resolved from [`KINDLING_SINK_ENV`].
///
/// `pub(crate)` for KDS-004: the `anvil kindling usage` views consult it to
/// decide whether to read the daemon (the authoritative store).
///
/// KDS-005 retired the `ndjson` sink, so the daemon is now the **default** — the
/// only backends are the daemon and `off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KindlingSinkSelection {
    /// Append to the Kindling daemon via `KindlingDaemonSink` (capped spool
    /// fallback). The default.
    Daemon,
    /// Disable the daemon `command.invoked` producer entirely (no rows).
    Off,
}

/// Parse a raw [`KINDLING_SINK_ENV`] value (case-insensitive, trimmed) to a
/// [`KindlingSinkSelection`].
///
/// KDS-005: the default (unset / empty) is now `Daemon`. `off` disables the
/// producer. The retired `ndjson` value, and any unrecognised value, resolve to
/// `Daemon` (with a warn) — so a typo never silently disables capture, and an
/// operator still pinning `ndjson` keeps capturing (via the daemon now).
fn parse_kindling_sink(value: Option<&str>) -> KindlingSinkSelection {
    let Some(raw) = value else {
        return KindlingSinkSelection::Daemon;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "daemon" => KindlingSinkSelection::Daemon,
        "off" => KindlingSinkSelection::Off,
        "ndjson" => {
            tracing::warn!(
                target: "anvil::usage",
                "ANVIL_KINDLING_SINK=ndjson is retired (KDS-005) — the bespoke NDJSON sink \
                 is gone; using the daemon sink. Set `off` to disable capture.",
            );
            KindlingSinkSelection::Daemon
        }
        // Don't echo the raw value — an operator could mis-assign a secret to
        // this var; the message alone is enough to debug a typo.
        _ => {
            tracing::warn!(
                target: "anvil::usage",
                "unrecognised ANVIL_KINDLING_SINK value; using the daemon sink \
                 (expected daemon|off)",
            );
            KindlingSinkSelection::Daemon
        }
    }
}

/// Resolve [`KINDLING_SINK_ENV`] to a [`KindlingSinkSelection`]. Read fresh so an
/// operator can flip it per daemon start. `pub(crate)` for the KDS-004
/// source-aware usage-view guard.
pub(crate) fn resolve_kindling_sink() -> KindlingSinkSelection {
    parse_kindling_sink(env::var(KINDLING_SINK_ENV).ok().as_deref())
}

/// USAGE-004 / KDS-005: build the daemon-side command-invocation usage emitter,
/// wired to the daemon-backed `KindlingDaemonSink` (the only backend now; the
/// `ndjson` sink was retired and the daemon is the default — `off` disables it).
///
/// Returns `None` — no usage rows — when the sink is `off`, when the spool path
/// can't be resolved, or when the sink can't be built (no NDJSON fallback any
/// more). The per-startup `daemon_session_id` stamps every daemon row;
/// individual calls are correlated by `traceparent`, not this id.
#[must_use]
pub fn daemon_usage_emitter() -> Option<Arc<CommandInvokedEmitter>> {
    // Whole-observation break-glass (parity with `daemon_observation_producers`
    // and the CLI `usage_collection_disabled`): one toggle silences EVERY usage
    // producer, as the operator-controls runbook documents. Previously this
    // daemon `command.invoked` producer ignored it — a consent gap vs the docs.
    if env::var_os("ANVIL_INTERCEPT_DISABLE_OBSERVATION").is_some_and(|v| v == "1") {
        tracing::warn!(
            target: "anvil::usage",
            "ANVIL_INTERCEPT_DISABLE_OBSERVATION=1 — daemon command.invoked usage \
             producer disabled (break-glass)",
        );
        return None;
    }
    // KDS-002/-005: `off` disables the daemon command.invoked producer; every
    // other value (default included, post-KDS-005) routes through the daemon.
    if matches!(resolve_kindling_sink(), KindlingSinkSelection::Off) {
        tracing::info!(
            target: "anvil::usage",
            "ANVIL_KINDLING_SINK=off — daemon command.invoked usage producer disabled",
        );
        return None;
    }

    // KDS-005: the daemon command.invoked producer always routes through the
    // daemon-backed sink now (the bespoke NDJSON `DaemonUsageSink` is retired).
    // `repo_id` is left to the client default (its `project_root` / CWD) — the
    // daemon serves the project it was started in; authoritative per-call
    // scoping is a follow-up (mirrors the KDS-004 read-side note).
    let spool = crate::kindling_daemon_sink::default_spool_path()
        .map_err(|err| {
            tracing::warn!(
                target: "anvil::usage",
                error = %err,
                "usage export disabled: could not resolve the kindling spool path",
            );
        })
        .ok()?;
    let inner: Arc<dyn KindlingObservationSink> =
        match crate::kindling_daemon_sink::KindlingDaemonSink::new(None, spool) {
            Ok(sink) => Arc::new(sink) as Arc<dyn KindlingObservationSink>,
            Err(err) => {
                // No NDJSON fallback any more (retired) — degrade to no export
                // rather than silently writing to a retired sidecar path.
                tracing::warn!(
                    target: "anvil::usage",
                    error = %err,
                    "usage export disabled: the daemon command.invoked sink could not be built",
                );
                return None;
            }
        };
    // N2 (cross-ref): the daemon runs on a `new_current_thread` tokio runtime,
    // and the sink's `try_emit_command_invoked` `block_on`s the (capped) spool
    // append synchronously. Wired RAW, that would run on the single event-loop
    // thread inside async `handle_connection`, stalling the whole loop. Wrap the
    // sink in the SAME `NonBlockingObservationSink` decorator the save-time /
    // fence producers use: one drain thread owns the blocking work, the dispatch
    // path only `try_send`s an envelope and returns. Semantics preserved:
    // at-most-once (a full/disconnected channel drops + counts the row, never
    // duplicates it) and FIFO ordering per the single bounded `sync_channel`.
    let Some(non_blocking) =
        NonBlockingObservationSink::new(inner, DEFAULT_OBSERVATION_CHANNEL_CAPACITY)
    else {
        tracing::warn!(
            target: "anvil::usage",
            "usage export disabled: could not start the non-blocking usage \
             drain thread",
        );
        return None;
    };
    let sink = Arc::new(non_blocking) as Arc<dyn KindlingObservationSink>;
    Some(Arc::new(CommandInvokedEmitter::new(
        sink,
        Uuid::new_v4().to_string(),
        // CIB-197: the daemon runs inside this same `anvil` binary, so
        // its rows carry this binary's version + install method too.
        env!("CARGO_PKG_VERSION").to_string(),
        crate::commands::version::detect_install_method_cached()
            .label()
            .to_string(),
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
    fn list_false_positive_reports_returns_empty_when_absent() {
        let dir = tempdir().expect("tempdir");
        let reports = list_false_positive_reports_in(dir.path()).expect("list");
        assert!(reports.is_empty(), "expected no reports, got {reports:?}");
    }

    #[test]
    fn list_false_positive_reports_skips_corrupt_rows() {
        let dir = tempdir().expect("tempdir");
        record_false_positive_in(dir.path(), "ANV-CORE-001", "src/a.rs", 1, None)
            .expect("record good");
        let path = fp_log_path(dir.path());
        let mut content = std::fs::read_to_string(&path).expect("read");
        content.push_str("{not-json\n");
        std::fs::write(&path, content).expect("append corrupt");

        let reports = list_false_positive_reports_in(dir.path()).expect("list");
        assert_eq!(
            reports.len(),
            1,
            "valid row should survive corrupt tail: {reports:?}"
        );
        assert_eq!(reports[0].check_id, "ANV-CORE-001");
    }

    #[test]
    fn list_false_positive_reports_lists_recorded_reports_without_source_or_plaintext_path() {
        let dir = tempdir().expect("tempdir");
        let plaintext_path = "src/secret.rs";
        record_false_positive_in(
            dir.path(),
            "ANV-CORE-002",
            plaintext_path,
            7,
            Some("let secret = \"sk-test\";".to_string()),
        )
        .expect("record fp");

        let reports = list_false_positive_reports_in(dir.path()).expect("list");
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        assert_eq!(report.check_id, "ANV-CORE-002");
        assert_eq!(report.line, 7);
        assert!(
            !report.hashed_path.contains(plaintext_path),
            "plaintext path leaked through listed hash: {report:?}"
        );

        let json = serde_json::to_string(report).expect("serialise summary");
        assert!(
            !json.contains(plaintext_path),
            "plaintext path leaked in list JSON: {json}"
        );
        assert!(
            !json.contains("sk-test"),
            "opt-in source snippet must not be returned by list: {json}"
        );
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
            version: "0.9.0-beta",
            install_method: "dev_build",
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
            // CIB-197: every written row self-describes its producer.
            assert_eq!(parsed.version, "0.9.0-beta");
            assert_eq!(parsed.install_method, "dev_build");
        }
        assert!(!contents.contains("user@example.com"), "raw email leaked");
        assert!(!contents.contains("/secret/place"), "raw value leaked");
        assert!(!contents.contains("zzsecretzz"), "sensitive value leaked");
        assert!(contents.contains("<redacted>"), "redaction marker expected");
    }

    #[cfg(unix)]
    fn test_command_observation() -> CommandInvokedObservation {
        let ctx = CommandInvocationContext {
            session_id: "33333333-3333-4333-8333-333333333333",
            timestamp: "2099-06-14T10:00:00Z",
            command: "check",
            principal: ANONYMOUS_PRINCIPAL,
            traceparent: None,
            version: "0.9.0-beta",
            install_method: "dev_build",
        };
        from_command_invocation(&ctx, Vec::new(), Vec::new())
    }

    #[cfg(unix)]
    #[test]
    fn append_usage_refuses_symlinked_sidecar_at_open_time() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().expect("tempdir");
        let path = usage_log_path(dir.path());
        let parent = path.parent().expect("parent");
        create_private_dir(parent).expect("kindling dir");
        let outside = dir.path().join("outside.ndjson");
        fs::write(&outside, "outside\n").expect("outside");
        symlink(&outside, &path).expect("sidecar symlink");

        let err = append_usage_observation_to(&path, &test_command_observation())
            .expect_err("symlinked sidecar must be refused");
        assert!(
            format!("{err:#}").contains("symlink") || format!("{err:#}").contains("Too many"),
            "error should identify symlink refusal: {err:#}"
        );
        assert_eq!(
            fs::read_to_string(&outside).expect("outside unchanged"),
            "outside\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn record_false_positive_refuses_symlinked_sidecar_at_open_time() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().expect("tempdir");
        let path = fp_log_path(dir.path());
        let parent = path.parent().expect("parent");
        create_private_dir(parent).expect("kindling dir");
        let outside = dir.path().join("outside-fp.ndjson");
        fs::write(&outside, "outside\n").expect("outside");
        symlink(&outside, &path).expect("fp sidecar symlink");

        let err = record_false_positive_in(dir.path(), "ANV-CORE-001", "src/x.rs", 1, None)
            .expect_err("symlinked false-positive sidecar must be refused");
        assert!(
            format!("{err:#}").contains("symlink") || format!("{err:#}").contains("Too many"),
            "error should identify symlink refusal: {err:#}"
        );
        assert_eq!(
            fs::read_to_string(&outside).expect("outside unchanged"),
            "outside\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn append_usage_refuses_symlinked_kindling_parent() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().expect("tempdir");
        let outside = tempdir().expect("outside");
        let parent = dir.path().join("kindling");
        symlink(outside.path(), &parent).expect("kindling parent symlink");
        let path = parent.join(USAGE_NDJSON);

        let err = append_usage_observation_to(&path, &test_command_observation())
            .expect_err("symlinked kindling parent must be refused");
        assert!(
            format!("{err:#}").contains("symlink"),
            "error should identify symlink parent: {err:#}"
        );
        assert!(
            !outside.path().join(USAGE_NDJSON).exists(),
            "must not write through symlinked parent"
        );
    }

    #[cfg(unix)]
    #[test]
    fn append_usage_tightens_existing_kindling_parent() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let path = usage_log_path(dir.path());
        let parent = path.parent().expect("parent");
        fs::create_dir_all(parent).expect("mkdir");
        fs::set_permissions(parent, fs::Permissions::from_mode(0o777)).expect("loosen parent");

        append_usage_observation_to(&path, &test_command_observation()).expect("append");

        let mode = fs::metadata(parent)
            .expect("stat parent")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "parent should be owner-only");
    }

    #[cfg(unix)]
    #[test]
    fn trim_read_does_not_follow_symlinked_sidecar() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().expect("tempdir");
        let path = usage_log_path(dir.path());
        let parent = path.parent().expect("parent");
        create_private_dir(parent).expect("kindling dir");
        let outside = dir.path().join("outside-usage.ndjson");
        fs::write(
            &outside,
            "{\"timestamp\":\"2000-01-01T00:00:00Z\",\"kind\":\"command.invoked\"}\n",
        )
        .expect("outside");
        symlink(&outside, &path).expect("sidecar symlink");

        append_usage_observation_to(&path, &test_command_observation())
            .expect_err("symlinked sidecar must be refused");

        assert_eq!(
            fs::read_to_string(&outside).expect("outside unchanged"),
            "{\"timestamp\":\"2000-01-01T00:00:00Z\",\"kind\":\"command.invoked\"}\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn trim_does_not_reuse_preexisting_regular_temp_file() {
        let dir = tempdir().expect("tempdir");
        let path = usage_log_path(dir.path());
        create_private_dir(path.parent().expect("parent")).expect("kindling dir");
        fs::write(
            &path,
            "{\"timestamp\":\"2000-01-01T00:00:00Z\",\"kind\":\"command.invoked\"}\n",
        )
        .expect("sidecar");
        let old_fixed_tmp = path.with_extension("ndjson.trim.tmp");
        fs::write(&old_fixed_tmp, "sentinel\n").expect("preexisting temp");

        trim_usage_sidecar_at(&path, Utc::now());

        assert_eq!(
            fs::read_to_string(&old_fixed_tmp).expect("old temp unchanged"),
            "sentinel\n",
            "trim must not truncate or reuse the old deterministic temp path"
        );
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

    // KDS-005: the `DaemonUsageSink` NDJSON-writer tests are removed with the
    // sink itself. The daemon `command.invoked` producer now routes through
    // `KindlingDaemonSink` (covered by the `kindling_daemon_sink` parity / spool
    // tests); its non-blocking wrap matches the shared `DaemonObservationSink`
    // decorator path.

    // ── KDS-002 / KDS-005: ANVIL_KINDLING_SINK selection ────────────────

    #[test]
    fn parse_kindling_sink_defaults_to_daemon() {
        // KDS-005: unset / empty / whitespace and the retired `ndjson` value all
        // resolve to the daemon (the new default).
        for v in [None, Some(""), Some("   "), Some("ndjson"), Some("NDJSON")] {
            assert_eq!(
                parse_kindling_sink(v),
                KindlingSinkSelection::Daemon,
                "{v:?}"
            );
        }
    }

    #[test]
    fn parse_kindling_sink_recognises_daemon_and_off_case_insensitively() {
        for v in ["daemon", "DAEMON", "  Daemon  "] {
            assert_eq!(
                parse_kindling_sink(Some(v)),
                KindlingSinkSelection::Daemon,
                "{v:?}"
            );
        }
        for v in ["off", "OFF", "  Off "] {
            assert_eq!(
                parse_kindling_sink(Some(v)),
                KindlingSinkSelection::Off,
                "{v:?}"
            );
        }
    }

    #[test]
    fn parse_kindling_sink_unrecognised_falls_back_to_daemon() {
        // KDS-005: a typo must never silently disable capture (Off); it resolves
        // to the daemon default so capture is never lost by a bad value.
        for v in ["daemonn", "sqlite", "true", "1", "disable"] {
            assert_eq!(
                parse_kindling_sink(Some(v)),
                KindlingSinkSelection::Daemon,
                "{v:?}"
            );
        }
    }

    #[test]
    fn off_disables_the_daemon_usage_emitter() {
        // `off` short-circuits to no emitter (no rows) without needing a state
        // dir. Clear the break-glass so this exercises the `off` path itself.
        temp_env::with_vars(
            [
                ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", None::<&str>),
                ("ANVIL_KINDLING_SINK", Some("off")),
            ],
            || {
                assert!(
                    daemon_usage_emitter().is_none(),
                    "ANVIL_KINDLING_SINK=off must wire no command.invoked emitter",
                );
            },
        );
    }

    #[test]
    fn break_glass_disables_the_daemon_usage_emitter() {
        // ANVIL_INTERCEPT_DISABLE_OBSERVATION=1 silences this producer too
        // (parity with the docs' "every usage producer" claim), regardless of
        // the sink selection.
        temp_env::with_vars(
            [
                ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", Some("1")),
                ("ANVIL_KINDLING_SINK", Some("ndjson")),
            ],
            || {
                assert!(
                    daemon_usage_emitter().is_none(),
                    "the whole-observation break-glass must wire no emitter",
                );
            },
        );
    }

    #[test]
    fn default_unset_still_wires_an_emitter() {
        // KDS-005: with the var unset the producer is wired to the daemon sink
        // (the new default). Re-root state under a temp ANVIL_HOME so the test
        // never touches the real home.
        let home = tempdir().expect("temp home");
        temp_env::with_vars(
            [
                ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", None::<&str>),
                ("ANVIL_KINDLING_SINK", None::<&str>),
                ("ANVIL_HOME", Some(home.path().to_str().expect("utf8 home"))),
            ],
            || {
                assert!(
                    daemon_usage_emitter().is_some(),
                    "default/unset must wire the daemon usage emitter (KDS-005 default)",
                );
            },
        );
    }

    #[test]
    fn daemon_selection_wires_an_emitter() {
        // `ANVIL_KINDLING_SINK=daemon` builds the daemon-backed sink (capped
        // spool). No daemon contact happens at construction. Temp ANVIL_HOME
        // isolates the spool dir.
        let home = tempdir().expect("temp home");
        temp_env::with_vars(
            [
                ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", None::<&str>),
                ("ANVIL_KINDLING_SINK", Some("daemon")),
                ("ANVIL_HOME", Some(home.path().to_str().expect("utf8 home"))),
            ],
            || {
                assert!(
                    daemon_usage_emitter().is_some(),
                    "ANVIL_KINDLING_SINK=daemon must wire an emitter",
                );
            },
        );
    }

    /// N2 invariant (KDS-005): the daemon `command.invoked` sink MUST be wrapped
    /// in `NonBlockingObservationSink`, so emitting never blocks the caller (the
    /// daemon event loop) on the sink's I/O — even when the daemon is down and
    /// the inner `KindlingDaemonSink` would `block_on` a ~1s connect/spool. A
    /// burst of emits must return far faster than that work would take inline;
    /// the drain thread does it in the background. Guards against a regression
    /// that re-introduces the original event-loop stall.
    #[test]
    fn daemon_emitter_emits_are_non_blocking() {
        use anvil_intercept::kindling_observation::CommandInvokedEmissionRequest;
        use std::time::Instant;

        let home = tempdir().expect("temp home");
        temp_env::with_vars(
            [
                ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", None::<&str>),
                ("ANVIL_KINDLING_SINK", Some("daemon")),
                ("ANVIL_HOME", Some(home.path().to_str().expect("utf8 home"))),
            ],
            || {
                let emitter = daemon_usage_emitter().expect("daemon emitter wired");
                let params = serde_json::json!({});
                let start = Instant::now();
                for _ in 0..20 {
                    emitter.try_emit(&CommandInvokedEmissionRequest {
                        method: "anvil/gctx/search_symbols",
                        principal: Some("abc123"),
                        params: &params,
                        timestamp: "2026-06-26T10:00:00Z",
                        traceparent: None,
                    });
                }
                let elapsed = start.elapsed();
                // 20 `try_send`s are microseconds; unwrapped they'd be ~20 × the
                // connect budget. 1s is a generous, regression-catching bound.
                assert!(
                    elapsed < Duration::from_secs(1),
                    "20 emits took {elapsed:?} — the daemon sink is not NonBlocking-wrapped",
                );
            },
        );
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

    /// 094b: a non-UTF-8 byte mid-file must not defeat retention. Before the
    /// fix, `fs::read_to_string` returned `Err` on the first invalid byte and
    /// the trim bailed silently — so a torn write blocked every subsequent
    /// trim and the file grew past the cap forever. The trim now reads
    /// line-by-line and skips the corrupt line, so stale leading rows are
    /// still dropped.
    #[test]
    fn trim_skips_non_utf8_line_and_still_trims() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("usage.ndjson");

        let now = Utc::now();
        let old =
            (now - chrono::Duration::days(30)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let recent =
            (now - chrono::Duration::hours(1)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Seed: two stale rows, then a row carrying a raw non-UTF-8 byte
        // (0xFF — invalid in UTF-8, a torn-write signature), then a recent
        // row. The stale leading rows must still be dropped; the corrupt
        // line is dropped from the rewrite; the recent row survives.
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(format!("{{\"timestamp\":\"{old}\",\"tag\":\"old1\"}}\n").as_bytes());
        buf.extend_from_slice(format!("{{\"timestamp\":\"{old}\",\"tag\":\"old2\"}}\n").as_bytes());
        buf.extend_from_slice(b"{\"timestamp\":\"");
        buf.extend_from_slice(recent.as_bytes());
        buf.extend_from_slice(b"\",\"tag\":\"corr\xff\"}\n");
        buf.extend_from_slice(
            format!("{{\"timestamp\":\"{recent}\",\"tag\":\"recent\"}}\n").as_bytes(),
        );
        fs::write(&path, &buf).expect("seed sidecar with a non-UTF-8 byte");

        trim_usage_sidecar_at(&path, now);

        // The file must have been rewritten (the trim was not defeated).
        let after = fs::read(&path).expect("read trimmed");
        let after_str = String::from_utf8_lossy(&after);
        assert!(
            !after_str.contains("old1") && !after_str.contains("old2"),
            "stale leading rows must be dropped despite the corrupt line: {after_str}"
        );
        assert!(
            after_str.contains("recent"),
            "the recent row must survive: {after_str}"
        );
        // The corrupt byte must not remain in the rewritten file.
        assert!(
            !after.contains(&0xff),
            "the non-UTF-8 byte must be dropped from the rewrite"
        );
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
