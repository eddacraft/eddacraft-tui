# User-journey pass: `anvil welcome` and `anvil start` (2026-07-04)

Personas exercised:

- **New user** — never used anvil; follows the printed copy literally.
- **Experienced developer / returning user** — no time to learn, wants
  `anvil start` to just work; re-runs it on subsequent days.

Platforms considered: Linux (live runs), Windows and macOS (code-path review).

## Method

- Live runs of the shipped `anvil 0.8.2-beta` (Homebrew) in a sandboxed `HOME`
  plus a fresh two-file JavaScript git repo: plain, piped, and pseudo-TTY
  interactive runs; unauthenticated, `ANVIL_DEV=1` bypass, `--verify`,
  `--watch`, and re-run passes.
- Source review of current `main` (`8112fa5a9`) across
  `crates/anvil-cli/src/commands/{welcome,start}.rs`, `crates/anvil-cli/src/activation/`,
  `crates/anvil-cli/src/main.rs` (auth gate), `crates/anvil-intercept/src/ensure.rs`,
  `crates/anvil-tui/src/surfaces/`, and `crates/anvil-cli/src/commands/hooks.rs`.
- Each finding states whether it reproduces on `main` or is release-lag only.

Filed as CIB-162..179 in
[`plans/modules/continuous-improvement-backlog.aps.md`](../modules/continuous-improvement-backlog.aps.md).

## What works well (baseline to preserve)

- `anvil start --verify` — unauthenticated, read-only, byte-stable, honest.
- Honesty contracts hold across the daemon lifecycle line, watch-skip reasons,
  `(not cached)` agent-detection annotation, and baseline copy.
- Platform fundamentals: Windows daemon spawn (CIB-072), `%USERPROFILE%`
  resolution (`util.rs:90-100`), `KeyEventKind::Press` filtering on all key
  reads, panic-safe terminal teardown, platform-branched URL open and tutorial
  `mkdir`.
- The ungated `anvil welcome` demo posture (ADR-080), project-local first-run
  marker, and `ANVIL_SKIP_WELCOME` fleet bypass.
- Main (unreleased at time of writing) already fixes the worst new-user cliff:
  signed-out welcome copy bridges to `anvil auth login` + free `--verify`
  probe (`welcome.rs:1380`), and interactive `anvil start` offers an inline
  "Log in now? [Y/n]" prompt (`main.rs:924-972`). Shipped 0.8.2-beta still
  dead-ends: welcome says "Next: run `anvil start`", start replies
  "Authentication required." and exits 0.

## Findings → CIB mapping

| #   | CIB     | Severity | Finding                                                                                 |
| --- | ------- | -------- | --------------------------------------------------------------------------------------- |
| 1   | CIB-162 | P1       | Raw JSON tracing WARN lines leak into every human `anvil start`                          |
| 2   | CIB-163 | P1       | `anvil start` prints init's "Next: run `anvil start`" (circular)                         |
| 3   | CIB-164 | P1       | `verify:` block over-claims layers (hooks gated on `.git` only; L0 "active" at restart)  |
| 4   | CIB-165 | P1       | GH Actions workflow picker defaults both workflows pre-selected                          |
| 5   | CIB-166 | P2       | Three competing next-step lines in one `anvil start` output                              |
| 6   | CIB-167 | P2       | `ready_restart_required` sticky + jargon tier labels; missing `meaning:` on other states |
| 7   | CIB-168 | P2       | No `anvil intercept stop` for the auto-started daemon                                    |
| 8   | CIB-169 | P2       | `anvil start` exits 0 on auth-required, breaking `&&` chaining                           |
| 9   | CIB-170 | P2       | Showcase findings on clean repos look real (`[Example]` prefix only)                     |
| 10  | CIB-171 | P2       | Welcome TUI: q-vs-Esc scope trap; Esc-on-discovery advances; `.anvilrc` hardcoded        |
| 11  | CIB-172 | P3       | Smoke recipe has no Windows variant (`rm` fails in cmd.exe)                              |
| 12  | CIB-173 | P3       | Windows editor detection tries only `.exe`, not PATHEXT                                  |
| 13  | CIB-174 | P3       | Daemon timeout copy says "within 10s"; real ceiling ~12s                                 |
| 14  | CIB-175 | P3       | Watcher failure guidance is Linux-only; raw `notify error` elsewhere                     |
| 15  | CIB-176 | P3       | Git hooks assume a bundled `sh`; no detection/fallback                                   |
| 16  | CIB-177 | P3       | Bare `anvil` dumps 40+ commands; no first-run pointer                                    |
| 17  | CIB-178 | P3       | Language profile counts anvil's own generated files as unclassified                      |
| 18  | CIB-179 | P3       | Compact-mode TUI silently drops copy in small terminals                                  |

