# SSH Remote Host Daemon Design

**Status:** Proposed
**Date:** 2026-05-14
**ADR:** [`ADR-043`](../decisions/043-ssh-remote-host-daemon.md)
**APS:** [`ssh-remote-host-daemon`](../modules/ssh-remote-host-daemon.aps.md)

## Summary

SSH remote support uses the remote host as the Anvil execution scope. The daemon,
hooks, `anvil-run`, git operations, process control, witness writes, and status
probes run on the remote host. The local machine may initiate SSH commands and
render results, but it does not provide the protection claim for remote files.

## Goals

- Protect agent and editor workflows where the checkout lives on an SSH remote.
- Preserve ADR-036's execution-scope rule: the daemon runs where the kernel can
  observe writes and control processes.
- Preserve ADR-037's federation model: witnesses and L4 validation travel through
  git, not daemon-to-daemon RPC.
- Provide closed-set, honest remote protection states before any user-facing
  claim says SSH remote support is protected.

## Non-Goals

- SSHFS or local-daemon watching of remote paths.
- TCP transport for daemon IPC.
- Cross-Windows/WSL bridging.
- Remote attestation beyond the user's SSH trust boundary in the first slice.
- Hosted Anvil cloud, GitHub App, or central policy service dependency.

## Model

```text
local UI / editor / MCP shim
        |
        | ssh command/control
        v
remote shell / remote checkout
        |
        | local IPC on remote host
        v
remote anvil daemon + hooks + anvil-run
        |
        | git push / witness files / L4
        v
remote or hosted git enforcement
```

The remote Anvil binary is the authority for:

- `anvil intercept ensure`
- `anvil status --json`
- `anvil doctor --json`
- `anvil-run --tool ... -- <command...>`
- `anvil hook ...`
- `anvil baseline` and `anvil start` inside the remote checkout

## Protection Claim

Remote support must not reuse local `full` unless the protected files and daemon
are local to the same execution scope. The SSH driver should add explicit states,
for example:

| State | Meaning |
| ----- | ------- |
| `remote-unconfigured` | SSH target/checkouts are known, but no remote Anvil contract is established. |
| `remote-daemon-down` | SSH works, but remote `anvil intercept ensure` or status failed. |
| `remote-attached` | Remote daemon is reachable and the remote checkout identity matches. |
| `remote-protected` | Remote daemon, hooks, launcher/session registration, and witness/L4 surfaces are healthy. |
| `remote-degraded` | Remote support is active but one or more remote layers are unavailable. |
| `remote-path-uncertain` | Local/remote path mapping is ambiguous; protection claim is refused or downgraded. |
| `remote-version-mismatch` | Local driver and remote Anvil protocol versions are incompatible. |

Exact names are owned by the SSHREMOTE implementation task and must be pinned in
the protection-claim contract suite before release.

## Required Work

1. Define the SSH command/status contract.
2. Add remote bootstrap checks for binary presence, version, protocol, and
   `anvil intercept ensure`.
3. Teach driver surfaces to use remote status and validation commands without
   treating local daemon discovery as authoritative.
4. Route remote agent launches through remote `anvil-run`.
5. Ensure hooks and witness writes happen inside the remote checkout.
6. Add remote protection-claim fixtures and E2E tests.
7. Document adoption, troubleshooting, and failure modes.

## Open Questions

1. Should the first remote driver support only explicit configured SSH targets,
   or should it auto-detect common editor remote sessions?
2. Which local surfaces are first: CLI-only, MCP shim, editor driver, or all
   three behind one remote-driver facade?
3. Should the remote driver cache remote status locally, or always query live?
4. How much host-key / SSH config information should Anvil surface in audit
   output without leaking user environment details?
