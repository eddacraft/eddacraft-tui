---
id: boundaries
title: Architecture boundaries
description:
  Declared structural dependency constraints, how they are defined, and how the
  import-boundaries gate check enforces them.
owner: DOCDEF
upstream:
  - docs/architecture/quality-model.md
  - crates/anvil-cli/src/commands/architecture.rs
  - crates/anvil-cli/src/commands/drift.rs
  - crates/anvil-cli/src/commands/check_catalog.rs
  - crates/anvil-cli/src/commands/migrate.rs
verified_against: 0.9.6-beta
---

# Architecture boundaries

**For:** projects with directory layers that should not depend the wrong way

**Time:** 6 minutes

**Outcome:** know that a boundary is a declared dependency constraint, defined
by `anvil architecture` and enforced by the `import-boundaries` gate check

## What a boundary is

A **boundary** is a declared structural constraint about which parts of a
project may depend on one another. Prefer that word in ordinary use.

It is not interchangeable with:

- the `architecture` CLI, which defines the structure;
- the `import-boundaries` check (alias `architecture`), which enforces it;
- a policy pack; or
- a compiled anti-pattern rule.

## Define the structure

`anvil architecture` is the definition surface:

```text
anvil architecture validate
anvil architecture show
```

The definition lives in the `architecture` section of the project config, or in
`.anvil/architecture.yaml`. `anvil migrate architecture --apply` records
`architecture.source` when the standalone file is the source.

This page is not a second tutorial. Use
[define architecture boundaries](../first-project.md) and the
[architecture tutorial](../tutorials/architecture.md).

## Enforce as a gate check

Enforcement is the `import-boundaries` check (alias `architecture`). It is a
**gate** check. `anvil check` will not run it.

```text
anvil gate --only-checks import-boundaries --format plain
```

See the [check catalogue entry](../reference/checks.md#import-boundaries).

## Drift is not the check

`anvil drift` records snapshots and compares them over time. It tracks change;
it is not the boundary check. Use the [drift tutorial](../tutorials/drift.md).

## Related definitions

- [Check catalogue: `import-boundaries`](../reference/checks.md#import-boundaries)
- [How anvil evaluates a project](evaluation-model.md)
- [Architecture tutorial](../tutorials/architecture.md)
- [Define architecture boundaries](../first-project.md)
- [Drift tutorial](../tutorials/drift.md)
- [Glossary](glossary.md)
