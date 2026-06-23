use serde::{Deserialize, Serialize};

use crate::TrustLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Module,
    Export,
    /// A `type` / `interface` declaration's type-shape symbol (TS-G1).
    Interface,
    /// A `type X = …` alias declaration (TS-G1).
    TypeAlias,
    /// An `enum` declaration (TS-G1).
    Enum,
    /// A class/impl method, surfaced as a separate symbol. The owning type is
    /// encoded in the symbol name as `Owner.method` (TS-G2); a structural
    /// parent edge is deferred — `SymbolNode` carries no parent field and
    /// `FileSymbols` no symbol-edge channel, both out of LANGTS-002's scope.
    /// Name fidelity is best-effort: a computed name (`[Symbol.iterator]`)
    /// keeps its brackets, and a getter/setter is indistinguishable from a
    /// plain method of the same name. A literal top-level symbol named
    /// `Owner.method` would also collide — improbable in real TS, accepted
    /// under the deferral.
    Method,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: u64,
    pub kind: SymbolKind,
    pub name: String,
    pub visibility: Visibility,
    pub file: String,
    pub trust_level: TrustLevel,
    /// Source byte span of the symbol's defining node (GV2-032), or `None`
    /// when no span-producing pass ran (synthetic module nodes, external
    /// imports, reconstructed-from-store nodes). Offsets only, never text
    /// (PV-7(e)) — the locator GCTX-021 reads to extract a bounded snippet.
    /// `serde(default)` keeps the `GraphDelta` wire backward-compatible, and
    /// `skip_serializing_if` keeps span-less serialisations byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<ByteRange>,
}

/// Stable, cross-restart symbol identity (GV2-002).
///
/// Replaces the position-conflated `file::kind::name` baseline key and the
/// session-local `SymbolNode.id` counter as the *comparable* identity for
/// symbols. Two parses of the same source — in different sessions, after a
/// daemon restart, or on either side of a snapshot reload — assign equal
/// identities to the same symbols, because no component is session-local.
///
/// # Identity components
///
/// - `file` — the symbol's path identity. Workspace-root-relative by
///   contract (the parser feed supplies relative paths; the GV2-030 no-leak
///   test enforces relativity for anything persisted). A **file rename
///   changes every identity in the file**: rename is modelled as
///   delete + create, never tracked history (privacy verdict PV-4,
///   2026-06-08).
/// - `kind` / `name` — the structural identity. Same-name symbols in
///   different scopes stay distinct because method names encode their owner
///   (`Owner.method`, see [`SymbolKind::Method`]).
/// - `ordinal` — the overload disambiguator: the occurrence index of this
///   `(kind, name)` pair within the file, counted in parse (source) order.
///   Structural only — derived from source position ordering, never from
///   parameter source text or default-value expressions (privacy verdict
///   PV-1). Removing an earlier overload shifts later ordinals; that reads
///   as an identity change, which is the conservative, correct direction
///   for surface diffing.
///
/// # What this is not
///
/// Not a persisted format: the in-memory struct is the identity. Any future
/// compact/hashed encoding must use a named deterministic content hash
/// (privacy verdict PV-2) — never `std::hash::Hash` over the default
/// randomly-seeded hasher, which is not stable across processes. Edge
/// identity is derived, not stored: `(from, to, EdgeType)` over the
/// endpoints' `SymbolIdentity` values. Session/worktree identity and
/// APS/provenance references are join-time-only and never appear here
/// (privacy verdict PV-3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolIdentity {
    /// Workspace-root-relative path identity.
    pub file: String,
    /// Structural kind.
    pub kind: SymbolKind,
    /// Symbol name (methods encode their owner as `Owner.method`).
    pub name: String,
    /// Occurrence index among same-`(kind, name)` symbols in the file,
    /// in parse order. 0 for the first (or only) occurrence.
    pub ordinal: u32,
}

