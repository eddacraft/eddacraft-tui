//! MLP2-049: per-state golden fixtures for the `ProtectionClaim`
//! render surface.
//!
//! One JSON fixture per worktree-state (10) + one per surface-state
//! (8) lives under `crates/anvil-cli/tests/fixtures/status_v1/`. The
//! test regenerates the canonical JSON from a synthesised
//! [`ProtectionClaim`] and asserts it matches the fixture byte-for-
//! byte. Intentional state changes regenerate the fixtures via
//! `ANVIL_UPDATE_FIXTURES=1 cargo test -p eddacraft-anvil --test
//! status_render`; without the env var, a drift is a test failure
//! with a clear diff message.
//!
//! This closes the MLP-009 HARD-GATE rendering surface: every
//! closed-set state from spec §14 has a pinned byte-for-byte
//! representation, so a future refactor that changes the JSON
//! shape is caught at CI time rather than during a customer
//! demo.

// CLAWP-027: fixture (re)generation under `ANVIL_UPDATE_FIXTURES=1` must be
// serialised. The default Rust test harness runs the test functions in this
// file concurrently; several of them write the same fixture paths while others
// read them. Every fixture file access (read AND write) below funnels through
// `fixture_io_lock()` so a concurrent reader can never observe a torn or
// missing file mid-write. The lock is process-wide and held only for the
// duration of a single file operation (or the directory-count block in
// `fixture_directory_layout_is_pinned`), so it serialises fixture I/O without
// serialising the tests' actual work. In verify mode there are no writers, so
// readers only ever contend briefly with each other on the short lock-hold,
// never blocking on a write.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use anvil_kernel_types::protection_claim::{
    ProtectionClaim, SurfaceClaim, SurfaceClaimState, WorktreeClaimState,
};

/// Process-wide guard serialising all fixture file I/O across the
/// concurrently-scheduled test functions in this file. Returns a guard that
/// must be held for the duration of the read or write it protects.
fn fixture_io_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        // A poisoned lock means a prior test panicked mid-fixture-op; the
        // data on disk may be torn, but recovering the guard lets the
        // remaining assertions surface their own (more actionable) failure
        // rather than masking it with a poison panic.
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Read a fixture file to a string while holding the serialisation lock, so
/// the read cannot interleave with an `ANVIL_UPDATE_FIXTURES=1` write to the
/// same path on another thread.
fn read_fixture(path: &Path) -> std::io::Result<String> {
    let _guard = fixture_io_lock();
    fs::read_to_string(path)
}

/// Write a fixture file (creating parent dirs) while holding the
/// serialisation lock, so a concurrent reader never observes a half-written
/// or momentarily-absent file. The dir-create + write happen atomically with
/// respect to other lock holders.
fn write_fixture(path: &Path, contents: &str) {
    let _guard = fixture_io_lock();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent dir");
    }
    fs::write(path, contents).expect("write fixture");
}

/// `cargo test` working directory points at the crate root, so
/// `tests/fixtures/...` resolves correctly from a relative path.
fn fixtures_root() -> PathBuf {
    PathBuf::from("tests/fixtures/status_v1")
}

/// Render `claim` as pretty-printed JSON with a trailing newline —
/// the canonical fixture shape. Pretty-print so a human reading the
/// fixture sees the structure; the trailing newline matches typical
/// editor + git-diff conventions.
fn render_fixture(claim: &ProtectionClaim) -> String {
    let mut out = serde_json::to_string_pretty(claim).expect("serialise");
    out.push('\n');
    out
}

/// Compare `actual` against `path` byte-for-byte. When
/// `ANVIL_UPDATE_FIXTURES=1` is set, write the actual bytes back to
/// disk so re-running the test refreshes the fixture in-place. The
/// fixture path is reported in error messages so failures point at
/// the file to inspect.
fn assert_or_update_fixture(path: &Path, actual: &str) {
    // CLAWP-052: gate update mode on the exact value `1`, not mere
    // presence. An accidental `ANVIL_UPDATE_FIXTURES=0` (or any stray
    // value) must NOT silently turn golden assertions into fixture
    // rewrites — that would let a regression overwrite the goldens with
    // wrong output instead of failing.
    if std::env::var("ANVIL_UPDATE_FIXTURES").is_ok_and(|v| v == "1") {
        // Serialised write: holds the lock across dir-create + write so a
        // concurrent reader (round-trip / layout tests) never sees a
        // half-written or momentarily-absent file.
        write_fixture(path, actual);
        return;
    }
    let expected = match read_fixture(path) {
        Ok(s) => s,
        Err(err) => panic!(
            "missing fixture {}: {err}.\n\nRegenerate with `ANVIL_UPDATE_FIXTURES=1 cargo test --test status_render`.",
            path.display(),
        ),
    };
    assert_eq!(
        actual,
        expected,
        "fixture drift at {}\n\nRegenerate intentional changes with `ANVIL_UPDATE_FIXTURES=1 cargo test --test status_render`.",
        path.display(),
    );
}

