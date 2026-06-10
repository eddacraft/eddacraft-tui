# ADR-078: Capsule Retention and Prune Policy

## Status

**Accepted** — 2026-06-10, council review (accept-with-changes; the required
changes — tracked-deletion-via-index semantics, committer-date ordering with
full tie-break, `--keep-last 0` refusal, `--root` validation, symlink skip,
unordered-capsule accounting, exit-code table, TOCTOU/partial-failure
semantics — are applied in the accepting commit)

## Date

2026-06-10

## Context

ADR-074 fixed the review-capsule v0 format and deliberately kept staging
**on-demand and external by default**: `anvil capsule create --out <dir>`
refuses a non-empty directory and its help text tells the user to keep the
capsule outside the repository. ADR-073's state-boundary table names
`anvil/evidence/capsules/` as the home for capsules **when staged in-repo**,
tracked "on request". What neither ADR states is a retention policy: what
happens to staged capsules over time, and whether anything ever deletes them.

Left unstated, two failure modes follow:

1. **Accidental accumulation.** A team that opts into in-repo staging gets an
   unboundedly growing evidence tree with no stated disposal story —
   GITGOV-013's framing: "indefinite accumulation is a stated choice, not an
   accident" must become true.
2. **Accidental deletion.** Any *automatic* cleanup (create-time rotation,
   hook-driven pruning, age-based GC) would silently destroy governance
   evidence — the exact opposite of the capsule's purpose. A capsule is an
   audit artefact; deleting one must be a deliberate, attributable act.

A constraint shapes the mechanics: the frozen `anvil.capsule.v1` manifest
(`deny_unknown_fields`) carries **no creation timestamp**, so a prune surface
cannot honestly order capsules by "age" from capsule contents alone.
Filesystem mtimes are not portable across clones (git does not preserve
them). The only deterministic, repo-present ordering available without a
schema change is the head commit's **committer date** (`range.head` in the
manifest, resolved as `git log -1 --format=%ct <head>`), against the
repository — the same repo-present posture `anvil capsule verify` already
pins (ADR-074, GITGOV-009).

## Decision

**Retention is keep-until-explicitly-pruned. Nothing in Anvil ever deletes
capsule evidence automatically. A `prune` surface makes explicit disposal
safe, deterministic, and dry-run by default.**

1. **Default posture (restated, now normative):** capsules are created
   on-demand to an **external** `--out` directory; their lifecycle outside the
   repo is the operator's (send to the reviewer, attach to the audit, delete
   when done). Anvil states no retention for external capsules, and `prune`
   does not manage them.
2. **In-repo staging is opt-in and unpruned by default:** a team may stage
   capsules under `anvil/evidence/capsules/` (ADR-073). Staged capsules are
   tracked governance evidence and accumulate **indefinitely by default** —
   that is the deliberate, stated choice. Bounding the tree is an explicit
   operator action, never a side effect.
3. **`anvil capsule prune` is the explicit disposal surface:**
   - Operates on a staging root: `anvil/evidence/capsules/` by default,
     `--root <dir>` to override. `--root` must resolve to a directory inside
     the repository working tree and never inside `.git` (mirroring
     `create`'s `refuse_out_inside_git_dir` guard); anything else is refused.
   - **Candidate identification is schema-gated:** only immediate
     subdirectories of the root whose `manifest.json` parses as
     `anvil.capsule.v1` are candidates. Anything else — unparseable, missing
     manifest, foreign files — is skipped with a stderr warning and never
     deleted. Entries that are **symlinks are skipped** with a warning, never
     followed. A readable root that yields **zero candidates** produces a
     warning (the operator probably pointed at the wrong level).
   - **Selection is `--keep-last <N>`, N ≥ 1:** orderable capsules are sorted
     by (head-commit **committer date**, then head SHA lexicographic, then
     capsule directory name) — a total, deterministic order; the newest `N`
     are kept and the rest are selected for deletion. `--keep-last 0` is
     **refused** — delete-everything is a manual `git rm` decision, not a
     prune invocation. If `N` ≥ the orderable count, nothing is selected and
     prune exits 0 with a message.
   - **A capsule whose head commit the repository does not know** (shallow
     clone, foreign capsule) cannot be ordered honestly and is **always
     kept**, with a stderr warning. Unordered capsules sit **outside** the
     `--keep-last` accounting entirely: `N` applies only to the orderable
     population, and the unordered count is reported separately.
   - **Dry-run by default:** without `--apply`, prune prints the would-delete
     list on stdout and deletes nothing (exit 0). `--apply` performs the
     deletion after an **independent re-scan** — it never trusts a prior
     dry-run's output (the tree may have changed in between; the dry-run is
     advisory).
   - **Deletion goes through the git index for tracked capsules:** staged
     capsules are tracked files (ADR-073), and a filesystem-only delete would
     be silently reversible by `git restore`/`git checkout` — a "prune" that
     can quietly undo itself is not a real disposal surface. `--apply`
     therefore removes tracked capsule paths via the index (the `git rm -r`
     equivalent: working tree + staged deletion) and removes untracked
     leftovers from the filesystem. **Committing the staged deletion remains
     the operator's manual act** — prune never commits — so the attribution
     and review of *having pruned* stays in the normal git workflow.
   - **Partial failure is reported, not masked:** if some deletions fail
     (permissions, I/O), prune continues, reports each failure, and exits
     non-zero; succeeded deletions are listed so the resulting state is
     explicit.
   - **Exit codes:** `0` = success, including dry-run and nothing-to-do;
     `1` = any error (refused `--root` or `--keep-last 0`, not a git
     repository, or one or more deletions failed). Warnings (skipped
     entries, unordered capsules, zero candidates) go to **stderr** and do
     not change the exit code (ADR-002 posture).
   - Machine-readable (`--json`) output is **deferred** until a consumer
     exists; the dry-run stdout list is line-oriented (one path per line) so
     scripts can consume it meanwhile.
