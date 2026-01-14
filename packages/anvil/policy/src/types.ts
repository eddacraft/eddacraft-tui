/**
 * Policy types
 *
 * Type definitions for OPA policy management.
 */

/**
 * Signature algorithm for bundle verification
 */
export type SignatureAlgorithm = 'RS256' | 'RS384' | 'RS512' | 'ES256' | 'ES384' | 'ES512';

/**
 * Bundle configuration
 */
export interface BundleConfig {
  /** Bundle URL or path */
  url: string;
  /** Bundle name/identifier */
  name?: string;
  /** Signature requirement */
  requireSignature?: boolean;
  /** Allowed signature algorithms */
  allowedAlgorithms?: SignatureAlgorithm[];
}

/**
 * Policy evaluation result
 */
export interface PolicyResult {
  /** Whether the policy passed */
  allow: boolean;
  /** Detailed results from the policy */
  results?: Record<string, unknown>;
  /** Error message if evaluation failed */
  error?: string;
}

/**
 * Policy input for evaluation
 */
export interface PolicyInput {
  [key: string]: unknown;
}
