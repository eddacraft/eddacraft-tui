# ADR-036: Daemon Scope, Discovery, and OS-Boundary Policy

## Status

Accepted (rewritten 2026-05-07; accepted 2026-05-13 during Wave 0 carry-forward
reconciliation — decisions still hold against `v0.6.2-beta` and the
daemon-working slate plan)

## Date

2026-05-07

## Context

A1 shipped with the Anvil intercept daemon as a per-user singleton, manual
foreground start, and an algorithm-derived socket path that surfaces
re-derive on their own (`plans/archive/modules/intercept-daemon.aps.md` §Purpose,
INTD-001..INTD-016). RMCP's launch slice runs **embedded** validation; the
daemon-backed `scan_buffer` path is wired but not yet announced as the
default.

This ADR began as the "per-user singleton" decision. During subsequent
planning (round-2 brainstorm,
`plans/brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md`),
ground truth from real multi-machine / multi-agent workflows surfaced
several deeper concerns that reshape the decision:

1. **Per-user singleton is too coarse.** Real setups have 3+ checkouts
   across 3+ machines for a single project (laptop + desktop + Morgan's
   machine, plus remote MCP). The PID-file singleton guard would fight
   reality, not enforce correctness.
2. **The unifying concept is "execution scope"** — a place where one
   kernel can directly observe writes via inotify-equivalent. Within
   such a scope: one daemon. Across scopes: multiple daemons, by
   design.
3. **No discovery metadata** — surfaces cannot tell whether the
   daemon at the socket is the right version, alive, or even on the
   right OS instance.
4. **No model for cross-OS boundaries** — Windows-host editor + WSL
   terminal would silently both claim "protected" while neither sees
   the other's writes through the 9P bridge. False-confidence failure.
5. **No model for two simultaneous AI agents in one worktree.**
6. **Forks** — should inherit identity / rules, or get a new project?
7. **WSL distro derivation** — naive use of `WSL_DISTRO_NAME` env var
   is attacker-controllable from the same UID context.

## Decision

### D-1 — Daemon scope: per-execution-scope (not per-user singleton)

A daemon's natural reach is the set of files visible via the kernel's
inotify-equivalent. The unit is **execution scope** — boundaries that
create a separate execution scope:

- Container boundary (Docker / OCI / Podman)
- VM / kernel boundary (each WSL2 distro; macOS virtualisation)
- Sandbox boundary (macOS App Sandbox; Flatpak; snap)
- User boundary (different UID)

Within an execution scope: **one daemon per (uid, os)** is the
singleton (PID-file exclusive create on local FS). Across scopes:
multiple daemons coexist by design, **not as a failure mode**. A
typical user with laptop + desktop + Morgan's machine + WSL distros has
4+ daemons running, all serving the same `project_id`. Coordination is
implicit via git (every daemon's witnesses converge on the same
`anvil/witnessed.ndjson` through `merge=union`); no leader election,
no inter-daemon RPC.

### D-2 — Project identity: composite, with `project_uuid` authoritative

```
ProjectIdentity := {
  project_uuid:    "01997e4a-1b2c-7345-8901-abcdef123456",     // authoritative; in anvil/project-id (tracked)
  first_commit:    "a3b2ea4e...",     // cross-check from git log
  origin_canonical: "github.com/eddacraft/anvil"  // best-effort cross-check
}
```

Match precedence: `uuid` > `first_commit` > `origin`. Mismatched
cross-checks warn but don't auto-merge. Forks inherit `project_uuid`
by default (same project; rules travel) and record `forked_from:
<parent_uuid>`. Forks may opt-out by changing `project_uuid`.

`anvil start` writes `anvil/project-id` (UUID, plain text) at adoption
time. Light-init — no prompts, no AI, no analysis.

### D-3 — Discovery: `info.json` runtime sidecar with two-phase ready

Daemon writes `<runtime_dir>/anvil/intercept.info.json` (schema
`anvil.daemon.info.v1`):

- **Phase 1 — `listen()` time:** write with `ready: false`. Surfaces
  that read this poll for `ready: true`.
- **Phase 2 — init complete:** atomically replace with `ready: true`
  after rule registry loaded, fence state restored, watcher
  subscribed, IPC accept loop running.

Subject to the lstat ladder (INTD-002). Surfaces refuse on:
mismatched token, dead pid, starttime mismatch, proto-version
incompatibility, `ready: false` past startup budget — with structured
codes (`cross-boundary-detected`, `daemon-stale`,
`proto-version-mismatch`, `daemon-locality-untrusted`,
`daemon-not-running`, `daemon-already-running`,
`daemon-not-ready`).

