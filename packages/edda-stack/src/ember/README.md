# Ember implementation retired

| Type  | Authority | Owner   | Status | Freshness                                    |
| ----- | --------- | ------- | ------ | -------------------------------------------- |
| Guide | Derived   | EMBERRS | Live   | Last reviewed 2026-09-05 against EMBERRS-001 |

| Upstream                                                                        | Downstream                   |
| ------------------------------------------------------------------------------- | ---------------------------- |
| `packages/edda-stack/package.json`, `plans/modules/ember-rust-migration.aps.md` | Ember migration contributors |

The TypeScript Ember runtime was retired under EMBERRS-001. This directory is an
orientation marker only: no proposal store, candidate service, evaluator,
aggregator, decay worker or observation hook is built or exported from it. The
package no longer exports `/ember` or depends on the native JS SQLite driver.

The [Rust migration](../../../../plans/modules/ember-rust-migration.aps.md) owns
the replacement and the
[candidate guide](../../../../docs/guides/ember-candidates.md) explains current
availability. Existing proposal databases are preserved.

For migration fixtures and behaviour archaeology, use the
[historical implementation](https://github.com/eddacraft/anvil-001/tree/96bcaf39fd436ea5d68415dedc62c90976db1236/packages/edda-stack/src/ember).
Do not restore it as a runtime fallback.
