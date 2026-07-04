//! The single source of truth for the Rego policy rule-set vocabulary that
//! Anvil's policy surfaces recognise.
//!
//! Two surfaces evaluate `data.anvil.policies` Rego packs and extract findings
//! from the rule sets a member emits, and they MUST recognise the identical
//! rule families or a pack advisory that surfaces on one surface is silently
//! dropped on the other:
//!
//! - the gate policy check — `commands::gate::extract_policy_findings`
//!   (`anvil gate`), and
//! - the pre-write policy check — `mcp::policy_prewrite::extract_policy_findings`
//!   (the MCP save-time / pre-write path).
//!
//! Both consume [`VIOLATION_FAMILY_KEYS`] and [`WARNING_FAMILY_KEYS`] from here
//! rather than each re-listing the keys, so the two extractors cannot drift.
//! The families are the documented contract in
//! `docs/guides/opa-policy-testing.md`: "Both `violation` and `warning` rule
//! sets are recognised by the gate; `violation` becomes severity `error` by
//! default, `warning` becomes severity `warning`." The `warn`/`warnings` and
//! `violations`/`deny`/`denies` entries are accepted aliases carried from the
//! legacy OPA mapping.
//!
//! If a third rule-set extractor is ever added, it MUST consume these consts
//! too — add its name to the module doc above so the lockstep set stays
//! explicit.

/// Rego rule-set names whose members are **error**-class (blocking) findings —
/// the "violation family". `violation` is the documented canonical name.
pub(crate) const VIOLATION_FAMILY_KEYS: &[&str] = &["violation", "violations", "deny", "denies"];

/// Rego rule-set names whose members are **warning**-class (advisory,
/// non-blocking) findings — the "warning family". `warning` is the documented
/// canonical name; `warn`/`warnings` are accepted aliases.
pub(crate) const WARNING_FAMILY_KEYS: &[&str] = &["warning", "warn", "warnings"];
