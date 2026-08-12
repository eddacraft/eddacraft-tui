//! Capsule verification (GITGOV / ADR-074): recompute digests and check package integrity.

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::canonical::sha256_hex;
use crate::collect::{CommitsDocument, collect_commits};
use crate::collect_digests::{ToolIdentity, collect_digests};
use crate::manifest::CapsuleManifest;
use crate::verification::{CapsuleVerification, CheckResult, Verdict};

/// `manifest-digests` check name.
const CHECK_MANIFEST: &str = "manifest-digests";
/// `witness-chain` check name.
const CHECK_WITNESS: &str = "witness-chain";
/// `digests-vs-repo` check name.
const CHECK_DIGESTS: &str = "digests-vs-repo";
/// `exceptions` check name.
const CHECK_EXCEPTIONS: &str = "exceptions";

/// Verify the capsule at `capsule_dir` against `repo_root` at the
/// current time. See [`verify_capsule_at`].
#[must_use]
pub fn verify_capsule(capsule_dir: &Path, repo_root: &Path) -> CapsuleVerification {
    verify_capsule_at(capsule_dir, repo_root, Utc::now())
}

/// Verify the capsule at `capsule_dir` against `repo_root`, evaluating
/// time-sensitive checks (exception expiry) at `now`.
///
/// Returns the [`CapsuleVerification`] document; the caller writes it
/// back to `verification.json` and maps [`Verdict::exit_code`].
#[must_use]
pub fn verify_capsule_at(
    capsule_dir: &Path,
    repo_root: &Path,
    now: DateTime<Utc>,
) -> CapsuleVerification {
    // The manifest is the digest root. If it cannot be read or parsed the
    // capsule is unverifiable as a whole — a single `error` check, never
    // an overclaimed pass/degraded (ADR-074: do not overclaim).
    let manifest = match read_manifest(capsule_dir) {
        Ok(manifest) => manifest,
        Err(detail) => {
            return CapsuleVerification::from_checks(vec![CheckResult {
                name: CHECK_MANIFEST.to_string(),
                verdict: Verdict::Error,
                detail: Some(detail),
            }]);
        }
    };

    CapsuleVerification::from_checks(vec![
        check_manifest_digests(capsule_dir, &manifest),
        check_witness_chain(capsule_dir),
        check_digests_vs_repo(capsule_dir, repo_root),
        check_exceptions(capsule_dir, repo_root, now),
    ])
}

/// Read + schema-gate `manifest.json`. `Err` carries a human detail for
/// the `error` verdict.
fn read_manifest(capsule_dir: &Path) -> Result<CapsuleManifest, String> {
    let bytes =
        read_capsule_regular_file(&capsule_dir.join("manifest.json")).map_err(|e| match e {
            CapsuleFileError::NotFound => "cannot read manifest.json: not found".to_string(),
            CapsuleFileError::NonRegular { detail } | CapsuleFileError::Io { detail } => detail,
        })?;
    CapsuleManifest::from_json_bytes(&bytes).map_err(|e| format!("invalid manifest.json: {e}"))
}

/// Failure modes for reading a capsule-resident evidence file without
/// following a final-component symlink.
#[derive(Debug)]
enum CapsuleFileError {
    /// Path is absent (missing evidence).
    NotFound,
    /// Symlink or other non-regular entry — package-boundary violation.
    NonRegular { detail: String },
    /// Tool/stat/read failure.
    Io { detail: String },
}

/// Read a capsule-resident regular file without following a final-component
/// symlink. Capsule evidence must be package-local; a symlinked basename
/// would let mutable external state stand in for recorded digests
/// (basename validation alone is not enough — `std::fs::read` follows).
///
/// On Unix the open uses `O_NOFOLLOW` so a concurrent swap of a regular
/// file for a symlink cannot race between a metadata check and the
/// read. On non-Unix platforms we fall back to `symlink_metadata` then
/// `read` (best-effort; same class as other capsule path guards).
fn read_capsule_regular_file(path: &Path) -> Result<Vec<u8>, CapsuleFileError> {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );

    #[cfg(unix)]
    {
        use std::io::Read;
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CapsuleFileError::NotFound);
            }
            // `O_NOFOLLOW` on a final-component symlink surfaces as ELOOP.
            Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
                return Err(CapsuleFileError::NonRegular {
                    detail: format!("symlink refused: {name}"),
                });
            }
            Err(e) => {
                return Err(CapsuleFileError::Io {
                    detail: format!("cannot open {name}: {e}"),
                });
            }
        };
        // Metadata from the open fd (not a second path lookup).
        let meta = file.metadata().map_err(|e| CapsuleFileError::Io {
            detail: format!("cannot stat {name}: {e}"),
        })?;
        if !meta.is_file() {
            return Err(CapsuleFileError::NonRegular {
                detail: format!("non-regular capsule file: {name}"),
            });
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CapsuleFileError::NotFound
            } else {
                CapsuleFileError::Io {
                    detail: format!("cannot read {name}: {e}"),
                }
            }
        })?;
        Ok(bytes)
    }

    #[cfg(not(unix))]
    {
        let meta = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CapsuleFileError::NotFound);
            }
            Err(e) => {
                return Err(CapsuleFileError::Io {
                    detail: format!("cannot stat {name}: {e}"),
                });
            }
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            return Err(CapsuleFileError::NonRegular {
                detail: format!("symlink refused: {name}"),
            });
        }
        if !meta.is_file() {
            return Err(CapsuleFileError::NonRegular {
                detail: format!("non-regular capsule file: {name}"),
            });
        }
        std::fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CapsuleFileError::NotFound
            } else {
                CapsuleFileError::Io {
                    detail: format!("cannot read {name}: {e}"),
                }
            }
        })
    }
}

