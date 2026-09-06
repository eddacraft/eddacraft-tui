//! DPO-007: MCP pre-write → Kindling `gate_evaluated` producer.

use std::sync::OnceLock;

use anvil_intercept::kindling_observation::{ObservationContext, from_prewrite_diagnostics};
use anvil_kernel_types::Diagnostic;
use chrono::Utc;
use uuid::Uuid;

/// Emit a `pre-write` `gate_evaluated` row when `diagnostics` is non-empty.
/// Never changes the caller’s validation decision.
pub(crate) fn emit_prewrite_findings(path: &str, diagnostics: &[Diagnostic]) {
    if diagnostics.is_empty() {
        return;
    }
    let session_id = process_session_id();
    let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let gate_eval_id = Uuid::new_v4().to_string();
    let ctx = ObservationContext {
        session_id,
        timestamp: &timestamp,
        gate_eval_id: &gate_eval_id,
        file_path: path,
        duration_ms: 0,
    };
    let include_paths =
        std::env::var_os("ANVIL_OBSERVATION_INCLUDE_PATHS").is_some_and(|value| value == "1");
    let Some(observation) = from_prewrite_diagnostics(&ctx, diagnostics, include_paths) else {
        return;
    };
    crate::usage::emit_selected_gate_evaluated(observation);
}

fn process_session_id() -> &'static str {
    static SESSION: OnceLock<String> = OnceLock::new();
    SESSION.get_or_init(|| Uuid::new_v4().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_intercept::kindling_observation::PREWRITE_GATE_ID;
    use anvil_kernel_types::diagnostics::Severity;
    use anvil_kernel_types::{Category, DiagnosticSource, Location, Mode};

    fn diag(rule: &str, severity: Severity) -> Diagnostic {
        Diagnostic::new(
            format!("diag_{rule}"),
            severity,
            "finding",
            Location {
                file: "src/lib.rs".to_string(),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Other,
            DiagnosticSource {
                rule_id: rule.to_string(),
                source_module: "anvil-cli::mcp".to_string(),
            },
            Mode::Unknown("pre-write".to_string()),
        )
    }

    #[test]
    fn builder_is_silent_on_empty_and_tagged_on_warning() {
        let timestamp = "2026-09-06T00:00:00.000Z";
        let ctx = ObservationContext {
            session_id: "00000000-0000-4000-8000-000000000000",
            timestamp,
            gate_eval_id: "eval-1",
            file_path: "src/lib.rs",
            duration_ms: 0,
        };
        assert!(from_prewrite_diagnostics(&ctx, &[], false).is_none());
        let obs = from_prewrite_diagnostics(&ctx, &[diag("AP-001", Severity::Warning)], false)
            .expect("row");
        assert_eq!(obs.gate_id, PREWRITE_GATE_ID);
    }
}
