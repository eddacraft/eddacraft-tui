/**
 * Policy module exports
 *
 * OPA integration for flexible policy evaluation using Rego language.
 */

export { OPABinaryManager, getOPABinaryManager } from './opa-binary-manager.js';
export type { OPABinaryConfig, BinaryInfo } from './opa-binary-manager.js';

export { PolicyLoader } from './policy-loader.js';
export type { LoadedPolicy, PolicyDiscoveryResult, PolicyLoaderConfig } from './policy-loader.js';

export { OPAExecutor } from './opa-executor.js';
export type {
  OPAInput,
  PolicyViolation,
  OPAEvaluationResult,
  OPAExecutorConfig,
  ViolationCategory,
} from './opa-executor.js';