4. **No automatic pruning of capsule evidence anywhere:** `create` never
   rotates or evicts, no hook prunes, no age-based GC exists. If full-tree
   growth ever forces automation, that is a new decision superseding this
   one, not a default drift.

## Rationale

Capsules exist so governance evidence survives and travels; a retention
policy that quietly deletes them would undermine the wedge's core claim. The
inverse — pretending accumulation isn't happening — fails the
tooling-honesty doctrine. Stating "keep until explicitly pruned" plus a safe,
explicit prune surface makes the cost visible and the disposal deliberate.

Ordering by the head commit's committer date (not mtime, not author date,
not manifest time) keeps prune deterministic for a given repo + staging
tree, works across clones, requires no schema change to the frozen
`anvil.capsule.v1`, and reuses the repo-present contract verification
already established. Committer date (not author date) is the recency signal
that survives rebases coherently. The ordering is a **convenience sort, not
a security control**: committer dates are writable by anyone with push
access (`GIT_COMMITTER_DATE`), so prune claims determinism given a repo
state, never tamper-resistance — tamper-evidence lives in the witness chain
and digests (ADR-074), not in prune ordering.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Keep-until-explicitly-pruned + dry-run-default `prune --keep-last N` via the git index (chosen) | Evidence never destroyed silently; deterministic total order; no schema change; disposal is staged + attributable (a commit) | Staged trees grow until an operator acts; `--keep-last` needs the repo present |
| Age-based retention (`--older-than`, auto-GC) | Familiar log-rotation ergonomics | No honest age source: manifest is frozen without a timestamp; mtimes don't survive clones; auto-GC silently destroys evidence |
| Create-time rotation (keep newest N on create) | Zero-effort bounding | Automatic deletion of governance evidence as a side effect of creating more — worst failure mode |
| Docs-only manual path (`rm -rf` guidance, no command) | No code | Unguided deletion of evidence directories; no schema gate against deleting non-capsule data; GITGOV-013 explicitly wants the surface before v1 |
| Add `created_at` to the manifest and order by it | Self-contained ordering | Breaks the frozen v0 schema (`deny_unknown_fields` consumers); producer clock is unverifiable; committer date is already attestable |
| Filesystem-only deletion (no index involvement) | Less invasive (never touches the index) | Tracked files reappear on `git restore`/`checkout` — a disposal surface that silently undoes itself; the operator gets no signal |

## Consequences

- **Positive:** Retention is now a stated policy; the accumulation default is
  deliberate and documented; disposal is explicit, schema-gated, dry-run
  first, staged through the index, and leaves a reviewable commit trail.
- **Positive:** No change to the frozen `anvil.capsule.v1` schema or to
  `create`/`verify`/`explain` semantics.
- **Negative:** Teams staging in-repo must remember to prune (or accept
  growth); `prune` requires the repository (consistent with `verify`, but
  unlike `explain`); external capsule directories are out of scope.
- **Negative (known ordering hazard):** committer-date ordering reflects
  commit recency, not merge recency — a long-lived branch merged today
  carries an old head committer date and can sort below newer ephemeral
  capsules. The dry-run list is the review point for exactly this case;
  operators auditing long-lived branches should check it before `--apply`.
- **Risks:** An operator runs `--apply` against the wrong root; a future
  contributor adds "convenient" auto-pruning; concurrent capsule creation
  between dry-run and `--apply`.
- **Mitigations:** Dry-run is the default and `--apply` re-scans
  independently; candidates are schema-gated and symlinks skipped so a wrong
  root deletes nothing unless it genuinely contains capsules; `--root` is
  confined to the repo working tree; `--keep-last 0` is refused; this ADR
  records "no automatic pruning of capsule evidence" as normative so any
  future automation requires superseding it.

## References

- Related ADRs: ADR-002 (warnings over blocks), ADR-072 (Git substrate),
  ADR-073 (state boundary — `anvil/evidence/capsules/`), ADR-074 (capsule v0
  format, frozen manifest, repo-present verification; this is its retention
  sub-decision)
- APS modules: GITGOV-013 (`plans/modules/git-native-governance.aps.md`)
- Code anchors: `crates/anvil-cli/src/commands/capsule.rs`
  (`refuse_out_inside_git_dir` — the guard `--root` mirrors),
  `crates/anvil-capsule/src/manifest.rs` (`CAPSULE_SCHEMA`,
  `deny_unknown_fields`)
