//! Shared SARIF 2.1.0 emitter for Anvil (ADR-058, SARIFOUT-002).
//!
//! This crate owns the SARIF document *shape* for the bounded subset Anvil
//! emits (the GitHub Code Scanning ingest subset): `runs[]` / `tool.driver` /
//! `rules[]` / `results[]` / `locations[]` / `suppressions[]` /
//! `partialFingerprints`. It is a pure serialisation layer — no command or
//! collector is wired here. Consumers map their existing finding shape into
//! these types: the CLI's per-command adapters (SARIFOUT-003/004/005) and the
//! review-capsule diagnostics collector (GITGOV-008) each do so independently;
//! there is deliberately **no** unified in-process finding model (ADR-058).
//!
//! It lives in its own crate (rather than inside the `anvil-cli` binary) so
//! non-CLI producers — the review capsule first — can reuse the one emitter,
//! exactly the "shared across any future SARIF output" intent ADR-058 recorded.
//!
//! The bundled upstream SARIF 2.1.0 JSON Schema
//! (`sarif-schema-2.1.0.json`, vendored verbatim from schemastore) is the
//! validation gate: the test module checks emitted documents against it.
//!
//! Part of the public surface (e.g. `SuppressionKind::External`, `Level::None`,
//! `ReportingDescriptor::help_uri`) is a faithful slice of the SARIF model used
//! by some consumers and not others; as a library API it is exercised by the
//! test module rather than silenced with a blanket `dead_code` allow.
//!
//! User-facing docs for `--format sarif` live in the GitHub integration guide
//! (`docs/public/anvil/integrations/github.md`, "Code Scanning (SARIF)"); the
//! out-of-band Code Scanning upload check is
//! `docs/runbooks/sarif-code-scanning-upload.md`.

use std::collections::BTreeMap;

use std::num::NonZeroU32;

use serde::Serialize;
use sha2::{Digest, Sha256};

/// SARIF specification version Anvil emits.
pub const SARIF_VERSION: &str = "2.1.0";

/// `$schema` URI advertised in emitted documents. This is the schemastore
/// distribution of SARIF 2.1.0 (the bundled schema's own `$id` is the OASIS raw
/// URL); both describe the same 2.1.0 schema.
pub const SARIF_SCHEMA_URI: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// The bundled upstream SARIF 2.1.0 JSON Schema, vendored verbatim from
/// schemastore. Exposed as the single source of the validation gate so any
/// SARIF producer (this crate's tests, the CLI command adapters, the review
/// capsule) validates against the same schema rather than a private copy.
pub const SARIF_SCHEMA_JSON: &str = include_str!("sarif-schema-2.1.0.json");

/// `tool.driver.name` for every Anvil-emitted run.
pub const DRIVER_NAME: &str = "anvil";

/// `tool.driver.informationUri` — where consumers learn what `anvil` is.
pub const DRIVER_INFORMATION_URI: &str = "https://github.com/eddacraft/anvil-001";

/// A SARIF 2.1.0 log document (single run).
#[derive(Debug, Serialize)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: [Run; 1],
}

impl SarifLog {
    /// Wrap a single [`Run`] into a complete, schema-tagged SARIF log.
    #[must_use]
    pub fn new(run: Run) -> Self {
        Self {
            schema: SARIF_SCHEMA_URI,
            version: SARIF_VERSION,
            runs: [run],
        }
    }
}

/// A single SARIF run: one tool plus its results.
#[derive(Debug, Serialize)]
pub struct Run {
    tool: Tool,
    results: Vec<SarifResult>,
}

impl Run {
    /// Build a run from the Anvil driver (with its `rules[]`) and `results[]`.
    #[must_use]
    pub fn new(rules: Vec<ReportingDescriptor>, results: Vec<SarifResult>) -> Self {
        Self {
            tool: Tool {
                driver: Driver::anvil(rules),
            },
            results,
        }
    }
}

#[derive(Debug, Serialize)]
struct Tool {
    driver: Driver,
}

/// `tool.driver` — the `anvil` tool component and the rules it reported.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Driver {
    name: &'static str,
    information_uri: &'static str,
    version: &'static str,
    rules: Vec<ReportingDescriptor>,
}

impl Driver {
    fn anvil(rules: Vec<ReportingDescriptor>) -> Self {
        Self {
            name: DRIVER_NAME,
            information_uri: DRIVER_INFORMATION_URI,
            version: env!("CARGO_PKG_VERSION"),
            rules,
        }
    }
}

/// A `reportingDescriptor` (rule) entry in `tool.driver.rules[]`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingDescriptor {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short_description: Option<MultiformatMessageString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help_uri: Option<String>,
    /// SARIF property bag. ADR-071 §9: AST-tier (gate-time) rules carry
    /// `properties.tier = "ast"` so consumers can tell a gate-tier AST rule from
    /// a save-time regex rule (addressing the priority-inversion legibility
    /// risk).
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<RuleProperties>,
}

