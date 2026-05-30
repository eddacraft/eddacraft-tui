# ADR-060: Per-project state resolution under `ANVIL_HOME`

## Status

Proposed

## Date

2026-05-31

## Context

[DISTRIB-006](../modules/distribution-and-update.aps.md) (promoted from GitHub
issue [#1726](https://github.com/eddacraft/anvil-001/issues/1726)) adds an
`ANVIL_HOME` env var + `--anvil-home` flag that re-roots Anvil's
**install-owned** state — user state (today under `dirs::home_dir()`), the daemon
socket (today under the runtime dir), and kernel cache/logs — so a pre-release
candidate binary can run side-by-side with the production install and be tested
**without cutting a release**. The daemon single-instance rule
([ADR-036](036-daemon-scope-discovery-and-boundaries.md): one daemon per
`(uid, os)`, PID-file exclusive create) means a distinct socket prefix per
`ANVIL_HOME` is exactly what lets a candidate daemon and the prod daemon coexist.

Re-rooting *install-owned* state is uncontentious. The open question DISTRIB-006
is gated on is narrower and genuinely two-sided: **what happens to per-project
state** when a candidate runs under an `ANVIL_HOME` override.

Anvil keeps two distinct categories of state, and only the first is install-owned:

- **Install/user-owned** (re-rooted by `ANVIL_HOME` without debate): user config
  under the home dir, the daemon socket/PID under the runtime dir, kernel
  cache/logs.
- **Per-project, lives *with the repo*** (the contested part): resolved relative
  to the project root, not the home dir — `<root>/.anvil/baseline.json`
  (`activation/baseline.rs`), `<root>/.anvil/cache/` (`warmup_cache.rs`,
  `detect_agents.rs`), the witness chain under `<root>/.anvil/witness/`, and the
  project identity anchor `<root>/anvil/project-id`
  (`activation/orchestrator`). These are checked in or are durable
  project-local artefacts that the **production** install also reads and writes.

The collision the issue reported: when a candidate `cd`s into a real project, it
touches that project's `.anvil/` — and prod then sees whatever the candidate
wrote. A candidate baseline refresh, a witness line written by an unreleased
build, or a cache shape change can leak into the project state that prod depends
on. The decision is whether `ANVIL_HOME` should *also* re-root per-project
discovery, and that is a real trade between two failure modes:

- Keep project state shared → a candidate can corrupt/skew real project state.
- Re-root project state → the witness chain a candidate writes does **not**
  persist for prod, so "test the candidate on my real repo" loses the very
  durability (witness continuity, baseline) that makes the test meaningful.

This ADR exists to settle that one call so DISTRIB-006 can move Proposed → Ready.
It does **not** re-decide install-owned re-rooting (uncontested) and does **not**
cover cross-version chain *format* compatibility (a candidate writing a chain a
different anvil version reads) — that stays an `anvil migrate` concern
(DISTRIB-005), explicitly out of scope here.

## Decision

When `ANVIL_HOME` (or `--anvil-home`, which takes precedence) is set:

1. **Install/user-owned state is re-rooted** under the prefix — user state at
   `<ANVIL_HOME>/user/`, daemon socket/PID at `<ANVIL_HOME>/daemon.sock` (or the
   platform equivalent), kernel cache/logs at `<ANVIL_HOME>/cache/`. Production's
   `~/.anvil/`, runtime socket, and logs are never touched. This is the
   uncontested part and applies regardless of the option below.

2. **Per-project state resolution — adopt Option (a): keep it unchanged, plus a
   guard.** Per-project `<root>/.anvil/` (baseline, cache, witness) and
   `<root>/anvil/project-id` continue to resolve relative to the project root,
   exactly as prod does — `ANVIL_HOME` does **not** re-root project discovery.
   This preserves witness-chain continuity and baseline durability so a
   candidate test on a real repo is a *real* test. To contain the corruption
   risk that Option (a) otherwise carries, the candidate's project-touching
   writes are guarded:

   - When running under a non-default `ANVIL_HOME`, **mutating** per-project
     operations that change durable project state a different binary will read —
     baseline refresh/write, witness append, cutoff pinning — require an explicit
     opt-in (`--touch-project-state`, or env equivalent) and, absent it, run in a
     **read-only / dry-run** posture: the candidate scans, validates, renders,
     and exercises the daemon path against the real repo, but does not persist
     project-state mutations. Read paths (status, check, audit, watch render) are
     unrestricted.
   - `anvil status --json` reports the resolved install root in a new
     `installRoot` field **and** whether project-state writes are gated, so an
     operator can always see which install they are talking to and whether it can
     mutate the repo.

3. **Unsetting `ANVIL_HOME` returns to platform-default behaviour
   byte-for-byte** — no new fields, no path changes, no guard — for the 99% of
   users who never set it.

The combination is "Option (a) + write-guard": projects stay durable and shared
by default (so the test is meaningful), but an unreleased binary cannot silently
mutate a real project's baseline or witness chain unless the operator explicitly
allows it.

## Rationale

Option (b) (re-root project discovery under `<ANVIL_HOME>/projects/`) cleanly
prevents cross-pollination, but it defeats the stated purpose: the highest-value
test in #1726 is "run the candidate against my *actual* work for a week" (the
Boring-Week side-by-side use case). If the candidate writes its witness chain and
baseline into a sandbox the prod install never reads, you are no longer testing
the candidate on the real project — you are testing it on an empty shadow of it,
which a `/tmp` scratch dir already gives you. Option (b) optimises for isolation
at the cost of the only thing that made the feature worth filing.

Option (a) preserves the meaningful test but, unguarded, re-introduces exactly
the corruption collision #1726 describes. The write-guard is the minimum
addition that keeps Option (a)'s value while removing its sharp edge: reads and
the daemon path (the parts you actually want to exercise) are unrestricted;
durable project mutations from an unreleased binary are opt-in. This matches
Anvil's existing "warnings over blocks / explicit acknowledgement for dangerous
writes" posture (cf. the `--accept-suspicious` baseline-refresh guard already in
`anvil-baseline`).