/// Build a single [`CheckResult`] from a worst-of verdict + joined
/// detail. Consumes `details`, joining with `; ` (`None` when empty).
fn result(name: &str, verdict: Verdict, details: Vec<String>) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        verdict,
        detail: details.into_iter().reduce(|acc, d| format!("{acc}; {d}")),
    }
}

/// `manifest-digests`: every recorded file present and matching its
/// digest; required files all listed. Tamper → `block`, absence →
/// `degraded`.
fn check_manifest_digests(capsule_dir: &Path, manifest: &CapsuleManifest) -> CheckResult {
    let mut verdict = Verdict::Pass;
    let mut details = Vec::new();

    let missing_required = manifest.missing_required();
    if !missing_required.is_empty() {
        verdict = verdict.worst(Verdict::Degraded);
        details.push(format!(
            "manifest omits required files: {missing_required:?}"
        ));
    }

    for (name, recorded) in &manifest.files {
        // `manifest.json` is untrusted input: a hostile manifest could
        // name `../secret` or an absolute path and steer the read outside
        // the capsule. Reject anything that is not a plain basename
        // before any `join` (council/copilot: path traversal).
        if !is_capsule_basename(name) {
            verdict = verdict.worst(Verdict::Block);
            details.push(format!("unsafe manifest path: {name}"));
            continue;
        }
        match read_capsule_regular_file(&capsule_dir.join(name)) {
            Ok(bytes) => {
                if &sha256_hex(&bytes) != recorded {
                    verdict = verdict.worst(Verdict::Block);
                    details.push(format!("digest mismatch: {name}"));
                }
            }
            // Recorded but absent: the evidence is gone. Missing
            // evidence degrades, it does not pass (ADR-072 §4).
            Err(CapsuleFileError::NotFound) => {
                verdict = verdict.worst(Verdict::Degraded);
                details.push(format!("recorded file missing: {name}"));
            }
            // Symlink / non-regular: package-boundary violation → block
            // (same class as path traversal), not a soft miss.
            Err(CapsuleFileError::NonRegular { detail }) => {
                verdict = verdict.worst(Verdict::Block);
                details.push(detail);
            }
            Err(CapsuleFileError::Io { detail }) => {
                verdict = verdict.worst(Verdict::Error);
                details.push(detail);
            }
        }
    }

    // Files on disk neither recorded in the manifest nor the manifest
    // itself are foreign content — a finding (format.rs promises this).
    // A directory-listing failure is a tool failure, not "no extras"
    // (copilot): surface it as `error`, never silently skip.
    match std::fs::read_dir(capsule_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name != "manifest.json"
                    && !manifest.files.contains_key(name.as_ref())
                    && entry.file_type().is_ok_and(|t| t.is_file())
                {
                    verdict = verdict.worst(Verdict::Warn);
                    details.push(format!("unexpected file not in manifest: {name}"));
                }
            }
        }
        Err(e) => {
            verdict = verdict.worst(Verdict::Error);
            details.push(format!("cannot list capsule directory: {e}"));
        }
    }

    if verdict == Verdict::Pass {
        details.push(format!("{} files verified", manifest.files.len()));
    }
    result(CHECK_MANIFEST, verdict, details)
}

/// `witness-chain`: reuse `verify_chain_dag` over `witness.ndjson`.
fn check_witness_chain(capsule_dir: &Path) -> CheckResult {
    let path = capsule_dir.join("witness.ndjson");
    // Read via the O_NOFOLLOW-backed helper so a symlinked witness file
    // is refused before any digest/semantic consumption. Stage the
    // verified bytes to a private temp file for `verify_chain_dag`
    // (which only accepts paths and would otherwise re-open/follow).
    let bytes = match read_capsule_regular_file(&path) {
        Ok(bytes) => bytes,
        Err(CapsuleFileError::NotFound) => {
            return result(
                CHECK_WITNESS,
                Verdict::Degraded,
                vec!["witness.ndjson absent".to_string()],
            );
        }
        Err(CapsuleFileError::NonRegular { detail }) => {
            return result(CHECK_WITNESS, Verdict::Block, vec![detail]);
        }
        Err(CapsuleFileError::Io { detail }) => {
            return result(CHECK_WITNESS, Verdict::Error, vec![detail]);
        }
    };

    let stage = match stage_bytes_for_verify("witness", &bytes) {
        Ok(stage) => stage,
        Err(detail) => {
            return result(CHECK_WITNESS, Verdict::Error, vec![detail]);
        }
    };
    let staged_path = stage.path();
    match anvil_witness::verify_chain_dag(&[staged_path]) {
        Ok(report) if report.line_count == 0 => result(
            CHECK_WITNESS,
            Verdict::Degraded,
            vec!["empty witness chain (no lines)".to_string()],
        ),
        Ok(report) => result(
            CHECK_WITNESS,
            Verdict::Pass,
            vec![format!(
                "{} lines, {} merge(s)",
                report.line_count, report.merge_count
            )],
        ),
        // An I/O failure reading the chain is a tool failure, not a
        // broken chain — `error`, never an overclaimed `block` (copilot).
        Err(e @ anvil_witness::VerifyError::Io { .. }) => result(
            CHECK_WITNESS,
            Verdict::Error,
            vec![format!("cannot read witness chain: {e}")],
        ),
        Err(e) => result(
            CHECK_WITNESS,
            Verdict::Block,
            vec![format!("witness chain broken: {e}")],
        ),
    }
}

