# RTAI-005 Production LSP Diagnostics Implementation Plan

**Goal:** Reduce PR #3360 to a production, diagnostics-only `anvil lsp --stdio`
surface and remove every graph-navigation experiment from that branch.

**Architecture:** The LSP process is a thin Rust frontend over the existing
daemon `scan_buffer(mode = "midEdit")` contract. It owns protocol framing,
document lifecycle, debounce/coalescing, URI and diagnostic-range conversion,
and degradation; it does not query or project graph navigation. A shared
cross-platform daemon client keeps Unix sockets and Windows named pipes aligned.

**Tech Stack:** Rust, Tokio, JSON-RPC/LSP 3.17 framing, existing
`anvil-intercept-proto`, Unix domain sockets, Windows named pipes, Cargo tests.

---

## File Map

- `crates/anvil-cli/src/commands/lsp.rs` — stdio server orchestration and public
  `anvil lsp --stdio` entrypoint.
- `crates/anvil-cli/src/commands/lsp/protocol.rs` — bounded LSP frame parsing,
  lifecycle requests/notifications, URI and diagnostic DTO conversion.
- `crates/anvil-cli/src/commands/lsp/state.rs` — open-document versions,
  debounce/coalescing and cancellation state.
- `crates/anvil-cli/src/daemon_validation.rs` — shared cross-platform
  `scan_buffer` client used by MCP and LSP.
- `crates/anvil-cli/src/mcp/validation.rs` — delegates existing pre-write calls
  to the shared client without changing MCP behaviour.
- `crates/anvil-cli/src/main.rs` — declares the shared client module only; CLI
  shape remains `anvil lsp --stdio`.
- `crates/anvil-cli/tests/lsp_diagnostics.rs` — process-level LSP lifecycle,
  debounce, diagnostics, degradation, URI and Unicode contract tests.
- `crates/anvil-intercept/benches/midedit_roundtrip.rs` — production diagnostics
  path latency scenario.
- `crates/anvil-intercept/benches/baselines/midedit_roundtrip.json` — accepted
  diagnostics-only benchmark baseline.
- `crates/spike/src/rtai_005_lsp_vs_mcp.rs` — retains the closed wire-overhead
  spike; removes navigation-suite benchmarking.
- `crates/spike/Cargo.toml` and `Cargo.lock` — remove dependencies introduced
  only for navigation benchmarking.
- `crates/anvil-gctx-types/src/lib.rs`, `crates/anvil-gctx-egress/src/lib.rs`,
  `crates/anvil-graph-cache/src/{lib.rs,call_graph.rs}`,
  `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/{ipc.rs,save_time.rs}` — restored to `main` for
  this PR; navigation changes are rebuilt under LSPNAV.
- `plans/modules/realtime-ai-validation.aps.md` — RTAI-005 evidence and status;
  no navigation claims.

## Task 1: Remove navigation from PR #3360

**Files:**

- Modify: `crates/anvil-cli/src/commands/lsp.rs`
- Restore to `main`: `crates/anvil-gctx-types/src/lib.rs`
- Restore to `main`: `crates/anvil-gctx-egress/src/lib.rs`
- Restore to `main`: `crates/anvil-graph-cache/src/lib.rs`
- Restore to `main`: `crates/anvil-graph-cache/src/call_graph.rs`
- Restore to `main`: `crates/anvil-intercept-proto/src/protocol.rs`
- Restore to `main`: `crates/anvil-intercept/src/ipc.rs`
- Restore to `main`: `crates/anvil-intercept/src/save_time.rs`
- Modify: `crates/spike/src/rtai_005_lsp_vs_mcp.rs`
- Modify: `crates/spike/Cargo.toml`
- Modify: `Cargo.lock`

- [ ] Add a diff-contract test/script assertion that the RTAI branch exports no
      `symbol_at`, references query, impact-of-change, affected-tests, or
      navigation benchmark method.
- [ ] Run the assertion against the current branch and verify it fails on the
      experimental methods.
- [ ] Remove `textDocument/references`, both custom navigation methods, all
      graph query/client code, LSP-side workspace file reads, and navigation
      benchmark cases.
- [ ] Restore all GCTX, graph-cache and intercept protocol changes to the
      current `main` versions; preserve only files needed by diagnostics.
- [ ] Run `git diff --name-only origin/main...HEAD` and verify the graph/GCTX/
      intercept navigation files no longer appear unless a diagnostics-only
      shared-client refactor requires them.
- [ ] Commit: `refactor(rtai-005): remove navigation from lsp diagnostics`

## Task 2: Share the cross-platform mid-edit daemon client

**Files:**

- Create: `crates/anvil-cli/src/daemon_validation.rs`
- Modify: `crates/anvil-cli/src/mcp/validation.rs`
- Modify: `crates/anvil-cli/src/commands/lsp.rs`
- Modify: `crates/anvil-cli/src/main.rs`
- Test: `crates/anvil-cli/src/daemon_validation.rs`

- [ ] Write unit tests for `scan_buffer` request construction with distinct
      `midEdit` and `preWrite` modes, unique request IDs, timeout, response-size
      cap, structured daemon errors, Unix socket routing, and Windows pipe
      routing behind the appropriate `cfg`.
- [ ] Run `cargo test -p eddacraft-anvil daemon_validation` and verify the new
      tests fail because no shared client exists.
- [ ] Move the transport-neutral request/response logic and both platform
      transports from the MCP-local implementation into `daemon_validation`.
      Expose only `scan_buffer(mode, path, content, cancellation)` and keep wire
      types private. LSP owns workspace admission and most-specific-root
      selection, then passes the selected-workspace-relative path because the
      existing sealed daemon RPC has no workspace-root field.
