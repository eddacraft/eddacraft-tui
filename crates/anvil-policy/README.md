# anvil-policy

Policy support for Anvil — pack/library loading, legacy OPA compatibility,
exceptions, and policy lifecycle helpers.

The product policy runtime selected by ADR-040 is `crates/anvil-policy-engine`,
which wraps `regorus` behind the `anvil_policy_engine` facade. This crate still
owns pack and exception helpers and the legacy Go OPA executor used by
compatibility tests and the current `.anvil/policies` gate path.

## Modules

- **`config`** — policy configuration parsing
- **`evaluator`** — legacy `.anvil/policies` gate evaluation
- **`library`** — policy library loading and resolution
- **`loader`** — policy file discovery and loading
- **`opa`** — Go OPA reference/compatibility integration

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/anvil-policy`).
