---
id: changelog
title: Changelog
description: Release history for anvil.
sidebar_position: 1
---

# Changelog

All notable changes to anvil are documented here.

## [Unreleased]

### Added

- **`anvil --json watch` is now a stable NDJSON consumer surface** — the watch
  event stream pins to `anvil.watch.event.v1`. Every stdout line carries
  `schema_version`, `seq`, `timestamp`, `event_type`, and a typed `payload`.
  stdout is reserved for event records; warnings, banners, and child action
  stderr route to stderr. See
  [Watch JSON Output](../integrations/watch-output.md) for the consumer guide
  and
  [`docs/specs/watch-output-contract.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/specs/watch-output-contract.md)
  for the normative spec.

### Changed

- **Watch JSON payloads are typed, not debug strings** — `anvil --json watch`
  previously emitted lines whose `detail` field was a Rust debug-formatted
  string of the kernel event payload. Consumers reading that string MUST migrate
  to the structured `payload` object. The pre-WOUT shape was not guaranteed and
  is no longer produced.

## [0.7.1-beta] — 2026-05-22 — Activation Diagnostic Honesty

`v0.7.1-beta` is the first Boring Week patch on the `v0.7.0-beta` daemon-working
slate. It closes GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831):
early Windows + Scoop + PowerShell users could stay stuck at
`ready_restart_required` after `anvil start` even when the daemon was already
running and enforcing the worktree.

### Fixed

- **`anvil start --verify` reaches `protecting` from live daemon evidence.** The
  activation diagnostic now consumes the daemon's `ProtectionClaim` snapshot and
  promotes handshake-verified MCP clients to live validation when the daemon
  attests the canonical worktree.
- **Windows MCP responses now carry `protection_claim`.** The
  `anvil_validate_write` path now has Windows named-pipe parity with Unix, so
  Windows users no longer see the MCP daemon path reported as permanently
  `not-wired` when the daemon is healthy.
- **`ready_restart_required` repair hints distinguish daemon-state failures.**
  If activation stalls because the daemon is unreachable, unenforced for the
  worktree, stale, or all-quarantined, the hint now points at daemon inspection
  or restart instead of always saying to restart the editor.
- **L4 engine IO outages no longer look like missing engines.** A new
  `EngineUnavailableReason::IoError` separates retryable filesystem hiccups from
  permanently absent engines.
- **`anvil uninstall` detects Scoop and WinGet install roots.** Cleanup now
  recognises those Windows package-manager paths and keeps removal bounded to
  the install root.

### Added

- **Activation tracing surfaces actionable failures at `warn`.** Operators
  asking why activation did not promote now see daemon unreachable, unenforced,
  stale, or all-quarantined states without enabling debug logs.
- **`DaemonStatusV1.generated_at_unix`.** The daemon status wire shape includes
  a wall-clock snapshot anchor used as a second freshness check; older daemons
  deserialize as `0` and fall back to per-session freshness.
- **`anvil-run` SIGTERM diagnostics.** The manpage now names the transient fence
  that can appear after the launcher is killed and the recovery path: run the
  launcher again so session registration clears the fence.

### Changed

- **Activation freshness rejects far-future daemon timestamps.** A 90 second
  future-skew cap prevents broken clocks or replayed snapshots from permanently
  passing freshness checks.
- **Unix daemon-status reads enforce one wall-clock deadline.** A slow-drip
  daemon response can no longer stretch the activation IPC budget by resetting
  the timeout for every byte.

### Known gaps

- **Daemon-side `session.report_process` is still not implemented.** The
  launcher absorbs this gracefully; the daemon IPC handler remains tracked
  separately.
- **`anvil intercept restart` and `anvil intercept recover` do not exist yet.**
  Recovery still uses `anvil intercept start --foreground` and, where supported,
  `anvil intercept unblock --worktree <PATH>`.
- **`anvil intercept unblock --worktree` is still Unix-only.** Windows users
  with every surface quarantined should stop the daemon and start it again.
- **MCP `query_protection_claim` still uses the older 2 second IPC timeout.**
  The activation surface enforces the intended 500 ms budget; MCP timeout parity
  is a follow-up.

### Upgrade

- Homebrew: `brew upgrade eddacraft/tap/anvil`.
- Built-in updater: `anvil update`.
- WinGet: `winget upgrade --id eddacraft.anvil`.
- Scoop: `scoop update anvil`.

## [0.7.0-beta] — 2026-05-20 — Daemon-Working: End-to-End Verifiable Protection

The release theme is **daemon-working**: the protection claim is now verifiable
from code state alone. Hooks, the witness chain, baseline adoption, L4 policy,
and wrapped agent launch share a single typed `ProtectionClaim` shape rendered
on `anvil status --json`, `anvil doctor --json`, the `anvil_validate_write`
MCP-tool response (when the daemon is reachable; daemon-backed MCP is Unix-only
this release — the Windows MCP shim still reports `daemonStatus: not-wired` and
omits `protection_claim`), and the TypeScript driver-client. Most of the surface
delta is additive; the
[v0.6.x → v0.7.0-beta migration note](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md)
calls out the few places where operator action or expectation needs to shift.

### Added

- **End-to-end daemon-backed protection.** Every commit is witnessed, every save
  passes the same protection pipeline, and every agent-driven write is
  attributable to a registered session. `anvil doctor`, `anvil status`, the MCP
  server, and the TypeScript driver-client all emit the same typed
  protection-claim shape so editors, CI, and agents read identical state. See
  the
  [v0.7.0-beta release runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md).
- **Wrapped agent launch via `anvil-run`.** A new
  `anvil-run --tool <name> -- <command...>` launcher wraps AI-agent processes
  (Claude Code, Codex, Aider, and similar) so the daemon can attribute work,
  enforce fences, and clean up stale sessions. Daemon connectivity preflight,
  session registration with daemon-minted agent tags, process-group ownership,
  clean exit cleanup, zsh and bash shell-integration functions, a side-channel
  registration path via the pre-commit hook (`anvil-run hook register`) for
  sessions that cannot be launched through the wrapper, blocked-launch UX with
  actionable error output, and periodic heartbeats so the daemon notices when a
  launcher crashes. See the
  [`anvil-run` manpage](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-run.md).
- **`anvil doctor` typed protection-claim section.** `anvil doctor` now prints
  the worktree state and per-surface entries, with `--json` emitting the same
  `ProtectionClaim` shape as `anvil status --json`.
- **MCP server `anvil_validate_write` response carries `protection_claim`.** The
  field is optional; omitted when the daemon is unreachable (including on
  Windows, where the MCP daemon-validation client is still gated `cfg(unix)` and
  reports `daemonStatus: not-wired`). Pre-existing drivers round-trip the
  response unchanged.
- **`@anvil/driver-client` ships a `ProtectionClaim` parser.** Mirrors the Rust
  types so editors and agents read protection state in a typed shape; the MCP
  response adapter surfaces the claim when the daemon supplied one. Responses
  without the field parse cleanly for backward compatibility.
- **`anvil l4-validate` CLI command.** A dedicated subcommand for running L4
  verification over a commit range, replacing the previous `anvil hook pre-push`
  reuse for CI and GitHub Action consumers.
- **`anvil intercept unblock --acknowledge-cascade`.** When five fences fire on
  the same worktree within sixty seconds, Anvil engages a
  `degraded:fence-cascade` mode and refuses new sessions until an operator
  acknowledges. Use the new flag to clear; `anvil status` surfaces `cascaded` /
  `cascade_since` and the engaged state survives daemon restart.
- **`anvil intercept unblock --worktree <PATH>` / `--all`.** Per-fence operator
  recovery on the CLI. Pass `--worktree` to clear one fenced worktree (or
  `--all` to clear every fence); both are idempotent — re-running on an unfenced
  worktree exits zero with an informational note. `--dry-run` previews what
  would clear without modifying state. The previous "stop the daemon and delete
  the data directory" recovery is still available for corrupted on-disk state,
  but is no longer required for normal fence clearing.
- **`anvil edda list` ported to Rust.** The Edda memory-listing CLI is now part
  of the Rust `anvil` binary with identical behaviour to the legacy Node.js
  command — `--type`, `--status`, `--confidence`, `--since`, `--limit`, and the
  same `storage_found` / `storage_path` / `total` / `has_more` JSON envelope.
  Sort order, table headers, and exit codes match the previous surface.
- **`anvil insights` weekly summary.** New CLI surface and `anvil.insights.v1`
  JSON schema for editor and CI consumers, derived from the witness chain with
  no separate event store. This release populates `witness_events_observed`;
  `total_saves_observed`, `findings_raised`, `suppressions_applied`,
  `suppressions_resolved`, `baseline_edges_added`, and
  `daemon_uptime_percentage` ship as schema-locked placeholders (`0`) pending
  downstream metric wiring tracked in `INSIGHTS` follow-ups.
- **`anvil version --check` and security-advisory surface.**
  `anvil version --check` reports newer releases and security advisories against
  the running version. The watch TUI and `anvil status` show a one-line "update
  available" hint, rate-limited to once per 24 hours.
- **`anvil start --new-identity` and `anvil baseline --new-identity`.** Mints a
  fresh `project_uuid` and records the previous one as `forked_from`, giving
  forks an explicit opt-out from inheriting their parent repo's identity.
- **`anvil baseline --refresh --accept-suspicious`.** Adversarial-refresh
  detection — explicit acknowledgement required when a refresh would drop ≥75%
  of findings.
- **`anvil start --format json|toml`.** Choose `.anvil.json` or `.anvil.toml` at
  adoption time. The default remains yaml, and all three formats round-trip
  through the same canonical representation.
- **`anvil migrate`.** Optional one-command rewrite of a legacy `.anvilrc` into
  the `v0.7.0-beta`-native `.anvil.<ext>` shape. Opt-in; there is no deprecation
  timer in this release.
- **Witness chain (`anvil/witness/`) is load-bearing in-tree content.** An
  in-tree, hash-chained witness file at `anvil/witness/active.ndjson` records
  which protection layers fired on which commit. It travels with the repo via
  `git worktree add`, `git clone`, and `git push`. The pre-push hook calls
  `verify_chain_dag` across all witness segments; the L5 audit
  (`anvil audit-chain`) re-walks history to catch commits that bypassed
  pre-commit / pre-push. `.gitattributes` is pre-positioned by `anvil start`
  with `merge=union -text` so parallel branches never produce conflict markers.
  See the
  [witness-chain runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-witness-chain.md).
- **Hook coexistence with lefthook, husky, and pre-commit-framework.** Anvil
  hooks now install alongside the three dominant 2026 hook managers without
  conflict — registering as managed entries in the host manager's config rather
  than overwriting `.git/hooks/`. Uninstall removes only Anvil's own entries.
  Lefthook and pre-commit-framework require a one-time manual `extends:` or
  `repos:` merge after install; Husky and Plain are fully automatic. See the
  [hook coexistence runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-hook-coexistence.md).
- **AI tool auto-detect.** `anvil start` auto-detects Claude Code, Cursor,
  Aider, Windsurf, and Codex installations without configuration, reports a
  short summary, and writes the inventory to
  `.anvil/cache/detected-agents.json`. `anvil-run` consumes that cache to
  cross-reference `--tool` selections; a missing or stale cache is advisory, not
  an error.
- **Editor compatibility matrix and CI gate.** A documented compatibility matrix
  at `docs/policies/editor-coexistence.md` plus a headless harness and CI gate
  cover `rust-analyzer`, `tsserver`, `pyright`, `ruff`, `prettier`, and `eslint`
  against Rust, TypeScript, and Python fixtures.
- **Measured resource budget.** Anvil now publishes a documented resource
  ceiling (CPU steady-state and peak RSS) measured on a reference repository,
  with a CI workflow that fails the build on regression.
- **Release cadence and EOL policy.**
  [`docs/policies/release-cadence.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/policies/release-cadence.md)
  documents the hotfix iteration cadence, patch/minor/major scope semantics, the
  "sit on a release" minimum window, and the support window for `-beta`
  releases. Cross-linked from README and CONTRIBUTING.