impl SymbolIdentity {
    /// Assign stable identities to a file's symbols, in parse order.
    ///
    /// The returned vector parallels `symbols`: `identities[i]` is the
    /// identity of `symbols[i]`. Ordinals are assigned per `(kind, name)`
    /// pair in slice order, so the caller must pass symbols in parse
    /// (source) order — `SymbolGraph::symbols_in_file` and
    /// `FileSymbols.symbols` both preserve it.
    #[must_use]
    pub fn for_file_symbols(symbols: &[&SymbolNode]) -> Vec<SymbolIdentity> {
        let mut seen: std::collections::HashMap<(SymbolKind, &str), u32> =
            std::collections::HashMap::new();
        symbols
            .iter()
            .map(|s| {
                let ordinal = seen.entry((s.kind, s.name.as_str())).or_insert(0);
                let identity = SymbolIdentity {
                    file: s.file.clone(),
                    kind: s.kind,
                    name: s.name.clone(),
                    ordinal: *ordinal,
                };
                *ordinal += 1;
                identity
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EdgeType {
    Contains,
    References,
    Calls,
    Imports,
    /// A re-export: a symbol made available from another module
    /// (`export { x } from "m"`, `export * from "m"`, Rust `pub use`).
    /// First-class so impact analysis (GV2-011) can distinguish a re-export
    /// from a plain import — a re-export widens this module's public surface,
    /// a plain import does not. The carrier is [`ReexportEdge`] (file → module,
    /// mirroring [`ImportEdge`]); this variant tags the edge once re-exports
    /// are lifted into the symbol graph. Plural to match `Imports`/`Calls`.
    Reexports,
}

/// A source span as byte offsets — **no text** (privacy verdict PV-7(e),
/// 2026-06-08).
///
/// Structurally incapable of holding a source body, snippet, or literal: it
/// carries only positions, so it is safe to persist in the GV2 snapshot DTO
/// (GV2-030) and to project into assistant context (GCTX) without leaking
/// code. The frozen no-text shape lands now (GV2-010 schema); the
/// *population* of spans onto nodes/edges is a GCTX-projection concern wired
/// in v0.9 (a `span` field attaches when a producer and consumer exist),
/// per the ADR-075 A′-slice scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ByteRange {
    /// Inclusive start byte offset into the source file.
    pub start: u32,
    /// Exclusive end byte offset into the source file.
    pub end: u32,
}

impl ByteRange {
    /// Construct from a parser's `usize` byte range (e.g. tree-sitter's
    /// [`tree_sitter::Node::byte_range`]), saturating offsets into `u32`.
    /// Source files within the supported size never approach `u32::MAX`; a
    /// pathological >4 GiB file saturates rather than wrapping.
    #[must_use]
    pub fn from_range(range: std::ops::Range<usize>) -> Self {
        Self {
            start: u32::try_from(range.start).unwrap_or(u32::MAX),
            end: u32::try_from(range.end).unwrap_or(u32::MAX),
        }
    }

    /// Length of the span in bytes (`end - start`), saturating at 0 for an
    /// inverted range rather than panicking.
    #[must_use]
    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span covers zero bytes (empty or inverted).
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }
}

/// A stable, dependency-free 64-bit **FNV-1a** digest of `bytes` — the GV2-032
/// per-file content-freshness key (PV-9 CE-7), of the same digest *family* the
/// snapshot codec uses (GV2-030, PV-8). Stable across processes and releases,
/// unlike `std::hash` over the default randomly-seeded `SipHash` (not
/// persistence-stable). Unsalted, so it is cross-machine correlatable (git-blob
/// class — the N-2 residual named in the GCTX-001 note); acceptable for a
/// machine-local freshness check, never a security boundary.
#[must_use]
pub fn content_hash(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET_BASIS;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEdge {
    pub from: u64,
    pub to: u64,
    pub edge_type: EdgeType,
}

/// Extracted symbols from a single file.
///
/// A plain data carrier produced by the parser and consumed by the graph
/// algorithms in `anvil-graph-cache`. Relocated here from `anvil-kernel`'s
/// parser crate (ADR-064) so the graph layer can name it without depending on
/// the tree-sitter parser surface; the old `anvil_kernel::parser::extract`
/// path still resolves via a re-export. Carries `serde` for parity with the
/// sibling graph types (and the planned daemon `FileSymbols` feed, ADR-064 §4).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileSymbols {
    pub file: String,
    pub symbols: Vec<SymbolNode>,
    pub imports: Vec<ImportEdge>,
    /// Re-export edges (`export { x } from "m"`, `export * from "m"`,
    /// Rust `pub use`) — distinct from `imports`, which do not widen the
    /// public surface. Defaults empty so older serialized `FileSymbols`
    /// (pre-GV2-010) still deserialize.
    #[serde(default)]
    pub reexports: Vec<ReexportEdge>,
    /// Symbol-level call sites (GCALL-001 / ADR-086): the producer-side, **still
    /// unresolved** caller→callee records. The caller is a file-local
    /// [`LocalSymbolRef`]; the callee a [`CalleeRef`] resolved to a
    /// [`SymbolIdentity`] only at lift time (GCALL-003) against the import graph.
    /// Defaults empty so older serialized `FileSymbols` (pre-GCALL-002) still
    /// deserialize, and so a language whose extractor does not yet emit call
    /// sites (Rust/Python until GCALL-004/005) carries none.
    #[serde(default)]
    pub calls: Vec<CallSite>,
    /// `true` when this file's call-site extraction was bounded by the per-file
    /// [`MAX_CALL_SITES`] cap (ADR-086 §3) — `calls` holds the first
    /// `MAX_CALL_SITES` in deterministic walk order and the rest were dropped, so
    /// the file's call data is **incomplete**. The daemon folds this into the
    /// GCALL-007 CALL-1 `partial` egress marker so a consumer is never told a
    /// truncated caller set is complete. Defaults false so older serialized
    /// `FileSymbols` (pre-cap) still deserialize and an uncapped file is honest.
    #[serde(default)]
    pub calls_partial: bool,
    /// `true` when this file contains a **dynamic** import whose target could not
    /// be resolved to a string-literal specifier — a computed `require(someVar)`
    /// or `import(`./${x}`)` (CIB-093 N1). A literal dynamic import
    /// (`require('fs')`, `import('fs')`) is extracted as a normal [`ImportEdge`]
    /// and does **not** set this; only the unknowable computed form does.
    ///
    /// The trust pass operates on static import/re-export edges only, so a
    /// privileged module reached through a computed dynamic import produces no
    /// edge and would certify CLEAN. The daemon folds this flag onto the
    /// `GraphDelta` so the save-time certify path fails closed (degrades to
    /// `Partial`) rather than silently certifying a file that may reach a
    /// privileged built-in at runtime. Defaults `false` so older serialized
    /// `FileSymbols` still deserialize and a file with no dynamic import is
    /// honest.
    #[serde(default)]
    pub has_unresolved_dynamic_import: bool,
    /// GV2-032 freshness key: a stable [`content_hash`] of the source bytes this
    /// `FileSymbols` was extracted from, or `None` when the producer did not
    /// supply one. `update_file` records it on the resident graph so a snippet
    /// projector (GCTX-021) can re-validate the on-disk file against the parsed
    /// content before emitting source text (PV-9 CE-7) — a mismatch means the
    /// graph is stale for that file and the snippet is withheld. Defaults `None`
    /// so older serialized `FileSymbols` still deserialize.
    #[serde(default)]
    pub content_hash: Option<u64>,
}

/// A file-local reference to a symbol within its own file (GCALL-001 / ADR-086).
///
/// Identifies the **enclosing caller** of a [`CallSite`] using the same
/// `(kind, name, ordinal)` scheme as [`SymbolIdentity`] minus `file` (it is
/// always the current file). The extractor MUST derive `ordinal` from
/// [`SymbolIdentity::for_file_symbols`] over the file's emitted `symbols` so the
/// ordinal matches the lift-time identity exactly.
///
/// `module_scope == true` marks a top-level or anonymous-closure call site that
/// has **no** enclosing named symbol; it binds to the file's synthetic module
/// node at lift time, and `kind` / `name` / `ordinal` are then ignored (carried
/// as placeholders).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSymbolRef {
    /// Structural kind of the enclosing symbol (placeholder when `module_scope`).
    pub kind: SymbolKind,
    /// Name of the enclosing symbol (methods encode their owner as
    /// `Owner.method`); empty placeholder when `module_scope`.
    pub name: String,
    /// Occurrence index among same-`(kind, name)` symbols in the file, parse
    /// order — the [`SymbolIdentity::for_file_symbols`] scheme.
    pub ordinal: u32,
    /// True for a module-scope / anonymous-closure caller with no enclosing named
    /// symbol. Defaults false so an older serialized `CallSite` still
    /// deserializes.
    #[serde(default)]
    pub module_scope: bool,
}

/// The callee at a [`CallSite`], to be resolved to a [`SymbolIdentity`] at lift
/// time (GCALL-003 / ADR-086).
///
/// `name` is the **target module's export name**: the extractor reverse-maps a
/// local alias (`import { foo as bar } from "m"; bar()`) to the exported `foo`,
/// and a namespace member (`import * as ns from "m"; ns.foo()`) to `foo`. A
/// same-file callee carries `via_import: None`; an imported callee carries the
/// module specifier. A **default import** is named `"default"` (the extractor
/// records the binding, but the lift treats `"default"` as `Unresolved` in v1).
/// The model is best-effort and static — a callee the extractor cannot name at
/// all (dynamic dispatch, a computed member `obj[x]()`, an IIFE) emits no
/// `CallSite`; one it can name but that does not bind to a resident symbol
/// resolves to `Unresolved` at lift time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalleeRef {
    /// The callee's name in its defining module (export name, not the local
    /// alias).
    pub name: String,
    /// The module specifier the callee is imported from, or `None` for a
    /// same-file callee.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via_import: Option<String>,
}

