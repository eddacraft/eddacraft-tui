# ADR-037: Witness Chain and L4 Policy Framework

## Status

Accepted (2026-05-13 during Wave 0 carry-forward reconciliation — witness-chain
shape is the load-bearing primitive for MLP-002 and remains unchanged after
`v0.6.2-beta`)

## Date

2026-05-07

## Context

Anvil's defense-in-depth model (L0–L5; see ADR-036 §D-1 / spec §2)
needs a primitive that:

1. Carries deterministic per-commit proof of which Anvil layers fired
   and what they decided.
2. Travels with the repo via standard git operations (push / clone /
   worktree) so cross-machine federation works without hosted
   infrastructure.
3. Survives parallel branches, merges, and rebases without merge
   conflicts.
4. Detects tampering by adversarial actors who modify the proof
   record.
5. Lets a server-side gate (or pre-push hook) verify "this commit is
   safe to merge" by reading a single in-tree file at the commit's
   tree.

Existing alternatives evaluated (in round-2 brainstorm and rejected
for primary use):

- **Git notes ref (`refs/notes/anvil`):** doesn't push by default;
  most users don't know about it; clones often miss it; loses the
  self-contained property.
- **Signed commits (cryptographic):** strong but requires key
  management; conflicts with users' own GPG / SSH signing; heavy for
  v1.
- **Hosted sidecar service:** breaks the air-gapped doctrine
  (ADR-036 §D-1 principle 8 — "v1 must work fully without internet").

The user explicitly proposed the in-tree append-only file pattern
(round-2 brainstorm §1.7). It cleanly satisfies all five requirements
and emerges with three useful properties:

- `merge=union` solves the merge-conflict problem natively.
- `--no-verify` becomes self-defeating (no line appended → L4
  detects missing witness → rejects).
- Verification is a tail-of-file read; no special tooling.

## Decision

### D-1 — Per-commit witness line in `anvil/witnessed.ndjson`

The pre-commit hook (and other relevant hooks) appends a single
JSON-line per commit to `anvil/witnessed.ndjson` (tracked, in-tree,
under `anvil/` not `.anvil/` so it travels via worktree creation).

Witness line shape (canonical):

```jsonc
{
  "v": 1,
  "project_id": "01997e4a-1b2c-7345-8901-abcdef123456",
  "tree": "<git tree hash being committed>",
  "parent_commit": "<git parent hash>",       // single value for normal commits
  "parent_commits": ["...", "..."],           // present only for merge commits
  "scope": "linux:8d3f1a2c",
  "anvil_version": "0.6.0",
  "rules_sha": "<sha of effective rule set>",
  "agent": {"task_id": "...", "step_id": "...", "parent_session_id": "..."},
  "L0": {"status": "miss", "reason": "no-mcp"},
  "L2": {"status": "ok", "watcher_health": "ok"},
  "L3": {"status": "ok", "rules": 42, "mode": "block", "backend": "daemon", "latency_ms": 120},
  "prev_line_hash": "<sha256 of previous line>",
  "ts": "2026-05-07T12:34:56Z"
}
```

Inner shape (`Diagnostic` payloads, etc.) remains canonical
[`anvil.diagnostic.v1`][diag]. New metadata lives only on the witness
line envelope.

[diag]: ../specs/2026-04-26-diagnostic-envelope-coordination.md#canonical-inner-shape-diagnostic

### D-2 — Hash chain anchored at genesis

`prev_line_hash` chains every line to its predecessor. Anchors:

- `GENESIS-FRESH` — greenfield repo (`anvil start` on new git init)
- `GENESIS-BASELINED` — adopted-existing-repo (`anvil baseline`)

Tampering breaks the chain at the affected line and propagates forward.
L4 verification walks the chain forward from a known anchor; mismatch
→ structured rejection.

This is **provenance, not authentication.** A determined attacker can
forge witnesses claiming any decision; the chain hash makes
modification of *historical* witnesses tamper-evident, and L4's
`validate_at_l4` policy (D-5) provides the authentication backstop by
revalidating at server side when policy demands.

### D-3 — Active + archive + manifest, with rollover

```
anvil/witness/
├── manifest/
│   └── chain.ndjson              # append-only manifest events (merge=union)
├── active.ndjson                 # current witness lines (capped at 1000 lines / 1 MB)
└── archive/
    └── <scope-prefix>-<seq>-<merkle-prefix>.ndjson   # frozen, immutable
```

Active file capped at **1000 lines OR 1 MB** (whichever first).
Rollover (atomic, inside one `flock`):

1. Compute SHA-256 Merkle root of `active.ndjson`.
2. Rename to content-addressed
   `archive/<scope-prefix>-<seq>-<merkle-prefix>.ndjson`.
