# RTAI-005 — LSP vs MCP wire-protocol spike report

**Date:** 2026-07-16
**Branch:** `feat/rtai-005-lsp-spike`
**Spike binary:** `crates/spike/src/rtai_005_lsp_vs_mcp.rs` (`spike-rtai-005-lsp-vs-mcp`)
**Also added:** `anvil lsp --stdio` (`crates/anvil-cli/src/commands/lsp.rs`) — a
throwaway, not-production-hardened LSP surface proving the loop end to end.
**Status:** Spike closed. Settles RTAI-005's connection-lifecycle question
(per-call connect, matching the existing MCP client) and answers ADR-109's
open speed/token-perf question. Does not itself authorize a full build —
scheduling stays a separate, operator-owned step per ADR-109.

## What this is

Per RTAI-005's own readiness note, the recommended un-park step before a full
build is "an RTAI-001-style throwaway spike first (`anvil lsp --stdio`, one
rule, one fixture, didChange -> `scan_buffer` -> `publishDiagnostics`) to
settle the connection-lifecycle question cheaply." This spike does exactly
that, then goes one step further: ADR-109 (protocol plurality — anvil
supports LSP alongside MCP, not one instead of the other) raised a live
question about relative cost. This spike measures it.

Two things were built:

1. **`anvil lsp --stdio`** — a minimal, Unix-only, not-production-hardened LSP
   server. Handles `initialize`, `textDocument/didChange` ->
   `scan_buffer(mode=midEdit)` -> `textDocument/publishDiagnostics`,
   `shutdown`/`exit`. Connects to the daemon fresh per call, mirroring
   `crate::mcp::validation::SocketDaemonValidationClient`'s existing
   connection-lifecycle choice — deliberately not merged with that
   production, fail-closed-tested client. One rule (`secret-detection`), one
   fixture, per the RTAI-001 precedent.
2. **`spike-rtai-005-lsp-vs-mcp`** — spawns the *real* `anvil lsp --stdio` and
   `anvil mcp serve --stdio` subcommands as child processes against a *real,
   running* intercept daemon, drives 200 round-trips of the identical fixture
   over each protocol, and measures latency and wire-payload bytes for both.

This is a different shape from RTAI-001's own spike. RTAI-001 simulated the
daemon in-process (`mpsc` channel) to measure a protocol-free latency floor.
This spike holds the daemon and rule fixed and measures the *protocol*
difference instead — it genuinely needs both real subcommands built and a
live daemon, not a simulation.

## Setup

| Component | Spike implementation |
|---|---|
| LSP driver | Real `anvil lsp --stdio` subprocess, `Content-Length`-framed JSON-RPC (no `tower-lsp`/`lsp-types` dependency — hand-rolled, mirroring `anvil mcp serve --stdio`'s existing hand-rolled NDJSON loop) |
| MCP driver | Real `anvil mcp serve --stdio` subprocess (existing, shipped), NDJSON JSON-RPC, `tools/call anvil_validate_write` |
| Daemon | Real intercept daemon (`anvil intercept start --foreground`), scratch `ANVIL_HOME`, `ANVIL_DEV=1` |
| Rule | `secret-detection` (fires on a `ghp_...`-shaped token) — reached via the identical `scan_buffer` RPC on both paths (`mode=midEdit` for LSP, `mode=preWrite` for MCP) |
| Fixture | `const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n` |
| Iterations | 200 round-trips per protocol, after `initialize`/`initialized` handshake (not timed) |
| Connection lifecycle | Fresh `UnixStream::connect` per call on both paths — matches the existing MCP client's choice today; a persistent-connection variant is future work, not this spike's scope |

## Measurement

```text
RTAI-005 spike: LSP vs MCP mid-edit wire overhead
---------------------------------------------------
iterations  : 200
fixture     : "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n" (fires `secret-detection`)

=== LSP (didChange -> scan_buffer mode=midEdit -> publishDiagnostics) ===
latency  p50=31.54ms  p95=32.91ms  mean=31.72ms
payload  avg_request=215B  avg_response=332B  avg_total=547B

=== MCP (tools/call anvil_validate_write -> scan_buffer mode=preWrite) ===
latency  p50=32.81ms  p95=34.26ms  mean=33.03ms
payload  avg_request=230B  avg_response=1215B  avg_total=1445B

=== Comparison ===
latency p50: LSP 31.54ms vs MCP 32.81ms
latency p95: LSP 32.91ms vs MCP 34.26ms
payload avg total: LSP 547B vs MCP 1445B (2.64x)

ADR-031 mid-edit budget (validation.roundtrip p95 <= 80ms, warm daemon):
  LSP p95=32.91ms -> PASS
  MCP p95=34.26ms -> PASS
```

Reproduce with `ANVIL_BIN=<path-to-built-anvil> cargo run -q --release -p
anvil-spike --bin spike-rtai-005-lsp-vs-mcp` against a running daemon
(`anvil intercept start --foreground`, ideally under a scratch `ANVIL_HOME`).
Absolute latency is dev-machine specific; the ~2.6-2.8x payload ratio was
stable across three independent runs (two ad hoc Python-harness runs at
n=100/n=200 during exploration, plus this Rust spike at n=200) and is the
number to trust.

## What the numbers mean

**Speed: no meaningful difference, and this generalizes.** Both protocols
land at ~31-34ms p50/p95, both comfortably inside ADR-031's 80ms warm budget.
The gap between them (~1ms) is noise. This is expected once you see where the
time actually goes: both paths hit the identical `scan_buffer` RPC on the
identical daemon connection pattern (fresh `UnixStream::connect` per call).
Protocol framing (`Content-Length` headers vs NDJSON) is negligible next to
connect-plus-daemon-roundtrip cost. **This conclusion is protocol-shape-
independent** — it will hold for any future `scan_buffer`-backed LSP method,
not just diagnostics.

**Payload/tokens: real, but the headline 2.6-2.8x figure does not generalize
uniformly.** Decomposing where MCP's extra bytes go (measured on one sample
response, byte-exact):