/// Per-file cap on extracted [`CallSite`]s (ADR-086 §3 budget escape hatch).
///
/// A pathological, call-dense file (generated/bundled code, a giant dispatch
/// table) is bounded here so it can never blow the save-time lift budget
/// (ADR-031): over the cap the extractor keeps the first `MAX_CALL_SITES` call
/// sites in deterministic walk order, drops the rest, and sets
/// [`FileSymbols::calls_partial`]. Sized well above any hand-written file's
/// call-site count, so a real source file is never truncated; it bites only the
/// generated outliers the budget exists to contain.
pub const MAX_CALL_SITES: usize = 2_048;

/// A symbol-level call site (GCALL-001 / ADR-086): caller → callee at one
/// source location, **before** cross-file resolution.
///
/// Identity-only and text-free: a file-local caller reference, a callee name +
/// optional module specifier, and a 1-based line. No argument text, no receiver
/// expression, no source body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    /// The enclosing caller (or `module_scope`).
    pub from: LocalSymbolRef,
    /// The callee to resolve at lift time.
    pub callee: CalleeRef,
    /// 1-based line number of the call site (0 = unknown).
    pub line: u32,
}

/// An import edge from one file to another.
///
/// Relocated here from the parser crate (ADR-064); carries no parser logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEdge {
    pub from_file: String,
    pub to_source: String,
    /// 1-based line number of the import statement (0 = unknown).
    pub line: u32,
}

