# Architecture: Intercept Loop Enforcement

| Field | Value |
|-------|-------|
| Status | Accepted (ratified 2026-07-04 via planning council plan-18c47503 / ADR-097 bookkeeping; all constituent ADs were individually Accepted and shipped) |
| Planning Council | plan-ba94d8a5 |
| Prior Review | council-3695036e (18 findings: 7 critical, 11 major) |
| Date | 2026-04-02 |
| Participants | architect, pragmatic-lead, adversarial-reviewer, security-analyst, operations-reviewer |

## Problem Statement

Implement the Anvil Intercept Loop: a Rust daemon that detects file changes from
AI agent sessions, evaluates deterministic policy rules, and interrupts the
correct session via process-group control. Shell-first, single-host initially,
proving the core enforcement thesis. Evolves into a driver-based, host-aware
enforcement control plane.

## Constraints

The following were stated as non-negotiable during interrogation:

- **New crate(s), not subcommands** -- the intercept loop is a distinct concern
  from the existing CLI
- **Concurrency model** -- tokio for the daemon outer shell (UDS listener,
  signals, heartbeat timers); std threads for watcher and rule evaluation; single
  bridge point at a channel boundary
- **Windows support required** -- IPC (named pipes), process control (Job
  Objects), PGID verification (process snapshots), and socket permissions (ACLs)
  must all have Windows codepaths alongside Linux and macOS
- **One focused engineer** -- plan assumes serial execution by a single developer
- **Configurable enforcement, never kill ambiguously** -- on_ambiguous_ownership
  hard-capped at fence; interrupt only when attribution is certain
- **Fence persistence** -- blocked-worktree state must survive daemon restarts
- **Fail closed for wrapped launches** -- anvil-run refuses to launch if daemon
  is unreachable

## Architecture Decisions

### AD-1: Process-Group Management

- **Context:** The daemon must be able to interrupt specific agent sessions
  without affecting other processes in the same terminal. Process groups provide
  targeted signal delivery, but the interaction with daemon lifecycle and
  terminal multiplexers must be well-defined.
- **Decision:** Use `setpgid(child_pid, child_pid)` for agent processes launched
  via the anvil-run wrapper. Reserve `setsid()` exclusively for daemon
  daemonisation. On Windows, use Job Objects instead of process groups.
- **Rationale:** Preserves Ctrl-C routing in interactive shells, works cleanly
  in tmux panes, gives the daemon a unique PGID target per session. Job Objects
  are the Windows-native equivalent with similar isolation semantics.
- **Alternatives Considered:**
  - `setsid()` for all child processes -- rejected because it breaks Ctrl-C
    routing and terminal interaction
  - Rely on PID-only targeting -- rejected because child processes spawned by
    the agent would escape interrupt
- **Status:** Accepted

### AD-2: Unregistered Session Detection

- **Context:** Not all agents will be launched through anvil-run. The daemon
  must handle file changes from unattributed sources without incorrectly killing
  unrelated processes.
- **Decision:** Use hook side-channel as the primary registration mechanism
  (Claude Code PreToolUse hook calls `daemon register-session`).
  Worktree-fence-on-unknown as fallback for all unattributed changes, tagged
  `attribution:unknown-agent`. Skip /proc fd reverse-lookup (timing unreliable).
  Defer ambient process sentinel scanner to v2.
- **Rationale:** Hook-based registration is lightweight, composable with
  existing Claude Code hooks, and avoids the timing races inherent in /proc
  scanning. Fence-on-unknown is the safe default -- it prevents damage without
  risking incorrect process kills.
- **Alternatives Considered:**
  - /proc fd reverse-lookup for process attribution -- rejected due to
    unreliable timing (TOCTOU)
  - Ambient process sentinel scanner -- deferred to v2, complexity too high
    for initial proof
  - Ignore unregistered sessions entirely -- rejected as it creates a bypass
    path
