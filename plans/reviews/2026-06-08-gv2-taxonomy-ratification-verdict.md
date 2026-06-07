# Graph v2 Taxonomy — Ratification Council Verdict

**Date:** 2026-06-08
**Session:** `plan-ec495f8b` (formal taxonomy-ratification council)
**Panel:** architect, kernel-maintainer, adversarial-reviewer, security-analyst
**Artifact under review:** `docs/architecture/graph-v2-foundation-spec.md` (`origin/main`),
against ADR-061/063/064/067/069/031, the reshaped GV2 module, `intercept-as-built.md` §10,
`edda-stack.md`, and `crates/anvil-graph-cache` + `crates/anvil-kernel-types`.
**Gate:** the open GV2 Ready-checklist item — *"Graph taxonomy ratified by a formal
architecture-review council."*

---

## Verdict

**RATIFY-WITH-FIXES — no BLOCKs.** The five-graph taxonomy and ownership
boundaries (D1) ratify outright. The cross-graph identity model (D2), join model
(D3), API shape (D4), seam claims (D5), and privacy/identity-flow (D6) are sound
**to freeze once a set of honesty/scoping corrections are folded in**. None of
the fixes is a redesign — they correct places where the spec presents a
not-yet-true capability as fact, overclaims a seam, or over-generalises a
privacy acceptance. Per the decision rule, the taxonomy ratifies once the
conditions below are folded into the spec.

| Dim | Topic | Lead | Verdict |
|-----|-------|------|---------|
| D1 | Five-graph taxonomy + ownership boundaries | architect | **RATIFY** |
| D2 | Cross-graph identity model | kernel-maintainer | RATIFY-WITH-FIXES |
| D3 | Join model + worked trace | architect + adversarial | RATIFY-WITH-FIXES |
| D4 | Query/registry API shape (freeze-readiness) | architect | RATIFY-WITH-FIXES |
| D5 | Subsystem seam accuracy | adversarial + security | RATIFY-WITH-FIXES |
| D6 | Privacy / identity-flow at taxonomy level | security-analyst | RATIFY-WITH-FIXES |

---

## Ratification conditions (fold into the spec before the gate flips)

### C-1 — The control/session → file join bridge is undesigned (most substantive)

The worked trace's `control/session join → Attribution::Owned(SessionRecord)`
step is **not followable by any key the spec defines**. `SessionRecord.worktree`
is an absolute, canonicalised `PathBuf` and `WorktreeKey` is **crate-private to
`anvil-intercept`** (`rule_cache.rs`); the semantic/dependency graphs key files
by relative `String`. Relativising worktree-root → file identity is undesigned
and untyped, and importing `WorktreeKey` into the graph layer would **invert the
ADR-064 boundary**. _(adversarial D3/D5)_
**Fix:** add a known-gap entry; annotate the trace step as "bridge not yet
designed"; require GV2-013 to define a shared root-relativisation type in
`anvil-kernel-types` (not depend on `anvil-intercept`).

### C-2 — Worked trace + join table conflate symbol identity with file identity

The trace shows `dependents_of(chargeCard)` (symbol-granular), but the shipped
`DependencyGraph` is **file-keyed only** (`dependency.rs:41`). The hop bridges
**file** identity, not symbol identity as the join table (line 137) asserts.
_(architect D3)_
**Fix:** state the dependency hop bridges file identity in the shipped substrate;
mark symbol-granular dependents as a freeze-target needing symbol-level edges
(GV2-011) + stable symbol identity (GV2-002).

### C-3 — Hot-read "bounded reverse impact (depth ≤ hard cap)" is not yet enforced

`impact_closure` enforces a **file-count budget, not a hop-depth cap**
(`certify.rs:149-165`); a star-shaped graph traverses all N files in "one hop".
The spec lists it in the hot-read allowlist as if already true. _(architect D4,
adversarial D3)_
**Fix:** annotate that row as a freeze-target (depth cap is GV2-026; today
budget-capped, not depth-capped), mirroring the substrate-status table's
shipped-vs-pending distinction.

### C-4 — Identity: add the Boundary-trust gap; correct the snapshot-comparability claim

(a) `TrustLevel::Boundary` is silently excluded from the `previously_privileged`
baseline (`incremental.rs:71-73`); if a producer emits `Boundary` before GV2-002,
the export-diff under-fires — a correctness gap absent from the spec's gap list.
(b) Warm-start snapshot comparability does **not** require stable cross-restart
symbol identity — ADR-069 persists the `u64` ids in the DTO + reconciles by
content hash. GV2-002 blocks precise export-diffing and the trust/provenance
joins, **not** snapshot comparability — so GV2-002 need not land before sub-phase
B persistence, only before GV2-014. _(kernel-maintainer D2)_
**Fix:** add the Boundary gap (G-0n); correct the snapshot claim + the GV2-002
sequencing implication.

### C-5 — Seam accuracy: control/session overclaimed; INTD as-built stale wrt ADR-067

(a) The control/session seam is labelled "defined, cite don't redesign", but
`intercept-as-built.md` §10 is a runtime **registry**, not a join contract; the
join key is undesigned (see C-1). Downgrade to "types shipped, join contract
undesigned". (b) The INTD seam cites ADR-067, but `intercept-as-built.md` (last
reviewed 2026-05-07) **predates ADR-067 (2026-06-03)** — the symbol-feed pin
isn't reflected in the grounding doc. _(adversarial D5)_
**Fix:** downgrade the control/session seam wording; note the INTD as-built needs
a refresh before consumers treat that seam as verifiable from the grounding doc.

### C-6 — Privacy line is over-generalised to "every graph"

ADR-069's sealed-DTO + no-leak enforcement and its same-uid residual-risk
acceptance are **proven only for the daemon semantic/dependency snapshot**. The
new control/session graph (`SessionRecord.worktree` is absolute → home-dir/PII)
and plan/provenance graph (Edda = git-committed, shareable — **outside** the
same-uid boundary) do not inherit that acceptance. _(security D6/D5-provenance)_
**Fix:** scope the invariant per-graph — GV2-013 and GV2-014 each need their own
privacy ADR before persistence; the GV2-014 Edda join must be **ref-only** (no
inline memory bodies/secret-shaped literals), stated as a frozen constraint.

---

## Recommendation

Fold C-1…C-6 into the spine spec (all documentation/wording + gap-list edits;
no architecture reversal), then flip the GV2 Ready-checklist taxonomy gate. The
underlying ADR chain is sound; ratification surfaced one genuinely undesigned
bridge (C-1) that must be named-not-frozen, and five honesty/scoping corrections.
Owner (Josh) ratifies.

## Per-persona headlines

- **architect:** five-graph taxonomy + two-tier API shape sound to freeze, but
  the worked trace and hot-read allowlist present not-yet-true capabilities as
  fact — fix the wording before freezing.
- **kernel-maintainer:** identity gaps correctly mapped, but the spec overstates
  the snapshot dependency on GV2-002 and omits the Boundary-trust exclusion.
- **adversarial-reviewer:** ADR chain sound, but the control/session join bridge
  is undesigned and the INTD seam cites a stale as-built — freeze with those gaps
  named, not as-is.
- **security-analyst:** the daemon-snapshot privacy line is sound to freeze, but
  "binding on every graph" over-generalises a same-uid acceptance onto graphs
  that cross different boundaries — scope it per-graph.
