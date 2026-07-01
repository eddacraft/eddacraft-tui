# Witness Chain — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                                     |
| ------- | ------------- | ------ | ------ | ------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-18 as N4 doc-lane closure for v0.7.0-beta |

| Upstream                                                                                                                                                                                                                                   | Downstream                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [ADR-037 §D-1..D-9](../../plans/decisions/037-witness-chain-and-l4-policy.md), [`multilayer-protection-v2.aps.md`](../../plans/modules/multilayer-protection-v2.aps.md) (MLP-002..-005, MLP2-011..-015, MLP2-037, MLP2-061..-063, MLP-015) | [`crates/anvil-witness/`](../../crates/anvil-witness/), [`crates/anvil-hook/src/pre_push.rs`](../../crates/anvil-hook/src/pre_push.rs), [`crates/anvil-cli/src/commands/hook.rs`](../../crates/anvil-cli/src/commands/hook.rs), [`crates/anvil-cli/src/commands/audit_chain.rs`](../../crates/anvil-cli/src/commands/audit_chain.rs), [`crates/anvil-cli/src/commands/doctor.rs`](../../crates/anvil-cli/src/commands/doctor.rs) |

The witness chain is Anvil's in-tree, hash-chained record of which protection
layers fired on which commit. It lives under `anvil/witness/` so it travels via
`git worktree add`, `git clone`, and `git push` without hosted infrastructure.
This runbook explains the on-disk layout, how to verify the chain, what each
operator-facing command does, and how to recover from the failure modes that
show up in practice.

## On-disk layout

Everything under `anvil/witness/` is tracked content. The `.gitattributes` entry
set by `anvil start` pins `anvil/witness/active.ndjson` to `merge=union -text`
so parallel branches never produce conflict markers — each branch's lines
concatenate on merge and `verify_chain_dag` joins them through merge nodes.

```
anvil/
  witness/
    active.ndjson                 # current append target (flock-serialised)
    archive/
      <scope>-<seq>-<merkle>.ndjson  # rolled-over segments, lex-ordered
      <scope>-<seq>-<merkle>.ndjson
      …
    manifest/
      chain.ndjson                # one entry per rollover (archive_path, merkle, line_count, seq range)
```

- **active.ndjson** holds new lines. Each line is canonical UTF-8 JSON with
  sorted keys; two machines emitting the same logical record produce
  byte-identical bytes.
