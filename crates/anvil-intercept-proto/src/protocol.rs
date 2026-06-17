//! DRVR-002 / DRVR-008: Editor-driver protocol method names and
//! capability vocabulary.
//!
//! This module is the **authoritative** Rust definition of:
//!
//! - The JSON-RPC method-name constants the editor driver and the
//!   daemon agree on (the `ANVIL_*` constants).
//! - The capability lattice the §3.3 state machine moves a driver
//!   through (`Attached` → `Participating`).
//!
//! TS bindings in `packages/anvil-driver-client/src/protocol/` mirror
//! these constants byte-for-byte. The Rust side is the one source of
//! truth; if the two drift, the Rust side wins and the TS side is
//! re-pinned to match.
//!
//! ## Why this lives in `anvil-intercept-proto`
//!
//! The proto crate already owns the wire vocabulary the daemon and
//! launcher share (`IpcCommand`, `IpcEnvelope`, `SessionRecord`).
//! Method names are wire vocabulary; they belong with their siblings.
//! Putting them in `anvil-intercept` proper would force every
//! consumer (e.g. `auth.rs`'s capability negotiation) to depend on
//! the daemon binary's runtime crate, and any future
//! Wasm-/embedded-side daemon implementation would have to re-export
//! the names from a different crate. Keeping them in `proto` makes
//! the constants importable everywhere with no extra dependency.
//!
//! ## Method namespace policy
//!
//! Per §3.2 of the editor-and-mcp-driver design spec, **no new
//! `anvil/` method without a concrete editor feature that cannot be
//! expressed in stock LSP**. Every method below has a v1 consumer:
//!
//! - `anvil/publishDiagnostics` — server → client notification, the
//!   Anvil flavour of LSP `textDocument/publishDiagnostics` carrying
//!   `Diagnostic` from `anvil-kernel-types` rather than the LSP
//!   shape (so suppression / mode / category survive).
//! - `anvil/scan_buffer` — client → server request, the mid-edit
//!   buffer scan path. Companion to the existing `scan_buffer` JSON-RPC
//!   method; the `anvil/`-namespaced alias is what drivers advertise
//!   in their manifest.
//! - `anvil/enforcement/ack` — client → server, confirms an
//!   enforcement decision was carried out. Drivers that do not
//!   advertise this method are capped at read-only per DRVR-008.
//! - `anvil/gate/request` — client → server, asks for a gate-result
//!   stream over the telemetry lane. Resolves the M3 council-review
//!   item that `anvil/gate/request` was missing from the §3.2 method
//!   table while §3.7 referenced it.
//! - `anvil/suppression/apply` — client → server, requests the
//!   daemon to validate and normalise a `@anvil-ignore` comment per
//!   ADR-004.
//! - `anvil/status/query` — client → server, returns the current
//!   session / fence / driver state for a worktree.
//! - `anvil/validate_paths` — client → server, the save-time verdict
//!   verb (ADR-061 / DSV-002): certify a change set against the warm
//!   graph cache and return the verdict-shaped response.
//! - `anvil/workspace_status` — client → server, a read-only
//!   workspace-assurance snapshot (the `anvil status` surface).
//! - `anvil/request_full_scan` — client → server, ask the daemon to
//!   re-establish a clean baseline after assurance went stale.
//!
//! Registration invariant: every `pub const ANVIL_*` method name here
//! MUST also appear in [`ALL_ANVIL_METHODS`]; the
//! `all_anvil_methods_two_directional` test count-pins the slice so a
//! method cannot be listed without a backing const, nor the count
//! changed silently. (Rust cannot enumerate module consts at test time,
//! so a brand-new const left out of *both* the slice and the test is
//! the one case tests can't catch — hence this written rule.)
//!
//! LSP methods (`textDocument/publishDiagnostics`,
//! `textDocument/codeAction`, `workspace/applyEdit`,
//! `window/showMessage`, `initialize`/`initialized`) are pinned by
//! the LSP spec and are not re-declared here. Drivers speak both
//! languages over the same transport; the daemon routes by method
//! name at the JSON-RPC layer.

use anvil_gctx_types::{
    AffectedTestsOutcome, AffectedTestsQuery, FindCallersOutcome, FindCallersQuery,
    FindDependentsOutcome, FindDependentsQuery, ImpactOutcome, ImpactQuery, SearchSymbolsOutcome,
    SearchSymbolsQuery,
};
use serde::{Deserialize, Serialize};

/// Shared wire envelope for the set of diagnostics a scan response
/// carries on `params.diagnostics`. Each element is the canonical
/// `anvil.diagnostic.v1` shape (`anvil_kernel_types::Diagnostic`); see
/// `crates/anvil-kernel-types/src/diagnostics.rs`.
///
/// Owned here in `anvil-intercept-proto` so the `scan_buffer` response
/// ([`ScanBufferResponse`] in `anvil-intercept`) and the ADR-061
/// Sub-phase A `validate_paths` response type their `diagnostics`
/// field against the **same** type. This closes council finding **B3**
/// (2026-06-01 daemon-graph verdict): Task 1 of the save-time plan
/// "froze" a wire that named a phantom `ScanDiagnostics` the proto
/// crate did not own, and the real type (`ScanBufferResponse`) was
/// declared daemon-local — exactly the drift this alias removes.
///
/// Lighter form per the B3/C5 ruling: a type alias for
/// `Vec<anvil_kernel_types::Diagnostic>` rather than a wrapping struct,
/// and no re-export of `Diagnostic` (consumers name the kernel type
/// directly — one canonical path). Full envelope unification (a single
/// redaction guard hung off one struct) is deferred to Sub-phase A′.
pub type DiagnosticEnvelope = Vec<anvil_kernel_types::Diagnostic>;

/// Server → client notification carrying [`Diagnostic`] payloads. The
/// outer wrapper is the JSON-RPC notification envelope; the inner
/// `params.diagnostics` array holds the canonical
/// `anvil.diagnostic.v1` shape (see
/// `crates/anvil-kernel-types/src/diagnostics.rs`).
///
/// Distinct from LSP's `textDocument/publishDiagnostics` because the
/// payload preserves Anvil's `mode`, `category`, `suppression`, and
/// `correlationId` fields that the LSP shape would drop. Drivers
/// that want LSP rendering MUST translate locally; the daemon does
/// not emit a stock-LSP variant.
pub const ANVIL_PUBLISH_DIAGNOSTICS: &str = "anvil/publishDiagnostics";

/// Client → server request: scan a mid-edit buffer for diagnostics.
/// Companion to the existing `scan_buffer` method; the
/// `anvil/`-namespaced form is what drivers advertise in their
/// manifest so capability negotiation can confirm both ends speak
/// the namespaced form. Consumers of the legacy `scan_buffer` method
/// continue to work — both names route to the same handler.
pub const ANVIL_SCAN_BUFFER: &str = "anvil/scan_buffer";

/// Client → server: confirms an enforcement decision was carried
/// out. **DRVR-008's central method:** drivers that do not advertise
/// support for this method cannot be promoted past
/// [`Capability::Attached`] regardless of `.anvil.yaml` requesting
/// participation.
pub const ANVIL_ENFORCEMENT_ACK: &str = "anvil/enforcement/ack";

/// Client → server: asks the daemon to start streaming gate-result
/// snapshots over the telemetry lane (or, for one-shot consumers,
/// returns a single snapshot synchronously). Resolves the M3
/// council-review item.
pub const ANVIL_GATE_REQUEST: &str = "anvil/gate/request";

/// Client → server: requests the daemon validate and normalise a
/// `@anvil-ignore` comment per ADR-004. The driver supplies the
/// proposed comment + range + reason; the daemon returns the
/// normalised comment which the driver applies via
/// `workspace/applyEdit`.
pub const ANVIL_SUPPRESSION_APPLY: &str = "anvil/suppression/apply";

/// Client → server: returns current session / fence / driver state
/// for a worktree. Single-snapshot read; subscription form lives on
/// the telemetry lane.
pub const ANVIL_STATUS_QUERY: &str = "anvil/status/query";

/// Client → server: certify a set of changed paths against the warm
/// per-`WorktreeKey` graph cache and return the verdict-shaped
/// [`ValidatePathsResponse`]. The save-time hot-path verb (ADR-061
/// Sub-phase A); `watch` and the MCP `anvil_validate_write` tool are
/// thin clients of it. The wire is **frozen** across sub-phases — only
/// the cache backing it swaps (interim `SymbolGraph` → GV2 hot-read →
/// warm-start). See the daemon-save-time-validation module (DSV).
pub const ANVIL_VALIDATE_PATHS: &str = "anvil/validate_paths";

