---
id: audit-trail
title: Evidence and audit trails
description:
  Understand the local evidence anvil records and how to review it safely.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/audit.rs
  - crates/anvil-witness/src/lib.rs
  - crates/anvil-cli/src/commands/capsule.rs
verified_against: 0.9.0-beta
---

# Evidence and audit trails

anvil can retain local evidence that checks ran for a change. This helps a team
answer what was checked without treating log text as proof by itself.

## Evidence types

Depending on the workflow, evidence can include:

- findings and gate verdicts;
- the baseline used for comparison;
- witness records for protected changes;
- architecture or drift snapshots; and
- review capsules for a commit range.

## Review current evidence

Use the command that owns the evidence:

```text
anvil status
anvil audit
anvil insights
```

Use `--json` only when you need stable machine input.

## Privacy boundary

Code analysis is local. Evidence may contain file paths, rule identifiers,
commit identifiers, or counts. Inspect generated files before sharing them and
remove private paths or repository details.

## Evidence is not a substitute for review

A passing record proves only the checks and inputs named by that record. It does
not prove that every risk was considered or that deployment is safe.

## Next step

For portable commit-range evidence, read [review capsules](review-capsules.md).
