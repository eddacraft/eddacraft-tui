//! Multi-format config loader (MLP-011).
//!
//! Loads Anvil configuration from `.yaml`, `.yml`, `.json`, or `.toml`
//! files with a single unambiguous detection precedence, and emits a
//! canonical JSON byte-stream so equivalent configs in any format
//! collapse to the same `rules_sha` (per ADR-037 §D-1).
//!
//! ## Scope (MLP-011 v1)
//!
//! - [`ConfigFormat`] — the four recognised extensions, in precedence
//!   order (`yaml` → `yml` → `json` → `toml`).
//! - [`discover`] — find the first file matching a basename + any
//!   recognised extension in a directory; precedence-deterministic.
//! - [`parse_str`] / [`parse_file`] — produce a `serde_json::Value`
//!   from any of the four formats, so downstream typed parsers can
//!   `serde_json::from_value` against the same intermediate. These
//!   functions **parse only**; they do not enforce hard-pinned class
//!   rejection. Callers loading operator configuration should call
//!   [`validate_hard_pinned_classes`] on the parsed value.
//! - [`validate_hard_pinned_classes`] (MLP-013) — rejects five
//!   disable-attempt shapes for the `secrets` and `command-safety`
//!   classes (canonical + legacy locations + mode-disabled); tuning
//!   passes through. Error messages cite [ADR-039] and the
//!   `@anvil-ignore` bypass. This validation is shipped as part of the
//!   crate; callers should call it on the parsed configuration after
//!   parsing.
//! - [`canonical_json_bytes`] — RFC 8785-style canonical encoding
//!   (sorted object keys, no insignificant whitespace) so equivalent
//!   yaml + json + toml inputs hash byte-identical.
//!
//! [ADR-039]: https://github.com/eddacraft/anvil-001/blob/main/plans/decisions/039-baseline-policy-and-hard-pinned-classes.md
//!
//! ## Out of scope (deferred)
//!
//! - Typed `AnvilConfig` schema — owned by consumers (init.rs, gate,
//!   policy parser) so each surface can evolve its own typed view of
//!   the same `serde_json::Value` intermediate.
//! - `--format` CLI flag wiring on `anvil start` / `anvil init` —
//!   filed as MLP-011 follow-up; the library is the building block.
//! - `.anvilrc` → `.anvil.<ext>` filename migration — separate
//!   concern; the existing `.anvilrc` reader in `commands/gate.rs`
//!   keeps working unchanged.
//!
//! ## Design notes
//!
//! TOML's data model has no native `null`, which means a yaml `null`
//! cannot round-trip through TOML. Parsing into `serde_json::Value`
//! deliberately preserves this: a `null` in yaml/json survives; a
//! corresponding TOML file simply omits the key. Consumers that care
//! about presence vs. null should not rely on TOML to encode `null`.

mod canonical;
mod discover;
mod format;
mod migrations;
mod parse;
mod rule_modes;
mod validation;

pub use canonical::{CanonicalError, canonical_json_bytes};
pub use discover::{DISCOVER_PRECEDENCE, DiscoveredConfig, discover};
pub use format::ConfigFormat;
pub use migrations::{
    SchemaMigration, apply_steps, plan_for, plan_for_versions, production_migrations,
};
pub use parse::{MAX_CONFIG_FILE_BYTES, ParseError, parse_file, parse_str, read_to_string_bounded};
pub use rule_modes::{RuleMode, RuleModeError, RuleModes};
pub use validation::{HARD_PINNED_CLASSES, ValidationError, validate_hard_pinned_classes};
