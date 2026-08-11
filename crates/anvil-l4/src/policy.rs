use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anvil_config::{ConfigFormat, ParseError, parse_str};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// What an arriving commit must carry to be accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    /// Accept either a verified L3 witness OR a successful L4
    /// re-validation. Default for normal branches.
    L4OrL3,
    /// Only accept L4 re-validation; ignore L3 witnesses entirely.
    /// Useful for bot-only branches (Dependabot/Renovate) where the
    /// signal is "the bot doesn't run Anvil, so check it ourselves."
    L4Only,
    /// Only accept verified L3 witnesses; refuse anything else.
    /// Strict mode for high-assurance branches.
    L3Only,
}

/// Behaviour when a pushed commit has no L3 witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnNoWitness {
    /// Re-run validation server-side; the L4 witness (in
    /// `refs/notes/anvil-l4`) records the outcome.
    ValidateAtL4,
    /// Reject the push outright. Strict mode for branches that
    /// expect every commit to come with witness evidence already.
    Reject,
    /// Allow the push without further checks. Escape hatch for
    /// migration windows; not recommended for protected branches.
    Allow,
}

/// Behaviour when validation produces a block-level finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnBlock {
    /// Refuse the push.
    Reject,
    /// Allow the push; the block-level finding is recorded in the
    /// L4 witness but doesn't gate. Use sparingly.
    Allow,
}

/// Behaviour when validation produces a warn-level finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnWarn {
    /// Allow the push; the warning is recorded in the L4 witness.
    /// Default per CLAUDE.md "warnings over blocks."
    Allow,
    /// Promote warnings to blocks for this branch. Useful for
    /// high-assurance branches.
    Reject,
}

/// One per-branch rule. The first rule whose `pattern` matches the
/// branch name wins (declaration order = priority).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchRule {
    /// Glob pattern, e.g. `"main"`, `"dependabot/*"`, `"*"`.
    pub pattern: String,
    pub require: Requirement,
    pub on_no_witness: OnNoWitness,
    #[serde(default = "default_on_block")]
    pub on_block: OnBlock,
    #[serde(default = "default_on_warn")]
    pub on_warn: OnWarn,
}

fn default_on_block() -> OnBlock {
    OnBlock::Reject
}

fn default_on_warn() -> OnWarn {
    OnWarn::Allow
}

/// Baseline-adjacent metadata that drives `cutoff_commit` acceptance.
/// The `cutoff_commit` itself is populated by `anvil baseline`
/// (MLP-007); this crate only consumes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineSection {
    /// SHA of the commit at adoption time. Commits at or before
    /// this SHA in the first-parent ancestry are accepted without
    /// witness checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutoff_commit: Option<String>,
}

/// Parsed `anvil/policy.yml` (or `.json` / `.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Optional semver floor consumed by
    /// `anvil_rules::RequiredAnvilVersion`. Kept as `Option<String>`
    /// here so this crate doesn't pull in `anvil-rules`; the CLI
    /// caller composes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_anvil_version: Option<String>,

    /// Adoption-time pin from `anvil baseline`. Optional; absent in
    /// greenfield repos.
    #[serde(default)]
    pub baseline: BaselineSection,

    /// Per-branch rules in priority order.
    pub branches: Vec<BranchRule>,
}

