---
id: review-capsules
title: Review Capsules
description:
  Portable, file-first governance evidence for a commit range — verifiable
  locally without trusting Anvil Cloud.
sidebar_position: 5
---

# Review Capsules

An **Anvil Review Capsule** is a portable, file-first package of the governance
evidence for a commit range. It is an ordinary directory you can hand to a
reviewer, auditor, or downstream supplier so they can verify what anvil saw —
locally, offline, without trusting Anvil Cloud or re-running your pipeline.

## Why capsules?

Anvil already records governance evidence as it works. A capsule makes a slice
of that evidence **transferable and self-verifying**:

- **No service to trust.** A capsule is files plus a digest manifest. The
  recipient verifies it on their own machine — there is no API call, no account,
  and no network dependency.
- **Pinned to a range.** A capsule is scoped to a `<base>..<head>` commit range,
  so it answers "what governance held over exactly these changes?".
- **Tamper-evident.** Every document in the capsule is recorded under a SHA-256
  digest in the manifest, so a recipient can detect any post-hoc edit.

## Creating a capsule

```bash
anvil capsule create --range <base>..<head> --out <dir>
```

For example, to capture everything that landed since the last release tag:

```bash
anvil capsule create --range v0.7.4-beta..HEAD --out ../review-capsule
```

`--range` uses two-dot `<base>..<head>` semantics. `--out` is created if missing
and refused if it already contains files; keep it **outside** the repository
(writing inside `.git/` is refused outright). The capsule is on-demand and
external by default — anvil does not stage capsules inside your repo.

## Verifying a capsule

```bash
anvil capsule verify <dir>
```

Run from inside the repository the capsule was cut from: `verify` re-collects
the policy, rules, and baseline digests from the current repo, checks them
against the capsule's manifest, and writes the resulting verdict back into the
capsule's `verification.json`. The exit code is the verdict — `0` pass/warn, `1`
block, `2` degraded, `3` error — so it drops straight into a CI step or a
reviewer's script. (`anvil capsule create` prints the exact `verify` command to
run on completion.)

## What's inside

A capsule directory packages, under a digest-complete `manifest.json` using the
`anvil.capsule.v1` digest scheme:

- **Commit/range metadata** — the commits in the range and the range identity.
- **Policy, rules, and baseline digests** — the governance configuration that
  was in force, recorded by digest so the recipient can confirm which ruleset
  and baseline the evidence corresponds to.
- **The witness chain** — the verbatim witness lines for the range's sequence
  window, carried as-is so chain identity is preserved.
- **A verification record** — present from creation. A freshly created capsule
  starts in a degraded "no checks run" state, so an unverified capsule never
  claims `pass`.

The producer's anvil version and rule identity are written from the same binding
the witness-writing hook uses, so a capsule's rule identity matches its
witnessed lines by construction.

:::caution v0 scope

`anvil capsule create` and `anvil capsule verify` ship today. The remaining
subcommands — `anvil capsule explain` and `inspect` — and the richer evidence
collectors (SARIF diagnostics and applied policy exceptions) are planned for a
follow-up; today those evidence slots are written present-but-empty. See
[ADR-074](https://github.com/eddacraft/anvil-001/blob/main/plans/decisions/074-review-capsule-v0-format.md)
for the capsule format.

:::

## Related

- [Audit Trail](./audit-trail.md) — the provenance and evidence model capsules
  draw from.
- [Gates](./gates.md) — the checks whose results a capsule's verification record
  will eventually carry.