### Changed

- **`anvil status --json` ProtectionClaim shape.** Output is now a typed
  `ProtectionClaim` built from the live daemon snapshot, with per-surface
  entries drawn from a fixed set of state values. When the daemon is
  unreachable, output falls back to a worktree state derived from local data
  with an empty `surfaces` array rather than over-claiming coverage.
- **`anvil baseline` writes project identity.** Baseline now mints
  `anvil/project-id` on first run (preserved on re-run) and pins `cutoff_commit`
  into the canonical policy file in the same flow, so adopting Anvil into an
  existing repo no longer fails on a missing project identity.
- **Config filename: `.anvilrc` → `.anvil.<ext>`.** Anvil discovers
  `.anvil.yaml`, `.anvil.yml`, `.anvil.json`, and `.anvil.toml` first, falling
  back to legacy `.anvilrc` only when none are present. Run `anvil migrate` to
  convert an existing `.anvilrc` to the new filename.
- **Signed `anvil update`.** `anvil update` now verifies downloads against a
  published minisign signature on every supported install path — Homebrew,
  curl-installer sidecar, and the axoupdater library fallback — before replacing
  the running binary. Signature mismatches are loud and actionable.
- **Homebrew formula automation.** Releases now publish the matching Homebrew
  formula automatically, so `brew upgrade eddacraft/tap/anvil` picks up new
  versions without a manual tap refresh.
