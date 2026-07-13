---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between anvil versions.

## Upgrading to 0.9.0-beta

Upgrade from `0.8.x-beta` (or earlier) with the installer, package manager, or
`anvil update`. There is no config-file schema migration and no protection-claim
vocabulary change, but several **behavioural** contracts moved — read the
action-required list before scripting against the new cut.

```bash
# Upgrade via the installer (Homebrew-aware)
sh <(curl -fsSL https://install.eddacraft.ai)

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade --id eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's new in 0.9.0-beta

- **First win on your own code** via `anvil welcome` (consent-gated local fix
  preview), quiet repeat `anvil start`, shareable `anvil insights --share`
  scorecard, and consent-first install pickers (every candidate starts
  unticked).
- **Assistant graph context over MCP** — identity-only graph tools and
  `graph://` resources; source snippets require operator egress consent and a
  per-request opt-in. See the [MCP integration guide](../integrations/mcp.md).
- **Python project support** and **infrastructure-hygiene** gate surfaces
  (Dockerfiles, GitHub Actions, shell scripts, SQL migrations — default-on,
  flag-gated).
- **Warm-graph persistence default-on** and **daemon lifecycle** for interactive
  `anvil start` / `anvil watch` (Linux and macOS). Details below.

Full customer-facing notes live in the
[changelog](changelog.md#090-beta-2026-07-12-first-run-wins-and-the-assistant-graph).

### Action required

1. **Auth-required action commands now exit `3` (breaking for scripts).** When
   you are not logged in, gated _action_ commands — `anvil start`, `init`,
   `watch`, `gate`, `check`, `audit`, and siblings — exit `3` (authentication
   required) instead of `0`. The human message and the `--json` `authRequired`
   envelope shape are unchanged; only the exit code moved. Read-only
   `anvil status` and the `whoami` / `auth whoami` probes keep their prior
   contracts. If a script relied on exit `0` at the auth wall, gate on the
   explicit exit code or authenticate first.

2. **Interactive install pickers no longer pre-select.** Every workflow and MCP
   picker candidate starts unticked. Tick what you want (space), then apply.
   Enter with nothing ticked writes nothing. Non-interactive / CI auto-install
   is unchanged.

3. **Warm-graph persistence is default-on.** The save-time daemon persists and
   warm-restarts its resident graph by default (previously opt-in via
   `ANVIL_PERSIST_GRAPH`; graduated after the GBASE-010 gate, ADR-105).
   - **What it writes:** identity-only sealed snapshots — symbol names,
     import/path identity, edges, and content hashes for boundary checks. It
     **never** persists source text, snippets, or comments. Files are
     machine-local, `0600`, under a `0700` state dir.
   - **Where:** `graph-cache/base` under the state dir — one write-once artefact
     per repository per merge-base commit, with cheap per-worktree overlays.
   - **First run after upgrade:** pays a single cold rebuild per repository
     (snapshot format moved), then restarts warm from the shared base.
   - **Rollback:** set **`ANVIL_PERSIST_GRAPH=0` in the daemon's spawn
     environment** (not merely `~/.bashrc` / `~/.zshrc` — those do not reach a
     systemd-user- or IDE-launched daemon). For `systemd --user`, use
     `systemctl --user set-environment ANVIL_PERSIST_GRAPH=0` (or an
     `Environment=` line) and restart the daemon. Toggling off does not delete
     existing snapshots.
   - **Disk pressure:** `anvil graph-base gc` (or `--purge-all`). Deleting under
     `graph-cache/` by hand is also safe — the daemon cold-rebuilds.
   - **Failure posture:** all persistence failure is non-fatal.

4. **Policy exception grants count only when committed (ADR-100).** L4 gates
   (`pre-push`, `anvil l4-validate`, audit-chain rescans) read
   `anvil/exceptions/store.json` from the tree of the commit being validated —
   never from the working tree. Grant, commit the store, then push. The legacy
   `.anvil/exceptions.json` never influences gates; run
   `anvil exception migrate` and commit the tracked store.

5. **Allowlist confinement is exact-match only.** In `allowlist` mode the
   intercept daemon admits **exactly** the roots you add with
   `anvil workspace allow`, and nothing else (including no implicit primary
   check-in root). An empty allow-list admits no roots. If you use allowlist
   mode, add the roots you want served before upgrading. Default `open` mode is
   unchanged. See
   [workspace confinement](../operations/config.md#workspace-confinement).

### Daemon lifecycle (carried into 0.9.0-beta)

Daemon-backed save-time validation is the normal path on Linux and macOS:

- In an interactive terminal, `anvil start` auto-starts the per-user save-time
  daemon and reports the result on a `daemon:` line. An interactive
  `anvil watch` offers to start one when none is answering.
- A daemon already running is always reused; concurrent invocations never start
  a second one.
- Opt out with `--no-daemon` (or `ANVIL_NO_DAEMON=1` for `start`); `anvil watch`
  also honours `ANVIL_WATCH_DAEMON=0` as a hard opt-out that disables reuse too.
- Headless, `--json`, CI, hook, and piped runs never start, offer, or prompt and
  fall back deterministically to the scoped check; `--verify` stays read-only.
- `anvil intercept start --foreground` remains the operator/debug surface and
  the only launch mode on Windows for now.

### Optional: try the activation TUI

```bash
anvil start --tui
# or
ANVIL_ACTIVATION_TUI=1 anvil start
```

The TUI is still opt-in. Consent is end-to-end: nothing ticked means nothing
written. Scripts keep `--verify` / `--json` / `--no-tui`. See
[start output contracts](../guides/start-output-contracts.md).

## Versions covered

This page carries the per-version migration guides from `0.9.0-beta` back
through `0.1.2-beta`. Narrative release history for `0.8.x` and earlier also
lives in the [changelog](changelog.md).

## Upgrading to 0.7.2-beta

Drop-in patch upgrade from `0.7.1-beta` (or `0.7.0-beta`). There is no config
migration and no protection-claim vocabulary change.

```bash
# Upgrade via the installer (Homebrew-aware)
sh <(curl -fsSL https://anvil.dev/install)

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade --id eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's new in 0.7.2-beta

- **`anvil watch` now runs code-quality checks by default.** A bare
  `anvil watch` previously watched architecture and dependency edges only and
  ran no code-quality scan, while the dashboard still read "100% pass". It now
  runs `anvil check --all` on each save. Run `anvil watch --action none` to
  restore the architecture/dependency-only watch. `anvil start --watch` is
  unchanged (remains architecture-only).
- **Antipattern false positives in comments and strings are fixed.** The
  AP-003/GS-001 scanner now masks comments, string literals, and regex literals
  before applying code-construct rules, so prose or strings that merely mention
  `any` or contain a `!` are no longer reported.
- **`anvil version` warns when a stale `anvil` on `PATH` shadows the running
  binary** — surfacing the case where an updater reports a new version while an
  older shadowing install keeps running.
- **`anvil auth refresh` reports the 90-day refresh window** alongside the
  shorter access-token expiry, instead of printing only the ~7-day access-token
  expiry.
- **`anvil policy` command group (experimental).** New `list`, `explain`,
  `diff`, `validate`, and `test` subcommands, plus a preview `anvil policy eval`
  whose output shape may still change.

### Action required

None. This is a drop-in patch.

## Upgrading to 0.7.1-beta

Drop-in patch upgrade from `0.7.0-beta`. There is no config migration and no
protection-claim vocabulary change. The patch fixes activation honesty: if the
intercept daemon is running and attests the current worktree,
`anvil start --verify` and `anvil status --verify` can now promote from
`ready_restart_required` to `protecting` instead of asking for another editor
restart forever.

```bash
# Upgrade via the installer (Homebrew-aware)
sh <(curl -fsSL https://anvil.dev/install)

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade --id eddacraft.anvil

# Or via Scoop
scoop update anvil
```

After upgrade, verify activation from the repository root:

```bash
anvil start --verify
anvil status --verify
```

`protecting` means the daemon has attested the canonical worktree. If activation
still reports `ready_restart_required`, read the repair hint literally: it now
distinguishes editor restart from daemon unreachable, unenforced, stale, and
all-quarantined states.

### What's new in 0.7.1-beta

- **Activation diagnostic consumes daemon evidence** — fixes GH
  [#1831](https://github.com/eddacraft/anvil-001/issues/1831) by promoting
  handshake-verified MCP clients when the daemon reports live enforcement for
  the same worktree.
- **Windows MCP protection-claim parity** — `anvil_validate_write` now reaches
  the daemon through owner-only named pipes on Windows and can return the same
  `protection_claim` field as Unix.
- **Actionable `ready_restart_required` hints** — daemon-down, unenforced,
  stale-snapshot, and all-quarantined states point to daemon inspection or
  restart instead of always telling the user to restart Cursor or Claude Code.
- **L4 IO-outage distinction** — transient filesystem errors no longer surface
  as missing-engine hints.
- **Scoop / WinGet uninstall-root detection** — `anvil uninstall` recognises the
  Windows package-manager install roots and tightens its removal boundary.
- **`anvil-run` SIGTERM diagnostic note** — the runbook names the transient
  fence symptom after launcher termination and the recovery path.

### Known gaps carried from 0.7.0-beta

- Daemon-side `session.report_process` is still unimplemented; launcher sessions
  absorb the gap gracefully.
- `anvil intercept unblock --worktree` is still Unix-only. On Windows, stop the
  daemon and start it again if every surface is quarantined.
- MCP `query_protection_claim` still uses the older 2 second IPC timeout. The
  activation path itself uses the 500 ms budget.

## Upgrading to 0.7.0-beta

Drop-in upgrade from `0.6.x`. There is no required config migration — existing
`.anvilrc` files keep working untouched. The release theme is
**daemon-working**: hooks, the witness chain, baseline adoption, L4 policy, and
wrapped agent launch share a single typed `ProtectionClaim` rendered on
`anvil status --json`, `anvil doctor --json`, the `anvil_validate_write`
MCP-tool response (when the daemon is reachable; the Windows MCP shim still
reports `daemonStatus: not-wired` and omits `protection_claim`), and the
TypeScript driver-client. Most of the surface delta is additive; this section is
the public operator reference.

```bash
# Upgrade via the installer (Homebrew-aware)
sh <(curl -fsSL https://anvil.dev/install)

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade --id eddacraft.anvil

# Or via Scoop
scoop update anvil
```

After upgrade, confirm the new protection claim renders cleanly:

```bash
anvil status --json | jq '.claim.worktree_state'   # → "full"
```

A `"full"` worktree state on each surface that emits one (CLI, doctor,
daemon-backed MCP shim on Unix, TypeScript driver-client) means the
daemon-working contract is honoured. See the
[v0.7.0-beta release runbook §3](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
for the full cross-surface check.

### Action required

For most teams, none beyond the upgrade. Read the migration note before adopting
if any of the following apply:

- **Lefthook, husky, or pre-commit-framework users.** Anvil now registers as a
  managed entry instead of overwriting `.git/hooks/`. Lefthook and
  pre-commit-framework need a one-time manual `extends:` / `repos:` merge after
  install. See the
  [hook coexistence runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-hook-coexistence.md).
- **`.anvil.yaml` users with YAML anchors / aliases.** `anvil-config::parse` now
  rejects YAML aliases outright. Quote-escape the symbols inside string scalars
  or rewrite without aliases.
- **Multi-branch adoption (enterprise / monorepo).** If you adopt on multiple
  long-lived branches in parallel, each branch mints its own genesis anchor, and
  the first cross-branch merge fails with `OrphanMerge`. Roll out on the default
  branch first; see the multi-branch adoption guidance in these upgrade notes.
- **Teammates pushing without `anvil` on PATH.** The pre-push hook now applies
  the full L4-policy pipeline including witness-chain DAG verification.
  Unwitnessed commits are recovered with
  `anvil hook bootstrap --witness-recent`.
- **Cargo install users.** `anvil` and `anvil-run` ship as separate crates:

  ```bash
  cargo install --git https://github.com/eddacraft/anvil-001 \
    --tag v0.7.0-beta eddacraft-anvil --bin anvil
  cargo install --git https://github.com/eddacraft/anvil-001 \
    --tag v0.7.0-beta eddacraft-anvil-run --bin anvil-run
  ```

### What's new in 0.7.0-beta

- **`anvil-run`** — wrapped-launch ingress for `claude`, `codex`, `aider`, and
  similar agents. Stable exit codes (`64 / 69 / 73 / 75 / 78`) and shell
  integration via `shell/anvil-run.sh`. Full contract in the
  [`anvil-run` manpage](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-run.md).
- **`anvil l4-validate`** — dedicated L4-policy validator (formerly fused with
  `anvil hook pre-push`). Used by CI and GitHub Action consumers.
- **Protection-claim contract across surfaces.** `anvil status --json`,
  `anvil doctor --json`, the `anvil_validate_write` MCP-tool response, and the
  TypeScript driver-client all emit the same typed `ProtectionClaim` shape. The
  MCP response only carries `protection_claim` when the daemon is reachable — on
  Windows the daemon-validation client is gated `cfg(unix)`, so the envelope
  reports `daemonStatus: not-wired` and the field is omitted on that surface
  (status and doctor still render the claim on Windows). Pre-existing consumers
  continue to receive a backward-compatible response; the field is
  wire-additive.
- **Witness chain in-tree at `anvil/witness/`.** Hash-chained record of which
  protection layers fired on which commit, intended to be committed.
  `.gitattributes` is pre-positioned with `merge=union -text` so parallel
  branches never produce conflict markers. Full operator surface in the
  [witness-chain runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-witness-chain.md).
- **Hook coexistence with lefthook / husky / pre-commit-framework.** Detection
  precedence, install/uninstall round-trip guarantees, and per-framework
  behaviour in the
  [hook coexistence runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-hook-coexistence.md).
- **`.anvil.<ext>` configuration alternative.** `.anvil.yaml`, `.anvil.yml`,
  `.anvil.json`, and `.anvil.toml` discovered first; legacy `.anvilrc` falls
  back when none are present. `anvil migrate` rewrites at your convenience.
- **`anvil intercept unblock`.** Per-fence operator recovery —
  `--worktree <PATH>` for a single fence, `--all` for every fence, `--dry-run`
  to preview, `--acknowledge-cascade <worktree>` to clear a
  `degraded:fence-cascade` rate-limited fence (the `RateWindow` capacity is 4,
  so the **fifth** fire within a 60 s window engages cascade).
- **`anvil baseline --new-identity` / `anvil start --new-identity`.** Fork
  opt-out — mints a fresh `project_uuid` and records the previous one as
  `forked_from`.
- **`anvil baseline --refresh --accept-suspicious`.** Adversarial-refresh
  detection — explicit ack required when a refresh would drop ≥75% of findings.
- **`anvil audit-chain`.** L5 audit — re-walks a branch's commits and reports
  any without a witness line.
- **`anvil hook bootstrap --witness-recent`.** Retroactively witnesses
  unwitnessed commits in `@{u}..HEAD`.
- **`anvil insights`.** Local-only weekly summary derived from the witness
  chain. `witness_events_observed` is populated this release; other counters
  ship as schema-locked placeholders pending downstream metric wiring.
- **`anvil version --check`.** Advisory update check with install-method-aware
  upgrade hint; rate-limited to once per 24 hours.
- **Signed `anvil update`.** Every supported install path (Homebrew, curl
  installer, axoupdater fallback) verifies the published minisign signature
  before replacing the running binary.

### Carry-forward from `v0.6.0-beta`

The foreground-only daemon launch mode, macOS fence-first interrupt ladder, and
Windows CI scope (cross-compile matrix on `main`) are unchanged. The Windows MCP
correlation gap in this `0.7.0-beta` section was closed by `0.7.1-beta`. The
[v0.7.0-beta release runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
captures the live operational references without linking public docs into
archived runbooks.

### Downgrade

Downgrading to `v0.6.x` is supported on a clean repo (no committed
`anvil/witness/` lines from `v0.7.0-beta`-only writers). The witness chain shape
is forward-compatible — older binaries can read `v0.7.0-beta` lines but cannot
emit the merge-line / `rules_sha` / `cutoff_commit` extensions. The DAG verifier
accepts both shapes. For mixed-version teams, do **not** enable
`anvil audit-chain --rescan` during the mixed-version window — it would flag
`v0.6.x` lines lacking `rules_sha` as drift. See the migration note's
"Downgrade" section for the full procedure.

## Upgrading to 0.6.3-beta

Drop-in upgrade from `0.6.2-beta`. No breaking changes, no new APIs — this is a
hotfix for first-run and watch friction reported by beta users.

```bash
# Upgrade via Homebrew
brew upgrade eddacraft/tap/anvil

# Or via the installer (now detects an existing Homebrew install and steps aside)
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade --id eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.6.3-beta

- **Homebrew-aware curl installer** — `install.sh` no longer overwrites a
  Homebrew-managed `anvil` binary; it exits successfully with the
  `brew upgrade eddacraft/tap/anvil` hint.
- **Watch and audit ignore local agent/tool worktrees and caches by default** —
  `.claude`, `.opencode`, `.gemini`, `.serena`, `.worktrees`, plus the usual
  generated/cache/build directories.
- **Initial watch scan is baseline state, not new violations** — existing public
  exports, dependencies, and cross-layer imports no longer surface as save-time
  findings when `anvil watch` starts. Only later edits trigger findings.
- **`anvil watch` shows immediate startup feedback** — terse "starting" line
  before slow setup; plain-output fallback when stdin/stdout is not a terminal.
- **`anvil uninstall`** — project-scoped removal of Anvil state (`.anvil/`,
  `.anvilrc`, Anvil-managed git hooks). `--global` also removes user-level
  state, MCP entries, credentials, and the running daemon. The Anvil binary
  itself is never removed.

### Action Required

None — this is a drop-in upgrade. If you previously installed via Homebrew and
have been confused by the curl installer overwriting it, you can now run either
flow safely.

## Upgrading to 0.6.2-beta

Drop-in upgrade from `0.6.1-beta`. No breaking changes — this is a patch release
that polishes `anvil update` on Windows (no more file-lock crash, plus new
WinGet and Scoop install detection), gives `anvil check` clearer no-args
guidance, adds local trace correlation for daemon/CLI debugging
(`ANVIL_TRACE_SINK=file=<path>`), and closes a brute-force window in the OAuth
device-code confirmation flow.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade --id eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.6.2-beta

- **`anvil update` Windows polish** — no more
  `The process cannot access the file because it is being used by another process`
  crash. The in-process axoupdater path is disabled this release because
  `install-updater = false` keeps `aarch64-pc-windows-msvc` in the release
  matrix; `anvil update` now refuses cleanly on Windows and points to
  `winget upgrade --id eddacraft.anvil` or re-running the PowerShell installer.
  `--check` still works.
- **`anvil update` detects WinGet and Scoop** — installs via WinGet or Scoop now
  print the exact upgrade command (`winget upgrade --id eddacraft.anvil` or
  `scoop update anvil`), mirroring the existing Homebrew dispatch.
- **`anvil check` no-args message** — a bare `anvil check` now lists
  `--changed`, `--all`, and explicit paths, plus pointers to `anvil welcome` and
  `anvil status`.
- **Local trace correlation** — `ANVIL_TRACE_SINK=file=<path>` writes JSON-line
  traces to a user-private local file with correlation fields for daemon and CLI
  debugging. Disabled by default. Unix file permissions are enforced (`0600`;
  symlink/group/world-readable existing sinks are rejected). See
  `docs/observability/local-tracing.md`.
- **Security** — per-code brute-force counter on `/device/confirm` with an
  atomic upper bound, preventing exhaustion of valid device codes during the
  confirmation window.

## Upgrading to 0.6.1-beta

Drop-in upgrade from `0.6.0-beta`. No breaking changes — this is a patch release
that fixes the `anvil start` interactive flow end-to-end (the MCP picker no
longer reprints on every arrow press, the `Log in now?` prompt no longer hangs
when a previous TUI leaked raw mode, and the home-tilde path display is now
component-aware). Auth UX gains silent refresh via the stored refresh token plus
an explicit `anvil auth refresh` subcommand with cause-specific server error
messages. A HIGH-severity transitive CVE
(`@babel/plugin-transform-modules-systemjs`, CVE-2026-44728) is cleared via a
pnpm override.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.6.1-beta

- **`anvil start` interactive flow now usable end-to-end** — the MCP install
  picker stays on a single row (no more question reprinting on every arrow
  press), the `Log in now?` prompt accepts input even when a prior TUI left the
  terminal in raw mode, and home-tilde path display matches by path component
  (no more `~ice/...` for `/home/al` prefixes).
- **Silent licence refresh** — when a 7-day JWT lapses but the 90-day refresh
  token is still valid, `anvil` exchanges it inline before falling through to
  the `Log in now?` prompt. Eliminates a forced device-code re-login every week.
- **`anvil auth refresh` subcommand** — explicit refresh that exchanges the
  stored refresh token without re-running the device flow. `--json` supported;
  bypasses the licence-gate pre-check by design.
- **Cause-specific auth errors** — `/session/refresh` distinguishes expired /
  revoked / theft / inactive responses; the CLI surfaces an actionable message
  for each instead of a generic 401.
- **Vercel deploy hardening** — `domainImports` gated on prod stack with input
  validation; `delete-before-replace` env-var ordering for the
  `www.eddacraft.ai` cutover.
- **Security** — bumps `@babel/plugin-transform-modules-systemjs` to `>=7.29.4`
  via pnpm override (CVE-2026-44728, HIGH).

## Upgrading to 0.6.0-beta

Drop-in upgrade from `0.5.1-beta`. There are no breaking changes — the
substantive new behaviours in this release (the `anvil start` activation flow,
daemon-backed MCP pre-write validation, the Cursor / Claude Code MCP install
path, and the protection-loop tutorial) are opt-in via `anvil start`, so an
existing `anvil check` / `anvil watch` / `anvil gate` workflow keeps running
unchanged. Operators running the daemon should read the v0.6.0-beta operator
runbook for the five operational realities of the cut (foreground-only daemon,
cross-platform `intercept status` with the MCP correlation envelope still
Unix-only, fence persistence across restart with no `stop`/`unblock` CLI in v1,
macOS fence-first interrupt ladder, `main`-only Windows CI), and the v0.6.0-beta
security note for the four HIGH trust-boundary trade-offs the release council
surfaced.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.6.0-beta

- **`anvil start` activation entrypoint** — `install → cd repo → anvil start` is
  the canonical first minute. `--verify` runs a read-only protection probe;
  `--watch` opts into the save-time fallback when MCP cannot attach.
- **Activation protection states** — `protecting`, `ready_restart_required`,
  `watching`, `needs_action`, `unsupported`, and `error` are the single shared
  vocabulary across `anvil start`, `anvil status --verify`, `anvil doctor`, and
  the tutorial.
- **`anvil mcp install` for Cursor and Claude Code** — one-step MCP activation
  that writes `~/.cursor/mcp.json` or `~/.claude.json`. Windsurf, VS Code, and
  HTTP-transport flows remain on `anvil mcp-config`.
- **Daemon-backed `anvil_validate_write` MCP tool** — the MCP pre-write
  validation path now routes through the local daemon over owner-only IPC when
  reachable, with the embedded validation pipeline as a correctness-equivalent
  fallback. macOS peer-credential validation is now at parity with Linux.
- **Repo language profile** — activation, scan, and watch honour a
  per-repository language profile so coverage is honest: TypeScript is the
  supported tier in this release, SQL and Markdown are partial, Python and Rust
  are reported as unsupported instead of silently skipped. Secret detection
  still runs on all files.
- **Protection-loop tutorial** — the default tutorial path walks the protection
  loop end-to-end and ends with a real `anvil start --verify` invocation.
  Policy, Architecture, Drift, and CI tutorial paths remain.
- **`anvil version`** — install-method-aware version output detects Homebrew,
  Scoop, WinGet, the installer, or a dev build, and prints the recommended
  upgrade command. The JSON shape is pinned for agent and CI consumers.
- **Windows named-pipe daemon listener and CLI status client** — the daemon
  listener side ships on Windows with an owner-only DACL and rejected remote
  clients, and `anvil intercept status` drives the same wire shape over the
  named pipe via `connect_owner_only_pipe_client`. `--json` returns the same
  `DaemonStatusV1` on Unix and Windows. The remaining Windows gap is in the MCP
  correlation envelope only: `correlation.daemonStatus` returned by
  `anvil_validate_write` is always `not-wired` on Windows because the MCP daemon
  validation client is gated `cfg(unix)`. That narrower fix lands as part of
  `chore/windows-status`.
- **Operator-visible defaults you should know about.** The foreground daemon was
  the only supported launch mode in the 0.6.0-beta v1 path
  (`anvil intercept start --foreground`), and the `anvil intercept stop` /
  `anvil intercept unblock` front-end commands were not wired in that release.
  Fences persist across daemon restart by design. For current recovery guidance,
  prefer `anvil intercept unblock --worktree <path>` where supported,
  `anvil intercept stop` for a Unix background daemon, or Ctrl-C for a
  foreground daemon before removing `${XDG_DATA_HOME:-$HOME/.local/share}/anvil`
  as a full reset. On macOS, the interrupt ladder is fence-first this release
  because `current_process_start_time` lacks a macOS branch.

## Upgrading to 0.5.1-beta

CLI upgrade from `0.5.0-beta`. This release focuses on scanner signal quality,
TUI interaction fixes, incremental graph correctness, and release workflow
hardening. TypeScript package consumers should note that archived scanner-era
subpath exports were removed from `@eddacraft/anvil-core` and
`@eddacraft/anvil-runtime`.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.5.1-beta

- **Secret scanner false-positive reductions** — generic secret matching now
  requires a stronger right-hand-side shape, credit-card detection rejects UUID
  fragments, and entropy matching focuses on secret-shaped quoted values.
- **Antipattern suppression fixes** — `AP-*` checks now honour local
  `eslint-disable` directives and avoid reporting guarded `Map.get` after
  `has`/`set` flows as `GS-001`.
- **Audit path filtering** — audit scans skip broader environment-template files
  while still reporting real `.env` files regardless of directory.
- **TUI interaction polish** — audit, status, and watch surfaces support
  zooming; doctor acknowledges `f` to fix; tutorial path selection has more room
  for wrapped options.
- **Incremental graph correctness** — watch updates now avoid synthetic import
  ID collisions and preserve import-source ID `0`, preventing missed import
  edges in refreshed symbol graphs.
- **TypeScript package subpath cleanup** — archived scanner-era subpaths for
  antipattern, suppression, drift, gate, and export flows are no longer
  exported; use the Rust CLI surfaces instead.
- **Release safety** — the PR base guard workflow now detects release-sensitive
  PRs targeting the wrong branch when repository branch protection requires the
  check.

## Upgrading to 0.5.0-beta

Drop-in upgrade from `0.4.0-beta`. There are no breaking changes; every new
behaviour below is opt-in.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.5.0-beta

- **Git config-mode hooks (opt-in)** — install Anvil-owned hook commands through
  Git 2.54 native config with `anvil hooks install --config` and remove them
  with `anvil hooks uninstall --config`. File-mode hooks remain the default and
  Husky stays as the contributor bootstrap; both surfaces detect and warn about
  file/config coexistence and `core.hooksPath` overrides.
- **AI guardrail profile** — `anvil gate --profile ai` runs the AI-focused check
  set, treats missing or invalid governance configuration as blocking, and emits
  the canonical `anvil.diagnostic.v1` JSON envelope by default for agent and MCP
  consumers.
- **AI-001 reasoning rule** — a new info-severity rule that flags source
  comments justifying code with authority, social proof, or deflection rather
  than technical reasoning. Suppress per occurrence with
  `// @anvil-ignore AI-001` and a short reason.
- **`.env` and `.envrc` scanning (`SURFENV-001`)** — `.env`, `.env.*`, and
  `.envrc` files are parsed as key/value files; leaked secret values are
  reported with the variable name and source line. Suppress with
  `# @anvil-ignore SURFENV-001`.
- **`anvil mcp-config`** — generates, verifies, and writes Claude Code, Cursor,
  Windsurf, and VS Code MCP server configuration. Use `--write` to apply
  changes, `--verify` to diff against the on-disk config, and rely on the
  path-safety prompts before atomic writes overwrite an existing file. See
  [MCP Integration](../integrations/mcp.md) for the supported transports and
  per-client paths.
- **Scan performance cap** — first-run scans honour `ANVIL_SCAN_THREADS`
  (default `min(num_cpus, 4)`) so the parallel walk does not starve TUI or
  editor work; oversized lines are skipped before regex evaluation to eliminate
  the previous ReDoS risk.
- **Doctor outside git repos** — running `anvil doctor` outside a Git repository
  now produces a structured `git-repo` warning instead of failing the whole run.

### Operator-side: API migration runner

Anvil API deploys now ship with a first-party SQL migration runner with dry-run
support and drift detection. Operators running `anvil-api` should review the
migration runbook before the next deploy; CLI users do not need to take action.

## Upgrading to 0.4.0-beta

Drop-in upgrade from `0.3.3-beta` for most users. Three behavioural changes
require attention:

- **`anvil watch --exclude` now takes glob patterns, not bare directory names.**
  A previous `--exclude vendor` no longer excludes files under `vendor/`; use
  `--exclude 'vendor/**'` instead. The CLI prints a warning when a
  likely-bare-name pattern is detected.
- **`anvil doctor --json` envelope changed** from a bare array to
  `{ "checks": [...], "notifications": [...], "schema_version": "2.0.0" }`, and
  every check now carries a structured `remediation` object
  (`{ summary, command?, doc_url? }`). Consumers iterating the array must switch
  to `data.checks`; consumers that schema-validated the prior shape must accept
  the `remediation` field on every check and the new `schema_version` envelope
  field. Branch on `schema_version` to gate compatibility — pass / skipped
  checks emit `remediation: { summary: "" }`, fail / warn checks always populate
  `summary` and at least one of `command` or `doc_url`.
- **`anvil check`, `anvil gate`, `anvil audit` JSON outputs now include a
  `notifications[]` field** alongside their existing payloads. Consumers pinned
  to the prior shape will see an additional ignorable field; nothing is removed.
  The notification envelope shape is shared with `anvil doctor`.

### Operator-side: per-operator admin keys

If you're running the `anvil-api` backend and want to enable the new
per-operator admin key flow shipped in this release (replacing the single shared
admin key), set:

- `ADMIN_PER_OPERATOR_KEYS=1` — turns on per-operator key resolution
- `ADMIN_KEY_PEPPER=<random-32-byte-hex>` — pepper for the peppered-hash lookup;
  must be set before any per-operator keys can authenticate

When `ADMIN_PER_OPERATOR_KEYS=1` is set without a non-empty `ADMIN_KEY_PEPPER`,
the middleware falls back to the legacy shared-key auth and logs an error
server-side. CLI requests will not see the misconfiguration directly. Provision
both via your secret manager (Pulumi handles this for the EddaCraft-managed
deployment) before rolling operators onto per-operator keys.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.4.0-beta

- **`anvil watch --patterns / --exclude`** — user-supplied glob filter on the
  watch loop. Previously the flags were declared but never read; watch silently
  used a hardcoded scope.
- **Post-init auto-analysis** — `anvil init` now runs an inline first scan and
  surfaces a real signal (top warnings + counts) rather than pointing at
  `anvil doctor`.
- **Doctor structured remediation** — every `anvil doctor` check emits a
  concrete remediation field (link, command, or auto-fix prompt); no check
  terminates at "see README".
- **`anvil watch` startup banner** — prints active include / exclude scope so
  the active filter is visible at a glance.
- **Workspace hardening** — cargo-hakari workspace-hack, cargo-deny policy,
  third-party notices via cargo-about (RUSTNX).

## Upgrading to 0.3.3-beta

Drop-in upgrade from `0.3.2-beta`. No configuration migration is required.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.3.3-beta

- **Windows distribution** — WinGet landed and Scoop became part of the
  documented install/upgrade story.
- **Admin operations** — the separate `anvil-admin` operator CLI gained
  list/show/invite/audit/revoke and migration tooling.
- **Windows UX fixes** — onboarding, discovery, and key handling improved.

## Upgrading to 0.3.2-beta

Drop-in upgrade from `0.3.1-beta`. No configuration migration is required.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

## Upgrading to 0.3.1-beta

Drop-in upgrade from 0.3.0-beta. No configuration changes required.

```bash
# Upgrade via the installer (overwrites existing binary)
curl -fsSL https://install.eddacraft.ai | sh

# Or via Homebrew
brew upgrade eddacraft/tap/anvil

# Or via the built-in updater
anvil update
```

### What's New in 0.3.1-beta

- **Docs domain cutover** — `docs.eddacraft.ai` now served via a dedicated proxy
  with a Nordic terminal-themed landing page.
- **Welcome screen fixes** — first-user onboarding flows restored after
  regressions in 0.3.0-beta.
- **Auth error messages** — clearer error messages during login and device-code
  flows.
- **TUI version display** — shell footer now shows the correct version.

No breaking changes. All existing configuration, credentials, and workflows
continue to work without modification.

## Upgrading to 0.3.0-beta

`0.3.0-beta` was the release where anvil became a native Rust binary. Current
docs assume a fresh install on the Rust CLI rather than a staged migration from
the legacy Node.js package.

```bash
# Install the native binary
curl -fsSL https://install.eddacraft.ai | sh
```

If an older npm-installed `anvil` is still earlier on your `PATH`, remove
`@eddacraft/anvil-cli` and re-run `anvil --version` so you know the native
binary is the command being executed.

Your `.anvilrc` and `.anvil/` directory work without changes.

For full details, see [The Switch to Rust](./rust-rewrite.md).

### What's New

- **Native binary** — 5–10x faster scanning, 80% less memory in watch mode, no
  Node.js dependency.
- **Kernel engine** — foreground watch mode, incremental parsing, and real-time
  semantic graph updates in the native Rust runtime.
- **Ratatui TUI** — rebuilt interactive surfaces with the eddacraft Terminal
  Standard design system.
- **Welcome & onboarding** — first-run interactive experience; run
  `anvil welcome` anytime.
- **New commands** — `anvil new`, `anvil wizard`, `anvil audit`, `anvil drift`,
  `anvil validate`, `anvil gate-config`.
- **Structured exit codes** — `0` (pass), `1` (error), `2` (gate fail), `3`
  (auth required), `4` (config error).
- **Beta auth** — device-flow and OTP authentication with OS keychain storage.

### Breaking Changes

- **Installation method** — install anvil as a native binary via the installer,
  Homebrew, WinGet, or Scoop.
- **CI workflows** — replace `pnpm anvil` / `npx anvil` with direct `anvil`
  calls. Remove Node.js setup steps if anvil was the only reason they existed.
- **Docs access** — the `/anvil` documentation is now gated behind GitHub OAuth
  for beta users. Sign in with the GitHub account tied to your beta invite.
  Public eddacraft docs (APS, Kindling, edda-stack) remain open.

## Upgrading to 0.2.1-beta

Drop-in upgrade from any previous 0.2.x version. No configuration changes
required.

### What's New in 0.2.1

- **Project memory** — anvil now tracks patterns and decisions in your codebase
  via the Edda memory system and Ember proposal engine.
- **Security hardening** — input validation and subprocess execution
  improvements across the platform.
- **Dependency patches** — minimatch, axios, svgo, tar, and others.

No breaking changes. The new memory features are opt-in and do not affect
existing scanning behaviour.

## Upgrading to 0.1.2-beta

This was the first public beta. No breaking migrations from alpha beyond the
configuration key change below.

### Note for Early Alpha Testers

If you used an internal alpha build, the top-level configuration key changed
from `"checks"` to `"gates"`:

```json
// Old (alpha)
{
  "checks": {
    "architecture": { ... }
  }
}

// Current (0.1.x-beta)
{
  "gates": {
    "architecture": { ... }
  }
}
```

Run `anvil init --force` to regenerate your configuration, or rename the key
manually in `.anvilrc`.

## Future Versions

Upgrade guides are added here as new versions ship.

## Getting Help

If you encounter upgrade issues:

1. Check the [Troubleshooting guide](/anvil/operations/troubleshooting)
2. Search [existing issues](https://github.com/eddacraft/anvil/issues)
3. Open a new issue with:
   - Old version
   - New version
   - Error message
   - Steps to reproduce

---

**See also:** [Changelog](/anvil/releases/changelog),
[The Switch to Rust](/anvil/releases/rust-rewrite)
