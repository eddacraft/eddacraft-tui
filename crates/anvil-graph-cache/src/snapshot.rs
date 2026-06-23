//! Sealed, allowlist-only snapshot DTO + `postcard` codec for the warm graph
//! cache (GV2-030, [ADR-069](../../../plans/decisions/069-graph-v2-persistence.md)).
//!
//! This module turns the ADR-069 §8 privacy line — *persist structural identity
//! only, never source text* — from a review convention into a **compile-time +
//! unit-tested property**. The on-disk artefact is a hand-authored allowlist DTO
//! ([`SnapshotPayload`]) whose graph rows are **projected field-by-field** from
//! the live graph types via destructuring ([`SnapshotNode::project`] /
//! [`SnapshotEdge::project`]): adding a field to [`SymbolNode`] / [`SymbolEdge`]
//! for an unrelated reason **fails to compile here** until someone consciously
//! decides whether it belongs in a snapshot. There is no `serde(flatten)`, no
//! `Vec<u8>`/`Bytes`, no `serde(other)` catch-all, and no [`crate::GraphDelta`]
//! field — the smuggling channel does not compile in.
//!
//! # Envelope + integrity (ADR-069 §1 — validated at load, before accepting)
//!
//! [`SnapshotPayload::to_bytes`] frames the postcard body behind a fixed
//! [`HEADER_LEN`]-byte header: magic, `format_version`, `backing_schema_version`,
//! node-count, edge-count, and a **CRC-32** of the body. [`SnapshotPayload::from_bytes`]
//! validates **magic → versions → checksum → decode → counts** and returns a
//! typed [`SnapshotLoadError`] on any mismatch (the daemon discards and
//! cold-rebuilds — never panics, never accepts partially-validated indexes). The
//! checksum + counts close the gap ADR-069 §1 calls out: a single-bit corruption
//! that still decodes cleanly (e.g. an enum-discriminant shift) would otherwise
//! survive the content-hash reconcile unchallenged.
//!
//! **CRC-32 is a corruption check, NOT an integrity/authenticity guarantee
//! (CIB-092 / N6).** It is not collision-resistant: a same-uid writer to the
//! graph-cache dir can forge a body with a valid CRC. The accepted control is
//! the machine-local, owner-only, same-uid persistence boundary (snapshots are
//! default-off, `0600`, written under a `0700` state dir) — see PV-12. Do not
//! treat a valid CRC as proof the snapshot was written by this daemon. Any move
//! off that boundary (shared CI container, off-machine transfer) needs a
//! keyed/strong digest and a fresh privacy review before it is relied upon.
//!
//! # The allowlist (PV-6 / PV-7d — every `String`-typed field, and why each is
//! identity-only)
//!
//! [`SnapshotPayload`] carries `String`s in exactly these positions, all of which
//! are *structural identity*, never source bodies/snippets/literal values:
//!
//! - [`SnapshotNode::name`] (projected from [`SymbolNode::name`]) — symbol /
//!   qualified name; the `dependents_of` reverse index is keyed on it (cannot be
//!   hashed without breaking impact analysis, verdict N-2). Methods encode their
//!   owner as `Owner.method`.
//! - [`SnapshotNode::file`] — the symbol's **workspace-root-relative** path
//!   identity. Relativity is enforced at build time by [`SnapshotNode::project`]
//!   and re-asserted structurally by the no-leak test — never left to convention.
//! - The per-file id-ordering keys ([`SnapshotPayload`] `file_symbol_order`.0) —
//!   the same relative file paths, also relativity-checked.
//! - The dependency-edge endpoints — relative file-path identity (forward edges
//!   only; the reverse index is rebuilt on load, ADR-069 §1).
//!
//! All other fields are integers or sealed enums ([`SymbolKind`], [`Visibility`],
//! [`EdgeType`], [`TrustLevel`]). Any span type is the no-text [`ByteRange`]
//! (PV-7e) — structurally incapable of holding the spanned text; the v1 payload
//! carries no spans at all.
//!
//! # What is deliberately absent (PV-6 / PV-7c)
//!
//! No session, worktree, attribution, provenance, plan-ref, or trust-posture
//! field — and **no [`crate::GraphDelta`]** (its `errors: Vec<String>` and
//! `previously_*` baseline sets embed `file::kind::name` concatenations). The v1
//! DTO is the **semantic + dependency graph only**. GV2-013 (session/control
//! graph) and GV2-014 (plan/provenance graph) each require their own privacy ADR
//! before their graphs become persistable (spec condition C-6); this DTO must not
//! be "conveniently" extended to carry them.
//!
//! # PV-9 — machine-local boundary (a gate, not a note)
//!
//! Identity keys in this snapshot (cleartext symbol names, import specifiers,
//! relative paths) are **machine-local**. Any feature that exports, syncs, or
//! transmits snapshot bytes off the originating machine requires a **new privacy
//! review before that export surface ships**. The cleartext residual is accepted
//! for default-on graduation *only within* the same-uid, owner-only,
//! machine-local boundary (ADR-069 §8 residual-risk note); no scrub pass is
//! required for v1.
//!
//! # PV-12 — residual risks, named honestly (boundary-erosion paths)
//!
//! The same-uid trust boundary that makes the cleartext residual acceptable is
//! eroded by, and these are accepted-and-named, not papered over:
//! - backup tools, dotfile syncers, and cloud-synced home directories picking up
//!   `~/.local/state/anvil/graph-cache/`;
//! - CI / containerised `ANVIL_HOME` mounts readable by orchestrators;
//! - unsalted hashes (the snapshot-filename hash here) being cross-machine
//!   **correlatable** — the same exposure class as git blob hashes.
//!
//! Toggling [`persist_graph_enabled`] off does **not** delete existing snapshots
//! (they go inert until re-enabled); deleting anything under `graph-cache/` is a
//! safe operator escape hatch (the daemon cold-rebuilds the affected key).

use std::collections::BTreeSet;
use std::path::Path;

use anvil_kernel_types::{
    ByteRange, EdgeType, SymbolEdge, SymbolKind, SymbolNode, TrustLevel, Visibility,
};

use crate::dependency::DependencyGraph;
use crate::symbol_graph::SymbolGraph;

/// Sealed magic prefix identifying an Anvil graph-cache snapshot (ADR-069 §1).
/// `GC1` = graph-cache, generation 1. A mismatched prefix ⇒
/// [`SnapshotLoadError::BadMagic`] ⇒ cold rebuild (never a panic).
pub const SNAPSHOT_MAGIC: [u8; 8] = *b"ANVILGC1";

/// Envelope / codec version (ADR-069 §6 `format_version`). Bumped when the
/// envelope framing or codec changes. A mismatch ⇒
/// [`SnapshotLoadError::VersionMismatch`] ⇒ cold rebuild.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// Backing index-schema generation (ADR-069 §6 `schema_epoch` /
/// `backing_schema_version`). Bumped when the resident `SymbolGraph` backing is
/// swapped, invalidating all pre-swap snapshots. No migration code is ever
/// written — a mismatch is one cold rebuild ([`SnapshotLoadError::VersionMismatch`]).
///
/// CIB-093e: bump this ONLY when the persisted snapshot DTO layout changes — it
/// is deliberately independent of [`crate::incremental::GRAPH_DELTA_SCHEMA_VERSION`].
/// A delta-wire bump that leaves the on-disk DTO unchanged must NOT invalidate
/// every warm-start snapshot (which would force needless cold rebuilds), so this
/// version owns its own value. It starts at `1` — the value it effectively had
/// when aliased to the delta-wire version — so existing snapshots stay valid and
/// the committed golden wire bytes do not shift.
pub const SNAPSHOT_BACKING_SCHEMA_VERSION: u32 = 2;

/// Fixed snapshot-header length, in bytes: magic(8) + `format_version`(4) +
/// `backing_schema_version`(4) + node-count(8) + edge-count(8) + CRC-32(4). The
/// header is hand-framed (not postcard) so magic/version/checksum/counts can be
/// validated **before** the body is decoded or trusted (ADR-069 §1).
pub const HEADER_LEN: usize = 8 + 4 + 4 + 8 + 8 + 4;

