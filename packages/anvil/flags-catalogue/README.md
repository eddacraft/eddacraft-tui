# @eddacraft/anvil-flags-catalogue

| Type   | Authority | Owner   | Status | Freshness                                                                                         |
| ------ | --------- | ------- | ------ | ------------------------------------------------------------------------------------------------- |
| README | Derived   | FLAGCAT | Live   | Reviewed 2026-08-23 against `flags/*.json`, the contracts schemas, and the package implementation |

| Upstream                                                                                | Downstream                                                   |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `flags/*.json`, `packages/anvil/contracts/src/schemas/feature-flags.schema.ts`, ADR-076 | TypeScript flag consumers and product-catalogue integrations |

Validated accessors for anvil's operational feature flags and product catalogue.
The package imports the repository-root `flags/*.json` files, validates them
against `@eddacraft/anvil-contracts` at module load, and exports typed read-only
accessors.

## Sources (`flags/` at the repo root)

| File                      | Contents                                                |
| ------------------------- | ------------------------------------------------------- |
| `flags/manifest.json`     | Operational rollout, entitlement, and kill-switch flags |
| `flags/groups.json`       | Operational flag defaults (ADR-048 defaults carriers)   |
| `flags/audiences.json`    | Canonical audience inventory                            |
| `flags/environments.json` | Canonical environment inventory                         |
| `flags/surfaces.json`     | Strict v2 product catalogue under ADR-076               |

## Usage

```ts
import {
  featureFlagManifest,
  flagByKey,
  productCatalogue,
} from '@eddacraft/anvil-flags-catalogue';

const flag = flagByKey('docs.access');
const manifest = featureFlagManifest(); // validated FeatureFlagManifest
const catalogue = productCatalogue(); // authoritative ProductCatalogueManifest v2
```

`productCatalogue()` is the authoritative accessor for product feature groups,
product features, delivery surfaces, reviewed internal-plumbing exclusions, and
the delivery-surface migration ledger. Retired delivery keys remain reserved;
the ledger records strict retired-source to active-target splits and merges.
`flagSurfaces()` is deprecated: it returns only a deterministic legacy
projection of the original 46 CLI product features. That projection is
explicitly incomplete and must not drive catalogue completeness, entitlement, or
runtime enforcement. The frozen v1 fixture is authoritative only for the exact
compatibility payload returned by `flagSurfaces()` during the compatibility
window. `flags/surfaces.json` through `productCatalogue()` remains canonical for
v2 product, completeness, and enforcement truth.

The product catalogue uses its own schema version `2`; operational flag
inventories remain at schema version `1`. The package is edge-bundle safe:
consumer code imports only bundler-inlined JSON and contracts types, with no
`fs`, `path`, or `process`. Validation and cross-inventory integrity checks run
once, synchronously, at module load.

## Local validation

```bash
pnpm exec nx test flags-catalogue --skip-nx-cache
```

## Scope

The package exposes declared catalogue data; it does not derive host
enforcement. FLAGCAT-012..015 own host completeness checks, operational-flag
linkage, generated human-readable views, and any approved product-tier mapping.
Runtime cascade-off and catalogue-derived availability enforcement remain out of
scope.

See
[ADR-076](../../../plans/decisions/076-feature-catalogue-surface-registry.md)
for the product model and the
[v2 schema design](../../../plans/specs/2026-08-23-product-catalogue-v2-schema.md)
for the physical contract.
