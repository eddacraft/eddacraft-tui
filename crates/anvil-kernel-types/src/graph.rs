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

    /// The identity of the symbol with session-local id `id` within its
    /// file's parse-ordered symbol slice, or `None` if `id` is not in
    /// `symbols`.
    #[must_use]
    pub fn of_symbol(symbols: &[&SymbolNode], id: u64) -> Option<SymbolIdentity> {
        let identities = Self::for_file_symbols(symbols);
        symbols
            .iter()
            .position(|s| s.id == id)
            .map(|i| identities[i].clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    Contains,
    References,
    Calls,
    Imports,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSymbols {
    pub file: String,
    pub symbols: Vec<SymbolNode>,
    pub imports: Vec<ImportEdge>,
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
    fn identity_of_symbol_resolves_by_session_id() {
        let syms = [
            node(7, SymbolKind::Function, "foo", "a.ts"),
            node(9, SymbolKind::Function, "foo", "a.ts"),
        ];
        let refs: Vec<_> = syms.iter().collect();
        let second = SymbolIdentity::of_symbol(&refs, 9).unwrap();
        assert_eq!(second.ordinal, 1);
        assert!(SymbolIdentity::of_symbol(&refs, 42).is_none());
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
}
