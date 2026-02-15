/**
 * @eddacraft/anvil-core
 *
 * Core domain logic for the Anvil system.
 * Contains antipattern detection, architecture analysis, drift detection,
 * suppression management, validation, and other core functionality.
 *
 * Note: Some modules perform filesystem I/O (provenance, drift snapshots,
 * architecture baseline, suppression store). Heavy I/O orchestration
 * (gate runner, caching, OPA execution) lives in @eddacraft/anvil-runtime.
 *
 * @module @eddacraft/anvil-core
 */

// Contracts (schemas, types, events) — formerly @eddacraft/anvil-contracts
export * from './contracts/index.js';

// Platform config — formerly @eddacraft/anvil-platform-config
export * from './config/index.js';

// Antipattern detection
export * from './antipattern/index.js';

// Suppression management
export * from './suppression/index.js';

// Architecture analysis
export * from './architecture/index.js';

// Drift detection
export * from './drift/index.js';

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