/// Client → server: a read-only snapshot of a worktree's
/// [`WorkspaceAssurance`] (the `anvil status` surface) without
/// submitting any change set. Companion to [`ANVIL_VALIDATE_PATHS`].
pub const ANVIL_WORKSPACE_STATUS: &str = "anvil/workspace_status";

/// Client → server: request the daemon run a full (cold) scan of a
/// worktree to re-establish a `Clean` baseline after the assurance
/// state has gone `Stale`/`Unavailable`. Returns the post-request
/// [`WorkspaceAssurance`] (typically `running`/`pending`).
pub const ANVIL_REQUEST_FULL_SCAN: &str = "anvil/request_full_scan";

/// Client → server: a read-only, identity-only GCTX symbol search (GCTX-010,
/// ADR-084). The daemon performs the egress projection and returns sealed DTOs
/// ([`GctxSearchSymbolsResponse`]); the MCP server never holds a graph. This is
/// the assistant-facing context surface, dispatched on its own read-only
/// [`crate`]-side arm — never the save-time `validate_paths` path.
pub const ANVIL_GCTX_SEARCH_SYMBOLS: &str = "anvil/gctx/search_symbols";

/// Client → server: a read-only, identity-only GCTX dependents traversal
/// (GCTX-011, ADR-084). Given a workspace-relative file, the daemon walks its
/// reverse-impact (importer) set over the resident dependency graph and returns
/// sealed file-keyed DTOs ([`GctxFindDependentsResponse`]). Like
/// [`ANVIL_GCTX_SEARCH_SYMBOLS`], it dispatches on its own read-only arm via the
/// `GctxDispatch` surface — never the save-time `validate_paths` path.
pub const ANVIL_GCTX_FIND_DEPENDENTS: &str = "anvil/gctx/find_dependents";

/// Client → server: a read-only, identity-only GCTX caller traversal (GCTX-014,
/// ADR-084 / GCALL-007). Given a `SymbolIdentity`, the daemon walks its reverse
/// **call** graph (the symbols that call it) over the resident symbol graph and
/// returns sealed identity-only DTOs ([`GctxFindCallersResponse`]). Dispatched on
/// the same read-only `GctxDispatch` surface; never the save-time path.
pub const ANVIL_GCTX_FIND_CALLERS: &str = "anvil/gctx/find_callers";

/// Client → server: a read-only, identity-only GCTX impact-of-change report
/// (GCTX-012, ADR-084). Given a set of **changed file paths** (never diff
/// content), the daemon projects the blast radius — affected symbols, the
/// dependent-file closure, and heuristic known tests — and returns a sealed
/// `ImpactReport` ([`GctxImpactOfChangeResponse`]). Dispatched on the same
/// read-only `GctxDispatch` surface; never the save-time path.
pub const ANVIL_GCTX_IMPACT_OF_CHANGE: &str = "anvil/gctx/impact_of_change";

/// Client → server: a read-only, identity-only GCTX affected-tests report
/// (GCTX-013, ADR-084). Given a set of **changed file paths** (never diff
/// content), the daemon projects the likely-relevant test files (with evidence
/// edges + distance) and the changed non-test files with no resident test
/// importer (coverage gaps), and returns a sealed `AffectedTestsReport`
/// ([`GctxAffectedTestsResponse`]). Dispatched on the same read-only
/// `GctxDispatch` surface; never the save-time path.
pub const ANVIL_GCTX_AFFECTED_TESTS: &str = "anvil/gctx/affected_tests";

/// Capability lattice for the §3.3 state machine.
///
/// `Attached` is the read-only floor: every successfully-handshaken
/// driver reaches this state. `Participating` is the
/// enforcement-candidate state: drivers that have passed the
/// allowlist gate (DRVR-007) AND advertise
/// [`ANVIL_ENFORCEMENT_ACK`] (DRVR-008) can be promoted to it.
///
/// **Order matters.** The enum derives `Ord` so callers can compare
/// "is requested capability higher than what the manifest allows"
/// without re-implementing the lattice; v1 only has two states so
/// the comparison is trivial, but future capability tiers (e.g.
/// `Trusted` for cross-host drivers) extend the lattice rather than
/// rewriting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Read-only diagnostic mode. Default for every driver after
    /// successful handshake. Subscribes to telemetry, renders
    /// diagnostics, applies suppression edits — but never acks
    /// enforcement decisions and is never escalated to fence on
    /// refusal.
    Attached,
    /// Enforcement-participating mode. Receives
    /// `enforcement.decision` events; ack-or-refuse contract per
    /// §2.5; subject to the reliability budget in §2.6. Reaching
    /// this state requires BOTH the DRVR-007 allowlist check AND
    /// the DRVR-008 method advertisement.
    Participating,
}

impl Capability {
    /// Wire string for log / telemetry emission. Kebab-case to match
    /// the rest of the daemon's structured-log vocabulary.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::Participating => "participating",
        }
    }
}

/// Convenience: every `anvil/` method this protocol version defines.
/// Useful for tests and for documentation generation; consumers
/// negotiating capability use the named constants directly.
pub const ALL_ANVIL_METHODS: &[&str] = &[
    ANVIL_PUBLISH_DIAGNOSTICS,
    ANVIL_SCAN_BUFFER,
    ANVIL_ENFORCEMENT_ACK,
    ANVIL_GATE_REQUEST,
    ANVIL_SUPPRESSION_APPLY,
    ANVIL_STATUS_QUERY,
    ANVIL_VALIDATE_PATHS,
    ANVIL_WORKSPACE_STATUS,
    ANVIL_REQUEST_FULL_SCAN,
    ANVIL_GCTX_SEARCH_SYMBOLS,
    ANVIL_GCTX_FIND_DEPENDENTS,
    ANVIL_GCTX_FIND_CALLERS,
    ANVIL_GCTX_IMPACT_OF_CHANGE,
    ANVIL_GCTX_AFFECTED_TESTS,
];

// ============================================================================
// DSV-002 — the frozen `validate_paths` verdict wire (ADR-061 Sub-phase A)
// ============================================================================
//
// These types pin the forward-compatible, verdict-shaped contract the daemon
// answers `validate_paths`/`workspace_status`/`request_full_scan` with, and
// that all four delivery surfaces (watch+daemon, watch+fallback, MCP+daemon,
// MCP+fallback) integrate against. The **wire is frozen once and the backing
// swaps underneath it** across sub-phases (interim `SymbolGraph` cache → GV2
// hot-read slice → warm-start persistence) so consumers never re-integrate.
//
// Forward-compatibility rule (MLP2-052 style): no type here uses
// `#[serde(deny_unknown_fields)]`, so a newer daemon can add additive fields
// without breaking an older client's deserialise. Wire strings are frozen:
// `snake_case` for method-adjacent/state vocabulary, `kebab-case` for the
// reason/family vocabulary that mirrors the daemon's structured-log strings.
//
// Two boundaries are deliberately NOT this crate's job and live downstream:
//   * Resource bounds. `ValidatePathsRequest.paths` and the `String` fields
//     are unbounded by design here; request-count and per-string length caps
//     are enforced at the daemon request handler (DSV-003+), so the
//     deserialise-tolerant wire does not become an unbounded-allocation
//     surface for a same-uid client.
//   * Request correlation. `request_full_scan` carries no scan-id today; if a
//     consumer ever needs to distinguish "my scan" from a coalesced in-flight
//     one, an `Option<String>` id is an additive (non-breaking) field to add
//     when DSV-005/-007 give it a real caller — not pre-emptively here.

/// How a single changed path changed, as classified before the daemon
/// re-derives identity from disk. Internally tagged on `change` so it
/// flattens cleanly into [`ChangeDescriptor`]; `renamed` carries the
/// root-relative, slash-normalised previous path in `from`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "change")]
pub enum ChangeKindWire {
    /// The path did not exist in the prior generation and now does.
    Created,
    /// The path's content changed (includes atomic-save inode flips —
    /// the daemon classifies those as a modify, not a rename).
    Modified,
    /// The path existed and no longer does.
    Deleted,
    /// The path was renamed; `from` is the prior root-relative path.
    Renamed {
        /// Root-relative, slash-normalised previous path.
        from: String,
    },
}