/// Hard upper bound on an accepted snapshot, in bytes (ADR-069 §4 bounded load).
/// Over this cap ⇒ [`SnapshotLoadError::Oversized`] ⇒ cold rebuild, so a
/// crafted/oversized length cannot drive an allocation bomb. 256 MiB is far above
/// any realistic warm-cache payload while still bounding the worst case.
pub const MAX_SNAPSHOT_BYTES: usize = 256 * 1024 * 1024;

/// Typed snapshot-load failure. **Every variant maps to "discard and
/// cold-rebuild"** (ADR-069 §3) — the daemon never panics and never refuses to
/// start on a bad snapshot.
///
/// Privacy (PV-10 / verdict N-3): variants carry **no `String` payloads and no
/// decoded field values** — only numeric framing metadata. The only telemetry
/// label a counter may bind is the **variant name** (`BadMagic`,
/// `VersionMismatch`, `ChecksumMismatch`, `CountMismatch`, `Oversized`,
/// `Corrupt`); never a `WorktreeKey` path, absolute path, or symbol name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotLoadError {
    /// Fewer than [`HEADER_LEN`] bytes, or the magic prefix did not match
    /// [`SNAPSHOT_MAGIC`].
    #[error("snapshot magic prefix mismatch")]
    BadMagic,
    /// The envelope `format_version` or `backing_schema_version` did not match
    /// this build's constants. Carries only the two numeric pairs — no strings.
    #[error("snapshot version mismatch")]
    VersionMismatch {
        /// `format_version` found in the envelope.
        found_format: u32,
        /// `format_version` this build expects.
        expected_format: u32,
        /// `backing_schema_version` found in the envelope.
        found_backing: u32,
        /// `backing_schema_version` this build expects.
        expected_backing: u32,
    },
    /// The body CRC-32 did not match the header (ADR-069 §1 integrity gate) — a
    /// corruption that may still decode cleanly. Carries no bytes.
    #[error("snapshot checksum mismatch")]
    ChecksumMismatch,
    /// The decoded node/edge counts did not match the header counts (ADR-069 §1)
    /// — e.g. a truncation the codec tolerated. Numeric only, no field values.
    #[error("snapshot node/edge count mismatch")]
    CountMismatch {
        /// Node count declared in the header.
        expected_nodes: u64,
        /// Node count actually decoded.
        found_nodes: u64,
        /// Edge count declared in the header.
        expected_edges: u64,
        /// Edge count actually decoded.
        found_edges: u64,
    },
    /// The declared/encoded body length exceeds [`MAX_SNAPSHOT_BYTES`].
    #[error("snapshot exceeds the maximum accepted size")]
    Oversized,
    /// The body was truncated, torn, or otherwise not decodable by the codec.
    /// Carries no decoder message (which could echo bytes) — only the variant.
    #[error("snapshot body is corrupt or truncated")]
    Corrupt,
}

/// Failure building a [`SnapshotPayload`] from live graphs. Distinct from
/// [`SnapshotLoadError`] (the *read* path). A build failure means the resident
/// state held a path that violates the workspace-root-relative contract —
/// surfaced rather than silently persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotBuildError {
    /// A path-bearing field was not workspace-root-relative (absolute, a `..`
    /// escape, leading whitespace, or a drive/UNC-rooted Windows path). No path
    /// string is carried — the offending value is identity text the error must
    /// not echo into logs.
    #[error("snapshot contains a non-workspace-root-relative path")]
    NonRelativePath,
}

/// Allowlisted projection of a semantic-graph node (PV-7a/d).
///
/// Built field-by-field from [`SymbolNode`] by destructuring in
/// [`SnapshotNode::project`] — so a **new [`SymbolNode`] field is a compile error
/// here** until it is consciously added to (or excluded from) the snapshot. This
/// is the structural seal: a live-type field cannot reach the on-disk artefact by
/// accident. Every field below is an integer, a sealed enum, or identity text.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SnapshotNode {
    id: u64,
    kind: SymbolKind,
    name: String,
    visibility: Visibility,
    file: String,
    trust_level: TrustLevel,
    /// GV2-032 source byte span — offsets only, never text (PV-7(e)), so it is
    /// allowlist-safe to persist. Persisting it lets a warm-started daemon serve
    /// snippets without a re-scan. **No `skip_serializing_if`**: the snapshot
    /// codec is `postcard` (non-self-describing), so the field must always be
    /// encoded (a `None` is one tag byte) or the decoder desyncs. `serde(default)`
    /// is a no-op for the postcard path (it cannot fill an absent trailing
    /// field) — it is kept only for the `serde_json` lens the no-leak test uses;
    /// on-disk forward/backward compat is governed solely by
    /// `SNAPSHOT_BACKING_SCHEMA_VERSION`, so any new field needs a version bump.
    #[serde(default)]
    span: Option<ByteRange>,
}

impl SnapshotNode {
    /// Project a live node, validating its `file` is workspace-root-relative.
    fn project(node: &SymbolNode) -> Result<Self, SnapshotBuildError> {
        // Exhaustive destructure (no `..`): adding a field to `SymbolNode` breaks
        // this line, forcing a reviewed decision rather than a silent leak.
        let SymbolNode {
            id,
            kind,
            name,
            visibility,
            file,
            trust_level,
            span,
        } = node.clone();
        check_relative(&file)?;
        Ok(Self {
            id,
            kind,
            name,
            visibility,
            file,
            trust_level,
            span,
        })
    }

    /// Reconstruct the live node (round-trip inverse of [`Self::project`]).
    fn into_node(self) -> SymbolNode {
        let SnapshotNode {
            id,
            kind,
            name,
            visibility,
            file,
            trust_level,
            span,
        } = self;
        SymbolNode {
            id,
            kind,
            name,
            visibility,
            file,
            trust_level,
            span,
        }
    }
}

/// Allowlisted projection of a semantic-graph edge — same compile-time seal as
/// [`SnapshotNode`]. All fields are integers or the sealed [`EdgeType`] enum.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SnapshotEdge {
    from: u64,
    to: u64,
    edge_type: EdgeType,
}

impl SnapshotEdge {
    fn project(edge: &SymbolEdge) -> Self {
        // Clone (not `*edge`) so the exhaustive destructure holds even if a future
        // `SymbolEdge` field stops being `Copy` — and to mirror `SnapshotNode::project`.
        let SymbolEdge {
            from,
            to,
            edge_type,
        } = edge.clone();
        Self {
            from,
            to,
            edge_type,
        }
    }

    fn into_edge(self) -> SymbolEdge {
        let SnapshotEdge {
            from,
            to,
            edge_type,
        } = self;
        SymbolEdge {
            from,
            to,
            edge_type,
        }
    }
}

/// The sealed, allowlist-only graph-cache snapshot body (ADR-069 §1, GV2-030).
///
/// Semantic + dependency graph **only** (PV-6). Graph rows are projected through
/// [`SnapshotNode`] / [`SnapshotEdge`] (compile-time seal), [`crate::GraphDelta`]
/// is **not** a field, and there is no opaque byte channel. See the module docs
/// for the per-field allowlist rationale.
///
/// Ordering is deterministic (every collection is sorted in
/// [`SnapshotPayload::from_graphs`]) so [`SnapshotPayload::to_bytes`] is
/// reproducible for a given logical graph — the property the ADR-069 §6 golden
/// round-trip fixture relies on.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotPayload {
    /// Semantic graph nodes, sorted by `id`. `String` fields (`name`, `file`)
    /// are identity-only; see module docs.
    nodes: Vec<SnapshotNode>,
    /// Semantic graph edges, sorted by `(from, to, edge_type discriminant)`.
    edges: Vec<SnapshotEdge>,
    /// Per-file symbol-id ordering, `(relative_file, ids_in_parse_order)`, sorted
    /// by file. Preserves the parse-order `SymbolIdentity` ordinals depend on,
    /// which a node-only set sorted by id would lose.
    file_symbol_order: Vec<(String, Vec<u64>)>,
    /// One past the largest symbol id ever inserted (`SymbolGraph::next_id`), so
    /// post-load synthetic-node allocation cannot collide with restored ids.
    next_id: u64,
    /// Dependency graph forward edges, `(source_relative_file,
    /// sorted_target_relative_files)`, sorted by source. **Forward only** — the
    /// reverse index is rebuilt on load (ADR-069 §1). All strings are relative
    /// file-path identity.
    dependency_edges: Vec<(String, Vec<String>)>,
    /// GV2-032 per-file content-freshness keys, `(relative_file, content_hash)`,
    /// sorted by file. Offsets-free integer digests (PV-7(e)-safe) so a
    /// warm-started daemon can serve snippets (CE-7) without a re-scan. The
    /// `serde(default)` is for the `serde_json` lens only; on the postcard path a
    /// pre-GV2-032 (v1) snapshot is rejected by the `SNAPSHOT_BACKING_SCHEMA_VERSION`
    /// 1→2 bump before decode, never defaulted in.
    #[serde(default)]
    file_hashes: Vec<(String, u64)>,
}