- [ ] Make MCP call the shared client in `preWrite` mode and LSP call it in
      `midEdit` mode. Do not change enforcement semantics or add a persistent
      connection; the accepted spike proved per-call connect meets ADR-031.
- [ ] Run `cargo test -p eddacraft-anvil daemon_validation` and the existing
      `cargo test -p eddacraft-anvil mcp_validate_write` tests; verify green.
- [ ] Commit: `refactor(cli): share cross-platform daemon validation client`

## Task 3: Harden the LSP protocol and document lifecycle

**Files:**

- Create: `crates/anvil-cli/src/commands/lsp/protocol.rs`
- Create: `crates/anvil-cli/src/commands/lsp/state.rs`
- Modify: `crates/anvil-cli/src/commands/lsp.rs`
- Test: `crates/anvil-cli/src/commands/lsp/protocol.rs`
- Test: `crates/anvil-cli/src/commands/lsp/state.rs`

- [ ] Write failing tests for the 4 MiB frame cap, case-insensitive LSP headers,
      duplicate/invalid `Content-Length`, malformed JSON request errors,
      initialize-before-use, shutdown/exit order, `didOpen`/full-sync
      `didChange`/`didClose`, monotonically increasing document versions,
      percent-encoded file URIs, non-file URI refusal, multi-root selection,
      CRLF, astral Unicode and combining characters.
- [ ] Run `cargo test -p eddacraft-anvil commands::lsp` and verify failures pin
      the missing production behaviour.
- [ ] Implement a bounded protocol state machine. Store only open document URI,
      version, text and pending cancellation handle. Reject out-of-order versions
      and unsupported URI schemes without filesystem access.
- [ ] Convert diagnostic byte/line data to zero-based UTF-16 positions from the
      request buffer, clamping only at valid UTF-8 boundaries and never reading
      another workspace file.
- [ ] Preserve stdout for framed protocol messages and route operational errors
      to tracing/stderr without paths or source.
- [ ] Run `cargo test -p eddacraft-anvil commands::lsp`; verify green.
- [ ] Commit: `feat(rtai-005): harden lsp lifecycle and coordinates`

## Task 4: Add debounce, coalescing and cancellation

**Files:**

- Modify: `crates/anvil-cli/src/commands/lsp/state.rs`
- Modify: `crates/anvil-cli/src/commands/lsp.rs`
- Test: `crates/anvil-cli/src/commands/lsp/state.rs`

- [ ] Write paused-time Tokio tests proving an 80 ms default debounce, latest
      version wins, identical content hashes avoid a round-trip, a new edit
      cancels the prior scan, close cancels pending work, and no diagnostic for
      a stale version is published.
- [ ] Run `cargo test -p eddacraft-anvil lsp_debounce` and verify red.
- [ ] Implement one bounded pending scan per open document using Tokio timers
      and cancellation tokens. Capture URI/version/hash with the request and
      compare again before publication.
- [ ] Treat daemon unavailable, timeout, cancellation and structured errors as
      no in-flight diagnostics for that version; never suppress the independent
      save-time path.
- [ ] Run `cargo test -p eddacraft-anvil lsp_debounce`; verify green.
- [ ] Commit: `feat(rtai-005): debounce and cancel mid-edit diagnostics`

## Task 5: Prove the production diagnostics flow end to end

**Files:**

- Create: `crates/anvil-cli/tests/lsp_diagnostics.rs`
- Modify: `crates/anvil-intercept/benches/midedit_roundtrip.rs`
- Modify: `crates/anvil-intercept/benches/baselines/midedit_roundtrip.json`

- [ ] Write a process harness that starts a real daemon in an admitted temporary
      workspace, starts `anvil lsp --stdio`, sends framed initialise/open/change/
      shutdown/exit traffic, and asserts version-matched `publishDiagnostics`
      for a known rule fixture.
- [ ] Add daemon-down, cancellation race, duplicate edit, Unicode/URI,
      multi-root, frame-overflow and clean-clear cases. Use the same assertions
      on Unix and Windows; platform helpers may differ but expected messages may
      not.
- [ ] Run `cargo test -p eddacraft-anvil --test lsp_diagnostics` and verify red.
- [ ] Make the minimal server/client changes needed for the process suite to
      pass without adding any navigation capability.
- [ ] Run `cargo test -p eddacraft-anvil --test lsp_diagnostics` on the local
      platform and ensure the Windows CI job executes the same target.
- [ ] Run `cargo bench -p eddacraft-anvil-intercept --bench midedit_roundtrip`
      and verify warm round-trip p95 remains within ADR-031's 80 ms budget.
- [ ] Commit: `test(rtai-005): prove lsp diagnostics end to end`

## Task 6: Reconcile APS and run the evidence gate

**Files:**

- Modify: `plans/modules/realtime-ai-validation.aps.md`

- [ ] Update RTAI-005 only with fresh diagnostics evidence, branch/PR reference,
      and its true lifecycle status. Keep LSPNAV as a separate Proposed module.
- [ ] Run `cargo test -p eddacraft-anvil --test lsp_diagnostics`.
- [ ] Run `cargo test -p eddacraft-anvil daemon_validation`.
- [ ] Run `cargo bench -p eddacraft-anvil-intercept --bench midedit_roundtrip`.
- [ ] Run `pnpm validate:changed`, `pnpm aps:active-lint`, and
      `pnpm aps:index:check`; record outputs in the PR evidence block.
- [ ] Run Council against the diagnostics-only diff. Add `council:reviewed` only
      after no critical or major findings remain.
- [ ] Commit: `docs(rtai-005): record production diagnostics evidence`