/// One entry in a [`ValidatePathsRequest`] change set: the path, how it
/// changed, and optional client-supplied hints. **The daemon never trusts
/// `content_hash` for a verdict** (it re-reads the openat2-guarded bytes);
/// the hint only short-circuits redundant work and feeds coalescing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeDescriptor {
    /// Root-relative, slash-normalised path that changed.
    pub path: String,
    /// The classified change kind, flattened onto the `change` tag.
    #[serde(flatten)]
    pub change: ChangeKindWire,
    /// Optional client hint; advisory only, never authoritative for a verdict.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content_hash: Option<String>,
    /// Optional client-observed mtime (epoch seconds); advisory only.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mtime: Option<i64>,
}

/// Request body for [`ANVIL_VALIDATE_PATHS`]: certify `paths` under
/// `workspace_root` (which must already be an admitted root — see DSV-003
/// auth).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatePathsRequest {
    /// Canonical, admitted workspace root the changes are relative to.
    pub workspace_root: String,
    /// The change set to certify.
    pub paths: Vec<ChangeDescriptor>,
}

/// What the verdict actually attests. `Certified` is a sound clean claim
/// over the [`check_families`](ValidatePathsResponse::check_families) only;
/// `Partial` means the daemon fell back to a scoped/Stale answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    /// A sound clean attestation over the listed `check_families`.
    Certified,
    /// Coverage is incomplete; consult `workspace_assurance` for the reason.
    Partial,
}

/// The check families a `certified` verdict attests (B2). Frozen as
/// `[antipattern]` for Sub-phase A — `coverage: certified` is **never** an
/// unscoped structural-safety claim; structural policy stays on whole-repo
/// `anvil gate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckFamily {
    /// The antipattern check family (`anvil-checks::antipattern`).
    Antipattern,
}

/// The coarse workspace-assurance state the `anvil status` surface renders.
///
/// Forward-compat: like [`StaleReason`], this enum is on the frozen wire and may
/// gain members in a later daemon, so it carries a [`AssuranceState::Unknown`]
/// `#[serde(other)]` fallback (DSV-045 / ADR-085 Decision 5b). Without it the
/// first added variant (`Bounded`) would *hard-fail* deserialisation of every
/// [`WorkspaceAssurance`]-bearing response on shipped v0.8.0-beta clients —
/// i.e. the addition would be **breaking, not additive**. With the fallback, an
/// older client meeting a newer daemon's unrecognised state string degrades to
/// `Unknown` rather than failing the whole parse. **Consumers MUST treat
/// `Unknown` fail-safe as `Stale`** (never as `Clean`); they MUST NOT map it to
/// a trusting path. New named states are therefore additive on both ends and do
/// not require a `protocolVersion` bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssuranceState {
    /// Warm state is consistent and the last verdict was certifiable.
    Clean,
    /// The verdict cannot be trusted clean; see the paired [`StaleReason`].
    Stale,
    /// A scan is queued but has not started.
    Pending,
    /// A scan is in flight.
    Running,
    /// The warm graph is populated but the workspace exceeded the walk file-count
    /// cap *after* the gitignore pre-filter, so coverage is bounded — known
    /// incomplete, not certifiable as complete. Carries
    /// [`WorkspaceAssurance::scan_coverage`] and, like the lifecycle states,
    /// **no** [`StaleReason`] (it is a lifecycle state, not a staleness cause).
    /// Deliberately named `Bounded`, *not* `Partial`, to avoid colliding with
    /// the unrelated [`Coverage::Partial`] (wire `"partial"`), which is a
    /// check-family-coverage axis (DSV-045 / ADR-085 Decision 5a). Consumers
    /// MUST handle it explicitly (no wildcard-to-`Clean`): a bounded graph is
    /// served as identity-results-marked-bounded, never as complete.
    Bounded,
    /// No daemon answered (daemon-absent / mid-session death). Never a
    /// truncated `clean`.
    Unavailable,
    /// `#[serde(other)]` fallback: a state string this build does not know,
    /// emitted by a newer daemon. **Treated as stale (fail-safe)** — never as
    /// clean. Never produced by this build's own serialiser for a known state.
    #[serde(other)]
    Unknown,
}

/// Why assurance is not `Clean`. Default-deny: any change class the daemon
/// cannot prove certifiable maps to one of these (unknown ⇒ stale, never
/// clean). Frozen kebab-case wire strings.
///
/// Forward-compat: the *fields* of the surrounding wire grow additively, but
/// this reason vocabulary is the one frozen enum most likely to gain members
/// in a later daemon. So it carries a [`StaleReason::Unknown`] `#[serde(other)]`
/// fallback — an older client that meets a newer daemon's unrecognised reason
/// string degrades to `Unknown` (still stale, fail-safe) rather than failing
/// the whole response parse. New named reasons are therefore additive on both
/// ends; they do not require a `protocolVersion` bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StaleReason {
    /// A change needs cross-file resolution the warm cache cannot supply
    /// yet (also the cold-key initial state).
    CrossFileResolutionNeeded,
    /// A path in the change set was deleted.
    Deleted,
    /// A path in the change set was renamed.
    Renamed,
    /// A symlink in the resolution path was retargeted.
    SymlinkRetarget,
    /// A config / boundary / policy file edit changed the rule surface.
    ConfigBoundaryPolicyEdit,
    /// A `.gitignore` scope change altered which paths are in play.
    GitignoreScopeChange,
    /// The bounded reverse-impact closure exceeded its budget.
    ImpactSetOverflow,
    /// The warm state for this worktree was evicted from the cache.
    WarmStateEvicted,
    /// A scan exceeded its time budget.
    ScanTimeout,
    /// No daemon answered the request.
    DaemonAbsent,
    /// An unrecognised *change class* the daemon could not classify — fails
    /// closed to stale. (Distinct from [`StaleReason::Unknown`], which is the
    /// wire-level fallback for an unrecognised *reason string*.)
    UnknownClass,
    /// `#[serde(other)]` fallback: a reason string this build does not know,
    /// emitted by a newer daemon. Treated as stale (fail-safe). Never produced
    /// by this build's own serialiser for a known reason.
    #[serde(other)]
    Unknown,
}

/// Walk coverage for a completed-but-bounded full scan (DSV-045 / ADR-085
/// Decision 5c). Carried on an [`AssuranceState::Bounded`] snapshot so a
/// consumer can surface *how* bounded a worktree is (e.g. "scanned 100k of
/// 250k files"). Named distinctly from the [`Coverage`] check-family axis: this
/// is a file-count truncation signal, not a verdict-attestation one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanCoverage {
    /// Files actually walked, parsed, and applied to the warm graph (the
    /// post-gitignore-filter `max_walk_files` cap).
    pub scanned_files: u64,
    /// Total files the gitignore-filtered, depth-capped walk found. Greater than
    /// `scanned_files` exactly when the worktree was truncated to `Bounded`. May
    /// be a **lower bound** when the daemon's internal walk-count ceiling is hit
    /// on a pathologically large tree (the true workspace may hold more), so a
    /// "scanned X of Y" render should treat Y as "at least Y".
    pub total_files: u64,
}

/// The workspace-assurance snapshot carried by every verdict and by the
/// standalone status/full-scan responses. Deliberately carries **no**
/// graph-version field: the wire is frozen against the backing, so a GV2
/// swap (Sub-phase A′) must not leak internal graph versioning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceAssurance {
    /// The coarse assurance state.
    pub state: AssuranceState,
    /// The cause when assurance is not trustworthy. Invariant: present iff
    /// `state` is `Stale` or `Unavailable` (an `Unavailable` snapshot carries
    /// [`StaleReason::DaemonAbsent`]); always `None` for `Clean`, `Pending`,
    /// `Running`, and `Bounded`, which are lifecycle states with no staleness
    /// cause. (`Bounded` is a *completed* scan whose coverage was truncated —
    /// its incompleteness rides [`Self::scan_coverage`], not a `StaleReason`.)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<StaleReason>,
    /// Monotonic, per-worktree **opaque** turnover token; bumps on eviction /
    /// cold rebuild so consumers can detect warm-state turnover. Not a global
    /// or graph-internal version (cf. the deliberate `graph_version` omission)
    /// — a backing swap must keep it an opaque counter.
    pub generation: u64,
    /// RFC 3339 timestamp of the last completed full scan, if any. The wire
    /// type is a `String` (a typed timestamp is not serde-version-stable);
    /// consumers MUST treat a parse failure as "scan time unknown" rather than
    /// propagating it as a verdict error.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_full_scan: Option<String>,
    /// Walk coverage, present only on an [`AssuranceState::Bounded`] snapshot
    /// (DSV-045). `default` + `skip_serializing_if` keep it forward-compatible:
    /// absent on older daemons, ignored by older clients.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scan_coverage: Option<ScanCoverage>,
}