- **MCP `anvil_validate_write` accepts patch-only payloads.** The pre-write
  validator now accepts a unified-diff `patch` instead of the full proposed
  `content` for change-shaped edits. Token cost scales with the size of the
  change rather than the file. The `content` mode remains supported; clients
  pick whichever fits their workflow.
- **MCP `anvil_validate_write` returns a recoverable workspace-root signal.**
  When the validator refuses on an untrusted workspace root it now returns an
  `expectedWorkspaceRoot` field on the rejection so callers can self-correct and
  retry without an operator round-trip. Pre-existing clients that ignore the
  field continue to receive the same refusal shape.
- **Pre-push hook is stricter.** Applies the full L4-policy pipeline: version
  floor (`required_anvil_version`), cutoff commit (`cutoff_commit`), a 2 s time
  budget (emits `partial=true` on exceed rather than blocking), and
  witness-chain DAG verification. Diagnostic lines map onto the recovery
  procedures in the
  [witness-chain runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-witness-chain.md).
- **YAML resource bounds enforced.** `anvil-config::parse` now rejects YAML
  aliases outright (1 MiB pre-parse cap, depth-32 post-parse cap). A
  `.anvil.yaml` that uses YAML anchors / aliases will be refused.
- **Fence persistence extended for cascade.** Fences still persist across daemon
  restart by design (carry-forward from `v0.6.0-beta`); the persisted state now
  also tracks the four-fence cascade window (`RateWindow::new(4, 60s)`), and
  clearing a cascade requires
  `anvil intercept unblock --acknowledge-cascade <worktree>`.

### Security

- **End-to-end agent-tag spoof rejection.** The launcher and TypeScript
  driver-client forward each writer's `ANVIL_AGENT_TAG` and PID lineage to the
  daemon, which cross-checks them against the tag it issued at registration.
  Spoofed tags block the offending write and fence the worktree with
  `degraded:spoofed-attribution`.
- **`anvil l4-validate` chain integrity check.** L4 validation now verifies
  witness-chain integrity before trusting any witnessed commit SHA as
  prior-layer evidence. Broken or tampered chains produce a blocking result
  instead of a silent allow or empty trusted set.

### Fixed

- **Local-noise ignore policy now covers every surface.** Generated files, cache
  directories, and agent worktrees are ignored consistently across `watch`,
  `audit`, hooks, baseline, drift, gate, and `anvil-run`. The canonical list
  lives in the kernel and is re-exported to CLI surfaces so the two cannot
  drift; `.venv` is now included and `__pycache__` reconciled.