/// Write `bytes` to a private temp file that is removed on drop, so a
/// path-only verifier cannot be pointed at a symlinked capsule leaf.
fn stage_bytes_for_verify(label: &str, bytes: &[u8]) -> Result<StagedVerifyFile, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut path = std::env::temp_dir();
    path.push(format!(
        "anvil-capsule-{label}-{}-{nanos}.tmp",
        std::process::id()
    ));
    std::fs::write(&path, bytes).map_err(|e| format!("cannot stage {label} for verify: {e}"))?;
    Ok(StagedVerifyFile { path })
}

/// Temp path that is unlinked on drop.
struct StagedVerifyFile {
    path: std::path::PathBuf,
}

impl StagedVerifyFile {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StagedVerifyFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// A manifest file key must be a single plain path component — no
/// separators, `..`, root, or drive prefix — so an untrusted manifest
/// cannot steer a read outside the capsule directory.
fn is_capsule_basename(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

/// `digests-vs-repo`: re-collect commits + policy/rules/baseline digests
/// from `repo_root` and compare canonical bytes to the capsule's files. A
/// divergence is `degraded` (stale / different repo), inability to
/// re-collect is `degraded`.
fn check_digests_vs_repo(capsule_dir: &Path, repo_root: &Path) -> CheckResult {
    // The capsule's own commits.json gives the resolved range to
    // re-collect against; its rules.json gives the producing identity so
    // the digest pipeline matches by construction.
    let commits = match read_capsule_commits(capsule_dir) {
        Ok(commits) => commits,
        Err(detail) => return result(CHECK_DIGESTS, Verdict::Degraded, vec![detail]),
    };
    let identity = match read_capsule_identity(capsule_dir) {
        Ok(identity) => identity,
        Err(detail) => return result(CHECK_DIGESTS, Verdict::Degraded, vec![detail]),
    };

    let mut verdict = Verdict::Pass;
    let mut details = Vec::new();

    match collect_commits(repo_root, &commits.base, &commits.head) {
        Ok(recollected) => {
            if !bytes_match(
                capsule_dir,
                "commits.json",
                &recollected.to_canonical_bytes(),
            ) {
                verdict = verdict.worst(Verdict::Degraded);
                details.push("commits.json no longer matches the repo".to_string());
            }
        }
        Err(e) => {
            verdict = verdict.worst(Verdict::Degraded);
            details.push(format!("cannot re-collect commit range: {e}"));
        }
    }

    match collect_digests(repo_root, &identity) {
        Ok(recollected) => {
            for (name, bytes) in [
                ("policy.json", recollected.policy.to_canonical_bytes()),
                ("rules.json", recollected.rules.to_canonical_bytes()),
                ("baseline.json", recollected.baseline.to_canonical_bytes()),
            ] {
                if !bytes_match(capsule_dir, name, &bytes) {
                    verdict = verdict.worst(Verdict::Degraded);
                    details.push(format!("{name} no longer matches the repo"));
                }
            }
        }
        Err(e) => {
            verdict = verdict.worst(Verdict::Degraded);
            details.push(format!("cannot re-collect digests: {e}"));
        }
    }

    if verdict == Verdict::Pass {
        details.push("commits + policy/rules/baseline match the repo".to_string());
    }
    result(CHECK_DIGESTS, verdict, details)
}

/// `exceptions`: verify each applied exception via the EXCEPT-005
/// surface. Expired/revoked/invalid-scope → `block`; unattributed →
/// `degraded`; empty → `pass`.
fn check_exceptions(capsule_dir: &Path, repo_root: &Path, now: DateTime<Utc>) -> CheckResult {
    use anvil_policy::exceptions::{
        ExceptionStore, ExceptionVerdict, PolicyException, verify_exception_at,
    };

    let bytes = match read_capsule_regular_file(&capsule_dir.join("exceptions.json")) {
        Ok(bytes) => bytes,
        Err(CapsuleFileError::NotFound) => {
            return result(
                CHECK_EXCEPTIONS,
                Verdict::Degraded,
                vec!["exceptions.json absent".to_string()],
            );
        }
        Err(CapsuleFileError::NonRegular { detail }) => {
            return result(CHECK_EXCEPTIONS, Verdict::Block, vec![detail]);
        }
        Err(CapsuleFileError::Io { detail }) => {
            return result(CHECK_EXCEPTIONS, Verdict::Error, vec![detail]);
        }
    };

    // Present-but-unparseable evidence is a tool-can't-interpret failure,
    // not "stale/missing" — `error`, so a targeted corruption cannot
    // downgrade an otherwise-`block` exception set to `degraded` (council).
    let exceptions: Vec<PolicyException> = match serde_json::from_slice(&bytes) {
        Ok(exceptions) => exceptions,
        Err(e) => {
            return result(
                CHECK_EXCEPTIONS,
                Verdict::Error,
                vec![format!("unparseable exceptions.json: {e}")],
            );
        }
    };

    if exceptions.is_empty() {
        return result(
            CHECK_EXCEPTIONS,
            Verdict::Pass,
            vec!["no applied exceptions".to_string()],
        );
    }

    // The capsule snapshot is frozen at create time, so revocation
    // that happened *after* create is invisible to it. Re-consult the
    // live exception store — tracked, or the legacy fallback for
    // unmigrated repos; absent-store loads as empty — with the same
    // re-collection discipline as `check_digests_vs_repo`, so a
    // since-revoked grant blocks and a grant absent from the live
    // store degrades (2026-07-04 council, EXCEPT-009).
    let live = ExceptionStore::load(repo_root);

    let mut verdict = Verdict::Pass;
    let mut details = Vec::new();
    if let Err(e) = &live {
        verdict = verdict.worst(Verdict::Degraded);
        details.push(format!("exception store unreadable: {e}"));
    }
    for ex in &exceptions {
        let mut classify =
            |candidate_verdict: ExceptionVerdict, origin: &str| match candidate_verdict {
                ExceptionVerdict::Active => {}
                ExceptionVerdict::Unattributed => {
                    verdict = verdict.worst(Verdict::Degraded);
                    details.push(format!("unattributed exception {}{origin}", ex.id));
                }
                other => {
                    // An applied exception that is revoked/expired/invalid is
                    // a relied-upon deviation that no longer holds → block.
                    verdict = verdict.worst(Verdict::Block);
                    details.push(format!("{} exception {}{origin}", other.as_str(), ex.id));
                }
            };
        classify(verify_exception_at(ex, now), "");
        if let Ok(store) = &live {
            if let Some(live_record) = store.exceptions.iter().find(|l| l.id == ex.id) {
                classify(verify_exception_at(live_record, now), " (live store)");
            } else {
                verdict = verdict.worst(Verdict::Degraded);
                details.push(format!(
                    "exception {} is absent from the live exception store (revocation is \
                     a soft delete, so a record should persist; absence means the store \
                     was rewritten, removed, or never present in this checkout)",
                    ex.id,
                ));
            }
        }
    }
    if verdict == Verdict::Pass {
        details.push(format!("{} applied exception(s) valid", exceptions.len()));
    }
    result(CHECK_EXCEPTIONS, verdict, details)
}

/// Read + schema-gate the capsule's `commits.json`.
fn read_capsule_commits(capsule_dir: &Path) -> Result<CommitsDocument, String> {
    let bytes =
        read_capsule_regular_file(&capsule_dir.join("commits.json")).map_err(|e| match e {
            CapsuleFileError::NotFound => "cannot read commits.json: not found".to_string(),
            CapsuleFileError::NonRegular { detail } | CapsuleFileError::Io { detail } => detail,
        })?;
    CommitsDocument::from_json_bytes(&bytes).map_err(|e| format!("invalid commits.json: {e}"))
}

/// Build the producing [`ToolIdentity`] from the capsule's `rules.json`
/// so the re-collected digest pipeline matches the recorded one.
fn read_capsule_identity(capsule_dir: &Path) -> Result<ToolIdentity, String> {
    use crate::collect_digests::RulesDigest;
    let bytes =
        read_capsule_regular_file(&capsule_dir.join("rules.json")).map_err(|e| match e {
            CapsuleFileError::NotFound => "cannot read rules.json: not found".to_string(),
            CapsuleFileError::NonRegular { detail } | CapsuleFileError::Io { detail } => detail,
        })?;
    let rules =
        RulesDigest::from_json_bytes(&bytes).map_err(|e| format!("invalid rules.json: {e}"))?;
    Ok(ToolIdentity {
        anvil_version: rules.anvil_version,
        opa_runtime_version: rules.opa_runtime_version,
        rules: rules.rules,
    })
}

/// Whether `capsule_dir/name`'s bytes equal `expected` (a
/// `Result<Vec<u8>, _>` from a `to_canonical_bytes` encoder). A read
/// failure, non-regular entry, or encode failure is treated as a non-match.
fn bytes_match(
    capsule_dir: &Path,
    name: &str,
    expected: &Result<Vec<u8>, crate::CapsuleError>,
) -> bool {
    let Ok(expected) = expected else {
        return false;
    };
    match read_capsule_regular_file(&capsule_dir.join(name)) {
        Ok(actual) => &actual == expected,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::process::Command;

    use anvil_policy::exceptions::{ExceptionRevocation, PolicyException};
    use anvil_witness::{GenesisAnchor, WitnessLine};

    use super::*;
    use crate::collect::collect_commits;
    use crate::collect_diagnostics::collect_diagnostics;
    use crate::collect_digests::collect_digests;
    use crate::collect_witness::collect_witness;
    use crate::format::{CapsuleContent, write_capsule};
    use crate::manifest::Producer;

    fn git(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    /// A scratch repo (config + policy + two commits) with a one-line
    /// witness chain attesting head, so a capsule built from it verifies
    /// `pass`. Returns (dir, base, head).
    fn scratch_repo() -> (tempfile::TempDir, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        git(root, &["init", "-q", "--template="]);
        for (k, v) in [
            ("user.email", "v@test.invalid"),
            ("user.name", "verify-test"),
            ("commit.gpgsign", "false"),
        ] {
            git(root, &["config", k, v]);
        }
        std::fs::write(root.join(".anvil.yml"), "checks:\n  enabled: true\n").unwrap();
        std::fs::create_dir_all(root.join("anvil")).unwrap();
        std::fs::write(
            root.join("anvil/policy.yml"),
            "branches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
        )
        .unwrap();
        std::fs::write(root.join("a.txt"), "one").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "base"]);
        let base = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        std::fs::write(root.join("b.txt"), "two").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "head"]);
        let head = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        // Seed a witness chain attesting head so witness-chain passes.
        let wdir = root.join("anvil/witness");
        std::fs::create_dir_all(&wdir).unwrap();
        let mut line = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-06-08T00:00:00Z",
            "pre-commit",
            None,
        );
        line.commit_sha = Some(head.clone());
        std::fs::write(wdir.join("active.ndjson"), line.to_ndjson_line().unwrap()).unwrap();