/// A re-export edge: `from_file` re-exports `exported_name` from `to_source`
/// (`export { exported_name } from "to_source"`, Rust `pub use to_source`).
///
/// A **file-level carrier** (`from_file` → `to_source`, mirroring
/// [`ImportEdge`]) that names the re-exported symbol in `exported_name`. This
/// realizes the GV2-010 symbol→module re-export relationship at file
/// granularity — enough for impact analysis to attribute the widened public
/// surface to a specific export; lifting to symbol-level endpoints is GV2-011's
/// job. A wildcard re-export (`export * from "m"`, `pub use m::*`) uses
/// `exported_name == "*"`. Carries no source text (privacy line): names and the
/// module specifier are identity strings, not bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReexportEdge {
    /// The re-exporting file (workspace-root-relative).
    pub from_file: String,
    /// The re-exported symbol name, or `"*"` for a wildcard re-export.
    pub exported_name: String,
    /// The source module specifier the symbol is re-exported from.
    pub to_source: String,
    /// 1-based line number of the re-export statement (0 = unknown).
    pub line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node() -> SymbolNode {
        SymbolNode {
            id: 1,
            kind: SymbolKind::Function,
            name: "handleRequest".into(),
            visibility: Visibility::Public,
            file: "src/handler.ts".into(),
            trust_level: TrustLevel::Internal,
            span: None,
        }
    }

    fn sample_edge() -> SymbolEdge {
        SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Calls,
        }
    }

    // --- SymbolKind ---

    /// Every `SymbolKind` variant. Update when a variant is added so the
    /// distinctness + serde round-trip tests stay exhaustive.
    const ALL_SYMBOL_KINDS: [SymbolKind; 8] = [
        SymbolKind::Function,
        SymbolKind::Class,
        SymbolKind::Module,
        SymbolKind::Export,
        SymbolKind::Interface,
        SymbolKind::TypeAlias,
        SymbolKind::Enum,
        SymbolKind::Method,
    ];

    #[test]
    fn symbol_kind_all_variants_distinct() {
        for (i, a) in ALL_SYMBOL_KINDS.iter().enumerate() {
            for (j, b) in ALL_SYMBOL_KINDS.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn symbol_kind_copy_semantics() {
        let a = SymbolKind::Class;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn symbol_kind_serde_round_trip() {
        for variant in ALL_SYMBOL_KINDS {
            let json = serde_json::to_string(&variant).unwrap();
            let back: SymbolKind = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    // --- Visibility ---

    #[test]
    fn visibility_variants_distinct() {
        assert_ne!(Visibility::Public, Visibility::Internal);
    }

    #[test]
    fn visibility_serde_round_trip() {
        for v in [Visibility::Public, Visibility::Internal] {
            let json = serde_json::to_string(&v).unwrap();
            let back: Visibility = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    // --- EdgeType ---

    #[test]
    fn edge_type_all_variants_distinct() {
        let variants = [
            EdgeType::Contains,
            EdgeType::References,
            EdgeType::Calls,
            EdgeType::Imports,
            EdgeType::Reexports,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn edge_type_copy_semantics() {
        let a = EdgeType::Imports;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn edge_type_serde_round_trip() {
        for variant in [
            EdgeType::Contains,
            EdgeType::References,
            EdgeType::Calls,
            EdgeType::Imports,
            EdgeType::Reexports,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: EdgeType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    // --- SymbolNode ---

    #[test]
    fn symbol_node_construction() {
        let node = sample_node();
        assert_eq!(node.id, 1);
        assert_eq!(node.kind, SymbolKind::Function);
        assert_eq!(node.name, "handleRequest");
        assert_eq!(node.visibility, Visibility::Public);
        assert_eq!(node.file, "src/handler.ts");
        assert_eq!(node.trust_level, TrustLevel::Internal);
    }

    #[test]
    fn symbol_node_clone_is_independent() {
        let node = sample_node();
        let mut cloned = node.clone();
        cloned.name = "otherHandler".into();
        assert_eq!(node.name, "handleRequest");
        assert_eq!(cloned.name, "otherHandler");
    }

    #[test]
    fn symbol_node_serde_round_trip() {
        let node = sample_node();
        let json = serde_json::to_string(&node).unwrap();
        let back: SymbolNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, node.id);
        assert_eq!(back.kind, node.kind);
        assert_eq!(back.name, node.name);
        assert_eq!(back.visibility, node.visibility);
        assert_eq!(back.file, node.file);
        assert_eq!(back.trust_level, node.trust_level);
    }

    #[test]
    fn symbol_node_debug_contains_fields() {
        let dbg = format!("{:?}", sample_node());
        assert!(dbg.contains("handleRequest"));
        assert!(dbg.contains("Function"));
    }

    #[test]
    fn symbol_node_with_max_id() {
        let node = SymbolNode {
            id: u64::MAX,
            ..sample_node()
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: SymbolNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, u64::MAX);
    }

    #[test]
    fn symbol_node_with_empty_name() {
        let node = SymbolNode {
            name: String::new(),
            ..sample_node()
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: SymbolNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "");
    }

    // --- SymbolEdge ---

    #[test]
    fn symbol_edge_construction() {
        let edge = sample_edge();
        assert_eq!(edge.from, 1);
        assert_eq!(edge.to, 2);
        assert_eq!(edge.edge_type, EdgeType::Calls);
    }

    #[test]
    fn symbol_edge_serde_round_trip() {
        let edge = sample_edge();
        let json = serde_json::to_string(&edge).unwrap();
        let back: SymbolEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.from, edge.from);
        assert_eq!(back.to, edge.to);
        assert_eq!(back.edge_type, edge.edge_type);
    }

    #[test]
    fn symbol_edge_self_referencing() {
        let edge = SymbolEdge {
            from: 5,
            to: 5,
            edge_type: EdgeType::Contains,
        };
        assert_eq!(edge.from, edge.to);
    }

    #[test]
    fn symbol_edge_clone_is_independent() {
        let edge = sample_edge();
        let cloned = edge.clone();
        assert_eq!(edge.from, cloned.from);
        assert_eq!(edge.to, cloned.to);
        assert_eq!(edge.edge_type, cloned.edge_type);
    }

    // --- Deserialisation error cases ---

    #[test]
    fn invalid_symbol_kind_fails() {
        let result = serde_json::from_str::<SymbolKind>("\"NotARealKind\"");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_edge_type_fails() {
        let result = serde_json::from_str::<EdgeType>("\"Extends\"");
        assert!(result.is_err());
    }

    #[test]
    fn symbol_node_missing_field_fails() {
        // Missing trust_level
        let json = r#"{"id":1,"kind":"Function","name":"f","visibility":"Public","file":"a.ts"}"#;
        let result = serde_json::from_str::<SymbolNode>(json);
        assert!(result.is_err());
    }

    // --- SymbolIdentity (GV2-002) ---

    fn node(id: u64, kind: SymbolKind, name: &str, file: &str) -> SymbolNode {
        SymbolNode {
            id,
            kind,
            name: name.into(),
            visibility: Visibility::Public,
            file: file.into(),
            trust_level: TrustLevel::Unknown,
            span: None,
        }
    }

    #[test]
    fn identity_is_independent_of_session_ids() {
        // The same source parsed in two sessions allocates different u64 ids;
        // the stable identities must be equal.
        let a = [
            node(1, SymbolKind::Function, "foo", "a.ts"),
            node(2, SymbolKind::Class, "Bar", "a.ts"),
        ];
        let b = [
            node(901, SymbolKind::Function, "foo", "a.ts"),
            node(902, SymbolKind::Class, "Bar", "a.ts"),
        ];
        let ia = SymbolIdentity::for_file_symbols(&a.iter().collect::<Vec<_>>());
        let ib = SymbolIdentity::for_file_symbols(&b.iter().collect::<Vec<_>>());
        assert_eq!(ia, ib);
    }

    #[test]
    fn identity_disambiguates_overloads_by_ordinal() {
        let syms = [
            node(1, SymbolKind::Function, "foo", "a.ts"),
            node(2, SymbolKind::Function, "foo", "a.ts"),
            node(3, SymbolKind::Function, "bar", "a.ts"),
        ];
        let ids = SymbolIdentity::for_file_symbols(&syms.iter().collect::<Vec<_>>());
        assert_eq!(ids[0].ordinal, 0);
        assert_eq!(ids[1].ordinal, 1);
        assert_ne!(ids[0], ids[1], "overloads must have distinct identities");
        assert_eq!(ids[2].ordinal, 0, "different name starts its own count");
    }

    #[test]
    fn identity_distinguishes_same_name_different_kind() {
        // A free function `render` and a method `Widget.render` differ by
        // kind and/or encoded owner name — never collapse.
        let syms = [
            node(1, SymbolKind::Function, "render", "a.ts"),
            node(2, SymbolKind::Method, "Widget.render", "a.ts"),
            node(3, SymbolKind::Class, "render", "a.ts"),
        ];
        let ids = SymbolIdentity::for_file_symbols(&syms.iter().collect::<Vec<_>>());
        assert_eq!(
            ids.iter().collect::<std::collections::HashSet<_>>().len(),
            3
        );
        assert!(ids.iter().all(|i| i.ordinal == 0));
    }

    #[test]
    fn identity_changes_with_file_path() {
        // File rename = delete + create: identity includes the path.
        let before = [node(1, SymbolKind::Function, "foo", "old.ts")];
        let after = [node(1, SymbolKind::Function, "foo", "new.ts")];
        let ib = SymbolIdentity::for_file_symbols(&before.iter().collect::<Vec<_>>());
        let ia = SymbolIdentity::for_file_symbols(&after.iter().collect::<Vec<_>>());
        assert_ne!(ib[0], ia[0]);
    }

    #[test]
    fn identity_serde_round_trip() {
        let id = SymbolIdentity {
            file: "src/a.ts".into(),
            kind: SymbolKind::Method,
            name: "Owner.method".into(),
            ordinal: 1,
        };
        let json = serde_json::to_string(&id).unwrap();
        let back: SymbolIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    // --- ByteRange (GV2-010, no-text span per PV-7e) ---

    #[test]
    fn byte_range_len_and_is_empty() {
        let span = ByteRange { start: 10, end: 25 };
        assert_eq!(span.len(), 15);
        assert!(!span.is_empty());

        let empty = ByteRange { start: 7, end: 7 };
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn byte_range_inverted_saturates_not_panics() {
        // An inverted range must not panic (saturating semantics) and reads
        // as empty rather than producing a bogus length.
        let inverted = ByteRange { start: 30, end: 10 };
        assert_eq!(inverted.len(), 0);
        assert!(inverted.is_empty());
    }

    #[test]
    fn byte_range_serde_round_trip() {
        let span = ByteRange {
            start: 0,
            end: 4096,
        };
        let json = serde_json::to_string(&span).unwrap();
        let back: ByteRange = serde_json::from_str(&json).unwrap();
        assert_eq!(span, back);
    }

    // --- ReexportEdge (GV2-010) ---

    #[test]
    fn reexport_edge_named_construction() {
        let re = ReexportEdge {
            from_file: "src/index.ts".into(),
            exported_name: "Button".into(),
            to_source: "./components/button".into(),
            line: 3,
        };
        assert_eq!(re.exported_name, "Button");
        assert_eq!(re.to_source, "./components/button");
    }

    #[test]
    fn reexport_edge_wildcard_uses_star() {
        let re = ReexportEdge {
            from_file: "src/index.ts".into(),
            exported_name: "*".into(),
            to_source: "./widgets".into(),
            line: 1,
        };
        assert_eq!(re.exported_name, "*");
    }

    #[test]
    fn reexport_edge_serde_round_trip() {
        let re = ReexportEdge {
            from_file: "src/index.ts".into(),
            exported_name: "bar".into(),
            to_source: "./b".into(),
            line: 9,
        };
        let json = serde_json::to_string(&re).unwrap();
        let back: ReexportEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(re, back);
    }

    #[test]
    fn file_symbols_reexports_defaults_empty_on_legacy_json() {
        // Pre-GV2-010 serialized FileSymbols had no `reexports` field; it must
        // still deserialize (serde default) so older snapshots keep loading.
        let legacy = r#"{"file":"src/a.ts","symbols":[],"imports":[]}"#;
        let fs: FileSymbols = serde_json::from_str(legacy).unwrap();
        assert!(fs.reexports.is_empty());
    }

    // --- GCALL-002 call-site types ---

    #[test]
    fn file_symbols_calls_defaults_empty_on_pre_gcall_json() {
        // Pre-GCALL-002 serialized FileSymbols (and a non-TS language's feed)
        // carry no `calls` field; it must still deserialize (serde default).
        let legacy = r#"{"file":"src/a.ts","symbols":[],"imports":[],"reexports":[]}"#;
        let fs: FileSymbols = serde_json::from_str(legacy).unwrap();
        assert!(fs.calls.is_empty());
    }

    #[test]
    fn call_site_round_trips_and_omits_absent_via_import() {
        let same_file = CallSite {
            from: LocalSymbolRef {
                kind: SymbolKind::Function,
                name: "run".into(),
                ordinal: 0,
                module_scope: false,
            },
            callee: CalleeRef {
                name: "helper".into(),
                via_import: None,
            },
            line: 4,
        };
        let json = serde_json::to_string(&same_file).unwrap();
        // `via_import` omitted when None (skip_serializing_if); `module_scope`
        // false is carried.
        assert!(!json.contains("via_import"));
        assert_eq!(same_file, serde_json::from_str(&json).unwrap());

        let imported = CallSite {
            from: LocalSymbolRef {
                kind: SymbolKind::Module,
                name: String::new(),
                ordinal: 0,
                module_scope: true,
            },
            callee: CalleeRef {
                name: "foo".into(),
                via_import: Some("./m".into()),
            },
            line: 2,
        };
        assert_eq!(
            imported,
            serde_json::from_str(&serde_json::to_string(&imported).unwrap()).unwrap()
        );
    }

    #[test]
    fn local_symbol_ref_module_scope_defaults_false() {
        // An older `CallSite.from` without `module_scope` deserializes as a named
        // (non-module) caller.
        let legacy = r#"{"kind":"Function","name":"run","ordinal":1}"#;
        let r: LocalSymbolRef = serde_json::from_str(legacy).unwrap();
        assert!(!r.module_scope);
        assert_eq!(r.ordinal, 1);
    }
}
