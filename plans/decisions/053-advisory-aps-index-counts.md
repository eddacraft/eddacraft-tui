# ADR-053: Per-module APS index counts are advisory-derived, not PR-maintained

## Status

Accepted (effective on CIB-025 implementation — see Decision §5)

## Date

2026-05-27

## Context

`plans/index.aps.md` and each module header carry a per-module `N/M` (done/total)
count. CIB-022 made that count *derivable* from the module's per-item `Status:`
lines and added `aps:index:check` (a closeout-enforcement check in the ADR-042
family) to block on drift — but the count stayed a hand-edited cell: every PR
that completed a work item rewrote its module's `N/M`.

Because two PRs completing *different* items in the *same* module each rewrite
that one aggregate to a *different* value, they collide on it at merge — a
textual conflict that derivation does not prevent (it only makes the post-merge
fix deterministic). Observed live 2026-05-26: four CIB PRs (CIB-017, -018, -019,
-024) plus a triage all collided on the single `| CIB | … | N/M |` token, forcing
four serialised rebase-merges. `merge=union` (CIB-021) cannot help — union keeps
both lines, yielding two count rows — and a recompute-on-conflict git merge
driver only runs on local merges, not GitHub's server-side merge (the same limit
that defeated union for mergeability).

The fix requires PRs to stop writing the count. The open question (CIB-025
Gate 1) was how the stored count then stays fresh. A planning council
(2026-05-27, direction-validate) chose advisory freshness over a strict
per-merge regeneration bot.

## Decision

Per-module APS counts are an **advisory, eventually-consistent derived value**,
not a PR-maintained one:

1. **Feature PRs never edit the `N/M` count.** They flip only their own item's
   `Status:` line — distinct lines that never collide across concurrent
   same-module PRs.
2. **Truth is the per-item `Status:` lines.** The stored `N/M` is an at-a-glance
   convenience derived from them by `scripts/aps/index-counts.mjs`.
3. **`aps:index:check` freshness becomes advisory** (warn, exit 0) rather than
   blocking — a scoped exception to ADR-042's "closeout checks block by design",
   because once PRs stop maintaining the count a freshness mismatch is the
   *expected* state between reconciles, not an error to gate on.
4. **A single-writer reconcile refreshes the stored count** — the existing
   manual/ad-hoc `chore(plans): reconcile APS index` practice (run on demand via
   `npm run aps:index`; there is **no automated workflow today**, and adding one
   would be the option-1 escalation below). Being single-writer (one reconcile at
   a time, never concurrent with itself) it never contends, and because feature
   PRs don't touch the count a reconcile never conflicts with feature work.
5. **Effectivity.** This ADR records the decided end-state; it takes effect when
   CIB-025 downgrades `aps:index:check` to advisory. **Until CIB-025 lands the
   gate still blocks**, so the existing `.claude/rules/aps-index.md` guidance
   (update the module done/total count on completion) **remains in force** —
   contributors keep updating the count until then. CIB-025's implementation must
   update `.claude/rules/aps-index.md` in lockstep with the gate downgrade so the
   rule and the gate never contradict.

Implementation is tracked by CIB-025 (advisory shape).

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Advisory freshness + periodic reconcile (chosen)** | Zero same-module contention; no new protected-branch automation; aligns with ADR-002 (warnings over blocks); the `Status:` lines stay authoritative | Stored `N/M` can lag reality between reconciles; loosens the strict freshness CIB-022 / ADR-042 gave the check |
| **Post-merge regeneration bot (option 1 — the escalation)** | Strict freshness *and* zero contention | A bot committing to protected `main` on every merge: ruleset carve-out, merge races, history noise, a new trusted automation + its own ADR — disproportionate to a convenience count |
| **Don't store the count / compute-on-read** | Always correct; nothing to drift | Removes the at-a-glance `N/M` from the human-readable index, or reintroduces a generated view with its own "who commits it" problem |

Trade-off accepted: a count that may be briefly stale, in exchange for removing a
recurring merge-contention class with no new automation. The per-item `Status:`
lines remain authoritative, so a stale aggregate is never *wrong* about any
individual item — only about the rollup, until the next reconcile.

## Consequences

- **Positive:** Concurrent same-module PRs stop colliding on the count token; the
  serialised-rebase tax observed on 2026-05-26 disappears, with no
  protected-branch automation added.
- **Negative:** `plans/index.aps.md` and module headers may show a stale `N/M`
  between reconciles. Readers must treat the count as advisory; the `Status:`
  lines are truth.
- **Revisit trigger (→ escalate to option 1):** if advisory staleness ever causes
  a *wrong release-prep decision* or repeated reviewer confusion, escalate to the
  **post-merge regeneration bot** — a per-merge CI job that regenerates and
  commits the counts to `main`. That escalation would be its own ADR (superseding
  this advisory clause) and would need a ruleset carve-out for the bot's commits.
  Until that trigger fires, advisory is the standing decision.
- **Interaction with ADR-042:** ADR-042 keeps `adr:check` and `aps:drift`
  blocking; ADR-053 carves only the *count-freshness* aspect of `aps:index:check`
  out to advisory, because its maintenance model changed (no longer
  PR-maintained). `aps:index:check` may still block on *structural* problems
  (e.g. a malformed row) if CIB-025's generator adds such checks — only the
  freshness mismatch becomes advisory.

## References

- Related ADRs: ADR-002 (warnings over blocks), ADR-042 (closeout-enforcement exit codes — scoped exception here)
- APS modules: CIB-025 (advisory implementation + remaining gates), CIB-022 (count derivation this evolves), CIB-021 (`merge=union` sibling for the CI log)
- Planning council: `plans/brainstorms/2026-05-27-cib-025-planning-council.md`
- Code: `scripts/aps/index-counts.mjs`, `.github/workflows/ci.yml` (Docs Lint `aps:index:check`)
