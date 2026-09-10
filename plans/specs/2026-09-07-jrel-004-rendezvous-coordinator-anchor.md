# JREL-004 residual: scope-stable rendezvous coordinator

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | JREL | Proposed | 2026-09-07 against `origin/main` `26a6f44eb` (`crates/anvil-intercept/src/ensure.rs`, `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-cli/src/commands/doctor.rs`, `crates/anvil-cli/tests/daemon_identity.rs`, [ADR-036](../decisions/036-daemon-scope-discovery-and-boundaries.md), [ADR-060](../decisions/060-anvil-home-install-root-override.md), issue #4432) |

| Upstream | Downstream |
| -------- | ---------- |
| [JREL-004](../archive/modules/journey-reliability.aps.md), [#4432](https://github.com/eddacraft/anvil-001/issues/4432), [ADR-036](../decisions/036-daemon-scope-discovery-and-boundaries.md), [ADR-060](../decisions/060-anvil-home-install-root-override.md), CIB-382 (`acquire_daemon_rendezvous_repair_lock_for_socket_candidates`) | Implementation PR for #4432; `crates/anvil-intercept/ARCHITECTURE.md` once the code moves |

Design-only. Do not treat this as as-built. Josh decides the spawn/discovery fork below before any coordinator implementation PR.

## Named branch

`fix/jrel-004-rendezvous-coordinator-anchor` is **not** on the remote (checked 2026-09-07: `git ls-remote --heads origin` and GitHub `list_branches`). No implementation design is committed. Open companions #4434 / #4435 explicitly leave this residual out.

## 1. How it works today

### Candidate set (per process)

`ipc::resolve_socket_connect_candidates` / `resolve_socket_connect_dirs_with_env` (`crates/anvil-intercept/src/ipc.rs`):

1. `ANVIL_HOME` set → **that prefix only** (ADR-060 exclusivity).
2. Otherwise canonical bind dir first (`XDG_RUNTIME_DIR/anvil` if set, else `$HOME/.local/state/anvil`), then the sibling: state-home if `HOME` is set, then explicit `XDG_RUNTIME_DIR/anvil` or, when XDG is unset, implicit `/run/user/<uid>/anvil` when that directory exists.

`push_unique_dir` deduplicates with `PathBuf ==` (lexical). Bind, PID, and start-lock paths follow the same canonical dir (`resolve_socket_path`, `default_pid_file_path`). Windows is a single named pipe; no coordinator.

### Ensure (JREL-004 as merged in #4428)

`ensure_daemon_coordinated` → `ensure_daemon_at` (`crates/anvil-intercept/src/ensure.rs`):

1. Probe canonical then every sibling (`live_endpoints`). A live sibling is reused; two live endpoints are a conflict naming `anvil doctor --fix`; spawn only at **this process's canonical**.
2. Sibling probes use `SocketProbe::for_sibling` (owner-only path gate; planted symlink skipped). Canonical compare vs siblings is lexical (`candidate.as_path() != socket_path`).
3. Spawn serialises on `acquire_daemon_rendezvous_repair_lock_for_socket_candidates` **then** the per-install `intercept.ensure.lock`. Doctor `--fix` holds the same repair lock and launches with `RendezvousCoordination::HeldByCaller` (`doctor.rs` `apply_intercept_socket_rendezvous_fix`, `intercept.rs` `launch_save_time_daemon_under_rendezvous_lock`).

### Coordinator selection (the residual)

`acquire_daemon_rendezvous_repair_lock_for_socket_candidates`:

1. Parent dirs of **this process's** socket candidates.
2. Lexical sort/dedup, `ensure_secure_runtime_dir` on each, `canonicalize`, physical sort/dedup.
3. Lock `intercept.rendezvous-repair.lock` in `physical_dirs.first()`.

CIB-382 already made reversed **identical** sets and ancestor aliases take one lock. That is not enough. The coordinator is still `min(physical(this process's set))`. Two shells that share a user but not a runtime dir have different sets:

| Shell | Candidates (home `/var/home/u`) | `min` coordinator |
| ----- | -------------------------------- | ----------------- |
| XDG unset | `/var/home/u/.local/state/anvil`, `/run/user/<uid>/anvil` | typically `/run/user/<uid>/anvil` |
| `XDG_RUNTIME_DIR=/tmp/xdg` | `/tmp/xdg/anvil`, `/var/home/u/.local/state/anvil` | `/tmp/xdg/anvil` |

Different locks → both cold-start. The second shell then sees two live endpoints and is wedged on every command.

Intersection of those sets is **state-home**. Existing tests never construct this shape: `scope_environments()` and `rendezvous_repair_lock_serialises_reversed_candidate_order` reverse the **same two paths**. `daemon_identity.rs` cannot hermetically drive XDG-unset through the binary because the implicit sibling is the real `/run/user/<uid>`.

### Discovery hole (same residual, not just the lock)

Even a shared lock does not yield one daemon unless the loser can **see** the winner.

- A daemon is only spawned at this process's canonical (`ensure.rs` module docs / `ARCHITECTURE.md`).
- If the `/tmp/xdg` shell wins, it binds at `/tmp/xdg/anvil`. The XDG-unset shell re-probes state-home and `/run/user/<uid>` — not `/tmp/xdg` — and starts a second daemon after the lock is released.

Convergence today is race-dependent: it works when the winner happens to bind at a path in the loser's set. `ARCHITECTURE.md` currently claims disagreeing shells spawn one daemon; that overstates the code.

### Lexical probe / conflict

One physical socket reached by two lexical candidates (runtime dir under home via an ancestor alias, or `XDG_RUNTIME_DIR` pointing at a path that canonicalizes onto state-home) is counted twice → permanent false conflict. Leaf socket symlinks are already refused (`validate_socket_path_for_client`); ancestor aliases are not.

PR #4434 (not this change) collapses **answered** endpoints by PID file for conflict **reporting**. That is not candidate derivation and does not choose the coordinator. Keep it if it merges; do not treat it as this residual.

If **any** candidate parent fails `ensure_secure_runtime_dir` (including a symlink leaf), coordinator acquisition fails entirely and ensure degrades to the per-install lock only (`hold_spawn_rendezvous` warn). That must not remain a way around the scope lock.

## 2. What must stay true

### ADR-036

- One daemon per execution scope: container / VM / WSL distro / sandbox / UID. Not a global per-user singleton across machines.
- Dual-path connect is a **client** rendezvous, not a second listener.
- Isolated / namespace-remapped filesystems remain unsupported (D-7); do not invent a cluster lock.
- Windows/WSL refusal and `os_locality_token` unchanged.
- Doctor and ensure keep coordinator-before-start-lock ordering (CIB-382: no AB/BA).

### ADR-060

- Non-empty `ANVIL_HOME` / `--anvil-home` is an exclusive single-candidate install: own socket, PID, start lock, **and** coordinator. Never fall through to production `~/.anvil` or the default runtime dir.
- Unsetting `ANVIL_HOME` returns to default-scope behaviour.

Do not flatten isolated homes into the default-scope lock.

## 3. Recommended anchor (decision fork)

**Anchor (common to every viable option):** when `ANVIL_HOME` is unset and `HOME` is set, the rendezvous coordinator is always the **physical** state-home directory `$HOME/.local/state/anvil/intercept.rendezvous-repair.lock`, not `min(this process's candidates)`. Isolated home: lock in that prefix only. `HOME` unset: fall back to this process's only candidate (rare; document, do not invent a second UID-global path).

Establish **only the anchor** for the lock. Do not require every sibling dir to be creatable.

That lock is necessary and **not sufficient**. Pick one spawn/discovery rule:

| Option | Spawn / discovery | Trade-off |
| ------ | ----------------- | --------- |
| **B (recommended)** | Keep bind-at-canonical. Under the coordinator, publish an owner-only live-endpoint record at the anchor (socket path + pid). Later processes read it, probe through the existing owner-only gate, reuse if live, else spawn at their canonical and rewrite. | Meets the worked example without inverting XDG bind precedence. One new artefact; must be same-uid, non-symlink, rewritten under the lock, ignored when stale. Doctor `--fix` relocating to this-process canonical stays out of scope (next ensure republishes). |
| **A** | Default-scope **canonical bind becomes physical state-home**; XDG remains a sibling for reuse of an already-running daemon. | Simplest discovery (everyone already probes state-home). Inverts `resolve_socket_dir` precedence. Loses tmpfs `/run` logout cleanup. Only coherent if `doctor --fix` also treats the anchor as canonical; otherwise an XDG-shell `--fix` relocates back to XDG and reopens the hole. Larger ADR-036 D-3/D-4 amendment. |
| **C (reject)** | Shared lock only, still spawn at per-process canonical, no advertisement. | Does not meet #4432: XDG-set winner is invisible to XDG-unset loser. |

**Recommendation: B.** Smallest contract change that actually produces one daemon. Prefer A only if Josh wants one canonical path for default scope and will take doctor + `resolve_socket_dir` in the same implementation.

Do not use `/run/user/<uid>` as the default-scope anchor: it is absent on macOS, missing without a systemd user session, and **not** in the candidate set when `XDG_RUNTIME_DIR` is set to something else.

Windows: no change.

## 4. Physical identity before compare / probe

Apply before coordinator selection, candidate list construction, and conflict counting.

1. **Directories** (lock, bind parent): `ensure_secure_runtime_dir` already refuses a symlink **leaf**. Then `canonicalize` (or equivalent `dev`+`ino`) so ancestor aliases (`/var/home/u` vs `/home/u`) collapse. This is what CIB-382 already does for the lock set; lift it to the shared candidate list.
2. **Candidate list** (`push_unique_dir` / connect dirs): unique by physical parent identity, not `PathBuf ==`. Preserve canonical-first **order** among distinct physical dirs so bind/start still use this process's canonical spelling.
3. **Live sockets:** `lstat` the socket (do not follow). Same `(dev, ino)` is one endpoint. A planted **leaf** symlink stays `SocketPathIsSymlink` and is skipped, never reused (`validate_socket_path_for_client`).
4. **Not PID-primary:** two lexical paths to one inode are one endpoint even if a PID file is missing. PID collapse (#4434) may additionally treat one process recorded in two dirs as one instance; it must not replace inode dedup, and must not hide two distinct sockets bound by two processes.

## 5. Acceptance tests

Match #4432. The current reversed-identical-set tests stay; they are not this residual.

1. **Two differently-configured shells, one daemon (real binary).** Same `HOME`, no `ANVIL_HOME`, `XDG_RUNTIME_DIR` = `/tmp/xdg-a` vs `/tmp/xdg-b`. Concurrent `anvil` ensure (bare, ADR-114). Exactly one live daemon; the other reuses. Hermetic: both XDG values are set, so `/run/user/<uid>` is not implicit. Extend `crates/anvil-cli/tests/daemon_identity.rs`.
2. **In-process candidate sets that share only state-home.** Replace the false confidence in `concurrent_cross_environment_cold_starts_converge_on_one_daemon`: disjoint runtime dirs, shared home, one spawn. Coordinator test: those two sets contend on the state-home lock, not on `min(runtime)`.
3. **Physical dedup, no false conflict.** Runtime path ancestor-alias of state-home (or `XDG_RUNTIME_DIR` under home that canonicalizes onto state-home): one physical socket, `EnsureOutcome::Reused` or `Started`, never conflict. Planted **leaf** sibling symlink still not reused (`planted_sibling_symlink_is_not_reused`).
4. **Isolated home stays exclusive.** Existing `isolated_homes_start_distinct_daemons` / `connect_candidates_anvil_home_is_exclusive`: two `ANVIL_HOME` values still two daemons; neither coordinator is default state-home; XDG/home siblings are not probed.

## 6. Non-goals

Owned elsewhere; do not fold in:

- #4433 / #4435 / #4434: heartbeat spawn gate, driver-map stall, shutdown leftover child, Windows terminate race, operations/docs residuals from the #4428 Council.
- JREL-011: bounded wait / typed outcome when the coordinator cannot be acquired (today: warn and degrade).
- JREL-005: typed same-scope conflict on `intercept status`.
- Doctor `--fix` "move the daemon to this process's canonical" unless option A is chosen (then it must follow the new canonical).
- Changing ADR-036 execution-scope boundaries, WSL/Windows refusal, or isolated-home exclusivity.
- Making different `HOME` values for the same UID share a daemon (not in #4432; the intersection is state-home of **this** `HOME`).

## Implementation notes (after Josh picks A or B)

- Keep coordinator-before-start-lock; doctor still `HeldByCaller`.
- `ipc.rs` is a protected surface; implementation needs Council on that head.
- Do not edit shared APS modules from the implementation PR; closeout evidence in the PR, #4432, and tests.
- After the code moves: correct the overclaim in `crates/anvil-intercept/ARCHITECTURE.md` (disagreeing shells → one daemon only once discovery matches the lock). Diagrams unchanged (lifecycle, not transport topology).
- Option A would need an ADR-036 note on default-scope bind location; option B would not if the advertisement stays an ensure/doctor artefact beside the existing repair lock.
