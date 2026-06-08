//! Diagnostics collector (GITGOV-008, ADR-074 + ADR-058).
//!
//! Renders a capsule's diagnostics into `diagnostics.sarif` — a SARIF
//! 2.1.0 document — using the **shared** [`anvil_sarif`] emitter rather
//! than a capsule-local SARIF shape. This is the review capsule's
//! per-producer SARIF adapter in ADR-058 terms: it maps the canonical
//! [`Diagnostic`] (`anvil.diagnostic.v1`, the shape Anvil's checks and
//! daemon already emit) into the emitter's types. No unified in-process
//! finding model is introduced (ADR-058); no SARIF document shape is
//! re-modelled here (ADR-074 §Schema rules).
//!
//! **Present-but-empty discipline.** When there are no diagnostics the
//! collector still produces a complete, schema-valid SARIF document — a
//! single `anvil` run with empty `rules[]`/`results[]` — never a 0-byte
//! file or an omitted one. A missing `diagnostics.sarif` is a tamper
//! signal; an empty *document* is the honest "nothing to report".
//!
//! **v0 source.** No diagnostics source is wired into `anvil capsule
//! create` yet, so the CLI passes an empty slice and every v0 capsule
//! carries the empty document above. The mapping is implemented and
//! tested in full so that, once a source lands (a verify-time check
//! pass), results flow through unchanged — the collector is the
//! finished adapter, only its input is deferred.

use std::collections::BTreeSet;

use anvil_kernel_types::diagnostics::{Diagnostic, Severity};
use anvil_sarif::{
    Level, Location, Region, ReportingDescriptor, Run, SarifLog, SarifResult, stable_fingerprint,
};

use crate::errors::CapsuleError;

/// `partialFingerprints` key for the capsule's deterministic dedup
/// fingerprint — namespaced + versioned so the scheme can evolve.
const FINGERPRINT_KEY: &str = "anvilCapsule/v1";

/// The rendered `diagnostics.sarif` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedDiagnostics {
    /// Serialised SARIF 2.1.0 bytes written to `diagnostics.sarif`.
    /// Always a complete document — empty `results[]` when there are no
    /// diagnostics, never an empty byte stream.
    pub sarif: Vec<u8>,
}

/// Render `diagnostics` into a SARIF 2.1.0 document via the shared
/// [`anvil_sarif`] emitter.
///
/// Output is **byte-deterministic** for a given set of diagnostics,
/// independent of input order: rules are emitted as the sorted set of
/// distinct `source.rule_id`s, and results are sorted by
/// `(rule_id, file, line, column, summary, id)` — the per-instance `id`
/// is the final tie-breaker so the order is total even when two findings
/// match on every other key. SARIF result order carries no meaning, so
/// sorting is a free way to make the capsule digest reproducible
/// regardless of how a future source enumerates findings.
///
/// # Errors
///
/// [`CapsuleError::Serialise`] if the document cannot be encoded
/// (practically unreachable for emitter-built values).
pub fn collect_diagnostics(
    diagnostics: &[Diagnostic],
) -> Result<CollectedDiagnostics, CapsuleError> {
    // Distinct rule ids, sorted — the SARIF `tool.driver.rules[]`.
    let rule_ids: BTreeSet<&str> = diagnostics
        .iter()
        .map(|d| d.source.rule_id.as_str())
        .collect();
    let rules: Vec<ReportingDescriptor> =
        rule_ids.into_iter().map(ReportingDescriptor::new).collect();

    // Sort results into a deterministic order so the same finding set
    // always serialises to the same bytes. The per-instance `id` is the
    // final tie-breaker: it gives a total order even when two findings
    // share rule/file/line/column/summary (e.g. the same rule firing at
    // one location in two modes), so input order can never leak into the
    // bytes.
    let mut ordered: Vec<&Diagnostic> = diagnostics.iter().collect();
    ordered.sort_by(|a, b| {
        (
            &a.source.rule_id,
            &a.location.file,
            a.location.line,
            a.location.column,
            &a.summary,
            &a.id,
        )
            .cmp(&(
                &b.source.rule_id,
                &b.location.file,
                b.location.line,
                b.location.column,
                &b.summary,
                &b.id,
            ))
    });

    let results: Vec<SarifResult> = ordered.into_iter().map(diagnostic_to_result).collect();

    let log = SarifLog::new(Run::new(rules, results));
    // `serde_json::to_vec` (not the capsule's `canonical_json_bytes`) is
    // byte-deterministic here *because* the emitter's type graph is all
    // named structs (fields serialise in declaration order) plus one
    // `BTreeMap` (`partialFingerprints`, sorted). It keeps the
    // conventional SARIF key order consumers expect. Invariant: if the
    // emitter ever gains a `serde_json::Map`/`HashMap` field this stops
    // holding — re-establish ordering before relying on the digest.
    let sarif = serde_json::to_vec(&log).map_err(|e| CapsuleError::Serialise(e.to_string()))?;
    Ok(CollectedDiagnostics { sarif })
}