### D-4 — Lifecycle: `anvil intercept ensure` lazy launcher

Idempotent. Returns immediately if the daemon is up and compatible;
spawns it (detached, log to platform log path) otherwise. Called by
`DriverClient.connect()`, the MCP shim, the user (`anvil start`), and
optionally by platform supervisors (vNext: systemd user / launchd /
Task Scheduler).

**Hardened spawn:**
- Env-cleared then allowlist-restored (drops `LD_PRELOAD`,
  `DYLD_INSERT_LIBRARIES`, etc.) to defeat library-injection attacks
  from same-UID hostile peers.
- Daemon binary resolved sibling-to-CLI (`/proc/self/exe` →
  `dirname` → `<dir>/anvil` on Linux; equivalent on macOS / Windows),
  not via `PATH` search (defeats search-order hijacking).

Two-`ensure` race resolved by PID-file exclusive create on local FS;
loser exits with `daemon-already-running`, re-reads `info.json`, finds
the winner.

### D-5 — Cross-OS boundary: detect and refuse in v1

When `os_locality_token` does not match between surface and daemon
(Windows surface trying to reach a WSL daemon, or vice versa), the
surface refuses to attach with `cross-boundary-detected`. The
user-facing claim is downgraded to one of `cross-boundary-mixed`,
`pre-write-only` (with embedded fallback explanation), or
`unprotected`. v1 will **not** translate paths or bridge transports
between Windows and WSL.

Rejected for v1:
- Path translation (Windows ↔ WSL): case folding, NTFS junctions vs
  Linux symlinks, 9P inotify gaps — known footguns.
- Bridge transport: requires authentication + audit + transport spec.
- "Best-effort" cross-boundary: false-confidence is the worst failure
  mode.

vNext+: bridge driver pattern with explicit transport + audit
logging, its own ADR.

### D-6 — `os_locality_token` derivation (hardened against same-UID spoofing)

8-byte SHA-256 prefix of:

| Platform | Source string |
|---|---|
| Linux native | `linux\0<uid>\0<machine-id>` (from `/etc/machine-id`) |
| macOS | `macos\0<uid>\0<IOPlatformUUID>` |
| Windows native | `windows\0<user-sid>\0<MachineGuid>` |
| WSL distro | `wsl\0<uid>\0<host-machine-guid>\0<distro-name>` |

WSL distro-name **NOT** taken from `WSL_DISTRO_NAME` env var (user-
writable) or `/etc/wsl.conf` (root-writable inside the distro). Source
order, all kernel-controlled:

1. Read `/proc/sys/kernel/osrelease`; confirm `WSL2` substring (reject
   WSL1 with `daemon-locality-untrusted: wsl1-unsupported`).
2. Resolve distro identifier from `/proc/mounts` 9p rootfs line
   (immutable mount tag).
3. Cross-check `WSL_DISTRO_NAME` if present; mismatch logged but does
   not change the token.

Daemon computes the token **once at startup** and writes into
`info.json`. Surfaces re-derive at attach time and verify equality.

### D-7 — Identity uniqueness caveat (containerised / namespace-remapped)

`(uid, os_locality_token)` uniqueness holds **only on a non-
containerised, non-namespace-remapped local filesystem**. In rootless
Docker, user-namespace remapping, and bind-mounted dev-container
scenarios, two daemons may compute the same token and contend for the
same runtime path. v1 explicitly does not support these (§9.3 of the
spec); spec + this ADR record the caveat. Beyond the PID-file
exclusive-create guard, no further mitigation is in v1.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|---|---|---|
| **Per-execution-scope, multi-daemon** *(chosen)* | Matches real multi-machine workflows; one daemon per kernel-visible FS; clean WSL fence; minimal diff from existing INTD; multi-daemon emerges naturally rather than being a failure mode | Slightly more conceptual surface than "one daemon per user" |
| Per-user singleton (round-1 strawman) | Simplest mental model | Wrong for real workflows: 3+ checkouts across 3+ machines for one project; PID-file guard fights reality |
| Per-checkout daemon | Bounded blast radius per checkout | Multiplies parser pools / watcher trees / fence state / runtime dirs; discovery harder |
| Per-surface daemon | "Each surface owns its daemon" sounds simple | Split-brain; leader election; conflicting fences; introduces a distributed system Anvil isn't |
| Supervisor + per-project workers | Strong per-project sandboxing | Two binaries, two protocols, worker zombie reaping, parser-pool footprint; premature for v1 |

Why per-execution-scope specifically:

- **Matches kernel reality.** A daemon's natural reach is what its
  kernel's inotify can see. Defining the unit at the kernel boundary
  is honest about what one daemon can guarantee.