#[derive(Debug, Error)]
pub enum PolicyParseError {
    /// `anvil-config` could not decode the file.
    #[error("config decode error: {0}")]
    Config(#[from] ParseError),
    /// The decoded JSON didn't match the policy schema.
    #[error("policy schema mismatch: {0}")]
    Schema(#[from] serde_json::Error),
    /// At least one [`BranchRule`] is required; an empty `branches`
    /// list would mean no branch can ever match.
    #[error("policy must declare at least one branch rule")]
    NoBranches,
    /// `pattern` was an empty string. Empty patterns can't be
    /// distinguished from missing patterns and would silently match
    /// nothing.
    #[error("branch rule has empty pattern")]
    EmptyPattern,
    /// `required_anvil_version` was present but empty. An empty
    /// string would parse as an unrecognised floor at consumer
    /// fire-time; refuse here so the error surfaces at the policy
    /// boundary instead.
    #[error("required_anvil_version is set but empty; omit the field or supply a value")]
    EmptyRequiredAnvilVersion,
    /// `baseline.cutoff_commit` was present but empty. An empty SHA
    /// can't represent a commit; refuse so a half-written
    /// `anvil baseline` artefact doesn't silently disable cutoff
    /// acceptance.
    #[error("baseline.cutoff_commit is set but empty; omit the field or supply a SHA")]
    EmptyCutoffCommit,
    /// `baseline.cutoff_commit` was present but not hex-shaped (e.g.
    /// a symbolic ref like `HEAD` or a branch name). Such a value
    /// would silently fail to match any SHA in the first-parent
    /// ancestry at fire-time, leaving the cutoff a no-op with no
    /// operator-visible signal. Refuse at the policy boundary so the
    /// typo surfaces before the next push.
    #[error("baseline.cutoff_commit must be a hex-only SHA (4–64 chars); got {raw:?}")]
    InvalidCutoffCommit { raw: String },
}

impl Policy {
    /// Parse a policy from raw text in the given format.
    pub fn parse(raw: &str, format: ConfigFormat, path: &Path) -> Result<Self, PolicyParseError> {
        let value = parse_str(raw, format, path)?;
        let policy: Self = serde_json::from_value(value)?;
        policy.validate()?;
        Ok(policy)
    }

    /// Reject obvious schema violations that serde alone doesn't
    /// catch.
    fn validate(&self) -> Result<(), PolicyParseError> {
        if self.branches.is_empty() {
            return Err(PolicyParseError::NoBranches);
        }
        for rule in &self.branches {
            if rule.pattern.is_empty() {
                return Err(PolicyParseError::EmptyPattern);
            }
        }
        if let Some(v) = &self.required_anvil_version
            && v.is_empty()
        {
            return Err(PolicyParseError::EmptyRequiredAnvilVersion);
        }
        if let Some(c) = &self.baseline.cutoff_commit {
            if c.is_empty() {
                return Err(PolicyParseError::EmptyCutoffCommit);
            }
            if !is_hex_sha_shape(c) {
                return Err(PolicyParseError::InvalidCutoffCommit { raw: c.clone() });
            }
        }
        Ok(())
    }
}

/// True when `raw` looks like a git SHA: 4–64 lowercase or uppercase
/// hex characters, no other content. Mirrors the shape check in
/// `anvil_hook::is_hex_sha` but local to this crate so `anvil-l4`
/// does not gain a dependency on `anvil-hook`.
fn is_hex_sha_shape(raw: &str) -> bool {
    let len = raw.len();
    (4..=64).contains(&len) && raw.chars().all(|c| c.is_ascii_hexdigit())
}

/// MLP2-031: errors from [`pin_cutoff_commit`].
#[derive(Debug, Error)]
pub enum PolicyPinError {
    /// I/O error reading or writing the policy file.
    #[error("io: {0}")]
    Io(#[from] io::Error),
    /// `anvil-config` could not decode the existing file. The pin
    /// operation refuses to overwrite an unreadable policy file —
    /// surfaces the parse error so the operator can fix it rather
    /// than silently replacing their (possibly comment-rich) policy
    /// with the minimal schema.
    #[error("existing policy parse: {0}")]
    Parse(#[from] ParseError),
    /// The existing file's root JSON value is not an object — e.g. a
    /// bare list or scalar. Refuse rather than wrap it in an object,
    /// because that would silently lose user data.
    #[error("existing policy root is not a JSON object")]
    NotAnObject,
    /// The existing file has a `baseline` key but it's not a map (it's
    /// a scalar or list). Pinning `baseline.cutoff_commit` would
    /// require overwriting that value with a fresh map, which would
    /// silently lose the user's data; surface it instead.
    #[error(
        "policy `baseline` field is not a map; cannot pin `cutoff_commit` without overwriting user data"
    )]
    BaselineNotAMap,
    /// `cutoff_commit` failed the hex-shape check that
    /// [`Policy::validate`] applies on read. Refuse at the pin
    /// boundary so a typo doesn't write a no-op cutoff to disk.
    #[error("cutoff_commit must be a hex-only SHA (4–64 chars); got {raw:?}")]
    InvalidCutoffCommit { raw: String },
    /// Re-serialising the updated value back to the on-disk format
    /// failed. Surfaced separately from `Io` so a transient encoder
    /// bug doesn't masquerade as a disk error.
    #[error("serialise updated policy as {format:?}: {message}")]
    Serialise {
        format: ConfigFormat,
        message: String,
    },
    /// `path` is a symlink. Mirrors the
    /// `anvil_baseline::io::BaselineIoError::SymlinkRefusal` pattern
    /// so a hostile worktree state cannot redirect the policy write
    /// out of the repo.
    #[error("`{path}` is a symlink; refusing to write policy through it")]
    SymlinkRefusal { path: PathBuf },
}

/// MLP2-031: write `cutoff_commit` into an existing `anvil/policy.yml`
/// (or `.json` / `.toml`), preserving every other top-level field and
/// the existing format on disk.
///
/// This is the producer side of MLP2-021 (the consumer):
/// `anvil baseline` runs after the baseline scan completes, computes
/// the cutoff SHA, and calls this to pin it into the policy file so
/// the pre-push hook reads it from policy rather than from
/// `baseline.json`. Round-trip: `pin_cutoff_commit(...)` →
/// [`Policy::parse`] → `policy.baseline.cutoff_commit` matches.
///
/// ### Semantics
///
/// - **Format is detected from the path's extension** (yaml / yml /
///   json / toml). An unrecognised extension is rejected at the
///   `anvil-config` boundary via [`ParseError`].
/// - **Existing file required** — refuse to bootstrap a policy file
///   that doesn't exist yet. The opinionated reason: a missing policy
///   file means the operator never ran `anvil init` (or equivalent)
///   to seed defaults, and silently creating one here would skip the
///   `branches:` block, leaving the policy with no rules. The
///   higher-level orchestrator (`anvil baseline`) is the right place
///   to bootstrap; this primitive only updates an existing file.
/// - **Hex-shape validation** runs before any disk I/O so a malformed
///   cutoff doesn't reach the file.
/// - **Atomic write** — serialises the updated value to a temp file
///   in the same directory, fsyncs, then renames into place.
/// - **Symlink refusal** mirrors [`anvil_baseline::io::save`]: the
///   path itself and the temp sibling are both checked.
/// - **Comments are NOT preserved**. The on-disk value round-trips
///   through `serde_json::Value` → re-encode, which is shape-faithful
///   but strips comments and reorders keys. Documented as a follow-up
///   limitation; the v1 baseline contract is shape-level round-trip.
///
/// Wired into `anvil baseline` by MLP2-032 — the CLI calls this
/// after `save()` so that `anvil/baseline.json` and
/// `anvil/policy.{yml,…}` agree on the cutoff in one flow.
pub fn pin_cutoff_commit(path: &Path, cutoff: &str) -> Result<(), PolicyPinError> {
    // Hex-shape validation first so a malformed cutoff never reaches
    // disk and never triggers a partial write.
    if !is_hex_sha_shape(cutoff) {
        return Err(PolicyPinError::InvalidCutoffCommit {
            raw: cutoff.to_string(),
        });
    }

    refuse_if_symlink(path)?;

    let format = ConfigFormat::from_path(path).ok_or_else(|| {
        PolicyPinError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unrecognised policy file extension: {}", path.display()),
        ))
    })?;

