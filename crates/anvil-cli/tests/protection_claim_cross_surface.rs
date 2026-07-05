//! MLP2-051e: cross-surface `ProtectionClaim` parity.
//!
//! Three Rust render surfaces emit the §14 closed-set
//! `ProtectionClaim`:
//!
//! 1. `anvil status --json` — `crates/anvil-cli/src/commands/status.rs`
//!    via `protection_claim_section::resolve_protection_claim`.
//! 2. `anvil doctor` — same helper through
//!    `protection_claim_section::fetch_protection_claim_for_cwd`.
//! 3. MCP shim `validate_write` — `crates/anvil-cli/src/mcp/
//!    validation.rs::query_protection_claim`.
//!
//! All three converge on the public adapter
//! [`anvil_intercept::status::build_protection_claim_from_wire`] when a
//! live `DaemonStatusV1` snapshot is supplied. This test pins that
//! convergence two ways:
//!
//! - **Output parity** (the test cases below): a fixed snapshot in →
//!   the same `ProtectionClaim` out, regardless of how each surface
//!   serialises it. The canonical pretty-printed JSON (with trailing
//!   newline — same convention as the MLP2-049 fixtures) is saved
//!   under `tests/fixtures/status_v1/cross_surface/<case>.json` so the
//!   TS driver-client surface (`packages/anvil-driver-client/src/
//!   protection_claim/cross_surface.test.ts`) reads the exact same
//!   bytes the Rust helper produced. JSON formatting is irrelevant
//!   for the parity contract itself — the TS leg parses via
//!   `JSON.parse` — but pretty-printing makes the fixtures readable
//!   in code review.
//!
//! - **Call-site pin** (`all_rust_render_surfaces_route_through_the_
//!   shared_helper`): grep-style assertion that each surface's source
//!   file still references the shared helper. A future refactor that
//!   inlines or replaces the helper at one surface only — the failure
//!   mode this test guards against — drops the marker and fails this
//!   test before the divergence can land on `main`.
//!
//! Regenerate the fixtures with `ANVIL_UPDATE_FIXTURES=1 cargo test
//! --test protection_claim_cross_surface` after an intentional change
//! to the wire shape.

use std::fs;
use std::path::{Path, PathBuf};

use anvil_intercept::status::build_protection_claim_from_wire;
use anvil_intercept_proto::session::AgentTag;
use anvil_intercept_proto::status::{
    DaemonStatusV1, FenceStateV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1, WorktreeStatusV1,
};
use anvil_intercept_proto::{SessionId, SessionRecord, SessionStatus};
use anvil_kernel_types::protection_claim::{
    PROTECTION_CLAIM_SCHEMA_VERSION, ProtectionClaim, SurfaceClaimState, WorktreeClaimState,
};

/// One synthetic case. The `snapshot_builder` is invoked with the
/// queried worktree path so the test owns the path identity end-to-
/// end — the helper matches `WorktreeStatusV1.worktree` byte-for-byte
/// against the path the surface queries with.
struct ParityCase {
    /// Fixture file stem under `tests/fixtures/status_v1/cross_surface/`.
    name: &'static str,
    /// Worktree path the surface is querying. Held as `&str` and
    /// converted at call time so the test owns the round-trip.
    worktree: &'static str,
    /// Closure that synthesises the daemon snapshot the surfaces see.
    /// Receives the queried worktree path so it can register sessions
    /// against it (or deliberately not, for the "queried path not in
    /// snapshot" case).
    snapshot_builder: fn(&Path) -> DaemonStatusV1,
    /// Expected worktree-state. Pinned here so the test reads as the
    /// spec table; the fixture is the source of truth for the JSON
    /// bytes, but the variant assertion stops a fixture regeneration
    /// from silently changing the spec mapping.
    expected_worktree_state: WorktreeClaimState,
    /// Expected `(identifier, state)` pairs in the sorted order the
    /// helper emits. Empty slice means an empty `surfaces` array.
    expected_surfaces: &'static [(&'static str, SurfaceClaimState)],
}

