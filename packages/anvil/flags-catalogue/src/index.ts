/**
 * @eddacraft/anvil-flags-catalogue
 *
 * Single source of truth for Anvil feature-flag definitions and the
 * gating-model inventories (groups, audiences, environments). The repo-root
 * `flags/*.json` files are validated against the contracts schemas at module
 * load, and re-exported as typed accessors.
 *
 * @module @eddacraft/anvil-flags-catalogue
 */

export {
  featureFlagManifest,
  flagGroups,
  flagAudiences,
  flagEnvironments,
  flagSurfaces,
  mustAlwaysBeOpenSurfaces,
} from './manifest.js';

export {
  CLI_LICENCE_GATE,
  DOCS_ACCESS_FLAG,
  API_SCOPE_FLAGS,
  API_SCOPE_NAMES,
  DEFAULT_APPROVAL_SCOPES,
  CLI_LICENCE_GATE_KEY,
  DOCS_ACCESS_FLAG_KEY,
  API_SCOPE_FLAG_PREFIX,
  flagByKey,
  tryFlagByKey,
  isApiScopeName,
  canonicalAccountTier,
  type ApiScopeName,
} from './catalogue.js';
