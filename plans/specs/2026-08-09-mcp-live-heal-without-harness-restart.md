# MCP live-heal without harness restart

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for MCPLH design | [MCPLH](../modules/mcp-live-heal.aps.md) | Accepted | 2026-08-09 — Ready wave MCPLH-001..006 filed |

| Upstream | Downstream |
| -------- | ---------- |
| Field evidence (multi-session Grok/Claude/Codex MCP skew after brew upgrade), [CIB-242](../modules/continuous-improvement-backlog.aps.md) (status skew hint; no auto-kill), [MCPX](../modules/mcp-client-expansion.aps.md), [MCP26](../modules/mcp-dual-era-support.aps.md), [ADR-083](../decisions/083-gctx-mcp-delivery-target.md), [bare ensure](./2026-08-01-bare-anvil-ensure.md) | [MCPLH](../modules/mcp-live-heal.aps.md) Ready items 001..006; optional ADR if re-exec becomes cross-cutting beyond CLI MCP |

**Execution authority** is the MCPLH work items (Ready). This document is the
design contract those items implement. It does not itself authorise merge of
product code without the usual branch → validation → Council → PR path per
item.

## 1. Problem

Developer acceleration (MCP graph tools + pre-write validation) is **shipped**,
but on multi-harness developer machines it is often **unavailable in practice**.

Observed operator machine (2026-08-08, `anvil-001` worktree):

| Observation | Implication |
| ----------- | ----------- |
| CLI `0.9.3-beta`, daemon was `0.9.1-beta` for ~5 days | Upgrade left the Anvil-owned daemon stale |
| ~40 live `anvil mcp serve --stdio` children | Bulk of agent attach is long-lived, not one-shot |
| ~35 children still on Cellar `0.9.2-beta` | Absolute install paths pin dead binaries after brew |
| Parents were many long-lived `grok`, plus `claude` / `codex` | Skew is per-session, not one global “editor” |
| Config rewrite alone left sessions on old MCP | Live children keep the old image until death or re-exec |
| Full graph scan hit 60s budget (`scan-timeout`) | Graph `not_ready` can persist even when MCP is current |

**Hard constraint:** restarting agent sessions (Grok Build, Claude Code, Codex,
Cursor, …) is expensive. Parallel sessions hold context and open work. A bulk
refresh design that ends in “fully quit and reopen N parents” will not be used
and will not make agent-ready reliable.

## 2. Product principle

> **Anvil must refresh its own MCP workers under a live harness stdio pipe.
> Asking humans to restart agent sessions is a residual failure mode, not the
> bulk path.**

Success criterion for a host after upgrade or repair:

```text
daemon_version == cli_version
AND skewed_mcp_children == 0   (or all marked healing → current after one call)
AND activation claim protecting (or honest demotion with recovery)
```

Not success:

```text
mcp install … ok
```

alone.

## 3. Goals

1. **Live-heal** long-lived `mcp serve` processes to the preferred binary
   **without** ending the parent harness conversation.
2. **Bulk config rewrite** of Anvil-owned client entries to PATH-stable
   `command: anvil` (never Cellar absolute paths by default).
3. **Daemon recycle** when CLI and daemon versions diverge — Anvil-owned; no
   harness restart.
4. **Honest inventory and proof** so operators and agents see config / daemon /
   live MCP / graph as separate layers.
5. Preserve CIB-242 safety: **no silent auto-kill** of MCP children belonging to
   live foreign sessions.

## 4. Non-goals

- Auto-killing harness parents or their MCP children without explicit opt-in.
- Treating long-lived process retention as a packaging defect (CIB-242).
- Graph large-repo progressive warm (related reliability; separate design).
- LSP productisation or LSPNAV (agents use MCP; LSP is a different surface).
- Client-specific private APIs to “reload MCP” inside each vendor product
  (nice if documented; not required for v1).
- Multi-host fleet orchestration (JSON report shape only; no SSH fan-out).
- Changing MCP protocol versions or dual-era behaviour (MCP26 owns that).

## 5. Layer model

Staleness is independent per layer. Refresh must name which layer it heals.

