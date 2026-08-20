---
id: baseline
title: Introduction baseline
description:
  The record of findings accepted when anvil was introduced, so a brownfield
  project is not forced through a red gate on day one.
owner: DOCDEF
upstream:
  - crates/anvil-cli/src/commands/baseline.rs
verified_against: 0.9.6-beta
---

# Introduction baseline

**For:** teams introducing anvil to a repository that already has findings

**Time:** 5 minutes

**Outcome:** know that a baseline is not a check, and that it is not the
`anvil-baseline` policy pack

## What a baseline is

A **baseline** is a record of findings accepted when anvil was introduced. It
lets later gates focus on newly introduced problems instead of demanding an
immediate cleanup of all existing debt.

`anvil baseline` is **not** a check. It is not the `anvil-baseline` starter
policy pack, and it is not the eval-regression store.

## Record and verify

```text
anvil baseline
anvil baseline verify
```

`--refresh` updates an existing record at HEAD. Review the written baseline like
code before you commit it.

A brownfield repository should record a baseline before a team gate is blocking.
See [team workflow](../guides/team-flow.md).

## Related definitions

- [Policy packs](policy-model.md)
- [How anvil evaluates a project](evaluation-model.md)
- [Team workflow](../guides/team-flow.md)
- [Glossary](glossary.md)
