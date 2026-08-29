# Streaming the secret scan instead of skipping large files

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for SDT-007 / SDT-008 | SDT | Accepted | 2026-08-29 — design approved in-thread; implementation via SDT-007 and SDT-008 |

| Upstream | Downstream |
| -------- | ---------- |
| Operator design (2026-08-29); SDT-006 as-built; `crates/anvil-checks/src/secret/{check,scanner,entropy,context}.rs`; `crates/anvil-cli/src/commands/{check,audit}.rs`; ADR-085 / DSV-045 assurance states | [SDT-007 / SDT-008](../modules/secret-detection-truth.aps.md); `crates/anvil-checks/ARCHITECTURE.md` |

**Design approved 2026-08-29.** Execution authority is SDT-007 and SDT-008.
This specification does not authorise product code on its own, and neither item
is Ready — promotion remains an operator decision.

## The problem

SDT-006 made whole-file skips honest: a file anvil could not read now blocks a
clean pass and is named. It did not make them *scanned*. The result is that
anvil reports, accurately, that it cannot prove its own `pnpm-lock.yaml` clean —
and offers no way to fix that, because lockfiles deliberately bypass
`skip_extensions` to reach the GH #2584 URL-credential scan. The one file the
lockfile carve-out exists for is the one file the size cap drops.

Measured on this repository, three tracked files exceed the 1 MiB
`MAX_FILE_SIZE`: two `plans/audits/*.json` and `pnpm-lock.yaml`, all in
gate-scanned extensions.

## Why the cap exists, and why it does not need to

`MAX_FILE_SIZE` is documented as avoiding "excessive memory usage on binaries or
generated artefacts". That is a property of how the file is *read*, not of what
the scan *needs*:

- `fs::read_to_string(file)` materialises the whole file (`check.rs`).
- `content.lines().collect::<Vec<_>>()` materialises every line again
  (`scanner.rs`).

Neither pass requires that:

- **Patterns** iterate `content.lines().enumerate()` with no cross-line state.
  `scan_lockfile_url_credentials` is the same shape and needs no context at all.
- **Entropy** reaches only `context_window(lines, index, 2)` and
  `lines.get(index)` — verified by reading `is_benign_entropy_fixture`, which
  touches nothing wider.

A bounded reader with a radius-2 ring buffer therefore reproduces today's
results **exactly**. This is a refactor of the scan's *input* side, not of its
detection semantics, which is why zero corpus drift is the acceptance bar rather
than a hoped-for outcome.

## Rejected: deferred / async scanning

The idea that prompted this design was to park oversize files for a background
scan and process the flags later. It was rejected, and the reasoning is recorded
here because it is a reasonable idea that will be proposed again.

**It is architecturally viable.** Anvil already runs exactly this machine:
`AssuranceState` (`anvil-intercept-proto`, frozen wire, ADR-085 / DSV-045)
carries `Clean · Stale · Pending · Running · Bounded · Unavailable · Unknown`,
with `Bounded` meaning "coverage is bounded — known incomplete, not certifiable
as complete", fail-safe rules (`Unknown` is treated as `Stale`, never `Clean`),
and dirty-during-scan race handling in `assurance.rs`. The vocabulary for
deferred coverage exists and is proven.

It was rejected anyway, for three reasons:

1. **It solves the wrong bound.** Once memory is bounded by a sliding window,
   the only thing left to defer is time — and a 1.27 MB file through line-local
   regex is milliseconds. Deferral buys nothing for realistic file sizes.
2. **CI has no daemon.** No workflow starts one. Gate and CI are where an unread
   credential actually costs something, and they are precisely where there is no
   background worker to hand the file to. Parking would optimise save-time,
   which SDT-006 already made non-blocking, and skip the strict surface.
3. **A queue nobody drains is a false clean with extra steps.** This repository
   already runs a pending queue — `.git/anvil/ci-log-pending` — which stood at
   88 unprocessed entries when this design was written. Parking secrets behind a
   queue with that track record would reintroduce, with more machinery, the
   defect SDT-001 and SDT-006 were written to remove.

There is a fourth consideration that would have been solvable but is worth
recording: a verdict that depends on whether the daemon finished violates
anvil's stated determinism principle. It is resolvable — make scan state an
explicit input and let async results only move `parked → clean | finding`,
never the reverse — but that is a baseline model and would need its own ADR.

**Where deferral genuinely belongs:** SDT-005's opt-in live credential
verification. That waits on a provider's network endpoint, cannot be done at
save-time under any budget, and has no synchronous alternative. File size does
not meet that bar; a network round-trip does.

## Design

### Reading

Replace whole-file reads with a bounded reader. Both passes consume the same
line stream; entropy keeps a radius-2 ring buffer so its context window is
available without retaining the file.

```text
BufReader::lines()
  ring: [n-2, n-1, n, n+1, n+2]
    -> pattern pass   (line-local; needs nothing beyond n)
    -> entropy pass   (needs the ring only)
findings accumulate; dedupe, sort and suppression are unchanged
```

### The cap after streaming

`MAX_FILE_SIZE` survives **only as a runaway guard**, not as the everyday
boundary it is today. Its value must be derived from a measured scan-time
budget: memory is no longer the constraint, so a memory-derived number would be
arbitrary. Exceeding the guard keeps SDT-006 semantics unchanged — it blocks a
clean pass and names the file.

### One scannability predicate

`crates/anvil-cli/src/commands/check.rs` carries `is_secret_scannable`, a copy
of the scanner's `should_skip_file` / `file_exceeds_size_limit` predicates whose
own comment states it is "kept in lockstep with the upstream" pair. That is a
drift hazard documented as a drift hazard, and streaming would spring it: change
the scanner without changing the copy and `anvil check` silently keeps refusing
to scan files the scanner can now handle.

`anvil-checks` exports a single scannability predicate; the `anvil-cli` copy is
deleted.

## Consequences

- Anvil can prove its own `pnpm-lock.yaml` clean for the first time.
- Two 1.5 MB `plans/audits/*.json` files enter the scan for the first time. This
  is a genuine false-positive risk and is measurable — via the SDT-002
  calibration corpus for regression and `scripts/dogfood/external-fp` for
  volume. It is measured before default-on, not argued.
- `anvil check` and the scanner can no longer disagree about what is scannable.

## Non-scope

- **`max_line_bytes` is untouched.** It is ReDoS protection against catastrophic
  backtracking, not a memory bound; moving a pathological line to a background
  thread does not make it safe. SDT-001 already established that the defect is
  the silence, not the skip.
- **Git-history scanning is unaffected.** `git_scanner.rs` applies only
  `max_line_bytes` and never `MAX_FILE_SIZE` — it walks diff lines, not whole
  files, so it does not share this defect.
- **No queue, no daemon dependency, no determinism contract.** See the rejection
  above.
- **The surface gap is separate work.** Making `anvil audit` and planless
  `anvil check` report coverage failures instead of exiting 0 is SDT-008; it
  survives streaming independently, because unreadable files and the SCAN-001
  panic arm produce coverage failures at any file size.