- **Status:** Accepted

### AD-3: Configurable Enforcement Policy

- **Context:** Different projects and teams need different enforcement
  strictness. A secret-detection interrupt is appropriate for some teams but
  excessive for others. The system needs a configuration surface without
  per-rule granularity in v1.
- **Decision:** Project-level `.anvil.yaml` with enforcement block containing
  `mode` (warn | fence | interrupt), `on_ambiguous_ownership` (warn | fence),
  and `observe_only` (boolean dry-run). Daemon takes the stricter of project
  and user config. No per-rule granularity in v1. Ambiguous ownership
  hard-capped at fence as a code invariant.
- **Rationale:** Three-tier enforcement mode covers the spectrum from
  observation to active intervention. The "stricter wins" merge rule prevents
  accidental weakening. Hard-capping ambiguous ownership at fence is a safety
  invariant -- the system must never interrupt a process it cannot confidently
  attribute.
- **Alternatives Considered:**
  - Per-rule enforcement granularity -- deferred to v2 to keep configuration
    surface small
  - User config only (no project-level) -- rejected because enforcement
    policy should be committed to the repository
  - No observe-only mode -- rejected because teams need a safe rollout path
- **Status:** Accepted

### AD-4: IPC Transport and Wire Format

- **Context:** The daemon needs a lightweight, cross-platform IPC mechanism
  for session registration, heartbeats, and enforcement commands.
- **Decision:** NDJSON over Unix domain sockets (Linux/macOS) and named pipes
  (Windows). Socket path uses `XDG_RUNTIME_DIR` or platform equivalent.
  User-owned directory with restricted permissions (0700 on Unix, ACLs on
  Windows). Single socket for v1 (control and telemetry combined).
- **Rationale:** NDJSON is trivially parseable in Rust, debuggable with
  standard tools, and avoids schema overhead. UDS/named pipes are the
  fastest local IPC available. Platform-specific socket paths follow OS
  conventions for ephemeral user state.
- **Alternatives Considered:**
  - JSON-RPC 2.0 -- full protocol overhead not justified for v1's simple
    command set; can layer on later
  - gRPC -- rejected per ADR in design spec; local control plane does not
    need service-mesh complexity
  - Split control/telemetry lanes -- deferred to v2 per design spec phasing
- **Amendment (2026-05-06, INTD-016):** AD-4's "single socket for v1" stance
  is unchanged, but the wire posture now ships explicit DoS budgets:
  - Connection cap: 64 simultaneous driver connections (default), enforced at
    the listener via `tokio::sync::Semaphore`. Over-cap connections are
    dropped at accept.
  - Per-connection RPS: 100 sustained / 1000 burst (default), enforced via a
    per-connection token bucket. **Bucket exhaustion returns a structured
    `-32005 Server busy` JSON-RPC error and KEEPS the connection open** —
    closing on rate-limit would cause innocent retries to escalate.
  - Handshake timeout: 5 s from `accept()` to first NDJSON line. Slow-loris
    peers are dropped with the standard idle-timeout log.
  - Idle timeout: 60 s (matches the existing per-read deadline). Separate
    from the session heartbeat TTL.
  - Control-lane NDJSON frame cap: 64 KiB (default) for non-`scan_buffer`
    methods. The 1 MiB scan_buffer payload cap survives untouched. Frame
    size is enforced **before parsing** so a maliciously-shaped payload
    cannot consume parser stack.
  - **Plaintext-local-only TLS stance:** v1 IPC stays plaintext over the
    owner-only UDS / named pipe. TLS does not arrive until a remote-shell
    driver is in scope; until then, the trust boundary is the per-user
    socket / pipe ACL. This is recorded explicitly so a future maintainer
    does not silently bolt TLS onto a transport whose ACL already enforces
    per-user isolation.

  Limits are loaded from `enforcement.dos.*` in `.anvil.yaml` per INTD-008's
  reserved keys; project + user merge picks the **stricter** value per field.
  The `IpcLimits` struct surfaces the effective values to operator-visible
  status surfaces (INTD-011). See `crates/anvil-intercept/src/dos.rs` for
  the implementation.
