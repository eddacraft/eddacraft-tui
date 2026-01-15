/**
 * Gate Types for @anvil/contracts
 *
 * Pure type definitions for gate configuration and results.
 * No runtime dependencies - these are shared across packages.
 */

import type { APSPlan, Warning, WarningResult } from '../schemas/index.js';

/**
 * Policy bundle configuration for remote bundles in GateConfig
 */
export interface PolicyBundleConfig {
  /** Bundle URL */
  url: string;
  /** Bundle name/identifier */
  name?: string;
  /** Whether this bundle is enabled */
  enabled?: boolean;
  /** Polling interval in milliseconds for updates */
  polling_interval?: number;
  /** Authentication configuration */
  auth?: {
    type: 'bearer' | 'basic' | 'aws-sigv4';
    token?: string;
    username?: string;
    password?: string;
  };
}

/**
 * Signature algorithms supported for policy bundle verification
 */
export type SignatureAlgorithm = 'RS256' | 'RS384' | 'RS512' | 'ES256' | 'ES384' | 'ES512';

/**
 * Policy verification settings
 */
export interface PolicyVerificationConfig {
  /** Whether to require signatures on bundles */
  require_signatures?: boolean;
  /** Allowed signature algorithms */
  allowed_algorithms?: SignatureAlgorithm[];
  /** Public keys for verification (key ID to PEM content or path) */
  keys?: Record<string, string>;
}

/**
 * Policy-specific configuration within GateConfig
 */
export interface PolicyConfig {
  /** Remote policy bundles */
  bundles?: PolicyBundleConfig[];
  /** Signature verification settings */
  verification?: PolicyVerificationConfig;
}

// =============================================================================
// Stack Configuration (STACK-012)
// =============================================================================

/**
 * Configuration for a single stack layer (Kindling, Ember, or Edda)
 */
export interface StackLayerConfig {
  /** Whether this layer is enabled */
  enabled: boolean;
  /** Layer-specific configuration */
  [key: string]: unknown;
}

/**
 * Validation settings for stack integrity checks
 */
export interface StackValidationConfig {
  /** Check that provenance links resolve correctly across layers */
  check_provenance_integrity?: boolean;
  /** Check that schemas are compatible between layers */
  check_schema_compatibility?: boolean;
}

/**
 * Stack-wide configuration for the Edda Stack (Kindling -> Ember -> Edda)
 */
export interface StackConfig {
  /** Kindling layer configuration (observation) */
  kindling?: StackLayerConfig;
  /** Ember layer configuration (candidate memories) */
  ember?: StackLayerConfig;
  /** Edda layer configuration (canonical memories) */
  edda?: StackLayerConfig;
  /** Validation settings for stack integrity checks */
  validation?: StackValidationConfig;
}

/**
 * Watch configuration for file watching
 * This is a base type - runtime watch module has more detailed version
 */
export interface WatchConfig {
  /** File patterns to watch */
  patterns?: string[];
  /** Patterns to exclude */
  exclude?: string[];
  /** Debounce delay in milliseconds */
  debounce?: number;
  /** Git integration settings */
  git?: Record<string, unknown>;
  /** Allow additional properties for runtime extensions */
  [key: string]: unknown;
}

export interface GateCheck {
  name: string;
  description: string;
  enabled: boolean;
  config?: Record<string, unknown>;
}

/**
 * Extended details that can include warnings from anti-pattern/boundary checks
 */
export interface GateResultDetails {
  /** Warnings from anti-pattern or boundary detection */
  warnings?: WarningResult;
  /** Any other check-specific details */
  [key: string]: unknown;
}

export interface GateResult {
  check: string;
  passed: boolean;
  score?: number;
  message: string;
  details?: GateResultDetails;
  error?: string;
  skipped?: boolean;
}

/**
 * Helper to extract warnings from a GateResult
 */
export function getWarningsFromResult(result: GateResult): Warning[] {
  return result.details?.warnings?.warnings ?? [];
}

/**
 * Helper to check if a GateResult has blocking warnings
 */
export function hasBlockingWarnings(result: GateResult): boolean {
  const warnings = getWarningsFromResult(result);
  return warnings.some((w) => w.severity === 'error' && !w.suppressed);
}

export interface GateRunResult {
  overall: boolean;
  score: number;
  checks: GateResult[];
  summary: {
    total: number;
    passed: number;
    failed: number;
    skipped: number;
  };
}

export interface GateConfig {
  version: number;
  checks: GateCheck[];
  thresholds: {
    overall_score: number;
    [key: string]: number;
  };
  global_config?: Record<string, unknown>;
  /** Watch mode configuration */
  watch?: WatchConfig;
  /** Policy configuration including remote bundles */
  policy?: PolicyConfig;
  /** Stack configuration for Edda Stack layers */
  stack?: StackConfig;
}

// Use APSPlan directly instead of a separate interface
export type PlanData = APSPlan;

/**
 * Architecture context base type - full implementation in @anvil/core
 * Uses unknown for layers/modules since runtime has more detailed types
 */
export interface ArchitectureContextBase {
  /** Project root path */
  root?: string;
  /** Layer information - full type in @anvil/core */
  layers?: Record<string, unknown>;
  /** Module information */
  modules?: Record<string, unknown>;
  /** Allow additional properties for runtime extensions */
  [key: string]: unknown;
}

export interface CheckContext {
  /** Plan data - optional for planless mode */
  plan?: PlanData;
  workspace_root: string;
  config: GateConfig;
  check_config: Record<string, unknown>;
  /** Full codebase scan mode (no plan-based scoping) */
  fullScan?: boolean;
  /** Explicit file list for planless mode (workspace-relative or absolute paths) */
  targetFiles?: string[];
  /** Architecture context from ArchitectureCheck - passed to downstream checks (e.g., PolicyCheck) */
  architectureContext?: ArchitectureContextBase;
}

/**
 * Options for normalising target files
 */
export interface NormaliseFilesOptions {
  /** Filter function to check if file should be included */
  filter?: (filePath: string) => boolean;
  /** Whether to check file existence (default: true) */
  checkExists?: boolean;
}