fn worktree_fixture_path(state: WorktreeClaimState) -> PathBuf {
    fixtures_root()
        .join("worktree")
        .join(format!("{}.json", state.as_str()))
}

fn surface_fixture_path(state: SurfaceClaimState) -> PathBuf {
    fixtures_root()
        .join("surface")
        .join(format!("{}.json", state.as_str()))
}

/// Synthesise the canonical fixture claim for a worktree state. The
/// surfaces array is empty so the fixture isolates the worktree-
/// state field — surface-state fixtures live in a separate set with
/// a pinned `Full` worktree state.
fn fixture_for_worktree(state: WorktreeClaimState) -> ProtectionClaim {
    ProtectionClaim::new(state, vec![])
}

/// Synthesise the canonical fixture claim for a surface state. The
/// worktree state is pinned at `Full` (the spec's "all layers
/// present" anchor) so the fixture isolates the per-surface field.
/// The identifier is a stable placeholder, not a real session id.
fn fixture_for_surface(state: SurfaceClaimState) -> ProtectionClaim {
    ProtectionClaim::new(
        WorktreeClaimState::Full,
        vec![SurfaceClaim {
            identifier: "surface-fixture".into(),
            state,
        }],
    )
}

/// Pin every worktree-state JSON output against the fixture. Adding
/// a variant to [`WorktreeClaimState`] fails the test because the
/// fixture file does not exist yet; running with
/// `ANVIL_UPDATE_FIXTURES=1` materialises the new file. The
/// per-state mapping is also exercised by the closed-set assertion
/// in [`anvil_kernel_types::protection_claim`]'s own tests.
#[test]
fn every_worktree_state_matches_pinned_fixture() {
    for state in WorktreeClaimState::all() {
        let claim = fixture_for_worktree(*state);
        let actual = render_fixture(&claim);
        let path = worktree_fixture_path(*state);
        assert_or_update_fixture(&path, &actual);
    }
}

/// Pin every surface-state JSON output against the fixture. Same
/// `ANVIL_UPDATE_FIXTURES=1` recipe applies for intentional changes.
#[test]
fn every_surface_state_matches_pinned_fixture() {
    for state in SurfaceClaimState::all() {
        let claim = fixture_for_surface(*state);
        let actual = render_fixture(&claim);
        let path = surface_fixture_path(*state);
        assert_or_update_fixture(&path, &actual);
    }
}