    let raw = fs::read_to_string(path)?;
    let value = parse_str(&raw, format, path)?;
    let Value::Object(mut object) = value else {
        return Err(PolicyPinError::NotAnObject);
    };

    // Replace (or insert) the `baseline.cutoff_commit` field while
    // preserving every other key under `baseline` (and at the root).
    let baseline_entry = object
        .entry("baseline".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let Value::Object(baseline_map) = baseline_entry else {
        return Err(PolicyPinError::BaselineNotAMap);
    };
    baseline_map.insert(
        "cutoff_commit".to_string(),
        Value::String(cutoff.to_string()),
    );

    let updated = Value::Object(object);
    let bytes = serialise_in_format(&updated, format)?;

    // Atomic exclusive-temp-then-rename. A fixed sibling name (.<file>.tmp)
    // would let concurrent pin operations clobber each other and opens a
    // check-then-create symlink TOCTOU window. create_new (O_CREAT|O_EXCL
    // on Unix) never follows a planted symlink at the chosen path and
    // fails closed if the name is already occupied, so each invocation
    // stages into a private file before rename.
    let parent = path
        .parent()
        .ok_or_else(|| PolicyPinError::Io(io::Error::other("policy path has no parent")))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| PolicyPinError::Io(io::Error::other("policy path has no file name")))?;

