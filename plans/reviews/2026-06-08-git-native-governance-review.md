# Git-Native Governance Pack — Architecture Review

**Date:** 2026-06-08
**Reviewer:** Architecture review (single reviewer, code-grounded).
**Scope:** the Git-Native Governance brainstorm pack and its formalisation on
`origin/main` — `plans/brainstorms/git-native-governance/`
(README/context/solution/architecture/roadmap/agent-handoff), ADR-072 (Git
substrate), ADR-073 (durable/local state boundary), ADR-074 (capsule v0 format),
and modules GITGOV (`plans/modules/git-native-governance.aps.md`) + EXCEPT
(`plans/modules/git-native-exceptions.aps.md`).
**Method:** read the full pack; spot-checked the load-bearing claims against
actual code on `origin/main`.

---

## 1. Verdict

**Strong — and notably, the formalisation already performed the
schema-reconciliation pass that the GV2 module lacked.** The single biggest
hazard in a brainstorm→plan pipeline on this repo is freezing evidence schemas
against fictional shapes. ADR-074 catches its *own* brainstorm's fiction
(`solution.md`'s `WitnessExtract` with `L0/L2/L3/L4` sub-objects and
`agent.task_id` does not exist) and mandates embedding the **real**
`anvil-witness::WitnessLine` verbatim so verification reuses `verify_chain_dag`
rather than a parallel parser. That discipline is the right one, applied
pre-emptively.

The substrate decision (ADR-072) ratifies a pattern already in production
(witness/baseline/drift are already tracked, content-addressed, in-tree) rather
than inventing one. The wedge (Review Capsules) is well-chosen: a demonstrable
product loop on top of already-shipped evidence, needing no cloud, no Graph v2,
no new infrastructure.

**Recommendation:** proceed. Take ADR-072/073/074 to council; hold the capsule
wedge hard against the roadmap's later phases; resolve the open seams in §4
before freezing `anvil.capsule.v1`.

---

## 2. Verified against code (not taken from the docs)

| Claim | Result |
| --- | --- |
| ADR-073's "exceptions live in gitignored `.anvil/`" bug + its reconciliation | **Confirmed + already shipping.** `crates/anvil-policy/src/exceptions.rs` now reads `anvil/exceptions/store.json` with legacy read-fallback to `.anvil/exceptions.json` + one-time migration + tests. EXCEPT-001/002 landed (commit `137f3a147`); the index correctly marks EXCEPT "In Progress". |
| ADR-074's refutation of the brainstorm's `WitnessExtract` | **Accurate.** Real `WitnessLine` fields: `seq, scope, kind, prev_line_hash, project_uuid, commit_sha, parent_commits, prev_line_hashes, agent_tag, rules_sha, cutoff_commit, ts, validation_at` (`crates/anvil-witness/src/line.rs`). The brainstorm's `L0/L2/L3/L4` + `agent.task_id` shape is fiction; the ADR mandates the real shape. |
| Capsule code exists | **No** — only ADR-074 + public docs. Consistent with all-Proposed status. (See finding F-3.) |
| Governance modules registered in the APS index | **Yes** — GITGOV (Proposed), EXCEPT (In Progress), plus the wider family AGOV/DOCGOV/MDGOV/ILGOV/APGOV. |

---

## 3. Strengths

- **Wedge choice is correct.** Review Capsules sit on already-shipped evidence
  (witness ADR-037, baseline ADR-039, drift ADR-052). The MVP —
  `capsule create|verify|explain` over a file-first directory + digest
  manifest — is the smallest thing that proves the thesis and is well-bounded
  (ADR-074 "Deferred" list is explicit).
- **Substrate decision ratifies, not invents (ADR-072).** "Git proves; it does
  not serve hot reads" keeps SQLite/cache as projections, consistent with the
  three-pipe rule (ADR-035) and the GV2 split.
- **ADR-073 fixes a live governance bug**, not a theoretical one — and it is
  already half-shipped. Exceptions in gitignored `.anvil/` didn't travel with
  the repo or show in PR review, defeating the point of first-class exceptions.
- **Honest verdicts.** `degraded != pass`, missing evidence never passes,
  verdict is advisory (ADR-002) not a new blocking gate. Consistent with the
  tooling-honesty doctrine.
- **Privacy line generalises ADR-069** (the GV2-snapshot privacy line) to all
  durable evidence: redacted summaries/digests/pointers, never raw secrets.

---

## 4. Findings — what to push on

### F-1 (Major): Governance-module sprawl needs a stating-doc

There are now ~7 modules using "governance" — GITGOV, EXCEPT, AGOV
(agent-governance-patterns), DOCGOV (documentation), MDGOV (markdown), ILGOV
(intent-ledger), APGOV (API). The word means at least three different things
across them (evidence substrate vs policy vs docs). This is the same taxonomy
lesson as GV2: a family of related modules drifts without a one-page map of how
they join — which is the umbrella, which are layers, which are consumers.
**Fix:** the governance-family map (companion deliverable
`governance-module-family-map.md`).

