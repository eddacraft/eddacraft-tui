# Save to validation

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                                                        |
| ----- | ------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCRB | Live   | Last reviewed 2026-08-20 at `97899b00a` against MCP caller-buffer validation, watch routing/fallback, driver supervision, intercept MidEdit/save-time dispatch, and fence source |

| Upstream                                                                                                                                                                                         | Downstream                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| ADR-123, `crates/anvil-intercept/ARCHITECTURE.md`, MCP caller-buffer validation, `watch.rs`, `watch_save_time.rs`, `save_time_driver.rs`, intercept MidEdit/save-time dispatch, and fence source | `docs/runbooks/save-time-background-driver.md` and cross-owner save-time validation navigation |

## Audience, concern, and local authority

This sequence is for contributors and operators who need to distinguish
caller-buffer validation from validation after a file save. It owns the
cross-owner hand-off among editor/MCP clients, the background driver, and the
daemon. INTD's local
[intercept architecture](../../crates/anvil-intercept/ARCHITECTURE.md) owns IPC,
admission, guarded-read, scan, observation, and fence internals.

## Cross-owner sequence

```mermaid
sequenceDiagram
    actor Editor
    participant MCP as MCP pre-write client
    participant Driver as background save-time driver
    participant Daemon as intercept daemon
    participant Guard as guarded workspace read
    participant Checks as validation pipeline
    participant Observe as best-effort observation sink
    participant Fence as durable fence state

    rect rgb(238, 245, 255)
        Note over Editor,Checks: Caller-buffer lane - bytes are not read from disk
        Editor->>Daemon: scan_buffer(mode=MidEdit, caller bytes)
        Daemon->>Checks: validate caller bytes
        Checks-->>Daemon: diagnostics
        opt MidEdit result has findings and emitter admits it
            Daemon-->>Observe: best-effort MidEdit observation
        end
        Daemon-->>Editor: MidEdit verdict
        MCP->>Daemon: scan_buffer(mode=PreWrite, proposed bytes)
        Daemon->>Checks: validate proposed bytes
        Checks-->>Daemon: diagnostics
        Daemon-->>MCP: PreWrite verdict
        Note over MCP,Observe: PreWrite does not enter the MidEdit observation path
    end

    rect rgb(240, 255, 240)
        Note over Editor,Checks: Post-save lane - validate the saved paths
        Editor->>Editor: persist file save
        Driver->>Driver: evaluate routing eligibility
        alt disabled, no-daemon, non-check action, unsupported platform, or default-on daemon not live
            Driver->>Checks: subprocess check, scoped paths or all flag for an empty delete/initial cycle
        else eligible because daemon is live or routing is forced
            Driver->>Daemon: validate_paths(workspace, changed paths)
            alt daemon verdict
                Daemon->>Guard: admit workspace and read guarded paths
                Guard-->>Daemon: guarded bytes or explicit refusal
                Daemon->>Checks: validate guarded content and assurance
                Checks-->>Daemon: diagnostics, coverage, assurance
                Daemon-->>Driver: post-save verdict
            else absent, refused, error, disconnect, or timeout
                Driver->>Checks: scoped subprocess fallback
                Driver->>Driver: unavailable daemon-absent assurance and warn once
            end
            opt later daemon response after disconnect
                Driver->>Daemon: request full scan on reconnect
                Driver->>Driver: clear warning latch
            end
        end
    end

    rect rgb(255, 245, 238)
        Note over Daemon,Fence: Fence transitions are separate from ordinary verdicts
        Daemon->>Fence: live spoof detection - request fence
        Daemon-->>Fence: unsafe-interrupt fence defined but unwired
        Daemon-->>Fence: unregistered or watcher fence defined but unwired
        Note over Driver,Fence: An ordinary validation verdict or degraded assurance does not fence
    end
```

In prose: `scan_buffer` consumes bytes supplied by its caller. MidEdit and
PreWrite are distinct modes on that method; MCP `anvil_validate_write` uses
PreWrite because proposed content may not exist on disk. The post-save driver
instead sends changed path descriptors through `validate_paths`; the daemon
admits the workspace, reads the paths through its guarded boundary, and returns
diagnostics plus coverage and assurance.

Only MidEdit enters the best-effort observation path shown here. Missing
emitters, throttling, sink errors, and later queue loss do not change the
MidEdit verdict. PreWrite does not enter that path, and the post-save lane must
not be collapsed into the caller-buffer lane.

Routing is eligible only for a `check` watch when daemon routing is not
disabled, `--no-daemon` is absent, the platform has a transport, and either a
live daemon answers the default-on probe or routing is forced. Disabled,
not-live, unsupported, and non-check cases bypass the client and keep the
subprocess path. Empty delete- or initial-driven cycles also bypass
`validate_paths` and retain the existing `--all` safety net.

Once routed, daemon absence, refusal, JSON-RPC error, disconnect, or timeout
produces no verdict. The watch client reports `unavailable{daemon-absent}`,
warns once for the disconnect, and runs the subprocess fallback scoped to the
changed paths. A later successful response requests a full scan on reconnect and
clears the warning latch.

Fence persistence is a separate safety transition. The Linux spoof cross-check's
production path reaches `fence_worktree_for_spoof` and is live. The
unsafe-interrupt and unregistered/watcher fence implementations are defined and
tested but have no production call sites at this revision, so the diagram labels
them **defined but unwired**. An ordinary validation finding or degraded graph
assurance does not fence a worktree.

## Source trace

- The MCP-to-PreWrite edge traces to `crates/anvil-cli/src/mcp/validation.rs`
  and `crates/anvil-cli/src/daemon_validation.rs`.
- MidEdit/PreWrite mode parsing, caller-buffer evaluation, and the MidEdit-only
  latency/observation conditions trace to
  `crates/anvil-intercept/src/midedit.rs` and the `scan_buffer` dispatch in
  `crates/anvil-intercept/src/ipc.rs`.
- Routing eligibility, empty-cycle `--all`, scoped daemon fallback,
  `unavailable{daemon-absent}`, warn-once, and reconnect/full-scan behaviour
  trace to `crates/anvil-cli/src/commands/watch.rs` and
  `crates/anvil-cli/src/commands/watch_save_time.rs`; supervision opt-out and
  live/not-live state trace to `save_time_driver.rs`.
- Workspace admission, guarded reads, path validation, diagnostics, coverage,
  and assurance trace to `crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-intercept/src/save_time.rs`, and
  `crates/anvil-intercept/src/validate_paths.rs`.
- Live spoof fencing traces from `run_spoof_cross_check` and
  `spoof_block_response` in `crates/anvil-intercept/src/ipc.rs` to
  `FenceStore::fence_worktree_for_spoof`. Repository-wide caller searches leave
  the unsafe interrupt ladder and `UnregisteredChangePolicy` reachable only from
  definitions/tests, not production wiring, in
  `crates/anvil-intercept/src/interrupt.rs`,
  `crates/anvil-intercept/src/unregistered.rs`, and
  `crates/anvil-intercept/src/fence.rs`.

Operational start, status, log, restart, and opt-out procedures remain in the
[background-driver runbook](../runbooks/save-time-background-driver.md).
