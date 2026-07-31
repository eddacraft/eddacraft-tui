//! Activation finding baseline (LAUNCH-010): `.anvil/baseline.json`.
//!
//! Fingerprints first-run antipattern/secret findings for honest activation
//! copy — not the new-edges architecture baseline (`anvil baseline`).

use std::collections::BTreeSet;
use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anvil_checks::antipattern::{Warning, create_warning_fingerprint};
use anvil_checks::secret::SecretFinding;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Schema version. Bumped on any breaking shape change. Readers MUST
/// reject unknown versions rather than silently accepting partial data.
pub const SCHEMA_VERSION: u32 = 1;

const BASELINE_DIR: &str = ".anvil";
const BASELINE_FILE: &str = "baseline.json";

/// Errors from baseline read / write. Manually implemented (no
/// `thiserror` dep on `anvil-cli`) to keep the dep surface tight.
///
/// PR #1293 review fix (Copilot): the read and write paths use
/// distinct variants so a serialisation failure on write does not
/// surface as `"invalid baseline JSON"` — that message is only ever
/// honest when we read back something we couldn't parse.
#[derive(Debug)]
pub enum BaselineError {
    Io {
        path: String,
        source: std::io::Error,
    },
    /// `serde_json` failed to parse the on-disk baseline. Set only
    /// from the read path.
    InvalidJson(String),
    /// `serde_json` failed to serialise the in-memory baseline. Set
    /// only from the write path. In practice unreachable for the
    /// `Baseline` shape this module owns, but we propagate rather
    /// than panic if a future schema change introduces a non-
    /// serialisable field.
    Serialise(String),
    UnsupportedSchema {
        found: u32,
    },
}

impl fmt::Display for BaselineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "I/O error at {path}: {source}"),
            Self::InvalidJson(msg) => write!(f, "invalid baseline JSON: {msg}"),
            Self::Serialise(msg) => write!(f, "could not serialise baseline: {msg}"),
            Self::UnsupportedSchema { found } => write!(
                f,
                "unsupported baseline schema_version {found} (this build understands version {SCHEMA_VERSION}); \
                 delete `.anvil/baseline.json` to regenerate"
            ),
        }
    }
}

impl std::error::Error for BaselineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidJson(_) | Self::Serialise(_) | Self::UnsupportedSchema { .. } => None,
        }
    }
}

/// Path to the baseline file under `root`.
#[must_use]
pub fn baseline_path(root: &Path) -> PathBuf {
    root.join(BASELINE_DIR).join(BASELINE_FILE)
}

/// True when a baseline file exists under `root`.
#[must_use]
pub fn baseline_exists(root: &Path) -> bool {
    baseline_path(root).exists()
}

/// On-disk shape. Keys are stable contract — readers in
/// `anvil watch` / `anvil check` will rely on them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Baseline {
    /// Bumped on any breaking shape change. Readers MUST reject
    /// unknown versions.
    pub schema_version: u32,
    /// RFC3339 UTC timestamp of when the baseline was first written.
    pub created_at: String,
    /// Fingerprints of every non-suppressed finding present at
    /// activation time. A finding is considered baselined if its
    /// fingerprint is in this set; future findings outside it are
    /// "new" and must be surfaced.
    pub fingerprints: BTreeSet<String>,
    /// Per-kind counts at the moment the baseline was written.
    /// Snapshot only — `fingerprints.len()` is the authoritative
    /// total, but split counts let surfaces phrase the summary
    /// honestly ("3 antipattern, 1 secret" vs "4 findings").
    ///
    /// **Counts vs `fingerprints.len()`:** these are RAW finding
    /// counts and may exceed `fingerprints.len()` when multiple
    /// findings collapse to the same fingerprint (e.g. two secret
    /// regex hits on the same line + pattern, or two antipattern
    /// rules with identical id/file/line/pattern). Surfaces that
    /// need the deduped total should read [`Baseline::total`];
    /// surfaces that want raw kind breakdowns read these counts.
    pub counts: BaselineCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineCounts {
    /// Raw count of non-suppressed antipattern warnings the
    /// activation scan saw. May exceed the unique antipattern
    /// fingerprint count when multiple warnings collapse to the
    /// same `id:file:line:pattern` fingerprint.
    pub antipattern_findings: usize,
    /// Raw count of secret findings the activation scan saw. May
    /// exceed the unique secret fingerprint count for the same
    /// reason as above (e.g. two regex hits on the same line +
    /// pattern collapse to one fingerprint).
    pub secret_findings: usize,
}

impl Baseline {
    /// Total fingerprint count. Matches `fingerprints.len()` — the
    /// counts struct may differ if a finding produced no fingerprint
    /// (defensive: we only insert fingerprints we could derive).
    #[must_use]
    pub fn total(&self) -> usize {
        self.fingerprints.len()
    }

