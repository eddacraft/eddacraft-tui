# Track 3 governance surfaces — validation runs (FP report) — 2026-06-18

Acceptance bar (council §16.5 #9): **FP rate < 1% on Anvil's own repo AND ≥ 1
external codebase validation run**, per surface.

**Method:** each surface run via its opt-in flag against a real corpus,
read-only, with the merged `main` binary:

```
ANVIL_DEV=1 ANVIL_TRACK_SURFACE_<X>=1 anvil gate --only-checks <check> --format json
```

All surfaces are **warn-only** (never block the gate); "findings" below are the
unsuppressed messages the surface emitted, classified as true positives (TP —
the pattern really is present) or false positives (FP — flagged but not the
risk the rule targets).

## Results

| Surface (item) | Dogfood — Anvil | External repo | TP / FP | FP rate | Verdict |
| -------------- | --------------- | ------------- | ------- | ------- | ------- |
| SURFGHA (SURFGHA-007) | 2 findings | ripgrep — 7 findings | 9 TP / 0 FP | 0% | ✅ PASS |
| SURFSH (SURFSH-006) | 110 files, 0 findings | ripgrep — 2 files, 0 findings | 0 / 0 | 0% | ✅ PASS |
| SURFDOCK (SURFDOCK-006) | 0 Dockerfiles (none in repo) | hadolint — 1 Dockerfile, 0 findings | 0 / 0 | 0% | ✅ PASS |
| SURFSQL (SURFSQL-007) | 29 findings | — | see below | n/a | ⚠️ BLOCKED |

### SURFGHA-007 — PASS

- **Anvil** (`.github/workflows`, 27 workflows): 2 findings, both **true
  positives** of intentional usage —
  - `bench-nightly.yml:42` `runs-on: [self-hosted, bench]` (real self-hosted runner)
  - `crates/eddacraft-tui/.github/workflows/pr-redirect.yml:51` `pull_request_target:` (real PR-redirect trigger)
- **External** (`BurntSushi/ripgrep`): 7 findings, all `uses: dtolnay/rust-toolchain@master` — genuine **unpinned branch refs** (the exact supply-chain risk SURFGHA-002 targets).
- **0 false positives** across both corpora. The accepted Anvil findings are
  suppressible via `# @anvil-ignore SURFGHA-002` (or absorbed by a future
  drift baseline). Bar met.

### SURFSH-006 — PASS

- **Anvil**: 110 in-scope `*.sh`/`*.bash` scripts, **0 dangerous-command
  findings** (the shared `command_safety` catalogue is clean on Anvil's
  scripts).
- **External** (ripgrep): 2 scripts, 0 findings.
- **0 false positives**. Bar met. (Note: neither corpus contained the
  `rm -rf /` family, so external true-positive confirmation is light — the unit
  tests cover the detection; a future run against a repo with known-dangerous
  scripts would strengthen the external TP evidence.)

### SURFDOCK-006 — PASS

- **Anvil**: no Dockerfiles in the repo → no dogfood corpus.
- **External** (`hadolint/hadolint`): 1 Dockerfile, **0 findings** (a
  well-formed, pinned Dockerfile — expected clean).
- **0 false positives**. Bar met for the FP criterion, but external
  **true-positive** confirmation is light (the one external Dockerfile is
  clean). A follow-up run against a repo with a `:latest`/`ADD https://`/
  pipe-to-shell Dockerfile would confirm the detectors fire externally; the
  unit tests already cover each rule.

### SURFSQL-007 — BLOCKED (calibration finding)

- **Anvil** (`apps/anvil-api/src/db`): **29 findings, all in
  `schema.sql`**, all from the SURFSQL-003 hygiene rule (`CREATE TABLE`/
  `CREATE INDEX` without `IF NOT EXISTS`). The 16 versioned migrations under
  `db/migrations/` are **clean**, and the destructive catalogue (SURFSQL-002)
  found **0**.
- These 29 are **false positives for the rule's intent**: `schema.sql` is a
  full-schema **dump** applied once to a fresh database, not a re-running
  migration, so an `IF NOT EXISTS` idempotency guard is not expected there.
- **FP rate fails the < 1% bar** until remediated. Two complementary fixes,
  both already on the table:
  1. **SURFSQL-006 drift baseline** (in flight) — baseline the 29 pre-existing
     findings so only *new* unguarded DDL warns ("new edges only"). Mechanism
     choice is an open owner decision.
  2. **Schema-dump scoping** — apply the SURFSQL-003 hygiene rule only to files
     under a recognised migration directory (idempotency is a migration
     concern), so standalone schema dumps are not flagged. SURFSQL-002
     (destructive ops) still applies everywhere.
- SURFSQL-007 cannot be marked passing until one of these lands; tracked
  against SURFSQL-006.

## Summary

3 of 4 surfaces (SURFGHA, SURFSH, SURFDOCK) meet the §16.5 #9 acceptance bar
(0% FP, dogfood + ≥1 external). SURFSQL is the lone outstanding item, blocked on
the SURFSQL-006 baseline (or a schema-dump scoping fix) to clear the
`schema.sql` hygiene false positives.