### Operator artefacts

- [v0.6.x → v0.7.0-beta migration note](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md)
  — surface delta and operator action.
- [v0.7.0-beta release runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
  — protection-claim hard gate, tag-time pre-flight, recovery procedures.
- [`anvil-run` manpage](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-run.md)
  — wrapped-launch ingress, stable exit codes (`64 / 69 / 73 / 75 / 78`),
  shell-integration semantics.
- [Witness-chain operator runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-witness-chain.md)
  — line shape, failure modes, recovery procedures.
- [Hook coexistence runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-hook-coexistence.md)
  — install/uninstall behaviour per host hook manager.
- [Air-gap runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-air-gapped.md)
  — no-network promise (unchanged from `v0.6.x`) and how it's enforced in CI.

## [0.6.3-beta] — 2026-05-15 — Beta Watch UX + Uninstall Hotfix

Patch release for beta-user first-run and watch friction. No new APIs and no
breaking changes; the upgrade is drop-in for existing installs.

### Fixed

- **Homebrew-aware curl installer.** `install.sh` now detects an existing
  Homebrew-managed `anvil` binary before download. When found, it exits
  successfully and prints `brew upgrade eddacraft/tap/anvil` instead of
  overwriting the Homebrew-managed binary.
- **Watch and audit ignore local agent/tool worktrees and caches by default.** A
  shared ignore list covers `.claude`, `.opencode`, `.gemini`, `.serena`,
  `.worktrees`, and the usual generated/cache/build directories (`node_modules`,
  `target`, `dist`, and others).
- **Initial watch scan is baseline/readiness state, not new violations.**
  Existing public exports, dependencies, and cross-layer imports are no longer
  reported as save-time findings when `anvil watch` starts; only later file
  changes that introduce or re-surface an issue trigger findings.
- **`anvil watch` shows immediate startup feedback.** A terse "starting" line
  prints before the slow setup phase, so large repos no longer look hung on
  launch. `anvil watch` also falls back to plain output when stdin or stdout is
  not a terminal.

### Added

- **`anvil uninstall` command.** Project-scoped removal of Anvil state
  (`.anvil/`, `.anvilrc`, and Anvil-managed git hooks). Pass `--global` to also
  remove user-level state (`~/.anvil/`), Anvil MCP entries from `~/.claude.json`
  and `~/.cursor/mcp.json`, stored credentials, and the running daemon. The
  Anvil binary itself is never removed — uninstall that with Homebrew, WinGet,
  Scoop, Cargo, or the installer path after cleaning state. Auth-bypass is built
  in so stuck installs can be cleaned without logging in.
- **Refreshed beta and watch help.** The beta testing guide, troubleshooting,
  and quickstart now cover the watch baseline-scan semantics, the shared ignore
  policy, non-TTY fallback, and the new uninstall escape hatch.

### Upgrade

- Homebrew: `brew upgrade eddacraft/tap/anvil`.
- curl installer: rerun the installer — it now detects Homebrew and steps aside
  with the correct upgrade hint.
- WinGet / Scoop / direct download: pick up the new release as normal; no
  manifest or installer shape change.

## [0.6.0-beta] — 2026-05-07 — Wow-Start Activation & Daemon-Backed Mid-Edit Validation

### Added

- **`anvil start` activation entrypoint** — `install → cd repo → anvil start` is
  now the canonical first minute. `anvil start` is the dedicated activation
  command, with `--verify` for a read-only protection probe and `--watch` to opt
  into the save-time fallback when MCP cannot attach.
- **Activation protection states** — `protecting`, `ready_restart_required`,
  `watching`, `needs_action`, `unsupported`, and `error` are now the single
  shared vocabulary across `anvil start`, `anvil status --verify`,
  `anvil doctor`, and the tutorial. Operators and agents see the same literal
  state on every surface.
- **`anvil doctor` project-id check** — `anvil doctor` now verifies the
  `anvil/project-id` state written by `anvil start` and surfaces missing (warn)
  or malformed (fail) project identity as a doctor finding, so support can
  confirm at a glance whether a repo has been activated.
- **`anvil mcp install` for Cursor and Claude Code** — one-step MCP activation
  that writes `~/.cursor/mcp.json` or `~/.claude.json` directly, with an
  interactive picker when both are present. Windsurf, VS Code, and the
  HTTP-transport flows remain on `anvil mcp-config`.
- **Daemon-backed `anvil_validate_write` MCP tool** — the MCP pre-write
  validation path now routes through the local daemon over owner-only IPC (Unix
  domain socket on Linux/macOS; named-pipe on Windows). The embedded validation
  pipeline remains as a correctness-equivalent fallback when the daemon is not
  reachable.
- **Repo language profile** — `anvil start`, scan, and watch now honour a
  per-repository language profile so coverage claims are honest. TypeScript is
  the supported tier in `0.6.0-beta`; SQL and Markdown are partial; Python and
  Rust are reported as unsupported instead of silently skipped. Cross-language
  checks (secrets) continue to run on all files.
- **Protection-loop tutorial** — the default tutorial path now walks through the
  protection loop end-to-end: protection-loop intro, fixture description,
  simulated check, the activation-state vocabulary, and a real
  `anvil start --verify` run. The four legacy paths (Policy, Architecture,
  Drift, CI) remain available.
- **`anvil version`** — install-method-aware version surface that detects
  Homebrew, Scoop, WinGet, the installer, or a dev build, and prints current
  version, latest version, `update_available`, install method, and the
  recommended upgrade command. The JSON shape is pinned for agent and CI
  consumers.
- **macOS daemon peer-credential validation** — the daemon now uses
  `getpeereid(2)` on macOS for the same UID-based same-user trust check the
  Linux build performs via `SO_PEERCRED`. macOS deployments are at parity with
  Linux on the daemon trust boundary.
- **`anvil intercept status` on every supported target** — operator status
  surface for the daemon over local IPC on Linux, macOS, and Windows; `--json`
  returns the same shape on every OS, covering sessions, fences, latency, and
  uptime.
- **`anvil admin` parity in the main CLI** — admin operational commands are now
  reachable from the main `anvil` binary alongside the existing `anvil-admin`
  operator CLI.

### Changed

- **Foreground is the only supported daemon launch mode in v1** — start the
  daemon with `anvil intercept start --foreground`. Service-manager integration
  should run it under foreground supervision; background launch mechanics are
  not a v1 surface.
- **Fences persist across daemon restart by design** — an interrupted
  enforcement decision is no longer silently undone after a daemon crash,
  restart, or reboot. Recovery procedure and the deferred `anvil intercept stop`
  / `unblock` CLI subcommands are documented in the
  [v0.6.0-beta operator runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.0-beta-release-runbook.md).
- **macOS interrupt path is fence-first this release** — on macOS the interrupt
  ladder falls through to fence-on-uncertainty rather than running the full
  SIGINT → SIGTERM → SIGKILL sequence. Recovery procedure is documented in the
  [v0.6.0-beta operator runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.0-beta-release-runbook.md).
- **Windows MCP correlation gap** — `correlation.daemonStatus` returned by
  `anvil_validate_write` is always `not-wired` on Windows in this release; the
  daemon and `anvil intercept status` are wired, only the MCP correlation
  envelope on Windows is not. The narrower fix is tracked as a follow-up.
- **Public docs refresh** — install, quickstart, and the broader public Anvil
  docs were aligned with the activation-first first-minute and the daemon-backed
  MCP path.

### Fixed

- **MCP restart handshakes** — activation now waits for the MCP client's restart
  handshake before claiming `ready_restart_required` is resolved, so the next
  status read reflects the real connected state instead of the stale-pre-restart
  one.
- **Activation denylist alignment** — the activation pre-scan now uses the same
  file filter as the steady-state scan, so the first-signal walk does not
  surface findings the steady-state scan would skip.
- **Activation baseline** — old findings are baselined before the first
  activation signal, so the first genuine save produces a real signal rather
  than a long pre-existing list.
- **Honest watch fallback** — when MCP cannot pre-write attach, `anvil start`
  surfaces partial-protection messaging and offers the watch-mode fallback
  explicitly instead of pretending activation succeeded.

### Developer

- **Windows CI cross-compile runs on `main` and `dev`** — the cross-compile
  matrix now runs on pushes and PRs targeting either branch, closing the gap
  that let Windows-only build breakage land on `dev` between releases. Local
  reproduction:
  `cargo test --workspace --target x86_64-pc-windows-msvc -- --test-threads=1`
  matches the cross-compile job's smoke-test step.
- **MCP daemon integration tests run on Unix only this cut** — the daemon-backed
  integration suite is not yet wired for Windows; Windows coverage rides the
  same follow-up that closes the MCP correlation gap above.
- **Operator artefacts** — the release ships the
  [v0.6.0-beta operator runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.0-beta-release-runbook.md)
  (five operator items) and the
  [v0.6.0-beta security note](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.0-beta-security-note.md)
  (four HIGH security trade-offs documented for review).

## [0.5.1-beta] — 2026-05-03 — Scanner Signal & TUI Hotfixes

### Changed

- **TypeScript package subpaths** — archived scanner-era subpath exports were
  removed from `@eddacraft/anvil-core` and `@eddacraft/anvil-runtime`; use the
  Rust CLI surfaces for antipattern, suppression, drift, gate, and export flows.

### Added

- **TUI zoom controls** — audit, status, and watch surfaces now support zooming
  to inspect dense output more comfortably.

### Fixed

- **Secret scanner false positives** — generic secret matching keeps the
  v0.5.0-beta false-positive reductions while preserving dotted and
  punctuation-bearing secret values.
- **Antipattern suppressions** — `AP-*` checks now honour local `eslint-disable`
  directives, and `GS-001` avoids reporting guarded `Map.get` after `has`/`set`
  flows on the same map receiver.
- **Audit noise** — audit scans skip broader environment-template files while
  still reporting real `.env` files regardless of directory.
- **Doctor and tutorial interactions** — doctor now acknowledges `f` to fix, and
  tutorial path selection has more room for wrapped options.
- **Incremental kernel imports** — watch updates now keep synthetic import IDs
  separate from the allocator and treat import-source ID `0` as valid, avoiding
  missed or colliding import edges during incremental graph refreshes.

### Developer

- The TypeScript scanner stack and parity harness were archived now that the
  Rust scanner is authoritative.
- A PR base guard workflow now detects release-sensitive PRs targeting the wrong
  branch when required by branch protection.

## [0.5.0-beta] — 2026-05-01 — AI Guardrails & Mid-Edit Validation

### Added

- **Git config-mode hooks** — `anvil hooks install --config` and
  `anvil hooks uninstall --config` manage Git 2.54 native `hook.<event>.command`
  entries; file-mode hooks remain the default and Husky stays as the contributor
  bootstrap.
- **Hook setup visibility** — `anvil hooks status`, `anvil doctor`, onboarding,
  and tutorial now recognise config-mode hooks, `core.hooksPath`, third-party
  hook managers, and duplicate file/config execution risk.
- **AI guardrail profile** — `anvil gate --profile ai` runs the AI-focused check
  set with strict configuration handling and a stable JSON envelope for agent
  and MCP consumers.
- **AI-001 reasoning rule** — flags source comments that justify code with
  authority, social proof, or deflection instead of technical reasoning; honours
  `// @anvil-ignore AI-001` and emits at info severity.
- **`.env` and `.envrc` secret scanning** — `.env`, `.env.*`, and `.envrc` files
  are parsed as key/value files so leaked values report the variable name and
  source line (`SURFENV-001`).
- **`anvil mcp-config`** — Rust CLI command generates, verifies, and writes
  Claude Code, Cursor, Windsurf, and VS Code MCP server configuration with
  stdio/http transports, `--write`, `--verify`, workspace overrides, path-safety
  prompts, and atomic writes.
- **API SQL migration runner** — Anvil API deploys now have a first-party SQL
  migration runner with dry-run support and drift detection.

### Fixed

- **Doctor outside git repos** — missing git repositories now surface as a
  structured warning through the doctor JSON contract rather than failing the
  whole run.
- **First-run guidance** — init and onboarding copy now points unauthenticated
  users at `anvil auth login` where required, and inotify capacity warnings are
  clearer about what to change.
- **Release publishers** — Scoop and WinGet publishing paths now fail earlier on
  token or fork problems, and the cargo-dist installer is pinned by SHA256 in
  the release workflow.
- **API deploy stability** — CORS preflight caching, Vercel API routing, and the
  `svix`/`uuid` runtime override were tightened after post-release deploy
  failures.

### Improved

- **Scan performance and safety** — repository scans now use the shared parallel
  walk pattern across more CLI surfaces, skip oversized lines before regex
  evaluation, and cap first-run scan threads via `ANVIL_SCAN_THREADS` (default
  `min(num_cpus, 4)`).
- **AI workflow docs** — the AI guardrail profile, MCP and editor setup path,
  and beta tester guide now describe the current Rust CLI behaviour.
- **Git hook docs** — public docs now explain file-mode versus config-mode
  hooks, coexistence warnings, and the current decision to keep Husky as the
  contributor bootstrap.
- **Beta validation scenarios** — public tester scenarios were refreshed around
  the current onboarding, hooks, AI guardrail, and MCP flows.

## [0.4.0-beta] — 2026-04-25 — First-Run Polish & Native Scanner

### Changed (breaking)

- **`anvil watch --exclude` now uses glob patterns** — pass
  `--exclude 'vendor/**'` to skip a directory tree; bare directory names now
  warn so existing scripts surface the change instead of silently watching the
  wrong paths.
- **JSON output now carries notifications** — `anvil doctor --json` returns
  `{ "checks": [...], "notifications": [...], "schema_version": "2.0.0" }`, and
  `check`, `gate`, and `audit` include `notifications[]` alongside their
  existing payloads.

### Added

- **Native Rust scanner** — the Rust engine is now the authoritative scanner,
  with registry-backed rules, parallel scanning, rule provenance on findings,
  and fixture coverage for every shipped rule.
- **First-run scan after `anvil init`** — new projects get immediate findings,
  counts, and `file:line` pointers instead of being sent to another command.
- **Watch filtering** — `anvil watch --patterns` and `--exclude` now drive the
  watch loop, with a startup banner showing the active include/exclude scope.
- **`anvil check --artifact`** — scan generated files, build outputs, and other
  opaque artefacts outside the normal source-file filter.
- **`anvil licenses`** — prints bundled third-party attributions from
  `ACKNOWLEDGEMENTS.md`.
- **Scoop distribution** — Scoop joins WinGet and the existing installers, with
  README install instructions covering every supported package manager.
- **Per-operator admin keys** — admin operations can now use individually
  provisioned operator credentials instead of a single shared admin key.

### Fixed

- **`anvil watch` reliability** — partial setup failures no longer abort the
  loop, per-change panics are isolated, Ctrl-C exits cleanly, and error chains
  no longer leak the current working directory.
- **`anvil doctor` remediation** — checks now show concrete next actions, and
  `--fix` writes a valid default `.anvilrc` without running `git init` in unsafe
  directories.
- **`anvil init` robustness** — post-init git-history sampling now times out
  instead of hanging on slow filesystems or stalled remotes; tight inotify
  limits are reported up front with a fix hint.
- **Config-driven gate checks** — `.anvilrc` check selection now uses the same
  canonical names as the gate runner.
- **Non-interactive mode** — empty `ANVIL_NO_PROMPT` and `NONINTERACTIVE` values
  now correctly opt out of prompts.
- **Admin CLI and auth flows** — route coverage, timestamp validation, JSON
  output hygiene, EOF handling, and migration-send safety were tightened across
  the beta access surfaces.
- **Tutorial and TUI papercuts** — tutorial exit codes, `husky` handling, ASCII
  fallback, narrow-terminal titles, discovery scrolling, and `.anvilrc`
  detection races were corrected.

### Improved

- **Onboarding language** — init, welcome, tutorial, and watch now use the same
  defaults and explain scan truncation or watcher failures where the user can
  act on them.
- **Public docs** — release pages, install docs, the quality model, and the
  `.anvil` pattern reference were refreshed for the native scanner release.
- **Release preflight** — the historical bundled release gate ran Rust and
  TypeScript fmt, lint, typecheck, and tests before release.

## [0.3.3-beta] — WinGet Distribution & Windows UX

### Added

- **WinGet distribution** — Windows users can now install and upgrade anvil via
  WinGet.
- **Scoop support** — Scoop bucket install guidance is now part of the supported
  Windows distribution surface.
- **Admin operator tooling** — the separate `anvil-admin` operator CLI now
  covers `list`, `show`, `approve`, `invite`, `audit`, `revoke`, and
  `send-migration` for beta access operations.
- **Nightly stress test workflow** — CI benchmark coverage expanded to catch
  native-engine regressions earlier.

### Fixed

- **Windows TUI input** — duplicate keypress handling on Windows was removed.
- **Discovery and tutorial UX** — onboarding and tutorial completion flows were
  stabilised after the Rust cutover.
- **Admin approval and migration flows** — reliability and error handling were
  tightened across the beta-access operator tooling.

### Improved

- **Release automation** — Windows signing groundwork, public-release promotion,
  and release-script hardening all landed as part of the `0.3.3-beta` cycle.

## [0.3.2-beta] — Update Command & Onboarding Completion

### Added

- **`anvil update`** — the native binary now ships with an in-place updater via
  the `anvil update` command.
- **Welcome/onboarding completion** — the first-run experience, tutorial, and
  welcome hub reached feature-complete beta coverage.
- **Interactive release workflow** — release automation gained a manifest-driven
  handoff for agent-assisted release verification.

### Fixed

- **Tutorial command drift** — tutorial steps were realigned with the shipped
  Rust CLI.
- **Install flow polish** — installer next steps and Homebrew publishing were
  made more reliable.

## [0.3.1-beta] — Docs Cutover & Onboarding Fixes

### Added

- **Docs domain cutover** — `docs.eddacraft.ai` now served via a dedicated proxy
  with shared-secret middleware and a Nordic terminal-themed landing page.

### Fixed

- **Welcome screen** — first-user onboarding flows restored after regressions in
  0.3.0-beta.
- **Auth error messages** — raw HTTP errors replaced with user-friendly messages
  in device-code and login flows.
- **TUI version display** — shell footer now shows the correct version string.
- **Release pipeline** — ARM64 Windows target removed from cargo-dist (upstream
  dependency not yet available).

### Security

- **OAuth state hardening** — replay protection via issued-at timestamp with
  600-second expiry; nonce cookie cleared on all callback exit paths.
- **Docs proxy** — upstream auth redirect blocking and response header stripping
  for shared secrets.
- **CI credential scoping** — Azure credentials passed as composite action
  inputs instead of job-level environment variables.
- **Feature flag validation** — strict input validation on snapshot loading and
  explicit reason codes for unimplemented operators.

## [0.3.0-beta] — Rust CLI & Native Engine

### Changed

- **Native Rust binary** — anvil is now distributed as a single static binary
  with no Node.js runtime required. The Node.js package (`@eddacraft/anvil-cli`)
  is deprecated and will not receive further updates. See
  [The Switch to Rust](./rust-rewrite.md) for background on the Rust cutover and
  the small amount of path-cleanup guidance needed if you still have the old npm
  CLI installed.
- **Installation** — `curl -fsSL https://install.eddacraft.ai | sh` (macOS /
  Linux) or `irm https://install.eddacraft.ai/windows | iex` (Windows). Also
  available via Homebrew: `brew install eddacraft/tap/anvil`. Built-in
  self-updater via `anvil update`.
- **Platform support** — builds for x86_64 and aarch64 on macOS, Linux, and
  Windows (6 targets via cargo-dist).
- **Ratatui TUI** — all 10 interactive surfaces (welcome, tutorial, watch,
  wizard, status, doctor, init, audit, browser, gate) rebuilt using Ratatui with
  the eddacraft Terminal Standard design system.
- **Structured exit codes** — `0` (pass), `1` (general error), `2` (gate
  failure), `3` (auth required), `4` (config error).
- **Docs gating** — the `/anvil` documentation is now gated behind GitHub OAuth
  for beta users. Public docs (APS, Kindling, edda-stack) remain open.

### Added

- **Welcome screen & onboarding** — first-run detection with an interactive
  onboarding experience covering tutorial paths, live watch demo, and hook
  installer guidance. `anvil welcome` anytime.
- **Kernel engine** — native core engine with file watching, incremental
  tree-sitter parsing, and a real-time semantic dependency graph (petgraph).
  Supports foreground watch, embedded one-shot checks, and a dual-run harness
  for engine comparison.
- **Rust check ports** — secret scan, anti-pattern detection, command safety,
  architecture parity tests, and benchmarks all ported to Rust.
- **Beta authentication** — passwordless device-code and OTP flows with secure
  credential storage, session refresh with theft detection, and
  `anvil auth login` / `anvil admin approve` commands.
- **New commands** — `anvil new` (template browser), `anvil wizard` (interactive
  setup), `anvil audit` (security findings scan), `anvil drift` (snapshot,
  compare, report, list), `anvil validate` (APS plan validation), and
  `anvil gate-config` (gate thresholds). `--json` output mode across all
  commands.
- **MCP config generation** — library functions for generating MCP server
  configuration for Claude Code, Cursor, Windsurf, and VS Code.
- **Kernel benchmarks** — criterion benchmarks and a stress test harness for
  watcher, parser, graph, and policy evaluation, wired into CI.

### Performance

- 5–10x faster scanning on typical projects.
- 80% less memory in watch mode (~30–50MB vs ~400MB for a 5,000-file monorepo).
- Cold start under 10ms (vs 200–400ms with Node.js).
- Tree-sitter parse throughput ~15,000 files/second.
- Parallel file walks via rayon; watch mode excludes ignored directories at the
  OS level.

### Security

- Device-code and OTP authentication hardened with theft detection on session
  refresh.
- Atomic credential file writes with restrictive permissions at creation time.
- Log inputs sanitised to prevent log injection.
- GitHub Action expression injection sanitised; all GitHub Actions pinned to
  commit SHAs.
- Dependency patches: fast-xml-parser (CVE-2026-33036), `@hono/node-server`
  (CVE-2026-39406), axios, picomatch, undici, yauzl, rustls-webpki, and others.

## [0.2.1-beta] - 2026-03-26

### Added

- **Project memory** — anvil now tracks patterns and decisions in your codebase
  via the Edda memory system and Ember proposal engine.
- **Security hardening** — input validation and subprocess execution
  improvements across the platform.
- **Dependency patches** — minimatch, axios, svgo, tar, and others.

## [0.2.0-beta] - 2026-03-14

### Added

- **Licence verification** — offline JWT verification replaces auth-check;
  background refresh, file store with resolution order, `whoami` licence details
- **Edda Stack memory system** — Ember candidate proposals, Edda canonical
  memories, observation-to-proposal mapping, provenance tracking, evolution
  service, and full integration contracts (`@eddacraft/edda-stack`)
- **Forge & Temper pipeline** — pre-commit code review via cross-model
  negotiation (Forge hook + reviewer agent), structured finding/response
  protocol with round cap, deferred finding auto-filing as GitHub issues, Temper
  self-healing GitHub Actions workflow with 2-cycle cap
- **Rust kernel spike** — Phase 0 validated: tree-sitter parsing, notify-rs
  watcher, petgraph dependency graph, Cargo workspace with CI
- **Rust check ports** — secret scan, anti-pattern detection, and command safety
  ported to Rust for 10-40x speedup
- **Ratatui TUI** — shared component library for native terminal UI, 10 Ink
  components ported to Ratatui
- **Security CI pipeline** — Semgrep SAST, dependency audit, TruffleHog secret
  scan, licence compliance, OSSF Scorecard on every PR
- **Interactive release command** — `anvil release` for guided CLI releases
- **APS nested index loading** — depth-limited recursive plan loading
- **Tutorial continuation** — continue to another tutorial path from completion
- **JSON output** — `--json` flag on `hooks status` and `plan create`

### Changed

- Monorepo CI split: lightweight dev-branch checks, full release gate on PRs
  targeting `main` (cross-platform macOS + Windows)
- Concurrency groups on all CI workflows to prevent redundant runs
- BMAD adapter supports v6 YAML document format
- APS state uses atomic locking with `O_EXCL` lock files

### Fixed

- 292 bug fixes across CLI, API, APS state management, adapters, security hooks,
  and documentation
- Pulumi provider download failures from GitHub API rate limits (GITHUB_TOKEN
  now passed to plugin downloader)
- Watch mode signal handler reliability
- CLI error handling and edge case robustness

### Security

- Automated security scanning on every PR (Semgrep, dependency audit,
  TruffleHog, licence compliance)
- Pre-commit security hook hardened (subshell variable loss, heredoc injection)
- OSSF Scorecard pinned to v2.4.3

## [0.1.2-beta] - 2026-02-22

### Fixed

- CLI error handling improvements
- Watch mode signal handler reliability

## [0.1.1] - 2026-02-21

### Added

- Interactive tutorial system
- Doctor diagnostics command
- Explain command for rule details
- Watch mode profiles (source, plans, all)

### Fixed

- Init wizard project detection improvements

## [0.1.0] - 2026-02-21

### Added

- Core validation engine with architecture boundary checks
- Anti-pattern detection (AP-001 through AP-007)
- Secret detection with pattern matching and entropy analysis
- Watch mode for real-time validation
- GitHub Action for CI integration
- VS Code extension for in-editor diagnostics
- TUI (terminal UI) for interactive commands
- MCP server configuration generation
- OPA/Rego custom policy support
- Drift detection with snapshot comparison
- Evidence system for audit trails
- Beta access and authentication system

### Security

- All evidence is cryptographically signed
- Secrets never appear in logs or output

---

## Versioning

anvil follows [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking changes to config or CLI
- **MINOR** — new features, backward compatible
- **PATCH** — bug fixes

## Upgrading

See [Upgrade Notes](/anvil/releases/upgrade-notes) for migration guides.

---

**See also:** [Upgrade notes](/anvil/releases/upgrade-notes),
[The Switch to Rust](/anvil/releases/rust-rewrite)