impl ReportingDescriptor {
    /// A rule with just an id.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            short_description: None,
            help_uri: None,
            properties: None,
        }
    }

    /// Attach a one-line `shortDescription.text`.
    #[must_use]
    pub fn short_description(mut self, text: impl Into<String>) -> Self {
        self.short_description = Some(MultiformatMessageString { text: text.into() });
        self
    }

    /// Attach a `helpUri`.
    #[must_use]
    pub fn help_uri(mut self, uri: impl Into<String>) -> Self {
        self.help_uri = Some(uri.into());
        self
    }

    /// Tag the rule's analysis tier in the SARIF property bag (ADR-071 §9).
    #[must_use]
    pub fn tier(mut self, tier: impl Into<String>) -> Self {
        self.properties = Some(RuleProperties { tier: tier.into() });
        self
    }
}

#[derive(Debug, Serialize)]
struct RuleProperties {
    tier: String,
}

#[derive(Debug, Serialize)]
struct MultiformatMessageString {
    text: String,
}

/// SARIF result severity (`result.level`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    None,
    Note,
    Warning,
    Error,
}

/// A single `result` in `runs[].results[]`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    rule_id: String,
    level: Level,
    message: Message,
    // Omitted when empty: gate findings are repo-level aggregates with no
    // physical location (SARIFOUT-005).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    locations: Vec<Location>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    suppressions: Vec<Suppression>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    partial_fingerprints: BTreeMap<String, String>,
}

impl SarifResult {
    /// A result for `rule_id` at `level` with the given message text. Add
    /// locations / suppressions / fingerprints with the builder methods.
    #[must_use]
    pub fn new(rule_id: impl Into<String>, level: Level, message: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            level,
            message: Message {
                text: message.into(),
            },
            locations: Vec::new(),
            suppressions: Vec::new(),
            partial_fingerprints: BTreeMap::new(),
        }
    }

    /// Append a physical location.
    #[must_use]
    pub fn location(mut self, location: Location) -> Self {
        self.locations.push(location);
        self
    }

    /// Append a suppression (baseline / `@anvil-ignore` acceptance).
    #[must_use]
    pub fn suppression(mut self, suppression: Suppression) -> Self {
        self.suppressions.push(suppression);
        self
    }

    /// Set a `partialFingerprints` entry.
    #[must_use]
    pub fn fingerprint(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.partial_fingerprints.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Serialize)]
struct Message {
    text: String,
}

/// A `location` with a `physicalLocation`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    physical_location: PhysicalLocation,
}

impl Location {
    /// A physical location at `uri` (repo-relative), optionally with a region.
    #[must_use]
    pub fn new(uri: impl Into<String>, region: Option<Region>) -> Self {
        Self {
            physical_location: PhysicalLocation {
                artifact_location: ArtifactLocation { uri: uri.into() },
                region,
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalLocation {
    artifact_location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
}

#[derive(Debug, Serialize)]
struct ArtifactLocation {
    uri: String,
}

/// A `region` (1-based line, optional 1-based column).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    start_line: NonZeroU32,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_column: Option<NonZeroU32>,
}

impl Region {
    /// A region covering `start_line` (1-based, non-zero per SARIF §3.36).
    #[must_use]
    pub fn line(start_line: NonZeroU32) -> Self {
        Self {
            start_line,
            start_column: None,
        }
    }

    /// Build a region from a 1-based line, rejecting zero.
    #[must_use]
    pub fn try_line(start_line: u32) -> Option<Self> {
        NonZeroU32::new(start_line).map(Self::line)
    }

    /// Add a 1-based `startColumn`, rejecting zero.
    #[must_use]
    pub fn try_column(mut self, start_column: u32) -> Option<Self> {
        self.start_column = NonZeroU32::new(start_column);
        self.start_column.map(|_| self)
    }

    /// Add a 1-based `startColumn` (non-zero).
    #[must_use]
    pub fn column(mut self, start_column: NonZeroU32) -> Self {
        self.start_column = Some(start_column);
        self
    }
}

/// SARIF §3.35 suppression kind.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SuppressionKind {
    /// Suppressed by an in-source marker (e.g. `@anvil-ignore`).
    InSource,
    /// Suppressed by an external store (e.g. the Anvil baseline).
    External,
}

/// A `result.suppressions[]` entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suppression {
    kind: SuppressionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    justification: Option<String>,
}

impl Suppression {
    /// A suppression of the given `kind`.
    #[must_use]
    pub fn new(kind: SuppressionKind) -> Self {
        Self {
            kind,
            justification: None,
        }
    }

