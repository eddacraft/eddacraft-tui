# JOURNEY-005 — Linux candidate journey rehearsal

- **Candidate SHA:** `d6d3aa39c` (main tip after the full JOURNEY release-cut
  merge set: CIB-184 #3279, WOW-005 #3280, CIB-073 #3282, CIB-183 #3283,
  ACTTUI-012 #3284, CIB-190 #3286)
- **Date:** 2026-07-12
- **Platform:** Linux x86_64 (kernel 6.17), release binary
  `cargo build --release -p eddacraft-anvil` at the candidate SHA
  (`anvil 0.8.1-beta`)
- **Environment:** hermetic — fresh temp `HOME`, fresh temp git repository
  with two TypeScript files carrying real AP-003/AP-001 findings, `ANVIL_DEV=1`
  (documented licence-gate dev override), non-TTY (piped) sessions. The
  interactive TTY paths are pinned separately by the PTY/e2e suites cited under
  each journey.

## Journeys and outcomes

### J1 — fresh `anvil welcome` (non-TTY)

Exit 0. Deterministic plain welcome with command list and one next step
(`run 'anvil start' for daily save-time protection`). No claims of activation
or protection. Interactive discovery/first-win path covered by
`cargo test -p eddacraft-anvil-tui tutorial` (346) + `welcome` (40) on the
same SHA.

### J2 — first `anvil start` (rich)

Exit 0 in **0.34 s**. Full first-run recipe: state
`ready_restart_required`, DLIFE-006 daemon-unreachable headline, honest
`daemon: not auto-started (non-interactive …)` posture, baseline recorded
(3 findings, "recorded for reference — not yet used to filter"), language
coverage table, fresh MCP install into the hermetic `$HOME/.claude.json`,
smoke-test recipe, exactly one `next:` action.

### J3 — repeat `anvil start` (collapsed, CIB-183)

Exit 0 in **0.026 s**. Output collapsed to exactly: state + headline,
daemon posture, save-time driver posture, one next step (6 lines).
**Byte-identical across consecutive runs.** No first-run recipe, install
block, or language table reprinted. CIB-190 value line honestly omitted
(no recorded witness/sidecar evidence in the fresh HOME — never "0 events").

### J4 — machine contracts

`anvil start --verify` and `anvil start --json`: byte-identical across
consecutive runs; JSON parses, `state` field consistent with the plain
surface.

### J5 — no-MCP opt-out

`ANVIL_NO_MCP=1 anvil start`: exit 0, honest state, no MCP install
activity.

### J6 — daemon lifecycle

- `anvil intercept start` without `--foreground` refuses with a correct
  pointer to the DLIFE ensure primitive (operator surface guarded).
- With a live daemon (`intercept start --foreground`, backgrounded):
  repeat `anvil start` graduates to **`protecting`**, `daemon: reusing the
  per-user save-time daemon`, `save-time driver: attached`, one closing
  next step — in **0.027 s**.

### J7 — durable worktree registration + daemon restart

- `anvil workspace register` reports "already registered" (the earlier
  `start` registered durably).
- `anvil intercept stop` → `intercept status` correctly errors with the
  recovery command.
- `anvil start` with the daemon down: honest degraded collapse
  (`ready_restart_required`, recovery next step names the headless
  `intercept start --foreground` path).
- Daemon restart → `intercept status` shows **1 active session
  immediately** (durable registration reloaded on start, ADR-094), and the
  next `anvil start` returns to the collapsed `protecting` output.

### J8 — repair path

Corrupted `.anvilrc` → `anvil start` exits 0 with the **rich** repair
output (`state: error`, `config: invalid`, full diagnostic — never the
quiet collapse). Restoring the config returns the collapsed `protecting`
output on the next run.

## JOURNEY-006 outcome metrics (Linux leg)

| Metric | Value |
| --- | --- |
| First-run `anvil start` wall clock | 0.34 s |
| Healthy repeat wall clock | 0.026–0.027 s |
| Healthy repeat output size | 6 lines, one next action |
| One-next-action compliance | every terminal state observed carried exactly one recovery/next action |
| Machine contracts | `--verify` / `--json` byte-stable across runs |
| Share-receipt privacy | redaction fixtures green on the SHA (`cargo test -p eddacraft-anvil -- insights`, 47; marker-seeded card/plain/v2 assertions) |
| Value-receipt honesty | line omitted with no evidence; never zero-filled |

## Cross-platform evidence (dispatched on the candidate SHA)

- `ci-nightly.yml` (cross-platform legs):
  <https://github.com/eddacraft/anvil-001/actions/runs/29161637384>
- `rust.yml` (cross-compile matrix incl. Windows):
  <https://github.com/eddacraft/anvil-001/actions/runs/29161638249>

## Not covered here (parked for the operator)

Interactive macOS and Windows journey rehearsal (fresh install, TTY
activation + consent, daemon reboot survival) cannot run from this Linux
host — parked in `plans/execution/escalation.queue.md` (ESC-001/ESC-002)
with the CI matrix runs above as the automated stand-in.