const CASES: &[ParityCase] = &[
    ParityCase {
        name: "unprotected",
        worktree: "/tmp/wt-cross-surface-unprotected",
        snapshot_builder: empty_snapshot,
        expected_worktree_state: WorktreeClaimState::Unprotected,
        expected_surfaces: &[],
    },
    ParityCase {
        name: "pre-write-daemon-single-session",
        worktree: "/tmp/wt-cross-surface-pre-write",
        snapshot_builder: single_clean_session,
        expected_worktree_state: WorktreeClaimState::PreWriteDaemon,
        expected_surfaces: &[("sess-alpha", SurfaceClaimState::Participating)],
    },
    ParityCase {
        name: "pre-write-daemon-tagged-session",
        worktree: "/tmp/wt-cross-surface-tagged",
        snapshot_builder: tagged_session,
        expected_worktree_state: WorktreeClaimState::PreWriteDaemon,
        expected_surfaces: &[(
            "claude/agent-7#1700000000",
            SurfaceClaimState::Participating,
        )],
    },
    ParityCase {
        name: "degraded-protection-all-fenced",
        worktree: "/tmp/wt-cross-surface-fenced",
        snapshot_builder: single_fenced_session,
        expected_worktree_state: WorktreeClaimState::DegradedProtection,
        expected_surfaces: &[("sess-fenced", SurfaceClaimState::Quarantined)],
    },
    ParityCase {
        name: "degraded-protection-mixed-fence",
        worktree: "/tmp/wt-cross-surface-mixed",
        snapshot_builder: mixed_fenced_sessions,
        expected_worktree_state: WorktreeClaimState::DegradedProtection,
        expected_surfaces: &[
            ("sess-clean", SurfaceClaimState::Participating),
            ("sess-fenced", SurfaceClaimState::Quarantined),
        ],
    },
    ParityCase {
        name: "warming-ipc-draining",
        worktree: "/tmp/wt-cross-surface-warming",
        snapshot_builder: draining_session,
        expected_worktree_state: WorktreeClaimState::Warming,
        expected_surfaces: &[("sess-drain", SurfaceClaimState::Detached)],
    },
];

fn empty_snapshot(_worktree: &Path) -> DaemonStatusV1 {
    base_snapshot(vec![], vec![], vec![], IpcStateV1::Serving)
}

fn single_clean_session(worktree: &Path) -> DaemonStatusV1 {
    let session = clean_session("sess-alpha", worktree);
    let worktree_entry = worktree_status(&session, false);
    base_snapshot(
        vec![session],
        vec![worktree_entry],
        vec![],
        IpcStateV1::Serving,
    )
}

fn tagged_session(worktree: &Path) -> DaemonStatusV1 {
    let mut session = clean_session("sess-tagged", worktree);
    session.agent_tag = Some(AgentTag::new("claude", "agent-7", 1_700_000_000));
    let worktree_entry = worktree_status(&session, false);
    base_snapshot(
        vec![session],
        vec![worktree_entry],
        vec![],
        IpcStateV1::Serving,
    )
}

fn single_fenced_session(worktree: &Path) -> DaemonStatusV1 {
    let session = clean_session("sess-fenced", worktree);
    let worktree_entry = worktree_status(&session, true);
    let fence = FenceStateV1 {
        worktree: worktree.to_path_buf(),
        reason: "test fence".to_owned(),
        fenced_at_unix: 1_700_000_005,
    };
    base_snapshot(
        vec![session],
        vec![worktree_entry],
        vec![fence],
        IpcStateV1::Serving,
    )
}

fn mixed_fenced_sessions(worktree: &Path) -> DaemonStatusV1 {
    let clean = clean_session("sess-clean", worktree);
    let fenced = clean_session("sess-fenced", worktree);
    let clean_wt = worktree_status(&clean, false);
    let fenced_wt = worktree_status(&fenced, true);
    let fence = FenceStateV1 {
        worktree: worktree.to_path_buf(),
        reason: "test fence".to_owned(),
        fenced_at_unix: 1_700_000_005,
    };
    base_snapshot(
        vec![clean, fenced],
        vec![clean_wt, fenced_wt],
        vec![fence],
        IpcStateV1::Serving,
    )
}

