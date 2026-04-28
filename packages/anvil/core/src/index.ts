/**
 * @eddacraft/anvil-core
 *
 * Core domain logic for the Anvil system.
 * Contains architecture analysis, drift detection, validation,
 * and other core functionality.
 *
 * Note: Anti-pattern detection and suppression parsing were archived
 * to `archive/anvil-ts-scanner/` under ADR-033 (2026-04-29). The
 * Rust scanner (`crates/anvil-checks/`) is now the sole engine.
 *
 * Note: Some modules perform filesystem I/O (provenance, drift snapshots,
 * architecture baseline). Heavy I/O orchestration (caching, OPA
 * execution) lives in @eddacraft/anvil-runtime.
 *
 * @module @eddacraft/anvil-core
 */

// Contracts (schemas, types, events) — formerly @eddacraft/anvil-contracts
export * from './contracts/index.js';

// Platform config — formerly @eddacraft/anvil-platform-config
export * from './config/index.js';

// Antipattern detection + suppression parsing archived under ADR-033
// → archive/anvil-ts-scanner/. Rust scanner is now authoritative.

// Architecture analysis
export * from './architecture/index.js';

// Drift detection archived under ADR-033 → archive/anvil-ts-scanner/core-drift/.
// Drift was scoped to anti-pattern + suppression deltas; with both archived,
// the module has nothing to capture.

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
