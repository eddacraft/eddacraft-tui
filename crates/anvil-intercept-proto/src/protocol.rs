//! DRVR-002 / DRVR-008: Editor-driver protocol method names and
//! capability vocabulary.
//!
//! This module is the **authoritative** Rust definition of:
//!
//! - The JSON-RPC method-name constants the editor driver and the
//!   daemon agree on (the `ANVIL_*` constants).
//! - The capability lattice the §3.3 state machine moves a driver
//!   through (`Attached` → `Participating`).
//!
//! TS bindings in `packages/anvil-driver-client/src/protocol/` mirror
//! these constants byte-for-byte. The Rust side is the one source of
//! truth; if the two drift, the Rust side wins and the TS side is
//! re-pinned to match.
//!
//! ## Why this lives in `anvil-intercept-proto`
//!
//! The proto crate already owns the wire vocabulary the daemon and
//! launcher share (`IpcCommand`, `IpcEnvelope`, `SessionRecord`).
//! Method names are wire vocabulary; they belong with their siblings.
//! Putting them in `anvil-intercept` proper would force every
//! consumer (e.g. `auth.rs`'s capability negotiation) to depend on
//! the daemon binary's runtime crate, and any future
//! Wasm-/embedded-side daemon implementation would have to re-export
//! the names from a different crate. Keeping them in `proto` makes
//! the constants importable everywhere with no extra dependency.
//!
//! ## Method namespace policy
//!
//! Per §3.2 of the editor-and-mcp-driver design spec, **no new
//! `anvil/` method without a concrete editor feature that cannot be
//! expressed in stock LSP**. Every method below has a v1 consumer:
//!
//! - `anvil/publishDiagnostics` — server → client notification, the
//!   Anvil flavour of LSP `textDocument/publishDiagnostics` carrying
//!   `Diagnostic` from `anvil-kernel-types` rather than the LSP
//!   shape (so suppression / mode / category survive).
//! - `anvil/scan_buffer` — client → server request, the mid-edit
//!   buffer scan path. Companion to the existing `scan_buffer` JSON-RPC
//!   method; the `anvil/`-namespaced alias is what drivers advertise
//!   in their manifest.
//! - `anvil/enforcement/ack` — client → server, confirms an
//!   enforcement decision was carried out. Drivers that do not
//!   advertise this method are capped at read-only per DRVR-008.
//! - `anvil/gate/request` — client → server, asks for a gate-result
//!   stream over the telemetry lane. Resolves the M3 council-review
//!   item that `anvil/gate/request` was missing from the §3.2 method
//!   table while §3.7 referenced it.
//! - `anvil/suppression/apply` — client → server, requests the
//!   daemon to validate and normalise a `@anvil-ignore` comment per
//!   ADR-004.
//! - `anvil/status/query` — client → server, returns the current
//!   session / fence / driver state for a worktree.
//!
//! LSP methods (`textDocument/publishDiagnostics`,
//! `textDocument/codeAction`, `workspace/applyEdit`,
//! `window/showMessage`, `initialize`/`initialized`) are pinned by
//! the LSP spec and are not re-declared here. Drivers speak both
//! languages over the same transport; the daemon routes by method
//! name at the JSON-RPC layer.

use serde::{Deserialize, Serialize};

/// Shared wire envelope for the set of diagnostics a scan response
/// carries on `params.diagnostics`. Each element is the canonical
/// `anvil.diagnostic.v1` shape (`anvil_kernel_types::Diagnostic`); see
/// `crates/anvil-kernel-types/src/diagnostics.rs`.
///
/// Owned here in `anvil-intercept-proto` so the `scan_buffer` response
/// ([`ScanBufferResponse`] in `anvil-intercept`) and the ADR-061
/// Sub-phase A `validate_paths` response type their `diagnostics`
/// field against the **same** type. This closes council finding **B3**
/// (2026-06-01 daemon-graph verdict): Task 1 of the save-time plan
/// "froze" a wire that named a phantom `ScanDiagnostics` the proto
/// crate did not own, and the real type (`ScanBufferResponse`) was
/// declared daemon-local — exactly the drift this alias removes.
///
/// Lighter form per the B3/C5 ruling: a type alias for
/// `Vec<anvil_kernel_types::Diagnostic>` rather than a wrapping struct,
/// and no re-export of `Diagnostic` (consumers name the kernel type
/// directly — one canonical path). Full envelope unification (a single
/// redaction guard hung off one struct) is deferred to Sub-phase A′.
pub type DiagnosticEnvelope = Vec<anvil_kernel_types::Diagnostic>;