```text
┌─────────────────────────────────────────────────────────────┐
│  Harness parent (grok / claude / codex / cursor / …)        │
│  Owns conversation state — restart is HARD                  │
│         │ stdio pipe                                        │
│         ▼                                                   │
│  MCP child: anvil mcp serve --stdio                         │
│  Owns tool catalogue + framing — MUST live-heal             │
│         │ RPC / local logic                                 │
│         ▼                                                   │
│  Intercept daemon (one per user install root)               │
│  Owns graph + scan_buffer — Anvil may recycle freely        │
│         │                                                   │
│         ▼                                                   │
│  Graph readiness (warm / not_ready / stale{reason})         │
│  Large-repo concern — out of v1 live-heal core              │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Owner of lifecycle | Bulk heal without session restart? |
| ----- | ------------------ | ---------------------------------- |
| Client config files | Anvil installer | Yes — rewrite on disk |
| Daemon | Anvil | Yes — stop / start |
| MCP child process | Harness parent process | **Yes only via re-exec or supervisor** |
| Harness session | Human / product | No — residual only |
| Graph warm | Daemon | Partial (daemon recycle may re-warm; budget is separate) |

## 6. Preferred binary resolution

Live-heal and install rewrite share one resolution rule.

**Default preferred command for all managed MCP entries:**

```text
command = "anvil"
args    = ["mcp", "serve", "--stdio"]
```

Resolve the preferred **executable** for re-exec checks as:

1. Explicit override: `--command` / install-time override / documented side-by-side
   layout under non-default `ANVIL_HOME` when that is the operator intent.
2. Else: first `anvil` on `PATH` (canonicalised when possible).
3. Do **not** treat a Cellar (or equivalent versioned install tree) path as
   preferred merely because `current_exe()` points there after a brew rename-swap.

**Skew** exists when any of:

- Running process version string ≠ preferred binary version string; or
- Canonical `current_exe()` is under a versioned install prefix that is not the
  same as the preferred binary’s prefix; or
- Preferred binary path is readable and its identity (inode/mtime/content hash
  policy TBD in implementation) differs from the running image.

Exact identity probe is an implementation choice; the **contract** is: after
heal, a new process started the same way as managed install would start matches
`anvil --version` on PATH (or the explicit override).

## 7. Live-heal: re-exec contract (v1)

### 7.1 Mechanism

A long-lived `anvil mcp serve --stdio` process that detects skew **re-execs**
into the preferred binary with the same argv shape:

```text
execve(preferred_anvil, ["anvil", "mcp", "serve", "--stdio", …], env)
```

Requirements:

- Re-exec only when the JSON-RPC framing layer is **between messages** (not
  mid-frame, not mid-tool handler after partial stdout write).
- Prefer re-exec **before dispatching** a new `tools/list` or `tools/call` once
  skew is known.
- Keep stdin / stdout / stderr FDs so the harness pipe remains valid.
- Set an anti-loop environment marker, e.g. `ANVIL_MCP_REEXECED=1`, cleared only
  by a successful “we are preferred” check on the new image. At most **one**
  re-exec attempt per process lifetime unless the marker is absent and skew
  appears again after an operator-driven preferred-binary change (generation
  bump — §9).

### 7.2 When to check

| Trigger | Required in v1? | Notes |
| ------- | --------------- | ----- |
| Start of `tools/list` | Yes | Cheap; heals catalogue skew |
| Start of `tools/call` | Yes | Heals mid-session after upgrade |
| `initialize` | Yes | New attaches start current |
| Idle timer only | No | Insufficient alone |
| Every read of a generation file (§9) | Yes if refresh poke ships | Bulk operator path |

Kill-switch: `ANVIL_MCP_NO_REEXEC=1` (or equivalent) disables re-exec; process
stays on its image and surfaces honest skew in status/tool metadata.

### 7.3 In-flight calls

- If skew is detected at the start of a call: re-exec **before** handling; the
  parent may retry the call, or the new image handles it if the client
  retransmits. Prefer documenting one retry for agents.
- Never re-exec after partial JSON-RPC output for the current response.
- Daemon connections used for a call are short-lived today (fresh connect per
  call pattern elsewhere); re-exec does not need to migrate a long-lived daemon
  socket in v1.

### 7.4 Agent-visible honesty

While healing or when re-exec is disabled and skew remains:

| Situation | Suggested outcome shape |
| --------- | ----------------------- |
| About to re-exec / just re-execing | Prefer silent re-exec between calls; if a response is required, a structured `upgrading` / retry hint beats bare `unavailable` |
| Re-exec failed or disabled | `unavailable` or tool-level warning with `recovery_hint` that does **not** lead with “restart your editor” |
| Daemon skew only | Existing daemon restart guidance; MCP binary may still be fine |

`anvil_status` (or equivalent) SHOULD expose:

```text
cli_version
mcp_binary_version   # this process
mcp_preferred_version
mcp_skew: boolean
daemon_version / daemon_skew
```

so agents can self-diagnose without folklore.

### 7.5 Platform notes

- **Unix:** `execve` on the same FDs is the primary design.
- **Windows:** re-exec over inherited stdio must be feature-tested. If unsafe,
  Windows may demote to “report skew + residual restart guidance” until a
  supervisor (§8) lands. Do not block Unix ship on Windows parity if demotion
  is honest.

### 7.6 Failure modes and mitigations

| Risk | Mitigation |
| ---- | ---------- |
| Re-exec loop | Anti-loop env; compare version/identity not only path string |
| Client treats re-exec as crash | Between-message only; optional one structured retry |
| Deleted Cellar path still in `current_exe` | Resolve preferred via PATH / install layout, not only `current_exe` |
| Absolute path in config | Bulk rewrite to bare `anvil` (§10) so preferred resolution stays stable |
| Re-exec to wrong binary (shadow on PATH) | Reuse existing shadow-detection ideas from version/doctor surfaces where applicable |

## 8. Supervisor / proxy (v2 fallback)

If real clients drop the pipe on re-exec, introduce a **stable supervisor**:

```text
harness ──stdio──► anvil mcp serve (supervisor)
                      │
                      └── spawns worker serve as child; restarts worker on skew
