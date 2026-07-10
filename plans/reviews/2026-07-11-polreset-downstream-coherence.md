# POLRESET Downstream Coherence Review — 2026-07-11

Post-implementation review of the POLRESET conductor (Done 10/10, closed
2026-07-05) and all seventeen downstream policy modules, checking that each
still makes sense against what actually shipped. Four parallel review agents
covered the module groups; every code claim below was spot-checked on merged
main.

## What POLRESET delivered (verified)

- **ADR-098** accepted 2026-07-04, reconciling ADR 002/015/037/040.
- **AD-1 complete**: full OPA → regorus replacement. PR-B (gate policy check
  through the regorus facade, OPAE-003, `2d71c2afa`) and PR-C (deletion of
  `opa.rs`/`evaluator.rs`/`loader.rs`/`bundle.rs`/`library.rs`, `3aa963008`)
  both landed. `crates/anvil-policy` survives holding only `exceptions.rs`,
  `config.rs`, `adversarial/`, `attack/`, `eval/`.
- **AD-3 fully realised in code**: `ControlDecision` is
  `Allow | Warn | Block | Fence | Interrupt` + `#[serde(other)] Unknown`
  (`crates/anvil-kernel-types/src/diagnostics.rs:77`); daemon `Mode` and the
  MCP-local `EnforcementMode` both folded into one shared kernel-types
  `EnforcementMode` posture type.