- **archive/** holds rolled-over segments. Rollover fires when active crosses
  either 1000 lines or 1 MiB (whichever is hit first). The check runs inside the
  flock so concurrent writers cannot race a half-archive into existence. Archive
  names are content-addressed (`<scope>-<seq>-<merkle>`); a re-run rollover that
  lands on identical bytes is idempotent.
- **manifest/chain.ndjson** records each rollover (archive path, full SHA-256 of
  the archived bytes, line count, `[start..=end]` seq range). Tooling can verify
  any archive against the manifest without re-walking the chain.

The ordered file list for verification — archives lex-ascending, then active —
is produced by `anvil_witness::witness_paths()`. Use that helper; do not roll
your own walker. The pre-push hook, `anvil l4-validate`, and `anvil audit-chain`
all converge on this function (MLP2-061/-062 closure).

## Line shape (v1)

Each line carries the fields from ADR-037 §D-1 plus the DAG additions from
MLP-005 / MLP2-011. The load-bearing fields for an operator inspecting a chain
break:

| Field                | Meaning                                                                                                                  |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `v`                  | Schema version. `1` today.                                                                                               |
| `seq`                | Monotonic per-line counter. Gaps trigger `SequenceGap`.                                                                  |
| `scope`              | `<platform>:<short-uuid>` partition key. Scope mismatch on append is a write-time error (`WriterError::ScopeMismatch`).  |
| `prev_line_hash`     | SHA-256 of the previous line's canonical bytes, or one of the genesis anchors below.                                     |
| `prev_line_hashes[]` | Present only on merge lines (`parent_commits[]` non-empty). Each entry must match an earlier line's `compute_line_hash`. |
| `parent_commit(s)`   | Commit SHA(s) being witnessed. Lockstep with `prev_line_hashes[]`.                                                       |
| `validation_at`      | Which hook wrote the line: `pre-commit`, `post-commit`, `post-merge`, `post-rewrite`, or `bootstrap-recovery`.           |
| `rules_sha`          | Effective rule-set digest at write time. Threaded by the pre-commit hook (MLP2-014).                                     |

Genesis anchors (the only legal `prev_line_hash` values on a chain's first
line):

- `GENESIS-FRESH` — the project was started with `anvil start` and no
  pre-existing history was baselined.
- `GENESIS-BASELINED` — the project was baselined onto an existing commit. The
  cutoff commit SHA is recorded on the line body as a separate `cutoff_commit`
  field, **not** glued onto the anchor string. ADR-037 §D-2 keeps the anchor
  namespace closed-set; `GenesisAnchor::parse()` explicitly rejects the
  colon-suffix form (e.g. `GENESIS-BASELINED:<sha>`).

Any other first-line value triggers `VerifyError::UnknownGenesis`. A non-first
line that carries a genesis anchor triggers `VerifyError::StrayGenesis`.

## Hooks that write to the chain

Installed by `anvil hook bootstrap` (run automatically by `anvil start`):

| Hook           | What gets appended                                                                                                 |
| -------------- | ------------------------------------------------------------------------------------------------------------------ |
| `pre-commit`   | One line per commit, `validation_at: "pre-commit"`, with `rules_sha` for the active rule set.                      |
| `post-commit`  | One line, `kind: "post-commit"`, to confirm the commit landed.                                                     |
| `post-merge`   | One **merge line** with `parent_commits[]` + `prev_line_hashes[]` lockstep arrays, built via `merge_witness_plan`. |
| `post-rewrite` | One retroactive line per `<old> <new>` pair on amend / rebase, `validation_at: "post-rewrite"`.                    |

All hooks use the `anvil-witness` flock-serialised writer, so 80-way concurrent
appends from parallel worktrees are safe.

The flock acquire is **bounded** (CIB-124): a stalled holder times out to
`ChainBroken`-adjacent `WriteFailed` rather than hanging the commit forever. The
default ceiling is 5 seconds — generous, since a normal hold is sub-millisecond,
so it only fires against a genuine wedge. Operators on slow storage or with very
high parallel-worktree commit volume can override it by exporting
`ANVIL_WITNESS_LOCK_TIMEOUT=<seconds>` (a positive integer) in the environment
the git hooks and the daemon run under; a malformed value is ignored with a
warning and the 5-second default is used.

Two caveats:

- **Compound worst-case.** Under the phase-3 daemon-first routing, a wedged lock
  is waited on by _both_ legs: the hook's daemon RPC has its own ~2 s socket
  timeout, then the hook falls back to the embedded writer, which waits the
  override against the _same_ lock. So the real worst-case commit hang is
  roughly `2s + ANVIL_WITNESS_LOCK_TIMEOUT`, not the override alone — set 30 and
  a wedge can hang for ~32 s.
- **Daemon restart.** Git hooks are fresh subprocesses and pick up the var
  immediately, but the daemon reads it **once at start**. After changing the
  var, restart the daemon (`anvil stop && anvil start`) so its leg uses the new
  value; otherwise the daemon path keeps the old value until the next restart.

## Verifying the chain

### `anvil doctor`

`anvil doctor` runs the cheap presence checks (witness file exists, parses,
chain is linear-or-DAG-consistent). It is the operator's first stop when the
pre-push hook prints `anvil: witness chain broke`. Output groups checks by
severity; the witness-related check fails with a remediation line pointing at
this runbook.

### `anvil audit-chain`

`anvil audit-chain` is the L5 audit — it re-walks a branch's commits and reports
any that lack a corresponding L3 witness line. Catches commits that bypassed
pre-commit / pre-push (admin overrides, force-push manipulation, hook-failure
recovery).

```bash
anvil audit-chain                                  # walk HEAD; default threshold 5
anvil audit-chain --branch main --since v0.6.0-beta
anvil audit-chain --threshold 1                    # fail on first unwitnessed commit
anvil audit-chain --max-runtime 60                 # cap wall-clock walk at 60s
anvil audit-chain --rescan                         # also re-evaluate today's rules vs history
anvil audit-chain --json                           # machine-readable report
```

Exit code is **non-zero when drift meets or exceeds `--threshold`** (inclusive,
so `--threshold 5` flips on the 5th unwitnessed commit). The nightly GitHub
workflow `anvil-audit.yml` in `.github/workflows/` (installed via the activation
orchestrator, MLP2-053) runs this command and emits a `degraded:audit-drift`
marker when threshold is met.

A Kindling observation row is appended to `anvil/kindling/audit-chain.ndjson` on
every run. Note this sidecar lives under `anvil/kindling/` — a different tree
from the rollover manifest at `anvil/witness/manifest/chain.ndjson`. Downstream
consumers tail it as a plain NDJSON stream.

### `anvil hook pre-push`

The pre-push hook calls `verify_chain_dag` across all witness segments before
allowing the push. A chain break here is the canonical "something tampered or
something dropped a line" signal:

```
anvil: witness chain broke — run `anvil doctor` for details
```

The hook does not block on rollover events, scope mismatches that resolve to a
different scope partition, or merge lines whose parents are present.

## Failure modes and recovery

### 1. `verify_chain` failed with `ChainBreak`

```
chain break at anvil/witness/active.ndjson:42: prev_line_hash mismatch
  expected <hash_a>, got <hash_b>
```

Cause: a line was edited, re-ordered, or its predecessor was removed.

- **Diagnose:** open `active.ndjson`, find line 42, compare its `prev_line_hash`
  against `compute_line_hash` of line 41. If line 41 was hand-edited, the bytes
  no longer match its recorded hash.
- **Recover:** revert the edit. If the edit is in git history, `git revert` the
  offending commit; the witness file is tracked content, so the revert restores
  it. Re-run `anvil doctor`.
- **Do not** rewrite lines to "fix" the hash — that is exactly the tamper signal
  the chain is designed to surface.

### 2. `SequenceGap` reported

```
sequence gap at anvil/witness/active.ndjson:17: expected seq 17, got 19
```

Cause: a line was deleted from the middle of the chain. Often the result of an
editor's whitespace-strip or a stray `sed` invocation.

- **Recover:** if the deletion is in the index but not committed,
  `git checkout -- anvil/witness/active.ndjson`. If it is committed,
  `git revert` the commit. If the deletion shipped, see "5. Chain corruption
  shipped" below.

### 3. `OrphanMerge` on a merge line

```
orphan merge parent at anvil/witness/active.ndjson:88 (parent index 1):
  prev_line_hashes[1] = <hash> not found in earlier lines
```

Cause: a merge line's `prev_line_hashes[1]` references a parent's witness line
that is missing from the walked file set. Two real cases:

- **Archive segment missing on disk.** Check `anvil/witness/archive/` against
  the manifest at `anvil/witness/manifest/chain.ndjson`. If a manifest entry
  references an archive not on disk, restore it from git (it is tracked).
- **Branch lacks the parent's witness lines.** The merge happened against a
  branch whose pre-commit hook never ran (e.g. legacy commits, `--no-verify`).
  Recovery is a five-step sequence — skipping any one of them produces a repeat
  OrphanMerge on re-merge:
  1. `git checkout <parent-branch>` — switch to the branch missing witnesses.
  2. Confirm the branch has an upstream (`git rev-parse @{u}` succeeds). If not,
     `git branch --set-upstream-to origin/<parent-branch>` first;
     `--witness-recent` walks `@{u}..HEAD` and silently does nothing when `@{u}`
     is unset.
  3. `anvil hook bootstrap --witness-recent` — appends retroactive witness
     lines.
  4. `git add anvil/witness/active.ndjson && git commit -m "..." && git push` —
     the witness file must be **committed and pushed** before the re-merge,
     otherwise the merged-from hashes still won't exist anywhere the verifier
     can find them.
  5. On the feature branch: rebase or merge from the updated parent, then
     re-attempt the original merge.

### 4. `UnknownGenesis` or `StrayGenesis`

```
first line in <path> does not reference a known genesis anchor: <actual>
```

Cause: someone hand-edited a `GENESIS-FRESH` or `GENESIS-BASELINED` anchor.

- **Recover:** `git log -- anvil/witness/active.ndjson` to find the change. The
  anchor is set once by `anvil baseline` (which emits the `GENESIS-BASELINED`
  line at MLP2-013) or by the first pre-commit append after `anvil start`
  (`GENESIS-FRESH`). Restore the original bytes. Do not glue the cutoff SHA onto
  the anchor string — `parse()` will reject the result and the line becomes
  `UnknownGenesis` instead of recovering.

### 5. Chain corruption shipped to the remote

When a broken chain reaches `main`:

1. Open an incident channel and stop merges to `main` until the chain is
   restored.
2. Identify the boundary commit on `main` via
   `anvil audit-chain --branch main --since <suspected-good-tag>`. The report
   has **two distinct shapes** and they route to different recovery paths — read
   carefully:
   - **Unwitnessed commit:** a line in the `Unwitnessed commits:` section names
     a commit SHA but no chain-break diagnostic was emitted. Example output
     fragment:

     ```text
     Unwitnessed commits (3):
       8f2a1b3c  feat: add metric (no witness line found)
       d0e9c4a2  refactor: extract helper (no witness line found)
       7a6b5c4d  fix: typo (no witness line found)
     Chain integrity: OK
     ```

     Cause: a teammate landed commits without `anvil` on PATH or with
     `--no-verify`. The chain itself is intact — there's just no witness for
     those SHAs. **Route → re-witness (step 3a).**

   - **Chain break:** the `Chain integrity` line reports a `ChainBreak` /
     `SequenceGap` / `OrphanMerge` with a specific `active.ndjson:<line>`
     anchor. Example:

     ```text
     Unwitnessed commits (0):
     Chain integrity: FAILED — ChainBreak at active.ndjson:412
       expected <hash_a>, got <hash_b>
     ```

     Cause: a witness line in the chain itself was tampered or replaced. The
     recorded commit may be perfectly fine; the _witness record_ is wrong.
     **Route → revert (step 3b).** Running `--witness-recent` here appends new
     lines on top of a corrupt chain and makes the incident worse.

3. Apply the matching recovery:
   - **3a — Re-witness (unwitnessed-commits route):**
     `anvil hook bootstrap --witness-recent` on a branch based at `main`, commit
     the witness file, push the recovery branch, then merge it. Use this when
     only the witness record is missing and the code is intact.

   - **3b — Revert (chain-break route):** `git revert <boundary-commit>` and
     push. Use this when the underlying commit itself is the problem (admin
     override, single-commit tamper). **Scope this to single-commit incidents.**
     For multi-commit force-push damage, every rewritten commit has a new SHA,
     leaving every downstream witness line orphaned by parent-commit-SHA —
     revert won't restore the chain. In that case escalate to the team and
     rebuild via `--witness-recent` from the last good tag, accepting that the
     merge history will show the rebuild.

4. Run `anvil audit-chain --threshold 1 --branch main` and confirm zero drift
   before re-opening merges.

### 5b. `ChainBroken` from an emptied or deleted `active.ndjson` (CIB-126)

- **Symptom:** a commit is blocked with `chain integrity broken`, but the chain
  is not tampered — `active.ndjson` is zero bytes or missing, with no archive
  segments. A crash mid-write, a disk glitch, or a stray `> active.ndjson` /
  `rm` can cause this. The durable chain-init marker
  (`anvil/witness/.chain-initialised`) survives the event, so the writer refuses
  to reseed genesis over the erased history (ADR-038) rather than silently
  starting a new chain — this is the CIB-126 protection working, not a false
  positive.
- **Diagnose:** confirm the active file is empty/absent and the marker is
  present:
  ```
  wc -c anvil/witness/active.ndjson   # 0, or "No such file"
  ls anvil/witness/.chain-initialised # present
  ls anvil/witness/archive/           # empty / absent
  ```
- **Recover (chain is committed — the normal case):** restore the last committed
  chain from git, then re-commit:
  ```
  git checkout -- anvil/witness/active.ndjson
  ```
  `anvil hook bootstrap --witness-recent` does **not** repair this — it appends
  through the same guarded path and no-ops on a `ChainBroken` chain.
- **Recover (chain was never committed — nothing to restore):** the history is
  genuinely gone. Acknowledge the loss and permit a fresh reseed by removing the
  marker (this is the only sanctioned reason to delete it):
  ```
  rm anvil/witness/.chain-initialised
  ```
  The next commit seeds a new genesis and re-writes the marker.
- **Fresh-clone caveat:** the marker is local runtime state (gitignored,
  self-healing via backfill on the first `append_chained`). On a fresh clone
  that has not yet committed, the marker is absent, so an active file zeroed
  _before_ the first commit still reseeds. The protection is active from the
  first commit onward.

### 6. `WriterError::ScopeMismatch` on append

```
scope mismatch: writer is configured for `linux:8d3f1a2c` but
                line.scope is `darwin:8d3f1a2c`
```

Cause: someone hand-built a line with the wrong scope partition.

- **Recover:** never hand-build witness lines. Use `anvil hook <kind>` so the
  writer derives the scope from the active project identity. If the line came
  from a script, fix the script.

### 7. Bootstrap recovery for missing witnesses

If a teammate committed without `anvil` on PATH, or pre-commit was bypassed with
`--no-verify`, the commits land without witness lines. The fence will fire on
next push.

```bash
anvil hook bootstrap --witness-recent
```

This walks `git rev-list --reverse @{u}..HEAD` and appends a retroactive
`validation_at: "bootstrap-recovery"` witness line per commit that is not yet
witnessed. Oldest-first ordering (`--reverse`) keeps the hash chain anchored to
each commit's actual parent. Commit the resulting `anvil/witness/active.ndjson`
change before pushing again.

Operator notes:

- **Upstream required.** The walk is `@{u}..HEAD`. On a branch without a
  tracking remote it walks zero commits and silently does nothing. Set one first
  via `git branch --set-upstream-to origin/<branch>`, or fall back to
  `git rev-list --reverse <base-sha>..HEAD --` to identify the unwitnessed range
  manually.
- **Run from one operator at a time.** The flock makes a single line append
  atomic, and the writer reads the chain tip inside `append_witness` so seqs
  stay monotonic across the boundary. The real concurrency race is between
  `commit_is_witnessed(SHA)` and the subsequent append: both reads happen
  outside the flock, so two operators running `--witness-recent` simultaneously
  on the same branch can both observe "not yet witnessed" for the same SHA and
  each append a retroactive line for it, producing **duplicate witnesses for the
  same commit SHA** (each line internally consistent, but the chain now carries
  two entries that claim to witness the same commit). Coordinate via the
  incident channel before running.
- **The commit of the witness file does trigger pre-commit.** The recovery
  commit gets one more witness line appended; that is expected and correct, not
  a recursive loop. **Do not use `--no-verify` on the recovery commit** — it
  would land the recovery without witnessing it and re-create the original
  problem on next push.
- **Idempotence within a single operator.** Re-running `--witness-recent` after
  committing the prior run is safe — `commit_is_witnessed()` walks the committed
  chain (active + archives) and skips SHAs that already appear. Re-running
  before committing the prior run's output (uncommitted `active.ndjson`) will
  also skip already-appended SHAs because `commit_is_witnessed()` reads the file
  regardless of git tracking state; the duplicate-witness risk above is
  specifically about **concurrent** operators racing on the same chain.

## Rollover boundary checks

A rollover is **not** a chain break. When active crosses 1000 lines or 1 MiB the
writer:

1. Computes the merkle of the active bytes inside the flock.
2. Renames `active.ndjson` → `archive/<scope>-<seq>-<merkle>.ndjson`.
3. Appends a `ManifestEntry` to `manifest/chain.ndjson` with the archive path,
   full SHA-256, line count, and `[start..=end]` seq range.
4. Truncates `active.ndjson` to zero bytes for the next append.

If a rollover lands on an archive name that already exists with identical bytes,
the operation is idempotent. If the names match but the bytes differ that is a
corruption signal — surface it via `anvil doctor` and treat it as case 5 above.

**Interrupted rollover.** The four steps run inside the flock, but a `kill -9`
or disk-full between step 2 (rename) and step 3 (manifest append) leaves the
archive on disk with no manifest entry. The chain itself remains valid:
`witness_paths()` enumerates files on disk regardless of manifest state, so
`verify_chain_dag` still walks the orphaned segment. Recovery is to run
`anvil audit-chain --rescan`, which rebuilds the audit view from the on-disk
segments. The manifest gap heals at the next rollover (the writer always
appends; it never rewrites past entries).

## Verifying archives against the manifest

The manifest's `merkle` field is the full hex SHA-256 of the archived file's
bytes. To verify an archive byte-for-byte without re-walking the chain:

```bash
shasum -a 256 anvil/witness/archive/<scope>-<seq>-<merkle>.ndjson \
  | awk '{print $1}'
```

Compare the output to the `merkle` value for that `archive_path` in
`anvil/witness/manifest/chain.ndjson`. A mismatch is the same severity as a
chain break — open an incident.

## Air-gap and CI

`anvil audit-chain`, `anvil doctor`, all `anvil hook` subcommands, and the
witness writer make **zero network calls**. The
[air-gap runbook](anvil-air-gapped.md) and
`crates/anvil-cli/tests/air_gapped.rs` enforce this under a Linux
network-namespace harness; any new witness-chain surface that ships on the MLP /
INTL slate must extend that test file (see the air-gap runbook's "How to extend
the gate" section).

The nightly L5 audit workflow lives at
[`crates/anvil-cli/src/templates/anvil-audit-workflow.yml`](../../crates/anvil-cli/src/templates/anvil-audit-workflow.yml)
and is copied into `.github/workflows/` as `anvil-audit.yml` by the activation
orchestrator. ADR-037 §D-9: active by default; operators disable by commenting
out the `schedule:` block, not by deleting the workflow.

## Provenance

- Filed 2026-05-18 as the N4 doc-lane closure for `v0.7.0-beta` (Wave 4
  release-gate evidence; see [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md)).
- Implementation is load-bearing across MLP-002 (genesis), MLP-003..-005 (DAG
  shape), MLP-015 (L5 audit), MLP2-011 (DAG verify), MLP2-012 (manifest stream),
  MLP2-013 (`GENESIS-BASELINED` anchor), MLP2-014 (`rules_sha` threading),
  MLP2-015 (80-writer stress test), MLP2-037 (`--witness-recent` bootstrap),
  MLP2-053..-056 (audit-chain workflow + Kindling row + rescan + time budget),
  MLP2-061..-063 (rollover/L4/policy hardening, shared `witness_paths`).
- Doctrine anchor: ADR-037 (Witness Chain and L4 Policy Framework) and ADR-036
  §D-1 (air-gap-first).
