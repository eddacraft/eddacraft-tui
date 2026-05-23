<!-- APS: Design spec for RELMGMT Phase 3 (agent-driven release) -->

# Agent-Driven Release — RELMGMT Phase 3

Date: 2026-04-20
Module: `RELMGMT`
Status: Ready
Supersedes: Part of `RELMGMT-009` (interactive shell orchestration) and
`RELMGMT-010` (manifest-gated `/release` skill). Both remain in history; the
runtime surfaces they describe are being replaced.

Superseded in part: `RELORCH-001` replaces the single-script release surface,
skill handoff, validation, and runtime command-shape sections with
[`2026-05-10-release-orchestration-design.md`](./2026-05-10-release-orchestration-design.md).
The RELMGMT Phase 3 trade-off that rejects persistent local manifests is retained
as a hard RELORCH constraint.

## Goal

Replace the current `scripts/release.sh` + `.release/manifest.json` +
`/release` skill contract with a two-surface split: a thin deterministic
preflight script the operator runs manually, and a `/release` skill that
owns every judgment step by reading live repo state each turn.

## Why

Phase 2 shipped a 21k-line shell orchestrator that writes a manifest and
hands off to a Claude skill that reads it. The handoff is fragile:

- Preflight sections drift out of sync with the runbook; each missed step
  surfaces downstream as a skill refusing to continue or asking the
  operator to backfill.
- Manifest staleness is a recurring failure mode — a crashed run, a retry
  from a different shell, or a resumed session all break the 24 h freshness
  gate.
- Making the shell path reliable enough to stop failing requires roughly
  10× more iteration than is justified for a solo-operator cadence.

The shell keeps winning the deterministic parts (tests, clippy, fmt) and
losing the judgment parts (version pick, branch strategy, changelog
review). Agents keep winning the judgment parts and losing nothing on the
deterministic parts so long as the deterministic parts actually ran. Split
the surfaces along that fault line.

## Scope

In scope for Phase 3:

- Rewrite `scripts/release.sh` to a preflight-only runner (~100 lines).
- Rewrite `.claude/skills/release/SKILL.md` to work from live state with
  no manifest contract.
- Delete the `.release/` directory, its manifest artefact, and the
  `.gitignore` entry.
- Update `docs/runbooks/release-runbook.md` quick-start to match.
- Reopen RELMGMT in `plans/index.aps.md` and add Phase 3 tasks to the
  module file.

Out of scope:

- Changing what an Anvil release contains (cargo-dist pipeline, binary
  formats, dual-repo publish).
- Changing release cadence, semver policy, or channel strategy
  (Phase 1 ratified outcomes stand).
- Adding an audit-trail persistence layer (see Tradeoff below).

## Surfaces

### Script — `scripts/release.sh`

Preflight only. No flags, no prompts, no git operations, no GitHub calls,
no manifest writes.

Runs in order, printing `PASS <step>` / `FAIL <step> (reason)` to stderr:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `pnpm format:check`
5. `pnpm lint:check`
6. `pnpm typecheck`
7. `pnpm test`

The `pnpm test` step invokes the root script, which runs
`nx run-many -t test --exclude=@eddacraft/anvil-e2e`. Package scope is
managed in `nx.json` / workspace config, not via an in-script filter list —
the old bundled-package helper is retired.

Prints a summary table at the end:

```
step                                  result
------------------------------------  ------
cargo fmt                             PASS
cargo clippy                          PASS
cargo test                            PASS
pnpm format:check                     PASS
pnpm lint:check                       PASS
pnpm typecheck                        PASS
pnpm test                             PASS
```

Exits with the count of failed steps (0 on clean pass). Operator invokes
it directly: `./scripts/release.sh`. Re-runnable; no side effects.

### Skill — `/release`

Single entry point for the judgment half. Removes the manifest gate
entirely. First prompt to operator:

> Did `./scripts/release.sh` pass clean? (y/n)

If `n`: ask them to run it and come back, exit. If `y`: proceed.

From that point the skill works from live repo state only:

1. **Assess.** Read `git log dev..main`, existing tags, `CHANGELOG.md`
   head, changed crates/packages since last tag. Propose a version bump
   (patch/minor/major) and a release type (beta/production) with rationale.
