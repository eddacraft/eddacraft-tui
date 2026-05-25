# @eddacraft/anvil-observability

| Type   | Authority | Owner | Status | Freshness                                                       |
| ------ | --------- | ----- | ------ | --------------------------------------------------------------- |
| README | Advisory  | TRACE | Live   | Last reviewed 2026-05-25 against `packages/anvil/observability` |

| Upstream                                                                | Downstream                                        |
| ----------------------------------------------------------------------- | ------------------------------------------------- |
| `crates/anvil-observability`, `plans/modules/tracing-foundation.aps.md` | TypeScript consumers of W3C `traceparent` helpers |

TypeScript mirror of the Rust `anvil-observability` trace-context surface.

This package owns W3C `traceparent` parsing, formatting, and extraction helpers
for TypeScript producers and consumers. The Rust crate remains the behavioural
authority; tests here pin byte-for-byte compatibility with Rust-emitted headers.