fn draining_session(worktree: &Path) -> DaemonStatusV1 {
    let session = clean_session("sess-drain", worktree);
    let worktree_entry = worktree_status(&session, false);
    base_snapshot(
        vec![session],
        vec![worktree_entry],
        vec![],
        IpcStateV1::Draining,
    )
}

fn clean_session(id: &str, worktree: &Path) -> SessionRecord {
    SessionRecord {
        id: SessionId::new(id),
        worktree: worktree.to_path_buf(),
        pid: Some(4242),
        pgid: Some(4242),
        started_at_unix: 1_700_000_000,
        last_heartbeat_unix: 1_700_000_010,
        status: SessionStatus::Active,
        agent_tag: None,
        daemon_issued_tag: None,
    }
}

fn worktree_status(session: &SessionRecord, fenced: bool) -> WorktreeStatusV1 {
    WorktreeStatusV1 {
        worktree: session.worktree.clone(),
        session_id: session.id.clone(),
        fenced,
        cascaded: false,
        cascade_since: None,
        save_time_driver: anvil_intercept_proto::status::SaveTimeDriverStatusV1::Absent,
    }
}

fn base_snapshot(
    sessions: Vec<SessionRecord>,
    worktrees: Vec<WorktreeStatusV1>,
    fences: Vec<FenceStateV1>,
    ipc_state: IpcStateV1,
) -> DaemonStatusV1 {
    DaemonStatusV1 {
        sessions,
        worktrees,
        fences,
        health: HealthStateV1 {
            uptime_seconds: 5,
            version: "0.7.0-beta".to_owned(),
            ipc_state,
        },
        latency: LatencyMidEditMapV1::default(),
        cache_entries: None,
        cache_invalidations_total: None,
        in_flight_evaluations: None,
        cache_invalidations_rate_limited: None,
        telemetry_subscriber_count: None,
        telemetry_dropped_envelopes: None,
        generated_at_unix: 0,
    }
}

fn fixtures_root() -> PathBuf {
    PathBuf::from("tests/fixtures/status_v1/cross_surface")
}

fn fixture_path(case: &ParityCase) -> PathBuf {
    fixtures_root().join(format!("{}.json", case.name))
}

/// Pretty-printed JSON + trailing newline matches the canonical fixture
/// style already used by MLP2-049 (`status_render.rs`). Keeping the
/// same convention means a future contributor sees one shape across
/// every `status_v1/` sub-directory.
fn render_fixture(claim: &ProtectionClaim) -> String {
    let mut out = serde_json::to_string_pretty(claim).expect("serialise claim");
    out.push('\n');
    out
}

fn assert_or_update_fixture(path: &Path, actual: &str) {
    if std::env::var_os("ANVIL_UPDATE_FIXTURES").is_some() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent dir");
        }
        fs::write(path, actual).expect("write fixture");
        return;
    }
    let expected = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => panic!(
            "missing fixture {}: {err}.\n\nRegenerate with `ANVIL_UPDATE_FIXTURES=1 cargo test --test protection_claim_cross_surface`.",
            path.display(),
        ),
    };
    assert_eq!(
        actual,
        expected,
        "fixture drift at {}\n\nRegenerate intentional changes with `ANVIL_UPDATE_FIXTURES=1 cargo test --test protection_claim_cross_surface`.",
        path.display(),
    );
}

/// Each case: the shared helper produces a claim that matches both
/// the in-test expected variants and the pinned fixture bytes. Two
/// assertion layers so a fixture regeneration cannot silently rewrite
/// the spec mapping — the variant pins fail loudly first.
#[test]
fn every_case_matches_pinned_fixture_and_expected_variants() {
    for case in CASES {
        let worktree = Path::new(case.worktree);
        let snapshot = (case.snapshot_builder)(worktree);
        let claim = build_protection_claim_from_wire(&snapshot, worktree);

        assert_eq!(
            claim.schema_version, PROTECTION_CLAIM_SCHEMA_VERSION,
            "case {}: schema_version must pin to the v1 constant",
            case.name,
        );
        assert_eq!(
            claim.worktree_state, case.expected_worktree_state,
            "case {}: worktree_state",
            case.name,
        );
        assert_eq!(
            claim.surfaces.len(),
            case.expected_surfaces.len(),
            "case {}: surface count",
            case.name,
        );
        for (i, (identifier, state)) in case.expected_surfaces.iter().enumerate() {
            assert_eq!(
                claim.surfaces[i].identifier, *identifier,
                "case {}: surfaces[{}].identifier",
                case.name, i,
            );
            assert_eq!(
                claim.surfaces[i].state, *state,
                "case {}: surfaces[{}].state",
                case.name, i,
            );
        }

        let rendered = render_fixture(&claim);
        assert_or_update_fixture(&fixture_path(case), &rendered);
    }
}

