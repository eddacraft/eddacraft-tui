# Changelog

All notable changes to this product are documented here.

This changelog contains customer-relevant changes only. Internal refactors and
engineering maintenance are recorded in the
[Engineering History](./ENGINEERING-HISTORY.md).

## [0.8.0-beta] — TBD — The Save-Time Daemon

> **Draft / unreleased.** This section accumulates customer-relevant changes
> landed on `main` since `v0.7.4-beta`; the date and final scope are pending the
> tag cut. The first minor since `v0.7.0-beta`, earned on architecture: it
> begins moving save-time governance off per-save cold-spawned `check` and onto
> a persistent intercept daemon that validates deltas
> ([ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md)). Most
> of the sub-phase A daemon work is foundational plumbing; the user-visible
> surface so far is the new `anvil status` assurance fields, opt-in workspace
> confinement, and the gate-summary dashboard.

### Added

- **`anvil status` surfaces save-time assurance and workspace confinement.**
  Status now reports whether save-time validation is being served by the
  persistent daemon and the workspace's confinement state, so the active
  protection posture is observable (DSV-007).
- **Opt-in workspace confinement mode.** A new opt-in mode confines the daemon's
  save-time validation to an admitted workspace root
  ([ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md) §7,
  DSV-008).
- **Live gate-summary dashboard.** `anvil gate` now persists run results to
  `.anvil/gates.json`
  ([#2242](https://github.com/eddacraft/anvil-001/issues/2242)), and a new live
  gate-summary dashboard surface renders them
  ([#2237](https://github.com/eddacraft/anvil-001/issues/2237), TUIDASH-013).
- **First-run adoption hint.** Anvil surfaces a first-week adoption signal hint
  to help new projects find their footing (INSIGHTS-004).

### Changed

- **`anvil watch` routes save-time checks through the persistent intercept
  daemon.** The foundational shift from cold-spawning `check` on every save to
  daemon-served delta validation lands behind the existing watch surface
  ([ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md)) — the
  durable fix for the watch-CPU report
  ([#2156](https://github.com/eddacraft/anvil-001/issues/2156)) that
  `v0.7.4-beta` addressed only with the RLB-007 stopgap.

### Fixed

- **Windows home resolution honours `USERPROFILE` / `HOME`.** Home-directory
  resolution on Windows now respects `USERPROFILE`/`HOME` consistently.
- **`anvil suppress` rejects drive-relative paths.** Suppression now rejects
  drive-relative file paths (e.g. `C:foo`), not just rooted ones.
- **Deep-tree TUI rendering no longer overflows the stack.** `eddacraft-tui`
  drops large render trees iteratively, preventing a stack overflow on deep
  component trees (eddacraft-tui 0.2.4).

## [0.7.4-beta] — 2026-06-01 — Side-by-Side Installs

A distribution and stability patch on the `v0.7.3-beta` slate. The headline is
an `ANVIL_HOME` install-root override that lets a development or candidate Anvil
run beside a production install without colliding on state. It also lands a
watch CPU fix that keeps per-save checks scoped to what actually changed, plus
Windows daemon hardening and a few CLI correctness fixes.

### Added

- **`ANVIL_HOME` / `--anvil-home` install-root override.** Anvil now resolves a
  single install root from the `--anvil-home` flag or the `ANVIL_HOME`
  environment variable, re-rooting the daemon socket, PID file, stored
  credentials, and all durable project-state writes underneath it. This lets a
  development or candidate Anvil run side-by-side with a production install
  without sharing or clobbering state. `anvil status --json` now reports
  `install_root` and `project_writes_gated`, so the active root and its
  write-gating are observable, and the project-mutating commands (`config`,
  `gate-config`, `hooks`, and the other state-writing commands) honour the gate
  ([#1726](https://github.com/eddacraft/anvil-001/issues/1726)). See the
  [side-by-side install runbook](./docs/runbooks/anvil-home-side-by-side.md).

### Changed

- **`anvil watch` per-save checks now scope to the files that changed.**
  Previously each save re-ran code-quality checks across the whole project;
  under multiple concurrent agents this saturated CPU — a single watch agent
  consumed ~7 of 16 cores, and ~2 agents could saturate the box
  ([#2156](https://github.com/eddacraft/anvil-001/issues/2156)). Per-save checks
  now scope to the changed paths. `anvil gate` is unchanged (it already
  self-scopes via git), and the untracked-file watch contract from
  [#1913](https://github.com/eddacraft/anvil-001/issues/1913) is preserved.

### Fixed

- **Windows daemon hardening.** The named-pipe client now caps its security
  quality-of-service (SQOS) impersonation level and verifies the server process
  is alive before connecting, closing two Windows-only daemon issues found in
  the clawpatch sweep.
- **Credential load faults return a configuration-error exit code.** A failure
  loading stored credentials now surfaces as `EXIT_CONFIG_ERROR` instead of a
  generic failure.
- **`anvil` honours per-command `--format json` in the auth gate.** The auth
  gate previously could ignore a command's `--format json` selection.
- **Blank `ANVIL_HOME` / `--anvil-home` is treated as unset.** An empty value no
  longer resolves to an empty install root; it falls back to the default
  consistently.

## [0.7.3-beta] — 2026-05-31 — Surfacing the Signal

A product-surface release: native read-only TUI dashboards, SARIF 2.1.0 findings
export on the scan commands, and new `anvil insights` views make Anvil's
existing signal visible and exportable.

### Added

- **SARIF 2.1.0 output** for the finding-emitting commands. `anvil check`,
  `anvil gate`, and `anvil audit` accept `--format sarif` and emit the GitHub
  Code Scanning subset of SARIF 2.1.0, so Anvil findings can be uploaded to Code
  Scanning (and other SARIF tools) without a per-command adapter. `check`'s
  `@anvil-ignore`-suppressed findings render under `suppressions[]`. The new
  `--format <auto|tui|plain|json|sarif>` flag on these commands is the canonical
  output selector; `--json` continues to work as an alias, and SARIF is never
  auto-selected. SARIF emission is exit-code-neutral. See the
  [GitHub integration guide](./docs/public/anvil/integrations/github.md) and the
  [SARIF upload runbook](./docs/runbooks/sarif-code-scanning-upload.md).
- **`anvil --json watch` is now a stable NDJSON consumer surface** — the watch
  event stream pins to `anvil.watch.event.v1`. Every stdout line carries
  `schema_version`, `seq`, `timestamp`, `event_type`, and a typed `payload`.
  stdout is reserved for event records; warnings, banners, and child action
  stderr route to stderr. See
  [Watch JSON Output](./docs/public/anvil/integrations/watch-output.md) for the
  consumer guide and
  [`docs/specs/watch-output-contract.md`](./docs/specs/watch-output-contract.md)
  for the normative spec.
- **`anvil dashboard` — native read-only TUI dashboards.** A new command with
  three live surfaces over persisted `.anvil/` state: **Architecture Health**
  (layer boundaries, violations, and rule compliance), **Drift Snapshots**
  (snapshot history and new-edge deltas vs baseline), and **Suppressions**
  (active suppressions with scope, file, reason, and expiry). Run
  `anvil dashboard` for an interactive picker, or
  `anvil dashboard <architecture|drift|suppressions>` to open one directly.
- **`anvil insights --suppressions` — suppressions health view.** Lists every
  active inline suppression in the project
  ([#1996](https://github.com/eddacraft/anvil-001/issues/1996)).
- **`anvil insights --drift` — drift trend sparkline.** Renders new
  cross-boundary edges per week over the last 8 weeks as a terminal sparkline,
  derived from the `anvil drift` snapshot store. Weeks without a snapshot read
  as no-data (distinct from a measured zero), and `--json` emits a
  schema-versioned `anvil.drift_trend.v1` document.
- **`anvil migrate schema` subcommand.** Reads the project's
  `created_by_version` and migrates the config schema forward
  ([#1984](https://github.com/eddacraft/anvil-001/issues/1984)).
- **Secret scanning now covers on-disk content, not just git history.** The
  secret scan that previously inspected commit history also scans the working
  tree ([#1994](https://github.com/eddacraft/anvil-001/issues/1994)). The
  git-history scan also reports the count of oversize lines it skipped, so a "0
  findings" result can no longer silently hide unscanned content.
- **`anvil welcome` reports the count of files skipped via `.gitignore`** in its
  discovery output, so the scan scope is no longer silent.

### Changed

- **Policy engine hardened.** The Rego evaluation path behind the experimental
  `anvil policy eval` now catches panics at the regorus facade (dedicated unwind
  profile), enforces a determinism fence and input bounds, and emits tracing on
  the eval path ([#1952](https://github.com/eddacraft/anvil-001/issues/1952)).
  `anvil policy eval` remains a preview — its output shape may still change.
- **Watch JSON payloads are typed, not debug strings** — `anvil --json watch`
  previously emitted lines whose `detail` field was a Rust debug-formatted
  string of the kernel event payload. Consumers reading that string MUST migrate
  to the structured `payload` object. The pre-WOUT shape was not guaranteed and
  is no longer produced.
- **`anvil welcome` discovery scan is faster.** The first-run discovery walk is
  now parallelised, cutting the dominant scan cost on large repositories while
  preserving deterministic finding order.

### Fixed

- **CLI diagnostics no longer corrupt `--json` output.** CLI log events
  (including default-level `warn!`/`error!` and anything enabled via `ANVIL_LOG`
  / `RUST_LOG`) were written to stdout, interleaving log lines with command
  output and breaking `anvil … --json` for `jq` and pipeline consumers. CLI
  diagnostics now go to stderr, leaving stdout reserved for command output.
- **`anvil update` no longer silently drops `--insecure-skip-verify` on the
  sidecar update path.** The flag was accepted by `clap` and honoured on the
  library-fallback path but silently ignored on the sidecar path; it now emits a
  loud warning when set on that path
  ([#1735](https://github.com/eddacraft/anvil-001/issues/1735)).
- **Scanners preserve distinct findings on the same line.** Multiple separate
  findings sharing a line are no longer collapsed into one.
- **Workflow installs now require explicit consent**, and workflow file writes
  are hardened against unintended overwrites
  ([#2003](https://github.com/eddacraft/anvil-001/issues/2003)).

## [0.7.2-beta] — 2026-05-25 — Save-Time Scanning & Tooling Honesty (Boring Week Patch 2)

`v0.7.2-beta` is the second Boring-Week patch tag on the `v0.7.0-beta`
daemon-working slate. It lands the beta-feedback fixes reported against
`v0.7.1-beta`: bare `anvil watch` now actually runs the code-quality scanners
instead of reporting "100% pass" while inspecting nothing, the antipattern rules
stop flagging `any`/`!` inside comments and strings, `anvil version` warns when
a stale shadowed binary is on `PATH`, and `anvil auth refresh` reports the real
90-day refresh window. It also introduces the experimental `anvil policy`
command group.

### Changed

- `anvil watch` now runs code-quality checks (`anvil check --all`) on each save
  by default. Previously a bare `anvil watch` watched architecture and
  dependency edges only and ran no code-quality scan, while the dashboard still
  read as "100% pass" — protection it was not providing
  ([#1913](https://github.com/eddacraft/anvil-001/issues/1913)). Run
  `anvil watch --action none` to restore the architecture/dependency-only watch.
  `anvil start --watch` is unchanged (remains architecture-only).

### Fixed

- **AP-003/GS-001 no longer flag `any` or `!` inside comments and string
  literals.** The antipattern scanner now masks comments, string literals, and
  regex literals before applying code-construct rules, so prose or string
  content that merely mentions `any` or contains a `!` is no longer reported as
  a finding. Match positions for genuine findings are unchanged
  ([#1914](https://github.com/eddacraft/anvil-001/issues/1914)).
- **`anvil version` warns when another `anvil` on `PATH` shadows the running
  binary.** On Windows a stale cargo-dist install in `~/.eddacraft/bin` could
  shadow a freshly-updated Scoop shim, so `scoop update` reported the new
  version while `anvil` kept running the old one. `anvil version` now detects
  the shadowing install and reports it
  ([#1920](https://github.com/eddacraft/anvil-001/issues/1920)).
- **`anvil auth refresh` reports the 90-day refresh window.** The command
  advertised a 90-day session in `--help` but printed only the ~7-day
  access-token expiry, so a successful refresh read as broken. The output now
  surfaces the 90-day refresh-token window alongside the access-token expiry
  ([#1921](https://github.com/eddacraft/anvil-001/issues/1921)).

### Added

- **`anvil policy` command group (experimental).** New `list`, `explain`,
  `diff`, `validate`, and `test` subcommands for working with policy
  configuration, plus an experimental `anvil policy eval` that evaluates a Rego
  policy against an input document (POLENG). The Rego evaluation surface is a
  preview — its output shape may change before it stabilises.

## [0.7.1-beta] — 2026-05-22 — Activation Diagnostic Honesty (Boring Week Patch 1)

`v0.7.1-beta` is the first Boring-Week patch tag on the `v0.7.0-beta`
daemon-working slate. The headline change is GH
[#1831](https://github.com/eddacraft/anvil-001/issues/1831): two early Windows +
Scoop + PowerShell users hit `ready_restart_required` after `anvil start`
installed the MCP server, with no path to `protecting` even when the intercept
daemon was running and enforcing the worktree. `v0.7.1-beta` closes that loop —
`anvil start --verify` and `anvil status --verify` now consume the daemon's
`ProtectionClaim` snapshot and promote handshake-verified MCP clients to live
validation when the daemon attests the canonical worktree, with concrete repair
hints when something blocks promotion.

### Fixed

- **`anvil start --verify` reaches `protecting` when the intercept daemon
  attests the worktree.** Previously the diagnostic capped at
  `ready_restart_required` forever — even after the user restarted their editor
  — because the activation surface had no consumer for the daemon's
  `ProtectionClaim` snapshot. The new wire-up consumes
  `anvil_intercept::status::build_protection_claim_from_wire` and promotes
  handshake-verified MCP clients to `LiveValidation` when the worktree is in
  `PreWriteDaemon` (or `DegradedProtection` with at least one `Participating`
  surface). Closes GH
  [#1831](https://github.com/eddacraft/anvil-001/issues/1831).
- **Windows MCP `validate_write` response carries the `protection_claim`
  field.** Prior to this release the field was always `None` on Windows because
  the IPC client only spoke Unix sockets. The new named-pipe client provides
  parity with the Unix path so Windows + Scoop + PowerShell users see the same
  typed claim in MCP responses as Unix users.
- **`ready_restart_required` repair hint distinguishes daemon-state failures.**
  When the diagnostic stalls because the intercept daemon is unreachable,
  unenforced for the worktree, stale, or all-quarantined, the hint now directs
  the user at `anvil intercept start --foreground` (or `anvil intercept status`
  to inspect the registered worktree set) instead of always saying "restart your
  editor".
- **L4 engine distinguishes IO outages from missing engines.** A new
  `EngineUnavailableReason::IoError` variant separates a transient filesystem
  hiccup from a permanently absent engine, so the `engine-missing` operator hint
  no longer fires for retryable IO.
- **`anvil uninstall` detects Scoop and WinGet install paths.** The cleanup now
  matches the canonical Scoop and WinGet install directories and tightens the
  boundary check so removal cannot stray outside the install root.

### Added

- **Activation tracing surfaces operator-actionable failures at `warn` level.**
  A user running `anvil start --verify` and asking "why isn't this working?" now
  sees the missing piece (daemon unreachable, worktree unenforced, stale
  snapshot, all-surfaces quarantined) at the default `ANVIL_LOG=warn` filter,
  instead of needing to set `ANVIL_LOG=debug` to find it. Transient states
  (warming, no-participating-surface) stay at `info`; the genuine pre-restart
  case stays at `debug`.
- **`DaemonStatusV1.generated_at_unix` wire field.** A daemon-level wall-clock
  anchor, distinct from per-session heartbeats, used as a second consistency
  check on snapshot freshness. Wire-additive via `#[serde(default)]`: a
  pre-`v0.7.1-beta` daemon talking to a post-`v0.7.1-beta` consumer deserialises
  with the field at `0`, which the consumer treats as "no anchor available —
  fall back to per-session freshness only". No driver-side change required.
- **`anvil-run` manpage documents the SIGTERM transient-fence behaviour.** A
  launcher killed by SIGTERM may briefly cause the daemon to fence the worktree;
  the next launcher invocation clears the fence as part of session registration.
  The new DIAGNOSTICS section names the symptom and the recovery.
- **`docgov` validates as-built source paths in DOCGOV closeouts.** A closeout
  that names a non-existent source file now fails the governance check at PR
  time instead of silently shipping a broken cross-reference.

### Changed

- **`activation::diagnostic::ActivationDiagnostic` carries a
  `daemon_attestation` field.** Renderers read this to distinguish pre-restart
  from daemon-down / unenforced when generating the `ready_restart_required`
  repair hint. Wire-additive via `#[serde(default)]`.
- **Unix `query_daemon_status_at` enforces a single wall-clock deadline.**
  Previously `set_read_timeout` capped each individual read syscall; a daemon
  writing one byte every (timeout − 1 ms) could keep the read loop alive for
  ~524 s before bail. The new implementation refreshes
  `set_read_timeout(deadline − now)` against a single `Instant`-based deadline
  so the activation 500 ms IPC budget is enforced end-to-end. Brings Unix parity
  with the Windows single-deadline path.
- **Activation freshness check bounds future-timestamp tolerance.** A new
  `MAX_FUTURE_CLOCK_SKEW = 90 s` upper bound on future timestamps bounds the
  downgrade-attack path to 90 seconds of clock skew — a daemon stamping
  `u64::MAX` (broken RTC, snapshot replay, malicious snapshot output) is
  rejected by the freshness gate instead of permanently passing it. NTP step
  adjustments and VM-clock drift between the daemon and the workstation remain
  tolerated. Workstations whose system clock is itself attacker-controlled
  remain outside Anvil's threat model.

### Security

- **Windows IPC trust on `v0.7.1-beta` relies on the named-pipe DACL set at pipe
  creation.** Client-side SID validation (defence-in-depth parity with the Unix
  `SO_PEERCRED` check) is tracked as MLP2-051j Draft and will land in a
  follow-up patch. Same-SID processes are inside the v1 trust boundary the same
  way same-UID processes are on Unix — operators should not infer full parity
  with the Unix hardening from the activation diagnostic's Windows wire-up.

### Known gaps (carried from v0.7.0-beta)

- **Daemon-side `session.report_process` IPC handler unimplemented**
  ([#1827](https://github.com/eddacraft/anvil-001/issues/1827), MLP2-074) —
  launcher absorbs gracefully; ships as a known gap pending the IPC handler
  implementation.
- **`anvil intercept restart` and `anvil intercept recover` subcommands do not
  exist.** The `ready_restart_required` repair hints in this release route
  through `anvil intercept start --foreground` (cross-platform restart) and
  `anvil intercept unblock --worktree <PATH>` (Linux only; Windows bails with
  "not yet supported"). MLP2-028 will add Windows peer-credential support so
  `unblock` works on both platforms.
- **`anvil intercept unblock --worktree` is not supported on Windows yet.**
  Windows users hitting `DegradedProtection` with every surface quarantined must
  stop the daemon (close its terminal, or end the process via Task Manager /
  `kill`) and start it again — the repair hint reflects this.
- **`anvil intercept stop` subcommand does not exist.** To stop the daemon on
  Windows, close the terminal that's running it or end the process via Task
  Manager (or `kill <PID>` from another shell). On Unix, `Ctrl-C` the foreground
  terminal or `kill <PID>`. The missing CLI surface is tracked alongside the
  existing `intercept restart` / `intercept recover` subcommand gaps.
- **MCP `query_protection_claim` path still uses a 2 s IPC timeout on both Unix
  and Windows.** The activation surface enforces the intended 500 ms budget;
  MLP2-051i tightens the MCP path to match.

### Distribution

- **Scoop / Homebrew / WinGet:** `anvil update` (or `scoop update anvil` /
  `brew upgrade anvil`) pulls the new binary. Signature verification path
  unchanged from `v0.7.0-beta` (DISTRIB-001).
- **GitHub Release:** binaries published with the same matrix as `v0.7.0-beta`
  (Linux x86_64 / ARM64, macOS x86_64 / ARM64, Windows x86_64).

## [0.7.0-beta] — 2026-05-21 — Daemon-Working End-to-End Protection

### Added

- **End-to-end daemon-backed protection.** Hooks, the witness chain, baseline
  adoption, L4 policy, and wrapped agent launch now operate as a single
  verifiable claim: every commit is witnessed, every save passes the same
  protection pipeline, and every agent-driven write is attributable to a
  registered session. `anvil doctor`, `anvil status`, the MCP server, and the
  TypeScript driver-client all emit the same typed protection-claim shape so
  editors, CI, and agents read identical state.
- **Wrapped agent launch via `anvil-run`.** A new
  `anvil-run --tool <name> -- <command...>` launcher wraps AI-agent processes
  (Claude Code, Codex, and similar) so the daemon can attribute work, enforce
  fences, and clean up stale sessions. Includes daemon connectivity preflight,
  session registration with daemon-minted agent tags, process-group ownership so
  the daemon can target the right process tree, clean exit cleanup,
  shell-integration functions for zsh and bash, a fallback registration path via
  the pre-commit hook for sessions that cannot be launched through the wrapper,
  blocked-launch UX with actionable error output, and periodic heartbeats so the
  daemon notices when a launcher crashes.
- **`anvil doctor` typed protection-claim section.** `anvil doctor` now prints
  the worktree state and per-surface entries, with `--json` emitting the same
  `ProtectionClaim` shape as `anvil status --json`.
- **MCP server `validate_write` response carries `protection_claim`.** The field
  is optional; omitted when the daemon is unreachable. Pre-existing drivers
  round-trip the response unchanged.
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
  Sort order, table headers, and exit codes match the previous surface so
  scripts and editor integrations carry forward unchanged.
- **`anvil insights` weekly summary.** New CLI surface and `anvil.insights.v1`
  JSON schema for editor and CI consumers, derived from the witness chain with
  no separate event store. This release populates `witness_events_observed`;
  `total_saves_observed`, `findings_raised`, `suppressions_applied`,
  `suppressions_resolved`, `baseline_edges_added`, and
  `daemon_uptime_percentage` ship as schema-locked placeholders (`0`) pending
  the downstream metric wiring tracked in `INSIGHTS` follow-ups.
- **`anvil version --check` and security-advisory surface.**
  `anvil version --check` reports newer releases and security advisories against
  the running version. The watch TUI and `anvil status` show a one-line "update
  available" hint, rate-limited to once per 24 hours.
- **`anvil start --new-identity` and `anvil baseline --new-identity`.** Mints a
  fresh `project_uuid` and records the previous one as `forked_from`, giving
  forks an explicit opt-out from inheriting their parent repo's identity.
- **`anvil start --format json|toml`.** Choose `.anvil.json` or `.anvil.toml` at
  adoption time. The default remains yaml, and all three formats round-trip
  through the same canonical representation.
- **Hook coexistence with lefthook, husky, and pre-commit-framework.** Anvil
  hooks now install alongside the three dominant 2026 hook managers without
  conflict — registering as managed entries in the host manager's config rather
  than overwriting `.git/hooks/`. Uninstall removes only Anvil's own entries.
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
- **Release cadence and EOL policy.** `docs/policies/release-cadence.md`
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
  versions without a manual tap refresh. The previous lag between a tag landing
  and Homebrew users seeing the upgrade is closed.
- **MCP `anvil_validate_write` accepts patch-only payloads.** The pre-write
  validator now accepts a unified-diff `patch` instead of the full proposed
  `content` for change-shaped edits. Token cost scales with the size of the
  change rather than the file, which removes the read-budget ceiling on edits to
  large files (the original 2026-05-18 friction report cited a 2770-line JSON
  file). The `content` mode remains supported; clients pick whichever fits their
  workflow.
- **MCP `anvil_validate_write` returns a recoverable workspace-root signal.**
  When the validator refuses on an untrusted workspace root it now returns an
  `expectedWorkspaceRoot` field on the rejection so callers can self-correct and
  retry without an operator round-trip. Pre-existing clients that ignore the
  field continue to receive the same refusal shape.
- **Action commands now surface `state: "authRequired"` when the session is
  absent or revoked.** `anvil status`, `anvil start`, and the licence-gated
  commands listed in `crates/anvil-cli/src/feature_flags.rs:38-58` no longer
  emit a generic error when the operator's authentication is missing or has been
  revoked (for example after refresh-token reuse detection). Each command now
  exits with code 0 and a structured
  `{"state":"authRequired","next":"anvil auth login"}` payload so scripts and
  editors can route the operator to the recovery step. Operators upgrading from
  `v0.6.x` on a machine with a revoked session must run `anvil auth login` once
  on first invocation. See the
  [`v0.6.x → v0.7.0-beta` migration runbook](docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md)
  for the recovery flow. Closes PR
  [#1822](https://github.com/eddacraft/anvil-001/pull/1822) /
  [#1824](https://github.com/eddacraft/anvil-001/pull/1824).

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
- **Lineage-anchor mint hardened at the daemon IPC boundary.** A new
  `verify_lineage_claim()` in `crates/anvil-intercept/src/ipc.rs` enforces
  `peer_pid == claim.pid` against the `SO_PEERCRED` peer-credential and
  overrides any client-supplied `pid_starttime` with the daemon's own
  `/proc/<pid>/stat` read before the value reaches
  `SessionRegistry::register_with_lineage`. Four regression tests pin the
  contract. This closes the lineage-mint defect originally documented in
  [`docs/runbooks/v0.7.0-beta-security-note.md`](docs/runbooks/v0.7.0-beta-security-note.md)
  §M1 — the registry still accepts the daemon-re-derived values, but the trust
  shift now happens at the IPC boundary rather than inside the registry. Closes
  [#1674](https://github.com/eddacraft/anvil-001/issues/1674) and MLP2-070.

### Known gaps

These are shipped behaviours an operator should know about before adopting
`v0.7.0-beta`. Each one has a tracked follow-up; none change the v1 same-UID
local-IPC trust boundary documented in
[`docs/archive/runbooks/v0.6.0-beta-security-note.md`](docs/archive/runbooks/v0.6.0-beta-security-note.md).

- **`telemetry.allow_cross_session` cross-session redaction reaches Fanout but
  not yet operators.** Post-MLP2-071 Phase 1 the daemon now constructs the
  cross-session fanout at startup with the operator-configured policy and a
  fresh per-startup HMAC salt (closes `v0.6.0-beta-security-note.md` §H2 in the
  same change). The `RegistryOwnershipResolver` is wired against the live
  session registry, and a session's subscriber binding can be set via
  `SessionRegistry::bind_subscriber`. **Still missing for operator-visible
  behaviour:** the IPC `telemetry.subscribe` surface that lets a driver register
  as a subscriber, and the production `NotificationEnvelope` producer site that
  calls `Fanout::route` on every emit. Neither shipped in this tag because no
  in-tree producer broadcasts notification envelopes to remote subscribers today
  (see `crates/anvil-intercept/src/fanout.rs:79-82`). The safe default (`false`)
  keeps the redaction filter on the cold path regardless. Tracked in
  [#1722](https://github.com/eddacraft/anvil-001/issues/1722) + MLP2-071 (Phase
  1 shipped; Phase 2 — subscriber surface + producer broadcast — opens alongside
  the production notification telemetry stream feature).
- **`anvil-run` reports child process metadata to the daemon, but the daemon has
  no `session.report_process` handler.** The launcher at
  `crates/anvil-run/src/spawn.rs:102-128` invokes the daemon JSON-RPC method
  after launching the child to forward `(pid, pid_starttime)` so the daemon's
  MLP-014 PID-reuse defence can pin its lineage anchor to the agent process. The
  daemon dispatch table at `crates/anvil-intercept/src/ipc.rs:2431` currently
  has no handler for this method, returning `-32601 Method not found`.
  `anvil-run` absorbs the error and proceeds — exit code, fence behaviour, and
  signal handling are unaffected — but the cross-check against out-of-lineage
  spoofs (hardened in code by the lineage fix above) covers the wrapping
  launcher's `pid_starttime` rather than the launched agent's. Operators see a
  one-line stderr warning on each launch until the handler ships. Tracked in
  [#1827](https://github.com/eddacraft/anvil-001/issues/1827) + MLP2-074.

### Upgrade

- Homebrew: `brew upgrade eddacraft/tap/anvil`.
- curl installer: rerun the installer at <https://anvil.dev/install>.
- WinGet: `winget upgrade --id eddacraft.anvil`.
- Scoop: `scoop update anvil`.
- Direct download: pick up the new release from
  [the v0.7.0-beta release page](https://github.com/eddacraft/anvil/releases/tag/v0.7.0-beta).
- Migration questions: see the
  [v0.6.x → v0.7.0-beta migration note](docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md).
  v0.6.x users with revoked sessions will hit `state: "authRequired"` on first
  invocation — see the migration note's "Action commands require an
  authenticated session" section for the one-step recovery.

## [0.6.3-beta] — 2026-05-15 — Beta Watch UX + Uninstall Hotfix

Patch release for beta-user first-run and watch friction. No new APIs and no
breaking changes; the upgrade is drop-in for existing installs.

### Fixed

- **Homebrew-aware curl installer.** `install.sh` now detects an existing
  Homebrew-managed `anvil` binary (under `/opt/homebrew/bin/anvil`,
  `/usr/local/bin/anvil`, or a `Cellar/anvil/.../bin/anvil` symlink) before
  download. When found, it exits successfully and prints
  `brew upgrade eddacraft/tap/anvil` instead of overwriting the Homebrew-managed
  binary.
- **Watch and audit ignore local agent/tool worktrees and caches by default.** A
  shared ignore list covers `.claude`, `.opencode`, `.gemini`, `.serena`,
  `.worktrees`, and the usual generated/cache/build directories (`node_modules`,
  `target`, `dist`, and others). Audit, watch, and the kernel file watcher all
  consume the same policy.
- **Initial watch scan is baseline/readiness state, not new violations.**
  Existing public exports, dependencies, and cross-layer imports are no longer
  reported as save-time findings when `anvil watch` starts; only later file
  changes that introduce or re-surface an issue trigger findings.
- **`anvil watch` shows immediate startup feedback.** A terse "starting" line
  prints before the slow setup phase, so large repos no longer look hung on
  launch. `anvil watch` also falls back to plain output when stdin or stdout is
  not a terminal, instead of attempting to open the TUI.

### Added

- **`anvil uninstall` command.** Project-scoped removal of Anvil state
  (`.anvil/`, `.anvilrc`, and Anvil-managed git hooks). Pass `--global` to also
  remove user-level state (`~/.anvil/`), Anvil MCP entries from `~/.claude.json`
  and `~/.cursor/mcp.json`, stored credentials, and the running daemon. The
  Anvil binary itself is never removed — uninstall that with Homebrew, WinGet,
  Scoop, Cargo, or the installer path after cleaning state. Auth-bypass is built
  in so stuck installs can be cleaned without logging in.
- **Refreshed beta and watch help.** `docs/public/anvil/beta-testing-guide.md`,
  troubleshooting, and quickstart now cover the watch baseline-scan semantics,
  the shared ignore policy, non-TTY fallback, and the new uninstall escape
  hatch.

### Upgrade

- Homebrew: `brew upgrade eddacraft/tap/anvil`.
- curl installer: rerun the installer — it now detects Homebrew and steps aside
  with the correct upgrade hint.
- WinGet / Scoop / direct download: pick up the new release as normal; no
  manifest or installer shape change.

## [0.6.2-beta] — `anvil update` Windows polish + device-code rate-limit

### Added

- **Local trace correlation for daemon and CLI debugging.** Set
  `ANVIL_TRACE_SINK=file=<path>` to write JSON-line tracing output to a
  user-private local file. On Unix, newly created files are opened with `0600`
  permissions; existing sinks are rejected if they are symlinks or are
  readable/writable by group or other users. The daemon records the incoming
  `trace_id`, `parent_id`, and `trace_flags` as correlation fields on the
  dispatch span and echoes the original `traceparent` header on the response.
  Local correlation only — not full OpenTelemetry parent propagation. Disabled
  by default. See `docs/observability/local-tracing.md`.

### Security

- **Per-code brute-force counter on `/device/confirm`.** The OAuth device-code
  confirmation endpoint now tracks a per-code attempts counter with an atomic
  upper bound, preventing brute-force exhaustion of valid device codes during
  the confirmation window. Closes a race in the counter's initial implementation
  in the same release.

### Fixed

- **`anvil update` on Windows no longer crashes with a file-lock error.** The
  cargo-dist updater sidecar is not shipped in this release
  (`install-updater = false` so aarch64-pc-windows-msvc can stay in the release
  matrix), so the in-process axoupdater path was attempting to overwrite the
  running `anvil.exe` and failing with
  `The process cannot access the file ... because it is being used by another process`.
  The command now refuses cleanly on Windows and points to
  `winget upgrade --id eddacraft.anvil` or re-running the PowerShell installer,
  with a note about closing editors running an Anvil MCP server. `--check` still
  works.
- **`anvil update` now detects WinGet and Scoop installs** and prints the one
  command that will actually upgrade you (`winget upgrade --id eddacraft.anvil`
  or `scoop update anvil`), mirroring the existing Homebrew dispatch. Previously
  WinGet/Scoop users fell through to the generic Windows refusal, which listed
  extra alternatives they did not need.
- **`anvil check` no-args error now suggests next steps.** A bare `anvil check`
  previously bailed with
  `No files specified. Use --all, --changed, or provide file paths.` — terse for
  first-time users. The message now lists `--changed`, `--all`, and explicit
  paths, plus pointers to `anvil welcome` and `anvil status`.

## [0.6.1-beta] — `anvil start` UX + Auth Refresh Polish

### Fixed

- **`anvil start` MCP picker no longer reprints the question on every arrow
  press.** Long option labels (which embedded full drift paths) wrapped on
  normal terminals, confusing `demand`'s line-count-based redraw. Labels are now
  tilde-shortened with a one- or two-word state tag so they fit in a single row;
  the from→to drift detail is preserved in the post-install render block.
  ([#1366](https://github.com/eddacraft/anvil-001/pull/1366))
- **`anvil start` home-tilde path display is now component-aware.** A home
  directory like `/home/al` no longer matches a path under `/home/alice/...` and
  render as `~ice/...`. Switched from `String::strip_prefix` to
  `Path::strip_prefix`.
  ([#1366](https://github.com/eddacraft/anvil-001/pull/1366))
- **`Log in now?` prompt no longer hangs when a previous TUI leaked raw mode.**
  `prompt_yes_no` defensively calls `crossterm::terminal::disable_raw_mode()`
  before reading; the MCP picker is wrapped in a `RawModeGuard` so a panic /
  SIGINT / unwind in the picker cannot leak the raw flag in the first place.
  ([#1371](https://github.com/eddacraft/anvil-001/pull/1371))
- **Silent licence refresh.** When the 7-day JWT lapses but the 90-day refresh
  token is still valid, `anvil` exchanges it inline before falling through to
  the `Log in now?` prompt. Eliminates a forced device-code re-login every week.
- **Vercel deploy infra:** `domainImports` gated on prod stack with input
  validation; `delete-before-replace` env-var ordering for the
  `www.eddacraft.ai` cutover.
- **Watch cancellation test deflaked.** Extracted `WaitOutcome::to_send_args` as
  a pure helper and asserted the mapping directly, removing the 30 s polling
  barrier and the `serial-watch-cancellation` nextest pin.
  ([#1379](https://github.com/eddacraft/anvil-001/pull/1379))

### Added

- **`anvil auth refresh` subcommand.** Exchanges the stored refresh token for a
  fresh licence without re-running the device flow. Supports `--json`; bypasses
  the licence-gate pre-check by design.
- **Cause-specific auth errors.** `/session/refresh` distinguishes expired /
  revoked / theft / inactive responses; the CLI surfaces an actionable message
  for each instead of a generic 401.

### Security

- Bump `@babel/plugin-transform-modules-systemjs` to `>=7.29.4` via pnpm
  override (CVE-2026-44728, HIGH — arbitrary code generation when compiling
  malicious input). Pulled in transitively via `@babel/preset-env` →
  `@babel/core`.

## [0.6.0-beta] — Wow-Start Activation & Daemon-Backed Mid-Edit Validation

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
  / `unblock` CLI subcommands are documented in
  `docs/archive/runbooks/v0.6.0-beta-release-runbook.md`.
- **macOS interrupt path is fence-first this release** — on macOS the interrupt
  ladder falls through to fence-on-uncertainty rather than running the full
  SIGINT → SIGTERM → SIGKILL sequence. Recovery procedure is documented in
  `docs/archive/runbooks/v0.6.0-beta-release-runbook.md`.
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
- **Operator artefacts** — the release ships
  `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` (five operator items)
  and `docs/archive/runbooks/v0.6.0-beta-security-note.md` (four HIGH security
  trade-offs documented for review).

## [0.5.1-beta] — Scanner Signal & TUI Hotfixes

### Changed

- **TypeScript package subpaths** — archived scanner-era subpath exports were
  removed from `@eddacraft/anvil-core` and `@eddacraft/anvil-runtime`; use the
  Rust CLI surfaces for antipattern, suppression, drift, gate, and export flows.

### Added

- **TUI zoom controls** — audit, status, and watch surfaces now support zooming
  to inspect dense output more comfortably.

### Fixed

- **Secret scanner false positives** — generic secret matching now requires a
  stronger right-hand-side shape, credit-card detection rejects UUID fragments,
  and entropy matching focuses on secret-shaped quoted values.
- **Antipattern suppressions** — `AP-*` checks now honour local `eslint-disable`
  directives, and `GS-001` avoids reporting guarded `Map.get` after `has`/`set`
  flows.
- **Audit noise** — audit scans now skip broader environment-template files
  while still reporting real `.env` files regardless of directory.
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

## [0.5.0-beta] — AI Guardrails & Mid-Edit Validation

### Added

- **Git config hook mode** — `anvil hooks install --config` and
  `anvil hooks uninstall --config` can now manage Git 2.54 native
  `hook.<event>.command` entries; file-mode hooks remain the default
- **Hook setup visibility** — `anvil hooks status`, `anvil doctor`, onboarding,
  and tutorial surfaces now recognise config-mode hooks, `core.hooksPath`,
  third-party hook managers, and duplicate file/config execution risk
- **AI guardrail profile** — `anvil gate --profile ai` runs the AI-focused check
  set with strict configuration handling and a stable JSON envelope for agent
  and MCP consumers
- **AI reasoning rule** — `AI-001` flags source comments that justify code with
  authority, social proof, or deflection instead of technical reasoning
- **`.env` secret scanning** — `.env`, `.env.*`, and `.envrc` files are parsed
  as key/value files so leaked values report the variable name and source line
- **MCP config generator** — `anvil mcp-config` generates, verifies, and writes
  Claude Code, Cursor, Windsurf, and VS Code MCP server configuration
- **API migration runner** — Anvil API deploys now have a first-party SQL
  migration runner with dry-run support and drift detection

### Fixed

- **Doctor outside git repos** — missing git repositories now warn through the
  structured doctor JSON contract instead of failing the whole run
- **First-run guidance** — init and onboarding copy now points unauthenticated
  users at `anvil auth login` where required, and inotify capacity warnings are
  clearer about what to change
- **Release publishers** — Scoop and WinGet publishing paths now fail earlier on
  token/fork problems, and the cargo-dist installer is pinned by SHA256 in the
  release workflow
- **API deploy stability** — CORS preflight caching, Vercel API routing, and the
  `svix`/`uuid` runtime override were tightened after post-release deploy
  failures

### Improved

- **Scan performance and safety** — repository scans now use the shared parallel
  walk pattern across more CLI surfaces, skip oversized lines before regex work,
  and cap first-run scan threads by default
- **AI workflow docs** — the AI guardrail profile, MCP/editor setup path, and
  beta tester guide now describe the current Rust CLI behaviour
- **Git hook docs** — public docs now explain file-mode versus config-mode
  hooks, coexistence warnings, and the current decision to keep Husky as the
  contributor bootstrap
- **Beta validation scenarios** — public tester scenarios were refreshed around
  the current onboarding, hooks, AI guardrail, and MCP flows

### Developer

- Canonical `anvil.diagnostic.v1` diagnostics now back the AI guardrail profile
  and the real-time AI validation work
- The real-time AI validation spike measured mid-edit secret detection at about
  1.4 ms p95, comfortably inside the draft latency budget
- Portable acknowledgement starter templates, release token runbooks, and APS
  release-follow-up tracking were added ahead of this release

## [0.4.0-beta] — First-Run Polish & Native Scanner

### Changed (breaking)

- **`anvil watch --exclude` now uses glob patterns** — pass
  `--exclude 'vendor/**'` to skip a directory tree; bare directory names now
  warn so existing scripts surface the change instead of silently watching the
  wrong paths
- **JSON output now carries notifications** — `anvil doctor --json` now returns
  `{ "checks": [...], "notifications": [...], "schema_version": "2.0.0" }`, and
  `check`, `gate`, and `audit` include `notifications[]` alongside their
  existing payloads

### Added

- **Native Rust scanner** — the Rust engine is now the authoritative scanner,
  with registry-backed rules, parallel scanning, rule provenance on findings,
  and fixture coverage for every shipped rule
- **First-run scan after `anvil init`** — new projects get immediate findings,
  counts, and `file:line` pointers instead of being sent to another command
- **Watch filtering** — `anvil watch --patterns` and `--exclude` now drive the
  watch loop, with a startup banner showing the active include/exclude scope
- **`anvil check --artifact`** — scan generated files, build outputs, and other
  opaque artefacts outside the normal source-file filter
- **`anvil licenses`** — prints bundled third-party attributions from
  `ACKNOWLEDGEMENTS.md`
- **Scoop distribution** — Scoop joins WinGet and the existing installers, with
  README install instructions covering every supported package manager
- **Per-operator admin keys** — admin operations can now use individually
  provisioned operator credentials instead of a single shared admin key

### Fixed

- **`anvil watch` reliability** — partial setup failures no longer abort the
  loop, per-change panics are isolated, Ctrl-C exits cleanly, and error chains
  no longer leak the current working directory
- **`anvil doctor` remediation** — checks now show concrete next actions, and
  `--fix` writes a valid default `.anvilrc` without running `git init` in unsafe
  directories
- **`anvil init` robustness** — post-init git-history sampling now times out
  instead of hanging on slow filesystems or stalled remotes; tight inotify
  limits are reported up front with a fix hint
- **Config-driven gate checks** — `.anvilrc` check selection now uses the same
  canonical names as the gate runner (closes #1016, #1041)
- **Non-interactive mode** — empty `ANVIL_NO_PROMPT` and `NONINTERACTIVE` values
  now correctly opt out of prompts
- **Admin CLI and auth flows** — route coverage, timestamp validation, JSON
  output hygiene, EOF handling, and migration-send safety were tightened across
  the beta access surfaces
- **Tutorial and TUI papercuts** — tutorial exit codes, `husky` handling, ASCII
  fallback, narrow-terminal titles, discovery scrolling, and `.anvilrc`
  detection races were corrected

### Improved

- **Onboarding language** — init, welcome, tutorial, and watch now use the same
  defaults and explain scan truncation or watcher failures where the user can
  act on them
- **Public docs** — release pages, install docs, the quality model, and the
  `.anvil` pattern reference were refreshed for the native scanner release
- **Release preflight** — `scripts/release.sh` now runs Rust and TypeScript fmt,
  lint, typecheck, and tests as one bundled release gate

### Developer

- Rust workspace hardening added `cargo-hakari`, `cargo-deny`, `cargo-about`,
  `cargo-nextest`, rust-cache, and parallelised Rust CI jobs
- The shared notification envelope, feature-flag resolver migration, in-house
  nx-rust plugin, and napi-rs prebuild bridge landed behind the CLI surfaces
- OPA policy tests, TypeScript `~6.0.3`, Husky `oxfmt` enforcement, APS archive
  cleanup, and release-agent workflow updates landed during the cycle

## [0.3.3-beta] — WinGet Distribution & Windows UX

### Added

- **WinGet distribution** — Windows users can now install and upgrade anvil via
  WinGet; the release workflow submits a manifest to the community repo on every
  tagged release
- **Authenticode signing infrastructure** — release pipeline wired for
  Authenticode signing of Windows binaries via Azure Trusted Signing and
  SSL.com; signing activates once identity provisioning clears
- **Branded post-install message** — installer prints a branded next-steps block
  with colour support, pointing new users at `anvil auth login` and
  `anvil welcome`
- **Admin waitlist + audit list endpoints** — read-only list endpoints exposed
  for the upcoming admin CLI (`ADMINCLI-001..004`)
- **Admin CLI operational commands** — `anvil admin` now includes `list`,
  `show`, `approve`, `invite`, `audit`, `revoke`, and `send-migration` so beta
  access operations can be handled from the CLI instead of ad-hoc API calls and
  dashboards
- **Nightly stress test workflow** — CI benchmark runner to catch performance
  regressions in the native engine early (`BENCH`)

### Fixed

- **Windows TUI input** — crossterm key events filtered to Press-only on
  Windows, eliminating duplicate keypresses in onboarding and discovery
- **Discovery layout** — two-panel layout restored with full scrolling and a
  reliable onboarding reset
- **Tutorial exit codes** — corrected exit codes, `husky` flag handling, and the
  verify-step sentinel so tutorials complete cleanly
- **Licence signing key probe** — anvil-api probes the ES256 signing key at boot
  and exposes the result in `/health`, surfacing missing-secret issues before
  requests hit the auth endpoints (`BAUTH`)
- **Admin approve reliability** — `/admin/approve` retries user_code constraint
  collisions and accepts longer codes so back-to-back approvals succeed
- **Admin email correction flow** — clearer email-mismatch UX in auth flows,
  plus an admin endpoint to correct beta-user email addresses without manual DB
  changes
- **Admin CLI robustness** — list flag validation, audit type alignment,
  `--json` warning handling, TTY detection, table sanitisation, and error-path
  handling tightened across the new admin surfaces
- **Migration send safety** — `send-migration` now honours `--no-dry-run`
  correctly, exits non-zero on delivery failures, and describes its audience
  scope more clearly
- **Auto-promote public release** — public GitHub Releases flip to Latest on
  every tagged production release
- **Dependency pin** — `follow-redirects` bumped to >=1.16.0 to close a known
  vulnerability

### Improved

- Public docs branding normalised (`DOCSYNC`)
- Structured error logging added to waitlist and auth routes
- Database consolidation guidance and admin runbooks expanded for operators

### Developer

- `scripts/release.sh` hardened across preflight, bundled tests, remote
  validation, and manifest handoff; `/release` skill updated in lockstep
- DBCON module landed for the Neon project consolidation; `WAITLIST_PAUSED` kill
  switch and waitlist-table bridge migration ship as part of that
  (operator-only, not exposed to CLI users)
- DBCON follow-on work now includes the option-B reset path and
  `ANVIL_API_DATABASE_URL` rename for the next database cutover stage
- ADR-024 published for the literate-core internal agent harness
- KERN and BENCH APS modules archived

## [0.3.2-beta] — Update Command & Onboarding Completion

### Added

- **`anvil update` subcommand** — self-update the CLI binary in-place with
  version check, download, and verification (`RCLI`)
- **`anvil admin invite` command** — invite beta users directly from the CLI
  with dual-mode flow (email + approval) and updated test coverage (`BAUTH`)
- **Welcome screen & onboarding complete** — all 18 WELCOME tasks finished;
  first-run onboarding experience fully wired (`WELCOME`)
  - Discovery mode, executable tutorial steps, live file watching demo
  - Fix step with dual-mode editing
  - Hook installer guidance
  - Gate and watch accessible from welcome menu
- **Interactive release script** — `scripts/release.sh` walks through preflight,
  branching, tagging, and workflow kickoff; writes `.release/manifest.json` as
  handoff contract for the `/release` Claude skill (`RMAN`)
- **Feature flag inventory** — ad-hoc flag inventory documented with governance
  guide (`FLAGS`)

### Fixed

- **API query ordering** — `ORDER BY` restored in `findActiveOtpCodes` to
  prevent non-deterministic OTP code selection
- **SQL centralisation** — inline SQL in API routes extracted to `db/queries.ts`
  for consistency and auditability
- **TUI tutorial commands** — tutorial paths synced with current CLI
  subcommands; audit surface scroll fixed for long result lists
- **Install script** — next-steps output always printed; Homebrew tap published
  automatically on release
- **CI stability** — Semgrep version pinned to prevent surprise breakage; OSSF
  Scorecard scoped to default branch only

### Improved

- Branding lowercased to `eddacraft` and `anvil` across the entire repo
- `aarch64-pc-windows-msvc` target added to cargo-dist (updater disabled pending
  upstream ARM64 Windows binary)
- Public docs aligned with branding changes (`DOCSYNC`)

### Developer

- ADR-020 (versioning strategy) published
- Decision log (`DECISION-LOG.md`) created as single source of truth for ADRs
- Completed APS modules archived; index reconciled
- APS rules tightened for agent conventions
- 59 unit tests for under-covered anvil-cli modules (`TCOV`)

## [0.3.1-beta] — Docs Cutover & Onboarding Fixes

### Added

- **Docs domain cutover** — `docs.eddacraft.ai` now served via a docs-shell
  proxy with shared-secret middleware protecting upstream apps (`DOCSAUTH2`)
- **Docs landing page** — Nordic terminal-themed hub at `docs.eddacraft.ai` with
  navigation to public and gated documentation sections

### Fixed

- **Welcome screen** — first-user onboarding flows restored after regressions in
  0.3.0-beta; council review findings and PR feedback addressed (`WELCOME`)
- **Docs auth** — CI build failures resolved for the domain cutover; upstream
  middleware and proxy hardened from review feedback; Docusaurus `baseUrl`
  deprecation warning suppressed
- **Auth error messages** — raw HTTP errors replaced with user-friendly messages
  in device-code and login flows
- **TUI version display** — shell footer now shows correct version string and
  ViewDocs handler fixed
- **Beta auth e2e** — e2e test harness for authentication flows fixed
- **Build scripts** — `%h` home-directory expansion replaced with absolute path
  to prevent misexpansion under `sudo`
- **Release pipeline** — `aarch64-pc-windows-msvc` removed from cargo-dist
  targets (upstream `axoupdater` lacks ARM64 Windows binaries)

### Improved

- Vercel auth on docs upstream projects replaced with header-based gating for
  simpler deployment
- `docsStateSecret` reused across Pulumi env vars instead of duplicating the
  secret reference
- Vercel preview deploys skip non-release branches via `vercel-ignore-build.sh`

## [0.3.0-beta] — Rust CLI & Native Engine

### Added

- **Rust CLI** — full native rewrite of the CLI in Rust using clap, replacing
  the Node.js/Commander.js implementation (`RCLI`, `RCLI2`)
  - 20 subcommands: check, watch, gate, gate-config, init, wizard, new, status,
    doctor, tutorial, welcome, audit, hooks, export, auth, admin, policy,
    architecture, validate, drift
  - `anvil policy evaluate` and `anvil architecture validate` wired to real OPA
    executor and config loader
  - `anvil auth login` — device-code authentication flow
  - `anvil admin approve` — approve beta access requests
  - `anvil new` — template browser for project scaffolding
  - `anvil wizard` — interactive setup with template scaffolding
  - `anvil audit` — repository scanning for security findings
  - `anvil drift` — architecture drift tracking (snapshot, compare, report,
    list)
  - `anvil validate` — APS plan file validation (structure, format, hashes)
  - `anvil gate-config` — gate check configuration and thresholds
  - `--json` output mode across all commands with structured error reporting
  - `--confidence` and `--since` filters on edda list
  - Node.js CLI archived to `archive/` — single binary, no runtime dependency
- **Beta authentication system** — passwordless device-code and OTP
  authentication for beta users (`BAUTH`)
  - Device code start, confirm, and poll endpoints
  - OTP request and verify endpoints
  - Session refresh with theft detection
  - Admin approve endpoint with invite email
  - Expired code cleanup cron job
  - Auth auto-refresh in CLI
  - Device code confirmation page on website
  - Beta invite and OTP code email templates
  - Resend audience management for waitlist
- **Docs auth gating** — `/anvil` docs gated behind GitHub OAuth via Vercel
  middleware (`DOCSAUTH`)
  - GitHub OAuth callback in BAUTH API
  - Vercel routing middleware with stateless ES256 JWT verification
  - Login, callback, and logout serverless functions
  - Pending approval and error pages for edge cases
  - Pulumi env vars for Key Vault secrets
- **Welcome screen & onboarding** — first-run detection with interactive
  onboarding experience (`WELCOME`)
  - First-run detection service anchored to workspace root
  - Onboarding welcome surface with discovery mode
  - Executable tutorial steps with live file watching demo
  - Fix step with dual-mode editing
  - Hook installer guidance
  - Gate and watch accessible from welcome menu
  - `ANVIL_DEV=1` bypass for local development testing
- **Ratatui TUI surfaces** — native terminal UI replacing Ink/React (`RATS`,
  `PORT`)
  - Welcome screen with brand block logo and watermark
  - Gate and watch accessible from welcome menu
  - Status dashboard, doctor diagnostics, audit results
  - Init wizard, template browser, gate explorer
  - Tutorial orchestrator with policy, architecture, drift, and CI paths
  - Watch dashboard with dirty-flag rendering to reduce flicker
  - Shell chrome with surface-specific help text and footer
  - Esc/back navigation from all surfaces
  - Loading frame during surface transitions
  - Render snapshot tests for all surfaces
- **Rust kernel** — native core engine with file watching, parsing, and graph
  analysis (`KERN`)
  - File watcher with debounce and backpressure
  - Tree-sitter parser with AST cache and symbol extraction
  - Petgraph symbol graph with module-level dependency tracking and cycle
    detection
  - Trust level annotation for graph nodes
  - Incremental graph updates with GraphDelta
  - Architecture config loading from YAML
  - Invariant evaluation framework — cross-layer, new dependency, public API,
    and privilege escalation checks
  - Event emitter with EngineEvent protocol
  - Foreground watch mode with event streaming
  - Embedded mode for one-shot checks
  - Dual-run harness for engine comparison
  - Engine mode flag for Rust/Legacy/Dual selection
  - Rayon parallel scanning for file walks
  - Architecture parity tests validating Rust engine against TypeScript baseline
- **Kernel benchmarks** — criterion micro-benchmarks and stress test harness for
  critical paths (`BENCH`)
  - Watcher saturation, graph memory, incremental throughput, policy scaling,
    and cold start scenarios
  - CI integration on main pushes and manual dispatch
- **@eddacraft/json-render** — JSON-driven dashboard rendering package for
  declarative UI specs with 3 dashboard spec templates
- **Rust engine checks** — native secret detection, anti-pattern detection, and
  command safety validation ported to Rust (`RENG`)
- **Distribution pipeline** — cross-platform binary releases via cargo-dist
  (`DIST`)
  - Binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows
    (x86_64, aarch64)
  - Shell and PowerShell installers served from `install.eddacraft.ai`
  - Homebrew tap (`brew install eddacraft/tap/anvil`)
  - Built-in self-updater (`anvil-update`)
  - Cross-repo release workflow publishing to `eddacraft/anvil`
- **Scan filter** — test fixture exclusion from check scans (`WELCOME-004`)
- **OPA v1 and Regal linting in CI** — Rego policies migrated to OPA v1 syntax,
  Regal linter added to Rust workflow (`TFIX-003`, `TFIX-004`)
- **Waitlist migration email** — bulk invite existing waitlist users with
  migration email template and personalised sign-off

### Improved

- Shared packages restructured per ADR-015 (flattened `packages/shared/` into
  `packages/platform/`)
- eddacraft-tui extracted to crates.io v0.1.0 for reuse across projects
  (`TUIEXTRACT`)
- Crate namespace renamed to `eddacraft-anvil-*` for crates.io publishing
- TUI welcome screen layout adapts to small terminals (24-row minimum)
- TUI position indicator only shown when Issues panel is focused
- TUI audit list viewport scrolling with inline expansion
- Watch mode excludes ignored directories from OS-level file watches
- Watch collections capped to prevent unbounded memory growth
- Pass rate calculation corrected (no double-counting)
- File walker prunes ignored directories during traversal
- Graph uses deterministic ordering for import resolution
- External trust level preserved correctly in trust annotation
- Architecture baseline output is deterministic (BTreeMap ordering)
- Workspace root computed once per gate run (performance)
- Vercel preview deploys skipped on non-main branches
- Branding updated to lowercase `anvil` and `eddacraft`
- Public docs aligned with Rust CLI: install commands, CI config,
  troubleshooting all updated for native binary (`DOCSYNC`)
- Node.js/npm references removed from all public documentation

### Fixed

- Terminal restore after subprocess failures now aborts hub loop cleanly
- Docs errors shown inline instead of flashing to console
- Empty `.anvilrc` rejected by doctor check
- Clap parse failures handled in JSON error path
- Windows path separators normalised in export tests
- Relative imports resolved correctly in kernel graph
- Side-effect module imports handled properly
- `PrivilegeExpansion` suppressed for already-privileged symbols
- Baseline policy evaluation runs before first watch snapshot
- Atomic file writes use secure permissions at creation, not after
- TOCTOU races removed in directory and file creation
- First-run marker anchored to workspace root (not CWD)
- Import edge line numbers propagated from parser
- Watch coverage file filter leak fixed (no more coverage/ artifacts)
- Watch event adapter double-counting and unbounded queue growth fixed
- Divergent `main`/`dev` branch histories reconciled (`BRECON`)

### Security

- Device-code and OTP authentication hardened with theft detection on session
  refresh
- Docs auth gating prevents unauthenticated access to `/anvil` documentation
- Licence signing guards against NaN TTL values
- API returns 500 on refresh signing errors instead of `valid:false`
- Log inputs sanitised to prevent log injection
- GitHub Action expression injection sanitised in anvil-check action
- All GitHub Actions pinned to commit SHAs
- CI release workflow hardened from council review
- Atomic credential file writes with restrictive permissions
- Dependency patches:
  - fast-xml-parser >= 5.5.6 (`CVE-2026-33036`)
  - @hono/node-server >= 1.19.13 (`CVE-2026-39406`)
  - axios >= 1.15.0
  - picomatch and smol-toml overrides for CVE fixes
  - undici and yauzl security patches
  - flatted and socket.io-parser overrides
  - rustls-webpki bumped to 0.103.10

### Developer

- TypeScript upgraded to 6.0 across all workspace packages (`MAINT-011`)
- Node engine floor raised to >= 22
- Rust toolchain bumped to 1.94.0 with Windows and macOS cross-compilation
- oxlint adopted as first-pass linter, oxfmt replaces prettier
- Criterion benchmarks added for kernel critical paths and wired into CI
- Stress test harness for kernel benchmarking (`BENCH`)
- Test coverage added for watch, doctor, export, auth device flow, status, and
  audit commands (`TCOV`)
- 59 unit tests for under-covered anvil-cli modules
- Integration test suite for checks crate
- GitHub Actions bumped: checkout v6, setup-node v6, download-artifact v8,
  nx-set-shas v5, labeler v6, azure/login v3, pnpm/action-setup v5
- Unused CI jobs removed (Playwright, e2e-harness, tui-tests)
- Benchmarks restricted to main pushes and manual dispatch
- CodeQL workflow added with paths-ignore
- Docusaurus upgraded to 3.10
- Dependency bumps: criterion 0.8, reqwest 0.13, dirs 6, Vite 8
- ADR-015 (shared packages restructure) and ADR-016 (unified config format)
  published

## [0.2.1-beta] — Project Memory & Pattern Detection

See [Engineering History](./ENGINEERING-HISTORY.md) for full technical details.
Edda/Ember/Stack CLI commands shipped in the Node.js CLI; Rust CLI ports are
deferred to a future release (RCLI3).

## [0.1.3]

Hardening, reliability, and quality-of-life improvements across the CLI.
Prepares the foundation for the Rust core engine (see ADR-011).

### Added

- BMAD v6 YAML document type support in format adapter
- `--json` output flag for `anvil hooks status` and `anvil plan create`
- Tutorial continuation — continue to another learning path after completing one
- APS nested index loading with configurable depth limiting (`CRB-011`)
- APS atomic task locking — prevents race conditions in multi-agent workflows
  (`CRB-010`)

### Improved

- CLI error messages now surface network errors clearly instead of raw "fetch
  failed"
- Dependency audit errors are surfaced instead of silently reported as clean
- Watch mode signal handler is more reliable under rapid restarts
- Tutorial completion persists when switching topics via the picker
- `--reset core` preserves non-core tutorial progress
- Windows compatibility across path handling, permissions, and signal handling

### Fixed

- Exit code consistency across CLI commands (`ISS-001`, `ISS-002`, `ISS-005`,
  `ISS-007`)
- Missing Ember database now returns a clean exit instead of an unhandled error
- Memory store rejects invalid sort parameters instead of producing unexpected
  results
- Comma-separated `--type` values now parsed correctly in list commands

### Security

- Input validation hardening across parsers, adapters, and plan loader
- Subprocess execution hardened across the codebase
- Dependency patches:
  - minimatch >= 9.0.7 (`CVE-2026-27904`)
  - axios >= 1.13.5 (`CVE-2026-25639`)
  - svgo (billion laughs DoS)
  - tar >= 7.5.10
  - serialize-javascript, ajv, undici

### Developer

- CLI stderr/stdout stream policy standardised — all diagnostic output routes to
  stderr, structured data to stdout
- Git hook scripts consolidated to a single source of truth
- Default API URL changed to `eddacraft-api.vercel.app`
- ADR-011: Rust core engine architecture decision published

## [0.1.2-beta] - 2026-02-22

### Fixed

- CLI error handling improvements
- Watch mode signal handler reliability

## [0.1.1] - 2026-02-21

Patch release focused on npm publish/install reliability for
`@eddacraft/anvil-cli`.

### Fixed

- Published CLI metadata no longer exposes `workspace:*` runtime dependencies to
  npm consumers
- Release workflow publishes CLI-required workspace packages and skips already
  published versions

## [0.1.0] - 2026-02-21

Initial pre-release of Anvil — the deterministic development automation platform
that makes AI-generated code safe to merge by catching architecture boundary
violations and anti-patterns at save time.

### Added

- `anvil check <files>` — analyse files for architecture violations and
  anti-patterns
- `anvil check --changed` — git-aware file detection for staged/unstaged changes
- `anvil check --staged` — check only staged files
- `anvil check --since <ref>` — check files changed since a git reference
- `anvil watch --source` — real-time feedback on file changes with sub-2-second
  latency
- 7 high-confidence AI escape-hatch anti-pattern detectors
- Pattern suppression with time-boxing and mandatory explanations
- Architecture boundary detection with automatic baseline inference
- Architecture templates: Layered, Hexagonal, Clean, DDD, Monorepo, Serverless,
  Nx Workspace, Starter
- `anvil architecture visualise` — Mermaid-based dependency graph rendering
- Interactive architecture wizard with live diagram previews
- `anvil drift snapshot` — capture current architecture state
- `anvil drift compare` — show changes between snapshots
- `anvil drift report` — visualise trends over time
- `anvil explain <id>` — deep-dive into warnings with context
- OPA/Rego policy framework with remote bundles, checksum verification, and
  authentication
- `anvil gate` — run quality gates on the codebase
- `anvil init` — visual TUI wizard for project setup
- `anvil status` — quick health check dashboard
- `anvil doctor` — diagnose setup issues
- `anvil tutorial` — interactive scan-watch-fix tutorial
- GitHub Action for PR checks with comment annotations
- VS Code extension with anti-pattern detection, architecture gate display, OPA
  policy violations, and click-to-navigate
- MCP tool server for real-time validation
- llms.txt export for AI tool consumption
- Command safety validation for AI tool commands
- HTML/CSS anti-pattern detection
- APS Markdown adapter for `.aps.md` planning documents
- `anvil plan load`, `anvil plan validate`, `anvil plan status` — APS planning
  document management

### Security

- 17 findings resolved across MCP server, runtime, CLI, adapters, storage, APS,
  and VS Code extension (3 critical, 10 high, 4 medium)
- External binary integrity verification
- Credential storage hardened with restrictive permissions
- API response validation strengthened throughout

[0.6.3-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.6.2-beta...v0.6.3-beta
[0.6.2-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.6.1-beta...v0.6.2-beta
[0.6.1-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.6.0-beta...v0.6.1-beta
[0.6.0-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.5.1-beta...v0.6.0-beta
[0.3.2-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.3.1-beta...v0.3.2-beta
[0.3.1-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.3.0-beta...v0.3.1-beta
[0.3.0-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.2.1-beta...v0.3.0-beta
[0.2.1-beta]:
  https://github.com/eddacraft/anvil-001/compare/v0.1.3...v0.2.1-beta
[0.1.3]: https://github.com/eddacraft/anvil-001/compare/v0.1.2-beta...v0.1.3
[0.1.2-beta]: https://github.com/eddacraft/anvil-001/releases/tag/v0.1.2-beta
[0.1.1]: https://github.com/eddacraft/anvil-001/releases/tag/v0.1.1
[0.1.0]: https://github.com/eddacraft/anvil-001/releases/tag/v0.1.0

## v0.7.3-beta

- Release preparation metadata generated.

## v0.7.4-beta

- Release preparation metadata generated.