- Pack validation retargeted to `anvil-policy-engine/src/pack/` (#3138);
  CPOL/IORISK context + risk contracts (#3139); pre-write enforcement routing
  with `ANVIL_POLICY_ENFORCEMENT` kill switch and fail-open `PrewriteBudget`
  (#3165); embedded `anvil-baseline` starter pack proven end-to-end (#3167);
  report-only eval-regression CI (#3170); adversarial depth via ATC #3181 +
  PATT #3175. EXCEPT-004..010 all merged (ADR-100 committed authority).

## Verdicts

| Module | Status today | Verdict |
| ------ | ------------ | ------- |
| POLVAL | In Progress, 5/5 items Done | MINOR STALENESS + status decision needed |
| OPAE | In Progress 8/11 | COHERENT (minor staleness) |
| EVALCI | In Progress | MINOR STALENESS (header lags merges) |
| EXCEPT | In Progress | MINOR STALENESS + **missing extraction item** |
| CPOL | In Progress, 3/3 Done | Flip to **Done** |
| IORISK | In Progress, 3/3 Done | Flip to **Done** (or add explicit scanner-intake items) |
| CPACKS | Draft 0/8 | **NEEDS RE-SCOPE** — 6 of 8 items already delivered |
| ATC | Done | MINOR STALENESS (bookkeeping) |
| PATT | Done | COHERENT |
| ACTAX | Proposed | MINOR STALENESS (borderline re-scope for Phase B) |
| OPAG | Proposed | **NEEDS RE-SCOPE** |
| AGOV | Draft | **NEEDS RE-SCOPE** (its own rescope list, unexecuted) |
| POLCAP | Proposed | MINOR STALENESS + ADR number collision |
| ORGHIER | Draft | MINOR STALENESS (dangling activation gate) |
| POLLC | Draft | MINOR STALENESS |
| COMPLY | Draft | MINOR STALENESS (self-aware, correctly gated) |
| POLFED | Draft | **NEEDS RE-SCOPE** — load-bearing prerequisite void |

## Finding 1 — the AD-2 `anvil-exceptions` extraction is due and untracked

ADR-098 AD-2's extraction trigger was `min(EXCEPT-006 landing, anvil-policy
disposition PR)`. Both fired (#3140; PR-C `3aa963008`), yet no
`crates/anvil-exceptions` exists, `exceptions.rs` still lives in
`crates/anvil-policy`, and no module, CIB item, or decision-log entry tracks
the extraction. POLRESET is Done and never mentions it. AD-2 also says
`crates/anvil-policy` is *ultimately deleted* once the extraction completes —
so every module still targeting that crate for new work is aimed at a
deletion-slated crate.

**Recommend:** file **EXCEPT-012** — extract the graph-free `anvil-exceptions`
crate (kernel-types-speaking) per ADR-098 AD-2; fold in the caller-less
`is_suppressed_at`/`filter_suppressed` disposition
(`crates/anvil-policy/src/exceptions.rs:688,693,739`) that EXCEPT-010 punted
"to the OPAE rebuild" (no OPAE item carries it).

## Finding 2 — modules that need re-scope

### POLFED (sharpest)
- Prerequisite "OPAE bundle primitives" no longer exists as plan-of-record:
  `bundle.rs` was deleted in PR-C and post-reset OPAE explicitly lists
  "remote bundle marketplace, federation, signing, or SSO" as out of scope.
  The module even contradicts itself (Interfaces block says OPAE no longer
  owns bundles; conceptual model + rescope item 4 still say it does).
- Acceptance criterion cites `anvil policy bundle sync` — no `bundle`
  subcommand exists in `PolicyCommand`.
- The shipped equivalent of "bundle primitives" is POLVAL's pack
  manifest/metadata in `anvil-policy-engine/src/pack/`; the planned boundary
  ADR should be re-titled POLVAL/POLFED. Approval provenance must cite
  ADR-100. Work items target the deletion-slated `anvil-policy` crate.

### OPAG
- Promotion checklist line "POLRESET design gate accepted and OPAE first
  slice promoted" is satisfied but unchecked; the sole live gates are
  agent-surface re-approval + the AD-4 tool-call-interception ADR.
- OPAG-003 (guidance contract) is substantially shipped as OPAE-005
  (`anvil-policy-engine/src/guidance.rs`); OPAG-004's EXCEPT-004/005/006 deps
  are all Merged; OPAG-002's save/CI checkpoints shipped via OPAE-006/#3165
  and #3170; OPAG-007's kill switch + budget shipped. Items need rewriting as
  deltas over shipped surfaces or an agent will re-plan merged work.
- `plans/execution/OPAG.steps.md` is still pure TS-era `nx test` — directly
  contradicts the module's "validation retargeted to Rust" claim.

### AGOV
- Its own 4-item rescope list (pending since 2026-04-26) is exactly right and
  unexecuted. Nearly every Files/Scope path is dead:
  `packages/anvil/runtime/src/gate/`, `apps/anvil-cli/`, `core/` do not
  exist. All validations are `nx test`.
- AGOV-002's pack-install CLI shipped in Rust as OPAE-004
  (`anvil policy install`) — drop/migrate it. AGOV-001/006/007 remain real
  and load-bearing for POLCAP/CPACKS/MDGOV.

### CPACKS
- Last reviewed 2026-07-02 — two days before POLRESET-007 shipped the
  `anvil-baseline` starter pack it still plans as future work.
- CPACKS-001..005 + most of 007 are delivered on main (starter pack, POLVAL
  admission, OPAE-004 install, remediation copy, docs). Three validation
  commands cite `eddacraft-anvil-policy` where no starter-pack code exists.
- Genuine residue: **CPACKS-006** — anvil-baseline fixtures are NOT in
  `ci/eval/suites.json` (only `arch_boundary` is). Small, ready work. Plus a
  docs known-gaps audit and CPACKS-008 as the expansion gate.

## Finding 3 — status lag on delivered modules

- **CPOL and IORISK**: POLRESET-004 delivered all 3/3 items in each; both
  still say In Progress. Flip to Done (IORISK: or add explicit
  scanner-intake items if it should stay open).
- **POLVAL**: 5/5 Done but In Progress on one unchecked criterion ("gate
  preflight can block on validation failure"). PR-B delivered compile-failure
  fail-fast only; full manifest admission runs at install time. Decide:
  accept that as satisfying the criterion → Done, or file POLVAL-006.
- **EVALCI**: header/intro still say "005..008 remain Proposed" — 005/006
  Merged via #3170. EVALCI-008's ATC-003 dep is satisfied; its only remaining
  decision gate is the CI-blocking-posture ADR (POLRESET design gate 3),
  which no item tracks authoring.
- **EXCEPT**: header blockquote still says the store "has no callers …
  unenforced" — actively wrong post-#3140/#3168. Last reviewed 2026-06-08.
- **ATC**: Last reviewed 2026-05-25 predates its own implementation; items
  lack `Merged … via PR #3181` provenance (index row has it).

## Finding 4 — POLCAP ADR number collision

POLCAP reserves **ADR-092** throughout (already once renumbered from 051) —
but `plans/decisions/092-mcp-optional-activation-spine.md` was Accepted
2026-06-26 (ACTMO). POLCAP-001's target path collides with an existing
accepted ADR file. Decisions run through 104; recommend de-numberising the
placeholder ("next free ADR at authoring time"). POLCAP also never got a
POLRESET-010 posture sweep and doesn't cite ADR-098, though its gate-refusal
semantics must reconcile with AD-3/AD-4 at its Planning Council.

## Finding 5 — cross-cutting staleness patterns

1. **AD-2 crate-targeting drift**: ORGHIER-002..005/007, POLLC-002..005/007,
   POLFED-002..007, ACTAX Phase B, OPAG and POLCAP validations, and COMPLY's
   retarget NOTE all scope future work to `crates/anvil-policy` /
   `-p eddacraft-anvil-policy` — the crate AD-2 deletes. Retarget to
   `anvil-policy-engine`, `anvil-cli`, or the future `anvil-exceptions`.
2. **Spent activation gates**: ORGHIER/POLLC/COMPLY/POLFED/AGOV/ACTAX all
   gate on "first policy-value slice ships" — now spent. Each needs a real
   next trigger (demand signal, council, or a concrete prerequisite). Only
   COMPLY's "evidence semantics" (still undefined anywhere) and POLCAP's
   council gate are genuinely unspent prerequisites.
3. **Exception-mechanism overlap**: ORGHIER per-tier "exemptions" and
   POLLC grace periods must be specified against the shipped EXCEPT store
   (ADR-100) and the two-axis ControlDecision/EnforcementMode vocabulary,
   not as second mechanisms / "errors→warnings" flips.
4. **Stale index rows**: `plans/index.aps.md` rows for CPOL (:735), IORISK
   (:736), EVALCI (:740, "005-008 Proposed"), ORGHIER (:725) and POLFED
   (:728, "OPAE bundle primitives").
5. **COMPLY factual error**: its NOTE claims policy-lifecycle is archived —
   POLLC is live Draft in `plans/modules/`.
6. **PATT dangler**: wiring a live `DefenceObserver` (replacing the baseline
   `ConformanceObserver`) is tracked nowhere.

## Recommended remediation batches

1. **EXCEPT-012** (new work item): `anvil-exceptions` extraction per AD-2 +
   dead-helper disposition. Highest-value single item.
2. **Bookkeeping PR** (statuses + headers + index rows): flip CPOL/IORISK
   (and decide POLVAL) to Done; refresh EVALCI/EXCEPT/ATC headers and the
   five stale index rows; fix COMPLY's "POLLC archived" error; add ATC merge
   provenance.
3. **CPACKS re-scope**: record 001..005/007 as satisfied-by #3167/OPAE-004,
   keep the CI eval-suite wiring (ready now) + docs audit + expansion gate.
4. **OPAG + AGOV re-scope pass**: rewrite items as deltas over shipped
   surfaces; execute AGOV's own rescope list; regenerate or delete
   `OPAG.steps.md`.
5. **POLFED re-base**: POLVAL pack primitives + POLLC lifecycle as the real
   prerequisites; fix the phantom `bundle sync` command; cite ADR-100;
   re-title the boundary ADR POLVAL/POLFED.
6. **AD-2 retarget sweep** across ORGHIER/POLLC/COMPLY/POLFED/ACTAX/OPAG/
   POLCAP scopes and validations (can fold into the relevant batches above).
7. **POLCAP**: de-numberise the ADR-092 placeholder; add ADR-098 to Cites
   with an explicit AD-3/AD-4 reconciliation note for its council.