    let (mut staging, tmp_path) = open_exclusive_staging_file(parent, file_name)?;
    if let Err(e) = staging.write_all(&bytes).and_then(|()| staging.sync_all()) {
        drop(staging);
        // Incomplete staging content is not recoverable; remove it.
        let _ = fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    // Close the staging handle before rename: Windows cannot rename a
    // file that still has an open *source* handle.
    drop(staging);
    if let Err(e) = refuse_if_symlink(path) {
        // Destination was never replaced; staging is disposable.
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    // Do not delete the exclusive staging file if replace fails. On
    // Windows, atomic_replace may have already removed the destination
    // before a second rename fails — leaving the staging file is the
    // only remaining good copy for operator recovery.
    atomic_replace(&tmp_path, path)?;
    Ok(())
}

/// Create a uniquely named staging file next to the destination policy.
///
/// Uses `OpenOptions::create_new` so the open is exclusive: on Unix this
/// maps to `O_CREAT|O_EXCL`, which refuses to follow a pre-existing
/// symlink and fails with `AlreadyExists` if the path is occupied. That
/// closes the fixed-name concurrent-clobber and check-then-create symlink
/// races that a shared `.<name>.tmp` path allowed.
fn open_exclusive_staging_file(
    parent: &Path,
    policy_file_name: &OsStr,
) -> Result<(File, PathBuf), PolicyPinError> {
    let stem = policy_file_name.to_string_lossy();
    let pid = std::process::id();
    for attempt in 0u32..32 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        // Mix pid, wall-clock nanos, and attempt so concurrent callers and
        // same-nanosecond retries almost never collide; create_new still
        // serialises any true collision safely.
        let nonce =
            nanos ^ (u128::from(pid) << 64) ^ (u128::from(attempt) << 48) ^ u128::from(attempt);
        let tmp_name = format!(".{stem}.{pid}-{nonce}.tmp");
        let tmp_path = parent.join(&tmp_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
        {
            Ok(file) => return Ok((file, tmp_path)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    Err(PolicyPinError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "exhausted exclusive temporary policy file name attempts",
    )))
}

fn serialise_in_format(value: &Value, format: ConfigFormat) -> Result<Vec<u8>, PolicyPinError> {
    match format {
        ConfigFormat::Yaml | ConfigFormat::Yml => serde_yaml::to_string(value)
            .map(|s| {
                // serde_yaml emits a trailing newline already; the
                // contract here is "ends with \n", so the conversion
                // is a straight `into_bytes`.
                s.into_bytes()
            })
            .map_err(|e| PolicyPinError::Serialise {
                format,
                message: e.to_string(),
            }),
        ConfigFormat::Json => serde_json::to_vec_pretty(value)
            .map(|mut v| {
                v.push(b'\n');
                v
            })
            .map_err(|e| PolicyPinError::Serialise {
                format,
                message: e.to_string(),
            }),
        ConfigFormat::Toml => {
            // toml's encoder works on `toml::Value`, so convert via
            // its serde bridge. A toml policy is unusual in practice
            // (yaml is the canonical format) but we support it for
            // multi-format parity per the MLP2-031 expected outcome.
            let toml_value: toml::Value =
                serde_json::from_value(value.clone()).map_err(|e| PolicyPinError::Serialise {
                    format,
                    message: format!("json→toml bridge: {e}"),
                })?;
            toml::to_string(&toml_value)
                .map(String::into_bytes)
                .map_err(|e| PolicyPinError::Serialise {
                    format,
                    message: e.to_string(),
                })
        }
    }
}

fn refuse_if_symlink(path: &Path) -> Result<(), PolicyPinError> {
    match path.symlink_metadata() {
        Ok(meta) if meta.file_type().is_symlink() => Err(PolicyPinError::SymlinkRefusal {
            path: path.to_path_buf(),
        }),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Atomic replace mirroring `anvil_baseline::io::atomic_replace`.
///
/// On POSIX, `rename` overwrites silently and is atomic; the outer
/// caller's `refuse_if_symlink(path)` immediately before this call
/// is the load-bearing TOCTOU guard.
///
/// On Windows, `std::fs::rename` calls `MoveFileExW` which can fail
/// with `AlreadyExists` (this fallback's specialised path) but may
/// also fail with `PermissionDenied` for an open destination — that
/// case surfaces as a raw `Io` error rather than a refusal, because
/// Anvil's pre-push lane is POSIX-only today. The Windows path is
/// best-effort here; full Windows-atomic policy writes are tracked
/// alongside the broader MLP2 Windows-driver work.
#[allow(dead_code)]
fn atomic_replace(src: &Path, dest: &Path) -> Result<(), PolicyPinError> {
    match fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            refuse_if_symlink(dest)?;
            fs::remove_file(dest)?;
            fs::rename(src, dest)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML: &str = r"
required_anvil_version: '0.6.0'
baseline:
  cutoff_commit: a3b2ea4e
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
    on_block: reject
  - pattern: dependabot/*
    require: l4_only
    on_no_witness: validate_at_l4
  - pattern: '*'
    require: l4_or_l3
    on_no_witness: validate_at_l4
";

    #[test]
    fn parse_accepts_canonical_yaml_shape() {
        let p = Policy::parse(VALID_YAML, ConfigFormat::Yaml, Path::new("<test>")).unwrap();
        assert_eq!(p.required_anvil_version.as_deref(), Some("0.6.0"));
        assert_eq!(p.baseline.cutoff_commit.as_deref(), Some("a3b2ea4e"));
        assert_eq!(p.branches.len(), 3);
        assert_eq!(p.branches[0].pattern, "main");
        assert_eq!(p.branches[0].require, Requirement::L4OrL3);
        assert_eq!(p.branches[0].on_no_witness, OnNoWitness::ValidateAtL4);
        assert_eq!(p.branches[0].on_block, OnBlock::Reject);
        assert_eq!(p.branches[0].on_warn, OnWarn::Allow); // default
        assert_eq!(p.branches[1].require, Requirement::L4Only);
    }

    #[test]
    fn parse_accepts_json_equivalent() {
        let json = r#"{
            "branches": [
                {"pattern": "main", "require": "l4_or_l3", "on_no_witness": "validate_at_l4"}
            ]
        }"#;
        let p = Policy::parse(json, ConfigFormat::Json, Path::new("<test>")).unwrap();
        assert_eq!(p.branches.len(), 1);
        assert!(p.required_anvil_version.is_none());
    }

    #[test]
    fn parse_accepts_toml_equivalent() {
        let toml = r#"
[[branches]]
pattern = "main"
require = "l4_or_l3"
on_no_witness = "validate_at_l4"
"#;
        let p = Policy::parse(toml, ConfigFormat::Toml, Path::new("<test>")).unwrap();
        assert_eq!(p.branches.len(), 1);
        assert_eq!(p.branches[0].pattern, "main");
    }

    #[test]
    fn parse_rejects_empty_branches() {
        let yaml = "branches: []\n";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::NoBranches));
    }

    #[test]
    fn parse_rejects_empty_pattern() {
        let yaml = r"
branches:
  - pattern: ''
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::EmptyPattern));
    }

    #[test]
    fn parse_rejects_unknown_require_value() {
        let yaml = r"
branches:
  - pattern: main
    require: bogus
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::Schema(_)));
    }

    #[test]
    fn parse_rejects_unknown_on_no_witness_value() {
        let yaml = r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: maybe
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::Schema(_)));
    }

    #[test]
    fn defaults_for_on_block_and_on_warn_kick_in_when_omitted() {
        let yaml = r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let p = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap();
        // ADR-037 §D-5 defaults: blocks reject, warns allow.
        assert_eq!(p.branches[0].on_block, OnBlock::Reject);
        assert_eq!(p.branches[0].on_warn, OnWarn::Allow);
    }

    #[test]
    fn baseline_section_default_is_empty() {
        let yaml = r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let p = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap();
        assert!(p.baseline.cutoff_commit.is_none());
    }

    #[test]
    fn parse_propagates_invalid_yaml_as_config_error() {
        let yaml = "branches: [\n";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::Config(_)));
    }

    #[test]
    fn parse_rejects_empty_required_anvil_version() {
        let yaml = r"
required_anvil_version: ''
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::EmptyRequiredAnvilVersion));
    }

    #[test]
    fn parse_rejects_empty_cutoff_commit() {
        let yaml = r"
baseline:
  cutoff_commit: ''
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::EmptyCutoffCommit));
    }

    #[test]
    fn parse_rejects_symbolic_ref_as_cutoff_commit() {
        // MLP2-021 Council follow-up: a symbolic ref like `HEAD`
        // or a branch name would silently fail to match any SHA in
        // the first-parent ancestry at fire-time, leaving the
        // cutoff a no-op with no operator signal. Refuse at the
        // policy boundary so the typo surfaces before push.
        for bad in ["HEAD", "main", "release/0.7", "v0.7.0"] {
            let yaml = format!(
                r"
baseline:
  cutoff_commit: '{bad}'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
"
            );
            let err = Policy::parse(&yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
            match err {
                PolicyParseError::InvalidCutoffCommit { raw } => {
                    assert_eq!(raw, bad);
                }
                other => panic!("expected InvalidCutoffCommit for {bad:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_rejects_three_char_cutoff_commit_as_too_short() {
        // 3-char hex slips past `all is_ascii_hexdigit` but isn't a
        // meaningful prefix for git rev-list lookup.
        let yaml = r"
baseline:
  cutoff_commit: 'abc'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        let err = Policy::parse(yaml, ConfigFormat::Yaml, Path::new("<test>")).unwrap_err();
        assert!(matches!(err, PolicyParseError::InvalidCutoffCommit { .. }));
    }

    #[test]
    fn parse_accepts_short_and_full_hex_cutoff_commit() {
        // 7-char abbreviation (git's default --abbrev=7) and full
        // 40-char sha1 / 64-char sha256 all parse.
        for good in ["a3b2ea4", "a3b2ea4e", &"a".repeat(40), &"b".repeat(64)] {
            let yaml = format!(
                r"
baseline:
  cutoff_commit: '{good}'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
"
            );
            let p = Policy::parse(&yaml, ConfigFormat::Yaml, Path::new("<test>"))
                .unwrap_or_else(|e| panic!("{good:?} should parse, got {e}"));
            assert_eq!(p.baseline.cutoff_commit.as_deref(), Some(good));
        }
    }

    // ---- MLP2-031: pin_cutoff_commit ---------------------------------

    fn yaml_without_cutoff() -> &'static str {
        r"required_anvil_version: '0.7.0'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
"
    }

    #[test]
    fn pin_inserts_cutoff_into_yaml_policy_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        fs::write(&path, yaml_without_cutoff()).unwrap();

        pin_cutoff_commit(&path, "a3b2ea4e").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let p = Policy::parse(&raw, ConfigFormat::Yaml, &path).unwrap();
        assert_eq!(p.baseline.cutoff_commit.as_deref(), Some("a3b2ea4e"));
        assert_eq!(p.required_anvil_version.as_deref(), Some("0.7.0"));
        assert_eq!(p.branches.len(), 1);
        assert_eq!(p.branches[0].pattern, "main");
    }

    #[test]
    fn pin_updates_existing_cutoff_in_yaml_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        let with_old = r"baseline:
  cutoff_commit: 'aaaaaaaa'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        fs::write(&path, with_old).unwrap();

        pin_cutoff_commit(&path, "bbbbbbbb").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let p = Policy::parse(&raw, ConfigFormat::Yaml, &path).unwrap();
        assert_eq!(p.baseline.cutoff_commit.as_deref(), Some("bbbbbbbb"));
    }

    #[test]
    fn pin_round_trips_through_json_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.json");
        let json = r#"{"branches":[{"pattern":"main","require":"l4_or_l3","on_no_witness":"validate_at_l4"}]}"#;
        fs::write(&path, json).unwrap();

        pin_cutoff_commit(&path, &"a".repeat(40)).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let p = Policy::parse(&raw, ConfigFormat::Json, &path).unwrap();
        assert_eq!(p.baseline.cutoff_commit.as_deref(), Some(&*"a".repeat(40)));
    }

    #[test]
    fn pin_round_trips_through_toml_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.toml");
        let toml_str = r#"
[[branches]]
pattern = "main"
require = "l4_or_l3"
on_no_witness = "validate_at_l4"
"#;
        fs::write(&path, toml_str).unwrap();

        pin_cutoff_commit(&path, "a3b2ea4e").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let p = Policy::parse(&raw, ConfigFormat::Toml, &path).unwrap();
        assert_eq!(p.baseline.cutoff_commit.as_deref(), Some("a3b2ea4e"));
    }

    #[test]
    fn pin_refuses_invalid_cutoff_before_io() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        fs::write(&path, yaml_without_cutoff()).unwrap();

        // Symbolic ref shape — rejected by the hex-shape check.
        let err = pin_cutoff_commit(&path, "HEAD").unwrap_err();
        match err {
            PolicyPinError::InvalidCutoffCommit { raw } => assert_eq!(raw, "HEAD"),
            other => panic!("expected InvalidCutoffCommit, got {other:?}"),
        }
        // Empty cutoff — rejected.
        let err = pin_cutoff_commit(&path, "").unwrap_err();
        assert!(matches!(err, PolicyPinError::InvalidCutoffCommit { .. }));
        // Too short (3 chars) — rejected.
        let err = pin_cutoff_commit(&path, "abc").unwrap_err();
        assert!(matches!(err, PolicyPinError::InvalidCutoffCommit { .. }));
        // Non-hex char — rejected.
        let err = pin_cutoff_commit(&path, "gggggg").unwrap_err();
        assert!(matches!(err, PolicyPinError::InvalidCutoffCommit { .. }));

        // The original file is untouched after refusals.
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("cutoff_commit"));
    }

    #[test]
    fn pin_refuses_when_policy_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        let err = pin_cutoff_commit(&path, "a3b2ea4e").unwrap_err();
        assert!(matches!(err, PolicyPinError::Io(_)));
    }

    #[test]
    fn pin_preserves_unknown_top_level_fields() {
        // Forward-compatibility: a policy file may contain top-level
        // fields the current crate doesn't model yet. The pin operation
        // must round-trip them, not drop them. Use a future-proofing
        // marker `_x_experiment_flag` so the test pins the contract
        // even as the schema grows.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        let with_extra = r"_x_experiment_flag: keep-me
required_anvil_version: '0.7.0'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        fs::write(&path, with_extra).unwrap();

        pin_cutoff_commit(&path, "a3b2ea4e").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(
            raw.contains("_x_experiment_flag") && raw.contains("keep-me"),
            "unknown field dropped: {raw}",
        );
        assert!(raw.contains("a3b2ea4e"), "cutoff_commit not written: {raw}");
    }

    #[test]
    fn pin_writes_atomically_via_temp_then_rename() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        fs::write(&path, yaml_without_cutoff()).unwrap();

        pin_cutoff_commit(&path, "a3b2ea4e").unwrap();

        // After pin, neither the legacy fixed staging name nor any unique
        // exclusive staging temp may linger beside the policy file.
        let legacy_tmp = tmp.path().join(".policy.yaml.tmp");
        assert!(
            !legacy_tmp.exists(),
            "legacy temp file leaked: {legacy_tmp:?}"
        );
        let leftovers: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name())
            .filter(|n| {
                let s = n.to_string_lossy();
                s.starts_with(".policy.yaml.") && s.ends_with(".tmp")
            })
            .collect();
        assert!(leftovers.is_empty(), "staging temps leaked: {leftovers:?}");
    }

    #[cfg(unix)]
    #[test]
    fn pin_refuses_when_policy_path_is_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let real = tempfile::tempdir().unwrap();
        let real_path = real.path().join("real_policy.yaml");
        fs::write(&real_path, yaml_without_cutoff()).unwrap();
        let link = tmp.path().join("policy.yaml");
        symlink(&real_path, &link).unwrap();

        let err = pin_cutoff_commit(&link, "a3b2ea4e").unwrap_err();
        assert!(matches!(err, PolicyPinError::SymlinkRefusal { .. }));
        // Real file unchanged.
        let raw = fs::read_to_string(&real_path).unwrap();
        assert!(!raw.contains("cutoff_commit"));
    }

    #[cfg(unix)]
    #[test]
    fn pin_does_not_follow_legacy_fixed_temp_sibling_symlink() {
        // The historical fixed staging path (.<name>.tmp) is a TOCTOU and
        // concurrent-clobber hazard. Pin must not use it: a pre-planted
        // symlink there must neither be followed nor block the write.
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        fs::write(&path, yaml_without_cutoff()).unwrap();
        let outside = tmp.path().join("outside.yaml");
        symlink(&outside, tmp.path().join(".policy.yaml.tmp")).unwrap();

        pin_cutoff_commit(&path, "a3b2ea4e").unwrap();

        assert!(!outside.exists(), "legacy fixed temp symlink was followed");
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("a3b2ea4e"), "cutoff not written: {raw}");
        assert!(
            tmp.path()
                .join(".policy.yaml.tmp")
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink(),
            "legacy plant should remain an untouched symlink",
        );
    }

    #[test]
    fn pin_concurrent_calls_do_not_share_staging_path() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let tmp = tempfile::tempdir().unwrap();
        let path = Arc::new(tmp.path().join("policy.yaml"));
        fs::write(path.as_ref(), yaml_without_cutoff()).unwrap();

        let barrier = Arc::new(Barrier::new(2));
        let cutoffs = ["aaaaaaaa", "bbbbbbbb"];
        let mut handles = Vec::new();
        for cutoff in cutoffs {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                pin_cutoff_commit(path.as_ref(), cutoff)
            }));
        }
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert!(
            results.iter().all(std::result::Result::is_ok),
            "concurrent pin must not fail from shared staging clobber: {results:?}",
        );
        let raw = fs::read_to_string(path.as_ref()).unwrap();
        let p = Policy::parse(&raw, ConfigFormat::Yaml, path.as_ref()).unwrap();
        let cutoff = p.baseline.cutoff_commit.as_deref().unwrap();
        assert!(
            cutoff == "aaaaaaaa" || cutoff == "bbbbbbbb",
            "final cutoff should be one of the concurrent writers, got {cutoff}",
        );
        let leftovers: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name())
            .filter(|n| {
                let s = n.to_string_lossy();
                s.starts_with(".policy.yaml.") && s.ends_with(".tmp")
            })
            .collect();
        assert!(leftovers.is_empty(), "staging temps leaked: {leftovers:?}");
    }

    #[test]
    fn open_exclusive_staging_file_returns_distinct_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path();
        let name = std::ffi::OsStr::new("policy.yaml");
        let (f1, p1) = open_exclusive_staging_file(parent, name).unwrap();
        drop(f1);
        let (f2, p2) = open_exclusive_staging_file(parent, name).unwrap();
        drop(f2);
        assert_ne!(p1, p2, "exclusive staging paths must not collide");
        fs::write(&p1, b"one").unwrap();
        fs::write(&p2, b"two").unwrap();
        assert_eq!(fs::read(&p1).unwrap(), b"one");
        assert_eq!(fs::read(&p2).unwrap(), b"two");
        let _ = fs::remove_file(&p1);
        let _ = fs::remove_file(&p2);
    }

    #[test]
    fn pin_refuses_when_baseline_field_is_not_a_map() {
        // Council #C-MLP2-031-1: a hand-edited policy file that puts a
        // scalar or list under `baseline:` must not be silently
        // overwritten with a fresh map — that would lose the user's
        // data. Refuse with `BaselineNotAMap` so the typo surfaces.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yaml");
        let with_scalar_baseline = r"baseline: 'oops a scalar'
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
";
        fs::write(&path, with_scalar_baseline).unwrap();

        let err = pin_cutoff_commit(&path, "a3b2ea4e").unwrap_err();
        assert!(matches!(err, PolicyPinError::BaselineNotAMap));

        // Original file untouched.
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("oops a scalar"));
        assert!(!raw.contains("a3b2ea4e"));
    }

    #[test]
    fn pin_refuses_when_root_is_not_a_json_object() {
        // A policy file whose root is a list or scalar is invalid by
        // schema, but `parse_str` admits it before `Policy::parse`
        // would. Pin surfaces it as `NotAnObject` rather than
        // attempting to wrap.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.json");
        fs::write(&path, "[1,2,3]").unwrap();

        let err = pin_cutoff_commit(&path, "a3b2ea4e").unwrap_err();
        assert!(matches!(err, PolicyPinError::NotAnObject));
    }

    #[test]
    fn pin_writes_yaml_to_yml_extension_too() {
        // `.yml` and `.yaml` are both yaml — pin both extensions so a
        // policy file using either lands in the same shape on disk.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("policy.yml");
        fs::write(&path, yaml_without_cutoff()).unwrap();

        pin_cutoff_commit(&path, "a3b2ea4e").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let p = Policy::parse(&raw, ConfigFormat::Yml, &path).unwrap();
        assert_eq!(p.baseline.cutoff_commit.as_deref(), Some("a3b2ea4e"));
    }
}
