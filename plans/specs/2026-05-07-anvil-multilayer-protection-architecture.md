# Anvil Multi-Layer Protection Architecture

**Status:** Draft
**Date:** 2026-05-07
**Supersedes:** [`2026-05-07-daemon-lifecycle-and-discovery.md`](./2026-05-07-daemon-lifecycle-and-discovery.md)
(narrower scope; recommendations consolidated here)
**APS:** Will spawn module `multilayer-protection` (MLP) plus updates to
INTD, DRVR, RMCP, RMCPF, RTAI, LAUNCH (all named in §17).
**Decision:** [`ADR-036`](../decisions/036-daemon-scope-discovery-and-boundaries.md)
(updated for this scope) plus a new ADR-037 covering the witness chain
and L4 policy framework (separate doc).
**Brainstorm:** [`2026-05-07-daemon-sessions-surfaces-boundaries.md`](../brainstorms/2026-05-07-daemon-sessions-surfaces-boundaries.md)
(round 1: daemon scope) plus [`2026-05-07-anvil-multilayer-protection-brainstorm.md`](../brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md)
(round 2: multi-layer defense, witness chain, baseline, hooks).

> **Inner-shape rule.** Diagnostics carried at every layer are the canonical
> [`anvil.diagnostic.v1`][diag] payload. New metadata in this spec lives on
> outer envelopes (witness lines, MCP responses, status output, manifest
> entries) — never on `Diagnostic` itself.

[diag]: ./2026-04-26-diagnostic-envelope-coordination.md#canonical-inner-shape-diagnostic

---

## 0. What this document is

This spec consolidates the full session's design into one reference. An
engineer should be able to implement against it without reading the
brainstorm. Section §17 lists APS work items.

The architecture has shifted from "where does the daemon live" to "what
does Anvil's protection look like end-to-end across every scenario the
user actually hits." The daemon remains foundational but is one of
several mechanisms; the witness chain and L4 policy are what bind them
into a coherent guarantee.

---

## 1. Doctrine

These statements are non-negotiable design principles. Every decision in
this spec must trace back to one of them.

1. **Deterministic, pre-commit.** Anvil catches violations before they
   land in shared history. L0–L3 do this on the developer's machine.
   L4 catches what L0–L3 missed before the change reaches a remote.
   L5 audits anything that slipped through.
2. **Defense in depth, not single-layer.** Each surface contributes the
   strongest layer it can; layers compensate for one another's failure
   modes. L0 (MCP) is best-effort; L2/L3/L4 are mandatory deterministic
   gates.
3. **Failure reduces noise, not increases it.** The user's terminal is
   never a wash of Anvil error messages. Silent on success; one terse
   line on warning; one terse line plus actionable pointer on block;
   repeat-suppressed. Detail goes to log files.
4. **Honest claim only.** Anvil's status / MCP response / doctor output
   declares one of a closed set of states. "Protected" is never said
   when one or more layers are unverified or degraded. False confidence
   is the worst failure mode.
5. **Planless-first.** Anvil works without config (ADR-001). `anvil
   start` writes minimal anvil-managed files; nothing else is required
   from the user.
6. **New edges only.** Existing state at adoption time is grandfathered
   (ADR-003); only new violations after baseline are flagged. Security-
   class rules are exempt — secrets are still secrets.
7. **Anvil cloud is opt-in, not required.** v1 ships with zero hosted
   infrastructure. Hosted services (GitHub App, Anvil cloud) are
   amplifiers for team-scale enforcement, not foundations.
8. **Air-gapped by default.** v1 must work fully without internet.
   All built-in rules ship in the binary; witness chain stays local
   until the user pushes; no cloud calls in normal operation. Rule
   pack distribution (vNext) is git-based, not HTTPS-based, to
   preserve this property. Telemetry / usage analytics (if any) are
   explicit opt-in.

---

## 2. Layer model (L0 – L5)

| Layer | Trigger | Mechanism | Determinism | Bypass cost |
|---|---|---|---|---|
| **L0 — Pre-write (MCP)** | AI agent calls `anvil_validate_write` before writing | RMCP shim → daemon RPC or embedded fallback | Best-effort (LLM may not call the tool) | Zero — write never happens |
| **L1 — Mid-edit (editor driver)** | Editor buffer changes (`textDocument/didChange`) | DRVR `anvil/scan_buffer` request; emits Kindling `gate_evaluated` with `mode: midEdit` (no witness line — no commit yet) | Best-effort (debounced; not every keystroke) | Zero — caught before save |
| **L2 — Save-time (daemon watcher)** | Filesystem write event (inotify / FSEvents / RDCW) | INTD watcher pipeline | **Deterministic** — kernel guarantees the daemon sees every write | Already on disk; fence + alert |
| **L3 — Pre-commit (hook)** | `git commit` invokes pre-commit hook | `anvil hook pre-commit` binary | **Deterministic** | Many writes already committed locally; pre-push catches |
| **L4 — Pre-push / receive** | `git push` invokes pre-push, OR server-side receives push | `anvil hook pre-push` (client) + CI action (`.github/workflows/anvil.yml`) + GH App (v2) | **Deterministic** at server side; client-side bypassable via `--no-verify` | Local history mutation; PR blocked at server |
| **L5 — Audit** | Periodic re-scan of mainline (default nightly + on-demand) | `anvil audit` command (on-demand) + `.github/workflows/anvil-audit.yml` (cron, ships active by `anvil start`) | **Deterministic** but post-merge | Already shipped; record only |

L0/L1 are the speed layers — they prevent damage entirely when they fire.
L2/L3/L4 are the deterministic backbone. L5 catches whatever bypassed
everything (admin overrides, force-pushes that skipped checks).

### 2.1 What runs at which layer

Built-in rules (`anvil-checks` crate) and Rego custom rules
(`anvil/rules/*.rego`) both produce findings in the canonical
`anvil.diagnostic.v1` envelope. Layer-specific scope:

| Rule class | L0 | L1 | L2 | L3 | L4 | L5 |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| Secret detection (built-in) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| AI-001 reasoning patterns (built-in) | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| Command safety (built-in) | ✓ | — | ✓ | ✓ | ✓ | — |
| Anti-pattern (built-in) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Architecture boundaries (Rego) | — | — | ✓ | ✓ | ✓ | ✓ |
| Dependency policies (Rego) | — | — | ✓ | ✓ | ✓ | ✓ |
| Custom Rego packs | — | — | ✓ | ✓ | ✓ | ✓ |

Rego rules don't run at L0/L1 because (a) the broader fact graph isn't
available pre-write, and (b) OPA evaluation cost exceeds the mid-edit
latency budget. Built-in pattern rules cover those layers.

---

## 3. Daemon scope and identity

