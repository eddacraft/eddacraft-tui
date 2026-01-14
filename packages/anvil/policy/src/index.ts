/**
 * @anvil/policy
 *
 * OPA/Rego wrappers for policy evaluation.
 * Contains OPA binary manager, executor, policy loader, and bundle management.
 *
 * @module @anvil/policy
 */

// OPA binary management
export * from './opa-binary-manager.js';

// OPA executor
export * from './opa-executor.js';

// Policy loading
export * from './policy-loader.js';

// Bundle management
export * from './bundle-manager.js';

// Bundle verification
export * from './bundle-verifier.js';

// Types
export * from './types.js';