### F-2 (Major): Scope-creep gravity

The pack is self-aware (its risk table names it), but `solution.md` /
`architecture.md` sketch a large future surface — policy packs, EU AI Act packs,
supplier bundles, release seals, memory→policy loop, cloud amplifier. The MVP is
disciplined; the danger is the roadmap's ~10 future phases quietly becoming the
plan. **Fix:** hold to the capsule wedge until the create/verify/explain loop is
proven end-to-end; treat every later phase as separately decision-gated.

### F-3 (Major): Public docs may be ahead of code

`docs/public/kindling/concepts/capsules.md` and
`docs/public/kindling/quickstart/create-capsule.md` exist on `origin/main` while
**no capsule crate or command does**. If those present `anvil capsule` as
available, that is precisely the overclaim the pack itself argues against.
**Fix:** confirm those pages are marked forthcoming, or describe a distinct
Kindling concept — and if not, gate them behind the GITGOV implementation.

### F-4 (Major): exception-use ↔ witness-schema seam

`solution.md` §8.4 records `exceptions_used` by "extend[ing] the witness line or
outer envelope", but ADR-074 mandates **verbatim** `WitnessLine` reuse so
`verify_chain_dag` still applies. Those pull against each other. Where
exception-use is recorded — without forking the witness schema and breaking the
verbatim-reuse property — must be nailed before GITGOV-009 (verification engine).

### F-5 (Medium): detached verification is the load-bearing open question

GITGOV-009 / ADR-074 defer whether `verify` needs the original repository or can
work from `commits.json` metadata alone. This is the single open decision that
most affects the product promise ("an auditor/supplier verifies later without
access"). Correctly flagged as open and gated before `v1` freeze — the action is
simply to ensure it is resolved *at* the freeze, not after.

### F-6 (Minor): "v0" milestone vs `anvil.capsule.v1` schema string

The format is called "capsule v0" but the schema id is `anvil.capsule.v1`.
Harmless (v0 = milestone, v1 = schema version) but worth one clarifying line so
implementers don't trip.

---

## 5. Relationship to Graph v2 (complementary, not conflicting)

Checked specifically, since GV2 is the adjacent active stream:

- ADR-072 preserves the split: "the Kindling/Ember/Graph-V2 split is unchanged;
  Git is the canonical evidence and transport layer; local SQLite/cache stores
  are projections." **Capsule v0 deliberately avoids a Graph v2 dependency**
  (`solution.md` §11.6). The two streams are decoupled by design.
- ADR-072 §3 privacy line **generalises ADR-069's GV2-snapshot privacy line** —
  same vocabulary, no conflict.
- **Convergence to coordinate:** the pack's Sealed Edda provenance (EDDA-SEAL) is
  the *same open seam* flagged as G-02 in the GV2 foundation spec
  (`docs/architecture/graph-v2-foundation-spec.md`) — the Rust↔TS provenance
  boundary. GV2-014 (plan/provenance graph) and EDDA-SEAL touch the same durable
  artefacts under `anvil/edda/` and should share **one** provenance contract,
  not two independently-designed ones. This is the most important cross-stream
  action item.

---

## 6. Operational notes

- **No pull was performed.** The shared main checkout is dirty with sibling work
  (staged `.claude/skills/**` + `.opencode/skills/**` deletions, modified
  `ci-nightly.yml`, untracked `.notes/` + session files). This review and its
  companions were authored in a dedicated worktree (`docs/gitgov-review`) off
  `origin/main`.
- **Two copies of the pack exist.** The git-managed pack is
  `plans/brainstorms/git-native-governance/`; an **untracked**
  `docs/strategy/git-native-governance-pack/` also sits in the shared checkout (a
  parallel copy). Two-sources-of-truth risk — the untracked copy should be
  reconciled against, or removed in favour of, the committed pack.

---

## 7. Related docs

- Companion map: [`governance-module-family-map.md`](./governance-module-family-map.md)
- Brainstorm: [`../brainstorms/git-native-governance/`](../brainstorms/git-native-governance/)
- ADRs: [072](../decisions/072-git-native-governance-substrate.md),
  [073](../decisions/073-durable-vs-local-anvil-state.md),
  [074](../decisions/074-review-capsule-v0-format.md)
- Modules: [GITGOV](../modules/git-native-governance.aps.md),
  [EXCEPT](../modules/git-native-exceptions.aps.md)
- Adjacent stream: the GV2 foundation spine spec
  `docs/architecture/graph-v2-foundation-spec.md` (PR #2350 — not on
  `origin/main` until that merges, so referenced as a path rather than a link)
