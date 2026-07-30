# A1 Wow-Start Activation — Execution Plan

**Date:** 2026-05-04
**Owner:** feat/A1 implementation stream
**Source artefacts:**

- [`plans/index.aps.md`](../index.aps.md)
- [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) — Tier A1
- [`plans/archive/modules/launch-flow-readiness.aps.md`](../archive/modules/launch-flow-readiness.aps.md)

This document records the chosen sequence for delivering LAUNCH items
`-002, -006, -008, -009, -010, -011, -012, -013, -014, -015, -016` as
six reviewable PRs against `dev`. It is **not** an APS module — APS
ownership and progress remain in `launch-flow-readiness.aps.md`.

## Conflicts reconciled

Per the implementation directive (codebase = current truth, RELEASE-PLAN
= priority, APS = acceptance):

1. **APS LAUNCH header reads `5/16` Complete; RELEASE-PLAN A1 paragraph
   reads `In Progress 5/14`.** APS module file is authoritative; counts
   match the codebase: 5 Complete of 16 visible tasks. RELEASE-PLAN
   prose is stale by 2 — non-blocking; will refresh during PR 2 cleanup.

2. **`start` is currently a clap alias for `welcome`**
   (`crates/anvil-cli/src/main.rs:78`). LAUNCH-006 promotes it to the
   activation entrypoint. No conflict — that promotion is the work.

3. **`v0.5.1-beta` shipped 2026-05-03** (per RELEASE-PLAN), but the APS
   index header still anchors prose to `v0.5.0-beta`. Non-blocking;
   touch-up belongs in a separate docs PR.

## Dependency graph

```text
PR 2 (LAUNCH-008/-012)  ── state vocabulary ──┐
                                              ├──► PR 1 (LAUNCH-002/-006)
PR 5 (LAUNCH-015/-016)  ── repo-language profile ──┤
                                                   ├──► PR 4 (LAUNCH-010/-014)
PR 6 (LAUNCH-013)       ── install detector ──── (independent)
                                                   │
PR 3 (LAUNCH-009/-011)  ── MCP + watch fallback ──┘  (depends on PR 2)
```

## Sequence

| Order | PR | Items | Branch | Risk |
| ----- | -- | ----- | ------ | ---- |
| 1 | PR 2 | LAUNCH-008, LAUNCH-012 | `launch/a1-protection-states` | low (contract) |
| 2 | PR 5 | LAUNCH-015, LAUNCH-016 | `launch/a1-language-profile-filters` | medium (filter regression risk) |
| 3 | PR 6 | LAUNCH-013 | `launch/a1-install-upgrade-guidance` | low |
| 4 | PR 1 | LAUNCH-002, LAUNCH-006 | `launch/a1-start-entrypoint` | medium (composes existing primitives) |
| 5 | PR 3 | LAUNCH-009, LAUNCH-011 | `launch/a1-mcp-activation-fallback` | **high** — mandatory follow-up council |
| 6 | PR 4 | LAUNCH-010, LAUNCH-014 | `launch/a1-first-signal-integrity` | medium |

## Hard constraints (council-locked, do not violate)

- No no-args TUI theatre
- No rule-file injection (`.cursorrules`, `.clauderules`, global AI rules)
- No cloud login, team policy pull, CI setup, default git hook install
- No demo fixtures, challenge files, or guaranteed-catch prompt catalogues
- No Windsurf, VS Code, Copilot CLI, Codex CLI, process auto-attach, DRVR
- Cursor + Claude Code only for v1
- MCP pre-write validation is the only v1 enforcement claim
- Watch mode is fallback only, labelled as fallback
- Validate (or surface honestly) #1195 and #1197 fixes inside PR 3

## Per-PR contract

Each PR MUST:

1. Reference the LAUNCH item(s) in commits (`LAUNCH-XXX: ...`).
2. Include tests proving acceptance criteria.
3. Pass council review (minimum: `council-reviewer` + `adversarial-reviewer`).
4. Remediate every council finding before opening the PR.
5. If risk is high or council returns >5 findings / any high-risk /
   any false-protection / any MCP-attachment-correctness / any
   unsupported-language finding — run a follow-up council after
   remediation.
6. PR description follows the template in the implementation directive
   (Summary, Work items, Behaviour, Truthfulness, Validation, Council,
   Out of scope).
7. After PR open: wait 7m, run `address-pr-reviews` skill, address
   actionable items, commit fixes, reply where appropriate.

## Hand-off notes

- The Cargo.lock drift (`tokio 1.52.1 → 1.52.2`) from an ad-hoc
  `cargo update` is stashed on the original `feat/A1` worktree under
  message `ad-hoc cargo update tokio 1.52.1->1.52.2 (unrelated to
  LAUNCH work)`. Pop or drop in a separate chore commit; do not bundle
  into LAUNCH PRs.
- Worktree convention: `../anvil-001.<branch-slug>` (matches
  `wt-<branch-slug>` from `docs/guides/worktree-policy.md` allowing
  the existing prefix already in use on this machine).
- Branch from `dev` (matches `docs/guides/branching-strategy.md`).
- PR target: `dev`.
