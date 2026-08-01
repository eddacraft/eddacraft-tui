# ADR-114: Bare `anvil` as Daily Ensure Surface

## Status

**Accepted** — 2026-08-01 (operator: ship before `v0.10.0-beta` cut). Product
command split: bare `anvil` is the daily on-switch (daemon ensure + existing
MCP ensure); `anvil start` remains activate / reconfigure / reinstall.
Implementation: [ONSW](../modules/bare-ensure.aps.md). Conductor acceptance:
[JOURNEY-011](../modules/release-user-journeys.aps.md).

### Open questions resolved (accept)

1. **Never-activated predicate:** on-disk anvil project config is **Absent**
   (`activation::verify` config status) — no silent MCP/workflow/hook/init
   writes; recovery names `anvil start` / `anvil welcome`.
2. **Rollout:** **default-on** in the implementing PR (no feature gate);
   fixtures pin exit codes and no-install guarantees.
3. **Bare `--json`:** honour global `--json` with a compact ensure document in
   v1 (same global flag as other commands).
4. **Exit codes:** `0` success; `1` not-activated or general ensure failure;
   daemon ensure failure is non-zero with recovery copy (prefer exit `1` with
   clear recovery rather than claiming reserved `EXIT_DAEMON_DOWN` until that
   constant is product-emitted elsewhere).

## Date

2026-08-01

## Context

`anvil start` is the canonical activation path (ADR-082, ADR-092, ADR-103). It
owns consent-gated writes (MCP entries, workflows, hooks, init) and daemon
ensure. JOURNEY-002 / JOURNEY-003 shipped the "just works" and "healthy repeat"
gates on that command.

Two product frictions remain:

1. **Bare `anvil` is not the power switch.** Root clap uses
   `arg_required_else_help`: bare invocation exits 2 with long help. CIB-177 only
   added a first-run pointer (`anvil welcome` / `anvil start`) without changing
   parse or exit codes. Most users expect bare CLI names to *do something*
   useful.

2. **Healthy re-runs of `start` re-offer declined installs.** Consent offers are
   rebuilt from disk state (`NotPresent` / `SafeDrift` / pending workflows), not
   from a durable "user declined" preference. Enter-through with nothing ticked
   writes nothing; the next interactive `start` re-offers the same MCP and
   workflow rows. That is correct for a **reconfigure** surface and wrong for a
   **daily on-switch**.

JOURNEY-003 collapsed *output* on healthy protecting re-runs, but left the
consent re-offer path on the same command. The daily path and the install path
are still one verb.

Design discussion (2026-08-01) settled a clean split: keep `anvil start` as the
reinstall / reconfigure option; make bare `anvil` the subsequent on-switch that
ensures the daemon and already-owned MCP without reinstalling or re-prompting.

