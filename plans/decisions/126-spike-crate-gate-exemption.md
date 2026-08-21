# ADR-126: Spike crate exempt from strict clippy gates

## Status

Proposed

## Date

2026-08-21

## Context

`crates/spike` (`anvil-spike`) exists for throwaway validation spikes: proving a
library or approach end-to-end before committing to it. It is `publish = false`,
`dist = false`, excluded from nx test jobs (`ci.yml`, `ci-nightly.yml`), and
nothing in the workspace depends on it. Despite that, the crate is held to the
same bar as production crates: `rust.yml` runs
`cargo clippy --workspace --all-targets -- -D warnings` on Linux and the
Windows cross-target, which promotes every warn-level lint (including
`clippy::pedantic`, set to warn workspace-wide) to a CI failure inside spike
code. Landing the rataflow spike (PR #4074) required polishing exploratory code
to production lint standards, which inverts the point of a spike: the crate's
cost of entry should be "compiles and runs", not "lint-perfect on two
platforms".

The existing gate comment in `rust.yml` (CIB-193/CIB-204) explains why
`--exclude` cannot be used for crates that other workspace members depend on:
`cargo clippy` sets `RUSTC_WORKSPACE_WRAPPER`, which lints every workspace
member built in the dependency graph. That constraint does not apply to
`anvil-spike` — it is a leaf; no selected crate pulls it in.

## Decision

Add `--exclude anvil-spike` to both strict clippy invocations in `rust.yml`
(the Linux `clippy` job and the `clippy-windows` cross-target job).

Everything else stays: `anvil-spike` remains a workspace member, stays in
`cargo check/test/build --workspace` (so spikes cannot silently stop
compiling), keeps the shared lockfile, Hakari unification, and
`cargo fmt --check`, and keeps `[lints] workspace = true` as local-development
guidance that is no longer CI-enforced.

## Rationale

The requirement a spike must meet drops to exactly what a spike is for:
compile, run, demonstrate. Compile coverage in check/test/build prevents
bit-rot without imposing lint polish. Excluding the crate from clippy also
stops the clippy jobs compiling spike-only dependencies (e.g. rataflow),
a small CI-time saving.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Exclude from clippy jobs only (chosen) | Two-line change; keeps compile coverage, shared lock, Hakari; honest scope | Spike code no longer linted in CI at all |
| Full workspace exclusion (`[workspace] exclude`) | True separation — own lock, own lints, no gates | Lockfile drift, no shared build cache, drops out of nx graph, nothing stops rot; heavier than the problem |
| Per-crate `[lints]` overrides only | No CI change | `-D warnings` on the clippy command line still promotes any remaining warn-level lint; escaping fully means blanket `allow`s, losing the useful lints locally too |

## Consequences

- **Positive:** spikes land at spike cost; clippy jobs get marginally faster;
  the leaf-crate carve-out is documented next to the CIB-204 comment that
  explains when `--exclude` is invalid.
- **Negative:** lint regressions in `crates/spike` surface only when a
  developer runs clippy locally (the workspace `[lints]` table still applies
  there).
- **Risks:** a future crate adding a dependency on `anvil-spike` would silently
  re-include it in linting (harmless) while the exclusion comment goes stale;
  spike bins promoted to production later must be brought up to full lint
  standard at promotion time.
- **Mitigations:** the `rust.yml` comment states the leaf-crate precondition;
  promotion of spike code into a production crate already requires a normal
  reviewed PR where the full gates apply.

## References

- Related ADRs: ADR-057 (dev-environment hardening)
- APS modules: IMPV-001 (impact view productisation, validated by the spike)
- External: PR #4074 (rataflow spike), CIB-193/CIB-204 (cross-target clippy gate)