/// Map a single [`Diagnostic`] to a SARIF `result`.
///
/// v0 maps the GitHub Code Scanning subset the emitter models: rule id,
/// level, message, and a `region` from `line`/`column`. Fields outside
/// that subset are intentionally not carried — `end_line`/`end_column`
/// (the emitter's `Region` has no end), and `category`/`mode`/
/// `source_module`/`id`/`remediation_hint` (no SARIF slot without a
/// `properties` bag). A later iteration that needs them widens the
/// emitter and this mapper together (GITGOV-009+); none is silently lost
/// today because v0 carries no real diagnostics.
fn diagnostic_to_result(diag: &Diagnostic) -> SarifResult {
    let region = diag.location.line.map(|line| {
        let region = Region::line(line);
        match diag.location.column {
            Some(column) => region.column(column),
            None => region,
        }
    });
    let location = Location::new(diag.location.file.clone(), region);

    SarifResult::new(
        diag.source.rule_id.clone(),
        severity_to_level(diag.severity),
        diag.summary.clone(),
    )
    .location(location)
    .fingerprint(
        FINGERPRINT_KEY,
        stable_fingerprint(
            &diag.source.rule_id,
            &diag.location.file,
            diag.location.line,
            &diag.summary,
        ),
    )
}

/// Map diagnostic [`Severity`] to a SARIF result [`Level`].
///
/// `Info` is SARIF `note` (the spec's lowest non-suppressed level);
/// `Warning`/`Error` map straight across.
fn severity_to_level(severity: Severity) -> Level {
    match severity {
        Severity::Info => Level::Note,
        Severity::Warning => Level::Warning,
        Severity::Error => Level::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::diagnostics::{
        Category, DiagnosticSource, KnownMode, Location as DiagLocation, Mode,
    };
    use serde_json::Value;

    fn diag(
        rule_id: &str,
        severity: Severity,
        file: &str,
        line: Option<u32>,
        summary: &str,
    ) -> Diagnostic {
        Diagnostic::new(
            format!("diag_{rule_id}_{}", line.unwrap_or(0)),
            severity,
            summary,
            DiagLocation {
                file: file.to_string(),
                line,
                column: line.map(|_| 3),
                end_line: None,
                end_column: None,
            },
            Category::Secret,
            DiagnosticSource {
                rule_id: rule_id.to_string(),
                source_module: "anvil-checks::test".to_string(),
            },
            Mode::known(KnownMode::Gate),
        )
    }

    fn parse(collected: &CollectedDiagnostics) -> Value {
        serde_json::from_slice(&collected.sarif).expect("diagnostics.sarif is valid JSON")
    }

    #[test]
    fn collect_diagnostics_empty_is_complete_but_resultless() {
        let collected = collect_diagnostics(&[]).unwrap();
        assert!(
            !collected.sarif.is_empty(),
            "a document, never a 0-byte file"
        );

        let doc = parse(&collected);
        assert_eq!(doc["version"], "2.1.0");
        let run = &doc["runs"][0];
        assert_eq!(run["tool"]["driver"]["name"], "anvil");
        assert!(run["results"].as_array().unwrap().is_empty());
        assert!(
            run["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn collect_diagnostics_maps_a_finding_to_a_result() {
        let collected = collect_diagnostics(&[diag(
            "secret-aws-key",
            Severity::Error,
            "src/config.rs",
            Some(7),
            "Hardcoded AWS key",
        )])
        .unwrap();

        let doc = parse(&collected);
        let result = &doc["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "secret-aws-key");
        assert_eq!(result["level"], "error");
        assert_eq!(result["message"]["text"], "Hardcoded AWS key");
        let loc = &result["locations"][0]["physicalLocation"];
        assert_eq!(loc["artifactLocation"]["uri"], "src/config.rs");
        assert_eq!(loc["region"]["startLine"], 7);
        assert_eq!(loc["region"]["startColumn"], 3);
        // The rule is registered in the driver.
        assert_eq!(
            doc["runs"][0]["tool"]["driver"]["rules"][0]["id"],
            "secret-aws-key"
        );
        // A deterministic fingerprint is attached under the namespaced key.
        assert!(result["partialFingerprints"][FINGERPRINT_KEY].is_string());
    }

    #[test]
    fn collect_diagnostics_maps_severity_to_sarif_level() {
        for (severity, level) in [
            (Severity::Info, "note"),
            (Severity::Warning, "warning"),
            (Severity::Error, "error"),
        ] {
            let collected =
                collect_diagnostics(&[diag("r", severity, "a.rs", Some(1), "m")]).unwrap();
            let doc = parse(&collected);
            assert_eq!(doc["runs"][0]["results"][0]["level"], level);
        }
    }

    #[test]
    fn collect_diagnostics_dedupes_rules_in_driver() {
        // Two findings of the same rule → one rule descriptor, two results.
        let collected = collect_diagnostics(&[
            diag("dup-rule", Severity::Warning, "a.rs", Some(1), "first"),
            diag("dup-rule", Severity::Warning, "b.rs", Some(2), "second"),
        ])
        .unwrap();
        let doc = parse(&collected);
        assert_eq!(
            doc["runs"][0]["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(doc["runs"][0]["results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn collect_diagnostics_path_only_finding_omits_region() {
        let collected = collect_diagnostics(&[diag(
            "path-rule",
            Severity::Info,
            "README.md",
            None,
            "path-only",
        )])
        .unwrap();
        let doc = parse(&collected);
        let loc = &doc["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
        assert_eq!(loc["artifactLocation"]["uri"], "README.md");
        assert!(loc.get("region").is_none(), "no line → no region");
    }

    #[test]
    fn collect_diagnostics_is_order_independent_and_deterministic() {
        let a = diag("rule-a", Severity::Error, "a.rs", Some(1), "first");
        let b = diag("rule-b", Severity::Warning, "b.rs", Some(2), "second");

        let forward = collect_diagnostics(&[a.clone(), b.clone()]).unwrap();
        let reversed = collect_diagnostics(&[b, a]).unwrap();
        assert_eq!(
            forward, reversed,
            "result order is sorted, so input order cannot change the bytes"
        );
    }

    /// Two findings identical on every sort key except `id` (e.g. one
    /// rule firing at one location in two modes) still sort to a stable
    /// total order via the `id` tie-breaker — input order cannot leak.
    #[test]
    fn collect_diagnostics_tie_breaks_on_id_for_same_location_findings() {
        let mut a = diag("same-rule", Severity::Warning, "x.rs", Some(5), "same text");
        let mut b = a.clone();
        a.id = "diag_aaa".to_string();
        b.id = "diag_bbb".to_string();

        let forward = collect_diagnostics(&[a.clone(), b.clone()]).unwrap();
        let reversed = collect_diagnostics(&[b, a]).unwrap();
        assert_eq!(forward, reversed, "id breaks the tie deterministically");
    }

    /// The emitted document — empty and with results — validates against
    /// the bundled SARIF 2.1.0 schema, the same gate the emitter crate
    /// and the CLI adapters use (parity for the capsule producer).
    #[test]
    fn collect_diagnostics_output_validates_against_sarif_schema() {
        let schema: Value = serde_json::from_str(anvil_sarif::SARIF_SCHEMA_JSON)
            .expect("bundled schema is valid JSON");
        let validator = jsonschema::validator_for(&schema).expect("compile SARIF schema");

        for diagnostics in [
            Vec::new(),
            vec![diag("r", Severity::Error, "src/a.rs", Some(9), "finding")],
        ] {
            let collected = collect_diagnostics(&diagnostics).unwrap();
            let instance: Value = serde_json::from_slice(&collected.sarif).unwrap();
            let errors: Vec<String> = validator
                .iter_errors(&instance)
                .map(|e| format!("{e} at {}", e.instance_path()))
                .collect();
            assert!(
                errors.is_empty(),
                "diagnostics.sarif must validate against SARIF 2.1.0; errors:\n{}",
                errors.join("\n")
            );
        }
    }
}