2. **Choose branch strategy.** Based on changed-path size and risk,
   propose direct vs. stabilisation. Operator confirms.
3. **Open tracking issue.** `gh issue create --label release --title
   "release/vX.Y.Z"` with preflight-confirmed and assess-results as the
   first comment. Store the issue number in session context; do not
   persist to disk.
4. **Version bump + tag.** Walk version bump commits, tag push, back-merge
   if stabilisation.
5. **Monitor workflow.** `gh run watch` on the cargo-dist workflow. Re-read
   on each wake; resumable.
6. **Verify artefacts.** Check the 8 expected assets (6 archives + 2
   installers) on both `EddaCraft/anvil-001` and `EddaCraft/anvil`.
7. **Changelog review.** Cross-reference diff against `CHANGELOG.md`
   entries; surface gaps.
8. **Docs triage.** Apply `docs/guides/release-doc-checklist.md` against
   changed paths.
9. **Comms draft.** Release message from runbook template.
10. **Cleanup + close issue.** Back-merge to `dev`, release-branch
    deletion (if any), public-repo prerelease flag, install.eddacraft.ai
    health check, close the tracking issue.

All state is read live at each step — the skill is resumable from any
point by re-invoking `/release` and answering the "where did we get to"
prompt. If the operator starts a fresh Claude session mid-release, they
say "continuing release vX.Y.Z from step N" and the skill picks up by
re-reading git/gh.

### Deletions

- `.release/` directory and any files inside it.
- `.release/` entry in `.gitignore` (was RELMGMT-008).
- The manifest gate, validation, and freshness check in
  `.claude/skills/release/SKILL.md`.
- The orchestration phases (init, branch, tag, workflow kickoff, manifest
  write) in `scripts/release.sh` — roughly 90% of current content.

## Tradeoffs

**Audit trail moves.** Phase 2 kept a durable `.release/manifest.json`
with preflight results, SHAs, and run IDs. Phase 3 replaces this with
(a) terminal output the operator sees live, and (b) comments on the GH
tracking issue written by the skill. Net: the durable record is the GH
issue, not a local file. Accepted because the issue is the actual handoff
artefact between operator and any later reviewer; a local JSON was only
ever a Claude-handoff convenience.

**Preflight is less aggregated.** The script no longer reports to the GH
issue directly; the operator answers a y/n and the skill records that in
the issue. If the operator lies about preflight passing, the first `gh
run watch` will still catch it — we don't depend on the y/n for
correctness.

**No script-side retry on flaky tests.** Intentional — flaky tests
surface to the operator's terminal and they rerun `./scripts/release.sh`.
The shell is no longer trying to be smart about retry; the agent is no
longer trying to be smart about preflight.

## Phase 3 task list

| ID          | Title                                                    | Priority |
| ----------- | -------------------------------------------------------- | -------- |
| RELMGMT-012 | Slim `scripts/release.sh` to preflight-only              | high     |
| RELMGMT-013 | Rewrite `/release` skill to work from live state         | high     |
| RELMGMT-014 | Remove `.release/` directory + gitignore entry           | medium   |
| RELMGMT-015 | Update release runbook quick-start to match new flow     | medium   |

Dependencies: 012 and 014 are independent; 013 depends on nothing runtime
but should land in the same PR as 012 so the runbook update (015) covers
both. 015 depends on 012 + 013.

## Validation

- `./scripts/release.sh` run on a clean checkout exits 0 and prints the
  summary table.
- `./scripts/release.sh` with an induced failure (e.g. `touch src/bad.rs`
  with invalid syntax) exits non-zero and the summary table shows the
  failing step.
- `/release` invoked with no `.release/` directory present proceeds past
  the old manifest gate without error (gate removed).
- Dry-run of `/release` on a fake version produces a draft tracking issue
  with the expected sections filled from live state.
- Runbook quick-start references `scripts/release.sh` and `/release` with
  no manifest language remaining.

## Rollback path

Phase 2 surfaces are in git history. If the agent-driven flow proves
worse in practice, revert `scripts/release.sh` and `.claude/skills/
release/SKILL.md` to pre-Phase-3 state, restore `.release/` in
`.gitignore`, and reopen Phase 2 as the active surface. RELMGMT index
status reverts to `11/11 — Complete`. No data-migration concerns — the
manifest format was ephemeral by design.
