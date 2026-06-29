# Subsequent Worktree Registration UX Design (ACTMO-013)

| Type   | Authority | Owner | Status   | Freshness                          |
| ------ | --------- | ----- | -------- | ---------------------------------- |
| Design | Proposal  | Josh  | Proposed | Authored 2026-06-29; planning-council reviewed 2026-06-29 (4 personas; findings folded); pending owner sign-off |

| Upstream                                                                                                                                              | Downstream                                                                |
| --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| [ACTMO-013](../modules/activation-mcp-optional.aps.md), [ADR-092](../decisions/092-mcp-optional-activation-spine.md), [DSV-046](../modules/daemon-save-time-validation.aps.md), [RELEASE-PLAN](../../RELEASE-PLAN.md) | [ADR-094](../decisions/094-worktree-registration-ux.md), ACTMO-014..021 work items |

## Problem

`anvil start` reaches honest, daemon-backed protection without MCP (ADR-092): it
ensures the per-user daemon, registers the **current** worktree, installs git
hooks, and arms save-time validation. The MCP-optional spine shipped
(ACTMO-001..012, module 12/12).

The 2026-06-29 operator usefulness review found the spine answers "how does
*this* worktree get protected when I run `anvil start` *here*" but leaves the
surrounding lifecycle undefined:

1. **Outside a worktree.** `anvil start` always registers the current working
   directory as the worktree
   (`crates/anvil-cli/src/activation/daemon_registration.rs:26` is always called
   with `Path::new(".")`). What it does when cwd is not a Git worktree is
   undefined — it would register a junk session keyed to e.g. `$HOME`.

2. **Later-created worktrees.** A per-user daemon may already be running. When a
   developer creates a second Git/Worktrunk worktree an hour later, there is **no
   command** to register it — registration is implicit and cwd-only on
   `anvil start` (zero matches for `register.*path` / `register.*later`).

3. **No discovery / automation.** There is no bounded way to say "protect all my
   in-scope worktrees" or "auto-register new worktrees as I create them".

4. **Status is opaque.** `anvil status --json` carries the registered set
   (`worktrees: Vec<WorktreeStatusV1>` with `worktree` / `session_id` / `fenced`
   / `cascaded`, `crates/anvil-intercept-proto/src/status.rs:128-143`), but the
   plain-text renderer never lists registered vs unregistered worktrees, so a
   human cannot see what the daemon is protecting.

5. **No human-visible daemon vehicle.** The daemon is invisible infrastructure;
   the review asked whether a small local tray/menu-bar app should be the
   human-visible vehicle for control.

This is the difference between "the daemon is technically running" and "the
daemon is *useful*". The [RELEASE-PLAN](../../RELEASE-PLAN.md) names ACTMO-013 +
DSV-046 as the candidate `v0.9.0-beta` daemon-usefulness addendum.

This document resolves the registration UX, proposes a `Proposed`
[ADR-094](../decisions/094-worktree-registration-ux.md), defines a test matrix,
and splits the work into Ready / Proposed items. It is a **proposal**; the
headline decisions are flagged for owner sign-off. It has been hardened by a
four-persona planning council (see §"Planning-council review") — the most
important outcome of that review was discovering that the original draft built
*durable membership* on a *30-second heartbeat lease*; D4 below now owns that
decision explicitly.

## The load-bearing constraint: registration is membership, not a session lease

The existing registry is **in-memory** with a **30-second heartbeat TTL** and a
250 ms eviction tick (`crates/anvil-intercept/src/registry.rs:69-72,1325-1384`,
header comment "INTD-003: in-memory session registry"). `run_foreground` reloads
only persisted **fences** on startup (`crates/anvil-intercept/src/lib.rs:1307-1324`)
— there is no session restore.

`register_worktree_with_daemon` issues exactly one `session.register` RPC and
returns; no CLI command spawns a heartbeat loop. Therefore, **as the primitive
stands, any registration evicts ~30 s after the invoking process exits, and a
daemon restart / reboot drops the entire registered set silently.** That is the
exact "silently unprotected" failure this module exists to kill.