/// One evaluated path echoed back with the **daemon-computed** content hash
/// (not the client's hint) — the authoritative record of what was certified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatedPath {
    /// Root-relative path that was evaluated.
    pub path: String,
    /// Hash of the bytes the daemon actually read under the openat2 guard.
    /// `None` for `Deleted` and `Renamed` (the `from` side) entries: those
    /// have no daemon-readable bytes, so there is no content to hash. A
    /// `Created`/`Modified` entry always carries `Some(_)`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content_hash: Option<String>,
}

/// The verdict-shaped response to [`ANVIL_VALIDATE_PATHS`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatePathsResponse {
    /// Findings for the change set, in the canonical `anvil.diagnostic.v1`
    /// shape (the proto-owned shared envelope; B3).
    pub diagnostics: DiagnosticEnvelope,
    /// Each path the daemon evaluated, with its daemon-computed hash.
    pub evaluated: Vec<EvaluatedPath>,
    /// Workspace assurance after applying this change set.
    pub workspace_assurance: WorkspaceAssurance,
    /// What the verdict attests over the listed `check_families`.
    pub coverage: Coverage,
    /// The families `certified` attests (frozen `[antipattern]`; B2).
    /// Invariant: MUST be non-empty when `coverage` is `Certified` — a clean
    /// attestation over zero families is meaningless. May be empty only with
    /// `coverage: Partial` (e.g. an `Unavailable` snapshot attests nothing).
    pub check_families: Vec<CheckFamily>,
}

/// Request body for [`ANVIL_WORKSPACE_STATUS`]: a read-only assurance snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceStatusRequest {
    /// Canonical, admitted workspace root to report on.
    pub workspace_root: String,
}

/// Response to [`ANVIL_WORKSPACE_STATUS`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceStatusResponse {
    /// The current assurance snapshot for the worktree.
    pub workspace_assurance: WorkspaceAssurance,
}

/// Request body for [`ANVIL_REQUEST_FULL_SCAN`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestFullScanRequest {
    /// Canonical, admitted workspace root to re-scan.
    pub workspace_root: String,
}

/// Response to [`ANVIL_REQUEST_FULL_SCAN`]: the post-request assurance state
/// (typically `running`/`pending`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestFullScanResponse {
    /// The assurance snapshot after the scan was requested.
    pub workspace_assurance: WorkspaceAssurance,
}

/// Request body for [`ANVIL_GCTX_SEARCH_SYMBOLS`] (GCTX-010, ADR-084).
///
/// The query is the sealed, graph-free [`SearchSymbolsQuery`] value type; the
/// `workspace_root` is validated daemon-side against the connection's
/// admitted-root set (ADR-084 C3 / CE-8) before any projection runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxSearchSymbolsRequest {
    /// Canonical, admitted workspace root to project from.
    pub workspace_root: String,
    /// The identity-only search filters.
    #[serde(default)]
    pub query: SearchSymbolsQuery,
}

/// Response to [`ANVIL_GCTX_SEARCH_SYMBOLS`]: the daemon-projected sealed egress
/// DTO.
///
/// The daemon performs the CE-5 [`anvil_gctx_types`] projection itself, so this
/// response **is** the sealed DTO — the MCP consumer deserialises it without
/// ever linking graph internals. `workspace_assurance` always rides along (the
/// CE-7 degradation signal); `outcome` is `ready` with identity results when the
/// graph is readable, and a named non-`ready` variant otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxSearchSymbolsResponse {
    /// Workspace assurance at projection time (CE-7).
    pub workspace_assurance: WorkspaceAssurance,
    /// The status-tagged search outcome (sealed, identity-only).
    pub outcome: SearchSymbolsOutcome,
}

/// Request body for [`ANVIL_GCTX_FIND_DEPENDENTS`] (GCTX-011, ADR-084).
///
/// The query is the sealed, graph-free [`FindDependentsQuery`]; the
/// `workspace_root` is validated daemon-side against the connection's
/// admitted-root set (ADR-084 C3 / CE-8) before any projection runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxFindDependentsRequest {
    /// Canonical, admitted workspace root to project from.
    pub workspace_root: String,
    /// The file-keyed dependents traversal query.
    #[serde(default)]
    pub query: FindDependentsQuery,
}

/// Response to [`ANVIL_GCTX_FIND_DEPENDENTS`]: the daemon-projected sealed egress
/// DTO.
///
/// As with [`GctxSearchSymbolsResponse`], the daemon performs the CE-5 projection
/// itself, so this response **is** the sealed DTO. `workspace_assurance` always
/// rides along (the CE-7 degradation signal); `outcome` is `ready` with
/// file-keyed dependents when the dependency graph is readable, and a named
/// non-`ready` variant otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxFindDependentsResponse {
    /// Workspace assurance at projection time (CE-7).
    pub workspace_assurance: WorkspaceAssurance,
    /// The status-tagged dependents outcome (sealed, identity-only).
    pub outcome: FindDependentsOutcome,
}

/// Request body for [`ANVIL_GCTX_FIND_CALLERS`] (GCTX-014, ADR-084).
///
/// The query is the sealed, graph-free [`FindCallersQuery`]; the `workspace_root`
/// is validated daemon-side against the connection's admitted-root set
/// (ADR-084 C3 / CE-8) before any projection runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxFindCallersRequest {
    /// Canonical, admitted workspace root to project from.
    pub workspace_root: String,
    /// The symbol-keyed caller traversal query.
    #[serde(default)]
    pub query: FindCallersQuery,
}

/// Response to [`ANVIL_GCTX_FIND_CALLERS`]: the daemon-projected sealed egress
/// DTO (identity-only). The daemon performs the CE-5 projection itself, so this
/// response **is** the sealed DTO; `workspace_assurance` always rides along
/// (CE-7), and `outcome` is `ready` with identity-only callers when the call
/// graph is readable, a named non-`ready` variant otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxFindCallersResponse {
    /// Workspace assurance at projection time (CE-7).
    pub workspace_assurance: WorkspaceAssurance,
    /// The status-tagged callers outcome (sealed, identity-only).
    pub outcome: FindCallersOutcome,
}

/// Request body for [`ANVIL_GCTX_IMPACT_OF_CHANGE`] (GCTX-012, ADR-084).
///
/// The query is the sealed, graph-free [`ImpactQuery`] — **changed file paths
/// only**, never diff content (CE-6). The `workspace_root` is validated
/// daemon-side against the connection's admitted-root set (ADR-084 C3 / CE-8)
/// before any projection runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxImpactOfChangeRequest {
    /// Canonical, admitted workspace root to project from.
    pub workspace_root: String,
    /// The change set whose blast radius to report.
    #[serde(default)]
    pub query: ImpactQuery,
}

/// Response to [`ANVIL_GCTX_IMPACT_OF_CHANGE`]: the daemon-projected sealed
/// `ImpactReport` (identity-only).
///
/// As with the other GCTX responses, the daemon performs the CE-5 projection
/// itself, so this response **is** the sealed DTO. `workspace_assurance` always
/// rides along (CE-7); `outcome` is `ready` with the report when the graph is
/// readable, and a named non-`ready` variant otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxImpactOfChangeResponse {
    /// Workspace assurance at projection time (CE-7).
    pub workspace_assurance: WorkspaceAssurance,
    /// The status-tagged impact outcome (sealed, identity-only).
    pub outcome: ImpactOutcome,
}

/// Request body for [`ANVIL_GCTX_AFFECTED_TESTS`] (GCTX-013, ADR-084).
///
/// The query is the sealed, graph-free [`AffectedTestsQuery`] — **changed file
/// paths only**, never diff content (CE-6). The `workspace_root` is validated
/// daemon-side against the connection's admitted-root set (ADR-084 C3 / CE-8)
/// before any projection runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxAffectedTestsRequest {
    /// Canonical, admitted workspace root to project from.
    pub workspace_root: String,
    /// The change set whose likely tests + coverage gaps to report.
    #[serde(default)]
    pub query: AffectedTestsQuery,
}

