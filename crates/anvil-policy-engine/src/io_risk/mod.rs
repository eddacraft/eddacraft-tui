//! IO risk controls (IORISK): provider-agnostic input/output scanning.
//!
//! Scanners produce [`anvil_kernel_types::io_risk::RiskFinding`]s from the
//! shared taxonomy; the guidance layer maps them to remediation-first,
//! posture-parameterised output for packs and CI summaries.
//!
//! - [`pipeline`] — the [`Scanner`](pipeline::Scanner) contract and the
//!   deterministic [`ScannerChain`](pipeline::ScannerChain) executor
//!   (IORISK-002).

pub mod pipeline;

pub use pipeline::{IoDirection, IoPayload, ScanReport, Scanner, ScannerChain, ScannerError};
