//! Multi-format config loader (MLP-011).
//!
//! Loads Anvil config from `.yaml` / `.yml` / `.json` / `.toml` with a
//! single merge model.

mod canonical;
mod delegation;
mod discover;
mod format;
mod gate_section;
mod migrations;
mod parse;
mod rule_modes;
mod validation;

pub use canonical::{CanonicalError, canonical_json_bytes};
pub use delegation::{DelegationError, ResolvedSection, SectionProvenance, resolve_section};
pub use discover::{DISCOVER_PRECEDENCE, DiscoveredConfig, discover};
pub use format::ConfigFormat;
pub use gate_section::{GateSection, GateSectionError};
pub use migrations::{
    LEGACY_CAMEL_KEYS, SchemaMigration, apply_steps, legacy_keys_deprecation_note,
    normalize_legacy_keys, plan_for, plan_for_versions, production_migrations,
};
pub use parse::{MAX_CONFIG_FILE_BYTES, ParseError, parse_file, parse_str, read_to_string_bounded};
pub use rule_modes::{RuleMode, RuleModeError, RuleModes};
pub use validation::{HARD_PINNED_CLASSES, ValidationError, validate_hard_pinned_classes};