    /// True when this baseline contains a fingerprint matching the
    /// supplied antipattern warning.
    ///
    /// Contract surface for downstream PRs that wire the baseline
    /// into `anvil watch` / `anvil check` filtering. Not yet called
    /// from production code in this PR — exercised by unit tests.
    #[must_use]
    #[allow(dead_code)] // contract surface for downstream consumers
    pub fn contains_warning(&self, w: &Warning) -> bool {
        self.fingerprints.contains(&warning_fingerprint(w))
    }

    /// True when this baseline contains a fingerprint matching the
    /// supplied secret finding.
    ///
    /// Contract surface for downstream PRs (see [`Self::contains_warning`]).
    #[must_use]
    #[allow(dead_code)] // contract surface for downstream consumers
    pub fn contains_secret(&self, s: &SecretFinding) -> bool {
        self.fingerprints.contains(&secret_fingerprint(s))
    }
}

/// Build a baseline from the supplied finding sets. Suppressed
/// antipattern warnings are excluded — they are already silent in
/// regular scan output, so baselining them would be redundant and would
/// cause an un-suppression to look like a "new" finding.
#[must_use]
pub fn build_baseline(warnings: &[Warning], secrets: &[SecretFinding]) -> Baseline {
    let mut fingerprints = BTreeSet::new();
    let mut antipattern_findings = 0usize;

    for w in warnings.iter().filter(|w| w.suppressed.is_none()) {
        fingerprints.insert(warning_fingerprint(w));
        antipattern_findings += 1;
    }
    for s in secrets {
        fingerprints.insert(secret_fingerprint(s));
    }

    Baseline {
        schema_version: SCHEMA_VERSION,
        created_at: Utc::now().to_rfc3339(),
        fingerprints,
        counts: BaselineCounts {
            antipattern_findings,
            secret_findings: secrets.len(),
        },
    }
}

/// Read the baseline from `root`. Returns `Ok(None)` when the file is
/// absent — that is the expected pre-activation state, not an error.
pub fn read_baseline(root: &Path) -> Result<Option<Baseline>, BaselineError> {
    let path = baseline_path(root);
    let path_str = path.display().to_string();
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(BaselineError::Io {
                path: path_str,
                source: e,
            });
        }
    };

    let baseline: Baseline =
        serde_json::from_str(&contents).map_err(|e| BaselineError::InvalidJson(e.to_string()))?;

    if baseline.schema_version != SCHEMA_VERSION {
        return Err(BaselineError::UnsupportedSchema {
            found: baseline.schema_version,
        });
    }

    Ok(Some(baseline))
}

/// Write the baseline atomically under `root`. Creates `.anvil/` if
/// absent. Uses tempfile + persist so a crashed write cannot leave a
/// truncated `baseline.json` that future reads would reject as invalid.
pub fn write_baseline(root: &Path, baseline: &Baseline) -> Result<(), BaselineError> {
    let dir = root.join(BASELINE_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| BaselineError::Io {
        path: dir.display().to_string(),
        source: e,
    })?;

    let path = baseline_path(root);
    let path_str = path.display().to_string();
    let body = serde_json::to_string_pretty(baseline)
        .map_err(|e| BaselineError::Serialise(e.to_string()))?;

    // tempfile_in writes into the target directory so the rename below
    // is a same-filesystem move (atomic on POSIX, atomic-replace on
    // Windows via `persist`).
    let mut tmp = tempfile::Builder::new()
        .prefix(".baseline-")
        .suffix(".json.tmp")
        .tempfile_in(&dir)
        .map_err(|e| BaselineError::Io {
            path: dir.display().to_string(),
            source: e,
        })?;
    tmp.write_all(body.as_bytes())
        .map_err(|e| BaselineError::Io {
            path: path_str.clone(),
            source: e,
        })?;
    tmp.flush().map_err(|e| BaselineError::Io {
        path: path_str.clone(),
        source: e,
    })?;
    tmp.persist(&path).map_err(|e| BaselineError::Io {
        path: path_str,
        source: e.error,
    })?;
    Ok(())
}

/// Stable fingerprint for an antipattern warning. Wraps
/// `anvil_checks::antipattern::create_warning_fingerprint` and prefixes
/// with the kind so the namespace is unambiguous in the on-disk set.
fn warning_fingerprint(w: &Warning) -> String {
    format!("antipattern:{}", create_warning_fingerprint(w))
}