- **Status:** Accepted (amended 2026-05-06 for INTD-016)

### AD-5: Daemon Lifecycle Model

- **Context:** The daemon must be a reliable singleton that starts, stops,
  and restarts cleanly without orphaned state.
- **Decision:** Per-user persistent singleton daemon. PID file for lifecycle
  management. Explicit start/stop/restart commands. Fence state persisted to
  disk so it survives daemon restarts. Embedded mode (in-process, no socket)
  for CI environments.
- **Rationale:** A persistent daemon avoids cold-start latency on every
  file change. PID file prevents double-start races. Disk-persisted fence
  state ensures that a daemon crash does not silently unblock a fenced
  worktree. Embedded mode keeps CI simple without requiring socket setup.
- **Alternatives Considered:**
  - On-demand daemon (start per session, exit when idle) -- rejected because
    cold-start latency defeats the purpose of fast interception
  - Systemd/launchd service management -- deferred; PID file is simpler for
    initial proof and works cross-platform
- **Status:** Accepted

### AD-6: Rule Integration Strategy

- **Context:** The daemon needs fast deterministic checks. The existing
  anvil-checks crate already implements secret detection and antipattern
  scanning.
- **Decision:** Reuse anvil-checks via a thin InterceptRule trait that wraps
  existing secret and antipattern checks. Add PathDenyList and RegexContent
  as new rule types. Rules output a binary allow | interrupt signal. The
  enforcement pipeline maps interrupt signals through the configured
  enforcement mode: in warn mode, interrupts become logged warnings; in fence
  mode, they become worktree fences; in interrupt mode, they trigger the
  process-group signal ladder. Full four-level rule output deferred to v2.
- **Rationale:** Wrapping existing checks avoids duplication and proves the
  integration path. Binary rule output keeps the rule contract simple, while
  the enforcement pipeline provides the configurable response gradient
  defined in AD-3.
- **Alternatives Considered:**
  - Full four-level decision model (allow/warn/block/interrupt) from day one
    -- deferred to reduce v1 scope per C-001 finding
  - New rule implementations from scratch -- rejected; existing checks are
    tested and benchmarked
- **Status:** Accepted

### AD-7: PGID Verification Before Signal Delivery

- **Context:** The council review (C-002) identified a TOCTOU race between
  looking up a PGID from the session registry and actually signalling it.
  The process group may have exited or been reassigned.
- **Decision:** Verify PGID ownership via /proc (Linux), sysctl (macOS), or
  process snapshot APIs (Windows) immediately before signalling. If
  verification fails, fall back to worktree fence instead of signalling.
  Fence immediately on any signal delivery failure, then escalate.
- **Rationale:** Defence in depth -- verification reduces the TOCTOU window.
  Falling back to fence on verification failure ensures safety even when
  process-group state is stale.
- **Alternatives Considered:**
  - Signal without verification -- rejected due to C-002 TOCTOU risk
  - Double-fork to isolated PID namespace -- rejected as over-engineered
    for v1
- **Status:** Accepted

## Open Questions

1. **Lease model timing** -- when should full session leases (with expiry,
   renewal, and revocation) replace the simple active/blocked/ended status?
   The design spec defines leases but v1 uses a simpler model.
2. **Multi-worktree attribution** -- how should the daemon handle two
   registered sessions writing to the same worktree? v1 constrains to single
   session per worktree; the long-term solution is unclear.
3. **MCP PreToolUse integration depth** -- acknowledged as a stretch goal.
   If implemented, should it replace or supplement the hook side-channel?
4. **Graph-assisted hot-path checks** -- the design spec describes Tier 1
   graph reads (boundary membership, symbol ownership) as hot-path eligible.
   When should these be added?