### 3.1 Per-execution-scope, not per-user-singleton

A daemon's natural reach is the set of files visible via the kernel's
inotify-equivalent. The unit is **execution scope** — a place where one
kernel can directly observe writes.

Boundaries that create a separate execution scope:

- Container boundary (Docker / OCI / Podman)
- VM / kernel boundary (each WSL2 distro; macOS virtualisation)
- Sandbox boundary (macOS App Sandbox, Flatpak, snap)
- User boundary (different UID)

Within an execution scope: **one daemon per (uid, OS)** is the singleton.
Across scopes: multiple daemons coexist by design, not as a failure
mode.

### 3.2 `os_locality_token`

Identifies a daemon's execution scope. Hashed-prefix derived from:

| Platform | Source string |
|---|---|
| Linux native | `linux\0<uid>\0<machine-id>` (from `/etc/machine-id`) |
| macOS | `macos\0<uid>\0<IOPlatformUUID>` |
| Windows native | `windows\0<user-sid>\0<MachineGuid>` |
| WSL distro | `wsl\0<uid>\0<host-machine-guid>\0<distro-name>` (distro from `/proc/sys/kernel/osrelease` + `/proc/mounts` 9p tag — env vars are advisory only per F-A1 hardening) |

Rendered as `<platform>:<8-byte-hex-prefix>`. Surfaces re-derive their own
token at attachment time and verify against `info.json`'s recorded token.

### 3.3 Daemon discovery: `info.json`

Daemon writes `<runtime_dir>/anvil/intercept.info.json` at `listen()`
time, atomically replaced on restart, removed on graceful shutdown.
Two-phase write: `ready: false` until init complete, `ready: true` after.
Carries pid, starttime ticks (per platform), version, proto_version,
transport path, log path, `os_locality_token`, supported method list.

Per-platform runtime dirs:

