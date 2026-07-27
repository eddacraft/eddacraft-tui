//! Dual-era MCP protocol host (MCP26 / ADR-113).
//!
//! Builds against the locked MCP `2026-07-28` **release candidate** until
//! MCP26-001 seals the final schema. Modern clients use per-request `_meta`
//! and `server/discover`; legacy initialise-era clients keep the existing
//! handshake path.

pub mod dispatch;
pub mod domain;
pub mod meta;
pub mod render;
pub mod trace;
pub mod versions;

pub use dispatch::handle_message;