/// Server → client notification carrying [`Diagnostic`] payloads. The
/// outer wrapper is the JSON-RPC notification envelope; the inner
/// `params.diagnostics` array holds the canonical
/// `anvil.diagnostic.v1` shape (see
/// `crates/anvil-kernel-types/src/diagnostics.rs`).
///
/// Distinct from LSP's `textDocument/publishDiagnostics` because the
/// payload preserves Anvil's `mode`, `category`, `suppression`, and
/// `correlationId` fields that the LSP shape would drop. Drivers
/// that want LSP rendering MUST translate locally; the daemon does
/// not emit a stock-LSP variant.
pub const ANVIL_PUBLISH_DIAGNOSTICS: &str = "anvil/publishDiagnostics";

/// Client → server request: scan a mid-edit buffer for diagnostics.
/// Companion to the existing `scan_buffer` method; the
/// `anvil/`-namespaced form is what drivers advertise in their
/// manifest so capability negotiation can confirm both ends speak
/// the namespaced form. Consumers of the legacy `scan_buffer` method
/// continue to work — both names route to the same handler.
pub const ANVIL_SCAN_BUFFER: &str = "anvil/scan_buffer";

/// Client → server: confirms an enforcement decision was carried
/// out. **DRVR-008's central method:** drivers that do not advertise
/// support for this method cannot be promoted past
/// [`Capability::Attached`] regardless of `.anvil.yaml` requesting
/// participation.
pub const ANVIL_ENFORCEMENT_ACK: &str = "anvil/enforcement/ack";

/// Client → server: asks the daemon to start streaming gate-result
/// snapshots over the telemetry lane (or, for one-shot consumers,
/// returns a single snapshot synchronously). Resolves the M3
/// council-review item.
pub const ANVIL_GATE_REQUEST: &str = "anvil/gate/request";

/// Client → server: requests the daemon validate and normalise a
/// `@anvil-ignore` comment per ADR-004. The driver supplies the
/// proposed comment + range + reason; the daemon returns the
/// normalised comment which the driver applies via
/// `workspace/applyEdit`.
pub const ANVIL_SUPPRESSION_APPLY: &str = "anvil/suppression/apply";

/// Client → server: returns current session / fence / driver state
/// for a worktree. Single-snapshot read; subscription form lives on
/// the telemetry lane.
pub const ANVIL_STATUS_QUERY: &str = "anvil/status/query";

/// Capability lattice for the §3.3 state machine.
///
/// `Attached` is the read-only floor: every successfully-handshaken
/// driver reaches this state. `Participating` is the
/// enforcement-candidate state: drivers that have passed the
/// allowlist gate (DRVR-007) AND advertise
/// [`ANVIL_ENFORCEMENT_ACK`] (DRVR-008) can be promoted to it.
///
/// **Order matters.** The enum derives `Ord` so callers can compare
/// "is requested capability higher than what the manifest allows"
/// without re-implementing the lattice; v1 only has two states so
/// the comparison is trivial, but future capability tiers (e.g.
/// `Trusted` for cross-host drivers) extend the lattice rather than
/// rewriting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Read-only diagnostic mode. Default for every driver after
    /// successful handshake. Subscribes to telemetry, renders
    /// diagnostics, applies suppression edits — but never acks
    /// enforcement decisions and is never escalated to fence on
    /// refusal.
    Attached,
    /// Enforcement-participating mode. Receives
    /// `enforcement.decision` events; ack-or-refuse contract per
    /// §2.5; subject to the reliability budget in §2.6. Reaching
    /// this state requires BOTH the DRVR-007 allowlist check AND
    /// the DRVR-008 method advertisement.
    Participating,
}

impl Capability {
    /// Wire string for log / telemetry emission. Kebab-case to match
    /// the rest of the daemon's structured-log vocabulary.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::Participating => "participating",
        }
    }
}

