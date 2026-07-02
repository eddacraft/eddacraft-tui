/**
 * Policy module exports
 *
 * OPA integration for flexible policy evaluation using Rego language.
 */

export { OPABinaryManager, getOPABinaryManager } from './opa-binary-manager.js';
export type { OPABinaryConfig, BinaryInfo } from './opa-binary-manager.js';

export { PolicyLoader } from './policy-loader.js';
export type { LoadedPolicy, PolicyDiscoveryResult, PolicyLoaderConfig } from './policy-loader.js';

export { OPAExecutor, OPA_DENIED_BUILTINS } from './opa-executor.js';
export type {
  OPAInput,
  PolicyViolation,
  OPAEvaluationResult,
  OPAExecutorConfig,
  ViolationCategory,
} from './opa-executor.js';

export { BundleManager, getBundleManager } from './bundle-manager.js';
export type {
  BundleConfig,
  BundleAuthConfig,
  BundleCacheEntry,
  BundleManagerConfig,
  BundleSyncResult,
} from './bundle-manager.js';

export { BundleVerifier, loadKeyFromFile } from './bundle-verifier.js';
export type {
  VerificationResult,
  PublicKeyConfig,
  BundleVerifierConfig,
  SignatureManifest,
  SignatureAlgorithm,
} from './bundle-verifier.js';
