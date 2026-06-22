# ADR-090: Daemon-originated, worktree-scoped health envelopes

## Status

Accepted

## Date

2026-06-23

## Context

The INTD-015 telemetry fan-out (`crates/anvil-intercept/src/fanout.rs`) is the
cross-session privacy guard for diagnostic envelopes. Its load-bearing rule:
an envelope whose `correlation.originating_session_id` is **absent** is denied
to **every** subscriber — there is no proven originator, so cross-session
filtering cannot be performed safely (the diagnostic-envelope coordination spec,
"Subscribers MUST treat unknown session ids as not authorised").

That rule is correct for **session-scoped** content (a save-time gate result, a
mid-edit notification): such an envelope without a session id is a producer bug,
and dropping it is the safe response.

It is **wrong** for **daemon-originated health** events. The snapshot
persist-failure notification (ADR-035 / ADR-069 §10; tracked as CIB-092h →
CIB-098) is produced by the daemon itself — the shutdown flush and the
background-scan executor — which carry **no originating session**
(`TelemetryCorrelation::default()`). So the persist-failure envelope is
universally denied and never reaches an operator. An operator who set
`ANVIL_PERSIST_GRAPH=1` and hits a full/EROFS state dir gets only a
`tracing::warn!` line plus the CIB-092b cumulative metrics — no user-visible
degradation notification, which is exactly what ADR-069 §10 asked for.

We want that operator to receive the degradation signal **without** weakening
the session-deny that protects session-scoped content.

## Decision

Introduce a distinct, **explicitly-marked** envelope class: a
**daemon-originated, worktree-scoped health envelope**. Such an envelope:

1. carries **no session-scoped content** — only daemon-level health and the
   `correlation.worktree` of the affected workspace (a path the operator of that
   workspace already owns and knows);
2. carries `correlation.worktree` (the scoping key, in place of a session id);
3. is **explicitly flagged** by a dedicated marker on the envelope (a boolean —
   `serde(default)` for wire back-compat), set only by sanctioned daemon-health
   producers. The flag is never inferred from a category/kind heuristic, so a
   session-scoped envelope that merely *lost* its session id can never be
   mistaken for one.

The fan-out's `decide`, when `originating_session_id` is absent:

- if the envelope is **not** flagged daemon-health → **Deny** (the INTD-015
  invariant is unchanged for everything else);
- if it **is** flagged daemon-health **and** carries a worktree → authorize by
  **worktree**: deliver only to subscribers that own a session bound to that
  worktree (a new `OwnershipResolver::is_authorised_for_worktree`, backed by the
  registry's `sessions_for_worktree` × `lookup_subscriber_binding`); **Deny** to
  every other subscriber (including subscribers of other worktrees).

A daemon-health envelope with no worktree is denied (nothing to scope to).

## Consequences

- The persist-failure notification reaches the **owning** operator (the
  subscriber bound to that worktree) and **no one else** — cross-session and
  cross-worktree privacy is preserved: a subscriber only ever sees its own
  worktree's daemon health.
- The no-session **Deny** remains the default for all session-scoped envelopes;
  the new path is opt-in by an explicit producer flag and constrained to
  daemon-level health content.
- The worktree path is delivered in cleartext only to that worktree's own
  subscriber (who already knows their root); it is never exposed cross-worktree.
  No new redaction surface is introduced.
- Future daemon-health producers (e.g. other ADR-035 operational notifications)
  reuse the same marker + worktree scoping rather than inventing a new lane.
- The MLP2-071 D6 spoof-fence (which denies a `degraded:spoofed-attribution`
  origin to non-owning subscribers on the *session* path) is **intentionally not
  applied** to the worktree path: a daemon-health envelope carries no
  session-attributed content and is delivered only to the worktree's own binding
  owner, so there is nothing a spoofed peer could exfiltrate. The omission is by
  design, not a gap.
- Authorization resolves the subscriber binding against the registry's **stored
  canonical worktree key without a fresh `fs::canonicalize`** — the envelope
  worktree is already canonical, and re-statting the path would suppress delivery
  in exactly the degraded states (full/EROFS/deleted/unmounted) the notification
  is for. It remains an exact canonical-key match, so no mis-delivery is possible.

## Alternatives considered

- **Operator status/metrics pull surface only** (extend CIB-092b counters +
  `anvil status`, no fan-out change): fully within the existing invariant, but
  it is a *pull* signal — it does not deliver the *push* notification ADR-069
  §10 specified. Kept as the complementary readout, not the delivery mechanism.
- **Broadcast daemon-health to all subscribers**: rejected — it would leak one
  worktree's root path to subscribers of other worktrees. Worktree scoping is
  the minimal authorized set.
- **ADR-first then build**: this ADR *is* the decision; the implementation lands
  with it (CIB-098).
