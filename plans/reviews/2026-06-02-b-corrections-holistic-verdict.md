# Daemon Save-time B-corrections — Holistic Council Re-review Verdict

**Date:** 2026-06-02
**Reviewers:** Review council — architect, kernel-maintainer, adversarial-reviewer,
operations-reviewer, security-analyst, pragmatic-lead; synthesised by council-judge.
**Scope:** the *body* of council corrections ("B-corrections") folded into the daemon
save-time Sub-phase A work — not a single diff. Targets:
[the Sub-phase A plan](../execution/2026-06-01-daemon-save-time-subphase-a.md),
[ADR-061](../decisions/061-save-time-daemon-delta-validation.md),
[ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md),
[the validation contract](../specs/2026-06-01-daemon-save-time-validation-contract.md),
and the landed code (`anvil-intercept-proto`, `anvil-kernel/src/graph`, `anvil-checks`).
**Baseline:** the original
[daemon-graph council verdict](./2026-06-01-daemon-graph-council-verdict.md) (the
"do not start as written" gate that B1–B8 + item 8 were written to clear).
**Central question:** is the plan's claim *"all pre-implementation corrections RESOLVED
— Sub-phase A coding may begin"* justified when the corrections are viewed as a whole?

## 1. Verdict — GO-WITH-CONDITIONS

Five personas returned GO-WITH-CONDITIONS; the adversarial reviewer returned NO-GO. The
council-judge adjudicated the split as a **framing dispute, not a factual one**: every
reviewer agrees the substance is real (the `anvil-graph-cache` crate is unbuilt; the
Task 7 `FileSymbols` feed is unspecified). The disagreement is only whether the
*labeling* overstatement is fatal.

**Adjudication.** The adversarial facts are all true — `crates/anvil-graph-cache`,
`certify.rs`, `kernel_cache.rs`, and `run_antipattern_check_bytes` do not exist, and B5
is an Accepted ADR with zero extraction. But the original verdict's B5 step-1 was *"make
the crate-boundary decision and write the ADR,"* which **is** complete (ADR-064 is
Accepted and names the file moves, deps, and the binding kernel-parse decision). The
defect is therefore an **honesty-of-the-stamp problem**: *"all corrections RESOLVED / no
corrections remain"* conflated **decided** with **built**. That is a relabel-plus-Task-0
condition, not a NO-GO — the parallel lane (Tasks 1–5, 10, 11) provably has zero
dependency on the unbuilt crate and can start now. **B5 severity: CRITICAL → MAJOR.**
Gate: **GO-WITH-CONDITIONS.**

## 2. Blocking conditions

All four are MAJOR and are addressed in the same change that records this verdict.

1. **Relabel the B5 stamp + add an explicit Task 0** (extract `anvil-graph-cache` per
   ADR-064 §1–5, hard predecessor of Tasks 6/7/8). The "all RESOLVED / coding may begin"
   stamp is design-resolved-only and overstates readiness.
   *Where:* `subphase-a.md` Resolution-status block + new Task 0 + File Map rows.
   *Gate:* before the Resolution stamp is honest. *Raised by:* all six.
