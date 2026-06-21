/**
 * Import path mappings for monorepo migration
 *
 * Maps old @eddacraft/anvil-core imports to new package structure based on
 * the discovery document at docs/planning/monorepo-phase0-discovery.md
 */

/**
 * Subpaths under `@eddacraft/anvil-core` whose backing TS components were
 * archived under ADR-033 (the TS scanner stack — anti-pattern engine,
 * suppression parser, drift snapshot/compare, gate runner, constraint
 * collector, exporter). The capabilities those components provided are
 * either now served by the Rust scanner (anti-pattern detection,
 * suppression handling, gate evaluation) or have no current
 * implementation (drift snapshot/compare, TS-side export).
 *
 * Imports from these subpaths must NOT be rewritten — there is no
 * 1:1 active replacement. `getRewrittenPath()` returns `undefined`
 * for them so the caller surfaces the broken import explicitly.
 */
export const ARCHIVED_CORE_SUBDIRS: readonly string[] = [
  'antipattern',
  'suppression',
  'drift',
  'export',
  'gate',
];

/**
 * Mapping of core/src subdirectories to target packages
 * Based on Phase 0 discovery (MONO-000b). Subpaths in
 * `ARCHIVED_CORE_SUBDIRS` are intentionally absent — see that
 * constant's docs.
 */
export const CORE_SUBDIR_TO_PACKAGE: Record<string, string> = {
  // Contracts package (schemas, types, validation)
  schema: '@eddacraft/anvil-contracts',
  types: '@eddacraft/anvil-contracts',
  validation: '@eddacraft/anvil-contracts',

  // Core package (pure domain logic)
  provenance: '@eddacraft/anvil-core',
  warnings: '@eddacraft/anvil-core',
  explain: '@eddacraft/anvil-core',
  architecture: '@eddacraft/anvil-core',

  // Runtime package (I/O and orchestration)
  cache: '@eddacraft/anvil-runtime',
  watch: '@eddacraft/anvil-runtime',

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

  // antipattern + suppression archived under ADR-033
  // → anvil-archive/anvil-ts-scanner/. Use the anvil CLI / RMCP instead.
  '@eddacraft/anvil-core/provenance': '@eddacraft/anvil-core/provenance',
  '@eddacraft/anvil-core/warnings': '@eddacraft/anvil-core/warnings',
  '@eddacraft/anvil-core/explain': '@eddacraft/anvil-core/explain',
  '@eddacraft/anvil-core/architecture': '@eddacraft/anvil-core/architecture',
  // drift archived under ADR-033 → anvil-archive/anvil-ts-scanner/core-drift/.

  '@eddacraft/anvil-core/cache': '@eddacraft/anvil-runtime/cache',
  '@eddacraft/anvil-core/watch': '@eddacraft/anvil-runtime/watch',
  // gate + export archived under ADR-033
  // → anvil-archive/anvil-ts-scanner/runtime-gate/, runtime-export/.

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
 * Gets the rewritten import path for a given source path.
 *
 * Returns `undefined` for paths under `@eddacraft/anvil-core/<subpath>`
 * where `<subpath>` is in `ARCHIVED_CORE_SUBDIRS` — the TS components
 * those paths used to expose were archived under ADR-033 and have no
 * 1:1 active replacement, so a silent rewrite would be a lie. Callers
 * should treat `undefined` as a broken import that needs manual review.
 */
export function getRewrittenPath(sourcePath: string): string | undefined {
  // Archived subpaths: explicitly refuse to rewrite (see ARCHIVED_CORE_SUBDIRS).
  for (const subdir of ARCHIVED_CORE_SUBDIRS) {
    const archivedPrefix = `@eddacraft/anvil-core/${subdir}`;
    if (sourcePath === archivedPrefix || sourcePath.startsWith(`${archivedPrefix}/`)) {
      return undefined;
    }
  }

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
