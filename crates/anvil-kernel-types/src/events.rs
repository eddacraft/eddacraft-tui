use serde::{Deserialize, Serialize};

use crate::EngineId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Progress,
    Snapshot,
    Violation,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineEvent {
    pub event_type: EventType,
    pub seq: u64,
    pub timestamp: String,
    pub engine: EngineId,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    Progress {
        phase: String,
        current: u64,
        total: u64,
    },
    Snapshot {
        node_count: u64,
        edge_count: u64,
        files_watched: u64,
    },
    Violation {
        policy_id: String,
        file: String,
        symbol: String,
        message: String,
    },
    Error(ErrorPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub file: Option<String>,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    ParseError,
    ConfigError,
    Internal,
}
