---
id: review-capsules
title: Review capsules
description: Package and verify governance evidence for a commit range.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/capsule.rs
  - crates/anvil-capsule/src/lib.rs
verified_against: 0.9.0-beta
---

# Review capsules

A review capsule is a portable directory containing governance evidence for a
commit range. It is useful when a reviewer needs a closed, locally verifiable
snapshot rather than access to the original working environment.

## Create a capsule

Check the exact arguments in your installed version:

```text
anvil capsule create --help
```

Choose a deliberate commit range and output location. Capsule creation records
evidence; it does not approve the changes.

## Verify a capsule

```text
anvil capsule verify path/to/capsule
```

Verification checks the capsule's structure, digests, and recorded verdict. Use
the exit code for automation.

## Explain a capsule

```text
anvil capsule explain path/to/capsule
```

Explain is read-only and human-oriented. It reports the verdict stored in the
capsule; it does not replace verification.

## Share safely

Review the capsule for repository names, commit metadata, file paths, and
diagnostics before sending it outside your team. A capsule is portable, not
automatically public.

## Next step

Read [evidence and audit trails](audit-trail.md) for the limits of retained
evidence.

## Related definitions

- [How anvil evaluates a project](evaluation-model.md)
- [Evidence and audit trails](audit-trail.md)
- [Policy model](policy-model.md)