3. Append `archive_sealed` event to `manifest/chain.ndjson`.
4. Create new `active.ndjson`; first line's `prev_line_hash` is the
   merkle root.

Pruning default: `keep-all` (configurable via `enforcement.witness.archive_retention`).

Content-addressed naming so two machines parallel-rolling produce
non-conflicting filenames. `merge=union` on `chain.ndjson` reconciles
parallel manifests at git merge time.

### D-4 — Lock-protected chain integrity

Every witness write acquires `flock(LOCK_EX)` on `anvil/witness/.lock`
(separate sentinel file so we can rotate active without losing the
lock). Inside the lock:

1. Read chain head (last line of active.ndjson; or merkle root of
   most recent archive if active is empty).
2. Compose witness line with `prev_line_hash = chain_head`.
3. Check rollover threshold; rollover if needed.
4. Append.

Lock hold: <1ms typical, <10ms at rollover. Validation work (rule
evaluation) happens **outside** the lock — only the append itself is
locked. Multi-agent waves of 80+ concurrent commits serialise on the
lock briefly but don't block validation.

`flock` auto-releases on file close (kernel-managed) — crash-safe.

### D-5 — L4 policy framework

`anvil/policy.yml` (or `.json` / `.toml` per format flexibility)
declares per-branch rules:

```yaml
required_anvil_version: "0.6.0"        # optional exact-semver floor
baseline:
  cutoff_commit: a3b2ea4e...           # everything before is legacy
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
    on_block: reject
  - pattern: dependabot/*
    require: l4_only
    on_no_witness: validate_at_l4
  - pattern: '*'
    require: l4_or_l3
    on_no_witness: validate_at_l4
```

