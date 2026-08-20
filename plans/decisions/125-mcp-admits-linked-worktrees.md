# ADR-125: MCP workspaceRoot admits linked git worktrees of the same repository

## Status

Proposed

## Date

2026-08-20

## Context

CIB-007 made `anvil_validate_write` return `expectedWorkspaceRoot` when the
caller passed a workspace the MCP server would not trust. The trust check
itself stayed exact equality with the MCP process cwd (other tools used
path containment under that cwd). Option (a) — admit linked worktrees —
was deferred because it widens the MCP path boundary and needed an ADR.

Agent harnesses start `anvil mcp serve --stdio` from the primary checkout.
Everyday work happens in sibling Worktrunk or harness worktrees of the same
Git repository. Those roots are not inside the server cwd, so every MCP
tool refuses them (`untrusted-workspace-root` / "must be inside the MCP
server root"). Agents then validate against the primary tree (content-mode)
and write in the worktree. That is not MCP access for the workspace being
edited.

This MCP path check is **not** daemon allowlist confinement. ADR-097 still
governs which roots the intercept daemon will serve in `allowlist` mode.
Admitting a worktree here only lets the stdio MCP server run its tools
against that tree. Save-time daemon admission is unchanged.

## Decision

MCP tools admit a caller `workspaceRoot` when its canonical path is:

1. The MCP server cwd, or a directory inside it (the historical rule), or
2. A **registered** Git worktree of the same repository as the server cwd,
   or a directory inside such a worktree.

"Registered" means the on-disk Git layout Git itself writes: the main
worktree is the parent of the common git dir when that dir is named
`.git`; linked worktrees are the parents of the paths recorded in
`<common>/worktrees/*/gitdir`. Resolution parses those files in the MCP
layer (portable; intercept's gitdir helpers are unix-gated) and does
**not** spawn `git`.

Refuse:

- another Git repository (different common dir)
- a non-Git sibling directory
- a directory that only plants a `.git` `gitdir:` pointer at this
  repository without a matching `<common>/worktrees/*/gitdir` entry

`untrusted-workspace-root` remains the validate-write code for a refused
root and still carries `expectedWorkspaceRoot` (the server cwd) so callers
can self-correct. Other tools keep a string error. The message names
linked worktrees as well as containment.

Responses that redact `workspaceRoot` must not emit an absolute linked
worktree path. A worktree that is not inside the server cwd redacts to
`worktree:<basename>`.

This does not amend ADR-097, ADR-061 §7, or ACTMO durable registration.

## Rationale

Linked worktrees of the MCP server's repository **are** that workspace.
Worktrunk, Grok, and Codex all isolate there. The original equality check
was defending "do not let the client point this session at a different
project", not "only the directory the harness happened to spawn in".

Listing registered worktrees from the **trusted** server cwd is tighter
than comparing `git-common-dir` of the caller path: a forged `.git` file
can impersonate the common dir, but it does not appear in
`<common>/worktrees/` unless Git registered it.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Registered worktrees of the same repo (chosen) | Matches how agents actually work; still rejects other projects; no `git` spawn | Slightly wider MCP path set than cwd-only |
| Compare git-common-dir only | Simpler | Admits a forged `.git` gitdir pointer |
| Start a new MCP server per worktree | No anvil trust change | Every harness must re-exec; does not help mixed-root sessions |
| Keep cwd-only + agent content-mode | No product change | Worktrees do not get MCP; recurring field friction |
| Implicitly admit any same-uid path | Convenient | Defeats the other-project guard; collides with ADR-097's lesson |

## Consequences

- **Positive:** `anvil_validate_write`, `anvil_apply_patch`, graph tools,
  check/gate/status, and the other MCP tools can run against the worktree
  the agent is editing. Patch-mode becomes available there. Graph RPCs
  receive the worktree path the daemon already keys overlays on.
- **Negative:** A registered worktree is now an admitted MCP root even if
  the harness spawned in a sibling tree. That is the intended widening.
- **Risks:** A stale `worktrees/` admin dir whose checkout was deleted is
  skipped (canonicalize fails). A worktree added after the process started
  is visible on the next call (no cache).
- **Mitigations:** Daemon allowlist (ADR-097) still applies to save-time
  RPCs. Path escape and symlink checks still run inside the admitted root.

## References

- Related ADRs: CIB-007 recoverability was the deferral; ADR-097 (daemon
  allowlist; not amended); ADR-094 (worktree registration UX); ADR-105
  (per-worktree graph overlays)
- APS: CIB-007 option (a), follow-up to deferred worktree-aware accept
- Field: `theme:worktree-mcp-root`, `theme:worktree-developer-functions`
