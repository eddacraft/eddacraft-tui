/**
 * Import path mappings for monorepo migration
 *
 * Maps old @eddacraft/anvil-core imports to new package structure based on
 * the discovery document at docs/planning/monorepo-phase0-discovery.md
 */

/**
 * Mapping of core/src subdirectories to target packages
 * Based on Phase 0 discovery (MONO-000b)
 */
export const CORE_SUBDIR_TO_PACKAGE: Record<string, string> = {
  // Contracts package (schemas, types, validation)
  schema: '@eddacraft/anvil-contracts',
  types: '@eddacraft/anvil-contracts',
  validation: '@eddacraft/anvil-contracts',

  // Core package (pure domain logic)
  provenance: '@eddacraft/anvil-core',
  warnings: '@eddacraft/anvil-core',
  antipattern: '@eddacraft/anvil-core',
  suppression: '@eddacraft/anvil-core',
  explain: '@eddacraft/anvil-core',
  architecture: '@eddacraft/anvil-core',
  drift: '@eddacraft/anvil-core',

  // Runtime package (I/O and orchestration)
  cache: '@eddacraft/anvil-runtime',
  watch: '@eddacraft/anvil-runtime',
  export: '@eddacraft/anvil-runtime',
  'gate/checks': '@eddacraft/anvil-runtime',
  'gate/config': '@eddacraft/anvil-runtime',
  'gate/formatters': '@eddacraft/anvil-runtime',
  'gate/parsers': '@eddacraft/anvil-runtime',
  'gate/rules': '@eddacraft/anvil-runtime',
  'gate/gate-runner': '@eddacraft/anvil-runtime',
  'gate/gate-config': '@eddacraft/anvil-runtime',

  // Policy package (OPA/Rego)
  'gate/policy': '@eddacraft/anvil-policy',

  // Ports package (interfaces)
  'gate/check.interface': '@eddacraft/anvil-ports',

  // Platform packages
  crypto: '@eddacraft/anvil-platform/crypto',
  utils: '@eddacraft/anvil-shared/util',
};

/**
 * Direct import path rewrites
 * Maps full import paths to their new locations
 */
export const IMPORT_REWRITES: Record<string, string> = {
  // Main package rewrites
  '@eddacraft/anvil-core': '@eddacraft/anvil-contracts',
  '@eddacraft/anvil-core/schema': '@eddacraft/anvil-contracts',
  '@eddacraft/anvil-core/types': '@eddacraft/anvil-contracts',
  '@eddacraft/anvil-core/validation': '@eddacraft/anvil-contracts',

  '@eddacraft/anvil-core/antipattern': '@eddacraft/anvil-core/antipattern',
  '@eddacraft/anvil-core/suppression': '@eddacraft/anvil-core/suppression',
  '@eddacraft/anvil-core/provenance': '@eddacraft/anvil-core/provenance',
  '@eddacraft/anvil-core/warnings': '@eddacraft/anvil-core/warnings',
  '@eddacraft/anvil-core/explain': '@eddacraft/anvil-core/explain',
  '@eddacraft/anvil-core/architecture': '@eddacraft/anvil-core/architecture',
  '@eddacraft/anvil-core/drift': '@eddacraft/anvil-core/drift',

  '@eddacraft/anvil-core/cache': '@eddacraft/anvil-runtime/cache',
  '@eddacraft/anvil-core/watch': '@eddacraft/anvil-runtime/watch',
  '@eddacraft/anvil-core/export': '@eddacraft/anvil-runtime/export',
  '@eddacraft/anvil-core/gate': '@eddacraft/anvil-runtime/gate',

  '@eddacraft/anvil-core/crypto': '@eddacraft/anvil-platform/crypto',
  '@eddacraft/anvil-core/utils': '@eddacraft/anvil-shared/util',
};

/**
 * Symbols that should be imported from @eddacraft/anvil-contracts
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
 * Symbols that should be imported from @eddacraft/anvil-ports
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
 * Symbols that should be imported from @eddacraft/anvil-core
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

  // Architecture (both old and new spellings for migration compat)
  'analyseArchitecture',
  'analyzeArchitecture',
  'ArchitectureAnalyser',
  'ArchitectureAnalyzer',
  'createArchitectureAnalyser',
  'createArchitectureAnalyzer',
  'AnalyserOptions',
  'AnalyzerOptions',
  'detectLayers',
  'validateBoundaries',

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
 * Symbols that should be imported from @eddacraft/anvil-runtime
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
 * Symbols that should be imported from @eddacraft/anvil-policy
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
  if (CONTRACT_SYMBOLS.includes(symbol)) return '@eddacraft/anvil-contracts';
  if (PORT_SYMBOLS.includes(symbol)) return '@eddacraft/anvil-ports';
  if (CORE_SYMBOLS.includes(symbol)) return '@eddacraft/anvil-core';
  if (RUNTIME_SYMBOLS.includes(symbol)) return '@eddacraft/anvil-runtime';
  if (POLICY_SYMBOLS.includes(symbol)) return '@eddacraft/anvil-policy';
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
    if (sourcePath.startsWith(`@eddacraft/anvil-core/${subdir}`)) {
      const remainder = sourcePath.slice(`@eddacraft/anvil-core/${subdir}`.length);
      return `${pkg}${remainder}`;
    }
  }

  return undefined;
}