/// Structural equality via the canonical wire bytes. The encoding is
/// deterministic ([`SnapshotPayload::from_graphs`] sorts every collection), so
/// byte-equality is exactly structural equality. Used by the golden round-trip
/// test.
impl PartialEq for SnapshotPayload {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for SnapshotPayload {}

impl SnapshotPayload {
    /// Build a payload from live graphs, validating every path is
    /// workspace-root-relative and sorting all collections deterministically.
    ///
    /// # Errors
    /// [`SnapshotBuildError::NonRelativePath`] if any symbol `file`, file-order
    /// key, or dependency endpoint is absolute, contains a `..` escape, has
    /// leading whitespace, or is drive/UNC-rooted.
    pub fn from_graphs(
        sym: &SymbolGraph,
        dep: &DependencyGraph,
    ) -> Result<Self, SnapshotBuildError> {
        // --- nodes (projected, sorted by id) ---
        let mut nodes: Vec<SnapshotNode> = sym
            .inner()
            .node_weights()
            .map(SnapshotNode::project)
            .collect::<Result<_, _>>()?;
        nodes.sort_by_key(|n| n.id);

        // --- edges (projected, sorted by (from, to, edge discriminant)) ---
        let mut edges: Vec<SnapshotEdge> = sym
            .inner()
            .edge_weights()
            .map(SnapshotEdge::project)
            .collect();
        edges.sort_by_key(|e| (e.from, e.to, edge_sort_key(e)));

        // --- per-file id ordering (sorted by file; ids keep parse order) ---
        let files: BTreeSet<&str> = nodes.iter().map(|n| n.file.as_str()).collect();
        let file_symbol_order: Vec<(String, Vec<u64>)> = files
            .into_iter()
            .map(|f| {
                let ids = sym.symbols_in_file(f).iter().map(|s| s.id).collect();
                (f.to_owned(), ids)
            })
            .collect();
        // `files` is a BTreeSet over already-relativity-checked node paths, so the
        // keys are sorted and relative by construction.

        // --- dependency forward edges (sorted by source; targets sorted) ---
        let mut dependency_edges: Vec<(String, Vec<String>)> = dep
            .forward_edges()
            .map(|(src, targets)| {
                let mut t: Vec<String> = targets.iter().cloned().collect();
                t.sort();
                (src.to_owned(), t)
            })
            .collect();
        dependency_edges.sort_by(|a, b| a.0.cmp(&b.0));
        for (src, targets) in &dependency_edges {
            check_relative(src)?;
            for t in targets {
                check_relative(t)?;
            }
        }

        // --- GV2-032 per-file content-freshness keys (sorted by file) ---
        let mut file_hashes: Vec<(String, u64)> = sym
            .file_hashes()
            .iter()
            .map(|(f, h)| (f.clone(), *h))
            .collect();
        file_hashes.sort_by(|a, b| a.0.cmp(&b.0));
        for (f, _) in &file_hashes {
            check_relative(f)?;
        }

        Ok(Self {
            nodes,
            edges,
            file_symbol_order,
            next_id: sym.next_id(),
            dependency_edges,
            file_hashes,
        })
    }

    /// Serialise to the sealed binary form: a fixed [`HEADER_LEN`]-byte header
    /// (magic, versions, node/edge counts, body CRC-32) followed by the postcard
    /// body (ADR-069 §1).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        // `postcard` serialisation of this fixed allowlist DTO (only integers,
        // sealed enums, and identity strings) cannot fail; the `expect` documents
        // that invariant. (OOM aborts the process — it is not a returnable error
        // on any codec path.)
        let body = postcard::to_stdvec(self).expect("sealed snapshot DTO is always serialisable");
        let crc = crc32(&body);
        let node_count = self.nodes.len() as u64;
        let edge_count = self.edges.len() as u64;

        let mut out = Vec::with_capacity(HEADER_LEN + body.len());
        out.extend_from_slice(&SNAPSHOT_MAGIC);
        out.extend_from_slice(&SNAPSHOT_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&SNAPSHOT_BACKING_SCHEMA_VERSION.to_le_bytes());
        out.extend_from_slice(&node_count.to_le_bytes());
        out.extend_from_slice(&edge_count.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    /// Decode + validate a snapshot. Returns a typed error on any anomaly;
    /// **never panics** (ADR-069 §3).
    ///
    /// Validation order (ADR-069 §1): size cap → header present → magic →
    /// versions → **body checksum** → decode → **node/edge counts**. Each step
    /// short-circuits before any field is trusted.
    ///
    /// # Errors
    /// - [`SnapshotLoadError::Oversized`] if `bytes` exceeds [`MAX_SNAPSHOT_BYTES`].
    /// - [`SnapshotLoadError::BadMagic`] if too short, or on a magic mismatch.
    /// - [`SnapshotLoadError::VersionMismatch`] on a format/backing mismatch.
    /// - [`SnapshotLoadError::ChecksumMismatch`] if the body CRC-32 disagrees.
    /// - [`SnapshotLoadError::Corrupt`] if the codec cannot decode the body.
    /// - [`SnapshotLoadError::CountMismatch`] if decoded counts ≠ header counts.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SnapshotLoadError> {
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(SnapshotLoadError::Oversized);
        }
        if bytes.len() < HEADER_LEN {
            // Too short to carry a header — treat as a wrong/empty file.
            return Err(SnapshotLoadError::BadMagic);
        }
        let (header, body) = bytes.split_at(HEADER_LEN);

        if header[0..8] != SNAPSHOT_MAGIC {
            return Err(SnapshotLoadError::BadMagic);
        }
        let found_format = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        let found_backing = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
        if found_format != SNAPSHOT_FORMAT_VERSION
            || found_backing != SNAPSHOT_BACKING_SCHEMA_VERSION
        {
            return Err(SnapshotLoadError::VersionMismatch {
                found_format,
                expected_format: SNAPSHOT_FORMAT_VERSION,
                found_backing,
                expected_backing: SNAPSHOT_BACKING_SCHEMA_VERSION,
            });
        }
        let expected_nodes = u64::from_le_bytes([
            header[16], header[17], header[18], header[19], header[20], header[21], header[22],
            header[23],
        ]);
        let expected_edges = u64::from_le_bytes([
            header[24], header[25], header[26], header[27], header[28], header[29], header[30],
            header[31],
        ]);
        let found_crc = u32::from_le_bytes([header[32], header[33], header[34], header[35]]);

        // Checksum the raw body BEFORE decoding — catches a corruption that the
        // codec would otherwise tolerate into a structurally-valid wrong graph.
        if crc32(body) != found_crc {
            return Err(SnapshotLoadError::ChecksumMismatch);
        }

        let payload: Self = postcard::from_bytes(body).map_err(|_| SnapshotLoadError::Corrupt)?;

        let found_nodes = payload.nodes.len() as u64;
        let found_edges = payload.edges.len() as u64;
        if found_nodes != expected_nodes || found_edges != expected_edges {
            return Err(SnapshotLoadError::CountMismatch {
                expected_nodes,
                found_nodes,
                expected_edges,
                found_edges,
            });
        }

        Ok(payload)
    }

