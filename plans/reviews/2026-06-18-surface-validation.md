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
| SURFDOCK (SURFDOCK-006) | 0 Dockerfiles (no dogfood corpus) | hadolint — 1 Dockerfile, 0 findings | 0 / 0 | n/a | ⚠️ INCONCLUSIVE |
| SURFSQL (SURFSQL-007) | 17 files, 0 findings (after SURFSQL-008) | sqlx — 110 raw → 0 after baseline, 1 on a new edge | baseline-absorbed | 0% effective | ✅ PASS |

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

### SURFDOCK-006 — INCONCLUSIVE (not yet a pass)

- **Anvil**: no Dockerfiles in the repo → **no dogfood corpus**, so the
  "Anvil FP rate" half of the bar cannot be measured (it is vacuously 0, not
  evidence).
- **External** (`hadolint/hadolint`): 1 Dockerfile, **0 findings** (a
  well-formed, pinned Dockerfile — expected clean), so no true-positive
  confirmation either.
- **0 false positives observed, but the run is inconclusive**: no dogfood
  corpus and a clean single external file give no positive evidence the
  detectors fire correctly in the wild. The unit tests cover each rule, but
  SURFDOCK-006 should **not** be marked passing until a run against a repo with
  a real `:latest`/`ADD https://`/pipe-to-shell Dockerfile (and ideally a repo
  that actually ships Dockerfiles) provides dogfood + external true positives.

### SURFSQL-007 — PASS (after SURFSQL-008 + SURFSQL-006)

The first run was blocked by 29 `schema.sql` findings and a high external
finding volume. Both are resolved by the two fixes the calibration run called
for, which have since merged:

- **Dogfood (Anvil) — PASS.** `ANVIL_DEV=1 ANVIL_TRACK_SURFACE_SQL=1 anvil gate
  --only-checks sql-migrations` → **17 SQL files, 0 findings**. The 29
  `apps/anvil-api/src/db/schema.sql` findings are gone: SURFSQL-008 (#2794)
  scopes out canonical schema-definition files (a `schema.sql` outside a
  migration dir is applied once to a fresh DB, so `IF NOT EXISTS` does not
  apply). The versioned migrations under `db/migrations/` were already clean;
  SURFSQL-002 destructive = 0. The "< 1% FP on Anvil" criterion is met.
- **External (`launchbadge/sqlx`, 114 `.sql` files) — PASS via baseline.** A
  raw run flags **110 findings** (77 forward `CREATE TABLE/INDEX`, 9 `.down.sql`
  rollback `DROP TABLE`, 16 data-backfill `UPDATE`, 8 temp-col `DROP COLUMN`) —
  all idiomatic tracked-migration patterns, none individually actionable. This
  is the **drift-baseline case** SURFSQL-006 handles. Measured end-to-end:

  | Step | SURFSQL findings |
  | ---- | ---------------- |
  | Raw run (no snapshot) | 110 |
  | After `anvil drift snapshot` (baseline established) | **0** |
  | After adding one *new* unguarded `CREATE TABLE` | **1** (only the new edge) |

  On an established repo the operator baselines once and thereafter sees **zero
  noise**, while genuinely new unguarded DDL still warns. The surface meets the
  FP bar under the architecture's new-edges-only model; SURFSQL-002 destructive
  detection remains active on new edges.
- **TP/FP reading.** The 110 raw findings are *true* matches of the rules (they
  really are unguarded DDL / unscoped writes) but **non-actionable** on an
  established corpus — exactly what the baseline absorbs. The effective FP rate
  an operator experiences after baselining is **0**.
- **Verdict: PASS** — dogfood clean (SURFSQL-008) + external noise absorbed with
  new-edge detection intact (SURFSQL-006).

## Summary

- **SURFGHA** and **SURFSH** meet the §16.5 #9 bar: a real dogfood corpus
  (27 workflows / 110 scripts) + an external repo, **0% FP**. SURFGHA also has
  external true positives (the ripgrep `@master` refs); SURFSH's TP evidence is
  unit-test-only (neither corpus had a dangerous command).
- **SURFDOCK** is **inconclusive**, not passing: Anvil ships no Dockerfiles
  (no dogfood corpus) and the single external Dockerfile was clean — 0 FP but
  no positive evidence. Needs a Dockerfile-bearing corpus to clear the bar.
- **SURFSQL** now **passes**: SURFSQL-008 (schema-dump scoping, #2794) clears
  the Anvil dogfood leg (17 files, 0 findings), and SURFSQL-006 (drift baseline)
  absorbs the external migration-noise (sqlx 110 → 0 after one snapshot) while
  still warning on new unguarded DDL. The destructive catalogue stays active on
  new edges.

### Acceptance-bar note (no in-scope files)

A surface with **no in-scope files** in a corpus is **inconclusive** for that
corpus, not a pass: absence of files yields a vacuous 0% FP and no evidence the
detector behaves correctly. A pass requires a real corpus with a measured FP
rate under the threshold (SURFGHA/SURFSH), ideally plus a true-positive
confirmation.