/// Convenience: every `anvil/` method this protocol version defines.
/// Useful for tests and for documentation generation; consumers
/// negotiating capability use the named constants directly.
pub const ALL_ANVIL_METHODS: &[&str] = &[
    ANVIL_PUBLISH_DIAGNOSTICS,
    ANVIL_SCAN_BUFFER,
    ANVIL_ENFORCEMENT_ACK,
    ANVIL_GATE_REQUEST,
    ANVIL_SUPPRESSION_APPLY,
    ANVIL_STATUS_QUERY,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the wire strings. These constants are part of the
    /// daemon ↔ driver contract; changing them is a breaking
    /// protocol change and requires bumping `protocolVersion`.
    #[test]
    fn anvil_method_names_are_stable() {
        assert_eq!(ANVIL_PUBLISH_DIAGNOSTICS, "anvil/publishDiagnostics");
        assert_eq!(ANVIL_SCAN_BUFFER, "anvil/scan_buffer");
        assert_eq!(ANVIL_ENFORCEMENT_ACK, "anvil/enforcement/ack");
        assert_eq!(ANVIL_GATE_REQUEST, "anvil/gate/request");
        assert_eq!(ANVIL_SUPPRESSION_APPLY, "anvil/suppression/apply");
        assert_eq!(ANVIL_STATUS_QUERY, "anvil/status/query");
    }

    #[test]
    fn all_anvil_methods_lists_every_constant_exactly_once() {
        let listed: std::collections::HashSet<&str> = ALL_ANVIL_METHODS.iter().copied().collect();
        assert_eq!(
            listed.len(),
            ALL_ANVIL_METHODS.len(),
            "ALL_ANVIL_METHODS must not contain duplicates"
        );
        // Every named constant is in the listed set.
        for method in [
            ANVIL_PUBLISH_DIAGNOSTICS,
            ANVIL_SCAN_BUFFER,
            ANVIL_ENFORCEMENT_ACK,
            ANVIL_GATE_REQUEST,
            ANVIL_SUPPRESSION_APPLY,
            ANVIL_STATUS_QUERY,
        ] {
            assert!(
                listed.contains(method),
                "ALL_ANVIL_METHODS missing {method}"
            );
        }
    }

    #[test]
    fn capability_serialises_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Capability::Attached).unwrap(),
            "\"attached\""
        );
        assert_eq!(
            serde_json::to_string(&Capability::Participating).unwrap(),
            "\"participating\""
        );
    }

    #[test]
    fn capability_round_trips_through_json() {
        for variant in [Capability::Attached, Capability::Participating] {
            let s = serde_json::to_string(&variant).unwrap();
            let back: Capability = serde_json::from_str(&s).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn capability_lattice_orders_attached_below_participating() {
        // Lattice property used by negotiate_capability: requested
        // > granted means a downgrade fired. Pin the relation here so
        // a future enum reordering trips the test instead of the
        // daemon silently letting a manifest cap a driver above its
        // request.
        assert!(Capability::Attached < Capability::Participating);
    }

    #[test]
    fn capability_as_str_matches_serde() {
        // Hand-rolled `as_str` for log emission must agree with the
        // serde rename. Easy to drift when adding a new variant; this
        // test pins the invariant.
        for variant in [Capability::Attached, Capability::Participating] {
            let from_serde = serde_json::to_value(variant).unwrap();
            assert_eq!(from_serde, variant.as_str());
        }
    }

    fn sample_diagnostic() -> anvil_kernel_types::Diagnostic {
        use anvil_kernel_types::diagnostics::{
            Category, DiagnosticSource, KnownMode, Location, Severity,
        };
        use anvil_kernel_types::{Diagnostic, Mode};

        Diagnostic::new(
            "AP-001",
            Severity::Warning,
            "sample finding",
            Location {
                file: "src/lib.rs".to_string(),
                line: Some(12),
                column: Some(3),
                end_line: None,
                end_column: None,
            },
            Category::Antipattern,
            DiagnosticSource {
                rule_id: "AP-001".to_string(),
                source_module: "anvil-checks::antipattern".to_string(),
            },
            Mode::known(KnownMode::MidEdit),
        )
    }

    /// B3: the envelope is the canonical `anvil.diagnostic.v1` array,
    /// owned here in the proto crate (not re-declared daemon-local) so
    /// `scan_buffer` and `validate_paths` reference one type.
    #[test]
    fn diagnostic_envelope_serialises_as_canonical_diagnostic_array() {
        let envelope: DiagnosticEnvelope = vec![sample_diagnostic()];
        let json = serde_json::to_value(&envelope).expect("serialise envelope");
        assert!(json.is_array(), "envelope serialises as a JSON array");
        assert_eq!(json[0]["schema_version"], "anvil.diagnostic.v1");
        assert_eq!(json[0]["id"], "AP-001");
        assert_eq!(json[0]["severity"], "warning");
        assert_eq!(json[0]["category"], "antipattern");
    }

    #[test]
    fn diagnostic_envelope_round_trips_through_json() {
        let envelope: DiagnosticEnvelope = vec![sample_diagnostic()];
        let wire = serde_json::to_string(&envelope).expect("serialise");
        let back: DiagnosticEnvelope = serde_json::from_str(&wire).expect("deserialise");
        assert_eq!(envelope, back);
    }
}
