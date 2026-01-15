/**
 * @anvil/core
 *
 * Pure domain logic for the Anvil system.
 * Contains antipattern detection, architecture analysis, drift detection,
 * suppression management, validation, and other core functionality.
 *
 * This package has NO I/O operations - all I/O is handled by @anvil/runtime.
 *
 * @module @anvil/core
 */

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
