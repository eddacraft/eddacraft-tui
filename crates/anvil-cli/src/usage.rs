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
//! [`record_invocation`] is called once from `main`, above the command
//! dispatch, so it fires uniformly for *every* command. There is no
//! per-command wiring to forget — adding a new subcommand cannot bypass
//! the producer (R2 mitigation). Emission is strictly best-effort: a
//! failure is logged and dropped, never surfaced to the exit code.
//!
//! ## Surfaced blocker (per the module's out-of-scope clause)
//!
//! The JSON-RPC daemon dispatch boundary (`anvil-intercept::ipc`) has
//! no user principal and no flag resolver, so the IPC-side producer is
//! intentionally **not** implemented here; it is filed as a follow-up
//! rather than emitting a principal-less, asymmetric row.

use std::env;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use anvil_intercept::kindling_observation::{
    CommandInvocationContext, CommandInvokedObservation, from_command_invocation,
};
use anvil_observability::TraceContext;
use anvil_observability::redaction::{ArgShape, redact_arg};
use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
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
    fs::create_dir_all(dir).with_context(|| format!("create salt dir {}", dir.display()))?;
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
    tok.strip_prefix('-')
        .is_none_or(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.'))
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
    writeln!(f, "{serialised}")
        .with_context(|| format!("append usage row to {}", path.display()))?;
    Ok(())
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
pub fn record_invocation(command_name: &str) -> Result<()> {
    let state_dir = credentials::credentials_dir().context("resolve usage state directory")?;
    let salt = load_or_create_salt_in(&state_dir)?;

    // Credential load failure or absence is fine — an unauthenticated
    // invocation records the `anonymous` principal.
    let email = credentials::load().ok().flatten().and_then(|c| c.email);
    let principal = anonymise_principal(email.as_deref(), &salt);

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
    // USAGE-001 emits an empty flag_set; USAGE-002 owns populating it.
    let obs = from_command_invocation(&ctx, arg_shapes, Vec::new());

    let path = usage_log_path(&state_dir);
    append_usage_observation_to(&path, &obs)
}

/// Resolve the usage NDJSON path under the user-scoped state directory.
fn usage_log_path(state_dir: &Path) -> PathBuf {
    state_dir.join("kindling").join(USAGE_NDJSON)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
            timestamp: "2026-06-14T10:00:00Z",
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
}
