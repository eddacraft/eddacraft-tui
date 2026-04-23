---
'eddacraft-anvil': patch
---

**Pre-release security sweep — `uuid` and `fast-xml-parser` overrides**

Bumped two transitive dependencies via `pnpm.overrides` and the npm-style root
`overrides` block to close two medium-severity Dependabot advisories:

- **`uuid` → `>=14.0.0`** (advisory GHSA-w5hq-g745-h8pq, missing buffer bounds
  check in `v3`/`v5`/`v6` when `buf` is provided). Forces all transitive uuid
  installs above the patched line. Previously pulled `uuid@8.3.2` via
  `@azure/msal-node`, `@azure/storage-blob`, and `sockjs`, plus `uuid@10.0.0`
  via `svix`/`standardwebhooks`.
- **`fast-xml-parser` → `>=5.7.0`** (advisory GHSA-gh4j-gqv2-49f6, XML comment
  / CDATA injection via unescaped delimiters). Previously pulled `5.5.11` via
  `@azure/core-xml`.

**Compatibility notes:**

- `uuid@14` is ESM-only. The CJS consumers in our tree (`sockjs`,
  `@azure/msal-node`) reach uuid via `require()`, which works under Node
  >=22.12 with synchronous-ESM interop. The repo already pins `engines.node
  >=22.13.0`, so the override is safe at the supported Node baseline. Anyone
  consuming this package under an older Node toolchain or via a Jest/ts-jest
  transform that does not support ESM interop will need to update.
- The two overrides blocks are deliberately asymmetric. The npm-style root
  `overrides` uses bare `"uuid": ">=14.0.0"` so any npm consumer pulling this
  monorepo gets the patched line regardless of the requested range. The
  `pnpm.overrides` block uses the scoped `"uuid@<14.0.0": ">=14.0.0"` form so
  pnpm only intervenes on resolutions that would otherwise land below the
  fix.
