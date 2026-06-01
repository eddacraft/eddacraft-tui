# @eddacraft/anvil-flags-catalogue

Single source of truth for Anvil's feature-flag definitions and the gating-model
inventories. Reads the repo-root `flags/*.json` files, validates them against
the `@eddacraft/anvil-contracts` schemas at module load, and re-exports typed
accessors.

## Sources (`flags/` at the repo root)

| File                      | Contents                                              |
| ------------------------- | ----------------------------------------------------- |
| `flags/manifest.json`     | Every shipped flag definition (sorted by `key`)       |
| `flags/groups.json`       | Primary groups + defaults (ADR-048 defaults carriers) |
| `flags/audiences.json`    | Canonical audience inventory                          |
| `flags/environments.json` | Canonical environment inventory                       |

## Usage

```ts
import {
  featureFlagManifest,
  flagByKey,
  CLI_LICENCE_GATE,
  API_SCOPE_FLAGS,
} from '@eddacraft/anvil-flags-catalogue';

const flag = flagByKey('docs.access');
const manifest = featureFlagManifest(); // validated FeatureFlagManifest
```

The package is edge-bundle safe: the consumer path imports only the JSON files
(bundler-inlined) plus `@eddacraft/anvil-contracts` types — no `fs`/`path`/
`process`. Validation + cross-inventory integrity checks run once,
synchronously, at module load (a bad shape throws at import, not at first use).
Environment derivation stays in the per-surface helpers.

## Scope

Introduced by **FLAGCAT-002**. Rust codegen from the same manifest is
**FLAGCAT-004**; migrating existing call sites onto this package is
**FLAGCAT-003 / -005**.