```

Properties:

- Config continues to advertise `anvil mcp serve --stdio` (no client churn).
- Supervisor resolves preferred binary on **each** worker spawn.
- Optional JSON-RPC queue across one worker restart (or single retryable error).
- Higher cost (framing, Windows, tests). **Not** v1 unless re-exec fails in
  soak against Grok / Claude Code / Codex / Cursor.

Decision rule: implement re-exec first; promote supervisor only with evidence
that parents tear down on re-exec.

## 9. Operator bulk poke: refresh generation

### 9.1 Intent

Operators need one verb that heals **config + daemon + signals live MCP** without
walking session UIs.

Illustrative CLI (names may change in APS):

```text
anvil mcp refresh [--dry-run] [--json]
                  [--clients all|detected|id…]
                  [--daemon restart|reuse|auto]
                  [--processes report|orphan-reap|none]
```

### 9.2 Cascade (default)

```text
1. CONFIG   Rewrite Anvil-owned entries → preferred command `anvil`
2. DAEMON   If auto and version skew (or attestation dead): stop → wait → ensure
3. SIGNAL   Bump refresh generation (and/or best-effort cooperative signal)
4. REPORT   Inventory live mcp serve by parent; skewed / healing / current / orphan
5. PROOF    Pointer to start --verify / agent_ready fields
```

Live children that implement §7 re-exec on next tools/list or tools/call after
the generation bump — **no harness restart**.

### 9.3 Generation file

- Location: under install / runtime root (e.g. user `ANVIL_HOME` or XDG runtime),
  **not** project `.anvil/` (refresh is host/install scoped).
- Semantics: monotonic counter or timestamp; serve processes treat “generation >
  last_seen” as “re-check preferred binary and re-exec if skewed.”
- `anvil mcp refresh` bumps generation after config rewrite so even processes
  that only poll generation still heal without PATH install alone.

### 9.4 Process policy ladder

| Mode | Behaviour | Default? |
| ---- | --------- | -------- |
| `report` | List skewed/current/orphan grouped by parent command + PID | Yes |
| `orphan-reap` | SIGTERM only if parent PID is gone (same-user, identified as anvil mcp serve) | Opt-in |
| `force-skewed` | Kill skewed children of live parents | **Forbidden as default**; if ever offered, explicit flag + confirm; document that many parents will not respawn until session restart — last resort, not bulk path |
| Self-heal re-exec | Inside serve | Always-on unless kill-switch |

Aligns with CIB-242: visibility and guidance yes; silent mass-kill no.

### 9.5 Proof surface

Extend verify/status (conceptually) with:

```text
mcp_configs: [{ client, path, command, drifted, action }]
daemon: { version, cli_version, skew, action }
mcp_processes: {
  total, skewed, current, orphan,
  by_parent: [{ command, parent_pids, skewed_children }]
}
graph: { state: ready|warming|stale|unavailable, reason? }  # honest; may remain not_ready
agent_ready: boolean | { ok, blockers[] }
```

`agent_ready` is false while skewed MCP remain **and** they have not yet had a
chance to heal (e.g. no tool call since generation bump). After heal, skewed
should be zero without session restart.

## 10. Config bulk rewrite

### 10.1 Scope

Every Anvil-owned MCP entry the installer knows how to manage:

- Global clients (Claude Code, Cursor, Grok, Codex, … per registry).
- Project-scoped clients where Anvil wrote the entry (VS Code / Zed patterns).
- Only **owned** or canonical-shaped Anvil entries; leave third-party servers
  untouched.

### 10.2 Rewrite rules

- Default write: `command: anvil`, args `mcp serve --stdio` (+ type discriminator
  where the client requires it).
- Idempotent: no mtime churn when already correct (existing start tests already
  guard this class of behaviour).
- Detect “drifted” when command is an absolute versioned install path, wrong
  args, or missing type field for clients that need it.
- `--command` remains for air-gapped / side-by-side candidates.

### 10.3 Relationship to existing flags

| Existing | Role after this design |
| -------- | ---------------------- |
| `anvil mcp install --client` | Single-client write; should share preferred-command rules |
| `anvil start --all-mcp-clients` / `--mcp-client` | Install/reconfigure; not a live-heal |
| bare `anvil` ensure | Daemon + presence; should not be the only bulk rewrite path |
| `anvil mcp refresh` (proposed) | Bulk rewrite + daemon + generation + report |

## 11. Residual: when session restart is still required

Only when:

1. Re-exec (or supervisor worker restart) **failed** or is kill-switched; or
2. The harness tears down stdio on child image change (evidence-driven); or
3. The client never invokes MCP again (no call → no heal trigger) and the
   operator needs tools immediately without waiting — then “reconnect MCP” or
   session restart for **that** parent only.

Copy must lead with:

```text
Anvil tried to recycle MCP in place. This session still runs <old>.
Reconnect MCP for this client, or retry a tool call after: anvil mcp refresh
```

not “restart all your agents.”

## 12. Relationship to graph readiness

Live-heal fixes **binary and attach** skew. It does **not** fix:

- full-scan `scan-timeout` on huge worktrees;
- eternal GCTX `not_ready` / warming;
- unclassified file flood.

Those need a separate large-repo graph design (progressive ready, budget
honesty, ignore policy). Status must not claim `agent_ready` solely because MCP
binary matches if graph tools are the operator’s success bar — either:

- define `agent_ready` as pre-write attach only, and `graph_ready` separately; or
- document composite readiness with clear blockers.

Recommendation: **split claims** so live-heal can ship without waiting on graph
budget work.

## 13. Security and trust

- Re-exec must only target the preferred Anvil binary resolution (§6), never an
  arbitrary path from untrusted project config.
- Generation file must be user-owned runtime state with same trust posture as
  daemon socket/PID files.
- Orphan reap and any force-kill remain same-user and shape-checked (command
  line matches anvil mcp serve).
- No elevation; no killing other users’ processes.

## 14. Phased delivery (APS — Ready)

Filed as exclusive module [MCPLH](../modules/mcp-live-heal.aps.md) (2026-08-09).

| Slice | Work item | Outcome | Session restart needed? |
| ----- | --------- | ------- | ----------------------- |
| **A** | **MCPLH-001 Ready** | PATH-stable install: never write Cellar/versioned absolute by default | No (prevents new skew) |
| **B** | **MCPLH-002 Ready** | Self-heal re-exec in `mcp serve` + kill-switch + anti-loop | No (primary heal) |
| **C** | **MCPLH-003 Ready** | `anvil mcp refresh`: config bulk rewrite + generation bump + report | No |
| **D** | **MCPLH-004 Ready** | Daemon auto-recycle on skew inside refresh / ensure | No |
| **E** | **MCPLH-005 Ready** | status/verify: mcp process inventory + `mcp_skew` + split ready claims | No |
| **F** | **MCPLH-006 Ready** | orphan-reap opt-in | No |
| **G** | MCPLH-007 Draft | Supervisor only if re-exec fails soak | No |
| **H** | Out of module | Large-repo graph warm (separate design) | No |
| Residual | Messaging in 002/005 | Per-parent reconnect only on heal failure | Only on heal failure |

CIB-242 remains the **visibility** precedent; this design **extends** it with
heal mechanics rather than replacing its non-kill rule.

## 15. Acceptance scenarios (design-level)

1. **Brew upgrade while 12 Grok sessions run**  
   Operator runs `anvil mcp refresh` (or children self-check on next tool call).  
   Daemon matches CLI. Configs point at `anvil`. Within one tools/call per
   session, `mcp_binary_version` matches CLI. Conversations intact.

2. **Config pinned to old Cellar path**  
   Refresh rewrites to `anvil`. Next re-exec uses PATH. No full quit.

3. **Orphan mcp serve (parent dead)**  
   Optional orphan-reap removes them. Live parents untouched.

4. **Re-exec disabled**  
   Status shows skew; recovery does not claim protecting-with-current-tools
   falsely.

5. **Graph still warming**  
   MCP binary current; `graph_ready` false with `scan-timeout` reason; pre-write
   may still be protecting.

## 16. Open questions

| # | Question | Default if undecided |
| - | -------- | -------------------- |
| OQ-1 | Re-exec always-on vs opt-in for first release? | Always-on with kill-switch |
| OQ-2 | Identity probe: version string only vs inode/mtime/hash? | Version string + preferred path mismatch |
| OQ-3 | Should bare `anvil` bump generation on ensure? | Yes if daemon or CLI changed since last bump |
| OQ-4 | `agent_ready` include graph? | No — split `graph_ready` |
| OQ-5 | Windows v1 demote vs block? | Demote honestly; ship Unix heal |
| OQ-6 | New ADR vs module note only? | ADR if re-exec becomes cross-cutting contract; else APS + this spec |

## 17. Decision summary

| Decision | Choice |
| -------- | ------ |
| Bulk path | Config rewrite + daemon recycle + **in-process MCP re-exec** |
| Not bulk path | Restart all agent sessions |
| Default MCP command in configs | PATH `anvil`, not versioned absolute |
| Kill live MCP under live parents | Not default; not silent |
| Orphan cleanup | Opt-in |
| Supervisor | v2 if re-exec breaks clients |
| Graph warm | Separate from live-heal |
| Doc authority | Accepted design for MCPLH; execution via Ready items MCPLH-001..006 |

## 18. References

- CIB-242 — status hint for daemon/MCP binary skew; no auto-kill of foreign sessions
- MCPX — multi-client install registry
- MCP26 — dual-era MCP protocol (orthogonal)
- ADR-083 — GCTX MCP delivery target
- `plans/specs/2026-08-01-bare-anvil-ensure.md` — daily ensure vs reconfigure
- Field notes (session 2026-08-08): Cellar-pinned Grok config, 0.9.2 MCP children
  under multi-day Grok parents, daemon version skew, graph scan-timeout on large
  monorepo
