//! Provider-agnostic scanner contracts and the chain executor (IORISK-002).
//!
//! A [`Scanner`] inspects an [`IoPayload`] (a model input or output) and returns
//! zero or more [`RiskFinding`]s from the shared taxonomy
//! ([`anvil_kernel_types::io_risk`]). A [`ScannerChain`] runs a registered set
//! of scanners and aggregates their findings.
//!
//! This module ships **contracts only** — the trait, the payload type, the
//! executor, and (test-gated) trivial reference scanners. Concrete heavyweight
//! scanners (regex/ML/secret detectors) are later intake.
//!
//! ## Executor guarantees
//!
//! - **Deterministic order.** Findings are aggregated in scanner registration
//!   order, preserving each scanner's own returned order. No sorting, no
//!   deduplication — the same scanners over the same payload always yield the
//!   same [`ScanReport`].
//! - **Never short-circuits.** Every registered scanner runs, even if an
//!   earlier one panics or returns findings.
//! - **Panic isolation, surfaced not swallowed.** A scanner that panics is
//!   caught via [`std::panic::catch_unwind`] and recorded as an explicit
//!   [`ScannerError`] on the report's separate `scanner_errors` channel; the
//!   remaining scanners still run.
//!
//!   A scanner panic is recorded as an *executor error*, deliberately **not**
//!   as a [`RiskFinding`]: a fault in the detector is an operational problem,
//!   not an IO risk in the taxonomy, and folding it into `findings` would
//!   mis-classify it (and force an ill-fitting [`RiskCategory`]). Keeping a
//!   dedicated `scanner_errors` channel surfaces the failure to the caller
//!   without ever dropping it and without corrupting the risk signal. Effective
//!   only under `panic = "unwind"` (ADR-051); under `panic = "abort"` the
//!   process aborts before the catch runs.

use std::panic::{self, AssertUnwindSafe};

use anvil_kernel_types::io_risk::RiskFinding;
use serde::{Deserialize, Serialize};

/// Whether a payload is a model input or output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IoDirection {
    /// Data flowing into the model (a prompt, tool result, retrieved context).
    Input,
    /// Data flowing out of the model (a response, tool call).
    Output,
}

/// A unit of content presented to the scanner chain.
///
/// Provider-agnostic: `content` is the raw text to scan, `source` is a
/// free-form label identifying where it came from (a prompt id, `"response"`,
/// a file path), and `direction` says which side of the model it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoPayload {
    /// Which side of the model this payload is.
    pub direction: IoDirection,
    /// A free-form label for the payload's origin.
    pub source: String,
    /// The raw content to scan.
    pub content: String,
}

impl IoPayload {
    /// An [`IoDirection::Input`] payload.
    pub fn input(source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            direction: IoDirection::Input,
            source: source.into(),
            content: content.into(),
        }
    }

    /// An [`IoDirection::Output`] payload.
    pub fn output(source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            direction: IoDirection::Output,
            source: source.into(),
            content: content.into(),
        }
    }
}

/// A provider-agnostic IO risk scanner.
///
/// Implementors inspect a payload and return findings from the shared taxonomy.
/// A scanner must be pure with respect to the payload for the chain's
/// determinism guarantee to hold: given the same payload it should return the
/// same findings.
pub trait Scanner {
    /// A stable, human-readable scanner name, used to attribute findings and
    /// errors in a [`ScanReport`].
    fn name(&self) -> &str;

    /// Inspect `payload` and return zero or more findings, in a deterministic
    /// order.
    fn scan(&self, payload: &IoPayload) -> Vec<RiskFinding>;
}

/// An executor-level fault: a scanner that panicked mid-scan.
///
/// Kept separate from [`RiskFinding`]s so a detector fault is never mistaken for
/// an IO risk. Surfaced on [`ScanReport::scanner_errors`], never dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannerError {
    /// The [`Scanner::name`] of the scanner that faulted.
    pub scanner: String,
    /// The captured panic message. UK spelling in executor-authored text.
    pub message: String,
}