| Platform | Path |
|---|---|
| Linux | `$XDG_RUNTIME_DIR/anvil/` (fallback `~/.local/state/anvil/`) |
| macOS | `~/Library/Application Support/Anvil/runtime/` |
| Windows | `%LOCALAPPDATA%\Anvil\runtime\` |
| WSL | `/run/user/<uid>/anvil/` (per distro) |

### 3.4 Lifecycle: `anvil intercept ensure`

Idempotent lazy launcher. Called by `DriverClient.connect()`, the MCP
shim, `anvil start`, and platform supervisors (vNext). Spawns the daemon
detached on cold start; reaps stale `info.json` on confirmed staleness;
refuses on `cross-boundary-detected` / `proto-version-mismatch`.

Safe-spawn: env-clears `LD_PRELOAD`, `DYLD_INSERT_LIBRARIES`, etc.
before exec. Daemon binary resolved sibling-to-CLI, not via `PATH`
search.

### 3.5 Cross-OS boundary policy

Each WSL distro is a separate execution scope. Native Windows is its
own. Cross-boundary attachment is **detected and refused** in v1, not
bridged. Path translation between Windows and WSL is rejected as a v1
guarantee — too many failure modes (case folding, NTFS junctions vs
Linux symlinks, 9P inotify gaps). Defense: `os_locality_token`
mismatch → structured refusal. Recovery: `anvil doctor
--explain-boundary` produces the `cross-boundary-mixed` verdict and
names the remediation paths.

vNext: bridge driver pattern with explicit transport + audit logging,
its own ADR.

### 3.6 Multi-daemon reality

Real-world setup: laptop daemon + desktop daemon + Morgan's-machine
daemon, plus optional remote MCP. Three execution scopes, three local
daemons, all serving the same `project_id`. Coordination happens via
git (every daemon's witnesses converge on the same `anvil/witnessed.ndjson`
through merge=union). No leader election, no inter-daemon RPC.

---

## 4. Project identity

### 4.1 Composite identity

```
ProjectIdentity := {
  project_uuid:    "01997e4a-1b2c-7345-8901-abcdef123456",     // authoritative; in anvil/project-id (tracked)
  first_commit:    "a3b2ea4e...",     // cross-check from git log
  origin_canonical: "github.com/eddacraft/anvil"  // best-effort cross-check (canonicalised: scheme/host/case/.git-stripped)
}
```

Match precedence: `uuid` > `first_commit` > `origin`. Mismatched
cross-checks warn but don't auto-merge.

### 4.2 Bootstrap

`anvil start` writes `anvil/project-id` (UUID, plain text). Light-init —
no prompts, no AI, no analysis. The user's only required step is "stage
and commit `anvil/`."

### 4.3 Forks

Forks inherit `project_id` by default. A fork is "the same project,
different branch of governance." Fork-and-customize policy:

- Inherits parent's project_uuid
- Records `forked_from: <parent_uuid>` in `anvil/project-id`
- Can opt-out by changing `project_uuid` (becomes a new project)
- Can layer additional rules in `.anvil.yaml` / `anvil/rules/`

### 4.4 Why `anvil/` (no dot) and not `.anvil/`

Two directories with deliberately-different conventions:

| Path | Purpose | Tracked? | Carries to worktrees? |
|---|---|---|---|
| `anvil/` (no dot) | Tracked metadata that must travel: `project-id`, `witnessed.ndjson`, `policy.yml`, `baseline.json`, `rules/*.rego` | **Yes** | Yes — sidesteps any dotfile-skipping in tooling that creates worktrees |
| `.anvil/` (with dot) | Local execution state: cache, fence files, runtime `info.json`, logs, scratch | **No** (gitignored by default) | Doesn't matter — daemon recreates per execution scope |

Resolves the worktree-bootstrap dotfile-propagation issue.

---

## 5. Witness chain architecture

### 5.1 Files

```
anvil/
├── project-id                       # plain text UUID + forked_from
├── policy.yml | policy.json | policy.toml   # L4 policy + baseline cutoff
├── baseline.json                    # legacy findings, anvil-managed (always JSON)
├── rules/                           # custom Rego rules (vNext-first-class)
│   └── *.rego
└── witness/
    ├── manifest/
    │   └── chain.ndjson             # append-only manifest events (merge=union)
    ├── active.ndjson                # current witness lines (capped at 1000 lines / 1 MB)
    └── archive/
        └── <scope-prefix>-<seq>-<merkle-prefix>.ndjson   # frozen, immutable
```

### 5.2 Witness line shape

Each commit appends one line:

```jsonc
{
  "v": 1,
  "project_id": "01997e4a-1b2c-7345-8901-abcdef123456",
  "tree": "<git tree hash being committed>",
  "parent_commit": "<git parent hash>",          // single value for normal commits
  "parent_commits": ["...", "..."],              // present only for merge commits (DAG join)
  "scope": "linux:8d3f1a2c",
  "anvil_version": "0.6.0",
  "rules_sha": "<sha of effective rule set>",
  "agent": {
    "task_id": "claude-code-X",
    "step_id": "Y",
    "parent_session_id": "..."
  },
  "L0": {"status": "miss", "reason": "no-mcp"},
  "L2": {"status": "ok", "watcher_health": "ok"},
  "L3": {"status": "ok", "rules": 42, "mode": "block", "backend": "daemon", "latency_ms": 120},
  "prev_line_hash": "<sha256 of previous line>",  // hash chain
  "ts": "2026-05-07T12:34:56Z"
}
```

`prev_line_hash` chains every line; integrity verifiable forward from
`GENESIS-FRESH` (greenfield) or `GENESIS-BASELINED` (adopted-existing-repo)
anchor. Tampering breaks the chain.

### 5.3 Active + archive + manifest

Active file capped at **1000 lines OR 1 MB** (whichever first). On
threshold:

1. Compute SHA-256 Merkle root of `active.ndjson` content.
2. Rename to `archive/<scope-prefix>-<seq>-<merkle-prefix>.ndjson`
   (content-addressed naming so parallel rollovers on different
   machines don't collide).
3. Append `archive_sealed` event to `manifest/chain.ndjson`.
4. Create new `active.ndjson` whose first line `prev_line_hash`
   chains from the merkle root.

All four steps inside one `flock` on `anvil/witness/.lock`. Pruning
default: `keep-all` (configurable per `anvil/policy.yml`'s
`enforcement.witness.archive_retention`).

### 5.4 Concurrency: lock-protected chain integrity

Every witness write acquires `flock(LOCK_EX)` on `anvil/witness/.lock`.
Inside the lock:

1. Read chain head (last line of active.ndjson, or merkle root from
   most recent archive if active is empty).
2. Compose witness line with `prev_line_hash = chain_head`.
3. Check rollover threshold; rollover if needed.
4. Append.

Lock hold time: <1ms typical, <10ms at rollover. Concurrent agents
serialise but don't block long.

Validation work (rule evaluation) happens **outside** the lock — only
the witness append itself is locked. Multi-agent waves don't serialise
on validation.

### 5.5 Merge handling

Merge commits carry `parent_commits` (array) and `prev_line_hashes`
(array, one per parent). The witness chain is a DAG, not a linear list.
L4 verification walks the DAG from the commit being pushed back to a
known anchor.

---

## 6. Hook surface

### 6.1 Hooks installed by `anvil start`

| Hook | Purpose | v1? | Time budget |
|---|---|---|---|
| `pre-commit` | L3 validation; witness append; chain integrity | **v1** | <500ms p95 |
| `post-commit` | Kindling `action_executed`; daemon chain-head cache update | **v1** | <50ms p95 |
| `pre-push` | L4 client-side validation; chain integrity across pushed commits | **v1** | <2s p95 |
| `post-merge` | Witness chain merge-join recording; Kindling | **v1** | <100ms p95 |
| `post-rewrite` | Regenerate witnesses for amended/rebased commits | **v1** | <500ms × commit-count p95 |
| `prepare-commit-msg` | Inject task_id / agent attribution trailer | **v1.5** | — |
| `commit-msg` | Validate commit message style; check `@anvil-ignore` citations | **v1.5** | — |

Anything else (post-checkout, pre-rebase) is unnecessary — daemon
observes via watcher, no hook needed.

### 6.2 Integration with existing frameworks

`anvil start` detects framework and integrates non-destructively:

| Detected | Action |
|---|---|
| `.husky/pre-commit` (Husky) | Append `anvil hook pre-commit "$@"` to existing chain |
| `lefthook.yml` / `.toml` | Add Anvil step to lefthook config |
| `.pre-commit-config.yaml` | Add Anvil entry to repo hooks |
| `.cargo-husky/hooks/` | Append to its hook |
| `.githooks/` with `core.hooksPath` set | Append at end of pre-commit |
| Nothing detected | Install at `.git/hooks/pre-commit` directly |

Anvil's hook line uses no `|| true` — the binary itself decides exit
codes:

- exit 0 = pass, warn, or anvil-internal-error (proceed)
- exit 1 = block decision (refuse commit/push)
- panic → caught by Rust panic handler, exit 0 + log + one terse line

### 6.3 Noise discipline (the Serena rule)

Hard requirement: a flaky Anvil must not flood the user's terminal.

| Scenario | Output |
|---|---|
| Validation passed | *(silent)* |
| Validation found warn-level | `anvil: 1 warning (commit allowed) — anvil show <id>` |
| Validation found block-level | `anvil: 1 finding (block) — anvil show <id>` (exit 1) |
| Daemon unreachable, embedded ran | `anvil: daemon offline, embedded fallback used` (only on first occurrence in session) |
| Embedded also failed | `anvil: L3 errored — fallback to L4 (anvil doctor for details)` (proceed) |
| Hook didn't fire (worktree not bootstrapped) | *(no output — hook didn't run)* L4 detects on push |
| `--no-verify` used | *(nothing — git's own warning suffices)* |
| Hash chain break | `anvil: witness chain broke — anvil doctor --explain-chain` |

Repeat-failure suppression: same class+detail won't re-emit in the
same session. Detail goes to `~/.local/state/anvil/intercept.log` and
`intercept-panic.log`.

---

## 7. L4 policy framework

### 7.1 `anvil/policy.yml` shape

```yaml
required_anvil_version: "0.6.0"        # optional exact-semver floor
baseline:
  cutoff_commit: a3b2ea4e...           # everything before is legacy
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
    on_block: reject
    on_warn: accept
  - pattern: release/*
    require: l3_and_l4
    on_no_witness: reject
  - pattern: dependabot/*
    require: l4_only
    on_no_witness: validate_at_l4
  - pattern: '*'
    require: l4_or_l3
    on_no_witness: validate_at_l4
    on_warn: accept
```

The file is tracked, travels with the repo, format-flexible
(yaml/json/toml chosen by `.anvil.*` extension).

### 7.2 v1 L4 enforcement points (all client-side or repo-committed)

| Enforcement point | Where it runs | What it covers |
|---|---|---|
| `pre-push` hook | Developer's machine | Commits about to leave the machine; bypassable via `--no-verify` |
| CI action (`.github/workflows/anvil.yml`) | User's GitHub Actions runtime | PR validation; bot commits; web edits; everything that bypassed local |
| Pre-receive hook (vNext, self-hosted git) | User's git server | Universal coverage for self-hosted Forgejo / Gitea / GitLab / etc. |

**Zero hosted Anvil infrastructure required for v1.** Wow-start preserved.

### 7.3 v2 amplifier: GitHub App

Adds:
- Centralised enforcement (can't bypass via `--no-verify` or by disabling CI workflow)
- Branch-protection-rule integration
- Inline PR diagnostics
- Anvil cloud sidecar API (optional)

**Out of v1** because requiring sign-up + permission grant + branch-protection
config breaks the 60-second wow-start. v2 amplifier when teams want
enforced-can't-bypass.

### 7.4 `validate_at_l4` server-side fallback

When a commit lacks an L3 witness AND policy says `validate_at_l4`,
the L4 enforcement point:

1. Reads the commit's diff.
2. Runs the same rule pipeline as L3.
3. Generates an L4 witness with `scope: "l4-server:<instance>"`,
   `agent: null`, `L0/L2/L3: {status: "miss"}`, `L4: {status: ok|warn|block, backend: server-validator}`.
4. Writes to `refs/notes/anvil-l4` (server-side ref; fetchable on
   clone with config; doesn't pollute working tree).
5. Applies policy decision.

Catches:
- `--skip-hooks` opt-out users
- External contributor PRs without Anvil installed
- Bot commits (Dependabot, Renovate, etc.)
- Web/mobile/API direct edits
- Force-pushes that bypassed local hooks
- Squash merges that drop component witnesses
- History grafts (combined with `anvil baseline`)

### 7.5 `anvil baseline` for adopting Anvil into existing repos

One-shot at adoption:

1. Scan current tree against all rules. Record findings in
   `anvil/baseline.json` (always JSON, schema `anvil.baseline.v1`).
2. Pin `cutoff_commit: <HEAD-sha>` in `anvil/policy.yml`.
3. Write witness genesis line anchored at the cutoff
   (`prev_line_hash: GENESIS-BASELINED`).
4. Install hooks (same as `anvil start`).
5. Stage everything; user commits "Adopt Anvil."

**Per-rule-class default behavior:**

| Rule class | Default | Why |
|---|---|---|
| Architecture / boundaries | grandfather | Existing structure was conscious |
| AI reasoning patterns | grandfather | Existing comments are usually human |
| Style / formatting | grandfather | Don't gate on adoption |
| License headers | grandfather | Don't trigger 10k file edits |
| **Secret detection** | **do-not-grandfather** | Secrets in existing code are still secrets |
| **Command safety** | **do-not-grandfather** | Dangerous code is still dangerous |

Hard-pinned at config-parse time — `.anvil.yaml` cannot disable
secrets/command-safety. Per-finding bypass via `@anvil-ignore` (ADR-004).

Re-baselining (`anvil baseline --refresh`) auditable — refresh writes
a `baseline-refreshed` line to the witness chain; suspicious refreshes
(suddenly grandfathering large new findings) trigger
`degraded:baseline-suspicious` for human review.

---

## 8. Multi-agent coordination at burst scale

The 82-commits / 8-PRs / wave-of-subagents stress test profile.

### 8.1 What's covered by existing INTD work

| Concern | Mechanism | Source |
|---|---|---|
| RPC connection cap (64) | Structured rejection past cap | INTD-016 |
| RPC rate limits (100/1000 burst) | Token bucket per connection | INTD-016 |
| Frame size cap | 1 MiB scan_buffer / 64 KiB control | INTD-016 |
| Idle / handshake timeouts | 5s / 60s | INTD-016 |
| Parser concurrency | `thread_local!` per worker | INTD-001 |
| Watcher coalescing | 50ms per-session window | INTD-004 |
| Telemetry overflow | Bounded channel + drop event | INTD-013 |
| Session TTL (crashed launchers) | 30s heartbeat eviction | INTD-003 |
| Cross-session redaction | Daemon-enforced filter | INTD-015 |
| Fence persistence | On-disk transactional | INTD-007 |
| Embedded mode parity | Same envelope as daemon path | INTD-009 |

### 8.2 Gaps requiring new design (v1 mandatory unless tagged)

| Gap | Design |
|---|---|
| **Per-task fence isolation** | Fence keys become `(WorktreeKey, AgentTag)`; one bad sub-agent doesn't cascade-fence a whole worktree. Worktree-level fence still triggered for unattributable writers. |
| **Sub-agent attribution** | Primary: `ANVIL_TASK_ID` env var inherited through process spawns. Fallback: process-tree walk (parent PID chain) until a registered ancestor is found. Both required for 82-commits scale. |
| **Connection pooling under sub-agent waves** | DriverClient "child mode": parent agent's client serves as relay for child processes via local IPC. `ANVIL_DRIVER_SOCKET` env var propagated. Direct daemon connection as fallback. |
| **Kindling write throughput** | WAL mode + async write queue (1000 pending) + 10ms batch coalescing + per-priority lanes (drop low-priority on saturation, never `error`/`constraint_applied`). |
| **Rule version pinning per evaluation** | Witness line carries `rules_sha`. In-flight evaluations finish with their pinned rules even if `.anvil.yaml` updates mid-burst. |
| **DAG-aware witness verification at L4** | Merge commits carry `parent_commits[]` and `prev_line_hashes[]`. L4 walks the DAG. |
| **L4 read-time witness dedup** | Multiple lines for `(tree, parent_commit, scope, ts)` tuple → first-passing wins. |
| **Parallel worktree bootstrap dedup** | Per-worktree `flock` on `.git/worktrees/<name>/anvil-bootstrap.lock`. |

### 8.3 Pinned concurrency budgets

| Resource | Budget |
|---|---|
| Daemon RPC connections | 64 (INTD-016 default) |
| Daemon RPC rate per connection | 100 sustained / 1000 burst |
| Witness lock hold time | <1ms typical, <10ms at rollover |
| Watcher event queue | 16384 inotify default (overflow → scan-on-poll) |
| Kindling write queue | 1000 pending |
| Driver-relay child connections | 32 sub-agents per parent |
| Bootstrap thread pool | 4 workers |
| Per-worktree fence cascade limit | 5 fences in 60s → `degraded:fence-cascade` |

### 8.4 Named degraded modes (no terminal noise)

Reported in `anvil status` (closed set), never spammed to terminal:

- `degraded:watcher-overflow` — inotify queue overflowed; daemon
  switches to scan-on-poll for affected worktree
- `degraded:rpc-saturated` — All RPC slots in use; new connections
  retriable; daemon prioritises shorter calls
- `degraded:kindling-queue` — Async write queue near cap; lower-
  priority observations dropped (never `error` / `constraint_applied`)
- `degraded:lock-saturated` — Witness lock contention >100ms p95
  sustained; surfaces in `anvil status`
- `degraded:fence-cascade` — More than 5 fences in 60s; daemon refuses
  new fences pending operator review (no auto-recovery — the human
  decides)
- `degraded:baseline-suspicious` — Refresh suddenly grandfathers a
  large finding-count increase

---

## 9. Worktree bootstrap

### 9.1 The failure mode

`git worktree add ../feature-branch` doesn't run `pnpm install`, so
husky's `.husky/_/` runtime files (gitignored, generated by `husky
install`) are missing. `core.hooksPath = .husky/_` → git looks for
hook there → file doesn't exist → silent commit, no hooks fire,
no witness written.

### 9.2 Three-layer bootstrap recovery

1. **Daemon-driven silent self-heal (primary):** daemon watches
   `.git/worktrees/` for new worktrees. When one appears, the daemon
   runs an idempotent bootstrap on it (regenerates `.husky/_/` from
   `package.json`'s husky version, or installs `.git/hooks/pre-commit`
   if no framework). Silent — user sees nothing in terminal.
2. **`anvil hook bootstrap` command (recovery):** explicit user-run
   command if self-heal missed it. Detects framework, regenerates
   runtime files, walks back through unwitnessed-but-unpushed commits
   and writes retroactive witnesses.
3. **L4 catches the gap (last line of defense):** if commits made it
   out without witnesses (self-heal raced, user pushed before
   bootstrap), L4 detects missing witness on push:
   ```
   remote: anvil: 3 commits missing L3 witness (abc123, def456, 789abc).
   remote:        cause: hooks did not fire (worktree bootstrap incomplete).
   remote:        fix:   in your worktree, run: anvil hook bootstrap
   remote: error: push rejected
   ```

`anvil hook bootstrap --witness-recent` retroactively witnesses commits
in `<remote>..HEAD` range only — never tries to witness pre-baseline
history or already-pushed commits.

### 9.3 Cleanup → PR flow

User finds useful commits in old worktree, cherry-picks/rebases to
current branch:

| Flow | Witness behaviour |
|---|---|
| Cherry-pick to current branch | New commit on destination → fresh hook fires → fresh witness |
| Rebase old branch onto main | Each replayed commit fires hooks → fresh witnesses |
| Squash-merge | One new commit → one fresh witness covering squashed diff |
| Direct push of old branch | L4 `validate_at_l4` policy generates server-side witnesses |

Then `git worktree remove` deletes the old worktree — its unwitnessed
commits never reach a remote, so no witnesses needed for them.

---

## 10. Kindling integration

### 10.1 Per-(machine, project) DB location

```
Linux:    ~/.local/share/anvil/projects/<project_uuid>/kindling.db
macOS:    ~/Library/Application Support/Anvil/projects/<project_uuid>/kindling.db
Windows:  %LOCALAPPDATA%\Anvil\projects\<project_uuid>\kindling.db
```

Three checkouts of the same project on the same machine share one
Kindling DB. Cross-worktree session-trace queries work. Different
projects are isolated (Kindling already enforces this contract).

### 10.2 Why Kindling and witness file are complementary, not redundant

| Aspect | Witness file | Kindling |
|---|---|---|
| Lives in | Repo, tracked | Local checkout, NOT tracked |
| Granularity | Per-commit | Per-event (11 observation kinds) |
| Format | NDJSON, hash-chained | SQLite, indexed, queryable |
| Cross-machine | Yes (via git push/pull) | No (local only) |
| Source of truth for | "Was this commit witnessed by Anvil?" | "What was the full session that led to this commit?" |
| Read latency | tail/grep-friendly | Indexed query, milliseconds |
| Lifecycle | Forever in repo | 90 days default, auto-prune |

Witness lines carry `kindling_session_id` / `kindling_gate_eval_id`
pointers back into local Kindling for forensic dig-in (when the user
has access to the machine). L4 doesn't depend on remote Kindling
access — that's by design for data sovereignty.

### 10.3 Hook flow integrating Kindling

```
1. Hook fires
2. Read anvil/project-id; resolve Kindling DB path
3. Validate diff via daemon RPC (or embedded fallback)
4. Daemon emits Kindling observations to local DB:
   - session_start (if not already)
   - gate_evaluated
   - constraint_applied (per blocking rule)
   - action_executed
5. Daemon returns decision + Kindling identifiers (session_id, gate_eval_id)
6. Hook composes witness line carrying the identifiers
7. Hook acquires flock(anvil/witness/.lock):
     read chain head; check rollover; append witness; release lock
8. Hook decides exit code based on validation decision
```

Observations carry secret-detection sanitisation (existing kindling-
integration). Hard-pinned redaction rules apply.

---

## 11. Rule distribution and versioning

### 11.1 `rules_sha` computation

```
rules_sha = sha256(
  "anvil-rules-v1\n"
  + "anvil_version=" + binary_semver + "\n"
  + "opa_runtime_version=" + opa_version + "\n"   // present only if Rego rules present
  + "built_in_rules=[\n"
  +   for each enabled built-in (deterministic order): "  " + rule_id + ":" + sha256(params) + "\n"
  + "]\n"
  + "rego_rules=[\n"
  +   for each anvil/rules/*.rego (sorted): "  " + path + ":" + sha256(content) + "\n"
  + "]\n"
  + "config_sha=" + sha256(canonical_json(parsed_config)) + "\n"
)
```

Format-independent: hash the parsed config's canonical JSON, not the
raw bytes. `.anvil.yaml` and an equivalent `.anvil.toml` produce the
same `rules_sha`.

### 11.2 Mixed-version teams

Witness lines carry `rules_sha` + `anvil_version`. Different machines
producing witnesses with different `rules_sha` is normal; L4 verifies
"this rules_sha is from a recognised Anvil version."

`anvil/policy.yml` may set `required_anvil_version: "0.6.0"` (exact
semver) to floor the version. Pre-commit/pre-push hooks check this before validating;
fail with terse actionable message:
`anvil: this repo requires anvil >= 0.7.0 (you have 0.6.0)` (exit 1).

### 11.3 Hard-pinned rule classes

Configs cannot disable security-critical classes:

| Rule class | Hard-pinned? |
|---|---|
| `secrets` | **Yes** |
| `command-safety` | **Yes** |
| `architecture` / `ai-patterns` / `style` / `license` | No |

Parser refuses configs that try to disable hard-pinned classes — same
pattern as ADR-015 ambiguous-ownership hard-cap.

### 11.4 Rego custom rules (vNext-first-class, accepted at v1)

Continuing ADR-006 (hybrid DC + OPA). Custom rules live at
`anvil/rules/*.rego`, tracked in repo. OPA runtime bundled with Anvil
binary; pinned via `opa-agent-orchestration` work.

| Layer | Built-in (Rust) | Custom (Rego) |
|---|---|---|
| L0 (pre-write MCP) | ✓ | — (broader graph not available) |
| L1 (mid-edit) | ✓ | — (eval cost > budget) |
| L2 (save-time) | ✓ | ✓ |
| L3 (pre-commit) | ✓ | ✓ |
| L4 (pre-push / CI) | ✓ | ✓ |
| L5 (audit) | ✓ | ✓ |

Findings from both lanes converge on `anvil.diagnostic.v1`. Daemon
caches `(rule, input_fingerprint) → decision` to amortise OPA cost.

vNext beyond v1.5: org rule packs distributed via git refs (per
air-gapped doctrine §1 principle 8); community packs marketplace.

### 11.5 Org-shared rules via git submodule (v1 supported pattern)

For organisations that want to share rules across multiple repos
without inventing a pack mechanism, v1 supports a documented submodule
pattern:

```
org/anvil-rules-shared       (separate repo, contains *.rego files)
member-repo/
  anvil/
    rules/
      shared/                 (git submodule pointing at org/anvil-rules-shared)
        no-hardcoded-secrets.rego
        api-versioning.rego
      local/                  (per-repo custom rules)
        team-specific.rego
```

Anvil's rule discovery walks `anvil/rules/**/*.rego` recursively, so
submodule contents participate in the rule set automatically. Submodule
SHA pins the version; updating the submodule pin is an explicit
commit. `rules_sha` includes submodule contents (they're tracked
files at the working-tree level once checked out).

This is **convention, not new mechanism**. No code changes in Anvil
needed. Documented in the runbook so org admins know the pattern.

vNext (post-v1.5) may layer a richer pack distribution on top, but
git submodules cover the v1 air-gapped requirement and the v1.5
shared-rules requirement.

---

## 12. Config format flexibility

`anvil start` and `anvil baseline` detect existing config format in
order: `.anvil.yaml` → `.anvil.yml` → `.anvil.json` → `.anvil.toml`.
First match wins. New repos default to YAML. `anvil start --format
json|toml` overrides.

Within a repo: `.anvil.*` and `anvil/policy.*` use the same format
(consistency). `anvil/baseline.json` is **always JSON** (anvil-managed,
predictable schema for tooling). `anvil/witnessed.ndjson` and
`anvil/witness/manifest/chain.ndjson` are NDJSON (not configs).

---

## 13. Cross-platform support

| Platform | v1 status | Notes |
|---|---|---|
| Linux (native) | ✓ | Primary platform; INTD complete |
| macOS | ✓ | Via INTD; per-platform paths in §3.3 |
| Windows (native) | ✓ | Via INTD; named pipe transport |
| WSL (per distro) | ✓ | Each distro = separate execution scope |
| Cross-Windows ↔ WSL | ✗ refuse | `os_locality_token` mismatch → `cross-boundary-detected` |
| Dev container / Codespaces | later | Need daemon-in-container packaging |
| SSH remote | later | Remote-host daemon model selected in [`ADR-043`](../decisions/043-ssh-remote-host-daemon.md); implementation tracked by [`SSHREMOTE`](../modules/ssh-remote-host-daemon.aps.md) |
| Web / mobile / API edits | L4-only | Caught by CI action / GH App (v2) at push time |

macOS App Sandbox (Cursor, etc.): MCP shim falls back to embedded
validation; daemon-backed mode for sandboxed editors is v1.5+. v1
behaviour is honest fallback — `validation.backend = embedded`,
`pre-write-only` worktree state.

---

## 14. Closed-set protection-claim policy

`anvil status` / MCP response / doctor output declares one of these
states. Tooling, CI, and contract tests treat the set as closed.

### 14.1 Per-surface states

`unbound` | `attached` | `participating` | `embedded-fallback` |
`degraded` | `cross-boundary-refused` | `quarantined` | `detached`

### 14.2 Per-worktree states

| State | Meaning |
|---|---|
| `unprotected` | No daemon, no embedded fallback; `ensure()` failed |
| `warming` | Daemon up but `ready: false`, OR no surfaces attached yet |
| `pre-write-embedded` | MCP shims active; all on `embedded` backend |
| `pre-write-daemon` | MCP shims active; ≥1 daemon-backed |
| `save-time-only` | Editor driver Participating; no MCP |
| `full` | ≥1 daemon-backed MCP + ≥1 Participating editor driver |
| `degraded-protection` | Above states with ≥1 surface degraded |
| `cross-boundary-mixed` | Multiple surfaces detected on different `os_locality_token`s |
| `multi-daemon-detected` | Two `info.json` records observed |
| `path-uncertain` | Daemon canonicalisation differs from registered path |

`pre-write-embedded` ≠ `pre-write-daemon` — tooling MUST treat them
distinct (DLIFE-009 contract test pins this).

### 14.3 Doctor exit codes

| Exit | Meaning |
|---|---|
| 0 | Daemon up, all surfaces healthy |
| 1 | At least one surface degraded / embedded-fallback (also: generic error) |
| 2 | Validation block / gate failure (`EXIT_GATE_FAIL`) |
| 3 | Authentication required (`EXIT_AUTH_REQUIRED`) |
| 4 | Configuration error (`EXIT_CONFIG_ERROR`) |
| 5 | Cross-boundary detected/mixed (`EXIT_CROSS_BOUNDARY`) |
| 6 | Daemon not running (`EXIT_DAEMON_DOWN`) |
| 7 | `proto-version-mismatch` (`EXIT_VERSION_MISMATCH`; `anvil intercept restart` to fix) |
| 10 | Discovery failed (lstat ladder violation; `EXIT_DISCOVERY_FAILED`) |

Codes match the reserved constants in `crates/anvil-cli/src/main.rs`
and the CLI surface coherence spec §3. Codes 8 and 9 are intentionally
reserved. CI fails fast on 2, 5, 7, 10.

---

## 15. Onboarding walk-through

### 15.1 Greenfield (new repo)

```
$ mkdir my-new-project && cd my-new-project
$ git init -b main
$ anvil start
anvil: initialised project (id: 01997e4a-1b2c-7345-8901-abcdef123456, genesis: GENESIS-FRESH)
anvil: hooks installed (no framework — wrote .git/hooks/pre-commit ...)
anvil: CI workflow written (.github/workflows/anvil.yml)
anvil: stage and commit `anvil/` to enable protection
$ git add . && git commit -m "Initial: adopt Anvil"
```

~15 seconds. Zero external services.

### 15.2 Existing repo (legacy adoption)

```
$ cd ~/legacy-monorepo
$ anvil baseline
anvil: scanning 12,340 files...
anvil: baselined at a3b2ea4e (247 legacy findings recorded)
anvil: hooks installed (husky detected — appended to .husky/pre-commit)
anvil: stage and commit `anvil/` to enable protection going forward
$ git add anvil/ .gitattributes .husky/pre-commit .github/workflows/anvil.yml
$ git commit -m "Adopt Anvil (baseline at a3b2ea4e)"
$ git push
```

~30 seconds for medium repo. Legacy findings grandfathered; new
violations from this point forward are caught.

### 15.3 Fresh worktree (existing Anvil project)

```
$ git worktree add ../feature-x feature-branch
$ cd ../feature-x
# (daemon's silent self-bootstrap — user sees nothing)
$ git commit -m "..."   # hooks fire normally; witness written
$ git push              # succeeds
```

Or, in the fallback case where self-heal didn't fire:

```
$ git push
remote: anvil: 1 commit missing L3 witness (run `anvil hook bootstrap`)
$ anvil hook bootstrap
anvil: bootstrapped (1 commit witnessed retroactively)
$ git push
```

### 15.4 The user-facing pitch

> *Install Anvil. Run `anvil start` in a new repo or `anvil baseline` in
> an existing one. Every commit is validated before it lands. Every push
> is validated before it leaves your machine. Every PR is validated in
> your CI. Mixed-version teammates work fine. Sub-agent waves work fine.
> Multiple worktrees, multiple machines, parallel branches — they all
> converge through git, and Anvil's witness chain follows. No external
> service required for v1.*

---

## 16. v1 / v1.5 / vNext / Unsupported

### 16.1 v1 (next release)

- Per-execution-scope daemons (extends current INTD)
- `info.json` runtime sidecar with `ready` two-phase
- `os_locality_token` boundary detection + refusal codes
- `anvil/project-id` UUID identity (light-init at `anvil start`)
- Witness chain (`anvil/witnessed.ndjson` + manifest + archive,
  hash-chained, lock-protected, 1000-line / 1MB rollover)
- Hooks: pre-commit, pre-push, post-commit, post-merge, post-rewrite
- L4 client-side (pre-push) + CI action (`.github/workflows/anvil.yml`)
- **L5 audit** — `anvil audit` command (on-demand) + nightly CI workflow
  (`.github/workflows/anvil-audit.yml`) shipped active by default by
  `anvil start` / `anvil baseline`
- L4 policy framework (per-branch rules, `validate_at_l4`)
- `anvil baseline` for adopting existing repos
- **Per-task fence isolation** (multi-session-per-worktree promoted from
  DLIFE v1.5 to MLP v1 per user direction — required for the
  multi-agent / sub-agent-wave workflow)
- `ANVIL_TASK_ID` env var + process-tree attribution fallback
- DAG-aware witness verification at L4
- Rule version pinning (`rules_sha` per witness)
- Hard-pinned `secrets` and `command-safety` rule classes
- Config format flexibility (yaml / json / toml)
- Closed-set protection-claim policy with contract test suite
- Noise-disciplined output across all surfaces
- Worktree silent self-heal + `anvil hook bootstrap` recovery
- Cross-Windows/WSL boundary detect-and-refuse
- macOS App Sandbox embedded-fallback path (acknowledge gap)

### 16.2 v1.5

- Multi-session-per-worktree (`AgentTag` composite key)
- Per-worktree session cap config
- Driver-relay for sub-agent connection pooling
- Async Kindling write queue with priority lanes
- `prepare-commit-msg` / `commit-msg` hooks
- macOS App Sandbox detection observability
- Rego custom rules first-class (data path exists at v1; UX/discovery
  promotion)

### 16.3 vNext

- GitHub App (centralised v2 amplifier)
- GitLab integration
- Bitbucket Cloud integration
- Pre-receive hook script (universal self-hosted git)
- Anvil cloud sidecar API
- Org / community Rego rule packs
- Rule pack marketplace
- Bridge driver for cross-OS reach (Windows ↔ WSL with audit)
- Self-hosted Anvil cloud (Helm chart)
- SSH remote-host daemon driver (remote daemon + hooks + `anvil-run`; local
  display/control only)
- Platform supervisors (systemd / launchd / Task Scheduler)
- Per-project worker model (Option D from §1.4 of original spec) if
  multi-tenant sandboxing becomes a hard requirement

### 16.4 Explicitly unsupported in v1 and v1.5

- Cross-Windows ↔ WSL surfaces talking to one daemon
- Cross-UID daemon attachment
- TCP transport
- Non-git platforms (Supabase DB branches, Notion, Figma, etc.) —
  vNext driver story
- Adversarial cross-witness forgery (provenance, not authentication —
  L4 revalidation is the defense)
- Containerised / namespace-remapped scenarios with shared runtime
  paths (`(uid, os_locality_token)` collision)

---

## 17. APS work items (proposed)

### 17.1 New module: `multilayer-protection` (MLP)

Largest piece. Owns the witness chain, hook surface, baseline,
cross-machine federation behaviour. Specific work items:

- **MLP-001:** `anvil/project-id` UUID identity + composite check
- **MLP-002:** `anvil/witnessed.ndjson` + active/archive/manifest +
  hash chain + flock + rollover
- **MLP-003:** Pre-commit hook integration (husky / lefthook / pcf /
  plain) with self-contained binary
- **MLP-004:** Pre-push hook with chain integrity verification across
  push range
- **MLP-005:** post-commit / post-merge / post-rewrite hook handlers
- **MLP-006:** `anvil/policy.yml` + per-branch policy framework +
  `validate_at_l4` server-side (initially client-side at pre-push)
- **MLP-007:** `anvil baseline` command + scan + `anvil/baseline.json`
  + grandfather logic
- **MLP-008:** `anvil hook bootstrap` recovery command
- **MLP-009:** Worktree silent self-heal in daemon
- **MLP-010:** CI action published as `eddacraft/anvil-action` on
  GitHub Marketplace (separate publishing repo;
  `.github/workflows/anvil.yml` template references it as
  `uses: eddacraft/anvil-action@v1`)
- **MLP-011:** Multi-format config (yaml/json/toml) parsing + writing
- **MLP-012:** `rules_sha` computation in witness lines + verification
  at L4
- **MLP-013:** Closed-set protection-claim contract test suite
  (extends DLIFE-009)
- **MLP-014:** Multi-session-per-worktree (`AgentTag` composite key) +
  per-task fence isolation. Promoted from DLIFE-008 v1.5 to MLP v1.
  Includes per-worktree session cap (default 16), fence keyed on
  `(WorktreeKey, AgentTag)` not just `WorktreeKey`, fence cascade
  detection (`degraded:fence-cascade`).
- **MLP-015:** L5 audit command (`anvil audit`) + nightly CI workflow
  template (`.github/workflows/anvil-audit.yml`). On-demand and
  cron-driven scan of mainline; reports drift since last audit; emits
  Kindling `gate_evaluated` per audit run.
- **MLP-016:** Editor driver L1 → Kindling integration. Editor driver
  emits Kindling `gate_evaluated` with `mode: midEdit` on every
  mid-edit finding (warn / block decisions). Successful pass-no-finding
  edits remain silent. Coordinates with RTAI-007 telemetry contract.
- **MLP-017:** Air-gapped operation guarantee + test suite. Asserts
  no network calls in `anvil start`, `anvil baseline`, `anvil intercept
  ensure`, hooks, CI action's runtime. Pack distribution (vNext)
  constrained to git-based fetch.

### 17.2 New module: `daemon-lifecycle` (DLIFE)

From the original spec — items still relevant:

- DLIFE-001..DLIFE-007 (info.json sidecar, os_locality_token, refusal
  codes, ensure launcher, runtime path, status output, doctor) — all
  v1
- DLIFE-008 multi-session-per-worktree → **promoted to v1** (renamed
  MLP-014) because per-task fence isolation is mandatory at v1 scale
- DLIFE-009 protection-claim contract suite → folded into MLP-013
- DLIFE-010 macOS App Sandbox detection → v1.5
- DLIFE-011 log rotation / panic / ensure-attempt log → v1
- DLIFE-012 status JSON schema → v1

### 17.3 Updates to existing modules

- **INTD**: Out-of-Scope updated to reference DLIFE + MLP for lifecycle
  / discovery / witness chain
- **DRVR**: DRVR-001 amendment — DriverClient uses `info.json`
  discovery + `os_locality_token` check; new "child mode" relay
  (v1.5)
- **RMCP / RMCPF**: amendment — `validation.backend` honesty rule
  (set at result-generation time); `pre-write-embedded` / 
  `pre-write-daemon` worktree state distinction
- **RTAI**: RTAI-002 schedules with MLP-006 to keep protection-claim
  story coherent
- **LAUNCH**: extends `anvil start` orchestrator with MLP-001 / -002 /
  -003 / -006 / -010 / -011 steps; status surface gains MLP states
- **LANGTS** / others as relevant

### 17.4 New ADRs (split per user direction)

- **ADR-036 (rewritten)** — Daemon scope, discovery, OS boundaries,
  identity. Existing partial ADR; this spec extends scope so the ADR
  body is rewritten to match (forks-inherit policy, execution-scope
  framing, multi-daemon-by-design).
- **ADR-037 (new)** — Witness chain + hash chain + active/archive/
  manifest + L4 policy framework. Standalone — load-bearing for
  cross-machine determinism.
- **ADR-038 (new)** — Hook surface + noise discipline (the Serena
  rule). Standalone — pulls noise-discipline up to first-class
  governance principle.
- **ADR-039 (new)** — Baseline policy + hard-pinned rule classes
  (secrets / command-safety). Standalone — captures "new edges only"
  evolution and the security-class non-negotiables.

### 17.5 Documentation deliverables

- `docs/runbooks/anvil-onboarding.md` — greenfield / baseline /
  worktree flows
- `docs/runbooks/anvil-troubleshooting.md` — degraded modes, doctor
  exit codes, common failures
- `docs/vision/anvil-scope-guard.md` updated with v1-supported
  scenarios + non-git unsupported list
- Public API docs for the `anvil hook` command surface

---

## 18. Test surface summary (v1)

- **Witness chain integrity tests** — write-many-concurrent, verify-
  chain, simulate-rollover, simulate-merge-DAG
- **Hook integration tests** — fresh worktree without bootstrap,
  husky present, lefthook present, pre-commit-framework present, no
  framework
- **L4 policy tests** — `validate_at_l4`, `l3_and_l4`, `l4_only`,
  `cutoff_commit` legacy acceptance, baseline-suspicious detection
- **Multi-agent stress test** — replay 82-commits-in-pane fixture;
  assert no fence cascade, no chain breaks, no terminal noise
- **Boundary tests** — `os_locality_token` mismatch refusal,
  `proto-version-mismatch` refusal, stale-pid refusal
- **Cross-platform CI matrix** — Linux UDS + named-pipe, Windows pipe
  + LOCALAPPDATA paths, macOS Application Support paths, WSL distro
  scoping
- **Onboarding E2E** — `anvil start` greenfield, `anvil baseline`
  existing-repo, `anvil hook bootstrap` recovery, `--verify` no-write
  probes
- **Noise discipline tests** — repeat-suppression works, panic
  produces single line + log file, daemon-down emits one line per
  session

---

## 19. Open questions / followups

Tracked but not blocking v1:

1. **Rego runtime pinning.** OPA version per Anvil binary: which exact
   version, where it lives (vendored? linked?), how it updates.
2. **Pack distribution channels.** When v1.5 rule packs land, where
   are they fetched from? Anvil cloud? git refs? OCI registry?
3. **Privacy posture for L4 (when GH App lands).** Data residency
   options, retention, opt-out.
4. **Server-side witness storage on GH (when App lands).**
   `refs/notes/anvil-l4` requires the App to push notes — alternative
   is check-run output. Probably hybrid.
5. **L5 audit cadence + UX.** Cron? Nightly CI? On-demand?
6. **Anvil-on-Anvil dogfooding loops.** Using Anvil to develop Anvil —
   meta-loop concerns (protecting the rules that protect us).
7. **Off-grid / air-gapped scenarios.** Pre-push validates locally
   (no internet needed). CI action runs in user's CI (no internet to
   Anvil). All-self-hosted scenario for enterprises.
8. **Editor driver and witness chain.** L1 (mid-edit) doesn't
   currently write witnesses (no commit yet). Should it emit
   precursor observations into Kindling for forensics?
9. **What happens to Kindling DBs when project_uuid changes**
   (e.g., explicit fork-out, project restructure). **Deferred to
   vNext per user direction** — address when a real case surfaces.
   Until then, expected behaviour: changing `project_uuid` orphans
   the existing Kindling DB at the old path; new DB created at new
   path; witness chain breaks (new genesis line with `forked_from`
   reference); user explicitly accepts this when they edit the file.
10. **Rule lint / CI for `.anvil.yaml` itself.** Currently only the
    parser refuses bad configs at load time; a `anvil config check`
    command would catch errors earlier.

---

## 20. Status / acceptance

This spec is **Draft**. Promotion to **Proposed** requires:

- ADR-037 (witness chain) drafted
- ADR-038 (hook surface) drafted
- ADR-039 (baseline policy) drafted
- MLP module skeleton in `plans/archive/modules/multilayer-protection.aps.md`
- `plans/index.aps.md` updated with MLP row
- One implementation-volunteer council pass to surface engineering
  blockers

Promotion to **Accepted** requires the council pass to land green and
at least one MLP-001 / MLP-002 prototype building on top of INTD's
existing crates.
