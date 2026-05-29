use serde::{Deserialize, Serialize};

use crate::TrustLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    #[test]
    fn symbol_kind_all_variants_distinct() {
        let variants = [
            SymbolKind::Function,
            SymbolKind::Class,
            SymbolKind::Module,
            SymbolKind::Export,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
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
        for variant in [
            SymbolKind::Function,
            SymbolKind::Class,
            SymbolKind::Module,
            SymbolKind::Export,
        ] {
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
}
