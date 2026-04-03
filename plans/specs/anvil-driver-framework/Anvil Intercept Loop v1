## Anvil Intercept Loop v1

A light design for the fastest credible version.

---

## 1. Purpose

Anvil Intercept Loop v1 is a **single-host local enforcement prototype** designed to prove one thing:

> Anvil can detect a policy violation from agent-produced file changes and **interrupt the correct running session quickly enough to matter**.

This version is deliberately narrow. It is not the full driver framework. It is the smallest version that tests the real product risk.

---

## 2. Scope

### In scope

* local machine only
* zsh-driven agent launches
* tmux-aware session metadata
* WezTerm tolerated but not deeply integrated
* Rust daemon
* Unix domain socket IPC
* file watch
* deterministic hot-path checks only
* process-group based interruption
* blocked worktree relaunch prevention

### Out of scope

* remote SSH hosts
* web-hosted agent environments
* MCP as a primary control path
* editor integrations
* graph-assisted hot-path checks
* multi-host orchestration
* full lease/capability framework
* dual-lane transport split

---

## 3. Goal

### Primary goal

Prove the full loop works end to end:

1. agent launched via Anvil-aware shell wrapper
2. session registered with daemon
3. file change detected
4. daemon evaluates simple deterministic rules
5. daemon identifies owning session
6. daemon interrupts the correct process group
7. shell wrapper blocks immediate unsafe relaunch in the same worktree

### Success criteria

A forbidden file change in one active session causes:

* interruption of the correct session
* no interruption of unrelated sessions
* interruption within a low-latency local loop
* visible blocked state for the affected worktree/session

---

## 4. Design principles

### Shell-first

Use the shell as the universal ingress for local CLI agent tools.

### Process groups over polite cancellation

Real interrupt comes from process-group control, not product-specific APIs.

### Single-host truth

Keep everything on one machine for v1.

### Minimal contracts

Use the smallest possible session and command model.

### Deterministic only

Only cheap mechanical checks on the hot path.

---

## 5. High-level architecture

```text
zsh wrapper / anvil-run
    -> registers session with daemon
    -> launches agent in dedicated process group
    -> maintains local session context

agent process
    -> writes files in repo/worktree

anvil-daemon
    -> watches filesystem
    -> maps change to session
    -> runs deterministic checks
    -> issues interrupt/block decision

process control
    -> SIGINT / SIGTERM / SIGKILL on session PGID

shell wrapper
    -> prevents relaunch in blocked worktree
```

---

## 6. Core components

## 6.1 `anvil-daemon` (Rust)

Responsibilities:

* own in-memory session registry
* watch filesystem changes
* coalesce change bursts
* run simple rule checks
* map changes to sessions
* issue interrupt commands
* maintain blocked worktree state

### State held in memory

* active sessions
* worktree-to-session mappings
* blocked worktrees
* recent change correlation window

---

## 6.2 `anvil-run` (launcher)

A small wrapper used by zsh functions or aliases.

Responsibilities:

* create `session_id`
* detect cwd/repo/worktree
* capture `TMUX_PANE` if present
* register with daemon
* launch target command in its own process group
* update session with pid/pgid
* exit cleanly and unregister session when done

This can be:

* a small Rust binary, or
* a thin shell wrapper plus Rust helper

For v1, I would prefer a **small Rust launcher** for cleaner process-group handling.

---

## 6.3 zsh integration

zsh functions/aliases wrap tools such as:

* `claude`
* `codex`
* `gemini`

Example shape:

```bash
claude() {
  anvil-run --tool claude -- "$@"
}
```

This gives Anvil a consistent launch boundary without needing tool-specific logic.

---

## 6.4 watch loop

The daemon watches one or more repo/worktree roots for file changes.

Responsibilities:

* receive raw filesystem events
* coalesce noisy save bursts
* canonicalise into a small change model
* correlate change to a likely owning session

For v1, ownership correlation can be pragmatic rather than perfect.

---

## 6.5 process control

