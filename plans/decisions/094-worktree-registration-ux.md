# ADR-094: Worktree Registration UX

## Status

Proposed

## Date

2026-06-29

## Context

ADR-092 made `anvil start` reach honest daemon-backed protection without MCP, by
ensuring the per-user daemon and registering the **current** worktree. That spine
shipped (ACTMO-001..012). The 2026-06-29 operator usefulness review (see
[the worktree-registration UX design](../specs/2026-06-29-worktree-registration-ux-design.md),
hardened by a four-persona planning council) found the spine answers "protect
*this* worktree when I run `anvil start` *here*" but leaves the surrounding
lifecycle undefined:

- The registration primitive is **cwd-only and implicit** —
  `register_worktree_with_daemon` is always called with `Path::new(".")`
  (`crates/anvil-cli/src/activation/daemon_registration.rs:26`); there is no
  command to register a later/other worktree.
- **Durability gap (the load-bearing finding):** the registry is **in-memory**
  with a **30 s heartbeat TTL** (`crates/anvil-intercept/src/registry.rs:69-72`)
  and reloads only persisted **fences** on startup
  (`crates/anvil-intercept/src/lib.rs:1307-1324`). A one-shot CLI registration is
  therefore evicted ~30 s after the process exits, and a daemon restart drops the
  entire set — silently reintroducing the "unprotected worktree" failure this
  work exists to kill. A 30 s lease is correct for a *live agent session*; it is
  wrong for *durable worktree membership*.
- `anvil start` outside a Git worktree is undefined (would register a junk
  session keyed to e.g. `$HOME`).
- The only membership config — the confinement allowlist in `workspace.yaml`
  (`crates/anvil-intercept/src/confinement.rs:179-202`, both structs
  `#[serde(deny_unknown_fields)]`, default mode `open`) — is unwired to
  registration, and is a fail-closed admission floor, not a protection set.
- `anvil status` plain text never lists the registered set, though `query_status`
  carries it (`worktrees: Vec<WorktreeStatusV1>`).
- The review raised a local tray/menu-bar app as a possible human-visible daemon
  vehicle, constrained to a control surface only.

The [RELEASE-PLAN](../../RELEASE-PLAN.md) names this (with DSV-046) as the
candidate `v0.9.0-beta` daemon-usefulness addendum. A durable decision is needed
on the durability model, command surface, discovery model, and the app's scope
before the work splits into implementation items, because these establish
conventions that are expensive to reverse once shipped to beta users.

## Decision

1. **Durable registration is a daemon-side persisted, TTL-exempt set — not a CLI
   heartbeat and not the session lease.** Activation-tagged
   (`claimed_agent_id:"activation-spine"`) registrations are persisted to a
   durable store under `ANVIL_HOME`, exempt from the 30 s eviction, and reloaded
   on daemon startup before accepting connections (analogous to fence loading at
   `lib.rs:1311-1324`), with an INFO "registered N worktrees on startup" line.
   Live `anvil-run` agent sessions keep the existing lease semantics. A reaper
   drops + reports registered paths whose directory is gone; the number of
   distinct registered worktrees is capped (default 64, configurable). This
   reuses the `session.register` RPC shape but **changes daemon-side semantics**
   — the design's earlier "no wire-contract change" claim is retracted for this
   path.

2. **Explicit registration lives on `anvil workspace`, not `anvil intercept`.**
   Add `anvil workspace register [PATH]` (PATH defaults to cwd; explicit path
   registers a later/other worktree), `anvil workspace unregister [PATH]`, and
   extend `anvil workspace list` to a config↔registry join.
   `commands/workspace.rs` is config-only today, so it becomes a live daemon RPC
   client; `list` defines degraded behaviour (config half renders, registered
   half shows "daemon unavailable") and joins by canonicalising allowlist paths
   with `dunce`. The `pub(super)` primitive moves to a shared `registration`
   module.

3. **The client classifies daemon errors; identity is server-authoritative.**
   `WorktreeAlreadyOwned` (same canonical path via a different spelling/symlink,
   or a client `canonicalize` fallback diverging from the server) is treated as
   "heartbeat the existing owner", not `Rejected`; `WorktreeFenced` /
   `WorktreeCascaded` is the one refusal (points at `anvil intercept unblock`);
   `SessionCapExceeded` gives a cap message. The client uses `dunce::canonicalize`
   and verifies the daemon's returned worktree matches the request before
   treating a result as a heartbeat (64-bit `SessionId` collision safety).

4. **`anvil start` outside a worktree is honest and non-fatal.** It ensures the
   per-user daemon (unless `--no-daemon`/`ANVIL_NO_DAEMON`), does not register
   cwd, reports "daemon ready; no worktree registered" with guidance, exit 0. A
   "registerable worktree" is defined via `git rev-parse`: reject bare repos and
   cwd inside `.git/`; accept linked worktrees and submodule worktrees.

5. **Global opt-in registration is bounded by the confinement allowlist — never a
   filesystem scan — and confinement membership stays distinct from registration
   membership.** `anvil workspace register --all` registers the **exact**,
   allowlist-mode `allow` entries that are live unfenced worktrees; prefix
   entries are skipped with a warning (walking them would be the forbidden scan);
   all skips are reported; `open` mode reports "no allowlist entries"; `--no-daemon`
   bypasses with a message. A **persistent** auto-registration list is a
   **separate additive top-level key** `register_on_start: [paths]` in
   `workspace.yaml` (with a config format-version bump), **not** a field on
   `AllowEntry` — because adding a field to a `deny_unknown_fields` struct makes
   an older daemon fail **closed** and collapse the confinement trust floor. The
   daemon registers `register_on_start` entries on startup. This persistent key is
   deferred to ACTMO-019 pending owner sign-off on the schema commitment.