The current 30 s lease is the right model for a **live agent session** (an
`anvil-run`-wrapped agent whose liveness should expire when it crashes). It is
the wrong model for **durable worktree membership** ("this worktree is in the
protected set until I say otherwise"). This design separates the two concepts and
makes durable membership a first-class, daemon-owned, persisted concern (D4).

## Boundary with DSV-046 (no overlap)

ACTMO-013 and DSV-046 are two halves of the same usefulness gap and must agree:

- **DSV-046 owns the headless save-time *driver*** — who observes the filesystem
  unattended (daemon-owned watcher vs managed sidecar vs honest copy downgrade),
  its lifecycle, resource limits, restart recovery, and findings surfacing with
  no terminal. Today `anvil watch` is a **foreground** client feeding
  `validate_paths`; the daemon owns no background watcher
  (`crates/anvil-cli/src/commands/watch.rs`, `watch_save_time.rs`).
- **ACTMO-013 owns the *registration UX*** — how a worktree becomes (and is seen
  to be) a durable member of the protected set.

**Shared contract (the dynamic seam, per council M3):** the daemon's registry is
the single producer of membership-change events. When the durable registered set
changes (register / unregister / reaper-drop), the registry emits a
membership-change signal; **DSV-046's driver subscribes to that signal** and
attaches/detaches its observation per worktree. ACTMO-013 does not invent the
driver; DSV-046 does not invent membership. The registry already emits an
eviction signal for DSV-040 warm-state reclamation; if that signal does not yet
cover **additions**, surfacing additions is a named dependency DSV-046 must
consume (tracked on ACTMO-014 ↔ DSV-046).

**Cut-line honesty (council pragmatic MAJOR):** DSV-046 is itself a single
`Proposed` design item, not yet split into Ready implementation work. The
ACTMO-013 cut-line items below deliver the **registration UX and honest status**;
they do **not** by themselves deliver unattended validation. The
[RELEASE-PLAN](../../RELEASE-PLAN.md) "minimum useful release shape" requires
**both** ACTMO-014..017 **and** a promoted+split DSV-046. Promoting ACTMO's
Ready items without DSV-046 ships honest registration + status but still
foreground-driven validation; the cut-line statement is qualified accordingly and
DSV-046's split is a parallel prerequisite.

## Scope-Guard Alignment

Per `docs/vision/anvil-scope-guard.md`:

- **Increases prevention capability?** Yes — durable membership (D4) closes the
  gap where a created worktree silently goes unprotected.
- **Deterministic?** Yes — registration identity is a deterministic hash of the
  canonical worktree path; discovery (D5) is bounded by an explicit config list,
  never a filesystem scan.
- **Warnings over blocks?** Yes — outside-worktree `start` and an unregistered
  cwd warn and guide (exit 0). The one refusal is registering a **fenced**
  worktree, which is an existing fence-gate behaviour, surfaced honestly.
- **New edges only?** N/A (UX/orchestration, not a finding rule).

The one genuine scope risk is the **local app**; the work item pre-constrains it
to a control surface only. This design holds that line: design-only, deferred
past the cut (D8).

## What exists today (grounding)

| Concern | Reality | Citation |
| ------- | ------- | -------- |
| Registration primitive | `register_worktree_with_daemon(&Path)` (`pub(super)`, always called with `"."`) → `session.register`; deterministic `SessionId = sess_activation_{sha256(canonical)[..8]}`; `AgentTag{driver_id:"anvil-start", claimed_agent_id:"activation-spine"}` | `activation/daemon_registration.rs:12-36,100-120` |
| Client canonicalisation | `canonicalise_for_registration` calls `std::fs::canonicalize` and **falls back to the raw path on error** | `daemon_registration.rs:100-108` |
| Duplicate handling | `session already registered` → `session.heartbeat` → `Refreshed`; **but `WorktreeAlreadyOwned` (different SessionId, same path) is NOT matched** and falls through to `Rejected` | `daemon_registration.rs:45-46,143,163-171`; `registry.rs:602-608` |
| Outcomes | `Registered` / `Refreshed` / `DaemonUnavailable` / `Rejected(String)` | `daemon_registration.rs:18-24` |
| Registry model | **in-memory**; composite key `(canonical_worktree, Option<AgentTag>)`; 30 s TTL; 250 ms eviction; `DEFAULT_PER_WORKTREE_CAP = 16`; `by_worktree` is an **unbounded** map of distinct worktrees | `registry.rs:1,69-72,80,427-437,1325-1384` |
| Startup restore | reloads persisted **fences** only; **no session restore** | `lib.rs:1307-1324` |
| Fence gate | fenced/cascaded worktree rejects registration before registry mutation (`WorktreeFenced`/`WorktreeCascaded`) | `lib.rs` (RegistryDispatcher) |
| `anvil workspace` | **config-file editor only, no daemon IPC**: `mode`/`allow`/`deny`/`list` over `workspace.yaml` (`admission: open\|allowlist`, `allow: Vec<AllowEntry>{path, match}`, both `#[serde(deny_unknown_fields)]`, default mode `open`) | `commands/workspace.rs:36-48`; `confinement.rs:62,179-202` |
| `anvil intercept` | `start`/`status`/`unblock`/`stop` — no `register`; `stop` SIGTERMs and prints PID only | `commands/intercept.rs:41-66,129` |
| `anvil status` | plain text omits the worktree list; `--json` carries `WorktreeStatusV1{worktree, session_id, fenced, cascaded}` — **no assurance/heartbeat/watching field** | `commands/status.rs`; `status.rs:128-143` |
| Config / env | `ANVIL_HOME` + XDG/HOME; presence-based env toggles with explicit false spellings (`ANVIL_NO_MCP`, `ANVIL_NO_DAEMON`, `ANVIL_WATCH_DAEMON=0\|1`) | `install_root.rs:99-122`; `confinement.rs:388-444`; `start.rs:731,735` |
| `dunce` | already in the workspace lock (strips Windows `\\?\` UNC display paths) | `Cargo.lock` |
| Worktrunk | no integration — zero matches across crates and docs | (grep) |

The primitive is solid; the **durable-membership + UX layer above it is missing**.

## Decisions

### D1 — `anvil workspace register` is the explicit registration surface

Add registration to the existing `anvil workspace` noun (not `anvil intercept`,
not a new top-level verb):

- `anvil workspace register [PATH]` — register a worktree (PATH defaults to cwd;
  explicit path registers a later/other worktree).
- `anvil workspace unregister [PATH]` — `session.unregister` (idempotent).
- `anvil workspace list` — extend to show registered worktrees alongside
  confinement entries.

**Why `workspace`:** `intercept` is daemon **process** control; `workspace` is
the membership/confinement noun. Registration is membership.

**Honest scope notes the council surfaced:**
- `commands/workspace.rs` is **config-only today** — it does no daemon IPC.
  `register`/`unregister` make it a **live daemon RPC client**, and `list`
  becomes a **join** of static config and live `session.list`. This is a real
  responsibility expansion, scoped on ACTMO-015, not "just wiring".
- The primitive is `pub(super)` inside `activation`
  (`daemon_registration.rs:26`); ACTMO-014 relocates it to a shared
  `registration` module callable from both `activation` and `commands::workspace`
  (small module-boundary refactor, not a verbatim reuse).
- `list`'s **degraded behaviour** is defined: when the daemon is unreachable the
  config half still renders and the registered half shows "daemon unavailable"
  (consistent with D2's honesty rule). The config↔registry **join** canonicalises
  allowlist paths via `dunce::canonicalize` before comparing to the registry's
  canonical keys; a path that fails to canonicalise renders "allowlisted; path
  unresolved" rather than a false "not registered".

### D2 — `anvil start` outside a worktree: honest, non-fatal, guided

When cwd is not a registerable Git worktree, `anvil start`:

1. **Does not fail and does not register cwd.**
2. **Still ensures the per-user daemon** (the daemon is per-user) unless
   `--no-daemon` / `ANVIL_NO_DAEMON`.
3. **Reports honestly**: "daemon ready; no worktree registered (run from inside a
   worktree, or `anvil workspace register <path>`)" — an honest state distinct
   from `protecting` (ADR-092 honest-state rule).
4. Exit 0.

**"Registerable worktree" is defined precisely** (council Minor-1), because no
git-specific check exists today: resolve via `git rev-parse --show-toplevel`;
**reject** when cwd is inside `.git/` (`--is-inside-git-dir`) or a **bare** repo
(`--is-bare-repository`); **accept** linked worktrees and submodule worktrees
(`.git` as a file pointer) — these are valid independent worktrees. The resolved
top-level is the path that is canonicalised and registered. This helper is shared
by D1 and D2 (ACTMO-014).

### D3 — Re-registration is idempotent; the client must classify daemon errors

Re-registering an already-registered worktree is **not an error**. Today only the
exact-`SessionId` "session already registered" case heartbeats; the council found
two unhandled paths the client currently turns into an opaque `Rejected`:

- **`WorktreeAlreadyOwned`** (same canonical path reached via a different spelling
  → different client `SessionId`, or the client's `canonicalize` fell back to a
  raw path that the server canonicalised differently). The client **must treat
  `WorktreeAlreadyOwned` as "heartbeat the existing owner"**, not a rejection.
  Server-side canonicalisation is authoritative for identity; the client stops
  trusting its own fallback path for the *outcome decision*.
- **`WorktreeFenced` / `WorktreeCascaded`** → the one genuine refusal; the CLI
  reports it clearly and points at `anvil intercept unblock`.
- **`SessionCapExceeded`** → a clear cap message.

This **error-classification layer is new code** in ACTMO-014/015 (today
`orchestrator/mod.rs:289` collapses every `Rejected` into a `tracing::warn!`);
the design calls it out so implementers build it. The client also uses
`dunce::canonicalize` (already in-tree) so identity is stable and display paths
are free of `\\?\` on Windows (council Minor). Note: `SessionId` is a 64-bit hash
prefix; on the (rare) collision the client verifies the daemon's returned
worktree matches the requested one before treating a result as a heartbeat, so a
collision degrades to an honest re-register attempt rather than a silent
heartbeat of the wrong session (council Minor-3).

### D4 — Registration durability: a daemon-side persisted, TTL-exempt set (CRITICAL)

Durable membership is **not** a CLI heartbeat and **not** the 30 s session lease.
The daemon owns a **persisted registered-worktree set**:

1. Activation-tagged (`claimed_agent_id:"activation-spine"`) registrations are
   recorded to a **durable store under `ANVIL_HOME`** (a `registered-worktrees`
   record alongside the existing fence store) and are **exempt from the 30 s TTL
   eviction** (they are membership, not liveness). Live `anvil-run` agent
   sessions keep the existing lease semantics unchanged.
2. On startup, `run_foreground` **reloads the persisted registered set before
   accepting connections** — exactly analogous to how it already loads persisted
   fences (`lib.rs:1311-1324`) — and emits an INFO event
   "registered N worktrees on startup". This makes registration survive idle,
   daemon crash, reboot, and upgrade.
3. **Reaper:** on reload and on a periodic sweep, a registered path that no longer
   exists (e.g. `git worktree remove`d) is dropped and **reported** (INFO log +
   reflected in status), never silently retained.
4. **Global cap:** the number of **distinct** registered worktrees is capped
   (default 64, configurable alongside the existing
   `enforcement.session.per_worktree_max`); `register` past the cap returns a
   clear error. This bounds the otherwise-unbounded `by_worktree` map (council
   ops MAJOR).

**This is a registry/daemon change** — the design **retracts the earlier
"no wire-contract change" claim** for the durable path. The `session.register`
RPC shape is reused, but daemon-side semantics for activation-tagged
registrations (persist + no-evict + reload + reaper + cap) are new. This is the
keystone cut-line item (ACTMO-014); nothing else is honestly "Ready" until it
lands.

### D5 — Bounded global opt-in (allowlist-scoped, never a filesystem scan)

Two layers, deliberately separated to avoid the council's downgrade-bomb and
conflation findings:

- **Manual, no schema change (cut-line, ACTMO-018):**
  `anvil workspace register --all` registers the **exact**, allowlist-mode
  `allow` entries of `workspace.yaml` that are live, unfenced Git worktrees.
  - **Prefix entries are skipped with an explicit warning** ("N prefix entries
    skipped — only exact entries can be registered with --all"); walking them
    would be the forbidden filesystem scan (council Major-1).
  - Skipped entries (prefix / fenced / gone / not-a-worktree) are **reported, not
    silent** (council architect m3).
  - **`open` mode** (the planless-first default) has an empty/ignored allowlist,
    so `--all` honestly reports "no allowlist entries (confinement mode: open)"
    rather than appearing broken (council M1.2).
  - `--no-daemon` / `ANVIL_NO_DAEMON` **bypasses all registration** including
    `--all`, with the message "registration skipped (--no-daemon)" (council
    Major-2).
  - Registration RPCs are issued with the existing 500 ms per-call timeout; for a
    large allowlist `--all` prints per-entry progress and has a batch budget so it
    cannot read as a hang (council Minor-2).

- **Persistent auto-registration config (NOT cut-line, owner-gated,
  ACTMO-019):** a list of in-scope roots the **daemon** registers on startup. To
  avoid the `deny_unknown_fields` downgrade bomb (a new field on `AllowEntry`
  makes an **older** daemon fail **closed** and collapse the confinement trust
  floor — council Critical-2 / architect M1.1), this is a **separate additive
  top-level key** in `workspace.yaml` (`register_on_start: [paths]`), **not** a
  field on `AllowEntry`, with a config-file format-version bump and documented
  forward/back-compat. The in-scope set is an explicit operator-curated list —
  determinism preserved, no filesystem scan, and confinement admission and
  registration membership stay **distinct sets** (they genuinely are two
  different things: "what the daemon may serve" vs "what is actively
  registered"). This item is `Proposed` pending owner sign-off on the schema
  commitment.

### D6 — Status surfaces two distinct axes, honestly

The council found the original "watching / protecting / stale" list conflated two
different things and over-claimed "no wire change". The corrected model:

- **Membership axis** (derivable from `WorktreeStatusV1` today): `registered` /
  `fenced` / `cascaded`; a worktree on disk but absent from the registered set is
  `unregistered`. "stale" is **dropped** as a membership label — an evicted
  session simply isn't listed, so there is no observable "stale session" state.
- **Assurance axis** (the existing `AssuranceState`: `clean` / `stale` /
  `pending` / `running` / `bounded` / `unavailable`): a **parallel query**, not a
  field on `WorktreeStatusV1`.
- **`protecting` vs `watching`** (ADR-092 distinction) is **not free**: it
  requires either new fields on `WorktreeStatusV1` (a wire extension) or a
  per-worktree assurance/driver query. The design **acknowledges this as a
  wire/query addition** owned by ACTMO-017, with an explicit derivation table in
  that item, e.g. `protecting = registered ∧ ¬fenced ∧ assurance∈{clean}`;
  `watching = registered ∧ DSV-046 driver attached`; `fenced = fenced`. The
  derivation table is fixed in the work item before it is Ready.

`anvil status` (plain text) gains a registered-worktrees section flagging whether
the **current cwd** is registered. `anvil intercept stop` first best-effort
queries `session.list` and prints "stopping daemon; N worktree(s) registered
(re-register with `anvil workspace register` or run `anvil start`)" when N > 0,
and the daemon shutdown path emits one INFO event with the count (council ops
MAJOR).

### D7 — New-worktree auto-registration: a separate, portable, guided opt-in

Git has no native post-`worktree add` hook, so Anvil cannot transparently
intercept creation. Provide a guided opt-in as its **own subcommand**
(`anvil workspace install-hook`, **not** a flag on `register` — council pragmatic
MINOR):

- It installs a documented **Git config alias** pinned to a portable form that
  runs through `sh` on every platform Git supports (incl. Git-for-Windows
  MinGW). The exact one-liner must stay **POSIX `sh`/dash-safe** — no bashisms
  such as `${@: -1}` — e.g. capture the last positional with a POSIX loop:
  `git config --global alias.wt-add '!f() { git worktree add "$@" && p=; for a in "$@"; do p=$a; done && anvil workspace register "$p"; }; f'`.
  ACTMO-020 finalises the exact form (including the `git worktree add <path>
  [<commit-ish>]` edge case where the last arg is a commit-ish, not the path)
  and prints a **PowerShell equivalent** when it detects Windows without `sh`
  (council ops MAJOR). It does not silently shim `git`.
- **Worktrunk:** no integration exists today. If Worktrunk exposes a post-create
  hook, Anvil ships a hook **template** calling `anvil workspace register`; until
  that surface is confirmed this stays **design-only / Proposed** (ACTMO-020).

### D8 — Local daemon-control app: designed, scoped, deferred

Accepted as a future vehicle, **scoped strictly to daemon control** and **not
built for `v0.9.0-beta`**:

- **In scope (if built):** start/stop the daemon, list registered worktrees, show
  protection state and recent fences, prompt to register the current worktree —
  a thin client over existing `query_status` / `session.list` / `session.register`
  / `session.unregister` / `unblock` verbs.
- **Out of scope:** any findings/graph UI, config editor beyond
  register/unregister, or a separate product surface.
- **Deferred:** the CLI surface (D1–D6) is the minimum useful release. Tracked
  `Proposed` (ACTMO-021).

## Proposed test matrix

### Cut-line gate (ACTMO-014..017 + Windows parity)

| # | Scenario | Expected |
| - | -------- | -------- |
| 1 | `anvil start` in a non-worktree dir | exit 0; daemon ensured; "no worktree registered" guidance; cwd not registered (D2) |
| 2 | `anvil workspace register` (no path) in a worktree | current worktree registered; `Registered` reported (D1) |
| 3 | `anvil workspace register <explicit path>` | named worktree registered without changing cwd (D1) |
| 4 | **durability**: register, let the process exit, wait > 30 s | worktree **still registered** (TTL-exempt persisted set, D4) |
| 5 | **restart recovery**: register, restart the daemon (or reboot) | registered set **reloaded** on startup; INFO "registered N worktrees" (D4) |
| 6 | re-register an already-registered worktree | `Refreshed` (heartbeat), exit 0, not an error (D3) |
| 7 | register the same worktree via a different path spelling / symlink | classified as `WorktreeAlreadyOwned` → heartbeat existing owner, not `Rejected` (D3) |
| 8 | register a **fenced** worktree | refused `WorktreeFenced`; CLI points at `anvil intercept unblock` (D3) |
| 9 | **reaper**: register, then `git worktree remove` the dir | dropped + reported on sweep/reload; not silently retained (D4) |
| 10 | **cap**: register past the distinct-worktree cap | clear error; cap enforced (D4) |
| 11 | multiple worktrees on one daemon | each appears once in `session.list`; composite-key uniqueness holds |
| 12 | `anvil status` after registering 2 worktrees | plain-text lists both with membership + assurance; current cwd flagged registered/unregistered (D6) |
| 13 | `anvil intercept stop` with N registered | prints "N worktree(s) registered" guidance; daemon logs the count (D6) |
| 14 | `register --all` over an exact, allowlist-mode set | only live unfenced exact entries registered; prefix/fenced/gone **skipped + reported**; no filesystem scan (D5) |
| 15 | `register --all` in `open` mode | honest "no allowlist entries (confinement mode: open)" (D5) |
| 16 | `register --all` / start with `--no-daemon` | all registration skipped + message (D5/D2) |
| 17 | Windows named-pipe parity for register/list/unregister | same outcomes as Unix socket; display paths free of `\\?\` via `dunce` (transport split + D3) |
| 18 | bare repo / cwd inside `.git` / submodule / linked worktree detection | bare + `.git`-internal rejected; submodule + linked worktree accepted (D2) |

### Post-cut / Proposed-gated

| # | Scenario | Expected |
| - | -------- | -------- |
| P1 | `register_on_start` config entries after daemon restart | daemon auto-registers them on startup (D5 / ACTMO-019) |
| P2 | older daemon binary reads a `register_on_start`-bearing config | no fail-closed confinement collapse (format-version handling, D5 / ACTMO-019) |
| P3 | `install-hook` then `git wt-add` (incl. Windows `sh` + PowerShell forms) | new worktree auto-registers (D7 / ACTMO-020) |
| P4 | newly-created Worktrunk worktree becomes protected with no visible watch terminal | protected via registration + DSV-046 headless driver (D7 + DSV-046) |
| P5 | local-app-mediated registration (if app built) | app issues the same `session.register`; worktree appears in `anvil status` (D8 / ACTMO-021) |

## Proposed work-item split

If accepted, ACTMO-013 (the design) is marked **Done** and splits into:

| Item | Title | Status | Cut-line | Owns / depends |
| ---- | ----- | ------ | -------- | -------------- |
| ACTMO-014 | Durable registration primitive (daemon-persisted, TTL-exempt, reload-on-start, reaper, cap; shared module; error classification incl. `WorktreeAlreadyOwned`→heartbeat; `dunce`; registerable-worktree helper) | **Ready** | yes (keystone) | D2/D3/D4; ↔ DSV-046 membership-change signal |
| ACTMO-015 | `anvil workspace register` / `unregister` + `list` join semantics + degraded-daemon behaviour | **Ready** | yes | D1; dep 014 |
| ACTMO-016 | Outside-worktree `anvil start` honest behaviour + registerable-worktree detection | **Ready** | yes | D2; shares helper w/ 014 |
| ACTMO-017 | Registered-worktree status surfacing (membership + assurance axes, derivation table, `intercept stop` reporting) | **Ready** | yes | D6; dep 014; soft-dep DSV-046 for the `watching` label |
| ACTMO-018 | `anvil workspace register --all` over exact allowlist entries (skip+report prefix/fenced/gone; `--no-daemon` bypass) | **Ready** | additive | D5; dep 014/015 |
| ACTMO-019 | Persistent `register_on_start: [paths]` config key + daemon startup registration (format-version + forward-compat) | **Proposed** (schema commitment; owner sign-off) | no | D5; dep 014 |
| ACTMO-020 | Guided new-worktree auto-registration (`workspace install-hook` + Worktrunk template) | **Proposed** (design-gated on Worktrunk hook surface) | no | D7 |
| ACTMO-021 | Scoped local daemon-control app | **Proposed** (deferred past cut) | no | D8 |

The four **Ready, cut-line** items (014–017) deliver durable registration and
honest status. The [RELEASE-PLAN](../../RELEASE-PLAN.md) "minimum useful release
shape" additionally requires a promoted+split **DSV-046** for the unattended
validation half (see §"Boundary with DSV-046"); that split is a parallel
prerequisite, not owned here.

## Headline decisions for sign-off

1. **D4** — durable registration is a daemon-side persisted, TTL-exempt,
   reload-on-start set (retracts the "no wire change" claim; the keystone).
2. **D1** — `anvil workspace register` (vs `anvil intercept register`).
3. **D5** — `--all` over exact allowlist entries now; the persistent
   `register_on_start` config as a **separate additive key** with a format-version
   bump (not a field on `AllowEntry`), deferred to ACTMO-019.
4. **D8** — local app accepted as a scoped, deferred daemon-control vehicle only.

## Planning-council review (2026-06-29)

Four personas reviewed the draft (architect, adversarial, operations,
pragmatic-lead). Verdict: the UX layer and the work-item shape were sound, but
the draft rested on an unexamined assumption — a 30 s in-memory heartbeat lease
as a substrate for durable membership. Disposition of the load-bearing findings:

- **Durability (CRITICAL, unanimous)** → new D4 (daemon-persisted, TTL-exempt,
  reload-on-start, reaper, cap); "no wire change" retracted; ACTMO-014 made the
  keystone.
- **Canonicalize divergence / `WorktreeAlreadyOwned` opaque rejection
  (CRITICAL)** → D3 error-classification layer + `dunce`; server-authoritative
  identity.
- **`deny_unknown_fields` downgrade bomb (CRITICAL)** → D5 splits the persistent
  config into a separate additive key with format-versioning; confinement and
  registration stay distinct sets.
- **D4 conflation / open-mode / prefix entries (MAJOR)** → D5 scoping +
  skip-and-report + honest open-mode message.
- **`workspace.rs` config-only → live RPC client; `list` join + degraded
  behaviour (MAJOR)** → D1 honest-scope notes.
- **DSV-046 dynamic seam unowned + DSV-046 unsplit (MAJOR)** → membership-change
  signal contract + qualified cut-line.
- **Unbounded registered set / `intercept stop` silence (MAJOR)** → D4 cap + D6
  stop reporting.
- **`--no-daemon`, `--install-new-worktree-hook` portability, watching/protecting
  data source (MAJOR)** → D5 bypass, D7 separate portable subcommand, D6 two-axis
  model with acknowledged wire/query cost.
- **Minors** (git-worktree detection, `--all` latency, SessionId collision,
  reaper, test-matrix split) → folded into D2/D3/D4/D6 and the split matrix.

The ADR captures the durable decisions; this proposal awaits owner sign-off.

## References

- [ADR-092](../decisions/092-mcp-optional-activation-spine.md) — MCP-optional activation spine
- [ADR-094](../decisions/094-worktree-registration-ux.md) — worktree registration UX (this design's decision record)
- [ADR-061](../decisions/061-save-time-daemon-delta-validation.md) — daemon-mediated save-time validation
- [activation-mcp-optional](../modules/activation-mcp-optional.aps.md) — ACTMO module (ACTMO-013 + split items)
- [daemon-save-time-validation](../modules/daemon-save-time-validation.aps.md) — DSV-046 headless driver contract
- [RELEASE-PLAN](../../RELEASE-PLAN.md) — v0.9.0-beta daemon usefulness addendum