The enforcement backbone.

Every launched session must run in its own process group.

Interrupt ladder:

1. `SIGINT`
2. short timeout
3. `SIGTERM`
4. short timeout
5. `SIGKILL` as last resort

This should target the **PGID**, not just the immediate pid.

---

## 7. Session model

Keep it very small.

```ts
type Session = {
  sessionId: string
  tool: string
  cwd: string
  repoRoot?: string
  worktreeRoot: string
  tmuxPane?: string
  pid?: number
  pgid?: number
  status: "active" | "blocked" | "ended"
  startedAt: string
}
```

### Notes

* `worktreeRoot` is required because it is the main routing boundary in v1
* `repoRoot` is helpful but not essential for the first cut
* `tmuxPane` is for UX/correlation, not enforcement authority
* `status` is intentionally simple

---

## 8. Change model

Again, keep it tight.

```ts
type ChangeEvent = {
  changeId: string
  observedAt: string
  path: string
  event: "create" | "modify" | "delete" | "rename"
  worktreeRoot: string
}
```

Optional enrichment:

```ts
type ChangeBatch = {
  changeId: string
  observedAt: string
  worktreeRoot: string
  paths: string[]
}
```

For v1, batching a burst of file saves into one decision unit is probably enough.

---

## 9. Rule engine

Only include very cheap deterministic rules.

### Examples

* forbidden regex pattern in changed file
* forbidden file path write
* secret pattern detection
* blocked extension/file class
* known architecture path rule

### Do not include yet

* graph recompute
* semantic ownership analysis
* transitive impact
* expensive AST analysis unless already trivial

The rule engine should return only:

```ts
type Decision = "allow" | "interrupt"
```

You can add `warn` and `block` later, but for v1 the real test is interruption.

---

## 10. Ownership mapping

This is the hardest part of v1, so keep it pragmatic.

## Primary mapping rule

Map a change to the active session whose `worktreeRoot` best matches the changed path.

That works surprisingly well if:

* each agent is usually working in one worktree
* worktree roots are distinct
* sessions are registered at launch

## Secondary hints

Use:

* `cwd`
* most recently active session in that worktree
* tmux pane metadata for operator visibility only

## Explicit limitation

If two active sessions write in the same worktree simultaneously, attribution may be ambiguous in v1.

That is acceptable for the prototype as long as it is called out.

---

## 11. Worktree blocking

After an interrupt-worthy violation:

* mark the session `blocked`
* mark the worktree `blocked`

Effects:

* current session gets interrupted
* future `anvil-run` launches in that worktree are refused until manually cleared

This gives you a practical “fence” without implementing the full lease system.

### Minimal block state

```ts
type BlockedWorktree = {
  worktreeRoot: string
  reason: string
  blockedAt: string
}
```

---

## 12. IPC design

Use **one Unix socket** for v1.

No need to split control and telemetry yet.

## Methods

### `session.register`

Register a session before launch.

### `session.attachProcess`

Attach pid/pgid after process start.

### `session.unregister`

Mark session ended.

### `session.list`

Debugging only.

### `worktree.status`

Check whether a worktree is blocked.

### `worktree.unblock`

Manual clear for testing.

### `enforcement.interrupt`

Internal daemon-to-launcher/helper command, or local daemon action.

For v1, the daemon may not even need to send this over RPC if it can perform process-group control directly from registry state.

---

## 13. Recommended flow

## 13.1 Launch flow

1. user runs `claude`
2. zsh wrapper calls `anvil-run --tool claude`
3. `anvil-run` resolves cwd/worktree
4. `anvil-run` checks with daemon whether worktree is blocked
5. if blocked, refuse launch
6. if allowed, create `session_id`
7. register session with daemon
8. launch child in new process group
9. send pid/pgid to daemon
10. session becomes active

## 13.2 Intercept flow