    /// Rebuild the live graph pair from a decoded payload by **replaying inserts**
    /// (ADR-069 §1) — petgraph re-derives its own ephemeral `NodeIndex`es and the
    /// dependency reverse index is rebuilt from the forward edges. No on-disk
    /// `NodeIndex` is ever trusted (the bug is gone by construction). The
    /// persisted `next_id` high-water mark is restored as a floor so post-load
    /// synthetic ids cannot collide with ids the original session already spent
    /// (even ones whose nodes were later removed).
    ///
    /// # Errors
    /// [`SnapshotLoadError::Corrupt`] if the replayed body is internally
    /// inconsistent (a duplicate id or a dangling edge endpoint) — treated as a
    /// corrupt snapshot, i.e. cold-rebuild, never a panic.
    pub fn into_graphs(self) -> Result<(SymbolGraph, DependencyGraph), SnapshotLoadError> {
        let mut sym = SymbolGraph::new();
        for node in self.nodes {
            sym.add_symbol(node.into_node())
                .map_err(|_| SnapshotLoadError::Corrupt)?;
        }
        for edge in self.edges {
            sym.add_edge(edge.into_edge())
                .map_err(|_| SnapshotLoadError::Corrupt)?;
        }
        // Restore the high-water mark (M-2): replay alone only reaches
        // max(surviving_id)+1, which under-restores after removals.
        sym.set_next_id_floor(self.next_id);
        // GV2-032: restore the per-file content-freshness keys so a warm-started
        // daemon can serve snippets (CE-7) without waiting for a re-save.
        for (file, hash) in self.file_hashes {
            sym.set_file_hash(file, Some(hash));
        }

        let mut dep = DependencyGraph::new();
        for (src, targets) in self.dependency_edges {
            dep.set_dependencies(&src, targets);
        }

        Ok((sym, dep))
    }
}

/// Order key for an edge's `edge_type`, so the edge sort is total + deterministic
/// without requiring `EdgeType: Ord` from the kernel-types crate.
fn edge_sort_key(edge: &SnapshotEdge) -> u8 {
    use anvil_kernel_types::EdgeType::{Calls, Contains, Imports, Reexports, References};
    match edge.edge_type {
        Contains => 0,
        References => 1,
        Calls => 2,
        Imports => 3,
        Reexports => 4,
    }
}

/// Reject any path that is not workspace-root-relative.
fn check_relative(path: &str) -> Result<(), SnapshotBuildError> {
    if is_workspace_root_relative(path) {
        Ok(())
    } else {
        Err(SnapshotBuildError::NonRelativePath)
    }
}

/// Pure predicate behind [`check_relative`] — exposed for the no-leak test's
/// structural scan, which applies the same rule to every serialised string.
///
/// Rejects: an empty path; a path with **leading whitespace** (which could hide a
/// rooted path from a naive prefix check); a `/`-rooted POSIX path; a `\`-rooted
/// or drive/UNC Windows path (`C:\…`, `\\server\…`); and any path containing a
/// `..` segment (a relative escape above the root). Splits on both separators so
/// the check is cross-platform regardless of which separator the producer emitted.
#[must_use]
pub fn is_workspace_root_relative(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    // Leading whitespace could mask a rooted path from the prefix checks below.
    if path.starts_with(|c: char| c.is_whitespace()) {
        return false;
    }
    // POSIX-rooted, Windows-rooted (`\foo`), or UNC (`\\server`).
    if path.starts_with('/') || path.starts_with('\\') {
        return false;
    }
    // Drive-letter root: `C:` / `C:\` / `c:/`.
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return false;
    }
    // `..` escape, on either separator.
    for segment in path.split(['/', '\\']) {
        if segment == ".." {
            return false;
        }
    }
    true
}

/// Derive the snapshot filename for a workspace from its **canonical root path**
/// (PV-8, ADR-069 §2). A named, stable, dependency-free **128-bit FNV-1a** digest
/// of the canonical root's **raw OS bytes**, rendered as 32 lowercase hex chars +
/// `.snap` — never the rendered path, never the default (randomly-seeded) hasher.
/// Worktree identity persists *only* as this filename key-hash; no absolute path
/// appears in the result.
///
/// The hash needs only to be **stable** (same root → same filename across runs and
/// releases) and **collision-resistant enough** to disambiguate the handful of
/// workspace roots one machine sees — not cryptographic. The former SHA-256 carried
/// a `sha2` (+ `cpufeatures`/`libc`) dependency on this deliberately-lean crate
/// (ADR-064) for **no crypto benefit**: the digest is unsalted, so it never
/// resisted correlation (PV-12). FNV-1a hand-rolled keeps the property that
/// matters with zero dependencies (CIB-092e). 128 bits (two independently-seeded
/// FNV-1a lanes) keeps the birthday-collision probability negligible across any
/// realistic worktree count, far better than the 64-bit single-lane minimum.
///
/// Hashes [`std::ffi::OsStr::as_encoded_bytes`] (not `to_string_lossy`) so two
/// distinct non-UTF-8 roots cannot lossy-collapse to the same filename (a
/// Linux-only collision risk).
///
/// Takes a `&Path` rather than a `WorktreeKey` so this crate does not depend on
/// `anvil-intercept` (ADR-064). The caller passes the already-canonicalised root.
///
/// Note (PV-12): the hash is unsalted and therefore cross-machine correlatable —
/// the same exposure class as a git blob hash, accepted under the machine-local
/// boundary (see module docs). The same property held for the prior SHA-256.
#[must_use]
pub fn snapshot_filename(canonical_workspace_root: &Path) -> String {
    let bytes = canonical_workspace_root.as_os_str().as_encoded_bytes();
    // Two independently-seeded FNV-1a lanes → a 128-bit digest. The second lane
    // is offset-seeded so it is not a trivial function of the first.
    let hi = fnv1a64(bytes, FNV_OFFSET_BASIS);
    let lo = fnv1a64(bytes, FNV_OFFSET_BASIS ^ 0x517c_c1b7_2722_0a95);
    format!("{hi:016x}{lo:016x}.snap")
}

