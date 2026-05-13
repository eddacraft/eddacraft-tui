# ADR-043: SSH Remote Host Daemon

## Status

Proposed

## Date

2026-05-14

## Context

The multi-layer protection spec marks SSH remote support as later work because
Anvil cannot honestly claim save-time or pre-write protection for remote files
by watching them from the local machine. ADR-036 already defines the relevant
boundary: a daemon's authority is one execution scope, meaning one kernel-visible
filesystem and process space. ADR-037 then makes cross-machine federation work
through git witnesses and L4 validation rather than daemon-to-daemon RPC.

Remote SSH development adds a common workflow that the current v1/v1.5 plans do
not cover:

- the user's UI may be local while the repository, agent process, git hooks, and
  file writes live on a remote host
- the remote host may have its own kernel, UID, runtime directory, shell, git
  configuration, and Anvil binary version
- a local daemon cannot observe remote writes without SSHFS-style filesystem
  illusions, and those illusions have the same false-confidence risk as the
  cross-Windows/WSL boundary rejected by ADR-036

The core decision is whether SSH support should bridge local surfaces to a local
daemon, tunnel daemon IPC, or run the daemon where the writes happen.

## Decision

SSH remote support will use a **remote-host daemon** model.

When work happens over SSH, the remote host is the execution scope. Anvil runs
`anvil intercept ensure`, hooks, `anvil-run`, status checks, and daemon IPC on
the remote host. The local machine may provide UI and control-plane commands,
but it must not claim local daemon protection for remote files.

The SSH driver contract is:

1. **Remote execution is authoritative.** The remote daemon computes its own
   `os_locality_token`, writes its own local runtime `info.json`, owns watcher
   state, fences remote process groups, and appends witness lines for commits
   produced in the remote checkout.
2. **Local side is display/control only.** Local editor, MCP, and CLI surfaces
   may run SSH commands to ask the remote Anvil binary for status, validation,
   or session launch, but the local daemon is not part of the protection claim.
3. **No path-translation guarantee.** Local paths and remote paths are distinct.
   Remote requests must carry remote `repo_root`, `worktree_root`, and `cwd` as
   reported by the remote process. Any uncertain mapping reports a degraded or
   refused state rather than best-effort protection.
4. **SSH is the transport and identity boundary.** v1 remote support relies on
   the user's SSH authentication and host-key verification. Anvil records remote
   attach/launch attempts for audit, but it does not invent a second remote auth
   layer in the first slice.
5. **Git remains the federation layer.** Witness convergence and L4 decisions
   continue through `anvil/witness/` and `refs/notes/anvil-l4`; no inter-daemon
   replication protocol is introduced.
6. **Protection claims stay closed-set and honest.** `anvil status`, MCP
   responses, and doctor output gain explicit remote states rather than
   overloading local `full` / `degraded-protection` states.

## Rationale

Running the daemon on the remote host aligns with the existing execution-scope
model and avoids path-watching lies. It keeps the high-integrity path local to
the remote kernel while preserving Anvil's existing cross-machine story: git
carries witnesses, L4 validates pushes, and CI/GitHub Action coverage catches
missed local layers.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Remote-host daemon | Honest watcher/process authority; fits ADR-036; hooks and `anvil-run` run where git and files live; no new daemon replication protocol | Requires remote bootstrap, remote status UX, SSH command orchestration, and version negotiation |
| Local daemon watches SSHFS-mounted repo | Simple mental model for local tools | False confidence: filesystem notifications, symlinks, permissions, case handling, and process control do not match the remote execution scope |
| Tunnel local surfaces directly to remote daemon IPC | Could reuse daemon protocol | Exposes local IPC protocol over a new trust boundary; needs auth, replay protection, and transport versioning before the core remote workflow is proven |
| Server-side only through L4/CI | Already mostly available | Does not provide save-time/pre-write protection for remote agent edits; too late for the core Anvil promise |

## Consequences

- **Positive:** SSH development can become a first-class protected workflow
  without weakening the execution-scope doctrine.
- **Positive:** Remote witnesses are naturally comparable with laptop/desktop
  witnesses because they share the ADR-037 envelope and project identity model.
- **Positive:** The local UI can stay lightweight; correctness lives on the
  remote host.
- **Negative:** Remote support needs explicit install/version/status handling
  before it can be user-friendly.
- **Negative:** Editor and MCP surfaces need remote-driver integration rather
  than assuming local socket discovery.
- **Risk:** Users may confuse local and remote protection states.
- **Mitigation:** Add remote-specific closed-set states and contract tests before
  claiming support.
- **Risk:** SSH host identity and remote Anvil binary provenance may be weak in
  unmanaged environments.
- **Mitigation:** Treat SSH as the initial trust boundary, record audit facts,
  and leave stronger remote attestation for a later ADR if design-partner needs
  justify it.

## References

- Related ADRs: ADR-036, ADR-037, ADR-038
- APS modules: SSHREMOTE, INTL, MLP, INTD, DRVR, RMCP/RMCPF
- Spec: `plans/specs/2026-05-14-ssh-remote-host-daemon.md`
