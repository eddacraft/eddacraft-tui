/**
 * Import path mappings for monorepo migration
 *
 * Maps old @anvil/core imports to new package structure based on
 * the discovery document at docs/planning/monorepo-phase0-discovery.md
 */

/**
 * Mapping of core/src subdirectories to target packages
 * Based on Phase 0 discovery (MONO-000b)
 */
export const CORE_SUBDIR_TO_PACKAGE: Record<string, string> = {
  // Contracts package (schemas, types, validation)
  'schema': '@anvil/contracts',
  'types': '@anvil/contracts',
  'validation': '@anvil/contracts',

  // Core package (pure domain logic)
  'provenance': '@anvil/core',
  'warnings': '@anvil/core',
  'antipattern': '@anvil/core',
  'suppression': '@anvil/core',
  'explain': '@anvil/core',
  'architecture': '@anvil/core',
  'drift': '@anvil/core',

  // Runtime package (I/O and orchestration)
  'cache': '@anvil/runtime',
  'watch': '@anvil/runtime',
  'export': '@anvil/runtime',
  'gate/checks': '@anvil/runtime',
  'gate/config': '@anvil/runtime',
  'gate/formatters': '@anvil/runtime',
  'gate/parsers': '@anvil/runtime',
  'gate/rules': '@anvil/runtime',
  'gate/gate-runner': '@anvil/runtime',
  'gate/gate-config': '@anvil/runtime',

  // Policy package (OPA/Rego)
  'gate/policy': '@anvil/policy',

  // Ports package (interfaces)
  'gate/check.interface': '@anvil/ports',

  // Platform packages
  'crypto': '@anvil/platform/crypto',
  'utils': '@anvil/shared/util',
};

/**
 * Direct import path rewrites
 * Maps full import paths to their new locations
 */
export const IMPORT_REWRITES: Record<string, string> = {
  // Main package rewrites
  '@anvil/core': '@anvil/contracts',
  '@anvil/core/schema': '@anvil/contracts',
  '@anvil/core/types': '@anvil/contracts',
  '@anvil/core/validation': '@anvil/contracts',

  '@anvil/core/antipattern': '@anvil/core/antipattern',
  '@anvil/core/suppression': '@anvil/core/suppression',
  '@anvil/core/provenance': '@anvil/core/provenance',
  '@anvil/core/warnings': '@anvil/core/warnings',
  '@anvil/core/explain': '@anvil/core/explain',
  '@anvil/core/architecture': '@anvil/core/architecture',
  '@anvil/core/drift': '@anvil/core/drift',

  '@anvil/core/cache': '@anvil/runtime/cache',
  '@anvil/core/watch': '@anvil/runtime/watch',
  '@anvil/core/export': '@anvil/runtime/export',
  '@anvil/core/gate': '@anvil/runtime/gate',

  '@anvil/core/crypto': '@anvil/platform/crypto',
  '@anvil/core/utils': '@anvil/shared/util',
};

/**
 * Symbols that should be imported from @anvil/contracts
 * These are Zod schemas, types, and validation utilities
 */
export const CONTRACT_SYMBOLS = [
  // Schema symbols
  'APSPlanSchema',
  'APSModuleSchema',
  'APSTaskSchema',
  'WarningSchema',
  'WarningIdSchema',
  'SeveritySchema',

  // Type symbols
  'APSPlan',
  'APSModule',
  'APSTask',
  'Warning',
  'WarningId',
  'Severity',
  'GateConfig',
  'GateResult',
  'CheckResult',

  // Validation symbols
  'validateAPSPlan',
  'validateAPSModule',
  'parseAPSPlan',
];

/**
 * Symbols that should be imported from @anvil/ports
 * These are interface definitions
 */
export const PORT_SYMBOLS = [
  'ICheck',
  'ICheckContext',
  'ICheckResult',
  'IGateRunner',
  'ICacheProvider',
  'IStorageProvider',
  'IConfigProvider',
];

/**
 * Symbols that should be imported from @anvil/core
 * These are pure domain logic functions
 */
export const CORE_SYMBOLS = [
  // Antipattern
  'scanForAntipatterns',
  'detectAntipatterns',
  'AntipatternScanner',

  // Suppression
  'createSuppressionManager',
  'SuppressionManager',
  'isWarningSuppressed',

  // Provenance
  'createProvenanceTracker',
  'ProvenanceTracker',

  // Warnings
  'createWarningId',
  'formatWarning',
  'WarningFormatter',

  // Architecture
  'analyzeArchitecture',
  'detectLayers',
  'validateBoundaries',
  'ArchitectureAnalyzer',

  // Drift
  'detectDrift',
  'createSnapshot',
  'comparePlanToSnapshot',
  'DriftDetector',

  // Explain
  'explainPlan',
  'explainModule',
  'PlanExplainer',
];

/**
 * Symbols that should be imported from @anvil/runtime
 * These are I/O and orchestration functions
 */
export const RUNTIME_SYMBOLS = [
  // Gate
  'GateRunner',
  'runGate',
  'createGateConfig',

  // Checks
  'ESLintCheck',
  'CoverageCheck',
  'SecretCheck',
  'PolicyCheck',
  'ArchitectureCheck',
  'CommandSafetyCheck',

  // Cache
  'FileCache',
  'createFileCache',
  'CacheManager',

  // Watch
  'FileWatcher',
  'createFileWatcher',
  'watchFiles',

  // Export
  'exportToLLMsTxt',
  'exportToMCP',
  'createExporter',
];

/**
 * Symbols that should be imported from @anvil/policy
 * These are OPA/Rego integration functions
 */
export const POLICY_SYMBOLS = [
  'OPABinaryManager',
  'OPAExecutor',
  'PolicyLoader',
  'BundleManager',
  'BundleVerifier',
  'evaluatePolicy',
  'loadPolicy',
  'createPolicyBundle',
];

/**
 * Maps symbols to their target packages
 */
export function getPackageForSymbol(symbol: string): string | undefined {
  if (CONTRACT_SYMBOLS.includes(symbol)) return '@anvil/contracts';
  if (PORT_SYMBOLS.includes(symbol)) return '@anvil/ports';
  if (CORE_SYMBOLS.includes(symbol)) return '@anvil/core';
  if (RUNTIME_SYMBOLS.includes(symbol)) return '@anvil/runtime';
  if (POLICY_SYMBOLS.includes(symbol)) return '@anvil/policy';
  return undefined;
}

/**
 * Gets the rewritten import path for a given source path
 */
export function getRewrittenPath(sourcePath: string): string | undefined {
  // Direct path match
  if (IMPORT_REWRITES[sourcePath]) {
    return IMPORT_REWRITES[sourcePath];
  }

  // Check for subpath matches
  for (const [subdir, pkg] of Object.entries(CORE_SUBDIR_TO_PACKAGE)) {
    if (sourcePath.startsWith(`@anvil/core/${subdir}`)) {
      const remainder = sourcePath.slice(`@anvil/core/${subdir}`.length);
      return `${pkg}${remainder}`;
    }
  }

  return undefined;
}