        (dir, base, head)
    }

    /// Build a real capsule for `base..head` of `repo` at `out`.
    fn build_capsule(repo: &Path, base: &str, head: &str, out: &Path) {
        let commits = collect_commits(repo, base, head).unwrap();
        let identity = ToolIdentity {
            anvil_version: "0.0.0-test".to_string(),
            opa_runtime_version: anvil_rules::OPA_RUNTIME_VERSION.to_string(),
            rules: Vec::new(),
        };
        let digests = collect_digests(repo, &identity).unwrap();
        let range: BTreeSet<String> = commits.commits.iter().map(|c| c.sha.clone()).collect();
        let witness = collect_witness(repo, &range).unwrap();
        let diagnostics = collect_diagnostics(&[]).unwrap();
        let exceptions = crate::collect_exceptions::collect_exceptions(repo).unwrap();
        let content = CapsuleContent {
            commits,
            digests,
            witness,
            diagnostics,
            exceptions,
            producer: Producer {
                anvil_version: "0.0.0-test".to_string(),
            },
        };
        write_capsule(out, &content).unwrap();
    }

    /// Overwrite a capsule file *and* re-record its digest in the
    /// manifest, so `manifest-digests` still passes — isolates a single
    /// check under test from incidental digest mismatch.
    fn rewrite_recorded(out: &Path, name: &str, bytes: &[u8]) {
        std::fs::write(out.join(name), bytes).unwrap();
        let mut manifest =
            CapsuleManifest::from_json_bytes(&std::fs::read(out.join("manifest.json")).unwrap())
                .unwrap();
        manifest.record_file(name, bytes);
        std::fs::write(
            out.join("manifest.json"),
            manifest.to_canonical_bytes().unwrap(),
        )
        .unwrap();
    }

    fn out_dir(parent: &tempfile::TempDir) -> std::path::PathBuf {
        parent.path().join("capsule")
    }

    #[test]
    fn verify_passes_on_a_fresh_intact_capsule() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        let v = verify_capsule(&out, dir.path());
        assert_eq!(v.verdict, Verdict::Pass, "checks: {:?}", v.checks);
        assert_eq!(v.verdict.exit_code(), 0);
    }

    #[test]
    fn verify_blocks_on_digest_tamper() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        // Tamper a file's bytes WITHOUT updating the manifest digest.
        std::fs::write(out.join("commits.json"), b"{\"tampered\":true}").unwrap();

        let v = verify_capsule(&out, dir.path());
        assert_eq!(v.verdict, Verdict::Block);
        assert_eq!(v.verdict.exit_code(), 1);
        let manifest_check = v.checks.iter().find(|c| c.name == CHECK_MANIFEST).unwrap();
        assert_eq!(manifest_check.verdict, Verdict::Block);
    }

    #[test]
    fn verify_degrades_on_missing_recorded_file() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        std::fs::remove_file(out.join("baseline.json")).unwrap();

        let v = verify_capsule(&out, dir.path());
        assert_eq!(v.verdict, Verdict::Degraded);
        assert_eq!(v.verdict.exit_code(), 2);
    }

    #[test]
    fn verify_degrades_on_empty_witness_chain() {
        // A repo with no witness tree → the capsule's witness.ndjson is
        // empty → witness-chain degrades, never passes.
        let (dir, base, head) = scratch_repo();
        std::fs::remove_dir_all(dir.path().join("anvil/witness")).unwrap();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        let v = verify_capsule(&out, dir.path());
        let witness = v.checks.iter().find(|c| c.name == CHECK_WITNESS).unwrap();
        assert_eq!(witness.verdict, Verdict::Degraded);
        assert_eq!(v.verdict, Verdict::Degraded);
    }

    #[test]
    fn verify_errors_when_manifest_is_unreadable() {
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        std::fs::create_dir_all(&out).unwrap();
        // No manifest.json at all.
        let v = verify_capsule(&out, stage.path());
        assert_eq!(v.verdict, Verdict::Error);
        assert_eq!(v.verdict.exit_code(), 3);
    }

    #[test]
    fn verify_degrades_when_repo_moved_on() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        // Change the policy file after capsule creation → the recorded
        // digest no longer matches the repo → degraded (stale), not block.
        std::fs::write(
            dir.path().join("anvil/policy.yml"),
            "branches:\n  - pattern: main\n    require: l4\n",
        )
        .unwrap();

        let v = verify_capsule(&out, dir.path());
        let digests = v.checks.iter().find(|c| c.name == CHECK_DIGESTS).unwrap();
        assert_eq!(digests.verdict, Verdict::Degraded);
        // manifest-digests still passes — the capsule's own files are intact.
        let manifest = v.checks.iter().find(|c| c.name == CHECK_MANIFEST).unwrap();
        assert_eq!(manifest.verdict, Verdict::Pass);
    }

    /// EXCEPT-009 end-to-end: a grant in the tracked store travels
    /// through the real collect pipeline into exceptions.json and the
    /// verifier re-verifies it — no hand-rewrite.
    #[test]
    fn exceptions_roundtrip_create_collects_active_grant_and_verify_passes() {
        let (dir, base, head) = scratch_repo();
        let now = Utc::now();
        let mut store = anvil_policy::exceptions::ExceptionStore::empty();
        store.add(applied_exception(now));
        let outcome = store.save(dir.path()).unwrap();
        assert!(matches!(
            outcome,
            anvil_policy::exceptions::WriteOutcome::Written
        ));

        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);
        let bytes = std::fs::read(out.join("exceptions.json")).unwrap();
        let recorded: Vec<PolicyException> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(recorded.len(), 1, "grant must be collected at create time");

        let v = verify_capsule_at(&out, dir.path(), now);
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(check.verdict, Verdict::Pass, "detail: {:?}", check.detail);
    }

    /// EXCEPT-009 council HIGH: a grant revoked in the tracked store
    /// AFTER capsule create must still block verify — the frozen
    /// snapshot alone would report it Active; the live-store recheck
    /// catches it.
    #[test]
    fn exceptions_since_revoked_grant_blocks_verify() {
        let (dir, base, head) = scratch_repo();
        let now = Utc::now();
        let grant = applied_exception(now);
        let grant_id = grant.id.clone();
        let mut store = anvil_policy::exceptions::ExceptionStore::empty();
        store.add(grant);
        let outcome = store.save(dir.path()).unwrap();
        assert!(matches!(
            outcome,
            anvil_policy::exceptions::WriteOutcome::Written
        ));

        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        // Revoke AFTER the capsule froze its snapshot.
        let write = anvil_policy::exceptions::ExceptionStore::update(dir.path(), |store| {
            if let Some(ex) = store.exceptions.iter_mut().find(|ex| ex.id == grant_id) {
                ex.revoked = Some(anvil_policy::exceptions::ExceptionRevocation {
                    revoked_at: Utc::now(),
                    revoked_by: "bob".to_string(),
                    reason: "withdrawn".to_string(),
                });
            }
        })
        .unwrap();
        assert!(matches!(
            write,
            anvil_policy::exceptions::WriteOutcome::Written
        ));

        let v = verify_capsule_at(&out, dir.path(), now);
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(
            check.verdict,
            Verdict::Block,
            "since-revoked grant must block: {:?}",
            check.detail
        );
    }

    /// EXCEPT-009 council: a snapshot grant missing from the tracked
    /// store degrades verify (revocation is a soft delete — a vanished
    /// record means the store was rewritten).
    #[test]
    fn exceptions_grant_missing_from_live_store_degrades_verify() {
        let (dir, base, head) = scratch_repo();
        let now = Utc::now();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);
        // Plant a snapshot grant the live (absent) store never had.
        let exceptions = vec![applied_exception(now)];
        rewrite_recorded(
            &out,
            "exceptions.json",
            &serde_json::to_vec(&exceptions).unwrap(),
        );
        let v = verify_capsule_at(&out, dir.path(), now);
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(check.verdict, Verdict::Degraded, "{:?}", check.detail);
    }

    /// EXCEPT-009 end-to-end: an unattributed grant collected at
    /// create time degrades verify through the real pipeline.
    #[test]
    fn exceptions_roundtrip_unattributed_grant_degrades_verify() {
        let (dir, base, head) = scratch_repo();
        let now = Utc::now();
        let mut grant = applied_exception(now);
        grant.owner = None;
        grant.created_by = None;
        let mut store = anvil_policy::exceptions::ExceptionStore::empty();
        store.add(grant);
        let outcome = store.save(dir.path()).unwrap();
        assert!(matches!(
            outcome,
            anvil_policy::exceptions::WriteOutcome::Written
        ));

        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);
        let v = verify_capsule_at(&out, dir.path(), now);
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(
            check.verdict,
            Verdict::Degraded,
            "detail: {:?}",
            check.detail
        );
    }

    fn applied_exception(now: chrono::DateTime<Utc>) -> PolicyException {
        PolicyException {
            schema_version: "anvil.exception.v1".to_string(),
            id: "exc_test".to_string(),
            policy_id: "AP-001".to_string(),
            file_pattern: "src/**".to_string(),
            finding_hash: None,
            reason: "legacy".to_string(),
            owner: Some("team".to_string()),
            created_by: Some("alice".to_string()),
            created_at: now,
            expires_at: None,
            revoked: None,
        }
    }

    #[test]
    fn verify_passes_with_a_valid_applied_exception() {
        let (dir, base, head) = scratch_repo();
        let now = Utc::now();
        // The live tracked store must carry the grant too: verify
        // re-consults it, and a snapshot-only grant degrades.
        let mut store = anvil_policy::exceptions::ExceptionStore::empty();
        store.add(applied_exception(now));
        let _ = store.save(dir.path()).unwrap();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        let exceptions = vec![applied_exception(now)];
        rewrite_recorded(
            &out,
            "exceptions.json",
            &serde_json::to_vec(&exceptions).unwrap(),
        );

        let v = verify_capsule_at(&out, dir.path(), now);
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(check.verdict, Verdict::Pass, "detail: {:?}", check.detail);
    }

    #[test]
    fn verify_blocks_on_a_revoked_applied_exception() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        let now = Utc::now();
        let mut ex = applied_exception(now);
        ex.revoked = Some(ExceptionRevocation {
            revoked_at: now,
            revoked_by: "bob".to_string(),
            reason: "withdrawn".to_string(),
        });
        rewrite_recorded(
            &out,
            "exceptions.json",
            &serde_json::to_vec(&vec![ex]).unwrap(),
        );

        let v = verify_capsule_at(&out, dir.path(), now);
        assert_eq!(v.verdict, Verdict::Block);
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(check.verdict, Verdict::Block);
    }

    #[test]
    fn verify_degrades_on_an_unattributed_applied_exception() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        let now = Utc::now();
        let mut ex = applied_exception(now);
        ex.owner = None;
        ex.created_by = None;
        rewrite_recorded(
            &out,
            "exceptions.json",
            &serde_json::to_vec(&vec![ex]).unwrap(),
        );

        let v = verify_capsule_at(&out, dir.path(), now);
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(check.verdict, Verdict::Degraded);
    }

    #[test]
    fn verify_is_deterministic() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);
        let now = Utc::now();
        assert_eq!(
            verify_capsule_at(&out, dir.path(), now),
            verify_capsule_at(&out, dir.path(), now)
        );
    }

    #[test]
    fn verify_warns_on_unexpected_file_not_in_manifest() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);
        std::fs::write(out.join("stowaway.txt"), b"foreign").unwrap();

        let v = verify_capsule(&out, dir.path());
        let manifest = v.checks.iter().find(|c| c.name == CHECK_MANIFEST).unwrap();
        assert_eq!(manifest.verdict, Verdict::Warn);
        assert_eq!(v.verdict.exit_code(), 0, "warn exits 0");
    }

    #[test]
    fn verify_blocks_on_path_traversal_in_manifest() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        // A hostile manifest naming a traversal path must not read out of
        // the capsule — it is rejected as `block`.
        let mut manifest =
            CapsuleManifest::from_json_bytes(&std::fs::read(out.join("manifest.json")).unwrap())
                .unwrap();
        manifest.record_file("../escape", b"x");
        std::fs::write(
            out.join("manifest.json"),
            manifest.to_canonical_bytes().unwrap(),
        )
        .unwrap();

        let v = verify_capsule(&out, dir.path());
        let check = v.checks.iter().find(|c| c.name == CHECK_MANIFEST).unwrap();
        assert_eq!(check.verdict, Verdict::Block);
    }

    /// Manifest basename checks alone are insufficient: `std::fs::read`
    /// follows a final-component symlink, so a hostile capsule can make
    /// a listed evidence basename point outside `capsule_dir`. Digest
    /// verification must refuse non-regular files before reading.
    #[cfg(unix)]
    #[test]
    fn verify_blocks_on_symlinked_manifest_evidence_file() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        // External target carries the same bytes as the recorded
        // commits.json digest so a follow-the-link verifier would pass.
        let original = std::fs::read(out.join("commits.json")).unwrap();
        let external = stage.path().join("outside-commits.json");
        std::fs::write(&external, &original).unwrap();

        std::fs::remove_file(out.join("commits.json")).unwrap();
        std::os::unix::fs::symlink(&external, out.join("commits.json")).unwrap();

        let v = verify_capsule(&out, dir.path());
        let manifest = v.checks.iter().find(|c| c.name == CHECK_MANIFEST).unwrap();
        assert_eq!(
            manifest.verdict,
            Verdict::Block,
            "symlinked evidence must block, not pass digest check: {:?}",
            manifest.detail
        );
        let detail = manifest.detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("symlink") || detail.contains("non-regular"),
            "detail should name the symlink refusal: {detail}"
        );
        // Semantic digests-vs-repo must not silently consume the
        // external target as a matching capsule file either.
        let digests = v.checks.iter().find(|c| c.name == CHECK_DIGESTS).unwrap();
        assert_ne!(
            digests.verdict,
            Verdict::Pass,
            "digests-vs-repo must not pass over a symlinked commits.json: {:?}",
            digests.detail
        );
        assert_eq!(v.verdict, Verdict::Block);
        assert_eq!(v.verdict.exit_code(), 1);
    }

    #[test]
    fn verify_errors_on_unparseable_exceptions() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);
        rewrite_recorded(&out, "exceptions.json", b"{not json");

        let v = verify_capsule_at(&out, dir.path(), Utc::now());
        let check = v
            .checks
            .iter()
            .find(|c| c.name == CHECK_EXCEPTIONS)
            .unwrap();
        assert_eq!(check.verdict, Verdict::Error);
        assert_eq!(v.verdict.exit_code(), 3);
    }

    /// GITGOV-012 witness-break: a witness chain that is present but
    /// internally broken (a second genesis anchor where a non-first
    /// line must reference a prior line's SHA-256) blocks. The digest
    /// is re-recorded so `manifest-digests` still passes — isolating
    /// the `verify_chain_dag` reuse as the check that catches the
    /// break, not the byte-digest check.
    #[test]
    fn verify_blocks_on_witness_chain_tamper() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        let genesis = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-06-08T00:00:00Z",
            "pre-commit",
            None,
        );
        // `to_ndjson_line` already appends the trailing newline, so two
        // copies form two NDJSON lines. Two genesis anchors: the second
        // line is non-first yet still claims genesis → a broken chain
        // (`StrayGenesis`).
        let line = genesis.to_ndjson_line().unwrap();
        let mut broken = line.clone();
        broken.extend_from_slice(&line);
        rewrite_recorded(&out, "witness.ndjson", &broken);

        let v = verify_capsule(&out, dir.path());
        let witness = v.checks.iter().find(|c| c.name == CHECK_WITNESS).unwrap();
        assert_eq!(
            witness.verdict,
            Verdict::Block,
            "detail: {:?}",
            witness.detail
        );
        assert_eq!(v.verdict, Verdict::Block);
        assert_eq!(v.verdict.exit_code(), 1);
        // The byte-digest check is NOT what caught it — the tampered
        // file's digest was re-recorded.
        let manifest = v.checks.iter().find(|c| c.name == CHECK_MANIFEST).unwrap();
        assert_eq!(manifest.verdict, Verdict::Pass);
    }

    /// GITGOV-012 missing-evidence: removing witness evidence degrades
    /// and never passes (ADR-074 closed-state honesty). A recorded
    /// file gone from disk is missing evidence (`degraded`), not a
    /// byte tamper (`block`).
    #[test]
    fn verify_degrades_on_missing_witness_after_tamper() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        std::fs::remove_file(out.join("witness.ndjson")).unwrap();

        let v = verify_capsule(&out, dir.path());
        let witness = v.checks.iter().find(|c| c.name == CHECK_WITNESS).unwrap();
        assert_eq!(witness.verdict, Verdict::Degraded);
        assert_ne!(v.verdict, Verdict::Pass, "missing evidence must never pass");
        assert_eq!(v.verdict, Verdict::Degraded);
        assert_eq!(v.verdict.exit_code(), 2);
    }

    /// GITGOV-012 witness-break, realistic edit: a second line whose
    /// `prev_line_hash` no longer matches the prior line's canonical
    /// hash — the canonical "someone edited the witness evidence"
    /// tamper — breaks the chain (`ChainBreak`) and blocks. The digest
    /// is re-recorded so `manifest-digests` still passes, isolating
    /// the `verify_chain_dag` reuse as the check that catches it.
    #[test]
    fn verify_blocks_on_witness_prev_hash_tamper() {
        let (dir, base, head) = scratch_repo();
        let stage = tempfile::tempdir().unwrap();
        let out = out_dir(&stage);
        build_capsule(dir.path(), &base, &head, &out);

        let line1 = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-06-08T00:00:00Z",
            "pre-commit",
            None,
        );
        let mut line2 = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-06-08T00:00:01Z",
            "pre-commit",
            None,
        );
        line2.seq = 2;
        // A wrong-but-plausible SHA-256 prev reference — not line1's
        // genuine canonical hash — so the DAG walk cannot anchor it.
        line2.prev_line_hash =
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string();

        // `to_ndjson_line` already terminates each line with a newline.
        let mut broken = line1.to_ndjson_line().unwrap();
        broken.extend_from_slice(&line2.to_ndjson_line().unwrap());
        rewrite_recorded(&out, "witness.ndjson", &broken);

        let v = verify_capsule(&out, dir.path());
        let witness = v.checks.iter().find(|c| c.name == CHECK_WITNESS).unwrap();
        assert_eq!(
            witness.verdict,
            Verdict::Block,
            "detail: {:?}",
            witness.detail
        );
        assert_eq!(v.verdict, Verdict::Block);
    }
}