L4 enforcement applies the matching pattern's rules. `on_no_witness:
validate_at_l4` runs server-side validation against the diff,
generating an L4 witness in `refs/notes/anvil-l4` (separate notes ref
so the App / CI runtime can write without polluting the in-tree
ledger).

A commit is accepted if **either**:
1. Valid L3 witness exists (chain verified) AND its decision satisfies
   policy, OR
2. L4 validation passed (witness in notes ref).

This handles legitimate "no L3 witness" scenarios:
- `--skip-hooks` opt-out users
- External contributors without Anvil installed
- Bot commits (Dependabot, Renovate, release-please)
- GitHub web/mobile/API direct edits
- Force-pushed rewritten history
- Squash merges that drop component witnesses
- History grafts (combined with `anvil baseline`)

### D-6 — DAG-aware verification at merge commits

Merge commits carry `parent_commits[]` and `prev_line_hashes[]` —
arrays, one entry per parent. The witness chain is a DAG, not a
linear list, joined at merges identical to git's own DAG.

L4 walks: for each commit being pushed, find the witness, verify each
`prev_line_hash` matches a known chain head from the corresponding
parent. Cost: `O(commits-being-pushed)`, not `O(repo-history)`.

### D-7 — `validate_at_l4` does NOT update the in-tree ledger

L4-generated witnesses live in `refs/notes/anvil-l4`, not in
`anvil/witnessed.ndjson`. The in-tree ledger only carries witnesses
generated locally (L0–L3). This separation:

- Lets the L4 enforcement point (CI action / pre-receive / GitHub App
  v2) write its own witnesses without amending the push.
- Preserves the `--no-verify` self-defeating property (no in-tree line
  for the commit → in-tree verification fails, but if L4 generated a
  notes-ref witness, that's acceptable per policy).
- Keeps the in-tree witness file clean (only "what the local side
  attested").

### D-8 — Witness file location: `anvil/` (no dot)

Tracked metadata that must propagate via worktrees / clones lives in
`anvil/` (no dot), not `.anvil/`. The dot-prefix directory is reserved
for local execution state (gitignored by default). This sidesteps
worktree-creation-tooling that skips dotfiles, and matches the
`.git/` (local) vs `.github/` (tracked) convention.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|---|---|---|
| **In-tree NDJSON with hash chain** *(chosen)* | Travels via git natively; `merge=union` solves conflicts; tamper-evident; no infra; verifiable from the file alone | Slightly more git-tracked file churn; hash chain ≠ cryptographic auth |
| Git notes ref (`refs/notes/anvil`) | No working-tree pollution | Doesn't push by default; users don't know about it; clones miss it |
| Signed commits | Cryptographic strength | Key management; conflicts with user's GPG; heavy for v1 |
| Hosted sidecar service | Centralised dashboard | Breaks air-gapped doctrine; requires infra |
| Post-commit append (no chain) | Simpler | No tamper detection; weaker provenance |

The in-tree NDJSON + hash chain pattern was proposed by the user in
round-2 brainstorming. It fits the air-gapped doctrine (ADR-036 §D-1
principle 8) and the deterministic-pre-commit doctrine (the witness
write happens in the same atomic operation as the commit).

### Why per-commit and not per-event

Earlier sketches considered per-validation-event lines (one per
`gate_evaluated`, similar to Kindling). Rejected because:

- Volume is too high for in-tree storage
- Per-commit is what L4 cares about (the unit of acceptance is the
  commit, not the validation)
- Kindling already covers per-event in the local-only governance
  store; witness file is the cross-machine portable subset

So Kindling and the witness file are **complementary, not competing**
(spec §10.2). Kindling is local rich SQLite governance facts;
witness file is minimal portable proof.

## Consequences

- **Positive — Cross-machine federation is implicit.** Every push
  carries the witness chain; `merge=union` reconciles parallel
  branches; L4 verifies at receive. No special infra.
- **Positive — `--no-verify` is detected automatically.** No witness
  appended → L4 sees missing witness → rejects (or applies
  `validate_at_l4` policy).
- **Positive — Tampering breaks the chain.** Modifying historical
  witnesses propagates a chain break forward; L4 detects.
- **Positive — Air-gapped works.** No cloud calls; verification is
  local file read.
- **Positive — Auditable.** `git log -p anvil/witnessed.ndjson` shows
  every witness ever made; reviewers see what was attested.
- **Positive — Compatible with Kindling.** Witness lines carry
  Kindling identifiers (`session_id`, `gate_eval_id`) for forensic
  dig-in when machine-local Kindling is accessible.
- **Negative — In-tree noise.** Every commit adds ~1 KB to
  `anvil/witnessed.ndjson`. Bounded by rollover; archives compress
  well in git pack delta encoding.
- **Negative — Schema is public contract.** Once shipped, changing
  the witness line shape is a breaking change. Mitigation: `v: 1`
  field for forward versioning.
- **Negative — Not cryptographic.** A determined attacker with
  same-UID access can forge witnesses. Defense: L4's `validate_at_l4`
  policy catches false witnesses by revalidating; baseline-suspicious
  detection catches large suppression injection.
- **Negative — `merge=union` requires `.gitattributes` opt-in.**
  `anvil start` writes the line; users who delete `.gitattributes`
  break it. Mitigation: hook detects missing `.gitattributes` line
  and warns.
- **Risk — Lock contention under burst.** Per the 82-commits stress
  profile, ~80 concurrent agents → serialise on lock. <10ms total at
  rollover, <1ms typical. If real-world contention exceeds, the
  "crude alternative" (lazy rollover; allow active to overshoot
  slightly) is the simplification path.
- **Risk — `chain.ndjson` parallel rollover.** Two machines
  simultaneously rollover → parallel archive files. Mitigated by
  content-addressed naming; manifest's `merge=union` reconciles.
- **Risk — Force-push rewrites the chain history.** Acceptable: new
  commits get fresh witnesses on replay; old commits in the rewritten
  range are orphaned (which is the point of a force push).

## References

- **Spec:** [`2026-05-07-anvil-multilayer-protection-architecture.md`](../specs/2026-05-07-anvil-multilayer-protection-architecture.md) §5, §7
- **Brainstorm:** [`2026-05-07-anvil-multilayer-protection-brainstorm.md`](../brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md) §1.7 (witness file user proposal), §3 (alternatives considered)
- **Companion ADRs:**
  - ADR-036 — Daemon scope, discovery, OS-boundary policy (parent of execution-scope concept)
  - ADR-038 — Hook surface + noise discipline (companion: hooks that write the witness)
  - ADR-039 — Baseline policy + hard-pinned classes (companion: baseline genesis + suspicious-refresh)
- **APS modules:**
  - `plans/archive/modules/multilayer-protection.aps.md` — MLP-002 (witness chain), MLP-006 (L4 policy), MLP-012 (rules_sha)
- **Related ADRs:**
  - ADR-001 — Planless-first (witness machinery is anvil-managed; user doesn't author it)
  - ADR-003 — New edges only (witness chain anchors at baseline cutoff)
  - ADR-004 — Suppression syntax (`@anvil-ignore` interacts with witness L3 status)
  - ADR-031 — Validation latency rubric (hook time budgets)
- **Inner-shape contract:**
  - [`2026-04-26-diagnostic-envelope-coordination.md`](../specs/2026-04-26-diagnostic-envelope-coordination.md) — `Diagnostic` carried inside witness lines is the canonical envelope
