<!--
APS Module: v0.6.0-beta Release Candidates
==========================================
Capture-as-you-find module for items targeted at the next release window
after v0.5.0-beta (which shipped 2026-05-01). Holds two kinds of entries:

  1. Deferrals from the v0.5.0-beta council / post-tag findings that
     were judged non-blocking for the v0.5.0 cut but should ride the
     next release rather than rot as silent debt.

  2. Forward-looking nominations — work items from other modules that
     are being earmarked for the next release window because they're
     small, low-risk, high-leverage, or obviously slot-fitting.

This is a *capture* surface, not a *commitment* surface. Anything in
here is a candidate, not a guarantee. Sequencing is owed against
plans/next-steps.md (the strategic frame) at cherry-pick time.

Naming: file is `v060` because items here ride the *next* release after
v0.5.0; if the next release tags as v0.5.1 (patch) instead of v0.6.0
(minor), rename the file then — the prefix V060F stays stable on items
already filed, but the module title and file should reflect the actual
target tag once chosen.

See: plans/aps-rules.md
-->

# v0.6.0-beta Release Candidates

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| V060F | —     | In Progress | 0/1      |

**Last reviewed:** 2026-05-01
**Predecessor:** [v050-release-followups](./v050-release-followups.aps.md)
**Sequencing context:** [plans/next-steps.md](../next-steps.md)

## Purpose

Hold the running list of items targeted at the next release after
v0.5.0-beta. The previous module (`v050-release-followups`) is now
historical — its target shipped — so new follow-ups and candidates land
here.

Two intake paths:

- **Deferrals** — anything the v0.5.0 council / external review flagged
  but that was consciously deferred so the tag could ship; anything the
  release run itself surfaced (workflow failures, post-deploy gaps,
  publisher bugs) that was patched manually but needs a permanent fix.
- **Nominations** — small or high-leverage items from other modules
  that the team wants to slot into the next release. These are
  pointers, not duplicates: the canonical tracking stays in the source
  module, V060F just records the nomination + rationale.

Each entry should carry enough context that a future reader can decide
whether to keep, drop, or reschedule it without rerunning the
discovery.

## In Scope

- Deferrals from v0.5.0-beta release prep (council rounds, external
  reviews, post-tag workflow / deploy / publisher findings)
- Hardening items born from v0.5.0 production runtime that should ride
  the next tag rather than wait
- Forward nominations from active modules where a specific work item
  is earmarked for the next release (recorded as a pointer, not a
  re-spec)

## Out of Scope

- Items already tracked in `v050-release-followups` that didn't ride
  v0.5.0 — those need a status reconciliation in that module first
  (mark Complete if they shipped, roll forward to V060F only if they
  remain open and the rationale still applies)
- Net-new feature work — features belong in their own module; V060F
  only nominates work items, not feature concepts
- Items gated on un-staffed dependencies (e.g. blocked on OPAE) —
  parking them here just creates noise

## Intake Conventions

- **Deferral entry:**
  - **Surface:** file/line or commit/PR
  - **Flagged by:** council reviewer, external review, or release run
  - **Intent:** what's broken / what hardening is needed
  - **Expected outcome:** the resolution shape
  - **Confidence:** high / medium / low
  - **Status:** Open by default; flip to Complete when shipped
- **Nomination entry:**
  - **Source module + work item:** e.g. `RCLI2 / RCLI2-009`
  - **Why earmark:** one line on why this fits the next-release window
    (size / risk / leverage)
  - **Status:** Nominated until the source item flips Complete

---

## Work items

### V060F-001: admin command parity for `anvil admin` (nomination)

- **Source:** [RCLI2-009](./rust-cli-tier2.aps.md#rcli2-009-admin-command-parity-listshowrevokeauditsend-migrationemail-update)
- **Status:** Nominated
- **Why earmark:** Operator-experience papercut — `anvil admin list`
  fails today with "unrecognized subcommand" because RCLI-016 only
  ported `approve` and `invite`. The other six commands still require
  the separate Node binary `anvil-admin` (`apps/admin-cli/`), which is
  not on PATH for normal operator setup. Pure 1:1 parity port over a
  well-tested API surface, plus one new CLI surface for the existing
  `POST /admin/user/email-update` endpoint. High confidence, medium
  priority, no policy/OPAE dependency.
- **Cuts:** unblocks retiring `apps/admin-cli/` for a single operator
  surface; closes the `anvil admin list` ergonomic gap that prompted
  this nomination.
- **Filed:** 2026-05-01

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Module accumulates aspirational nominations and stops being a real release-window list | High | Medium | Re-run a triage pass at cherry-pick time; demote nominations that no longer fit |
| `v050-release-followups` open items not reconciled before V060F starts collecting | Medium | Low | Add a one-line status for each open V050F item in the next reconciliation pass; only roll forward to V060F if still applicable |
| Release version target shifts (v0.5.1 patch vs v0.6.0 minor) | Medium | Low | File/title rename is cheap; existing V060F prefix stays stable on already-filed items |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| Deferrals | 0 | — |
| Nominations | 1 | In Progress |
| **Total** | **1** | — |
