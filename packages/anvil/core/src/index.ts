/**
 * @eddacraft/anvil-core
 *
 * Core domain logic for the Anvil system. Currently exposes
 * contracts (schemas / types / events), platform config,
 * architecture analysis, provenance tracking, warning utilities,
 * the architecture-rule explain service, validation helpers, and
 * crypto / utils.
 *
 * Note: the TS components that previously provided anti-pattern
 * detection, suppression parsing, and drift snapshot/compare were
 * archived to `anvil-archive/anvil-ts-scanner/` under ADR-033 (2026-04-29).
 * For anti-pattern detection and suppression handling, the Rust
 * scanner (`crates/anvil-checks/`) is now authoritative — invoke via
 * the `anvil` CLI / RMCP. Drift snapshot/compare has no current
 * implementation; reintroduction (likely in Rust) is out of scope
 * for this archive.
 *
 * Note: Some modules perform filesystem I/O (provenance,
 * architecture baseline). Heavy I/O orchestration (caching, OPA
 * execution) lives in @eddacraft/anvil-runtime.
 *
 * @module @eddacraft/anvil-core
 */

// Contracts (schemas, types, events) — formerly @eddacraft/anvil-contracts
export * from './contracts/index.js';

// Platform config — formerly @eddacraft/anvil-platform-config
export * from './config/index.js';

// The TS anti-pattern detector and suppression parser were archived
// under ADR-033 → anvil-archive/anvil-ts-scanner/. The capabilities are
// now served by the Rust scanner; this package no longer exposes
// them.

// Architecture analysis
export * from './architecture/index.js';

// The TS drift snapshot/compare components were archived under
// ADR-033 → anvil-archive/anvil-ts-scanner/core-drift/. They were coupled
// to the archived anti-pattern + suppression layers; no active
// replacement is shipped here. Drift detection as a capability may
// return in Rust on the daemon path; not in scope for this archive.

// Provenance tracking
export * from './provenance/index.js';

// Warning utilities
export * from './warnings/index.js';

// Explain functionality
export * from './explain/index.js';

// Validation
export * from './validation/index.js';

// Crypto utilities
export * from './crypto/index.js';

// General utilities
export * from './utils/index.js';
