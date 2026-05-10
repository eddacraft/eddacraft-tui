<!--
APS Module: Release Orchestration (v2)
======================================
Implements the deterministic scripts/release/*.sh command surface that the
rewritten /release skill (.claude/skills/release/SKILL.md) and release runbook
(docs/guides/release-runbook.md) describe as a target architecture.

PR #1368 (branch `review/build-release-simplification`) rewrote the skill and
runbook ahead of design. This module owns the design and implementation work
to make those docs honest. Design must precede docs — RELORCH-001 produces
the spec the docs should describe; until -001 lands, the docs are a target,
not a contract.

Predecessor: RELMGMT (archive/modules/release-management.aps.md) — Complete.
Supersedes (parts): plans/specs/2026-04-20-relmgmt-agent-driven-release-design.md
                    (single-script preflight design; replaced by per-phase
                    commands, but its ratified Tradeoffs — no persistent
                    manifest, GH issue as durable record — are inherited).

Council review: Planning Council session `plan-9a6b3a94` (architect,
pragmatic-lead, adversarial-reviewer, all COUNTER, 6 objections). Module
revised 2026-05-09 against those objections.

See: plans/aps-rules.md
-->

# Release Orchestration

| ID      | Owner | Status   | Progress |
| ------- | ----- | -------- | -------- |
| RELORCH | —     | In Progress | 1/11     |

**Execution authorisation:** Operator request "start RELORCH" on 2026-05-10 authorises executing `RELORCH-001` from Proposed state under `plans/aps-rules.md` status rule 1.

**Predecessor:** [release-management](../archive/modules/release-management.aps.md) (RELMGMT — Complete)
**Supersedes (in part):** [2026-04-20-relmgmt-agent-driven-release-design.md](../specs/2026-04-20-relmgmt-agent-driven-release-design.md) — its multi-script removal stands; its no-persistent-manifest tradeoff is inherited as a hard constraint below.
**Council review:** session `plan-9a6b3a94` (2026-05-09)

## Purpose

Land a deterministic per-phase command surface under `scripts/release/` so the
`/release` skill and release runbook can be thin operator wrappers rather than
re-implementations of release logic in agent prose.

PR #1368 rewrote `.claude/skills/release/SKILL.md` and
`docs/guides/release-runbook.md` to describe that command surface ahead of any
design or implementation. None of the commands exist yet; the legacy
`scripts/release.sh` is the only release-side shell asset on `dev`. This
module owns the design (`RELORCH-001`) and the work to honour it. Until
`RELORCH-001` ratifies the contract, the docs PR describes a target, not an
agreement.

## Coherence With The New Operating Model

RELORCH is a release-command implementation module for the target operating
model in
[`2026-05-09-plan-build-release-operating-model.md`](../specs/2026-05-09-plan-build-release-operating-model.md).
It does not own branching strategy, APS lifecycle semantics, or release
authority vocabulary beyond the release command surface.

Current-state versus target-state boundary:

- Current-state release assets still include legacy `scripts/release.sh` and
  may still execute against the repository's `dev` integration model while the
  migration is incomplete.
- Target-state release commands must be compatible with tagging a verified
  `main` SHA and must not encode `dev -> main` promotion as a permanent
  assumption.
- Any command that accepts `--base main --head dev` during migration must treat
  that as compatibility input, not as the canonical future release topology.

Release state authority:

| State | Authority |
| --- | --- |
| Operator decisions, recovery narrative, resumability comments | GitHub release tracking issue |
| Candidate metadata and generated notes | Deterministic command output and CI artefact, shape defined by `RELORCH-001` |
| Released source snapshot | Annotated tag on `main` |
| Distributed artefacts | GitHub Release assets |
| Shipped-state reconciliation for APS | Machine-readable release record |

The GitHub tracking issue is the single durable operator log. It is not the
canonical shipped-state artefact. `RELORCH-001` must define how release commands
create or locate the release record without reintroducing the old mutable
`.release/manifest.json` handoff.

## In Scope

- A design spec (`RELORCH-001`) that supersedes the relevant parts of the
  2026-04-20 spec and inherits its no-persistent-manifest constraint.
- A test harness (`RELORCH-002`) that gates every command on its JSON / exit-
  code contract and runs in CI on every PR touching `scripts/release/`.
- One deterministic command per release phase under `scripts/release/`:
  `assess`, `preflight`, `prepare`, `promote`, `tag`, `monitor`, `verify`,
  `closeout`.
- Synchronisation of `.claude/skills/release/SKILL.md` and
  `docs/guides/release-runbook.md` with the as-built commands; decommission
  of legacy `scripts/release.sh` only after a differential trust-building
  window (see `RELORCH-011`).

## Out of Scope

- Changing what an Anvil release contains (cargo-dist pipeline, archive
  formats, dual-repo publication strategy). DIST owns that surface.
- Changing release cadence, semver policy, or channel strategy — RELMGMT
  Phase 1 ratified outcomes still apply.
- Public-repo / install-site delivery infrastructure — `verify.sh` consumes
  those surfaces but does not own them.
- Net-new release features (cosign attestations, SBOMs, etc.) — own module
  if pursued.

## Constraints (load-bearing — `RELORCH-001` must honour)

These are inherited or ratified before -001 begins. They are *not* open
questions for the spec to redecide.

1. **No persistent on-disk state between commands.** Each command reads from
   `git`, `gh`, and stdin/argv; writes to stdout (JSON) and the GitHub
   tracking issue (durable record). No `.release/manifest.json`, no working-
   tree state file, no per-run cache directory. The 2026-04-20 spec deleted
   the manifest after Phase 2 failed at the freshness gate; this module
   inherits that decision. If a transient is unavoidable (e.g. multi-edit
   atomicity inside `prepare.sh`), it must be `mktemp`-scoped to a single
   process and cleaned up on any exit path.
2. **GitHub tracking issue is the single durable operator log.** The 13-field
   metadata block currently described in `SKILL.md` §Resumability lives in
   structured comments on the tracking issue, not in a side-channel file.
   `RELORCH-001` ratifies the comment shape and the parser the skill uses.
   Shipped-state truth remains the release record that joins tag, APS items,
   artefacts, and verification evidence.
3. **Idempotency is local-state-only by default.** Commands must be safe to
   re-run before any irreversible side effect. `tag.sh` (`RELORCH-007`) is
   the explicit exception: pre-push is idempotent, post-push requires
   recovery, not retry. The contract must distinguish the two phases.
4. **Cross-platform Bash.** Every command runs on macOS and Linux CI. No GNU-
   only flags without a portable fallback.

## Risks

- **Idempotency drift.** Commands may *claim* idempotency without testing
  it. The harness (`RELORCH-002`) must include a `kill -9` mid-run / re-run
  case for every command, not just unit-level checks.
- **JSON contract drift between commands and skill.** If the skill's
  invocations and the command outputs diverge, the failure surfaces silently
  in the next release. Schema lives in `RELORCH-001`; CI check fails the PR
  on schema mismatch.
- **`gh` / `git` auth assumptions.** Every command shells to `gh`; failure
  modes when token scope is wrong, repo is private, or auth has expired must
  be explicit and recoverable, not "command exits 1 with no diagnostic."
- **Decommission timing.** Removing legacy `scripts/release.sh` before
  parity is proven leaves operators with no working preflight on a real
  release. `RELORCH-011` gates removal on a differential window, not a
  single dry-run.
- **Module drift to indefinite-progress.** Comparable hardening modules
  (DOCSYNC at 11/22) have stalled. Closure criterion below is the gate.

## Closure Criterion

The module is Complete when:

1. All 8 commands exist, pass the harness contract on every PR, and have
   each driven at least one real release end-to-end.
2. `scripts/release.sh` has been deleted from `dev` (per `RELORCH-011`'s
   differential-window gate).
3. `.claude/skills/release/SKILL.md` and `docs/guides/release-runbook.md`
   reference only commands that exist; `grep -rn 'scripts/release\.sh' .claude docs scripts`
   returns no live references.
4. The skill's startup probe finds every command on a fresh clone.

If after 3 real releases any of (1)–(4) is still false, re-scope: either
trim the command surface (drop a command, fold it into another) or extend
the closure criterion explicitly. Do not let the module sit open at
"6/11 — In Progress" indefinitely.

## Rollback Path

If `RELORCH-001` proves wrong after `RELORCH-005` (`prepare.sh`) is built —
e.g. the no-persistent-manifest constraint forces unworkable command-to-
command coupling — the rollback is:

1. Stop further command work.
2. Open a new ADR proposing the constraint change with concrete failure
   evidence from the harness.
3. If accepted, re-spec via a new RELORCH-001 revision; if rejected, revert
   any commands built against the broken assumption to `dev` baseline (legacy
   `scripts/release.sh` is still present until `RELORCH-011` lands).
4. The 2026-04-20 spec's flow remains the working fallback throughout.

## Phasing

The module ships in two phases inside this single file. Phase 1 produces a
working orchestration nucleus that lets the docs become honest; Phase 2
covers the remaining commands.

- **Phase 1 (nucleus, 6 items):** `RELORCH-001` (spec), `RELORCH-002`
  (harness), `RELORCH-003` (`assess`), `RELORCH-004` (`preflight`),
  `RELORCH-010` (`closeout`), `RELORCH-011` (wire-up + decommission).
- **Phase 2 (5 items):** `RELORCH-005` (`prepare`), `RELORCH-006`
  (`promote`), `RELORCH-007` (`tag`), `RELORCH-008` (`monitor`),
  `RELORCH-009` (`verify`).

Phase 1 alone does not retire legacy `scripts/release.sh` — `RELORCH-011`
gates the removal on Phase 2 commands also being green through the
differential window. But Phase 1 is enough for the docs PR to describe a
contract that exists for the operator-entry, preflight, and closeout
phases, with the rest explicitly Phase-2-tracked.

## Sequencing

- `RELORCH-001` first; everything cites it.
- `RELORCH-002` second (was previously claimed parallel — it is not, the
  harness encodes the JSON schema -001 produces).
- `RELORCH-003..-010` (commands) gated on -002 existing, then parallelisable
  in pairs.
- `RELORCH-011` last; gates on Phase 1 + Phase 2 commands all green through
  the differential window.

## Interfaces

**Depends on:**

- DIST (cargo-dist workflow, asset matrix) — `monitor.sh` and `verify.sh`
  read this surface.
- `gh` CLI, `git`, `cargo`, `pnpm` — toolchain surface every command shells
  to.

**Exposes:**

- `scripts/release/*.sh` — per-phase command surface.
- A JSON output schema per command (defined in `RELORCH-001`).
- A structured comment shape on the GH tracking issue carrying the 13-field
  metadata `SKILL.md` §Resumability currently lists.

**Consumers:**

- `.claude/skills/release/SKILL.md`
- `docs/guides/release-runbook.md`

---

## Tasks

### RELORCH-001: Command surface design spec

- **Status:** Complete
- **Phase:** 1
- **Intent:** Define the contract every `scripts/release/*.sh` command obeys —
  arguments, exit codes, JSON output schema, the structured-comment metadata
  shape on the tracking issue, the local-vs-remote idempotency split for
  `tag.sh`, and the failure-reporting shape the `/release` skill consumes.
  Honour the load-bearing constraints (above) without redebating them. Define
  how the command surface emits or locates the release record while preserving
  the no-persistent-local-manifest constraint.
- **Expected Outcome:** A new design doc under `plans/specs/` that
  supersedes the relevant parts of `2026-04-20-relmgmt-agent-driven-release-design.md`
  and is the single document `RELORCH-002..-011` cite:
  [`2026-05-10-release-orchestration-design.md`](../specs/2026-05-10-release-orchestration-design.md).
- **Validation:** Spec links from this module; spec explicitly addresses each
  of the four constraints above; spec defines the tracking-issue comment shape,
  release-record shape, and harness schema in a form `RELORCH-002` can consume.
- **Completed:** 2026-05-10 — Added
  [`2026-05-10-release-orchestration-design.md`](../specs/2026-05-10-release-orchestration-design.md)
  and marked the superseded RELMGMT Phase 3 sections.
- **Confidence:** medium — main risk is the structured-comment design
  ratifying without surprises.
- **Files:** `plans/specs/<date>-release-orchestration-design.md`,
  `plans/specs/2026-04-20-relmgmt-agent-driven-release-design.md` (mark
  superseded sections).

---

### RELORCH-002: Test harness for command surface

- **Status:** Proposed
- **Phase:** 1
- **Intent:** Provide the harness that gates every command on the JSON /
  exit-code contract from `RELORCH-001`. Include `kill -9` mid-run + re-run
  cases for idempotency proof, fixture repos, and CI wiring so a contract
  break fails the PR.
- **Expected Outcome:** A CI job runs the harness on every PR touching
  `scripts/release/`. Intentionally broken contract (e.g. drop a JSON field,
  break exit-code semantics) fails the harness; healthy run is green.
- **Validation:** Harness runs on macOS + Linux CI; `kill -9` test is real,
  not mocked.
- **Files:** `scripts/release/_test/`, CI config touchpoints.

---

### RELORCH-003: `scripts/release/assess.sh`

- **Status:** Proposed
- **Phase:** 1
- **Intent:** Produce a structured assessment of the candidate release —
  candidate version, release type, branch strategy recommendation, touched
  areas, risk signal — from live `git` / `gh` state. No persistent state.
- **Expected Outcome:** `bash scripts/release/assess.sh --base main --head dev --json`
  emits a JSON object validating against `RELORCH-001`'s schema; non-JSON
  mode prints a human summary; exit 0 even when no release is warranted.
- **Validation:** Harness contract green; dry run on current `main`/`dev`
  divergence produces expected fields.
- **Files:** `scripts/release/assess.sh`.

---

### RELORCH-004: `scripts/release/preflight.sh`

- **Status:** Proposed
- **Phase:** 1
- **Intent:** Run deterministic local gates (fmt, clippy, tests, lint,
  typecheck, pnpm test) and verify pinned tool versions; reach parity with
  legacy `scripts/release.sh` and add the version-pin checks RELMGMT Phase 3
  deferred.
- **Expected Outcome:** Exit code equals the count of failed gates; `--json`
  mode emits a structured per-gate pass/fail object; toolchain version
  mismatches are explicit, not silent.
- **Validation:** Harness contract green; clean checkout exits 0; induced
  failure (invalid Rust syntax) exits non-zero with the failed gate
  identified in JSON.
- **Files:** `scripts/release/preflight.sh`.
- **Coordinates with:** `RELORCH-011` (differential trust-building window
  before legacy `scripts/release.sh` is removed).

---

### RELORCH-005: `scripts/release/prepare.sh`

- **Status:** Proposed
- **Phase:** 2
- **Intent:** Drive every release-time edit (version surfaces, release notes,
  generated public docs) and create or resume the GH release tracking issue.
  Reconstruct state from git/gh + structured-comment metadata each run; no
  side-channel state.
- **Expected Outcome:** Re-running on the same version is idempotent before
  any push/PR-creation side effect; partial failure leaves a recoverable
  state with the failed step identified in JSON output.
- **Validation:** Harness `kill -9` mid-run case completes successfully on
  re-run; end-to-end dry run against a fake version produces the expected
  diff.
- **Files:** `scripts/release/prepare.sh`.
- **Risks:** Highest-complexity command in the module; multi-file edit
  atomicity is a real problem under the no-persistent-state constraint. May
  need an `mktemp`-scoped transient (process lifetime only).

---

### RELORCH-006: `scripts/release/promote.sh`

- **Status:** Proposed
- **Phase:** 2
- **Intent:** Open or resume the promotion PR (direct or stabilisation
  branch), report merge status, surface conflict / review-block conditions.
- **Expected Outcome:** Re-running while the PR is open returns "awaiting
  merge" cleanly; once merged, returns "merged at <sha>" so the skill can
  proceed.
- **Validation:** Harness contract green; dry run reproduces existing
  release-PR shape on a scratch branch.
- **Files:** `scripts/release/promote.sh`.

---

### RELORCH-007: `scripts/release/tag.sh`

- **Status:** Proposed
- **Phase:** 2
- **Intent:** Verify `main` HEAD, expected version, source provenance; push
  the release tag. Distinguish pre-push (idempotent: re-run safely until
  push succeeds) from post-push (non-idempotent: any retry must be a
  recovery path, not a re-run). Record tagged SHA + workflow-run lookup
  hint in JSON.
- **Expected Outcome:** Pre-push mismatch (wrong remote URL, version
  collision, provenance fail) exits non-zero without pushing. Post-push,
  the command refuses re-invocation for the same version unless `--recover`
  is passed; recovery mode investigates remote state and either resumes
  monitoring or fails clearly.
- **Validation:** Harness exercises both pre-push idempotency (re-run before
  push, same SHA) and post-push recovery semantics (re-run after push,
  refuses without `--recover`).
- **Files:** `scripts/release/tag.sh`.

---

### RELORCH-008: `scripts/release/monitor.sh`

- **Status:** Proposed
- **Phase:** 2
- **Intent:** Locate and watch the cargo-dist workflow for a given tag;
  surface failures with enough structure for the skill to ask the right
  recovery question (retry / abort / emergency).
- **Expected Outcome:** Default mode blocks until terminal state; `--poll`
  returns current state and exits.
- **Validation:** Against a recent tag, JSON output names the failed job
  (if any) and links to logs.
- **Files:** `scripts/release/monitor.sh`.

---

### RELORCH-009: `scripts/release/verify.sh`

- **Status:** Proposed
- **Phase:** 2
- **Intent:** Verify private + public releases, expected cargo-dist asset
  matrix, release provenance, package-manager publication state (Homebrew,
  Scoop, WinGet), and `https://install.eddacraft.ai` health. Optionally
  produce a comms draft from a template.
- **Expected Outcome:** Structured verification report; missing assets,
  provenance mismatches, or unhealthy install site exit non-zero with a
  per-check status field.
- **Validation:** Run against a recently shipped release reproduces the
  manual verification the skill currently performs in prose.
- **Files:** `scripts/release/verify.sh`.
- **Note:** `RELORCH-001` must decide whether this is one command or two
  (`verify-release.sh` + `verify-publishers.sh`); current default is one,
  split if it exceeds ~200 lines or the JSON schema becomes unreadable.

---

### RELORCH-010: `scripts/release/closeout.sh`

- **Status:** Proposed
- **Phase:** 1
- **Intent:** Perform back-merge, release-branch cleanup, public-repo
  prerelease flag, tracking-issue final update, and issue closure once
  verification has passed.
- **Expected Outcome:** Refuses to close while verification has not run
  (input gate); on success, leaves the tracking issue closed with a final
  summary comment matching the `RELORCH-001` schema.
- **Validation:** Harness contract green; dry-run mode prints actions
  without executing; happy-path on a fake release issue closes cleanly.
- **Files:** `scripts/release/closeout.sh`.

---

### RELORCH-011: Wire skill + runbook to as-built commands; retire legacy runner

- **Status:** Proposed
- **Phase:** 1 (partial — Phase 1 commands wired) and 2 (full retirement of
  `scripts/release.sh` after Phase 2 commands ship and pass the differential
  window)
- **Intent:** Once each Phase's commands land, update
  `.claude/skills/release/SKILL.md` and `docs/guides/release-runbook.md` to
  reference the actual commands. Delete legacy `scripts/release.sh` only
  after a differential trust-building window: at least 3 real releases run
  with both `scripts/release.sh` (legacy preflight) and
  `scripts/release/preflight.sh` (new) executed and their outputs compared,
  with no diff failures the operator could not explain.
- **Expected Outcome:** Skill and runbook describe only commands that exist;
  legacy runner is gone; one full real release proves the wiring;
  `grep -rn 'scripts/release\.sh' .claude docs scripts` returns no live
  references.
- **Validation:** Skill's startup probe against the command surface
  succeeds; differential-window log is recorded on the tracking issues for
  the 3 releases used as evidence.
- **Files:** `.claude/skills/release/SKILL.md`,
  `docs/guides/release-runbook.md`, `scripts/release.sh` (deletion). The
  differential-window evidence lives on the tracking issues for the 3
  releases used (per Constraint #2) — not in a checked-in file — so it does
  not reintroduce on-disk state across commands.
- **Coordinates with:** `RELORCH-004`.