/// Response to [`ANVIL_GCTX_AFFECTED_TESTS`]: the daemon-projected sealed
/// `AffectedTestsReport` (identity-only).
///
/// As with the other GCTX responses, the daemon performs the CE-5 projection
/// itself, so this response **is** the sealed DTO. `workspace_assurance` always
/// rides along (CE-7); `outcome` is `ready` with the report when the graph is
/// readable, and a named non-`ready` variant otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GctxAffectedTestsResponse {
    /// Workspace assurance at projection time (CE-7).
    pub workspace_assurance: WorkspaceAssurance,
    /// The status-tagged affected-tests outcome (sealed, identity-only).
    pub outcome: AffectedTestsOutcome,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the wire strings. These constants are part of the
    /// daemon ↔ driver contract; changing them is a breaking
    /// protocol change and requires bumping `protocolVersion`.
    #[test]
    fn anvil_method_names_are_stable() {
        assert_eq!(ANVIL_PUBLISH_DIAGNOSTICS, "anvil/publishDiagnostics");
        assert_eq!(ANVIL_SCAN_BUFFER, "anvil/scan_buffer");
        assert_eq!(ANVIL_ENFORCEMENT_ACK, "anvil/enforcement/ack");
        assert_eq!(ANVIL_GATE_REQUEST, "anvil/gate/request");
        assert_eq!(ANVIL_SUPPRESSION_APPLY, "anvil/suppression/apply");
        assert_eq!(ANVIL_STATUS_QUERY, "anvil/status/query");
    }

    #[test]
    fn all_anvil_methods_lists_every_constant_exactly_once() {
        let listed: std::collections::HashSet<&str> = ALL_ANVIL_METHODS.iter().copied().collect();
        assert_eq!(
            listed.len(),
            ALL_ANVIL_METHODS.len(),
            "ALL_ANVIL_METHODS must not contain duplicates"
        );
        // Every named constant is in the listed set. Kept in sync with the
        // DSV-002 additions; the stronger bidirectional/count pin lives in
        // `all_anvil_methods_two_directional`.
        for method in [
            ANVIL_PUBLISH_DIAGNOSTICS,
            ANVIL_SCAN_BUFFER,
            ANVIL_ENFORCEMENT_ACK,
            ANVIL_GATE_REQUEST,
            ANVIL_SUPPRESSION_APPLY,
            ANVIL_STATUS_QUERY,
            ANVIL_VALIDATE_PATHS,
            ANVIL_WORKSPACE_STATUS,
            ANVIL_REQUEST_FULL_SCAN,
        ] {
            assert!(
                listed.contains(method),
                "ALL_ANVIL_METHODS missing {method}"
            );
        }
    }

    #[test]
    fn capability_serialises_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Capability::Attached).unwrap(),
            "\"attached\""
        );
        assert_eq!(
            serde_json::to_string(&Capability::Participating).unwrap(),
            "\"participating\""
        );
    }

    #[test]
    fn capability_round_trips_through_json() {
        for variant in [Capability::Attached, Capability::Participating] {
            let s = serde_json::to_string(&variant).unwrap();
            let back: Capability = serde_json::from_str(&s).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn capability_lattice_orders_attached_below_participating() {
        // Lattice property used by negotiate_capability: requested
        // > granted means a downgrade fired. Pin the relation here so
        // a future enum reordering trips the test instead of the
        // daemon silently letting a manifest cap a driver above its
        // request.
        assert!(Capability::Attached < Capability::Participating);
    }

    #[test]
    fn capability_as_str_matches_serde() {
        // Hand-rolled `as_str` for log emission must agree with the
        // serde rename. Easy to drift when adding a new variant; this
        // test pins the invariant.
        for variant in [Capability::Attached, Capability::Participating] {
            let from_serde = serde_json::to_value(variant).unwrap();
            assert_eq!(from_serde, variant.as_str());
        }
    }

    fn sample_diagnostic() -> anvil_kernel_types::Diagnostic {
        use anvil_kernel_types::diagnostics::{
            Category, DiagnosticSource, KnownMode, Location, Severity,
        };
        use anvil_kernel_types::{Diagnostic, Mode};

        Diagnostic::new(
            "AP-001",
            Severity::Warning,
            "sample finding",
            Location {
                file: "src/lib.rs".to_string(),
                line: Some(12),
                column: Some(3),
                end_line: None,
                end_column: None,
            },
            Category::Antipattern,
            DiagnosticSource {
                rule_id: "AP-001".to_string(),
                source_module: "anvil-checks::antipattern".to_string(),
            },
            Mode::known(KnownMode::MidEdit),
        )
    }

    /// B3: the envelope is the canonical `anvil.diagnostic.v1` array,
    /// owned here in the proto crate (not re-declared daemon-local) so
    /// `scan_buffer` and `validate_paths` reference one type.
    #[test]
    fn diagnostic_envelope_serialises_as_canonical_diagnostic_array() {
        let envelope: DiagnosticEnvelope = vec![sample_diagnostic()];
        let json = serde_json::to_value(&envelope).expect("serialise envelope");
        assert!(json.is_array(), "envelope serialises as a JSON array");
        assert_eq!(json[0]["schema_version"], "anvil.diagnostic.v1");
        assert_eq!(json[0]["id"], "AP-001");
        assert_eq!(json[0]["severity"], "warning");
        assert_eq!(json[0]["category"], "antipattern");
    }

    #[test]
    fn diagnostic_envelope_round_trips_through_json() {
        let envelope: DiagnosticEnvelope = vec![sample_diagnostic()];
        let wire = serde_json::to_string(&envelope).expect("serialise");
        let back: DiagnosticEnvelope = serde_json::from_str(&wire).expect("deserialise");
        assert_eq!(envelope, back);
    }

    // ---- DSV-002: frozen `validate_paths` verdict wire (ADR-061 §2/§5) ----

    fn sample_assurance() -> WorkspaceAssurance {
        WorkspaceAssurance {
            state: AssuranceState::Stale,
            reason: Some(StaleReason::CrossFileResolutionNeeded),
            generation: 7,
            last_full_scan: None,
            scan_coverage: None,
        }
    }

    /// The three new method constants are pinned to their wire form.
    /// Changing any is a breaking protocol change (bump `protocolVersion`).
    #[test]
    fn validate_paths_method_const() {
        assert_eq!(ANVIL_VALIDATE_PATHS, "anvil/validate_paths");
        assert_eq!(ANVIL_WORKSPACE_STATUS, "anvil/workspace_status");
        assert_eq!(ANVIL_REQUEST_FULL_SCAN, "anvil/request_full_scan");
    }

    /// Every `ChangeKindWire` variant round-trips through the flattened
    /// `ChangeDescriptor` envelope, including the `renamed` `from` payload.
    #[test]
    fn change_descriptor_roundtrip_all_variants() {
        let cases = [
            ChangeDescriptor {
                path: "src/lib.rs".to_string(),
                change: ChangeKindWire::Created,
                content_hash: None,
                mtime: None,
            },
            ChangeDescriptor {
                path: "src/lib.rs".to_string(),
                change: ChangeKindWire::Modified,
                content_hash: Some("deadbeef".to_string()),
                mtime: Some(1_717_000_000),
            },
            ChangeDescriptor {
                path: "src/old.rs".to_string(),
                change: ChangeKindWire::Deleted,
                content_hash: None,
                mtime: None,
            },
            ChangeDescriptor {
                path: "src/new.rs".to_string(),
                change: ChangeKindWire::Renamed {
                    from: "src/old.rs".to_string(),
                },
                content_hash: None,
                mtime: None,
            },
        ];
        for case in cases {
            let wire = serde_json::to_string(&case).expect("serialise descriptor");
            let back: ChangeDescriptor =
                serde_json::from_str(&wire).expect("deserialise descriptor");
            assert_eq!(case, back, "round-trip changed the descriptor: {wire}");
        }
    }

    /// The change discriminant serialises as a flattened `change` tag
    /// (`snake_case`), with `renamed` carrying its `from` sibling field.
    #[test]
    fn change_descriptor_uses_flattened_change_tag() {
        let modified = ChangeDescriptor {
            path: "src/lib.rs".to_string(),
            change: ChangeKindWire::Modified,
            content_hash: None,
            mtime: None,
        };
        let json = serde_json::to_value(&modified).expect("serialise");
        assert_eq!(json["change"], "modified");
        assert_eq!(json["path"], "src/lib.rs");
        // Optional fields skip-serialise when absent.
        assert!(json.get("content_hash").is_none());
        assert!(json.get("mtime").is_none());

        let renamed = ChangeDescriptor {
            path: "src/new.rs".to_string(),
            change: ChangeKindWire::Renamed {
                from: "src/old.rs".to_string(),
            },
            content_hash: None,
            mtime: None,
        };
        let json = serde_json::to_value(&renamed).expect("serialise");
        assert_eq!(json["change"], "renamed");
        assert_eq!(json["from"], "src/old.rs");
    }

    /// Forward-compat (MLP2-052 style): an additive unknown field on the
    /// response must deserialise OK so a newer daemon can extend the wire
    /// without breaking an older client.
    #[test]
    fn response_tolerates_unknown_additive_field() {
        let wire = serde_json::json!({
            "diagnostics": [],
            "evaluated": [],
            "workspace_assurance": {
                "state": "clean",
                "generation": 1
            },
            "coverage": "certified",
            "check_families": ["antipattern"],
            "future_field_from_a_newer_daemon": {"nested": true}
        });
        let resp: ValidatePathsResponse =
            serde_json::from_value(wire).expect("unknown additive field must deserialise");
        assert_eq!(resp.coverage, Coverage::Certified);
        assert_eq!(resp.check_families, vec![CheckFamily::Antipattern]);
        assert_eq!(resp.workspace_assurance.state, AssuranceState::Clean);
    }

    /// Every `StaleReason` serialises to its frozen kebab-case wire string.
    #[test]
    fn stale_reason_kebab_wire_strings() {
        let cases = [
            (
                StaleReason::CrossFileResolutionNeeded,
                "cross-file-resolution-needed",
            ),
            (StaleReason::Deleted, "deleted"),
            (StaleReason::Renamed, "renamed"),
            (StaleReason::SymlinkRetarget, "symlink-retarget"),
            (
                StaleReason::ConfigBoundaryPolicyEdit,
                "config-boundary-policy-edit",
            ),
            (StaleReason::GitignoreScopeChange, "gitignore-scope-change"),
            (StaleReason::ImpactSetOverflow, "impact-set-overflow"),
            (StaleReason::WarmStateEvicted, "warm-state-evicted"),
            (StaleReason::ScanTimeout, "scan-timeout"),
            (StaleReason::DaemonAbsent, "daemon-absent"),
            (StaleReason::UnknownClass, "unknown-class"),
        ];
        for (variant, wire) in cases {
            assert_eq!(
                serde_json::to_value(variant).unwrap(),
                serde_json::Value::String(wire.to_string()),
                "{variant:?} must serialise as {wire}"
            );
            let back: StaleReason =
                serde_json::from_value(serde_json::Value::String(wire.to_string())).unwrap();
            assert_eq!(back, variant, "{wire} must deserialise back to {variant:?}");
        }
    }

    /// The assurance surface carries no `graph_version` field — the wire is
    /// frozen against the backing, so a future GV2 swap (sub-phase A′) must
    /// not leak the graph's internal versioning to consumers.
    #[test]
    fn no_graph_version_field() {
        let assurance_json = serde_json::to_value(sample_assurance()).unwrap();
        assert!(
            assurance_json.get("graph_version").is_none(),
            "WorkspaceAssurance must not expose graph_version"
        );

        let status = WorkspaceStatusResponse {
            workspace_assurance: sample_assurance(),
        };
        let status_json = serde_json::to_value(&status).unwrap();
        assert!(
            status_json.get("graph_version").is_none(),
            "WorkspaceStatusResponse must not expose graph_version"
        );
        // ...nor nested under workspace_assurance.
        assert!(
            status_json["workspace_assurance"]
                .get("graph_version")
                .is_none()
        );
    }

    /// B2: `coverage: certified` attests ONLY the antipattern family. A
    /// certified response serialises `check_families: ["antipattern"]`.
    #[test]
    fn response_carries_check_families() {
        let resp = ValidatePathsResponse {
            diagnostics: vec![],
            evaluated: vec![EvaluatedPath {
                path: "src/lib.rs".to_string(),
                content_hash: Some("abc123".to_string()),
            }],
            workspace_assurance: WorkspaceAssurance {
                state: AssuranceState::Clean,
                reason: None,
                generation: 2,
                last_full_scan: None,
                scan_coverage: None,
            },
            coverage: Coverage::Certified,
            check_families: vec![CheckFamily::Antipattern],
        };
        let json = serde_json::to_value(&resp).expect("serialise response");
        assert_eq!(json["coverage"], "certified");
        assert_eq!(json["check_families"], serde_json::json!(["antipattern"]));
        // `reason` skip-serialises when None on a clean verdict.
        assert!(json["workspace_assurance"].get("reason").is_none());
    }

    /// The full request/response pair round-trips losslessly.
    #[test]
    fn validate_paths_request_response_round_trip() {
        let req = ValidatePathsRequest {
            workspace_root: "/home/me/proj".to_string(),
            paths: vec![ChangeDescriptor {
                path: "src/lib.rs".to_string(),
                change: ChangeKindWire::Modified,
                content_hash: Some("h".to_string()),
                mtime: Some(42),
            }],
        };
        let back: ValidatePathsRequest =
            serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert_eq!(req, back);

        let resp = ValidatePathsResponse {
            diagnostics: vec![sample_diagnostic()],
            evaluated: vec![EvaluatedPath {
                path: "src/lib.rs".to_string(),
                content_hash: Some("h".to_string()),
            }],
            workspace_assurance: sample_assurance(),
            coverage: Coverage::Partial,
            check_families: vec![CheckFamily::Antipattern],
        };
        let back: ValidatePathsResponse =
            serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
        assert_eq!(resp, back);
    }

    /// Correction item 9: `ALL_ANVIL_METHODS` is two-directionally pinned.
    /// Forward — every named method constant is in the slice. Backward —
    /// the slice carries no entry that is not a known, named constant, and
    /// its length is count-pinned. A new method therefore cannot be added
    /// to the slice unpinned, nor a constant defined without listing it.
    #[test]
    fn all_anvil_methods_two_directional() {
        // The exhaustive set of named method constants this protocol version
        // defines. Adding a method means adding it here AND to the slice.
        let named: std::collections::HashSet<&str> = [
            ANVIL_PUBLISH_DIAGNOSTICS,
            ANVIL_SCAN_BUFFER,
            ANVIL_ENFORCEMENT_ACK,
            ANVIL_GATE_REQUEST,
            ANVIL_SUPPRESSION_APPLY,
            ANVIL_STATUS_QUERY,
            ANVIL_VALIDATE_PATHS,
            ANVIL_WORKSPACE_STATUS,
            ANVIL_REQUEST_FULL_SCAN,
            ANVIL_GCTX_SEARCH_SYMBOLS,
            ANVIL_GCTX_FIND_DEPENDENTS,
            ANVIL_GCTX_FIND_CALLERS,
            ANVIL_GCTX_IMPACT_OF_CHANGE,
            ANVIL_GCTX_AFFECTED_TESTS,
        ]
        .into_iter()
        .collect();
        let listed: std::collections::HashSet<&str> = ALL_ANVIL_METHODS.iter().copied().collect();

        // Count pin: no silent additions, no silent drops.
        assert_eq!(
            ALL_ANVIL_METHODS.len(),
            14,
            "ALL_ANVIL_METHODS count changed — pin and the named set must move together"
        );
        // Forward: every named const is listed.
        assert!(
            named.is_subset(&listed),
            "a named method is missing from ALL_ANVIL_METHODS"
        );
        // Backward: every listed entry is a known named const.
        assert!(
            listed.is_subset(&named),
            "ALL_ANVIL_METHODS carries an entry with no backing named constant"
        );
    }

    /// GCTX-010 / ADR-084: the search RPC method name is frozen and the
    /// request/response envelopes round-trip, including the named non-`ready`
    /// degradation outcome that rides alongside the assurance snapshot.
    #[test]
    fn gctx_search_symbols_wire_round_trips() {
        assert_eq!(ANVIL_GCTX_SEARCH_SYMBOLS, "anvil/gctx/search_symbols");

        let request = GctxSearchSymbolsRequest {
            workspace_root: "/home/me/proj".into(),
            query: SearchSymbolsQuery {
                name: Some("handle".into()),
                ..Default::default()
            },
        };
        let back: GctxSearchSymbolsRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(request, back);

        // A request may omit `query` entirely (serde default).
        let bare: GctxSearchSymbolsRequest =
            serde_json::from_str(r#"{"workspace_root":"/p"}"#).unwrap();
        assert_eq!(bare.query, SearchSymbolsQuery::default());

        // Ready response with an empty projection + a degraded response.
        for outcome in [
            SearchSymbolsOutcome::Ready(anvil_gctx_types::SearchSymbolsProjection {
                symbols: Vec::new(),
                next_cursor: None,
                redaction_summary: anvil_gctx_types::RedactionSummary::default(),
            }),
            SearchSymbolsOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            SearchSymbolsOutcome::Unavailable,
            SearchSymbolsOutcome::Disabled,
        ] {
            let response = GctxSearchSymbolsResponse {
                workspace_assurance: WorkspaceAssurance {
                    state: AssuranceState::Stale,
                    reason: Some(StaleReason::CrossFileResolutionNeeded),
                    generation: 3,
                    last_full_scan: None,
                    scan_coverage: None,
                },
                outcome,
            };
            let back: GctxSearchSymbolsResponse =
                serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
            assert_eq!(response, back);
        }
    }

    /// GCTX-011 / ADR-084: the dependents RPC method name is frozen and the
    /// request/response envelopes round-trip, including the named non-`ready`
    /// degradation outcome alongside the assurance snapshot.
    #[test]
    fn gctx_find_dependents_wire_round_trips() {
        assert_eq!(ANVIL_GCTX_FIND_DEPENDENTS, "anvil/gctx/find_dependents");

        let request = GctxFindDependentsRequest {
            workspace_root: "/home/me/proj".into(),
            query: FindDependentsQuery {
                file: Some("src/a.ts".into()),
                max_depth: Some(2),
                ..Default::default()
            },
        };
        let back: GctxFindDependentsRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(request, back);

        // A request may omit `query` entirely (serde default).
        let bare: GctxFindDependentsRequest =
            serde_json::from_str(r#"{"workspace_root":"/p"}"#).unwrap();
        assert_eq!(bare.query, FindDependentsQuery::default());

        for outcome in [
            FindDependentsOutcome::Ready(anvil_gctx_types::FindDependentsProjection {
                dependents: Vec::new(),
                next_cursor: None,
                redaction_summary: anvil_gctx_types::RedactionSummary::default(),
            }),
            FindDependentsOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            FindDependentsOutcome::Unavailable,
            FindDependentsOutcome::Disabled,
        ] {
            let response = GctxFindDependentsResponse {
                workspace_assurance: WorkspaceAssurance {
                    state: AssuranceState::Stale,
                    reason: Some(StaleReason::CrossFileResolutionNeeded),
                    generation: 3,
                    last_full_scan: None,
                    scan_coverage: None,
                },
                outcome,
            };
            let back: GctxFindDependentsResponse =
                serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
            assert_eq!(response, back);
        }
    }

    /// GCTX-014 / ADR-084: the callers RPC method name is frozen and the
    /// request/response envelopes round-trip, including the named non-`ready`
    /// degradation outcome alongside the assurance snapshot.
    #[test]
    fn gctx_find_callers_wire_round_trips() {
        use anvil_kernel_types::{SymbolIdentity, SymbolKind};

        assert_eq!(ANVIL_GCTX_FIND_CALLERS, "anvil/gctx/find_callers");

        let request = GctxFindCallersRequest {
            workspace_root: "/home/me/proj".into(),
            query: FindCallersQuery {
                target: Some(SymbolIdentity {
                    file: "src/a.ts".into(),
                    kind: SymbolKind::Function,
                    name: "handle".into(),
                    ordinal: 0,
                }),
                max_depth: Some(2),
                ..Default::default()
            },
        };
        let back: GctxFindCallersRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(request, back);

        // A request may omit `query` entirely (serde default).
        let bare: GctxFindCallersRequest =
            serde_json::from_str(r#"{"workspace_root":"/p"}"#).unwrap();
        assert_eq!(bare.query, FindCallersQuery::default());

        for outcome in [
            FindCallersOutcome::Ready(anvil_gctx_types::FindCallersProjection {
                callers: Vec::new(),
                next_cursor: None,
                redaction_summary: anvil_gctx_types::RedactionSummary::default(),
                partial: false,
            }),
            FindCallersOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            FindCallersOutcome::Unavailable,
            FindCallersOutcome::Disabled,
        ] {
            let response = GctxFindCallersResponse {
                workspace_assurance: WorkspaceAssurance {
                    state: AssuranceState::Stale,
                    reason: Some(StaleReason::CrossFileResolutionNeeded),
                    generation: 3,
                    last_full_scan: None,
                    scan_coverage: None,
                },
                outcome,
            };
            let back: GctxFindCallersResponse =
                serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
            assert_eq!(response, back);
        }
    }

    /// GCTX-012 / ADR-084: the impact RPC method name is frozen and the
    /// request/response envelopes round-trip, including the named non-`ready`
    /// degradation outcome alongside the assurance snapshot.
    #[test]
    fn gctx_impact_of_change_wire_round_trips() {
        assert_eq!(ANVIL_GCTX_IMPACT_OF_CHANGE, "anvil/gctx/impact_of_change");

        let request = GctxImpactOfChangeRequest {
            workspace_root: "/home/me/proj".into(),
            query: ImpactQuery {
                changed_files: vec!["src/a.ts".into(), "src/b.ts".into()],
                max_depth: Some(2),
            },
        };
        let back: GctxImpactOfChangeRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(request, back);

        // A request may omit `query` entirely (serde default).
        let bare: GctxImpactOfChangeRequest =
            serde_json::from_str(r#"{"workspace_root":"/p"}"#).unwrap();
        assert_eq!(bare.query, ImpactQuery::default());

        for outcome in [
            ImpactOutcome::Ready(anvil_gctx_types::ImpactReport {
                affected_symbols: Vec::new(),
                dependent_files: Vec::new(),
                known_tests: Vec::new(),
                summary: anvil_gctx_types::ImpactSummary::default(),
            }),
            ImpactOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            ImpactOutcome::Unavailable,
            ImpactOutcome::Disabled,
            ImpactOutcome::InvalidQuery {
                reason: "changed_files exceeds the 200-file cap".into(),
            },
        ] {
            let response = GctxImpactOfChangeResponse {
                workspace_assurance: WorkspaceAssurance {
                    state: AssuranceState::Stale,
                    reason: Some(StaleReason::CrossFileResolutionNeeded),
                    generation: 3,
                    last_full_scan: None,
                    scan_coverage: None,
                },
                outcome,
            };
            let back: GctxImpactOfChangeResponse =
                serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
            assert_eq!(response, back);
        }
    }

    /// GCTX-013 / ADR-084: the affected-tests RPC method name is frozen and the
    /// request/response envelopes round-trip, including the named non-`ready`
    /// degradation outcome alongside the assurance snapshot.
    #[test]
    fn gctx_affected_tests_wire_round_trips() {
        assert_eq!(ANVIL_GCTX_AFFECTED_TESTS, "anvil/gctx/affected_tests");

        let request = GctxAffectedTestsRequest {
            workspace_root: "/home/me/proj".into(),
            query: AffectedTestsQuery {
                changed_files: vec!["src/a.ts".into(), "src/b.ts".into()],
                max_depth: Some(2),
            },
        };
        let back: GctxAffectedTestsRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(request, back);

        // A request may omit `query` entirely (serde default).
        let bare: GctxAffectedTestsRequest =
            serde_json::from_str(r#"{"workspace_root":"/p"}"#).unwrap();
        assert_eq!(bare.query, AffectedTestsQuery::default());

        for outcome in [
            AffectedTestsOutcome::Ready(anvil_gctx_types::AffectedTestsReport {
                tests: Vec::new(),
                coverage_gaps: Vec::new(),
                heuristic: true,
                summary: anvil_gctx_types::AffectedTestsSummary::default(),
            }),
            AffectedTestsOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            AffectedTestsOutcome::Unavailable,
            AffectedTestsOutcome::Disabled,
            AffectedTestsOutcome::InvalidQuery {
                reason: "changed_files exceeds the 200-file cap".into(),
            },
        ] {
            let response = GctxAffectedTestsResponse {
                workspace_assurance: WorkspaceAssurance {
                    state: AssuranceState::Stale,
                    reason: Some(StaleReason::CrossFileResolutionNeeded),
                    generation: 3,
                    last_full_scan: None,
                    scan_coverage: None,
                },
                outcome,
            };
            let back: GctxAffectedTestsResponse =
                serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
            assert_eq!(response, back);
        }
    }

    // ---- DSV-002 council follow-ups (2026-06-03 batch review) ----

    /// Forward-compat on the request side: a `ChangeDescriptor` with an extra
    /// unknown field — including alongside a **unit** change variant — still
    /// deserialises. This is the riskiest serde surface (a `#[serde(flatten)]`
    /// of an internally-tagged enum), so probe it with raw JSON, not just a
    /// serialiser round-trip.
    #[test]
    fn change_descriptor_tolerates_unknown_field_raw_json() {
        // Unit variant + unknown sibling key.
        let created: ChangeDescriptor = serde_json::from_str(
            r#"{"path":"src/lib.rs","change":"created","editor_context":"vscode"}"#,
        )
        .expect("unknown field on a unit-variant descriptor must deserialise");
        assert_eq!(created.change, ChangeKindWire::Created);
        assert_eq!(created.path, "src/lib.rs");

        // Struct variant (`renamed`) + unknown sibling key.
        let renamed: ChangeDescriptor = serde_json::from_str(
            r#"{"path":"src/new.rs","change":"renamed","from":"src/old.rs","tool":"git"}"#,
        )
        .expect("unknown field on a struct-variant descriptor must deserialise");
        assert_eq!(
            renamed.change,
            ChangeKindWire::Renamed {
                from: "src/old.rs".to_string()
            }
        );
    }

    /// Forward-compat on the nested object: an unknown field on
    /// `WorkspaceAssurance` (not just the top-level response) deserialises OK,
    /// so a newer daemon can grow the assurance object additively.
    #[test]
    fn workspace_assurance_tolerates_unknown_field() {
        let assurance: WorkspaceAssurance = serde_json::from_value(serde_json::json!({
            "state": "clean",
            "generation": 3,
            "warm_since": "2026-06-03T00:00:00Z"
        }))
        .expect("unknown field on WorkspaceAssurance must deserialise");
        assert_eq!(assurance.state, AssuranceState::Clean);
        assert_eq!(assurance.generation, 3);
        assert!(assurance.reason.is_none());
    }

    /// An unrecognised reason string from a newer daemon degrades to
    /// [`StaleReason::Unknown`] (fail-safe stale) rather than failing the
    /// whole parse — the `#[serde(other)]` forward-compat contract.
    #[test]
    fn unknown_stale_reason_string_degrades_to_unknown() {
        let parsed: StaleReason =
            serde_json::from_value(serde_json::Value::String("budget-exhausted".to_string()))
                .expect("an unrecognised reason must deserialise to the fallback");
        assert_eq!(parsed, StaleReason::Unknown);

        // And a known reason still maps to its named variant, not the fallback.
        let known: StaleReason =
            serde_json::from_value(serde_json::Value::String("scan-timeout".to_string())).unwrap();
        assert_eq!(known, StaleReason::ScanTimeout);

        // Inside the assurance envelope, too.
        let assurance: WorkspaceAssurance = serde_json::from_value(serde_json::json!({
            "state": "stale",
            "reason": "some-future-reason",
            "generation": 1
        }))
        .expect("a future reason inside WorkspaceAssurance must not fail the parse");
        assert_eq!(assurance.reason, Some(StaleReason::Unknown));
    }

    /// `EvaluatedPath.content_hash` is `None` for entries with no readable
    /// bytes (deleted / renamed-from), and skip-serialises when absent.
    #[test]
    fn evaluated_path_omits_hash_when_absent() {
        let deleted = EvaluatedPath {
            path: "src/gone.rs".to_string(),
            content_hash: None,
        };
        let json = serde_json::to_value(&deleted).unwrap();
        assert_eq!(json["path"], "src/gone.rs");
        assert!(
            json.get("content_hash").is_none(),
            "absent content_hash must skip-serialise, not emit null"
        );
        let back: EvaluatedPath = serde_json::from_value(json).unwrap();
        assert_eq!(back, deleted);
    }

    /// The `WorkspaceAssurance.reason` invariant round-trips for the two
    /// reason-bearing states: `Unavailable` carries `DaemonAbsent`, `Stale`
    /// carries its cause, and lifecycle states carry no reason.
    #[test]
    fn workspace_assurance_reason_states_round_trip() {
        let unavailable = WorkspaceAssurance {
            state: AssuranceState::Unavailable,
            reason: Some(StaleReason::DaemonAbsent),
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        };
        let back: WorkspaceAssurance =
            serde_json::from_str(&serde_json::to_string(&unavailable).unwrap()).unwrap();
        assert_eq!(back, unavailable);

        // A lifecycle state (running) carries no reason and skip-serialises it.
        let running = WorkspaceAssurance {
            state: AssuranceState::Running,
            reason: None,
            generation: 5,
            last_full_scan: Some("2026-06-03T12:00:00Z".to_string()),
            scan_coverage: None,
        };
        let json = serde_json::to_value(&running).unwrap();
        assert_eq!(json["state"], "running");
        assert!(json.get("reason").is_none());
        let back: WorkspaceAssurance = serde_json::from_value(json).unwrap();
        assert_eq!(back, running);
    }

    /// DSV-045: `AssuranceState::Bounded` round-trips on the wire string
    /// `"bounded"` — distinct from `Coverage::Partial`'s `"partial"`.
    #[test]
    fn bounded_state_round_trips_distinct_from_partial() {
        assert_eq!(
            serde_json::to_value(AssuranceState::Bounded).unwrap(),
            serde_json::Value::String("bounded".to_string()),
        );
        let back: AssuranceState =
            serde_json::from_value(serde_json::Value::String("bounded".to_string())).unwrap();
        assert_eq!(back, AssuranceState::Bounded);
        // The unrelated Coverage axis keeps its own wire string; the two do not
        // collide.
        assert_eq!(
            serde_json::to_value(Coverage::Partial).unwrap(),
            serde_json::Value::String("partial".to_string()),
        );
    }

    /// DSV-045 / ADR-085 Decision 5b: an unrecognised *state* string from a
    /// newer daemon degrades to [`AssuranceState::Unknown`] (fail-safe) rather
    /// than hard-failing the whole `WorkspaceAssurance` parse. This is the
    /// affordance that makes `Bounded` (and any future state) additive on a
    /// shipped v0.8.0-beta client instead of breaking.
    #[test]
    fn unknown_assurance_state_string_degrades_to_unknown() {
        let parsed: AssuranceState =
            serde_json::from_value(serde_json::Value::String("some-future-state".to_string()))
                .expect("an unrecognised state must deserialise to the fallback");
        assert_eq!(parsed, AssuranceState::Unknown);

        // The whole envelope still parses when a newer daemon sends an unknown
        // state — the regression ADR-085 exists to prevent.
        let assurance: WorkspaceAssurance = serde_json::from_value(serde_json::json!({
            "state": "warming-up-from-snapshot",
            "generation": 9
        }))
        .expect("a future state inside WorkspaceAssurance must not fail the parse");
        assert_eq!(assurance.state, AssuranceState::Unknown);
    }

    /// DSV-045 / ADR-085 Decision 5c: a `Bounded` snapshot carries
    /// `scan_coverage` and (like the lifecycle states) no `reason`; the
    /// coverage skip-serialises when absent so it stays forward-compatible.
    #[test]
    fn bounded_snapshot_carries_scan_coverage_and_no_reason() {
        let bounded = WorkspaceAssurance {
            state: AssuranceState::Bounded,
            reason: None,
            generation: 4,
            last_full_scan: Some("2026-06-16T00:00:00Z".to_string()),
            scan_coverage: Some(ScanCoverage {
                scanned_files: 100_000,
                total_files: 250_000,
            }),
        };
        let json = serde_json::to_value(&bounded).unwrap();
        assert_eq!(json["state"], "bounded");
        assert!(json.get("reason").is_none(), "Bounded carries no reason");
        assert_eq!(json["scan_coverage"]["scanned_files"], 100_000);
        assert_eq!(json["scan_coverage"]["total_files"], 250_000);
        let back: WorkspaceAssurance = serde_json::from_value(json).unwrap();
        assert_eq!(back, bounded);

        // scan_coverage skip-serialises when absent (the common Clean case).
        let clean = WorkspaceAssurance {
            state: AssuranceState::Clean,
            reason: None,
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        };
        let clean_json = serde_json::to_value(&clean).unwrap();
        assert!(
            clean_json.get("scan_coverage").is_none(),
            "absent scan_coverage must skip-serialise, not emit null"
        );
    }
}
