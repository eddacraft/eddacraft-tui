use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anvil_witness::{
    GenesisAnchor, RolloverPolicy, WitnessLine, WitnessWriter, WriterError, verify_chain_dag,
};
use thiserror::Error;

use crate::store::{Baseline, FormatError};

/// Repo-relative path of the baseline file. Always exactly this —
/// changing it is a breaking change for every downstream consumer
/// (gate, L4, audit).
pub const BASELINE_PATH: &str = "anvil/baseline.json";

#[derive(Debug, Error)]
pub enum BaselineIoError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("format: {0}")]
    Format(#[from] FormatError),
    #[error("`{path}` is a symlink; refusing to read/write baseline outside the repo")]
    SymlinkRefusal { path: PathBuf },
    /// MLP2-013: the witness chain primitive refused the genesis
    /// emission (symlinked witness root, lock contention, etc.). The
    /// baseline file itself is already written when this fires — the
    /// genesis emission happens after `save()` so the on-disk
    /// `anvil/baseline.json` is never blocked by a witness-side
    /// failure. Callers should still surface the error so an
    /// operator can re-run after fixing the underlying state.
    #[error("witness genesis emission failed: {0}")]
    Witness(#[from] WriterError),
}

/// Load `anvil/baseline.json` from `repo_root`. Returns `None` when
/// the file is absent.
///
/// Refuses to read through a symlink at any point in `anvil/` or
/// `anvil/baseline.json` to prevent a malicious worktree state from
/// redirecting the baseline read to an out-of-tree file. This
/// matches the TOCTOU-hardened pattern MLP-001 established for
/// `anvil/project-id`.
pub fn load(repo_root: &Path) -> Result<Option<Baseline>, BaselineIoError> {
    let parent = repo_root.join("anvil");
    refuse_if_symlink(&parent)?;
    let path = repo_root.join(BASELINE_PATH);
    refuse_if_symlink(&path)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let baseline = Baseline::from_bytes(&bytes)?;
    Ok(Some(baseline))
}

/// Save `baseline` to `anvil/baseline.json` under `repo_root`,
/// creating `anvil/` if needed.
///
/// TOCTOU-hardened: the symlink check fires before AND after
/// `create_dir_all` so a racing process cannot swap the directory
/// for a symlink between our pre-check and our write. The write
/// itself is atomic — we stage into a uniquely named exclusive
/// sibling and rename into place, which prevents readers from
/// observing a half-written file under crash or concurrent reload.
///
/// The staging file is **not** a fixed `.baseline.json.tmp` sibling.
/// A shared name lets two concurrent savers — two `anvil baseline`
/// invocations, or a `--refresh` racing a partial-scan resume — clash
/// on one staging path: the loser's rename fails with `NotFound`
/// after the winner moved the shared temp away, or both interleave
/// `write_all` into the same file and a corrupted mixture is renamed
/// into `anvil/baseline.json`. That is a silent integrity failure in
/// the file every downstream consumer (gate, L4, audit) treats as
/// truth. Each save now stages into its own `create_new` file.
pub fn save(repo_root: &Path, baseline: &Baseline) -> Result<(), BaselineIoError> {
    let parent = repo_root.join("anvil");
    refuse_if_symlink(&parent)?;
    fs::create_dir_all(&parent)?;
    refuse_if_symlink(&parent)?;

    let final_path = repo_root.join(BASELINE_PATH);
    refuse_if_symlink(&final_path)?;

    let bytes = baseline.to_canonical_bytes()?;
    let (mut staging, tmp_path) = open_exclusive_staging_file(&parent)?;
    if let Err(e) = staging.write_all(&bytes).and_then(|()| staging.sync_all()) {
        drop(staging);
        let _ = fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    // Close the staging handle before rename: Windows cannot rename a
    // file that still has an open *source* handle.
    drop(staging);
    if let Err(e) = (|| -> Result<(), BaselineIoError> {
        refuse_if_symlink(&final_path)?;
        atomic_replace(&tmp_path, &final_path)?;
        Ok(())
    })() {
        // Best-effort cleanup of the exclusive staging file on failure.
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    Ok(())
}

/// Create a uniquely named staging file inside `anvil/`.
///
/// Uses `OpenOptions::create_new` so the open is exclusive: on Unix
/// this maps to `O_CREAT|O_EXCL`, which refuses to follow a
/// pre-existing symlink and fails with `AlreadyExists` if the path is
/// occupied. That closes both the fixed-name concurrent-clobber race
/// and the check-then-create symlink window a shared
/// `.baseline.json.tmp` path allowed.
fn open_exclusive_staging_file(parent: &Path) -> Result<(File, PathBuf), BaselineIoError> {
    let pid = std::process::id();
    for attempt in 0u32..32 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        // Mix pid, wall-clock nanos, and attempt so concurrent callers
        // and same-nanosecond retries almost never collide; create_new
        // still serialises any true collision safely.
        let nonce =
            nanos ^ (u128::from(pid) << 64) ^ (u128::from(attempt) << 48) ^ u128::from(attempt);
        let tmp_path = parent.join(format!(".baseline.json.{pid}-{nonce}.tmp"));
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
    Err(BaselineIoError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "exhausted exclusive temporary baseline file name attempts",
    )))
}

/// MLP2-013: `validation_at` value recorded on the genesis line a
/// baseline save emits. Keeps the verb in lockstep with the
/// `WitnessLine` schema's documented vocabulary (`"pre-commit"`,
/// `"pre-push"`, `"bootstrap-recovery"`, ...). Exposed as a public
/// constant so the L4 lane (MLP2-031 / MLP2-046) can pin-match
/// against it without re-spelling the literal.
pub const BASELINE_VALIDATION_AT: &str = "baseline";

/// MLP2-013: save `baseline` AND emit a witness genesis line under the
/// same call.
///
/// Round-trips through:
///
/// 1. [`save`] — writes `anvil/baseline.json` (atomic temp+rename).
/// 2. [`WitnessWriter::append`] — emits the first chain line with
///    `anchor` recorded in `prev_line_hash` and `baseline.cutoff_commit`
///    threaded into the line body's `cutoff_commit` field
///    (`Some(...)` for `GENESIS-BASELINED`, `None` for
///    `GENESIS-FRESH`).
///
/// **Idempotent.** If the active witness chain already carries any
/// lines, this function leaves it alone and only re-writes
/// `baseline.json`. Re-running `anvil baseline` is therefore safe;
/// the chain doesn't grow a duplicate genesis on each invocation.
///
/// **Ordering note.** The baseline file is written **before** the
/// witness emission, so a witness-side failure (symlinked
/// `anvil/witness/`, etc.) cannot prevent the baseline record from
/// landing on disk. The witness error is surfaced via
/// [`BaselineIoError::Witness`] so the caller can still report it.
///
/// **`anchor` vs `cutoff_commit` pairing.** This function does NOT
/// cross-check `anchor` against `baseline.cutoff_commit` — that's
/// the caller's responsibility. The convention encoded in ADR-037
/// §D-2 is that [`GenesisAnchor::Baselined`] pairs with
/// `Some(cutoff)` and [`GenesisAnchor::Fresh`] pairs with `None`,
/// but the API surface stays permissive so a future "baselined
/// without an exact SHA" use case doesn't require an enum split.
pub fn save_with_genesis(
    repo_root: &Path,
    baseline: &Baseline,
    anchor: &GenesisAnchor,
) -> Result<(), BaselineIoError> {
    save(repo_root, baseline)?;

    let writer = WitnessWriter::open(repo_root, "active", RolloverPolicy::default())?;
    let active_path = writer.active_path();

    // Idempotency: if the chain already has lines, do not emit a
    // second genesis. The witness chain is append-only and a second
    // root would be rejected by the verifier (StrayGenesis on a
    // linear chain, or — once MLP-005 merge witnesses are present —
    // surfaced through the DAG walk's stray-genesis check). Errors
    // are treated as "non-empty but broken" — emitting another
    // genesis would obliterate evidence, so we skip in that case too
    // and let the caller's recovery flow (e.g. `anvil hook bootstrap`)
    // handle repair.
    //
    // Council MAJOR (wave 1I review) — uses `verify_chain_dag` so a
    // chain that contains merge witnesses is recognised as non-empty
    // rather than falling through to a second genesis emit. The
    // `Ok(_) | Err(_)` arm covers both DAG-broken and legacy-broken
    // states uniformly.
    if active_path.exists() {
        match verify_chain_dag(&[active_path.as_path()]) {
            Ok(dag) if dag.line_count == 0 => {
                // Empty file — fall through to emit genesis.
            }
            Ok(_) | Err(_) => return Ok(()),
        }
    }

    let genesis = WitnessLine::genesis(
        anchor,
        baseline.metadata.project_uuid.clone(),
        "active",
        baseline.metadata.created_at.clone(),
        BASELINE_VALIDATION_AT,
        baseline.cutoff_commit.clone(),
    );
    writer.append(&genesis)?;
    Ok(())
}

/// Atomically replace `dest` with `src`. POSIX `rename(2)` overwrites
/// silently; Windows `MoveFileExW` (which `std::fs::rename` calls)
/// returns `AlreadyExists` when `dest` exists, so we fall back to
/// remove-then-rename on that one error path. The window between the
/// remove and the rename is narrow and only matters on Windows; on
/// POSIX the first rename always wins.
fn atomic_replace(src: &Path, dest: &Path) -> Result<(), BaselineIoError> {
    match fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            // Refuse symlinks again before the remove so we don't
            // chase a swapped link out of the repo.
            refuse_if_symlink(dest)?;
            fs::remove_file(dest)?;
            fs::rename(src, dest)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Refuse if `path` is a symlink — including a *broken* symlink whose
/// target doesn't exist. `Path::exists()` returns false for a broken
/// symlink (it follows the link before checking), which would let an
/// attacker stage a symlink to a non-existent file as a "doesn't
/// exist" path and bypass our refusal. We use `symlink_metadata()`
/// which inspects the link itself, not its target.
fn refuse_if_symlink(path: &Path) -> Result<(), BaselineIoError> {
    match path.symlink_metadata() {
        Ok(meta) if meta.file_type().is_symlink() => Err(BaselineIoError::SymlinkRefusal {
            path: path.to_path_buf(),
        }),
        // Not a symlink → not refused. Either a regular file/dir or
        // something else (socket, etc.); the caller's subsequent
        // operations will surface a more specific error if so.
        Ok(_) => Ok(()),
        // ENOENT → path doesn't exist at all, including as a symlink.
        // That's fine; the caller will create it.
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute_fingerprint;
    use crate::finding::BaselineFinding;
    use crate::store::{Baseline, BaselineMetadata};
    use anvil_witness::{GenesisAnchor, WitnessLine, verify_chain_dag};

    fn metadata() -> BaselineMetadata {
        BaselineMetadata {
            created_at: "2026-05-13T00:00:00Z".to_string(),
            created_by_version: "0.7.0-beta".to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
        }
    }

    fn sample() -> Baseline {
        Baseline::new(
            metadata(),
            vec![BaselineFinding {
                rule_id: "anti-pattern:guardrail-suppression".to_string(),
                file_path: "src/lib.rs".to_string(),
                fingerprint: compute_fingerprint(
                    "anti-pattern:guardrail-suppression",
                    "// @ts-ignore",
                )
                .unwrap(),
            }],
        )
    }

    #[test]
    fn save_then_load_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let b = sample();
        save(tmp.path(), &b).unwrap();
        let loaded = load(tmp.path()).unwrap().expect("baseline exists");
        assert_eq!(loaded, b);
    }

    #[test]
    fn load_returns_none_when_file_absent() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn save_creates_anvil_directory_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!tmp.path().join("anvil").exists());
        save(tmp.path(), &sample()).unwrap();
        assert!(tmp.path().join("anvil").is_dir());
        assert!(tmp.path().join(BASELINE_PATH).is_file());
    }

    #[test]
    fn save_writes_canonical_trailing_newline() {
        let tmp = tempfile::tempdir().unwrap();
        save(tmp.path(), &sample()).unwrap();
        let bytes = fs::read(tmp.path().join(BASELINE_PATH)).unwrap();
        assert!(bytes.ends_with(b"\n"), "canonical bytes end in newline");
    }

    /// Names of leftover staging temps beside `anvil/baseline.json`.
    /// Covers both the legacy fixed sibling and the unique
    /// exclusive-staging pattern.
    fn staging_leftovers(repo_root: &Path) -> Vec<String> {
        fs::read_dir(repo_root.join("anvil"))
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| {
                n.starts_with(".baseline.json.")
                    && Path::new(n)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("tmp"))
            })
            .collect()
    }

    #[test]
    fn save_is_atomic_via_temp_then_rename() {
        let tmp = tempfile::tempdir().unwrap();
        save(tmp.path(), &sample()).unwrap();
        // After save, neither the legacy fixed staging name nor any
        // unique exclusive staging temp may linger.
        let legacy = tmp.path().join("anvil").join(".baseline.json.tmp");
        assert!(!legacy.exists(), "legacy temp file leaked: {legacy:?}");
        let leftovers = staging_leftovers(tmp.path());
        assert!(leftovers.is_empty(), "staging temps leaked: {leftovers:?}");
    }

    #[cfg(unix)]
    #[test]
    fn save_refuses_when_anvil_dir_is_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        symlink(other.path(), tmp.path().join("anvil")).unwrap();
        let err = save(tmp.path(), &sample()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn load_refuses_when_anvil_dir_is_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        // Make the symlink target a real anvil dir so load would
        // otherwise succeed; the refusal is policy-driven, not
        // missing-file-driven.
        fs::create_dir(other.path().join("anvil")).unwrap();
        symlink(other.path().join("anvil"), tmp.path().join("anvil")).unwrap();
        let err = load(tmp.path()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn save_refuses_when_baseline_file_is_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("anvil")).unwrap();
        let outside = tmp.path().join("outside.json");
        fs::write(&outside, "{}").unwrap();
        symlink(&outside, tmp.path().join(BASELINE_PATH)).unwrap();
        let err = save(tmp.path(), &sample()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }

    #[test]
    fn save_overwrites_existing_baseline_atomically() {
        let tmp = tempfile::tempdir().unwrap();
        save(tmp.path(), &sample()).unwrap();
        // Modify and resave.
        let mut b = sample();
        b.cutoff_commit = Some("abc123".to_string());
        save(tmp.path(), &b).unwrap();
        let loaded = load(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.cutoff_commit.as_deref(), Some("abc123"));
    }

    #[cfg(unix)]
    #[test]
    fn save_does_not_follow_legacy_fixed_temp_sibling_symlink() {
        // The historical fixed staging path (`.baseline.json.tmp`) is
        // both a concurrent-clobber and a check-then-create TOCTOU
        // hazard. Save must not use it at all: a pre-planted symlink
        // there must neither be followed nor block the write.
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("anvil")).unwrap();
        let outside = tmp.path().join("outside.json");
        let legacy = tmp.path().join("anvil").join(".baseline.json.tmp");
        symlink(&outside, &legacy).unwrap();

        save(tmp.path(), &sample()).unwrap();

        assert!(
            !outside.exists(),
            "legacy fixed temp symlink was written through"
        );
        let loaded = load(tmp.path()).unwrap().expect("baseline exists");
        assert_eq!(loaded, sample());
        assert!(
            legacy.symlink_metadata().unwrap().file_type().is_symlink(),
            "legacy plant should remain an untouched symlink",
        );
    }

    /// A shared staging name breaks concurrent saves two ways: the
    /// loser's `rename` hits `NotFound` because the winner already
    /// moved the shared temp away, or two writers interleave into the
    /// same file and a corrupted mixture is renamed into place. Both
    /// are probabilistic per round, so the guard races repeatedly —
    /// against the fixed-name implementation this trips within the
    /// first rounds rather than passing by luck.
    #[test]
    fn save_concurrent_calls_do_not_share_staging_path() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        const ROUNDS: usize = 25;
        let cutoffs = ["aaaaaaaa", "bbbbbbbb", "cccccccc", "dddddddd"];

        for round in 0..ROUNDS {
            let tmp = tempfile::tempdir().unwrap();
            let root: Arc<PathBuf> = Arc::new(tmp.path().to_path_buf());
            let barrier = Arc::new(Barrier::new(cutoffs.len()));
            let mut handles = Vec::new();
            for cutoff in cutoffs {
                let root = Arc::clone(&root);
                let barrier = Arc::clone(&barrier);
                handles.push(thread::spawn(move || {
                    let mut b = sample();
                    b.cutoff_commit = Some(cutoff.to_string());
                    barrier.wait();
                    save(root.as_ref(), &b)
                }));
            }
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            assert!(
                results.iter().all(std::result::Result::is_ok),
                "round {round}: concurrent save must not fail from shared staging clobber: \
                 {results:?}",
            );

            // The surviving file must be one writer's complete
            // baseline, not a byte-level mixture of several.
            let loaded = load(root.as_ref())
                .unwrap_or_else(|e| panic!("round {round}: baseline unreadable after race: {e}"))
                .expect("baseline exists");
            let cutoff = loaded.cutoff_commit.as_deref().unwrap();
            assert!(
                cutoffs.contains(&cutoff),
                "round {round}: final cutoff should be one of the concurrent writers, got {cutoff}",
            );
            let leftovers = staging_leftovers(root.as_ref());
            assert!(
                leftovers.is_empty(),
                "round {round}: staging temps leaked: {leftovers:?}",
            );
        }
    }

    #[test]
    fn open_exclusive_staging_file_returns_distinct_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path();
        let (f1, p1) = open_exclusive_staging_file(parent).unwrap();
        drop(f1);
        let (f2, p2) = open_exclusive_staging_file(parent).unwrap();
        drop(f2);
        assert_ne!(p1, p2, "exclusive staging paths must not collide");
        fs::write(&p1, b"one").unwrap();
        fs::write(&p2, b"two").unwrap();
        assert_eq!(fs::read(&p1).unwrap(), b"one");
        assert_eq!(fs::read(&p2).unwrap(), b"two");
        let _ = fs::remove_file(&p1);
        let _ = fs::remove_file(&p2);
    }

    #[cfg(unix)]
    #[test]
    fn refuse_if_symlink_catches_broken_symlinks() {
        // `Path::exists()` returns false for a broken symlink (it
        // follows the link). The earlier impl used `.exists()` and
        // would silently allow a broken-symlink baseline path. The
        // fixed impl uses `symlink_metadata` and refuses on the
        // link itself.
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("anvil")).unwrap();
        // Symlink to a target that doesn't exist.
        symlink(
            tmp.path().join("nonexistent-target"),
            tmp.path().join(BASELINE_PATH),
        )
        .unwrap();
        let err = save(tmp.path(), &sample()).unwrap_err();
        assert!(matches!(err, BaselineIoError::SymlinkRefusal { .. }));
    }

    /// MLP2-013 (round-trip AC): `save_with_genesis` with a
    /// `GENESIS-BASELINED` anchor emits the first witness line, the
    /// line's `prev_line_hash` is exactly the anchor literal, and
    /// the cutoff SHA threaded through `baseline.cutoff_commit`
    /// survives the canonical-bytes round-trip on the line body
    /// (ADR-037 §D-2).
    #[test]
    fn baseline_save_emits_genesis_baselined_with_cutoff() {
        let tmp = tempfile::tempdir().unwrap();
        let mut baseline = sample();
        baseline.cutoff_commit = Some("a3b2ea4ecafef00d".to_string());
        save_with_genesis(tmp.path(), &baseline, &GenesisAnchor::Baselined).unwrap();

        // baseline.json materialised.
        assert!(tmp.path().join(BASELINE_PATH).is_file());

        // Witness chain has one line and it's the anchor.
        let active = tmp.path().join("anvil/witness/active.ndjson");
        assert!(active.is_file(), "witness active file should exist");
        let contents = fs::read_to_string(&active).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 1, "expected exactly one genesis line");
        let parsed: WitnessLine = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed.prev_line_hash, "GENESIS-BASELINED");
        assert_eq!(parsed.cutoff_commit.as_deref(), Some("a3b2ea4ecafef00d"));
        assert_eq!(parsed.seq, 1);
        assert_eq!(parsed.validation_at, BASELINE_VALIDATION_AT);
    }

    /// MLP2-013: greenfield `anvil start`-style adoption emits
    /// `GENESIS-FRESH` with **no** `cutoff_commit` body field.
    /// Pinning the omitted-when-None shape keeps the canonical
    /// bytes for greenfield chains identical regardless of whether
    /// the baseline carries a (separate) cutoff or not.
    #[test]
    fn start_adoption_emits_genesis_fresh_without_cutoff() {
        let tmp = tempfile::tempdir().unwrap();
        let baseline = sample();
        // Belt-and-braces: greenfield never carries a cutoff.
        assert_eq!(baseline.cutoff_commit, None);
        save_with_genesis(tmp.path(), &baseline, &GenesisAnchor::Fresh).unwrap();

        let active = tmp.path().join("anvil/witness/active.ndjson");
        let contents = fs::read_to_string(&active).unwrap();
        assert!(
            !contents.contains("cutoff_commit"),
            "GENESIS-FRESH genesis must not carry cutoff_commit on the body, got: {contents}"
        );
        assert!(contents.contains("\"prev_line_hash\":\"GENESIS-FRESH\""));
    }

    /// MLP2-013 idempotency AC: running `save_with_genesis` on a
    /// chain that already carries lines (e.g. a hook lane fired
    /// first and seeded the chain, then an operator runs
    /// `anvil baseline --refresh`) must NOT emit a second genesis
    /// line. The chain stays exactly as it was.
    #[test]
    fn genesis_emission_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let baseline = sample();
        // First save seeds the chain.
        save_with_genesis(tmp.path(), &baseline, &GenesisAnchor::Fresh).unwrap();
        let active = tmp.path().join("anvil/witness/active.ndjson");
        let before = fs::read_to_string(&active).unwrap();

        // Second save with a different (Baselined) anchor must
        // leave the existing chain alone.
        let mut baseline2 = sample();
        baseline2.cutoff_commit = Some("a3b2ea4ecafef00d".to_string());
        save_with_genesis(tmp.path(), &baseline2, &GenesisAnchor::Baselined).unwrap();
        let after = fs::read_to_string(&active).unwrap();

        assert_eq!(
            before, after,
            "second save must not append a duplicate genesis line",
        );
        assert_eq!(
            after.lines().count(),
            1,
            "chain must still contain exactly one (the original) genesis line",
        );
    }

    /// MLP2-013: a chain seeded by `save_with_genesis` is a valid
    /// witness chain — `verify_chain` accepts the
    /// `GENESIS-BASELINED` root and subsequent pre-commit lines
    /// chain off it cleanly. This is the AC the L4 lane relies on:
    /// once the genesis line is on disk, an L4 verifier can walk
    /// the chain from genesis through every pre-commit witness
    /// without special-casing the anchor type.
    #[test]
    fn verifier_accepts_chain_starting_with_genesis_baselined() {
        use anvil_witness::{RolloverPolicy, WitnessWriter, compute_line_hash};
        let tmp = tempfile::tempdir().unwrap();
        let mut baseline = sample();
        baseline.cutoff_commit = Some("a3b2ea4ecafef00d".to_string());
        save_with_genesis(tmp.path(), &baseline, &GenesisAnchor::Baselined).unwrap();

        // Re-open the chain and append two pre-commit lines on top
        // of the genesis. Use the writer's append() directly to
        // simulate the hook-lane wiring (MLP-003) without pulling
        // anvil-cli into our dev-deps.
        let writer = WitnessWriter::open(tmp.path(), "active", RolloverPolicy::default()).unwrap();
        let active = writer.active_path();
        let contents = fs::read_to_string(&active).unwrap();
        let genesis: WitnessLine = serde_json::from_str(contents.lines().next().unwrap()).unwrap();
        let mut prev = compute_line_hash(&genesis.to_canonical_bytes().unwrap());
        for seq in 2..=3 {
            let line = WitnessLine {
                seq,
                scope: "active".to_string(),
                kind: "witness".to_string(),
                prev_line_hash: prev.clone(),
                project_uuid: baseline.metadata.project_uuid.clone(),
                commit_sha: Some(format!("c{seq}")),
                parent_commits: Vec::new(),
                prev_line_hashes: Vec::new(),
                agent_tag: None,
                rules_sha: None,
                cutoff_commit: None,
                ts: "2026-05-13T00:00:00Z".to_string(),
                validation_at: "pre-commit".to_string(),
            };
            writer.append(&line).unwrap();
            prev = compute_line_hash(&line.to_canonical_bytes().unwrap());
        }

        let report = verify_chain_dag(&[active.as_path()]).unwrap();
        assert_eq!(report.line_count, 3);
        assert_eq!(report.anchor, Some(GenesisAnchor::Baselined));
    }
}
