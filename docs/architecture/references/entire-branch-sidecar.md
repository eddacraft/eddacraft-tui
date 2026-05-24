# Entire's Branch-as-Sidecar Pattern for Session Storage

| Type  | Authority | Owner | Status | Freshness                                                                                                              |
| ----- | --------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | GV2   | Live   | Last reviewed 2026-05-25 against `plans/modules/graph-v2-foundation.aps.md` and external Entire architecture reference |

| Upstream                                                                                                                   | Downstream                                                                                   |
| -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| External Entire architecture reference, `plans/modules/graph-v2-foundation.aps.md`, `plans/modules/usage-analytics.aps.md` | Graph/provenance planning, Kindling session-storage ideas, future sidecar/checkpoint designs |

**Source:** [entireio/cli](https://github.com/entireio/cli) — MIT, ~4.2k stars

**Primary reference:**
[`docs/architecture/sessions-and-checkpoints.md`](https://github.com/entireio/cli/blob/main/docs/architecture/sessions-and-checkpoints.md)

**Why we care:** Entire stores AI agent session data (transcripts, prompts,
checkpoint snapshots, token usage) entirely in git, on parallel branches that
travel with the repo. The pattern is directly relevant to anything in Anvil that
captures agent activity (kindling, council outputs, APS work-item history) and
wants it to survive `git clone` without an external store.

## TL;DR

Entire uses a **two-tier branch system** linked to main via commit trailers:

1. **Shadow branches** (`entire/<commit[:7]>-<worktreeHash[:6]>`) — temporary,
   per worktree+base-commit. Hold full WIP file snapshots plus a
   `.entire/metadata/` overlay. Enable intra-session rewind before any commit
   exists.
2. **Metadata branch** (`entire/checkpoints/v1`) — permanent, sharded by
   checkpoint ID. The committed record that ships with the repo.
3. **Linkage** — user commit gets an `Entire-Checkpoint: <12-hex>` trailer; the
   trailer is the only pointer between the two histories. No git notes, no
   parent-child relationship.

## The Two Tiers

### Tier 1 — Shadow branch (in-flight state)

```
ref:    entire/<commit[:7]>-<worktreeHash[:6]>
type:   standard branch (not orphan)
keyed:  by base commit + worktree
```

Tree contents at any temporary checkpoint:

```
<worktree files…>                     # actual WIP code
.entire/metadata/<session-id-1>/
  ├── full.jsonl                      # transcript
  ├── prompt.txt                      # checkpoint-scoped user prompts
  └── tasks/<tool-use-id>/            # task checkpoints
.entire/metadata/<session-id-2>/      # concurrent sessions interleave
  └── …
```

Key properties:

- The shadow branch contains **the worktree itself**. A "checkpoint" is just a
  commit on this branch. That's what makes pre-commit rewind possible: there is
  a real tree object for every saved state.
- Multiple concurrent sessions on the same worktree share the shadow branch —
  their metadata directories sit side by side, identified by `<session-id>`.
- Auto-migrated when the base commit changes (e.g. user commits, then resumes).

Triggers for temporary checkpoints:

| Event            | Strategy  |
| ---------------- | --------- |
| On save          | Temporary |
| On task complete | Temporary |

### Tier 2 — Metadata branch (permanent record)

```
ref:    entire/checkpoints/v1
layout: sharded by checkpoint ID (12-hex random)
```

Tree layout under one checkpoint:

```
<id[:2]>/<id[2:]>/                    # e.g. a3/b2c4d5e6f7/
  ├── metadata.json                   # CheckpointSummary, aggregated stats
  ├── 0/                              # session 0 (0-based indexing)
  │   ├── metadata.json               # session-specific
  │   ├── full.jsonl                  # transcript
  │   ├── prompt.txt
  │   └── content_hash.txt
  ├── 1/                              # additional sessions for same checkpoint
  └── …
```

Key properties:

- Sharded like git's loose-object directory pattern — keeps tree fanout
  manageable for repos with thousands of checkpoints.
- 0-based session subdirectories acknowledge that **a single user commit can
  have multiple sessions contributing to it** (parallel agents, multiple
  iterations).
- Commits on this branch carry their own trailers:
  `Entire-Session: 2026-01-20-<uuid>`, `Entire-Strategy: manual-commit`.

## Linking Main History to the Sidecar

Linkage is **one-directional via commit trailer**:

```
commit <sha> on main
Author: …
Date:   …

    Add feature X

    Entire-Checkpoint: a3b2c4d5e6f7
```

Lookup direction:

```
user commit ──extract trailer──▶ a3b2c4d5e6f7
                                 │
                                 ▼
            entire/checkpoints/v1 tree at a3/b2c4d5e6f7/
```

Notable design choices:

- **Trailer, not git note.** Trailers ride inside the commit message, so they
  survive `git log`, `git format-patch`, mirrors, and forks without extra
  config. Notes need explicit fetch/push refspecs.
- **No parent pointer.** The metadata branch's commits do not have the user's
  commit as a parent. The two histories are entirely independent at the DAG
  level — the only join is the trailer string.
- **Trailer is removable.** The `prepare-commit-msg` hook injects it; the user
  can strip it pre-commit and the user-facing commit will have no checkpoint
  reference.

## Lifecycle

```
agent activity ──┐
                 ▼
         shadow branch    (entire/<commit[:7]>-<worktreeHash[:6]>)
         + temp checkpoint per save / task
                 │
                 │ user runs `git commit`
                 ▼
         prepare-commit-msg hook
           → injects "Entire-Checkpoint: <id>" trailer
                 │
                 ▼
         post-commit hook
           → CondenseSession reads accumulated shadow state
           → writes committed checkpoint to entire/checkpoints/v1
                 │
                 ▼
         shadow branch may be GC'd or migrated to new base commit
```

`WriteCommittedOptions` payload (from the docs): `CheckpointID`, `SessionID`,
`Strategy`, `Branch`, `Transcript`, `Prompts`, `Context`, `FilesTouched`,
`TokenUsage`.

## Design Rationale

The Entire docs describe _what_ and _how_, not _why this over alternatives_. The
following is our analysis.

### vs git notes (`refs/notes/*`)

- Notes are not fetched/pushed by default; users have to configure refspecs.
- Notes are awkward for storing rich tree structures (transcripts, multiple
  files per checkpoint).
- Notes are rebase-hostile — they're attached to commit SHAs, which change.
- Tooling support is uneven: some hosts (older GitLab, mirrors) handle notes
  poorly.

### vs separate repository

- No atomic operation pairs main commit with metadata commit.
- `git clone` doesn't bring metadata along automatically.
- Two repos = two GC schedules, two sets of credentials, two URLs to keep in
  sync.

### vs metadata in main tree (`.entire/` checked in)

- Pollutes diffs, blame, and the working tree on every commit.
- Forces every collaborator to deal with files they don't care about.
- Rebase-hostile — touching `.entire/` files creates phantom merge conflicts.
- Breaks `.gitignore` purity.

### Why the two-tier split specifically

- **Shadow branch must contain the worktree** so rewind has a real tree to
  restore from. That's incompatible with the permanent record's compact, sharded
  form.
- Condensing shadow → metadata at commit time turns transient state into a
  curated artifact — gives you the equivalent of a "release note" for the
  agent's session.
- The shadow branch can be aggressively GC'd; the metadata branch is the history
  you keep.

## Trade-offs

- **Trailer pollution.** Every Entire-tracked commit on main has an extra line
  in the message. Small but visible noise on `git log`. Mitigation: trailer is
  optional and the user can strip it.
- **Branch clutter.** `git branch -a` lists `entire/...` branches unless the
  user filters. Power users will want a `refs/entire/*` namespace under `refs/`,
  not under `refs/heads/`.
- **Shadow branch bloat.** Including full worktree contents in temp checkpoints
  means the shadow branch grows with every save. The doc doesn't cover GC policy
  — when does abandoned shadow state get pruned?
- **Rebase / amend hazard.** Trailer-based linkage breaks if the user amends or
  rebases without re-running the post-commit hook. The metadata commit still
  exists but the user commit no longer references it.
- **Mirror semantics.** Tools like `git-sync` will replicate the sidecar branch
  by default — generally desirable, but means private session data goes wherever
  the repo goes. Worth flagging in security/privacy docs.
- **Schema versioning.** The `v1` suffix on `entire/checkpoints/v1` is an
  explicit version anchor. Anyone borrowing the pattern should adopt the same
  discipline.

## Relevance to Anvil

Capture-adjacent surfaces that could borrow this pattern:

- **kindling-capture** (PostToolUse hook) — currently writes to an external
  store. A git-native option would mean session data ships with `git push` for
  free; pay the cost in ref management and shadow-branch GC.
- **council** outputs — multi-agent review findings could live on a
  `refs/anvil/council/v1` ref, sharded by review ID, with a trailer linking back
  to the reviewed commit.
- **APS work-item history** — `plans/` already lives on main and has the
  pollution problem the sidecar pattern solves. A `refs/anvil/aps-history/v1`
  branch could hold execution traces without bloating `plans/*.aps.md`.

Things to copy verbatim if we adopt:

- Two-tier split (shadow for in-flight, metadata for permanent).
- Trailer-based linkage with explicit version anchor in the ref name.
- ID-sharded tree layout for the permanent branch.
- Hook-driven condensation at commit boundaries.

Things to do **better than Entire**:

- Use `refs/anvil/*` namespace, not `refs/heads/anvil/*`, to keep `git branch`
  output clean.
- Document GC policy for shadow refs from day one.
- Define explicit behavior on `git commit --amend` and `git rebase` — re-run the
  hook, or surface a warning.
- Add the security/privacy section the Entire doc is light on: who can read the
  metadata branch on push, and is anything sensitive ever in transcripts.

## Open questions

- How does Entire prune shadow branches? Time-based? Reference-counted?
- Does `entire/checkpoints/v1` have a single linear history per repo, or one
  metadata commit per user commit? (The docs imply per-checkpoint commits but
  don't say whether they're chained.)
- How does Entire behave with shallow clones, partial clones, or sparse
  checkout? (Not covered in the docs we read.)
- What's the on-disk size impact for a repo with 1000+ checkpoints?