1. agent writes file
2. daemon watcher receives event
3. daemon coalesces event burst
4. daemon resolves worktree
5. daemon maps change to active session
6. daemon runs deterministic rules
7. if allowed, do nothing
8. if interrupt:

   * mark session blocked
   * mark worktree blocked
   * send `SIGINT` to PGID
   * escalate if needed

## 13.3 Relaunch flow

1. user attempts new launch in blocked worktree
2. `anvil-run` queries daemon
3. daemon returns blocked
4. launcher refuses with reason

---

## 14. UX expectations

Keep it minimal but visible.

### On interrupt

Print something like:

```text
Anvil interrupted session sess_123
Reason: forbidden pattern detected in src/foo.ts
Worktree blocked: /path/to/worktree
```

### On blocked relaunch

```text
Anvil refused launch in blocked worktree
Reason: prior policy violation
Run: anvil worktree unblock <path>
```

### Optional tmux UX later

* pane message
* status line marker
* pane title update

Not required for v1.

---

## 15. Failure handling

## If daemon is unavailable

Choose one of these and stick to it:

### Recommended for prototype

Fail closed for wrapped launches:

* `anvil-run` refuses to launch if daemon cannot be reached

This gives a stronger prototype signal.

## If interrupt fails

Escalate:

* SIGINT
* SIGTERM
* SIGKILL

Log which stage succeeded.

## If ownership is ambiguous

For v1:

* interrupt the session mapped to that worktree if exactly one active session matches
* otherwise log ambiguity and optionally block worktree only

Do not risk killing the wrong unrelated session outside the same worktree.

---

## 16. Constraints and limitations

This version deliberately does **not** solve:

* remote host control
* SSH-launched sessions on another machine
* hosted agent environments
* editor-native agents
* two agents actively writing in the same worktree
* sophisticated provenance reconstruction
* lease hierarchy
* driver capability negotiation
* graph-assisted policy decisions

That is fine. The job of v1 is not completeness. It is proving the loop.

---

## 17. Suggested module breakdown

## Rust daemon

* `daemon/main.rs`
* `daemon/ipc.rs`
* `daemon/session_registry.rs`
* `daemon/watch.rs`
* `daemon/rules.rs`
* `daemon/enforcement.rs`
* `daemon/worktree_state.rs`

## Launcher

* `launcher/main.rs`
* `launcher/register.rs`
* `launcher/process.rs`

## zsh glue

* `shell/anvil.zsh`

---

## 18. Implementation order

### Phase 1

Build the launcher and registry.

* `anvil-run`
* `session.register`
* pid/pgid capture
* blocked worktree check

### Phase 2

Build watch + rules + interrupt.

* file watch
* worktree mapping
* regex/path rules
* process-group interrupt ladder

### Phase 3

Stabilise UX and debugability.

* status command
* unblock command
* clearer logs
* session list
* better coalescing

### Phase 4

Optional local niceties.

* tmux pane metadata
* pane-aware messaging
* simple status indicators

---

## 19. Success test scenarios

### Scenario A

One local Claude session in one worktree writes a forbidden pattern.

Expected:

* correct process group interrupted
* worktree blocked

### Scenario B

Two different sessions in two different worktrees.

Expected:

* only offending worktree’s session interrupted

### Scenario C

User retries launch in blocked worktree.

Expected:

* launch refused

### Scenario D

Interrupt ignores SIGINT.

Expected:

* escalation to SIGTERM, then SIGKILL if needed

---

## 20. Future path after v1

If this works, the next logical steps are:

* split control and telemetry lanes
* add `warn` and explicit `block`
* add remote shell sidecar on dev servers
* formalise driver model
* introduce leases
* add editor/web drivers
* add better ownership correlation
* add graph hot-read checks where cheap

---

## 21. Summary

**Anvil Intercept Loop v1** is a deliberately narrow prototype:

* one host
* one daemon
* one launcher boundary
* shell-first ingress
* worktree-based ownership
* deterministic checks only
* process-group interruption as the core enforcement primitive

It is the fastest path to proving the hardest part of Anvil:

> not that Anvil can observe agent behaviour, but that it can **actually stop it**.