- **Multi-daemon is the rule, not the exception.** User's actual
  setups have 3+ daemons today; the model has to embrace this rather
  than fight it.
- **Cross-boundary fence is unmistakable.** Each WSL distro = its own
  execution scope = its own daemon = `os_locality_token` mismatch on
  cross-boundary attach attempt.
- **Lowest diff from current INTD.** Existing INTD-001..INTD-016
  implement the per-(uid, os) singleton; this ADR just adds the
  multi-daemon coordination surface (which is mostly "nothing"
  because git is the rendezvous).

### Why forks inherit project_uuid by default

- **Rules travel with the repo.** A fork of `anvil` is still "anvil"
  unless explicitly renamed; rules from upstream apply by default.
- **Implicit governance domain.** Project_uuid defines a domain;
  forks stay in the domain unless they leave.
- **Opt-out is available.** Changing `project_uuid` (with
  `forked_from` recorded) is the explicit signal "this is now a
  separate project."

## Consequences

- **Positive — Multi-machine / multi-agent workflows just work.**
  3 daemons + remote MCP for one project is supported, not
  pathological.
- **Positive — Cross-OS boundaries are honest.** Cross-WSL/Windows
  attach refused; `cross-boundary-mixed` state names the situation.
- **Positive — Forward-compatible identity.** Composite
  `ProjectIdentity` admits `forked_from` extensions; future composite
  fields default-null.
- **Positive — Wow-start preserved.** `anvil start` light-init writes
  the UUID; users don't need to know about identity machinery.
- **Negative — More conceptual surface.** "Execution scope" is a new
  term; documentation needs to introduce it.
- **Negative — App Sandbox on macOS unprotected at daemon level in
  v1.** Embedded fallback only; no editor-driver participation for
  sandboxed editors until vNext.
- **Negative — Containerised / namespace-remapped scenarios out of
  v1.** Documented as unsupported.
- **Risk — `ensure` race / spawn-failure handling.** Mitigated by
  bounded poll, PID-file exclusive create, structured failure codes,
  spawn_log_path, and tests.
- **Risk — Stale `info.json` after hard kill.** Mitigated by per-
  platform starttime check (Linux: `/proc/<pid>/stat[22]`; macOS:
  `proc_pidinfo`; Windows: `GetProcessTimes`) and `anvil doctor
  --reap`.
- **Risk — WSL distro spoofing from same-UID context.** Mitigated by
  D-6 — `WSL_DISTRO_NAME` is advisory only; kernel-controlled sources
  (`/proc/sys/kernel/osrelease` + `/proc/mounts` 9p tag) are
  authoritative.

## References

- **Round-1 brainstorm:** [`2026-05-07-daemon-sessions-surfaces-boundaries.md`](../brainstorms/2026-05-07-daemon-sessions-surfaces-boundaries.md)
- **Round-2 brainstorm:** [`2026-05-07-anvil-multilayer-protection-brainstorm.md`](../brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md)
- **Spec (consolidated):** [`2026-05-07-anvil-multilayer-protection-architecture.md`](../specs/2026-05-07-anvil-multilayer-protection-architecture.md)
- **Spec (round-1, superseded):** [`2026-05-07-daemon-lifecycle-and-discovery.md`](../specs/2026-05-07-daemon-lifecycle-and-discovery.md)
- **Companion ADRs (split per planning direction):**
  - ADR-037 — Witness chain + L4 policy framework
  - ADR-038 — Hook surface + noise discipline
  - ADR-039 — Baseline policy + hard-pinned rule classes
- **APS modules:**
  - `plans/archive/modules/intercept-daemon.aps.md` (parent — INTD)
  - `plans/archive/modules/daemon-lifecycle.aps.md` (DLIFE — partial scope; some work items consolidated into MLP)
  - `plans/archive/modules/multilayer-protection.aps.md` (MLP — primary v1 module)
- **Related ADRs:**
  - ADR-015 — Intercept loop enforcement (parent of INTD)
  - ADR-030 — Surface drivers supersede napi cutover (parent of DRVR)
  - ADR-031 — Validation latency rubric (owns `ensure` startup budget)
  - ADR-033 — Park IDE MCP / retire TS scanner
- **External:**
  - JSON-RPC 2.0 (pinned by INTD-014)
  - [`editor-and-mcp-driver-design.md`](../specs/anvil-driver-framework/editor-and-mcp-driver-design.md) §2
  - [`2026-05-06-editor-driver-protocol.md`](../specs/2026-05-06-editor-driver-protocol.md)