This ADR amends the product surface posture from ADR-082 §Decision ("the product
surface should be `anvil start`, `anvil watch`, and `anvil status`") by adding
**bare `anvil` as the daily ensure surface**, without removing `start`'s
activation ownership.

## Decision

### 1. Command split

| Command | Role |
| ------- | ---- |
| **Bare `anvil`** | Daily **ensure** surface: turn protection on for this worktree without first-time install consent. Idempotent daemon ensure, worktree attestation when already in the spine, MCP ensure for **already present** anvil-owned entries only. |
| **`anvil start`** | **Activate / reconfigure / reinstall**: full consent plan (MCP NotPresent, workflows, hooks, init, identity, SafeDrift repair policy). Remains the recovery path after uninstall, decline, or deliberate reconfiguration. |
| **`anvil welcome`** | First-run tour / repository-specific first win (unchanged; ADR-080). |
| **`anvil status` / `anvil watch`** | Report and sustained watch (unchanged ADR-082 tiering for watch). |

### 2. Bare `anvil` behaviour contract

#### 2.1 When ensure runs

Bare `anvil` runs the ensure path when **all** of:

- no subcommand was parsed (true bare root invocation);
- the process is not solely requesting help (`--help` / `-h` still print the
  full command catalogue, including the updated bare-role blurb);
- the invocation is not a machine-only help probe that must preserve the
  pre-ADR-114 help contract (see §4 rollout).

#### 2.2 Ensure steps (ordered)

1. **Resolve worktree.** If not in a registerable git worktree, print a short
   honest message and exit non-zero (or 0 with advisory — pin in ONSW-002). Do
   not invent project state outside a repo.
2. **Daemon ensure.** Idempotent per-user daemon ensure (same primitive as
   ADR-082 / DLIFE `ensure_daemon`). Honour `--no-daemon` / `ANVIL_NO_DAEMON`
   if those flags are accepted on the root (prefer global flags already on
   `Cli`; do not invent a second opt-out namespace).
3. **Worktree spine.** If the worktree is already durable-registered or
   activation spine can attest without new consent-gated project writes, register
   / re-attest per ACTMO / ADR-094 semantics. Bare must **not** perform first-time
   project init, workflow install, or hook install.
4. **MCP ensure-only.**
   - If an anvil-owned MCP entry is **present** (`UpToDate` or `SafeDrift` under
     ADR-044): verify; apply SafeDrift rewrite only under the same ownership
     rules as activation (one-line notice). Never `UnsafeDrift` overwrite.
   - If MCP is **NotPresent** (user never installed or declined): **do not
     install and do not re-offer a picker**. Emit one recovery line naming
     `anvil start` (or `anvil start --no-mcp` posture as appropriate).
   - Honour `ANVIL_NO_MCP` / root-equivalent: skip MCP ensure entirely.
5. **Output.** Healthy path: short confidence summary in the JOURNEY-003 shape
   (protection / daemon / worktree posture, at most one next action). Degraded
   paths keep actionable detail. No activation TUI, no workflow picker, no
   multi-select consent.

#### 2.3 First-run / never-activated honesty

If the repository has never completed activation (no project config / no spine
membership / no prior successful start evidence — exact predicate in ONSW-002):

- bare **must not** silently run the full `start` consent plan;
- bare **must** either (a) run pure daemon ensure if safe and report "not
  activated — run `anvil start`", or (b) print the first-run pointer and exit
  without writes beyond what daemon ensure requires.

Preference: **honest pointer + optional daemon ensure**, never surprise MCP or
workflow writes.

#### 2.4 Non-interactive / CI / piped

| Context | Bare behaviour |
| ------- | -------------- |
| Interactive TTY | Ensure path + short human summary |
| Piped / non-TTY / `CI` / `ANVIL_NO_PROMPT` | Deterministic ensure **or** explicit refuse — **no prompts, no hang**. Prefer ensure with compact plain/JSON if `--json` is on the root; otherwise compact plain. |
| `--help` / `-h` | Full help catalogue (exit 0); first-run pointer and bare-role blurb in `before_help` |

Exit codes for ensure success/failure are owned by ONSW-004 and must be
fixture-pinned. CIB-177's "bare always exits 2" contract is **superseded** for
the ensure path only; help-only bare behaviour remains help-shaped.

### 3. What bare must never do

- Install MCP for `NotPresent` clients
- Offer or install GitHub Actions workflows
- Install git hooks for the first time
- Run interactive consent chrome (ACTTUI)
- Treat decline-as-not-this-run as permanent preference **on bare** (decline
  simply means "still NotPresent"; only `start` re-opens install)
- Claim L0 "active" when only config is present and the editor has not attached
  (honesty pins from LAUNCH-014 / CIB-164)

### 4. Rollout

1. **Proposed → Accepted** this ADR (operator).
2. **ONSW-001..004** implement spine, MCP ensure-only, contracts, and docs.
3. **JOURNEY-011** product acceptance: first-run honesty, healthy subsequent
   bare ensure, declined-install no re-offer on bare, `start` still reconfigures.
4. Optional staged flip: if risk requires it, gate bare-ensure behind
   `ANVIL_BARE_ENSURE=1` for one release, then default-on. Prefer default-on in
   the same PR if fixtures fully pin exit codes and help; otherwise one-release
   opt-in is acceptable.

### 5. Documentation and help

- `docs/runbooks/cli-surface.md`: document bare as ensure; `start` as
  activate/reconfigure.
- Root `before_help` / `FIRST_RUN_POINTER`: reword so day-one users see
  `welcome` / `start`, and subsequent use is implied by bare (exact copy in
  ONSW-005).
- Public quickstart: "after first `anvil start`, bare `anvil` keeps protection
  on."

## Rationale

Splitting **on** from **install** matches user mental models and removes the
JOURNEY-003 / consent re-offer contradiction without inventing a durable
decline store. `start` keeps the hard consent work (CIB-165/184, ACTTUI,
ADR-044). Bare reuses existing ensure primitives (daemon, SafeDrift MCP) so the
implementation surface is thin.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **Chosen: bare = ensure; start = reconfigure** | Clear verbs; fixes re-offer friction; thin reuse of ensure | Breaks CIB-177 exit-2 bare contract; needs careful first-run honesty |
| Alias bare → full `start` | One implementation | Re-offers declined installs; TUI/consent on every bare type | 
| Persist "declined forever" prefs for `start` | Quiet re-runs of `start` | Preference store, reset UX, ADR-044 repair conflicts; still no bare power switch |
| New subcommand `anvil on` / `anvil ensure` | Explicit | Worse discoverability than bare; two new names to teach |
| Keep bare as help only | Zero contract break | Leaves daily path on the wrong verb |

## Consequences

- **Positive:** Daily on-switch is discoverable (`anvil`); declined installs stay
  declined until the user deliberately runs `start`; daemon/MCP ensure remain
  idempotent.
- **Positive:** `start` can stay a rich activation/reconfigure surface without
  fighting quiet-repeat product goals.
- **Negative:** Public CLI contract change (bare no longer always exit 2 + help).
  Docs, tests (`bare_invocation.rs`), and script assumptions must move together.
- **Negative:** Slightly more complex root clap / dispatch path.
- **Risks:** Silent writes on first bare run; over-claiming MCP "on"; CI scripts
  that treated bare exit 2 as "show help".
- **Mitigations:** §2.3 first-run honesty; honesty pins for L0; fixture-pinned
  exit codes and non-interactive behaviour; optional one-release feature gate.

## References

- Related ADRs: [ADR-044](044-mcp-entry-activation-owned.md),
  [ADR-080](080-ungate-welcome-demo-surface.md),
  [ADR-082](082-daemon-lifecycle-user-startup.md),
  [ADR-092](092-mcp-optional-activation-spine.md),
  [ADR-094](094-worktree-registration-ux.md),
  [ADR-103](103-tty-default-activation-tui.md)
- Design: [`plans/specs/2026-08-01-bare-anvil-ensure.md`](../specs/2026-08-01-bare-anvil-ensure.md)
- APS modules: [bare-ensure (ONSW)](../modules/bare-ensure.aps.md),
  [JOURNEY-011](../modules/release-user-journeys.aps.md),
  ACTMO (daemon spine), activation orchestrator consent plan
- Supersedes for bare invocation only: CIB-177 exit-2-always contract (pointer
  intent retained in help)
- Historical brainstorm: bare as wow-start was floated in
  `plans/brainstorms/2026-05-02-wow-start-claude.md` and not adopted; this ADR
  chooses ensure, not wow-start, for bare