/// The outcome of running a [`ScannerChain`]: aggregated findings plus any
/// executor-level scanner faults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    /// Findings in registration order, each scanner's order preserved.
    pub findings: Vec<RiskFinding>,
    /// Scanner faults (panics), in registration order. Empty on a clean run.
    pub scanner_errors: Vec<ScannerError>,
}

impl ScanReport {
    /// Whether the chain ran without any scanner faulting.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.scanner_errors.is_empty()
    }
}

/// A registered, ordered set of scanners run as a chain.
#[derive(Default)]
pub struct ScannerChain {
    scanners: Vec<Box<dyn Scanner>>,
}

impl ScannerChain {
    /// An empty chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a scanner to the chain, preserving registration order.
    pub fn register(&mut self, scanner: Box<dyn Scanner>) -> &mut Self {
        self.scanners.push(scanner);
        self
    }

    /// Builder form of [`register`](Self::register).
    #[must_use]
    pub fn with(mut self, scanner: Box<dyn Scanner>) -> Self {
        self.scanners.push(scanner);
        self
    }

    /// Number of registered scanners.
    #[must_use]
    pub fn len(&self) -> usize {
        self.scanners.len()
    }

    /// Whether the chain has no scanners.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scanners.is_empty()
    }

    /// Run every scanner over `payload` and aggregate the results.
    ///
    /// Runs in registration order, never short-circuits, and isolates a
    /// panicking scanner into [`ScanReport::scanner_errors`]. See the [module
    /// docs](self) for the full guarantee set.
    #[must_use]
    pub fn scan(&self, payload: &IoPayload) -> ScanReport {
        let mut report = ScanReport::default();
        for scanner in &self.scanners {
            // Capture the name before the guarded call so a panic in `scan`
            // can still be attributed.
            let name = scanner.name().to_string();
            match panic::catch_unwind(AssertUnwindSafe(|| scanner.scan(payload))) {
                Ok(findings) => report.findings.extend(findings),
                Err(payload) => {
                    report.scanner_errors.push(ScannerError {
                        scanner: name,
                        message: panic_payload_message(payload.as_ref()),
                    });
                }
            }
        }
        report
    }
}