/// Stable fingerprint for a secret finding. Pattern name + file + line
/// is a defensible fingerprint: secret pattern names are stable across
/// releases, and the redacted match is intentionally non-deterministic
/// (different redaction lengths) so it would be a poor key.
///
/// Path normalisation: backslashes are collapsed to forward slashes so
/// the same finding on Windows and POSIX produces the same fingerprint.
/// Callers are expected to feed repo-relative paths (see
/// `services::sample_analyser::run_baseline_scan`); this normalisation
/// is the defensive last line for any future call site that forgets.
fn secret_fingerprint(s: &SecretFinding) -> String {
    let normalised_file = s.file.replace('\\', "/");
    format!("secret:{}:{}:{}", normalised_file, s.line, s.pattern_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_checks::antipattern::{Confidence, Location, WarningCategory, WarningSeverity};
    use anvil_checks::secret::types::FindingType;
    use tempfile::TempDir;

    fn sample_warning(id: &str, file: &str, line: usize) -> Warning {
        Warning {
            id: id.to_string(),
            fingerprint: None,
            category: WarningCategory::AntiPattern,
            severity: WarningSeverity::Warning,
            confidence: Confidence::High,
            title: "t".to_string(),
            message: "m".to_string(),
            explanation: "e".to_string(),
            suggestion: "s".to_string(),
            nudge: None,
            location: Location {
                file: file.to_string(),
                line,
                column: None,
                end_line: None,
                end_column: None,
            },
            pattern: Some("ap-pattern".to_string()),
            suppressed: None,
            family: None,
            definition_ref: None,
            spectrum_position: None,
        }
    }

    fn sample_secret(file: &str, line: usize, pattern: &str) -> SecretFinding {
        SecretFinding {
            file: file.to_string(),
            line,
            finding_type: FindingType::Pattern,
            pattern_name: pattern.to_string(),
            redacted_match: "***".to_string(),
            redacted_line: "***".to_string(),
        }
    }

    #[test]
    fn build_includes_warnings_and_secrets() {
        let warnings = vec![sample_warning("w-1", "src/a.ts", 10)];
        let secrets = vec![sample_secret("src/a.ts", 12, "aws-access-key")];
        let b = build_baseline(&warnings, &secrets);
        assert_eq!(b.counts.antipattern_findings, 1);
        assert_eq!(b.counts.secret_findings, 1);
        assert_eq!(b.total(), 2);
        assert!(b.contains_warning(&warnings[0]));
        assert!(b.contains_secret(&secrets[0]));
    }

    #[test]
    fn build_excludes_suppressed_warnings() {
        let mut w = sample_warning("w-1", "src/a.ts", 10);
        w.suppressed = Some(anvil_checks::antipattern::Suppression {
            reason: "ack".to_string(),
            author: None,
            timestamp: None,
            scope: anvil_checks::antipattern::SuppressionScope::Line,
        });
        let b = build_baseline(&[w], &[]);
        assert_eq!(b.counts.antipattern_findings, 0);
        assert_eq!(b.total(), 0);
    }

    #[test]
    fn write_then_read_round_trip() {
        let dir = TempDir::new().unwrap();
        let warnings = vec![sample_warning("w-1", "src/a.ts", 10)];
        let secrets = vec![sample_secret("src/b.rs", 5, "github-pat")];
        let b = build_baseline(&warnings, &secrets);

        write_baseline(dir.path(), &b).expect("write");
        let loaded = read_baseline(dir.path()).expect("read").expect("present");

        assert_eq!(loaded, b);
        assert!(loaded.contains_warning(&warnings[0]));
        assert!(loaded.contains_secret(&secrets[0]));
    }

    #[test]
    fn read_returns_none_when_absent() {
        let dir = TempDir::new().unwrap();
        assert!(read_baseline(dir.path()).unwrap().is_none());
        assert!(!baseline_exists(dir.path()));
    }

    #[test]
    fn read_rejects_unknown_schema_version() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(BASELINE_DIR)).unwrap();
        std::fs::write(
            baseline_path(dir.path()),
            r#"{"schema_version":999,"created_at":"2026-01-01T00:00:00Z","fingerprints":[],"counts":{"antipattern_findings":0,"secret_findings":0}}"#,
        )
        .unwrap();
        let err = read_baseline(dir.path()).unwrap_err();
        assert!(
            matches!(err, BaselineError::UnsupportedSchema { found: 999 }),
            "expected UnsupportedSchema, got {err:?}"
        );
    }

    #[test]
    fn read_rejects_invalid_json() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(BASELINE_DIR)).unwrap();
        std::fs::write(baseline_path(dir.path()), "not json").unwrap();
        assert!(matches!(
            read_baseline(dir.path()).unwrap_err(),
            BaselineError::InvalidJson(_)
        ));
    }

    #[test]
    fn write_creates_anvil_dir_if_absent() {
        let dir = TempDir::new().unwrap();
        let b = build_baseline(&[], &[]);
        write_baseline(dir.path(), &b).expect("write");
        assert!(dir.path().join(".anvil").is_dir());
        assert!(baseline_exists(dir.path()));
    }

    #[test]
    fn fingerprints_distinguish_kind() {
        // A warning and a secret at the same file:line must not collide.
        let w = sample_warning("collide", "src/x.ts", 7);
        let s = sample_secret("src/x.ts", 7, "collide");
        let b = build_baseline(&[w], &[s]);
        assert_eq!(b.total(), 2, "kind prefix must keep fingerprints distinct");
    }
}
