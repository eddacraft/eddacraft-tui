/**
 * Policy module exports
 *
 * Re-exports from @eddacraft/anvil-policy to avoid code duplication.
 * OPA integration for flexible policy evaluation using Rego language.
 *
 * Note: SignatureAlgorithm is intentionally not re-exported here as it is
 * already exported from @eddacraft/anvil-contracts via gate.types.
 */

export {
  OPABinaryManager,
  getOPABinaryManager,
  PolicyLoader,
  OPAExecutor,
  BundleManager,
  getBundleManager,
  BundleVerifier,
  loadKeyFromFile,
} from '@eddacraft/anvil-policy';

export type {
  OPABinaryConfig,
  BinaryInfo,
  LoadedPolicy,
  PolicyDiscoveryResult,
  PolicyLoaderConfig,
  OPAInput,
  PolicyViolation,
  OPAEvaluationResult,
  OPAExecutorConfig,
  ViolationCategory,
  BundleConfig,
  BundleAuthConfig,
  BundleCacheEntry,
  BundleManagerConfig,
  BundleSyncResult,
  VerificationResult,
  PublicKeyConfig,
  BundleVerifierConfig,
  SignatureManifest,
} from '@eddacraft/anvil-policy';