6. **Status surfaces two distinct axes.** Membership (`registered` / `fenced` /
   `cascaded` / `unregistered`, from `WorktreeStatusV1`) and assurance
   (`clean`/`stale`/`pending`/`running`/`bounded`/`unavailable`, a parallel
   query). "stale" is dropped as a membership label (evicted sessions aren't
   listed). `protecting` vs `watching` (ADR-092) requires new `WorktreeStatusV1`
   fields or a per-worktree assurance/driver query — acknowledged as a
   wire/query addition with a fixed derivation table (ACTMO-017).
   `anvil intercept stop` reports the count of registered worktrees losing
   protection.

7. **The DSV-046 seam is owned by a registry membership-change signal.** The
   registry is the sole producer of register/unregister/reaper events; DSV-046's
   headless driver subscribes and attaches/detaches observation per worktree.
   ACTMO-013 does not invent the driver; DSV-046 does not invent membership. The
   `v0.9.0-beta` "minimum useful release" needs both the ACTMO cut-line items and
   a promoted+split DSV-046 (a parallel prerequisite).

8. **New-worktree auto-registration is a guided opt-in** via its own
   `anvil workspace install-hook` subcommand (a portable Git alias through `sh`,
   PowerShell equivalent on Windows) — not a flag on `register`, and never a
   silent `git` shim. A Worktrunk hook template ships only if Worktrunk exposes
   such a hook (design-gated, ACTMO-020).

9. **A local app, if built, is a scoped daemon-control surface only and deferred
   past the `v0.9.0-beta` cut** (start/stop, list registered, protection state,
   recent fences, register prompt — thin client over existing IPC verbs; no
   findings/graph/product UI). Tracked Proposed (ACTMO-021).

## Rationale

The daemon registration primitive is sound, but a 30 s in-memory heartbeat lease
cannot carry durable membership; making registration a persisted, TTL-exempt,
reload-on-start daemon concern (decision 1) is the only model that delivers
"register a worktree and walk away" and survives a restart. Anchoring the command
on `workspace` reuses the membership noun; classifying daemon errors and trusting
server-side canonicalisation (decision 3) removes a silent rejection trap.
Bounding discovery to the explicit allowlist (decision 5) is the only model
consistent with the scope-guard's determinism and the "no scan" constraint —
and keeping the persistent key separate from `AllowEntry` avoids turning a
registration convenience into a confinement-trust-floor outage on binary
downgrade. Keeping auto-registration a guided opt-in and the local app a
deferred, scoped control surface holds the line against scope creep into a
desktop product.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Daemon-persisted durable registration (chosen, decision 1) | Survives idle + restart; matches the "membership" semantics | Registry/daemon change; retracts "no wire change" |
| CLI-held heartbeat (a foreground/background thread per `register`) | No daemon change | Re-creates `anvil watch`'s foreground problem; dies with the process; no restart survival |
| Leave the 30 s lease as-is | Zero change | Registrations silently evaporate in 30 s — the failure this work exists to fix |
| `anvil workspace register` (chosen, decision 2) | Reuses the membership noun | Grows the `workspace` surface; makes a config-only command an RPC client |
| `anvil intercept register --worktree <path>` | Co-located with daemon status/stop | Conflates process control with membership |
| `register_on_start` field on `AllowEntry` | One struct | `deny_unknown_fields` → older daemon fails **closed**, collapsing the confinement trust floor |
| Separate `register_on_start` key + format-version (chosen, decision 5) | Confinement and registration stay distinct sets; safe forward-compat | A second key in one file |
| Global discovery by filesystem scan | "Just works" | Violates determinism + the no-scan constraint; privacy/perf risk |
| Silent `git worktree add` interception | Zero-effort | No native Git hook; would shim `git`; fragile and surprising |
| Build the local app now | Visible daemon | Scope creep; blocks the cut; CLI delivers the useful shape |

## Consequences

- **Positive:** later-created worktrees have one obvious, **durable** registration
  path that survives idle and daemon restart; `anvil start` is honest everywhere;
  status shows what is actually protected and what is losing protection on stop;
  the daemon stops feeling passive.
- **Negative:** a real registry/daemon change (persisted registered set,
  TTL-exemption, reload, reaper, cap); more surface on `anvil workspace`; a
  second config key + format-version.
- **Risks:** the local app could drift toward a product UI; Worktrunk's hook
  surface may not exist; a config format-version bump must not break existing
  files; the DSV-046 driver-attach edge must be implemented or registrations are
  "registered but not watched".
- **Mitigations:** app scope fixed to control-plane verbs and deferred
  (ACTMO-021); Worktrunk hook design-gated (ACTMO-020); format-version + back-compat
  test on ACTMO-019; the membership-change signal contract (decision 7) made an
  explicit ACTMO-014 ↔ DSV-046 dependency.

## References

- Design: [worktree-registration UX design](../specs/2026-06-29-worktree-registration-ux-design.md)
- [ADR-092](092-mcp-optional-activation-spine.md) — MCP-optional activation spine
- [ADR-061](061-save-time-daemon-delta-validation.md) — daemon-mediated save-time validation
- APS modules: ACTMO-013 (+ ACTMO-014..021), DSV-046
- [RELEASE-PLAN](../../RELEASE-PLAN.md) — v0.9.0-beta daemon usefulness addendum
