# ADR-143: Preserve established MCP sessions across binary updates

## Status

Accepted. The operator accepted JREL-001's conditional reconnect fallback in
PR #4396 and authorised its implementation on 2026-09-05.

## Date

2026-09-07

## Context

The v0.9.5 live-heal design attempted to replace a long-lived
`anvil mcp serve --stdio` process with `execve` after reading a complete
JSON-RPC request but before dispatching it. Keeping the stdio file descriptors
does not keep process-local state. The triggering frame has already left the
kernel pipe, a `BufReader` may hold later pipelined frames, and legacy protocol
negotiation is held in the old process.

JREL-001's public-boundary regression reproduced the failure after
initialisation: request id 2 triggered replacement and disappeared, while the
next pipelined request id 3 reached the new image. A mutating request therefore
could be lost, and retrying without an end-to-end idempotency contract could
repeat accepted work.

## Decision

A skewed MCP process may re-exec only during startup, before its first stdin
read. Once a frame has been read, the process is an established session and
must not replace itself.

At an established-session trigger (`initialize`, `tools/list`, or
`tools/call`), anvil re-checks the preferred executable and refresh generation.
If skew remains, it handles the request on the existing negotiated session and
emits one explicit reconnect instruction for the process. Diagnostic delivery
is best-effort and runs off the protocol thread, so a closed or saturated stderr
cannot stall later accepted frames. Request and reply ids, buffered input,
protocol-era state, and exactly-once handling take precedence over changing the
running image.

The refresh generation remains the cooperative signal. It now means
"re-check preferred identity and report reconnect when this established session
is stale", not permission to discard process-local protocol state.

MCPLH-007 remains the sole owner of any future stable supervisor or proxy. A
supervisor may later replace workers behind a persistent framing and
idempotency boundary and supersede this decision; JREL-001 does not implement
one.

## Rationale

The only safe in-place `execve` boundary in the current host is before the
first read. After that point, the process cannot reconstruct all consumed and
buffered input or prove whether a mutation was accepted. Honest targeted
reconnect guidance is less seamless, but it preserves the established
conversation and never converts an upgrade into request loss.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Startup-only re-exec plus explicit reconnect | Preserves requests, pipelines, protocol state, and exactly-once handling with a small bounded change | Existing sessions remain on the old image until the client reconnects |
| Continue post-read re-exec | Replaces the image without a new process supervisor | Demonstrably loses the triggering request and may lose read-ahead frames |
| Replay the consumed frame into the replacement | Could preserve one request in simple cases | Cannot safely preserve buffered frames, negotiated process state, or mutation idempotency |
| Introduce a stable supervisor now | Can retain framing state while replacing workers | Belongs to MCPLH-007 and requires a larger cross-platform lifecycle and protocol design |

## Consequences

- **Positive:** established sessions answer every accepted request and preserve
  legacy and modern protocol negotiation across an update signal.
- **Positive:** failed or unavailable replacements leave a usable session with
  one actionable reconnect instruction.
- **Positive:** reconnect reporting cannot block the MCP protocol loop even when
  stderr is closed or under backpressure.
- **Negative:** a running MCP child stays stale until that client reconnects.
- **Risk:** users may interpret a refresh generation as completed live
  replacement.
- **Mitigation:** CLI output, the runbook, and the MCPLH design now distinguish
  startup replacement from established-session reconnect.
- **Future:** MCPLH-007 may restore transparent worker replacement only behind a
  persistent supervisor that proves framing and idempotency.

## References

- Related ADRs: ADR-106, ADR-141
- APS modules: JREL-001, MCPLH-002, MCPLH-003, MCPLH-007, MCPLH-008
- Design: [MCP live-heal without harness restart](../specs/2026-08-09-mcp-live-heal-without-harness-restart.md)
- Evidence: `crates/anvil-cli/tests/mcp_reexec.rs`
