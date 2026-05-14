# Post-merge: feat/rmcpf-010-check-gate-port

PR: #1555
Branch: `feat/rmcpf-010-check-gate-port`
APS: RMCPF-010
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm RMCPF-010 row in `plans/index.aps.md` and the entry in
  `plans/modules/rust-mcp-full-port.aps.md` both show `Complete` after merge
  (agent: yes).
- [ ] Re-run `cargo test -p eddacraft-anvil --test mcp_serve_stdio` on `main`
  post-merge and confirm 19/19 green so the new check/gate tool tests landed
  cleanly with everything else (agent: yes).
- [ ] Manual smoke: run `cargo run -p eddacraft-anvil --bin anvil -- mcp serve
  --stdio` from a real workspace and issue `tools/list`, `tools/call
  anvil_check`, and `tools/call anvil_gate` (planless mode) over JSON-RPC.
  Confirm the responses carry the documented redacted `workspaceRoot`,
  `backend: "local"`, and `daemonStatus: "not-wired"` provenance (human
  required — exercises the live stdio path, not just the test harness).
- [ ] File RMCPF-010 follow-up tickets for: (a) wiring `anvil_check` onto the
  daemon `scan.files` surface once INTD delivers it, dropping the embedded
  fallback to `daemonStatus: "available"`; (b) tightening the MCP tool
  descriptors to `additionalProperties: false` across the surface;
  (c) replacing the 25 ms busy-poll in `wait_with_timeout` with the
  `wait-timeout` crate or a thread-join channel (agent: no — needs APS author
  judgement on Phase / Priority).

## Notes

The PR also refactored `crates/anvil-cli/src/mcp/tools/status.rs` to consume
the new shared helpers in `crates/anvil-cli/src/mcp/tools/shared.rs`; if the
post-merge regression check reports any status-tool behaviour drift, the most
likely cause is a divergence between the inlined helper and the shared one —
diff against the pre-merge `status.rs` to confirm parity.

Phase 1 deliberately ships `anvil_check` as the embedded fallback (no daemon
`scan.files` exists yet) and `anvil_gate` full mode shells the same binary
rather than running the gate path in process. Both are documented as Phase 1
positions; do not file regression issues against these choices without first
re-reading the RMCPF-010 closeout evidence in
`plans/modules/rust-mcp-full-port.aps.md`.
