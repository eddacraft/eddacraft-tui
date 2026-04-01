# @eddacraft/anvil-policy

OPA (Open Policy Agent) integration for Anvil. Manages OPA binary acquisition,
Rego policy loading, bundle management with signature verification, and policy
evaluation.

## Status

Winding down -- the Rust crate `anvil-policy` handles policy execution in the
CLI. This package remains in use by the TypeScript runtime and e2e tests.

## API Surface

- **`OPABinaryManager`** -- Downloads and manages the OPA binary
- **`PolicyLoader`** -- Discovers and loads `.rego` policy files
- **`OPAExecutor`** -- Evaluates policies against an input context
- **`BundleManager`** -- Syncs OPA bundles from remote registries
- **`BundleVerifier`** -- Verifies bundle signatures using public keys

## Consumers

- `@eddacraft/anvil-runtime` (gate checks)
- e2e tests

## Development

```bash
pnpm --filter @eddacraft/anvil-policy build
pnpm --filter @eddacraft/anvil-policy test
```