2. **Specify the kernel→daemon `FileSymbols` feed in Task 7.** ADR-064 (the "The
   cache-write path needs a parse" subsection) binds "the daemon does not parse" but
   Task 7/8 never wire how parsed `FileSymbols` arrive —
   so Task 7 either won't compile or the daemon re-pulls tree-sitter (defeating B5). Add
   the feed contract + a `daemon_does_not_link_tree_sitter` guard test.
   *Where:* `subphase-a.md` Task 7. *Gate:* before Task 7 compiles.
   *Raised by:* architect, kernel-maintainer, pragmatic-lead. (Most substantive
   *technical* gap.)
3. **Resolve the sibling-plan / DAG contradiction.** `subphase-a.md:152-153` ("Tasks
   10/11/16 are a Phase-2 merge dependency, NOT a sibling-plan candidate") contradicted
   the old sequencing note ("candidate sibling plan"); and B7 made Task 8 depend on Task
   10's pool, so pool-construction is spine, not detachable. Split Task 10: pool
   construction = spine predecessor of Task 8; only the background-scan loop + Task 11 +
   Task 16 are sibling-able, and still gate Phase-2 merge.
   *Where:* `subphase-a.md` Sequencing notes. *Gate:* before the DAG is buildable /
   Phase-2 merge. *Raised by:* adversarial, operations.
4. **Propagate the B7 net-new auth wording to the authoritative docs.** The execution
   plan's Task 2 was corrected, but `contract §4` and `ADR-061 §7` still said "reuse …
   `validate_workspace_roots`." Precisely: the SO_PEERCRED handshake/transport *is*
   reused; the `validate_workspace_roots` *wiring* is net-new (zero production callers;
   DRVR-001 Wave 2 left it unwired).
   *Where:* `contract §4`, `ADR-061 §7`. *Gate:* before Task 2 is coded against an
   authoritative source. *Raised by:* adversarial.

## 3. Non-blocking cleanups (folded in alongside)

- Drop the stale "flag for Council" on the `run_antipattern_check` wrapper — Council
  ruled the bytes+pool-core / thin-wrapper design correct, no escalation needed.
- Caller count `9 in 8 files` → **`10 in 9 files`** (`sample_analyser.rs` has two sites).
- `reverse_index_consistent_after_delta` must use **multi-step** delta sequences
  (cold-rebuild equivalence), not single-step. (kernel-maintainer)
- Negative assertion on `transition_emits_notification_envelope`: `reason`/`generation`/
  `scan_started_at` must **not** ride the envelope wire until the `NotificationContext`+
  `redact_envelope` prerequisite lands. (operations)
- Security test hardening: symlink-escape test for `renamed.from` (not just `path`);
  admitted-root identity = the once-opened `O_PATH` dirfd (root-level retarget TOCTOU
  above `RESOLVE_BENEATH`); document the default `open`-mode read blast-radius and point
  operators to `allowlist`. (security C1/C2/C3)
- Two-directional `ALL_ANVIL_METHODS` pin test so Task 1's three new method consts can't
  be omitted silently. (adversarial)
- ADR-064 fixes: the `dependents_of` "petgraph traversal" derivation (it is a `HashMap`
  reverse-index lookup; `petgraph` enters via `SymbolGraph`, which the cache also holds);
  the "B5-notes" mislabel → "item 8", with the corrected confinement premise.

*Withdrawn nit:* the claim that the proto crate "already had" a kernel-types dependency
(security F5a) was verified false — commit `56d56b172` (B3) added it; the plan's prose is
correct, no change.

## 4. Confirmed resolved (no action)

- **B2** — honest narrow attestation via `check_families: ["antipattern"]`, consistent
  across all surfaces; `anvil status` never prints "certified"/"structurally safe".
- **B3** — `DiagnosticEnvelope` landed in `anvil-intercept-proto` (`protocol.rs:83`),
  clean, no new dependency cycle; round-trip tests present.
- **B4** — conservative export-surface default via the `GraphDelta.previously_public`
  set-diff; `removed_edges` confirmed always-empty, `dependents_of` used exclusively.
- **B6** — initial assurance state genuinely `Stale(CrossFileResolutionNeeded)`, never
  `Clean`; auto-`request_full_scan` on connect; no silent-`partial`-forever path remains.
- **B7 daemon read-safety** — guarded-bytes core closes the TOCTOU on the daemon path;
  the thin wrapper does not leak an unguarded fallback into the daemon; `renamed.from`
  guarded.
- **Confinement placement (item 8a)** — the original finding's premise was wrong:
  `anvil_home_prefix()` lives in `anvil-intercept`, so `confinement.rs` loads operator
  config via the daemon's own resolver with no `anvil-cli` dependency. Placement correct.

## 5. What can start today vs what is gated

- **Start now (no dependency on the unbuilt crate):** **Task 0** (the ADR-064 extraction
  — do first), Tasks 1, 2 (after condition 4), 3, 4, 5, **Task 10a** (interactive-pool
  construction), and the Task 8 `run_antipattern_check_bytes` predecessor in `anvil-checks`.
- **Gated behind Task 0:** Tasks 6, 7, 8. Task 7 is additionally gated on the kernel
  `FileSymbols` feed (condition 2). Task 8 also needs Task 10a's interactive pool (B7 /
  condition 3). **Task 11** (DoS caps) depends on Task 10's `workspace_pool.rs`
  (the background-scan sub-task, 10b), so it is not a start-now item.

**Bottom line:** the corrections are *defensible in substance but were dishonest in
wording*. With the four MAJOR conditions applied (relabel + Task 0; the `FileSymbols`
feed; the sibling-plan/DAG reconciliation; the contract/ADR B7 propagation) and the nits
folded in, the gate is clean: **GO-WITH-CONDITIONS → conditions satisfied.**
