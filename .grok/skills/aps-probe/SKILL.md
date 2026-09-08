---
name: aps-probe
description: >-
  PROBE+ capability manifest for dev-loop drain mode: detect and cache a
  repo's real test/lint/build commands with evidence. Invoked by the loop
  before executing work items, or when a cached manifest may be stale.
---

# APS Probe (PROBE+)

The Orient capability phase of `dev-loop` drain mode. PROBE+ **detects** what a repository
can do — it never assumes — and records the answer as a cached, self-validating
**capability manifest**. Every later phase (ISOLATE, BUILD, VERIFY, LAND) binds
to the manifest's commands rather than guessing, so the loop is portable across
languages and harnesses and cannot silently run the wrong (or a fake) check.

A wrong or stale manifest poisons every downstream gate, so this phase is
designed around one idea: the manifest carries **evidence of its own
correctness**.

Run PROBE+ **once per repo** and cache the result. Re-run only when an evidence
check below says to.

## Manifest file

The manifest is a single JSON file at **`.dev-loop/capability-manifest.json`**
in the repository root — the same home as the run checkpoints. Create
`.dev-loop/` on the feature branch if absent. Every phase reads the manifest
from this path; never write a manifest anywhere else, and never trust one
found elsewhere.

Minimal shape — one entry per detected capability, each carrying the four-part
evidence in full:

```json
{
  "probedAt": "<ISO 8601 timestamp>",
  "capabilities": {
    "test": {
      "command": "pnpm test",
      "configHash": "<sha256 of the config file(s) the command was derived from>",
      "testCount": 214,
      "outputSample": "<first lines of the command's real output>"
    },
    "lint": {
      "command": "pnpm lint",
      "configHash": "<sha256>",
      "outputSample": "<first lines of real output>"
    }
  }
}
```

`testCount` is required for test capabilities and omitted for others; absence
of a capability is recorded as an explicit `"none found"` entry, never by
leaving the key out. An entry missing any required evidence field is
incomplete and must not be trusted (see below).

## What to detect

Detect each of the following; record absence as an explicit "none found", never as
an assumption:

- **Commands:** test, lint, typecheck, build — the actual invocation for this
  repo, derived from real config (manifest scripts, task runner, build file).
- **CI presence:** is there a CI definition, and which checks does it run.
- **Isolation mechanism:** worktree, devcontainer, or docker — how work can be
  done reversibly off the default branch.
- **Branch protections:** which branches are protected and what they require.
- **Secret-handling tooling:** how secrets are kept out of the repo (vault,
  env-file conventions, secret scanners).
- **Package manager:** the one actually in use (lockfile / config evidence).

Detection is **cross-language**: identify the toolchain from the files present,
not from a fixed assumption about ecosystem. If a capability genuinely does not
exist, the manifest records that fact so downstream phases adapt rather than
invent a command.

## The manifest carries its own evidence

For **each detected command** the manifest records, at minimum:

- **the command** — the exact invocation;
- **a config hash** — a hash of the config file the command was derived from;
- **the discovered test count** — how many tests the suite actually contains;
- **an output sample** — a short excerpt of the command's real output.

The count and sample together are the proof that the command does something. The
hash is the cache key for that command. A manifest entry without all four is
incomplete and must not be trusted by a later phase.

## Cache invalidation by config change

The manifest is **cached per repo**. Before _any_ use of the manifest, re-check
the recorded config-file hashes against the files on disk:

- **All hashes match** → the manifest is fresh; use it.
- **Any hash mismatches** → a config file changed; **re-probe** the affected
  capability (re-derive the command, re-run it, refresh count + sample + hash).

This makes config change the single, reliable trigger for re-probing. Nothing
else silently invalidates a cached command.

## Post-ISOLATE smoke test

After the loop isolates work (worktree/branch/container) and before BUILD
proceeds, run **one** detected command and require it to **exit 0**.

- **Exit 0** → the isolation is sound; BUILD may proceed.
- **Non-zero** → **invalidate the isolation section** of the manifest and force a
  **re-probe of isolation**. BUILD does not proceed on an unproven environment.

The smoke test catches an isolation mechanism that looked right at detection time
but does not actually produce a working environment.

## VERIFY guardrails

The manifest feeds two non-negotiable guards on the path to LAND. Parking
means appending an entry to the escalation queue file itself —
`plans/execution/escalation.queue.md` (per `aps-escalation-queue`) — never an
improvised location:

- **Zero-tests is suspect.** A manifest that discovers **zero** tests must
  **escalate before any LAND** — an empty suite is far more often a detection or
  configuration failure than a genuine state, and must be confirmed by a human,
  not waved through — park to the escalation queue with class
  `verification-blocked`.
- **Incomplete dossier blocks LAND.** If any required command is **missing,
  timed-out, or skipped**, set **`VERIFY_INCOMPLETE`**. This flag **blocks LAND
  even when the gates are set to auto** — an incomplete verification can never be
  treated as a pass — park to the escalation queue with class
  `verification-blocked`.

## The failure mode this prevents

A **placeholder test script** — one that exits 0 trivially (an empty suite, a
stubbed `test` target, a `true` command) — looks green and masks a real suite
that is not running. PROBE+ defeats this through the manifest's evidence
fields: a trivial green has no tests to count and no real output to sample, so
the evidence contradicts the exit code and the deception is caught before it
can produce a false green at VERIFY or LAND.

## Operating stance

- Detect, record, prove — never assume. Absence is a recorded fact, not a guess.
- Treat the manifest as cache: validate hashes before every use; re-probe on
  mismatch; refresh count + sample + hash whenever you re-derive a command.
- Surface suspicious results (zero tests, incomplete dossier, failed smoke test)
  rather than smoothing them over. These are escalation and re-probe triggers,
  not edge cases to suppress.
- Keep the manifest the single source of truth for "what this repo can do"; the
  rest of the loop reads it rather than re-deriving commands ad hoc.

## Cross-references

- `plans/designs/2026-06-18-canonical-aps-loop.design.md` — historical source
  (PROBE+ is change #1 / gap G1; flagged the highest-risk, keystone
  component in the stress test) (catalogue-repo history — not present in
  consuming projects; do not read at runtime).
- `dev-loop` — the canonical loop this phase precedes; it binds ISOLATE, BUILD,
  VERIFY, and LAND to this manifest.
