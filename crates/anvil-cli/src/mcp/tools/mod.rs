pub mod affected_tests;
pub mod apply_patch;
pub mod check;
pub mod find_callers;
pub mod find_dependents;
pub mod fix;
pub mod gate;
pub mod impact_of_change;
pub mod query_boundary;
pub mod registry;
/// MCP26-008 catalogue checks (JSON Schema 2020-12). Test-only: uses the
/// `jsonschema` dev-dependency and is not linked into the shipped binary.
#[cfg(test)]
pub mod schema_catalogue;
pub mod search_symbols;
pub mod shared;
pub mod status;
pub mod suppress;
pub mod symbol_context;
pub mod validate_write;