Cross-cutting (not a single CIB item): re-run verbosity — a fully-activated
re-run still prints ~25-30 lines including the full smoke recipe every time
(`start.rs:403-414` skips only on hard `Error`). CIB-164/166 carry the pieces;
a quiet "already protecting" path is the shared outcome to aim at.

## Evidence detail

### 1. JSON WARN leak (CIB-162)

Every `anvil start` where the daemon does not attest the worktree prints, for
example:

```json
{"timestamp":"2026-07-03T16:17:18.078109Z","level":"WARN","fields":{"message":"activation: daemon attestation skipped","reason":"daemon_unreachable","worktree_claim_state":""},"target":"anvil::activation::daemon_evidence"}
```

twice per re-run, without `--verbose`. Present on `main` by design:
`daemon_evidence.rs:222-244` emits at `warn` (post-ship hardening, council
2026-05-22, so failures are not invisible), the CLI default filter is `warn`
(`crates/anvil-observability/src/lib.rs:75`), and the subscriber is JSON-format
(`lib.rs:145`). The visibility decision is right; the rendering is wrong for a
human first-run surface — the same facts already exist as human copy in the
diagnostic (`meaning:`/`next:` lines).

### 2. Circular next step (CIB-163)

When config is absent the orchestrator runs init inline
(`orchestrator/mod.rs:170-173`); init's success block ends with
"Next: run `anvil start` to activate protection." (`init.rs:440-451`) — inside
`anvil start` itself. Observed live.

### 3. `verify:` over-claims (CIB-164)

- "L3/L4 commit + push hooks" is gated only on `.git` existing
  (`start.rs:412`); `install_activation_hooks_silent` discards per-hook results
  (`hooks.rs:863-864`). On shipped 0.8.2-beta the claim printed while
  `.git/hooks/` was empty (hook install is post-0.8.2).
- "L0 mcp pre-write" is listed under "active layers" while `state:` says
  `ready_restart_required` (`start.rs:868-869` uses
  `mcp_pre_write_wired_or_live`). Wired ≠ active.
- On an all-languages-unsupported repo the `.ts` smoke recipe and
  "Next: run `anvil watch`" still print, contradicting the `unsupported`
  verdict (`start.rs:403-414`, `start.rs:851-859`, `render.rs:832-839`).

### 4. Workflow picker consent (CIB-165)

Interactive activation shows "Install or enable GitHub Actions workflows?"
with **both** options pre-ticked (`orchestrator/mod.rs:497-520`); Enter-through
writes `.github/workflows/anvil.yml` + `anvil-audit.yml`. Observed live. This
is the most repo-visible write `anvil start` performs (team-wide, PR-triggering)
and the easiest to accept accidentally. Prompt correctly does not reappear once
files exist.

### 5. Competing next steps (CIB-166)

One live first-run printed three instructions: init's "Next: run
`anvil start`…", the diagnostic's "next: start the intercept daemon with
`anvil intercept start --foreground`…", and the closing "Next: run
`anvil watch`…". UJ-001's one-next-step-per-ending intent is defeated by three
surfaces each owning a "next" line (`init.rs:440-451`, `render.rs:757-761`,
`start.rs:851-859`).

### 6. State comprehension (CIB-167)