5. **Remote host sidecar model** -- the design spec describes remote
   enforcement. How should the daemon discover and authenticate remote
   enforcement points?
6. **Daemon auto-start** -- should shell integration automatically start the
   daemon if not running, or require explicit `anvil daemon start`?
7. **Git operations false positives** (C-015) -- should the watcher apply
   .git-aware filtering to avoid triggering rules on git-internal writes
   (e.g. rebase, merge, stash)?

## Risks

| Risk | Severity | Source | Mitigation |
|------|----------|--------|------------|
| PGID TOCTOU race allows signalling wrong process | Critical | C-002, security-analyst | AD-7: verify PGID ownership before signalling; fall back to fence |
| Signal delivery fails silently (process ignores SIGINT) | High | C-003, adversarial-reviewer | Explicit timeout ladder (SIGINT -> SIGTERM -> SIGKILL); fence immediately on failure |
| Fence state lost on daemon crash | High | C-004, operations-reviewer | AD-5: persist fence state to disk; reload on restart |
| Ownership attribution fails in shared worktrees | High | C-005, adversarial-reviewer | Single session per worktree constraint in v1; fence-on-ambiguous invariant |
| Shell wrapper is bypassable (agent launched without anvil-run) | Medium | C-006, security-analyst | AD-2: hook side-channel + fence-on-unknown fallback |
| Socket/pipe permissions allow unauthorised session registration | Medium | security-analyst | User-owned directory with restricted permissions (0700 / ACL) |
| Daemon unavailable leaves agents unmonitored | Medium | C-007, operations-reviewer | AD-5: fail closed for wrapped launches; fence-on-unknown for hook-registered sessions |
| Windows platform parity gaps | Medium | pragmatic-lead | Explicit platform abstraction layer; Windows in CI matrix from the start |
| v1 scope creep beyond single-host proof | Medium | C-001, pragmatic-lead | Strict out-of-scope list; binary allow/interrupt decision; no driver framework in v1 |

## Finding Disposition

Maps all 18 findings from council-3695036e to their resolution.

| Finding | Summary | Disposition |
|---------|---------|-------------|
| C-001 | v1 scope too large | Addressed -- new crate approach limits surface; incremental module delivery |
| C-002 | PGID TOCTOU race | Addressed by AD-7 |
| C-003 | Signal delivery unreliable | Addressed -- explicit timeout ladder in INTD-006; fence immediately |
| C-004 | No state persistence | Addressed by AD-5; INTD-007 |
| C-005 | Ownership attribution | Addressed -- single session per worktree; fence on ambiguous |
| C-006 | Wrapper bypass | Addressed by AD-2; INTL-007 hook side-channel |
| C-007 | No daemon lifecycle | Addressed by AD-5; INTD-001 |
| C-008 | Concurrency model | Addressed -- tokio outer shell, std threads for watcher; ADR constraint |
| C-009 | Driver composition | Deferred to v2 -- v1 has no driver taxonomy |
| C-010 | Kernel migration seam | Addressed -- fan out from ChangeBatch channel; INTD-004 |
| C-011 | Registration race | Addressed -- atomic registration in INTL-003/INTL-004 |
| C-012 | Wire framing | Addressed by AD-4 (NDJSON) |
| C-013 | MCP dismissal | Acknowledged -- MCP hook is stretch goal per Q-012 |
| C-014 | Full spec premature | Addressed -- full design spec archived as vision; v1 scoped to 3 modules |
| C-015 | Git operations false positives | Deferred -- .git-aware filtering noted as open question |
| C-016 | Enforcement ladder inconsistency | Addressed by AD-3 and AD-6 clarification |
| C-017 | No CI/CD guidance | Addressed -- embedded mode for CI in INTD-009 |
| C-018 | Socket path unspecified | Addressed by AD-4 |