/// Best-effort message from a `catch_unwind` payload. `panic!` carries a
/// `&'static str` or a `String`; anything else is opaque. Mirrors the helper in
/// the engine facade.
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "opaque panic payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::io_risk::{Confidence, RiskCategory, RiskLocation, RiskSeverity};

    /// Reference scanner: flags every occurrence of a marker substring,
    /// attributing a span. Test-only — a stand-in for a real detector.
    struct SubstringMarkerScanner {
        name: &'static str,
        marker: &'static str,
        category: RiskCategory,
    }

    impl Scanner for SubstringMarkerScanner {
        fn name(&self) -> &str {
            self.name
        }

        fn scan(&self, payload: &IoPayload) -> Vec<RiskFinding> {
            let mut findings = Vec::new();
            let mut from = 0;
            while let Some(rel) = payload.content[from..].find(self.marker) {
                let start = from + rel;
                let end = start + self.marker.len();
                findings.push(
                    RiskFinding::new(
                        self.category,
                        RiskSeverity::High,
                        Confidence::Medium,
                        format!("marker `{}` present in {}", self.marker, payload.source),
                        "Remove or neutralise the flagged marker.",
                    )
                    .with_location(RiskLocation {
                        source: Some(payload.source.clone()),
                        start: u32::try_from(start).ok(),
                        end: u32::try_from(end).ok(),
                    }),
                );
                from = end;
            }
            findings
        }
    }

    /// Reference scanner: always panics. Test-only — exercises isolation.
    struct PanickingScanner;
    impl Scanner for PanickingScanner {
        // The trait ties the returned `&str` to `&self`; a literal name trips
        // `unnecessary_literal_bound` but the signature cannot widen to
        // `&'static str` without diverging from the trait. Test-only.
        #[allow(clippy::unnecessary_literal_bound)]
        fn name(&self) -> &str {
            "panicking"
        }
        fn scan(&self, _payload: &IoPayload) -> Vec<RiskFinding> {
            panic!("scanner boom");
        }
    }

    fn injection_scanner() -> Box<dyn Scanner> {
        Box::new(SubstringMarkerScanner {
            name: "injection-marker",
            marker: "IGNORE PREVIOUS",
            category: RiskCategory::PromptInjection,
        })
    }

    fn secret_scanner() -> Box<dyn Scanner> {
        Box::new(SubstringMarkerScanner {
            name: "secret-marker",
            marker: "AKIA",
            category: RiskCategory::SensitiveDataLeakage,
        })
    }

    #[test]
    fn io_scanner_pipeline_empty_chain_yields_empty_report() {
        let report = ScannerChain::new().scan(&IoPayload::input("prompt", "hello"));
        assert!(report.findings.is_empty());
        assert!(report.is_clean());
    }

    #[test]
    fn io_scanner_pipeline_aggregates_in_registration_order() {
        let chain = ScannerChain::new()
            .with(injection_scanner())
            .with(secret_scanner());
        let payload = IoPayload::input("prompt:user", "IGNORE PREVIOUS then leak AKIA...");
        let report = chain.scan(&payload);

        assert_eq!(report.findings.len(), 2);
        // Registration order: injection scanner's finding first, secret second.
        assert_eq!(report.findings[0].category, RiskCategory::PromptInjection);
        assert_eq!(
            report.findings[1].category,
            RiskCategory::SensitiveDataLeakage
        );
        assert!(report.is_clean());
    }

    #[test]
    fn io_scanner_pipeline_marker_scanner_reports_span() {
        let chain = ScannerChain::new().with(injection_scanner());
        let payload = IoPayload::input("prompt:user", "xx IGNORE PREVIOUS yy");
        let report = chain.scan(&payload);
        let location = report.findings[0].location.as_ref().expect("location");
        assert_eq!(location.source.as_deref(), Some("prompt:user"));
        assert_eq!(location.start, Some(3));
        let marker_len = u32::try_from("IGNORE PREVIOUS".len()).expect("marker length fits u32");
        assert_eq!(location.end, Some(3 + marker_len));
    }

    #[test]
    fn io_scanner_pipeline_reports_every_marker_occurrence() {
        let chain = ScannerChain::new().with(secret_scanner());
        let payload = IoPayload::output("response", "AKIA one AKIA two");
        let report = chain.scan(&payload);
        assert_eq!(
            report.findings.len(),
            2,
            "both occurrences must be reported"
        );
    }

    #[test]
    fn io_scanner_pipeline_isolates_panicking_scanner_without_short_circuit() {
        // A panicking scanner sits between two healthy ones: its panic must be
        // surfaced as an error, and BOTH healthy scanners must still run.
        let chain = ScannerChain::new()
            .with(injection_scanner())
            .with(Box::new(PanickingScanner))
            .with(secret_scanner());
        let payload = IoPayload::input("prompt:user", "IGNORE PREVIOUS and AKIA");
        let report = chain.scan(&payload);

        // Healthy scanners' findings survive the middle scanner's panic.
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].category, RiskCategory::PromptInjection);
        assert_eq!(
            report.findings[1].category,
            RiskCategory::SensitiveDataLeakage
        );

        // The panic is surfaced explicitly, attributed, and not dropped.
        assert!(!report.is_clean());
        assert_eq!(report.scanner_errors.len(), 1);
        assert_eq!(report.scanner_errors[0].scanner, "panicking");
        assert!(
            report.scanner_errors[0].message.contains("scanner boom"),
            "panic payload must be surfaced: {}",
            report.scanner_errors[0].message
        );
    }

    #[test]
    fn io_scanner_pipeline_is_deterministic_across_runs() {
        let chain = ScannerChain::new()
            .with(injection_scanner())
            .with(secret_scanner());
        let payload = IoPayload::input("prompt", "IGNORE PREVIOUS AKIA");
        assert_eq!(chain.scan(&payload), chain.scan(&payload));
    }

    #[test]
    fn io_scanner_pipeline_payload_round_trips_through_json() {
        let payload = IoPayload::output("response", "body");
        let json = serde_json::to_string(&payload).expect("serialise");
        let back: IoPayload = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, payload);
        assert_eq!(serde_json::to_value(IoDirection::Input).unwrap(), "input");
    }
}
