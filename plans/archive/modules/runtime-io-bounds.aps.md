# Runtime I/O Bounds

| ID | Owner | Priority | Status |
| --- | --- | --- | --- |
| RIO | @joshuaboys | P1 | Complete |

**Packages:** eddacraft-anvil

## Purpose

Complete the runtime deadline and committed-blob resource repairs requested by
the operator on 2026-09-05. This exclusive module owns issues #4231 and #4232.
Implementation and rebase merge on green are authorised in that request.

## Design boundary

Preserve existing CLI commands, daemon wire framing, peer validation and cold
fallback behaviour. Use one monotonic two-second deadline for daemon request
writes and framed reads. Git history retains its three-second budget, drains
both pipes concurrently, caps stdout at 4 MiB and stderr at 64 KiB, and kills
and reaps its child on failure. Diagnostics contain reasons, not captured data.

Probe immutable Git object metadata before requesting contents. Keep the
existing 8 MiB per-blob limit; cap retained batch bodies at 64 MiB and metadata
at 4 MiB. Skip over-budget objects deterministically while retaining subsequent
small neighbours where capacity remains. Report bounded skip counts. Malformed
or over-limit transcripts return the existing non-fatal Git error.
These repair existing bounds; no new architecture or public API is introduced.

## Work Items

### RIO-001: Bound daemon validation and Git-history exchanges

- **Status:** Done
- **Intent:** Prevent slow peers and full Git pipes from hanging normal CLI use.
- **Expected Outcome:** Partial socket reads cannot renew the exchange budget;
  incomplete replies fail closed. History output larger than a pipe buffer
  completes; timeout and output-cap failures terminate and reap the child and
  retain the repo-walk fallback. Both output streams have explicit caps.
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`
- **Claim:** [#4231](https://github.com/eddacraft/anvil-001/issues/4231)
- **Files:** `crates/anvil-cli/src/mcp/validation.rs`,
  `crates/anvil-cli/src/services/sample_analyser.rs`, bounded process helper.

**changeType:** fix
**releaseIntent:** candidate
**releaseScope:** patch

### RIO-002: Bound committed-blob materialisation

- **Status:** Done
- **Intent:** Prevent oversized or aggregate-heavy committed trees exhausting memory.
- **Expected Outcome:** Size probes precede content reads; neither oversized
  blobs nor an unbounded batch transcript enter memory. Tests cover over-limit
  objects, aggregate exhaustion, small neighbouring objects and malformed frames.
  Resource failures remain observable and non-fatal to the caller.
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`
- **Claim:** [#4232](https://github.com/eddacraft/anvil-001/issues/4232)
- **Files:** `crates/anvil-cli/src/graph_base_producer.rs`, bounded process helper.

**changeType:** fix
**releaseIntent:** candidate
**releaseScope:** patch

## Verification evidence

The hosted Test job [101240565646](https://github.com/eddacraft/anvil-001/actions/runs/33941784695/job/101240565646)
passed the full workspace suite, including all nine new regression tests, on
2026-09-05. Independent mini Council findings were addressed. Integration is
tracked by [PR #4397](https://github.com/eddacraft/anvil-001/pull/4397), which
requires green CI on its final head before rebase merge. No release is claimed.

## Non-scope

Onboarding redesign, release publication, graph format changes and other
subprocess call sites remain outside this repair.