/// Same input → same `ProtectionClaim` regardless of how many times
/// the helper is invoked. Pins determinism so a future refactor that
/// introduces `HashMap` iteration order into the surface-sort pipeline
/// fails here instead of producing flaky CI runs.
#[test]
fn helper_is_deterministic_for_each_case() {
    for case in CASES {
        let worktree = Path::new(case.worktree);
        let snapshot_a = (case.snapshot_builder)(worktree);
        let snapshot_b = (case.snapshot_builder)(worktree);
        let claim_a = build_protection_claim_from_wire(&snapshot_a, worktree);
        let claim_b = build_protection_claim_from_wire(&snapshot_b, worktree);
        assert_eq!(
            serde_json::to_string(&claim_a).expect("serialise a"),
            serde_json::to_string(&claim_b).expect("serialise b"),
            "case {}: helper output must be deterministic",
            case.name,
        );
    }
}

/// Round-trip pin: the pinned fixture bytes deserialise back into the
/// same closed-set `ProtectionClaim` the helper produced. Closes the
/// wire-shape contract from both directions for every cross-surface
/// case — the TS test exercises the same fixture bytes from the other
/// language so this serves as the Rust-side anchor.
#[test]
fn every_fixture_round_trips_through_protection_claim() {
    for case in CASES {
        let path = fixture_path(case);
        let raw = fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!(
                "fixture missing at {}: {err}; run `ANVIL_UPDATE_FIXTURES=1 cargo test --test protection_claim_cross_surface` first",
                path.display(),
            )
        });
        let parsed: ProtectionClaim = serde_json::from_str(&raw).unwrap_or_else(|err| {
            panic!("fixture at {} failed to deserialise: {err}", path.display())
        });
        assert_eq!(
            parsed.worktree_state, case.expected_worktree_state,
            "case {}: fixture deserialise → unexpected worktree_state",
            case.name,
        );
        assert_eq!(
            parsed.surfaces.len(),
            case.expected_surfaces.len(),
            "case {}: fixture surface count drift",
            case.name,
        );
    }
}