Terminal-first users park permanently on `ready_restart_required`; the MCP
tier prints `restart_handshake_verified` — a success-sounding token — directly
under a restart-required headline (`diagnostic.rs:87-96`). Only
`ready_restart_required` has a `meaning:` line; `needs_action` /
`unsupported` / `watching` never do (`render.rs:576-605`).

### 7. No daemon off switch (CIB-168)

`anvil start` auto-spawns the daemon (ADR-082 "activation is consent"), but
`anvil intercept` offers only `start` / `status` / `unblock` (verified live).
Only prevention (`--no-daemon`) or `anvil uninstall --global` exists.

### 8. Exit-code contract (CIB-169)

Auth-required is remapped to exit 0 for `start` (`main.rs:523-524`, issue
#1822), while the same command exits non-zero on MCP install failure precisely
so `anvil start && next-step` cannot silently advance (`start.rs:436-449`).
The two contracts contradict: `anvil start && …` advances past a completely
unactivated repo.

### 9. Showcase findings look real (CIB-170)

Clean-repo discovery substitutes curated fake findings distinguished only by an
inline `[Example]` title prefix (`showcase.rs:18-64`); `discovery_render.rs`
has no showcase special-casing — same panel title, badges, and plausible paths
(`src/services/auth.rs:42` "Hard-coded API key detected").

### 10. Welcome TUI navigation (CIB-171)

- From any hub sub-surface `q` exits the whole program while `Esc` returns to
  the menu, yet the footer advertises "esc/q quit" as equivalent
  (`welcome.rs:1155-1324`).
- `Esc` on discovery results advances into the tutorial instead of backing out
  (`welcome.rs:407-413`).
- The init-complete landing hardcodes `Config: .anvilrc` even when the wizard
  wrote `.anvil.yaml`/`.json`/`.toml` (`welcome.rs:328`,
  `init_complete.rs:41`).

### 11-18. Platform and polish

- **CIB-172:** `RECIPE_LINES` (`start.rs:840-844`) step 3 is
  `rm .anvil-smoke-test.ts` — fails in cmd.exe; no `cfg!(windows)` branch,
  unlike the tutorial's `mkdir` handling (`paths.rs:113-127`).
- **CIB-173:** editor detection tries only `.exe` on Windows
  (`detect_agents.rs:148-199`); `.cmd`/`.bat` shims (common for editor CLIs)
  are missed. Already a documented follow-up in-code.
- **CIB-174:** ensure-failure copy says "within 10s" (`ensure.rs:322`) but the
  bind wait can overrun to ~12s (`ensure.rs:346`, `start.rs:283`).
- **CIB-175:** inotify preflight is `cfg(target_os = "linux")`
  (`capacity.rs:68-84`); macOS/Windows hard failures surface as raw
  `starting kernel watcher: notify error: …` with no actionable hint.
- **CIB-176:** hooks are `#!/bin/sh` with no detection of a git that lacks a
  bundled `sh` (`hooks.rs:72-92`); in-script `command -v anvil` degrade is
  good, but hook execution itself silently no-ops on exotic setups.
- **CIB-177:** bare `anvil` fails clap parse and dumps the full 40+-command
  help (`main.rs:163-169`); `welcome`/`start` are buried mid-list with no
  first-run pointer.
- **CIB-178:** the language profile counted anvil's own artefacts — live runs
  crept "(1 unclassified file)" → 4 → 6 as anvil wrote `.anvilrc`, `anvil/`,
  and workflow files.
- **CIB-179:** welcome surfaces silently drop descriptions/taglines in compact
  mode (`welcome_render.rs:39-49`); the 80x24 hard gate guards only
  `anvil watch --tui` (`compat.rs`, `app.rs:44-49`).

## Suggested fix order

1. CIB-162 (JSON leak) — cheapest, biggest first-impression win.
2. CIB-163 (circular next step).
3. CIB-164 (verify honesty) — protects the trust story.
4. CIB-166 (single next-step arbiter) — pairs naturally with 163/164.
5. CIB-165 (workflow consent default) — owner decision, then mechanical.

The remainder are independent and parallel-safe except CIB-166/163/164 (shared
`start.rs` rendering) and CIB-167 (shared `render.rs` copy), which should land
sequentially.