/// FNV-1a 64-bit prime (`0x100000001b3`).
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
/// FNV-1a 64-bit offset basis (`0xcbf29ce484222325`).
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// One FNV-1a 64-bit lane over `bytes`, starting from `seed`. Pure, stable across
/// releases (unlike `std::hash::DefaultHasher`, which is `SipHash` with a
/// process-random key and is explicitly **not** persistence-stable).
fn fnv1a64(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Whether warm-graph persistence is enabled (PV-11, ADR-069 §7) — **default
/// off, fail-closed**. Only an affirmative value (`1`, `true`, `yes`, `on`,
/// case-insensitive) enables; anything else — unset, empty, unparseable, or a
/// negative — resolves to off, matching the project's "no silent defaults /
/// config-load-failure fails closed" discipline.
///
/// This is the gate's pure core. A write/load pipeline (out of GV2-030 scope —
/// the daemon owns timing, ADR-069 §9) MUST early-return when this is `false`, so
/// with the flag unset no snapshot write or `graph-cache/` dir creation happens
/// (ADR-069 §7 asserts byte-for-byte today's rebuild-on-restart behaviour). The
/// catalogue entry is `flags/manifest.json` key `daemon.persist-graph`
/// (defaultVariant `disabled`).
#[must_use]
pub fn persist_graph_enabled(raw: Option<&str>) -> bool {
    matches!(
        raw.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// CRC-32 (IEEE 802.3 / ISO-HDLC, polynomial `0xEDB8_8320`, reflected) of `data`.
///
/// Hand-rolled to keep the integrity gate dependency-free (this crate's gates
/// scrutinise every new dependency); the well-known check value `0xCBF4_3926`
/// for `b"123456789"` is asserted in the tests, pinning correctness.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            // Branchless: `mask` is all-ones iff the low bit is set.
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{EdgeType, SymbolKind, TrustLevel, Visibility};

    /// Committed ADR-069 §6 golden wire bytes for [`golden_fixture`]. Regenerated
    /// deliberately on a wire-format change (see `snapshot_wire_bytes_match_committed_golden`).
    /// 281 bytes: the 36-byte header (magic/versions/counts/CRC) + the postcard body.
    /// CIB-092 council survivor (item 5): the fixture now exercises **every** variant
    /// of `SymbolKind`/`Visibility`/`TrustLevel`/`EdgeType`, so a postcard
    /// variant-reorder or insertion on any one of them shifts these bytes.
    #[rustfmt::skip]
    const GOLDEN_SNAPSHOT_BYTES: &[u8] = &[
        0x41, 0x4e, 0x56, 0x49, 0x4c, 0x47, 0x43, 0x31, 0x01, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12, 0x13, 0x05, 0xb2,
        0x09, 0x01, 0x00, 0x05, 0x61, 0x6c, 0x70, 0x68, 0x61, 0x00, 0x08, 0x73,
        0x72, 0x63, 0x2f, 0x61, 0x2e, 0x74, 0x73, 0x00, 0x00, 0x02, 0x00, 0x05,
        0x61, 0x6c, 0x70, 0x68, 0x61, 0x00, 0x08, 0x73, 0x72, 0x63, 0x2f, 0x61,
        0x2e, 0x74, 0x73, 0x01, 0x00, 0x03, 0x01, 0x06, 0x57, 0x69, 0x64, 0x67,
        0x65, 0x74, 0x00, 0x08, 0x73, 0x72, 0x63, 0x2f, 0x61, 0x2e, 0x74, 0x73,
        0x02, 0x00, 0x04, 0x02, 0x04, 0x6d, 0x6f, 0x64, 0x78, 0x01, 0x08, 0x73,
        0x72, 0x63, 0x2f, 0x61, 0x2e, 0x74, 0x73, 0x03, 0x00, 0x05, 0x03, 0x02,
        0x65, 0x78, 0x00, 0x08, 0x73, 0x72, 0x63, 0x2f, 0x61, 0x2e, 0x74, 0x73,
        0x04, 0x00, 0x06, 0x04, 0x05, 0x53, 0x68, 0x61, 0x70, 0x65, 0x01, 0x08,
        0x73, 0x72, 0x63, 0x2f, 0x62, 0x2e, 0x74, 0x73, 0x00, 0x00, 0x07, 0x05,
        0x05, 0x41, 0x6c, 0x69, 0x61, 0x73, 0x00, 0x08, 0x73, 0x72, 0x63, 0x2f,
        0x62, 0x2e, 0x74, 0x73, 0x01, 0x00, 0x08, 0x06, 0x06, 0x43, 0x6f, 0x6c,
        0x6f, 0x75, 0x72, 0x00, 0x08, 0x73, 0x72, 0x63, 0x2f, 0x62, 0x2e, 0x74,
        0x73, 0x02, 0x00, 0x09, 0x07, 0x0d, 0x57, 0x69, 0x64, 0x67, 0x65, 0x74,
        0x2e, 0x72, 0x65, 0x6e, 0x64, 0x65, 0x72, 0x01, 0x08, 0x73, 0x72, 0x63,
        0x2f, 0x62, 0x2e, 0x74, 0x73, 0x04, 0x00, 0x05, 0x01, 0x09, 0x02, 0x03,
        0x01, 0x00, 0x04, 0x06, 0x03, 0x05, 0x07, 0x04, 0x09, 0x01, 0x01, 0x02,
        0x08, 0x73, 0x72, 0x63, 0x2f, 0x61, 0x2e, 0x74, 0x73, 0x05, 0x01, 0x02,
        0x03, 0x04, 0x05, 0x08, 0x73, 0x72, 0x63, 0x2f, 0x62, 0x2e, 0x74, 0x73,
        0x04, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x01, 0x08, 0x73, 0x72, 0x63, 0x2f,
        0x61, 0x2e, 0x74, 0x73, 0x01, 0x08, 0x73, 0x72, 0x63, 0x2f, 0x62, 0x2e,
        0x74, 0x73, 0x00,
    ];

    fn node(id: u64, name: &str, file: &str, kind: SymbolKind) -> SymbolNode {
        SymbolNode {
            id,
            kind,
            name: name.to_owned(),
            visibility: Visibility::Public,
            file: file.to_owned(),
            trust_level: TrustLevel::Internal,
            span: None,
        }
    }

    /// A small but structurally varied fixture: two files, multiple symbols (incl.
    /// an overload pair so per-file ordering matters), several edge kinds, and a
    /// dependency graph with a shared target.
    fn fixture() -> (SymbolGraph, DependencyGraph) {
        let mut sym = SymbolGraph::new();
        sym.add_symbol(node(1, "alpha", "src/a.ts", SymbolKind::Function))
            .unwrap();
        sym.add_symbol(node(2, "alpha", "src/a.ts", SymbolKind::Function))
            .unwrap(); // overload — same (kind,name), ordinal 1
        sym.add_symbol(node(3, "Widget", "src/a.ts", SymbolKind::Class))
            .unwrap();
        sym.add_symbol(node(4, "render", "src/b.ts", SymbolKind::Function))
            .unwrap();
        sym.add_edge(SymbolEdge {
            from: 1,
            to: 4,
            edge_type: EdgeType::Calls,
        })
        .unwrap();
        sym.add_edge(SymbolEdge {
            from: 3,
            to: 1,
            edge_type: EdgeType::Contains,
        })
        .unwrap();
        sym.add_edge(SymbolEdge {
            from: 4,
            to: 3,
            edge_type: EdgeType::References,
        })
        .unwrap();

        let mut dep = DependencyGraph::new();
        dep.add_dependency("src/a.ts".to_owned(), "src/b.ts".to_owned());
        dep.add_dependency("src/a.ts".to_owned(), "src/shared.ts".to_owned());
        dep.add_dependency("src/b.ts".to_owned(), "src/shared.ts".to_owned());

        (sym, dep)
    }

    // ============================================================
    // (PV-7) The no-leak test — the heart of GV2-030.
    // ============================================================

    /// Walk every JSON string value reachable in the serialised payload and
    /// assert each is workspace-root-relative identity text (PV-7a/b). The
    /// `is_workspace_root_relative` predicate is the single gate — it already
    /// rejects absolute roots and `..` escapes, so no redundant per-shape
    /// assertions are layered on (those would false-reject a symbol name that
    /// legitimately contains `..`).
    #[test]
    fn snapshot_no_leak_paths_are_all_relative() {
        let (sym, dep) = fixture();
        let payload = SnapshotPayload::from_graphs(&sym, &dep).unwrap();

        // serde_json gives a walkable value tree; postcard is the on-disk codec,
        // but the *string set* is codec-independent, so JSON is the right lens for
        // a structural string scan.
        let json = serde_json::to_value(&payload).unwrap();
        let mut strings = Vec::new();
        collect_strings(&json, &mut strings);

        assert!(
            !strings.is_empty(),
            "fixture must contain identity strings to make the scan meaningful"
        );
        for s in &strings {
            assert!(
                is_workspace_root_relative(s),
                "leaked a non-relative / escaping string into the snapshot: {s:?}"
            );
        }
    }

    /// PV-7 by-construction guarantees, as executable assertions:
    /// - (a) **No `PathBuf` anywhere** — every path is a `String` (this binding
    ///   compiles only because the projected `file` is a `String`).
    /// - (c) **`GraphDelta` entirely absent** — not a field; no `errors` /
    ///   `previously_*`. The top-level key-set is asserted exactly.
    /// - (d) **Sub-field seal is now compile-time** — graph rows are projected
    ///   through `SnapshotNode`/`SnapshotEdge` by exhaustive destructure, so a new
    ///   `SymbolNode`/`SymbolEdge` field fails to compile here rather than leaking.
    /// - (e) The only span type is the no-text [`ByteRange`] on `SnapshotNode.span`
    ///   (GV2-032) — byte offsets, never source text — and the only digest is the
    ///   integer `file_hashes` content key; both are PV-7(e)-safe.
    #[test]
    fn snapshot_no_leak_by_construction() {
        let (sym, dep) = fixture();
        let payload = SnapshotPayload::from_graphs(&sym, &dep).unwrap();

        // (a) path data is String-typed (no PathBuf field exists to read).
        let file: &String = &payload.nodes[0].file;
        assert!(!file.is_empty());

        // (c) the serialised top-level keys are exactly the DTO body's — a new
        // leaky field would change this set and trip the assertion.
        let json = serde_json::to_value(&payload).unwrap();
        let obj = json.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "dependency_edges",
                "edges",
                "file_hashes",
                "file_symbol_order",
                "next_id",
                "nodes",
            ],
            "an unexpected top-level field appeared — review against the PV-6 allowlist"
        );
        for absent in [
            "errors",
            "previously_public",
            "previously_privileged",
            "previously_boundary",
            "previously_imported",
            "previously_reexported_privileged",
        ] {
            assert!(
                !obj.contains_key(absent),
                "delta-shaped field leaked: {absent}"
            );
        }
    }

    fn collect_strings(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(items) => {
                for item in items {
                    collect_strings(item, out);
                }
            }
            serde_json::Value::Object(map) => {
                for v in map.values() {
                    collect_strings(v, out);
                }
            }
            _ => {}
        }
    }

    // ============================================================
    // Golden round-trip + determinism (ADR-069 §6).
    // ============================================================

    #[test]
    fn snapshot_round_trip_preserves_structure() {
        let (sym, dep) = fixture();
        let payload = SnapshotPayload::from_graphs(&sym, &dep).unwrap();

        let bytes = payload.to_bytes();
        let decoded = SnapshotPayload::from_bytes(&bytes).unwrap();
        assert_eq!(payload, decoded, "payload must survive a byte round-trip");

        let (sym2, dep2) = decoded.into_graphs().unwrap();

        assert_eq!(sym.node_count(), sym2.node_count());
        assert_eq!(sym.edge_count(), sym2.edge_count());
        assert_eq!(sym.next_id(), sym2.next_id());
        for id in [1_u64, 2, 3, 4] {
            let a = sym.get_symbol(id).unwrap();
            let b = sym2.get_symbol(id).unwrap();
            assert_eq!(a.name, b.name);
            assert_eq!(a.file, b.file);
            assert_eq!(a.kind, b.kind);
        }
        let mut a_deps = dep.dependencies_of("src/a.ts");
        a_deps.sort_unstable();
        let mut a_deps2 = dep2.dependencies_of("src/a.ts");
        a_deps2.sort_unstable();
        assert_eq!(a_deps, a_deps2);
        assert_eq!(dep.edge_count(), dep2.edge_count());
        // Reverse index reconstructed (not persisted).
        let mut shared_rev = dep2.dependents_of("src/shared.ts");
        shared_rev.sort_unstable();
        assert_eq!(shared_rev, vec!["src/a.ts", "src/b.ts"]);
    }

    #[test]
    fn snapshot_to_bytes_is_deterministic() {
        let (sym, dep) = fixture();
        let a = SnapshotPayload::from_graphs(&sym, &dep).unwrap().to_bytes();
        let b = SnapshotPayload::from_graphs(&sym, &dep).unwrap().to_bytes();
        assert_eq!(a, b);
    }

    #[test]
    fn snapshot_round_trips_file_hashes_gv2_032() {
        // A warm-started daemon must recover the per-file content-freshness keys
        // (CE-7) so it can serve snippets without a re-scan.
        let (mut sym, dep) = fixture();
        sym.set_file_hash("src/a.ts".to_owned(), Some(0xDEAD_BEEF));
        sym.set_file_hash("src/b.ts".to_owned(), Some(0x0102_0304));

        let payload = SnapshotPayload::from_graphs(&sym, &dep).unwrap();
        let decoded = SnapshotPayload::from_bytes(&payload.to_bytes()).unwrap();
        let (sym2, _) = decoded.into_graphs().unwrap();

        assert_eq!(sym2.file_hash("src/a.ts"), Some(0xDEAD_BEEF));
        assert_eq!(sym2.file_hash("src/b.ts"), Some(0x0102_0304));
        assert_eq!(sym2.file_hash("src/never.ts"), None);
    }

    #[test]
    fn snapshot_round_trips_non_none_span_gv2_032() {
        // The fixture nodes carry span: None; exercise the Some(_) postcard path
        // explicitly so a broken Option<ByteRange> encoding cannot pass unnoticed.
        let mut sym = SymbolGraph::new();
        let mut n = node(1, "f", "src/a.ts", SymbolKind::Function);
        n.span = Some(ByteRange { start: 10, end: 42 });
        sym.add_symbol(n).unwrap();

        let payload = SnapshotPayload::from_graphs(&sym, &DependencyGraph::new()).unwrap();
        let (sym2, _) = SnapshotPayload::from_bytes(&payload.to_bytes())
            .unwrap()
            .into_graphs()
            .unwrap();
        assert_eq!(
            sym2.get_symbol(1).unwrap().span,
            Some(ByteRange { start: 10, end: 42 }),
            "GV2-032: a non-None span survives the snapshot round-trip",
        );
    }

    #[test]
    fn from_graphs_rejects_non_relative_file_hash_key_gv2_032() {
        // A file_hashes key is a workspace-root-relative path like every other
        // persisted path string; an absolute/escaping key fails the build-time
        // check_relative gate (the real no-leak guarantee for the new field).
        let mut sym = SymbolGraph::new();
        sym.add_symbol(node(1, "f", "src/a.ts", SymbolKind::Function))
            .unwrap();
        sym.set_file_hash("/etc/passwd".to_owned(), Some(0x99));
        let result = SnapshotPayload::from_graphs(&sym, &DependencyGraph::new());
        assert!(
            matches!(result, Err(SnapshotBuildError::NonRelativePath)),
            "a non-relative file_hashes key must be rejected at build time, got {result:?}",
        );
    }

    /// Build a fully-specified node — unlike [`node`], every enum-valued field is
    /// caller-chosen so the golden fixture can exercise each `Visibility` /
    /// `TrustLevel` variant, not just the `(Public, Internal)` default pair.
    fn golden_node(
        id: u64,
        name: &str,
        file: &str,
        kind: SymbolKind,
        visibility: Visibility,
        trust_level: TrustLevel,
    ) -> SymbolNode {
        SymbolNode {
            id,
            kind,
            name: name.to_owned(),
            visibility,
            file: file.to_owned(),
            trust_level,
            span: None,
        }
    }

    /// A *fully hand-pinned* snapshot fixture, kept deliberately independent of
    /// [`fixture`] so a future tweak to that shared fixture cannot silently shift
    /// the golden wire bytes below.
    ///
    /// CIB-092 council survivor (item 5): the fixture exercises **every** variant of
    /// the wire-bearing enums so a postcard variant-reorder/insertion on any single
    /// variant shifts the golden bytes and fails CI — the whole point of pinning.
    /// - `SymbolKind`: all 8 (Function ×2 as an overload pair so per-file ordering
    ///   is exercised, plus Class/Module/Export/Interface/TypeAlias/Enum/Method).
    /// - `Visibility`: both Public and Internal.
    /// - `TrustLevel`: all 5 (Unknown/Internal/Boundary/External/Privileged).
    /// - `EdgeType`: all 5 (Contains/References/Calls/Imports/Reexports).
    ///
    /// Ids and files are fixed and the inputs are added in a stable order so the
    /// emitted bytes stay deterministic.
    fn golden_fixture() -> SnapshotPayload {
        use SymbolKind::{Class, Enum, Export, Function, Interface, Method, Module, TypeAlias};
        use TrustLevel::{Boundary, External, Internal, Privileged, Unknown};
        use Visibility::{Internal as VInternal, Public};

        let mut sym = SymbolGraph::new();
        // Each tuple: (id, name, file, kind, visibility, trust_level). The kinds
        // cover all 8 variants; the visibility column hits both variants; the trust
        // column hits all 5. `alpha`/`alpha` on src/a.ts is the overload pair.
        let nodes = [
            (1, "alpha", "src/a.ts", Function, Public, Unknown),
            (2, "alpha", "src/a.ts", Function, Public, Internal), // overload of (1)
            (3, "Widget", "src/a.ts", Class, Public, Boundary),
            (4, "modx", "src/a.ts", Module, VInternal, External),
            (5, "ex", "src/a.ts", Export, Public, Privileged),
            (6, "Shape", "src/b.ts", Interface, VInternal, Unknown),
            (7, "Alias", "src/b.ts", TypeAlias, Public, Internal),
            (8, "Colour", "src/b.ts", Enum, Public, Boundary),
            (
                9,
                "Widget.render",
                "src/b.ts",
                Method,
                VInternal,
                Privileged,
            ),
        ];
        for (id, name, file, kind, vis, trust) in nodes {
            sym.add_symbol(golden_node(id, name, file, kind, vis, trust))
                .unwrap();
        }
        // One edge per `EdgeType` variant (all 5), in a fixed order.
        let edges = [
            (3, 1, EdgeType::Contains),
            (9, 1, EdgeType::References),
            (1, 9, EdgeType::Calls),
            (4, 6, EdgeType::Imports),
            (5, 7, EdgeType::Reexports),
        ];
        for (from, to, edge_type) in edges {
            sym.add_edge(SymbolEdge {
                from,
                to,
                edge_type,
            })
            .unwrap();
        }
        let mut dep = DependencyGraph::new();
        dep.add_dependency("src/a.ts".to_owned(), "src/b.ts".to_owned());
        SnapshotPayload::from_graphs(&sym, &dep).unwrap()
    }

    /// ADR-069 §6 **golden wire-bytes** fixture. The round-trip tests above only
    /// compare `to_bytes()` against another `to_bytes()` from the *same* binary, so
    /// a postcard/field/header/codec change drifts the writer and reader together
    /// and slips through undetected. This test pins `to_bytes()` of a fixed
    /// hand-built fixture against bytes **committed** to the source tree.
    ///
    /// ⚠️ **If this test fails, the on-disk snapshot wire format changed.** That is
    /// a breaking change to every persisted snapshot. Bump
    /// [`SNAPSHOT_BACKING_SCHEMA_VERSION`] (or [`SNAPSHOT_FORMAT_VERSION`] for an
    /// envelope/codec change) **deliberately**, then regenerate `EXPECTED` below by
    /// temporarily printing `golden_fixture().to_bytes()` and re-pinning. Do NOT
    /// "fix" the test by blindly pasting the new bytes without that version bump —
    /// the bump is what tells a deployed daemon to discard the now-incompatible
    /// snapshots and cold-rebuild instead of trusting a mis-shaped one.
    #[test]
    fn snapshot_wire_bytes_match_committed_golden() {
        // Generated once from the current code (see the regeneration note above).
        const EXPECTED: &[u8] = GOLDEN_SNAPSHOT_BYTES;
        let got = golden_fixture().to_bytes();
        assert_eq!(
            got, EXPECTED,
            "snapshot wire format changed — bump SNAPSHOT_BACKING_SCHEMA_VERSION \
             deliberately and regenerate the committed golden bytes (see the test doc)"
        );
        // Sanity: the committed bytes must themselves still pass the integrity gate
        // and round-trip, so a regenerated fixture cannot pin a broken artefact.
        assert_eq!(
            SnapshotPayload::from_bytes(EXPECTED).expect("golden bytes decode"),
            golden_fixture(),
            "committed golden bytes must round-trip through from_bytes",
        );
    }

    /// CIB-093e: the persisted-DTO schema version owns its own value and is NOT
    /// aliased to the delta-wire `GRAPH_DELTA_SCHEMA_VERSION`. It starts at `1` so
    /// existing snapshots (written when the two were equal) stay valid, and so the
    /// committed golden bytes — which embed it in the header at offset 12 — do not
    /// shift. A delta-wire bump must not silently invalidate warm-start snapshots.
    #[test]
    fn snapshot_backing_schema_version_is_independent_and_stable() {
        assert_eq!(
            SNAPSHOT_BACKING_SCHEMA_VERSION, 2,
            "persisted-DTO schema version must stay at its current value (2 — \
             bumped from 1 for GV2-032's `SnapshotNode.span` field, a real \
             on-disk DTO layout change); bump only on a real layout change"
        );
        // The header bytes still embed exactly this value (le u32 at offset 12),
        // so decoupling it from the delta-wire version did not move the golden bytes.
        let header = &golden_fixture().to_bytes()[12..16];
        assert_eq!(
            u32::from_le_bytes(header.try_into().unwrap()),
            SNAPSHOT_BACKING_SCHEMA_VERSION,
            "header backing_schema_version must equal the const"
        );
    }

    /// CIB-092 council survivor (item 5): the golden fixture must keep exercising
    /// **every** variant of each wire-bearing enum, or a postcard variant-reorder on
    /// an un-exercised variant would slip past the golden-bytes pin. This guards the
    /// fixture itself against a future trim that drops a variant's coverage.
    #[test]
    fn golden_fixture_exercises_every_wire_enum_variant() {
        let payload = golden_fixture();

        // `Vec::contains` (not `HashSet`) — `TrustLevel` is not `Hash`. The fixture
        // is tiny, so the linear scan is irrelevant.
        let kinds: Vec<SymbolKind> = payload.nodes.iter().map(|n| n.kind).collect();
        for kind in [
            SymbolKind::Function,
            SymbolKind::Class,
            SymbolKind::Module,
            SymbolKind::Export,
            SymbolKind::Interface,
            SymbolKind::TypeAlias,
            SymbolKind::Enum,
            SymbolKind::Method,
        ] {
            assert!(kinds.contains(&kind), "golden fixture must cover {kind:?}");
        }

        let visibilities: Vec<Visibility> = payload.nodes.iter().map(|n| n.visibility).collect();
        for vis in [Visibility::Public, Visibility::Internal] {
            assert!(
                visibilities.contains(&vis),
                "golden fixture must cover {vis:?}"
            );
        }

        let trust: Vec<TrustLevel> = payload.nodes.iter().map(|n| n.trust_level).collect();
        for level in [
            TrustLevel::Unknown,
            TrustLevel::Internal,
            TrustLevel::Boundary,
            TrustLevel::External,
            TrustLevel::Privileged,
        ] {
            assert!(
                trust.contains(&level),
                "golden fixture must cover {level:?}"
            );
        }

        let edge_types: Vec<EdgeType> = payload.edges.iter().map(|e| e.edge_type).collect();
        for edge in [
            EdgeType::Contains,
            EdgeType::References,
            EdgeType::Calls,
            EdgeType::Imports,
            EdgeType::Reexports,
        ] {
            assert!(
                edge_types.contains(&edge),
                "golden fixture must cover {edge:?}"
            );
        }
    }

    /// M-2: a session that inserted then removed high ids has `next_id` above any
    /// surviving node; the snapshot must restore that floor so reload does not
    /// re-issue a spent id.
    #[test]
    fn snapshot_restores_next_id_high_water_mark_after_removals() {
        let mut sym = SymbolGraph::new();
        for id in 1..=5 {
            sym.add_symbol(node(id, "s", "src/a.ts", SymbolKind::Function))
                .unwrap();
        }
        // next_id is now 6; emulate removing the top ids by snapshotting a graph
        // that only holds 1..=2 but whose next_id high-water mark is 6.
        let mut trimmed = SymbolGraph::new();
        trimmed
            .add_symbol(node(1, "s", "src/a.ts", SymbolKind::Function))
            .unwrap();
        trimmed
            .add_symbol(node(2, "s", "src/a.ts", SymbolKind::Function))
            .unwrap();
        trimmed.set_next_id_floor(6); // what the live graph would carry post-removal
        assert_eq!(trimmed.next_id(), 6);

        let dep = DependencyGraph::new();
        let payload = SnapshotPayload::from_graphs(&trimmed, &dep).unwrap();
        let (restored, _) = SnapshotPayload::from_bytes(&payload.to_bytes())
            .unwrap()
            .into_graphs()
            .unwrap();
        assert_eq!(
            restored.next_id(),
            6,
            "restored next_id must honour the persisted high-water mark, not max(surviving)+1"
        );
    }

    // ============================================================
    // Integrity gate: checksum, counts, version, magic (ADR-069 §1).
    // ============================================================

    #[test]
    fn snapshot_checksum_mismatch_is_typed_error_not_panic() {
        let (sym, dep) = fixture();
        let mut bytes = SnapshotPayload::from_graphs(&sym, &dep).unwrap().to_bytes();
        // Flip a bit in the body (past the header) — still decodes, fails CRC.
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        assert_eq!(
            SnapshotPayload::from_bytes(&bytes),
            Err(SnapshotLoadError::ChecksumMismatch)
        );
    }

    #[test]
    fn snapshot_count_mismatch_is_typed_error() {
        let (sym, dep) = fixture();
        let bytes = SnapshotPayload::from_graphs(&sym, &dep).unwrap().to_bytes();
        // Corrupt the header node-count (offset 16..24) and repair the body CRC so
        // the count check — not the checksum — is what fires.
        let mut tampered = bytes.clone();
        tampered[16] = tampered[16].wrapping_add(1);
        // The CRC covers only the body, so a header edit does not invalidate it;
        // the count check must catch this.
        match SnapshotPayload::from_bytes(&tampered) {
            Err(SnapshotLoadError::CountMismatch { .. }) => {}
            other => panic!("expected CountMismatch, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_version_mismatch_returns_typed_error_not_panic() {
        let (sym, dep) = fixture();
        let mut bytes = SnapshotPayload::from_graphs(&sym, &dep).unwrap().to_bytes();
        // Bump format_version in the header (offset 8..12).
        bytes[8] = bytes[8].wrapping_add(1);
        match SnapshotPayload::from_bytes(&bytes) {
            Err(SnapshotLoadError::VersionMismatch {
                found_format,
                expected_format,
                ..
            }) => {
                assert_eq!(found_format, SNAPSHOT_FORMAT_VERSION + 1);
                assert_eq!(expected_format, SNAPSHOT_FORMAT_VERSION);
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_bad_magic_is_typed_error() {
        let (sym, dep) = fixture();
        let mut bytes = SnapshotPayload::from_graphs(&sym, &dep).unwrap().to_bytes();
        bytes[0] = b'X';
        assert_eq!(
            SnapshotPayload::from_bytes(&bytes),
            Err(SnapshotLoadError::BadMagic)
        );
        // Too-short input is also BadMagic, not a panic.
        assert_eq!(
            SnapshotPayload::from_bytes(&[0u8; 4]),
            Err(SnapshotLoadError::BadMagic)
        );
    }

    #[test]
    fn snapshot_corrupt_bytes_return_typed_error_not_panic() {
        let (sym, dep) = fixture();
        let bytes = SnapshotPayload::from_graphs(&sym, &dep).unwrap().to_bytes();
        // Valid header (magic+version+counts+crc) but a torn body: keep the header
        // and half the body, then repair the CRC so the *decode* is what fails.
        let body = &bytes[HEADER_LEN..];
        let torn = &body[..body.len() / 2];
        let mut framed = Vec::new();
        framed.extend_from_slice(&bytes[0..16]); // magic + versions
        framed.extend_from_slice(&(0u64).to_le_bytes()); // node count (ignored; decode fails first)
        framed.extend_from_slice(&(0u64).to_le_bytes()); // edge count
        framed.extend_from_slice(&crc32(torn).to_le_bytes()); // matching CRC for the torn body
        framed.extend_from_slice(torn);
        assert_eq!(
            SnapshotPayload::from_bytes(&framed),
            Err(SnapshotLoadError::Corrupt)
        );
    }

    #[test]
    fn snapshot_oversized_is_rejected_before_decode() {
        let huge = vec![0_u8; MAX_SNAPSHOT_BYTES + 1];
        assert_eq!(
            SnapshotPayload::from_bytes(&huge),
            Err(SnapshotLoadError::Oversized)
        );
    }

    // ============================================================
    // from_graphs path validation.
    // ============================================================

    #[test]
    fn snapshot_from_graphs_rejects_absolute_symbol_path() {
        let mut sym = SymbolGraph::new();
        sym.add_symbol(node(1, "foo", "/etc/passwd", SymbolKind::Function))
            .unwrap();
        let dep = DependencyGraph::new();
        assert_eq!(
            SnapshotPayload::from_graphs(&sym, &dep),
            Err(SnapshotBuildError::NonRelativePath)
        );
    }

    #[test]
    fn snapshot_from_graphs_rejects_parent_escape_dependency() {
        let mut sym = SymbolGraph::new();
        sym.add_symbol(node(1, "foo", "src/a.ts", SymbolKind::Function))
            .unwrap();
        let mut dep = DependencyGraph::new();
        dep.add_dependency("src/a.ts".to_owned(), "../outside/secret.ts".to_owned());
        assert_eq!(
            SnapshotPayload::from_graphs(&sym, &dep),
            Err(SnapshotBuildError::NonRelativePath)
        );
    }

    #[test]
    fn is_workspace_root_relative_classification() {
        assert!(is_workspace_root_relative("src/a.ts"));
        assert!(is_workspace_root_relative("a.ts"));
        assert!(is_workspace_root_relative("./a.ts"));
        assert!(is_workspace_root_relative("has..dots.ts")); // `..` substring, not a segment
        assert!(!is_workspace_root_relative(""));
        assert!(!is_workspace_root_relative(" /etc/passwd")); // leading whitespace (N-2)
        assert!(!is_workspace_root_relative("\t/abs"));
        assert!(!is_workspace_root_relative("/etc/passwd"));
        assert!(!is_workspace_root_relative("\\windows\\rooted"));
        assert!(!is_workspace_root_relative("\\\\server\\share"));
        assert!(!is_workspace_root_relative("C:\\Users\\x"));
        assert!(!is_workspace_root_relative("c:/users/x"));
        assert!(!is_workspace_root_relative("../escape.ts"));
        assert!(!is_workspace_root_relative("src/../../escape.ts"));
    }

    // ============================================================
    // snapshot_filename (PV-8) + crc32 correctness.
    // ============================================================

    #[test]
    fn snapshot_filename_is_stable_and_carries_no_absolute_path() {
        let root = Path::new("/home/user/project");
        let a = snapshot_filename(root);
        let b = snapshot_filename(root);
        assert_eq!(a, b, "same input must yield the same filename");
        assert_eq!(&a[a.len() - 5..], ".snap");
        assert!(!a.contains('/'));
        assert!(!a.contains('\\'));
        assert!(!a.contains("home"));
        assert!(!a.contains("project"));
        // PV-8: a 128-bit FNV-1a digest, rendered as 32 lowercase hex chars +
        // `.snap` (replacing the former SHA-256 64-hex digest; the unsalted hash
        // had no crypto benefit, PV-12, so the crypto dep was dropped — CIB-092e).
        assert_eq!(a.len(), 32 + 5);
        assert!(a[..32].bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, snapshot_filename(Path::new("/home/user/other")));
    }

    #[test]
    fn snapshot_filename_pinned_for_a_known_root() {
        // Pin a concrete digest so an accidental change to the (now in-crate,
        // dependency-free) hash is caught. Regenerate deliberately if the hash
        // function is ever changed — but note that changes the on-disk key for
        // every worktree (a one-time cold rebuild, not a correctness hazard).
        assert_eq!(
            snapshot_filename(Path::new("/home/user/project")),
            "f8a71a04e8340307da890832f332fb1e.snap",
        );
    }

    #[test]
    fn crc32_matches_the_known_check_value() {
        // IEEE 802.3 check value for the canonical "123456789" input.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    // ============================================================
    // persist_graph_enabled gate (PV-11) — default off, fail-closed.
    // ============================================================

    #[test]
    fn persist_graph_gate_defaults_off_and_fails_closed() {
        assert!(!persist_graph_enabled(None));
        assert!(!persist_graph_enabled(Some("")));
        for v in ["1", "true", "TRUE", "Yes", " on ", "On"] {
            assert!(persist_graph_enabled(Some(v)), "{v:?} should enable");
        }
        for v in ["0", "false", "no", "off", "2", "enabled?", "garbage"] {
            assert!(!persist_graph_enabled(Some(v)), "{v:?} should stay off");
        }
    }
}
