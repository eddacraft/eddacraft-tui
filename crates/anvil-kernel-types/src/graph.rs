use serde::{Deserialize, Serialize};

use crate::TrustLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Module,
    Export,
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