    /// Attach a human-readable `justification`.
    #[must_use]
    pub fn justification(mut self, justification: impl Into<String>) -> Self {
        self.justification = Some(justification.into());
        self
    }
}

/// A deterministic `partialFingerprints` value so GitHub Code Scanning dedupes
/// the same finding across runs.
///
/// Stable by construction: the same `(rule_id, uri, line, message)` always
/// yields the same hex digest, independent of run order or machine. SARIF
/// fingerprint values are opaque strings, so a truncated SHA-256 hex digest is
/// sufficient and avoids leaking absolute paths.
#[must_use]
pub fn stable_fingerprint(rule_id: &str, uri: &str, line: Option<u32>, message: &str) -> String {
    use std::fmt::Write as _;
    let mut hasher = Sha256::new();
    // NUL-separate the components so distinct tuples cannot collide by
    // concatenation (e.g. "ab"+"c" vs "a"+"bc").
    hasher.update(rule_id.as_bytes());
    hasher.update([0]);
    hasher.update(uri.as_bytes());
    hasher.update([0]);
    hasher.update(line.unwrap_or(0).to_le_bytes());
    hasher.update([0]);
    hasher.update(message.as_bytes());
    let digest = hasher.finalize();
    // 16 hex chars (64 bits) is ample for dedup keying.
    let mut out = String::with_capacity(16);
    for b in &digest[..8] {
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bundled upstream SARIF 2.1.0 schema (the crate's pub const).
    const SARIF_SCHEMA: &str = super::SARIF_SCHEMA_JSON;

    /// Build a representative document exercising the full pinned subset:
    /// rules with descriptions, a plain result, and a suppressed result with a
    /// region + fingerprints.
    fn sample_log() -> SarifLog {
        let rules = vec![
            ReportingDescriptor::new("ANV-PAT-001")
                .short_description("Example anti-pattern")
                .help_uri("https://github.com/eddacraft/anvil-001"),
            ReportingDescriptor::new("secret-detection"),
        ];
        let results = vec![
            SarifResult::new("ANV-PAT-001", Level::Warning, "Example finding")
                .location(Location::new(
                    "src/lib.rs",
                    Some(
                        Region::line(NonZeroU32::new(42).expect("line"))
                            .column(NonZeroU32::new(5).expect("column")),
                    ),
                ))
                .fingerprint(
                    "anvilFingerprint/v1",
                    stable_fingerprint("ANV-PAT-001", "src/lib.rs", Some(42), "Example finding"),
                ),
            SarifResult::new("secret-detection", Level::Error, "Suppressed at baseline")
                .location(Location::new(
                    "src/config.rs",
                    Some(Region::line(NonZeroU32::new(7).expect("line"))),
                ))
                .suppression(Suppression::new(SuppressionKind::External).justification("baseline")),
        ];
        SarifLog::new(Run::new(rules, results))
    }

    #[test]
    fn zero_line_and_column_are_unrepresentable() {
        assert!(Region::try_line(0).is_none());
        let region = Region::try_line(1).expect("valid line");
        assert!(region.try_column(0).is_none());
    }

    #[test]
    fn sample_document_validates_against_bundled_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(SARIF_SCHEMA).expect("bundled schema is valid JSON");
        let instance = serde_json::to_value(sample_log()).expect("serialise SARIF log");
        let validator = jsonschema::validator_for(&schema).expect("compile SARIF schema");
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        assert!(
            errors.is_empty(),
            "emitted SARIF should validate against the 2.1.0 schema; errors:\n{}",
            errors.join("\n")
        );
    }

    #[test]
    fn pinned_subset_shape_is_stable() {
        // Golden shape for a minimal single-result log — pins field names,
        // camelCase, `$schema`/`version`, and that empty optionals are omitted.
        let log = SarifLog::new(Run::new(
            vec![ReportingDescriptor::new("ANV-PAT-001")],
            vec![
                SarifResult::new("ANV-PAT-001", Level::Warning, "hi").location(Location::new(
                    "a.rs",
                    Some(Region::line(NonZeroU32::new(1).expect("line"))),
                )),
            ],
        ));
        let json = serde_json::to_string_pretty(&log).expect("serialise");
        let expected = format!(
            r#"{{
  "$schema": "{SARIF_SCHEMA_URI}",
  "version": "2.1.0",
  "runs": [
    {{
      "tool": {{
        "driver": {{
          "name": "anvil",
          "informationUri": "{DRIVER_INFORMATION_URI}",
          "version": "{version}",
          "rules": [
            {{
              "id": "ANV-PAT-001"
            }}
          ]
        }}
      }},
      "results": [
        {{
          "ruleId": "ANV-PAT-001",
          "level": "warning",
          "message": {{
            "text": "hi"
          }},
          "locations": [
            {{
              "physicalLocation": {{
                "artifactLocation": {{
                  "uri": "a.rs"
                }},
                "region": {{
                  "startLine": 1
                }}
              }}
            }}
          ]
        }}
      ]
    }}
  ]
}}"#,
            version = env!("CARGO_PKG_VERSION"),
        );
        assert_eq!(json, expected);
    }

    #[test]
    fn fingerprint_is_deterministic_and_sensitive() {
        let a = stable_fingerprint("r", "f.rs", Some(10), "msg");
        let b = stable_fingerprint("r", "f.rs", Some(10), "msg");
        assert_eq!(a, b, "same inputs → same fingerprint");
        assert_eq!(a.len(), 16, "64-bit hex digest");
        assert_ne!(a, stable_fingerprint("r", "f.rs", Some(11), "msg"));
        assert_ne!(a, stable_fingerprint("r2", "f.rs", Some(10), "msg"));
        // NUL-separation: regrouping component boundaries changes the digest.
        assert_ne!(
            stable_fingerprint("ab", "c", None, "m"),
            stable_fingerprint("a", "bc", None, "m"),
        );
    }
}