| Component | Bytes | Share of the ~950B gap | Nature |
|---|---|---|---|
| MCP wrapper + double-JSON-encoding tax (`content[0].text` carries the real payload as an *escaped string*, plus `jsonrpc`/`id`/`result`/`content`/`type`/`isError` wrapper fields) | 221B | 23% | **Protocol-structural.** Applies to *any* MCP tool call, custom or spec-blessed. A custom LSP extension method avoids it structurally — LSP's `result`/notification `params` carry native JSON, never double-encoded. |
| `anvil_validate_write`'s governance fields (`decision`, `safeDefault`, `protection_claim`, `tier`, `correlation`, `schema`, `summary`) | 622B | 65% | **Tool-specific content**, not protocol-inherent. These exist because `validate_write` is a write *gate* ("is this write allowed") — a read-only query tool (`anvil_find_callers`, `anvil_impact_of_change`, `anvil_affected_tests`) has no gate decision to carry and would not include them regardless of transport. |
| Diagnostics-equivalent content (location/severity/rule/message) | 178B | — | Present on both sides either way; this is the actual information being conveyed. |

**Extrapolation to a full LSP suite with MCP tool-call parity** (ADR-109's
actual scope — not diagnostics-only): only the 23% protocol-structural
component is a durable LSP advantage for read-only query capabilities like
find-usages / impact-of-change / affected-tests. The 65% governance-field
component is an artifact of comparing against a write-gate response, not a
protocol effect — a fair comparison (lean query tool vs. lean custom LSP
method, same information) would not carry it. **Rough expectation: ~15-20%
lower payload/token cost for LSP on tool-call-parity capabilities**, not the
~2.6-2.8x measured here for diagnostics-vs-write-gate. That number should
compress further toward high single digits as result payloads grow — the
fixed wrapper overhead (~90B) amortizes over a larger response, though the
escaping-tax component scales roughly with how much quoted string content the
payload carries, so it does not vanish entirely. Not yet measured directly;
a matched-content benchmark against a real GCTX tool (e.g. `anvil_find_callers`
vs. a hand-built `textDocument/references` response for the same query) would
pin this exactly rather than extrapolate it, if/when that precision is
needed for a build decision.

**Bottom line:** the spike clears ADR-031's gate on both protocols, confirms
the connection-lifecycle question RTAI-005 flagged (fresh-connect-per-call is
fine at this budget), and gives ADR-109's protocol-plurality decision a real,
decomposed cost basis instead of an assumption — LSP's advantage is real but
concentrated in one structural mechanism (avoiding double-JSON-encoding), not
in "LSP is inherently leaner," which does not hold once governance-specific
content is factored out.

## Decisions

### Decision (a): connection lifecycle — fresh-connect-per-call or persistent?

**Decision: fresh-connect-per-call is fine at today's measured cost; defer a
persistent-connection variant until it's the bottleneck, not before.**

RTAI-005's readiness note flagged this as the open question a spike should
settle. At ~31-34ms round-trip against an 80ms budget, connect overhead is
not close to threatening the budget on either protocol. `anvil lsp --stdio`
mirrors `SocketDaemonValidationClient`'s existing choice rather than
inventing a new pattern. If a future full build finds keystroke-cadence
debounced calls pushing closer to budget, `crates/anvil-intercept/benches/
midedit_roundtrip.rs`'s persistent-connection harness is the existing
precedent to follow — this spike does not need to solve that now.

### Decision (b): does the LSP surface need `tower-lsp`/`lsp-types`?

**Decision: no, for this scope.** The spike's hand-rolled `Content-Length`
framing (~180 lines) is smaller than the MCP loop it mirrors and needed no
new dependency. If a full build later needs LSP capabilities this spike
doesn't touch (workspace/symbol negotiation edge cases, capability
handshaking beyond `textDocumentSync`), revisit — but nothing here forces
that dependency now.

### Decision (c): what does "LSP is cheaper" actually mean for the full suite?

**Decision: the payload advantage is real but tool-call-parity capabilities
should be budgeted at ~15-20%, not the ~2.6-2.8x this spike measured for
diagnostics.** See the decomposition above. Any future ADR or planning pass
that cites "LSP is N% cheaper" should cite the 23%-of-gap structural number
(protocol-inherent) and treat the rest as specific to `validate_write` being
a write gate, not to LSP vs MCP in general.