This is a recommendation, not a settled fact — the ADR is **Proposed**. The
genuine counter-argument is that the write-guard adds a mode and a flag to every
project-mutating path, and a reviewer may prefer the simplicity of Option (b)'s
hard isolation and accept the weaker test. That trade is the thing to ratify in
review.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **(a) Keep project-state resolution unchanged + write-guard** (chosen/recommended) | Candidate tests run against the *real* repo (witness continuity, baseline durability preserved) — the actual #1726 goal; reads + daemon path unrestricted; durable mutations from an unreleased binary are opt-in; mirrors existing `--accept-suspicious` posture | Adds a mode + `--touch-project-state` flag to every project-mutating path; "shared by default" still trusts the guard to hold; an operator who opts in can still skew real state |
| (b) Re-root project discovery under `<ANVIL_HOME>/projects/` | Total isolation — a candidate can never touch real project state; conceptually simple (one prefix re-roots everything) | Defeats the purpose: the candidate's witness/baseline don't persist for prod, so "test on my real repo" degrades to a shadow copy — barely better than the `/tmp` workaround the feature replaces |
| (a) unguarded (re-root install state only, project state fully shared) | Simplest; smallest diff | Re-introduces the exact corruption collision #1726 calls out — an unreleased binary silently writes a real project's baseline/witness |
| Per-project opt-in via `.anvil` config instead of a global rule | Project owner controls exposure | Per-repo config sprawl; the candidate tester (not the project) is the one who needs the safety, and they set `ANVIL_HOME`, so the control belongs on that axis |

## Consequences

- **Positive:** DISTRIB-006 moves Proposed → Ready with the contested call
  settled. Side-by-side candidate testing works against real repos with witness
  continuity intact, while an unreleased binary cannot silently corrupt project
  state. The `installRoot` field gives operators a clear "which install am I
  talking to" signal. Default behaviour is unchanged for users who never set
  `ANVIL_HOME`.
- **Negative:** Every project-state-mutating path gains an `ANVIL_HOME`-aware
  guard branch and the `--touch-project-state` opt-in — more surface to
  implement and test than Option (b)'s blanket re-root. "Shared by default"
  means the guard, not the filesystem, is what prevents cross-pollination.
- **Risks:** A mutating path that forgets the guard becomes a silent
  corruption vector; the read-only/dry-run posture could be incomplete (some
  command mutates project state via a path not classified as "mutating"); an
  operator who reflexively passes `--touch-project-state` re-opens the original
  risk.
- **Mitigations:** Centralise the guard in the same install-root resolver that
  re-roots install state (one chokepoint, not per-call-site); DISTRIB-006's test
  matrix asserts that under a non-default `ANVIL_HOME` a baseline refresh /
  witness append is refused without the opt-in and a real project's `.anvil/` is
  untouched; the `installRoot` + write-gated fields in `status --json` make the
  posture observable in tests and by operators.

## References

- Related ADRs: [ADR-036](036-daemon-scope-discovery-and-boundaries.md) (daemon
  one-instance / socket derivation — the constraint a per-`ANVIL_HOME` socket
  prefix works with), [ADR-044](044-mcp-entry-activation-owned.md) +
  [ADR-045](045-update-signing-scheme.md) (sibling distribution ADRs)
- APS modules: DISTRIB-006 (the work item this gates; Proposed → Ready on
  acceptance), DISTRIB-005 (`anvil migrate schema` — owns cross-version config /
  chain *format* compatibility, out of scope here)
- External: GitHub issue
  [#1726](https://github.com/eddacraft/anvil-001/issues/1726)