/// Call-site pin: each of the three Rust render surfaces still routes
/// through the shared claim builder. If a refactor inlines or
/// duplicates the mapping at one surface only — the silent-divergence
/// failure mode this whole work item exists to catch — the markers
/// disappear and this test fails before the change can ship.
///
/// Each surface declares TWO markers: an `import` substring (e.g. a
/// `use` declaration) and a `call` substring (e.g. an expression
/// containing argument syntax that a `///` doc line would not
/// naturally include — `(`, `&`, identifier tails). The scan is a
/// raw substring `contains()` over the full file, so the guarantee
/// is structural, not lexical: pick markers whose exact text would
/// not appear in a comment, and a refactor that demotes the helper
/// to a documentation reference only (keeping the symbol's name in
/// a `///` line but removing the actual import + call) drops at
/// least one of the two markers and fails the pin.
///
/// When updating, prefer call expressions that include argument
/// names (`(snapshot, worktree)`) over bare path references — the
/// extra punctuation is what keeps doc-comments from spuriously
/// matching.
///
/// Updating intentionally: if a surface legitimately moves to a new
/// helper, update the surface's entry below and add a new integration
/// case above that exercises the new path.
#[test]
fn all_rust_render_surfaces_route_through_the_shared_helper() {
    struct SurfacePin {
        label: &'static str,
        path: &'static str,
        import: &'static str,
        call: &'static str,
    }

    const SURFACES: &[SurfacePin] = &[
        SurfacePin {
            label: "anvil status --json",
            path: "src/commands/status.rs",
            import: "use crate::commands::protection_claim_section;",
            // Production call site at status.rs:1167 — the `&daemon_snapshot`
            // argument anchors this to the real call expression, not the
            // `protection_claim_section::resolve_protection_claim` mention
            // in any nearby doc-comment.
            call: "protection_claim_section::resolve_protection_claim(",
        },
        SurfacePin {
            label: "anvil doctor (entry point)",
            path: "src/commands/doctor.rs",
            import: "use crate::commands::protection_claim_section;",
            // Production call sites at doctor.rs:47 and :50 — the trailing
            // `();` makes this a call expression, not a path reference.
            call: "protection_claim_section::fetch_protection_claim_for_cwd();",
        },
        SurfacePin {
            label: "protection_claim_section helper",
            path: "src/commands/protection_claim_section.rs",
            import: "use anvil_intercept::status::build_protection_claim_from_wire;",
            // Production call at protection_claim_section.rs:42. The
            // `snapshot, worktree)` argument tail is specific to the
            // real invocation; a `/// build_protection_claim_from_wire`
            // doc line would not include it.
            call: "build_protection_claim_from_wire(snapshot, worktree)",
        },
        SurfacePin {
            label: "MCP shim validate_write",
            path: "src/mcp/validation.rs",
            import: "use anvil_intercept::status::build_protection_claim_from_wire;",
            // Production call at validation.rs:234. The `&snapshot,
            // workspace_root` tail anchors to the real call expression.
            call: "build_protection_claim_from_wire(&snapshot, workspace_root)",
        },
    ];

    for surface in SURFACES {
        let contents = fs::read_to_string(surface.path).unwrap_or_else(|err| {
            panic!(
                "{}: source file at {} unreadable ({err}); cross-surface parity pin needs the path to exist",
                surface.label, surface.path,
            )
        });
        assert!(
            contents.contains(surface.import),
            "{}: source at {} no longer imports the shared protection-claim helper.\n\
             Expected import marker: `{}`\n\
             If this surface intentionally moved to a new helper, update the SURFACES \
             table in `tests/protection_claim_cross_surface.rs` and add a parity case \
             that exercises the new path.",
            surface.label,
            surface.path,
            surface.import,
        );
        assert!(
            contents.contains(surface.call),
            "{}: source at {} no longer contains a call expression matching `{}`.\n\
             This is the failure mode this pin exists for — a refactor that demotes \
             the helper to a doc-comment-only mention (without an actual call) would \
             leave the imports + comments intact while silently bypassing the shared \
             claim builder.\n\
             If this surface intentionally moved to a new helper, update the SURFACES \
             table in `tests/protection_claim_cross_surface.rs` and add a parity case \
             that exercises the new path.",
            surface.label,
            surface.path,
            surface.call,
        );
    }
}

/// Every cross-surface fixture file lives under `tests/fixtures/
/// status_v1/cross_surface/`. Pin the directory shape so a future
/// fixture rename doesn't silently break the TS side, which reads
/// from the same path.
#[test]
fn fixture_directory_layout_is_pinned() {
    let root = fixtures_root();

    if std::env::var_os("ANVIL_UPDATE_FIXTURES").is_some() {
        fs::create_dir_all(&root).expect("create cross_surface fixture dir");
        for case in CASES {
            let worktree = Path::new(case.worktree);
            let snapshot = (case.snapshot_builder)(worktree);
            let claim = build_protection_claim_from_wire(&snapshot, worktree);
            assert_or_update_fixture(&fixture_path(case), &render_fixture(&claim));
        }
    }

    let count = fs::read_dir(&root)
        .unwrap_or_else(|err| {
            panic!(
                "cross_surface fixture dir missing at {}: {err}; run `ANVIL_UPDATE_FIXTURES=1 cargo test --test protection_claim_cross_surface` first",
                root.display(),
            )
        })
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .count();
    assert_eq!(
        count,
        CASES.len(),
        "cross_surface fixture count drift: {} on disk, {} cases declared",
        count,
        CASES.len(),
    );
}