/// CLAWP-027 regression: prove the fixture I/O lock serialises concurrent
/// writers and readers on the same path so a reader never observes a torn or
/// missing file. We hammer a single temp path from many threads, each writing
/// the full canonical render through `write_fixture` (the exact primitive the
/// update-mode path uses) then reading it straight back through
/// `read_fixture`; every read must parse to a complete `ProtectionClaim` that
/// round-trips to the canonical render. Without `fixture_io_lock()` gating
/// both sides, a reader could catch a partially-written file and fail to
/// deserialise (or hit a missing-file error during the create-dir window).
///
/// This drives the lock primitives directly on a temp path and never touches
/// `ANVIL_UPDATE_FIXTURES` or the real fixtures, so it neither perturbs
/// sibling tests (env vars are process-wide) nor runs only in update mode.
#[test]
fn fixture_io_is_serialised_against_concurrent_readers_and_writers() {
    use std::thread;

    let dir = tempfile::tempdir().expect("tempdir");
    // A nested path so the create-dir step is exercised on the first write,
    // widening the window a missing-file race would have hit.
    let path = dir.path().join("nested").join("claim.json");
    let canonical = render_fixture(&fixture_for_worktree(WorktreeClaimState::Full));

    let result = thread::scope(|scope| {
        let handles: Vec<_> = (0..32)
            .map(|_| {
                let path = path.clone();
                let canonical = canonical.clone();
                scope.spawn(move || {
                    for _ in 0..16 {
                        write_fixture(&path, &canonical);
                        let raw = read_fixture(&path).expect("read back fixture");
                        // A torn read would either fail to parse or yield a
                        // value that does not round-trip to the canonical render.
                        let parsed: ProtectionClaim =
                            serde_json::from_str(&raw).expect("guarded read is never torn");
                        assert_eq!(render_fixture(&parsed), canonical);
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .map(std::thread::ScopedJoinHandle::join)
            .collect::<Result<Vec<_>, _>>()
    });

    result.expect("no worker thread panicked on a torn or missing read");
}

/// Pin the directory layout so a future contributor sees how the
/// fixtures are organised without having to read the test code. The
/// fixture-set is fixed: 10 worktree variants + 8 surface variants
/// = 18 files. If this count drifts without an MLP-009 spec
/// revision, the contract has changed.
#[test]
fn fixture_directory_layout_is_pinned() {
    let worktree_dir = fixtures_root().join("worktree");
    let surface_dir = fixtures_root().join("surface");

    // Generate the fixtures first under update mode so a fresh
    // checkout populates the dirs deterministically; otherwise the
    // test would fail before the producers above have a chance to
    // write the files.
    if std::env::var("ANVIL_UPDATE_FIXTURES").is_ok_and(|v| v == "1") {
        for state in WorktreeClaimState::all() {
            assert_or_update_fixture(
                &worktree_fixture_path(*state),
                &render_fixture(&fixture_for_worktree(*state)),
            );
        }
        for state in SurfaceClaimState::all() {
            assert_or_update_fixture(
                &surface_fixture_path(*state),
                &render_fixture(&fixture_for_surface(*state)),
            );
        }
    }

    // Hold the serialisation lock across BOTH directory counts: the listing
    // must not interleave with a concurrent fixture write on another test
    // thread (sibling tests write the same dirs under ANVIL_UPDATE_FIXTURES=1),
    // which could otherwise yield a partial count. `assert_or_update_fixture`
    // above self-locks per write, so the lock is free here; we must not hold it
    // across those calls (the mutex is non-reentrant).
    let _guard = fixture_io_lock();
    let worktree_count = fs::read_dir(&worktree_dir)
        .expect("worktree fixture dir present")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .count();
    assert_eq!(
        worktree_count, 10,
        "spec §14.2 names ten worktree-state fixtures; got {worktree_count}",
    );
    let surface_count = fs::read_dir(&surface_dir)
        .expect("surface fixture dir present")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .count();
    assert_eq!(
        surface_count, 8,
        "spec §14.1 names eight surface-state fixtures; got {surface_count}",
    );
}

/// Round-trip pin: deserialising a fixture back into
/// [`ProtectionClaim`] yields the same in-memory value as the
/// renderer started from. Closes the wire-shape contract from both
/// directions (encode + decode).
#[test]
fn every_fixture_round_trips_through_protection_claim() {
    for state in WorktreeClaimState::all() {
        let path = worktree_fixture_path(*state);
        let raw = read_fixture(&path).unwrap_or_else(|err| {
            panic!(
                "fixture missing at {}: {err}; run `ANVIL_UPDATE_FIXTURES=1 cargo test --test status_render` first",
                path.display(),
            )
        });
        let parsed: ProtectionClaim = serde_json::from_str(&raw).unwrap_or_else(|err| {
            panic!("fixture at {} failed to deserialise: {err}", path.display())
        });
        assert_eq!(parsed.worktree_state, *state);
    }
    for state in SurfaceClaimState::all() {
        let path = surface_fixture_path(*state);
        let raw = read_fixture(&path).unwrap_or_else(|err| {
            panic!(
                "fixture missing at {}: {err}; run `ANVIL_UPDATE_FIXTURES=1 cargo test --test status_render` first",
                path.display(),
            )
        });
        let parsed: ProtectionClaim = serde_json::from_str(&raw).unwrap_or_else(|err| {
            panic!("fixture at {} failed to deserialise: {err}", path.display())
        });
        assert_eq!(parsed.surfaces.len(), 1);
        assert_eq!(parsed.surfaces[0].state, *state);
    }
}
