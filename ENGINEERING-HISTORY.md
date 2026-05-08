# Engineering History

Technical release history for engineers, platform teams, and technical
evaluators.

This log covers architecture, infrastructure, reliability, security, and
delivery changes behind each release. For end-user feature summaries, see the
[Changelog](./CHANGELOG.md).

## [Unreleased]

## [0.6.0-beta]

### Daemon-Backed Mid-Edit Validation (INTD)

- **Owner-only IPC** — daemon listener accepts connections only from the owning
  UID via `SO_PEERCRED` on Linux, `getpeereid(2)` on macOS, and a per-user DACL
  with `reject_remote_clients(true)` on the Windows named-pipe listener.
  `crates/anvil-intercept-win32` ships the Windows daemon side; the synchronous
  Win32 client backs `anvil intercept status` for parity with the Unix UDS path.
- **Process-group interrupt ladder** — INTD-006 lands the
  `SIGINT → SIGTERM → SIGKILL` ladder against the worker process group on Linux.
  macOS falls through to AD-7's fence-on-uncertainty invariant in this cut
  because the `current_process_start_time` helper is Linux-only; documented in
  the v0.6.0-beta release runbook §4.
- **Fence persistence** — INTD-005 records fence state to disk in the data
  directory, re-fences on daemon startup, and survives daemon crashes, restarts,
  and reboots. The `anvil intercept stop` and `unblock` CLI subcommands are
  deferred; recovery in v1 is the runbook's daemon-stop +
  fence-directory-removal procedure.
- **Daemon configuration & embedded fallback** — INTD-008 wires the daemon
  enforcement-config loader; INTD-010 evaluates rules in embedded mode when
  daemon dispatch is unavailable, keeping correctness equivalence with the
  daemon path. INTD-011 closes the unregistered-change fence so a write that
  bypasses validation still fails closed.
- **IPC DoS budgets and telemetry scoping** — INTD-009 caps per-connection
  request and response budgets so a misbehaving client cannot exhaust the
  daemon. INTD-015 scopes telemetry subscriptions to the requesting session
  rather than broadcasting cross-session.

### Editor Driver Framework (DRVR)

- **Driver client + protocol** — `anvil-driver-client` ships the shared client
  surface; DRVR-002 lands the editor-driver protocol with capability
  negotiation, and the trust-boundary spec is documented as a release artefact.
  RTAI-004 wires the mid-edit debouncer through `validateMidEdit`.

### Activation & MCP Launch (LAUNCH)

- **`anvil start` activation** — LAUNCH-002 owns the activation entrypoint with
  `--verify` and `--watch` flags. LAUNCH-009 wires Cursor / Claude Code MCP
  install with the shared activation-state vocabulary (`protecting`,
  `ready_restart_required`, `watching`, `needs_action`, `unsupported`, `error`)
  consumed by `anvil status --verify`, `anvil doctor`, and the protection-loop
  tutorial.
- **Repo language profile** — activation profiles the repository's languages and
  surfaces an honest skip ledger; TypeScript is the supported tier in this cut,
  SQL/Markdown partial, Python/Rust unsupported. Cross-language scans (secrets)
  continue running on every file.
- **Install-method-aware version surface** — `anvil --version` detects Homebrew,
  Scoop, WinGet, the cargo-dist installer, or a dev build and prints
  `update_available`, install method, and the recommended upgrade command. JSON
  shape is pinned for agent and CI consumers.

### Scanner Hot-Path Performance (V050F)

- **Allowlist regex caching** — V050F-006 (#1323) caches the compiled allowlist
  regexes in `prepare_pattern` and replaces `AllowlistGlob.pattern: String` with
  a precomputed `is_path_glob: bool`, eliminating an N×M regex compile on every
  scanned file.
- **Custom secret pattern compile errors** — V050F-011 (#1323) introduces
  `scan_content_with_compiled_patterns` and
  `scan_content_with_pattern_errors_and_stats` so callers receive per-pattern
  compile diagnostics instead of silent drop. The legacy `scan_content_with_*`
  wrappers preserve their signatures and emit `tracing::warn!` on dropped errors
  so the silent-loss path is observable.
- **Eager rayon pool init** — V050F-007 (#1330) extracts the half-cores
  global-pool cap into the dedicated `anvil-rayon-init` micro-crate and calls it
  from the CLI binary entry point and the NAPI `scan_artifact_json` entry,
  replacing the duplicated `Once` blocks in `kernel/watch.rs` and
  `kernel/embedded.rs`.

### CI Gating & Test Reliability

- **Cross-compile gate on `dev`** — PR #1325 (`ed957ce1`) widens the
  cross-compile trigger in `.github/workflows/rust.yml` from main-only to main
  + dev (push and PR), gated on `detect-rust-changes` so JS-only diffs don't
  spin up the Windows + macOS matrix. Closes the gap that let Windows-only
  build breakage land on `dev` between releases. Historical context preserved
  in `docs/runbooks/intd-012-windows-evidence.md` with a status banner.
- **MCP daemon integration tests Unix-gated** — daemon-backed integration suite
  is `#[cfg(unix)]` in this cut; Windows coverage rides the same follow-up as
  the MCP correlation envelope.
- **Coverage step non-blocking on push** — `cargo llvm-cov nextest` started
  failing the post-test profile-merge consistently on `dev` pushes
  (`error: no profile can be merged` from corrupt `.profraw`s). Strict test gate
  split from best-effort coverage in `76a17442`; coverage step marked
  `continue-on-error: true` so a coverage-merge failure doesn't mask real test
  signal. Underlying merge regression tracked separately.
- **Cancellation-test sync safety net widened** — the polling-loop bound in
  `cancellation_emits_cancelled_error_detail_not_spawn_failed` was ratcheted
  from 5 s to 30 s after sustained failures on `ubuntu-latest` under nextest's
  default parallel execution. Bound is a sync aid, not a timing assertion;
  structural follow-ups (worker-side notification, serial nextest group) noted
  inline.

## [0.5.1-beta]

### Scanner Signal Hardening

- **Secret false-positive reductions** — generic secret matching now requires a
  stronger right-hand-side shape, credit-card detection rejects UUID fragments,
  and entropy matching focuses on secret-shaped quoted values.
- **Antipattern suppression alignment** — `AP-*` checks now honour local
  `eslint-disable` directives, and `GS-001` avoids reporting guarded `Map.get`
  after `has`/`set` flows.
- **Audit input filtering** — audit scans skip broader environment-template
  files while still reporting real `.env` files regardless of directory.

### Kernel Incremental Graph Fixes

- **Synthetic import ID allocation** — watch graph updates now keep synthetic
  import IDs out of the allocator's file-ID range so incremental updates do not
  collide with real source files.
- **Import-source ID zero handling** — `update_file` now treats ID `0` as a
  valid import source, preserving edges that previously disappeared when the
  first allocated file participated in import analysis.

### TUI & Release Operations

- **TUI interaction fixes** — audit, status, and watch surfaces support zooming;
  doctor acknowledges `f` to fix; tutorial path selection has more room for
  wrapped options.
- **TypeScript scanner retirement** — the archived TypeScript scanner stack and
  parity harness now live under `archive/anvil-ts-scanner/`, with the Rust
  scanner remaining authoritative; stale scanner-era package subpath exports
  were removed from `@eddacraft/anvil-core` and `@eddacraft/anvil-runtime`.
- **PR base guard** — a release-sensitive PR base guard workflow now detects the
  branch-targeting mistake that caused the post-`v0.5.0-beta` recovery work when
  repository branch protection requires the check.

## [0.5.0-beta]

### Git Hook Compatibility (GHOOK)

- **Git 2.54 config-hook baseline** — compatibility policy added for native
  `[hook.<name>]` execution, with Anvil end users kept on the existing Git 2.30+
  floor unless they opt into config mode
- **`anvil hooks --config` path** — install/uninstall can append and remove
  Anvil-owned `hook.<event>.command` entries without touching foreign config or
  file hooks
- **Coexistence detection** — install, uninstall, status, doctor, onboarding,
  and tutorial surfaces detect file hooks, config hooks, third-party managers,
  `core.hooksPath`, and duplicate-execution risk
- **Contributor workflow decision** — GHOOK-005 accepted Option A: keep Husky as
  the repository bootstrap for now, while leaving `anvil hooks install --config`
  as an explicit power-user opt-in
- **Public docs sweep** — git-hook operations docs and CI/agent-harness examples
  now describe file-mode and config-mode behaviour together

### AI Guardrail & Diagnostics

- **AI guardrail profile complete** — `anvil gate --profile ai` now selects a
  curated check set, treats missing/invalid governance config as blocking, emits
  JSON by default, and documents the `anvil.gate-result.v1` contract
- **Canonical diagnostic shape** — `crates/anvil-kernel-types` now owns
  `anvil.diagnostic.v1` for gate, save-time, watch, and mid-edit diagnostics;
  the envelope coordination spec records how AIGUARD, RTAI, INTD, and DRVR share
  it
- **AI-001 reasoning rule** — `anvil-checks` now flags appeal-to-authority style
  comments, limits matching to comment regions, honours `@anvil-ignore AI-001`,
  and emits `Category::Reasoning` diagnostics at info severity
- **RTAI-001 phase-0 spike** — the mid-edit secret-detection loop measured about
  1.4 ms p95 over 1024 iterations, roughly 60x under the ADR-031 warm-path
  budget; the report chooses a single `scan_buffer` method with a mode
  discriminator for save-time versus mid-edit validation
- **Validation latency rubric** — ADR-031 pins latency budgets for save-time,
  mid-edit, and gate paths so future real-time validation work has an explicit
  performance envelope

### Scanner Coverage & Performance

- **Parallel scan rollout** — `gate`, `audit`, `check`, `drift`, policy,
  architecture validation, and watcher call-sites now share the gitignore-aware
  discovery plus rayon scan pattern; the SCAN benchmark recorded a 7.39x
  wall-time improvement on a synthetic 3k-file surface
- **ReDoS line-length guard** — `SecretCheckConfig::max_line_bytes` defaults to
  4096 bytes, skips oversized lines before regex evaluation, and reports skipped
  counts through `SecretCheckResult`
- **First-run pool cap** — first-run scans use `ANVIL_SCAN_THREADS` with a
  default cap of `min(num_cpus, 4)` to avoid starving TUI/editor work
- **`.env` secret surface** — `.env`, `.env.*`, and `.envrc` parsing routes
  values through the existing secret patterns, reports the variable name and
  source line, and supports `# @anvil-ignore SURFENV-001`
- **Scanner false-positive fixes** — AI-001 comment scanning is string-aware,
  and the TypeScript LSP fixture no longer trips the reasoning rule

### CLI, Onboarding & Editor Integration

- **`anvil mcp-config`** — Rust CLI command added for Claude Code, Cursor,
  Windsurf, and VS Code config generation; supports stdio/http transports,
  `--write`, `--verify`, workspace overrides, path-safety prompts, and atomic
  writes
- **Interactive fix handling** — start-flow surfaces share a single interactive
  fix service so doctor/status/onboarding prompts route consistently
- **Doctor missing-git behaviour** — `git-repo` now emits a structured warning
  rather than a failure when run outside a git repository
- **First-run copy** — inotify capacity guidance, instances-limit text, strict
  AI-guardrail config wording, and post-init auth-login next steps were
  tightened

### API, Infra & Release Operations

- **Database migration runner** — `apps/anvil-api` now has a first-party SQL
  migration runner, unit coverage for drift/pending cases, a manual runbook, and
  infra workflow wiring before Pulumi Up
- **Release publisher hardening** — the cargo-dist installer is SHA256-pinned;
  Scoop publisher pre-flight checks token reachability; WinGet publisher fork
  handling and `gh` argument usage were hardened after the v0.4.0-beta tag run
- **Release token runbook** — operator guidance now leads with editing the
  existing fine-grained PAT repository scope instead of rotating when Scoop or
  WinGet publishing gets a 403
- **Vercel/API runtime recovery** — Hono/Vercel entrypoint restoration, scoped
  API tsconfig, Nx framework-detection controls, and the `svix>uuid` override
  exception restored production deployment after the post-release runtime break
- **CORS and env exposure invariants** — tests now lock in lower CORS preflight
  cache lifetime and avoid treating all `NEXT_PUBLIC_` variables as sensitive

### Documentation, Plans & Attribution

- **Locked release slate** — release plan and roadmap now capture the A1 RTAI
  spike, A2 AI guardrail, A3 release engineering, and A4 language-credibility
  floor as the current release menu
- **Beta docs refresh** — tester guide and beta-user scenarios now cover the
  current onboarding, hooks, AI guardrail, MCP, and docs-auth flows
- **Portable attribution kit** — acknowledgement generation moved into a starter
  template set with `about.toml`, `about.hbs`, CI freshness snippet, and project
  example config
- **APS freshness** — GHOOK completed and archived; v0.5.0-beta
  release-follow-up, language audit, RTAI, AIGUARD, SCAN, RCLI2/RCLI3, and
  surface modules were reconciled against the current release plan

## [0.4.0-beta]

### Native Rust Scanner Becomes Authoritative

- **`.anvil` format parser and compiler** landed in `anvil-core` (`ANVFMT` Phase
  1); the registry-backed pattern catalogue replaces the legacy TS-side HTML/CSS
  catalogues entirely (`ANVFMT-014`, `ANVFMT-015`). Pattern reference docs in
  `docs/anvil` now describe the authoritative format (`ANVFMT-016`)
- **Rust scanner module (`RSCAN-001..008`)** — registry loader, artefact model +
  `scan_artifact` API, family provenance on `AntiPattern` and `Warning`,
  registry-backed pattern catalogue, rayon parallelisation, `--artifact` flag
  for non-source scanning, and a cross-engine fixture suite that runs identical
  inputs through the Rust and legacy TS scanners. Trust-boundary docs added;
  module closed via ADR-026
- **Scanner parity gaps closed (`SPG-001..006`)** — every shipped registry rule
  has a fixture, the antipattern scan has a Criterion bench, custom pattern
  compile errors surface at every secret-scan call site, and `flags:"i"` is
  honoured. Trust-boundary documentation added
- **napi-rs prebuild bridge for the legacy TS engine** (`TSRET-001`,
  `TSRET-002`) — full prebuild matrix across darwin x86/arm, linux x86/arm,
  windows x86/arm. ADR-030 supersedes the rest of TSRET (cutover from napi to
  surface drivers); pattern-registry getters added for the eventual driver
  bridge (`TSRET-003` prep)

### Workspace Hardening (RUSTNX)

- **`cargo-hakari` workspace-hack** generated and applied to every member crate
  to flatten transitive dependency feature unification (`RUSTNX-008`); internal
  crates marked `publish = false`
- **`cargo-deny` policy** added for licences, security advisories, and banned
  crates; CI gate runs the policy on every Rust PR (`RUSTNX-009`)
- **`cargo-about`** generates `ACKNOWLEDGEMENTS.md` with licence text for every
  transitive dependency; the new `anvil licenses` command surfaces it at runtime
- **`cargo-nextest`** adopted for CI test runs with per-target rust-cache keying
  (`RUSTNX-001`, `RUSTNX-002`)
- **Parallelised clippy + test jobs** in Rust CI (`RUSTNX-003`); test coverage
  cache pinned on `cargo-llvm-cov` version
- **Rust CI scope tightening** — affected-crate detection uses the PR base ref;
  vercel deployments gated by app-specific changes; nx-rust inferred targets pin
  the cargo package so vitest flags don't leak
- **Repository cleanup** — unused workspace dependencies dropped; APS module
  hygiene reconciled

### Notification Framework (NOTIFY)

- **Discovery and architecture phase** (`NOTIFY-001..005`) — inventory of
  current notification streams, taxonomy and priorities defined in
  `docs/anvil/quality/notifications`, delivery architecture and execution slices
  specified
- **Shared `Notification` envelope** (`NOTIFY-006..009`) — `check`, `gate`,
  `audit`, doctor, watch, tutorial, and onboarding/hooks all emit one envelope
  shape; subscriber filter contract documented; class and priority versioning
  surfaces in JSON outputs. `NotificationSource` trait in `anvil-tui` exposes
  current notices for future telemetry subscribers
- **Doctor JSON v2 contract** — `anvil doctor --json` now returns a root object
  with `checks`, `notifications`, and `schema_version`; every check carries a
  structured remediation object with summary, optional command, and optional
  docs URL

### Feature-Flag Plumbing (FLAGM)

- **`cli.licence-gate`** drives `requires_auth` and the `ANVIL_DEV=1` local
  override; bypass details (flag key, variant, reason) surface in verbose logs
  (`FLAGM-001..003`)
- **Admin invite path** moved from inline scope arrays to the shared flag
  resolver (`FLAGM-004`)
- **`anvil-api`** scope gates routed through `api.scope.*` flags (`FLAGM-005`);
  `/anvil` docs gate uses the same resolver
- **Dual-evaluation shims retired** (`FLAGM-006`); FLAGM module closed

### Admin / API / Operations

- **Per-operator admin keys** (`ADMINCLIH-001..004`) — peppered-hash lookup in
  `anvil-api`; per-operator key provisioning automated via Pulumi;
  send-migration uses a snapshot-token flow for atomicity; `--json` warning
  handling, AdminWriter hoist, stdout/stderr hygiene tightened
- **Anvil-API route coverage** — route-level tests for `/session/refresh`
  rotation (#777), auth-device flow (#665, #777), auth-otp flow (#672),
  auth-github callback (#787), waitlist + send-migration coverage gaps
- **Admin contracts** — ISO-8601 offset enforced on `IsoTimestamp`; missing
  README dropped from files manifest; zod schemas validate API responses
- **Email correction follow-on** — admin endpoint for email-mismatch repair
- **Watcher cwd redaction** (#1017) — error chains no longer leak working
  directory paths

### CLI / TUI Polish

- **Watch reliability** — partial setup survival, per-change panic isolation,
  SIGINT forwarding, redacted error chains
- **Watch filtering** — `--patterns` and `--exclude` now feed the watch loop
  instead of being declared-only flags; `--exclude` uses glob semantics, bare
  likely-directory names warn, and the plain-mode startup banner prints the
  active include/exclude scope
- **Watch animation** — animated stats and demo overlay; animations driven by an
  event loop instead of busy-spin
- **Onboarding** — post-init landing screen, shared default checks across
  welcome and init, ASCII fallback, title fit, TOCTOU fix on `.anvilrc`
  detection, `ANVIL_NO_PROMPT` / `NONINTERACTIVE` empty-string handling, login
  prompt for gated commands lacking credentials
- **`anvil init` first-touch diagnostics** — init runs an inline sample analysis
  with top warnings, counts, and `file:line` pointers; empty repos receive a
  tutorial/watch next-step hint. Git-history sampling is bounded by a timeout,
  and low inotify headroom surfaces as a fixable hint before watch startup
- **Doctor remediation safety** — non-passing checks expose runnable commands,
  docs links, or fix prompts in plain mode and TUI detail panels; `doctor --fix`
  writes a valid YAML `.anvilrc` and refuses unsafe `git init` in directories
  without project markers
- **Tutorial** — verify-step `husky` flow, scan truncation visibility,
  watcher-failure cause in static-mode notice, language model aligned with TUI
  surfaces
- **Welcome hub navigation** — arrow keys move through panel rows, list panels
  scroll, and unfocused panels freeze their scroll state
- **`.anvilrc` gate-check vocabulary** reconciled with the runner; legacy check
  names mapped during transition (#1016, #1041)

### Distribution (DIST-011)

- **Scoop bucket** published with PR-based update flow
- **WinGet icon** shipped; `IconSha256` template placeholder quoted; PR body
  written to file with `set -e` to fail on errors
- **README install section** covers every supported package manager
- **ADR-025** records the package-manager distribution strategy

### Test Infrastructure

- **OPA policy hardening (`TCOV-009..013`, `TFIX`)** — pinned `cargo-llvm-cov`
  with version-keyed cache; real-binary OPA integration tests; hermetic OPA via
  PATH-isolated tests; `rego.v1` import for OPA 0.60 compatibility;
  cross-platform OPA binary lookup
- **Tests covering 7 GH issues** closed during the cycle (#558, #672, #665,
  #777, #787, #723, #1052)
- **Transactional smoke tests** for email render templates
- **API neon SQL** fragment composition pinned for admin list queries

### Tooling & DX

- **Agent-driven release** (`RELMGMT` Phase 3) — `/release` skill drives version
  pick, branch strategy, tag, workflow, artefact verification, comms, and
  cleanup; reads live `git`/`gh` state each turn
- **nxrust plugin** now consumed from npm as `@eddacraft/nxrust`; inferred
  targets via `cargo metadata`; per-crate `project.json` no longer needed
- **Husky pre-commit** enforces `oxfmt` on markdown and TOML so format drift
  surfaces locally rather than in CI
- **`scripts/release.sh`** preflight runs Rust + TS fmt/lint/typecheck/test as
  one bundled gate; supports `ANVIL_RELEASE_STEP_TIMEOUT` for repos where the
  parallel nx test run exceeds 600 s

### Plan Hygiene & Documentation

- **9+ completed APS modules archived** (POLISH, RCLI, MAINT, ADMINCLI,
  ADMINCLIH, anvil-scanner-parity-gaps, NXRUST, DBCON, DIST, TUTOR); counts
  reconciled across `plans/index.aps.md`
- **WEAVE module renamed from LCORE** for the standalone weave-rs strategy;
  ADR-024 amended; design spec and implementation plan published
- **ATTRIB v3 attribution pipeline module** authored (Ready); `deny.toml`
  references aligned
- **`next-steps.md`** added as session-continuity artefact for cold restart
- **ADR-030 surface drivers supersede napi cutover** — TSRET-003/-004 superseded
  by DRVR; X5 sequencing decision closed
- **Quality model and follow-on plans** documented in `docs/anvil/quality/`
- **Public docs refresh** — release pages, historical release pages, pattern
  reference, install section, runbooks (`DOCSYNC`)

### Dependencies & Security

- **TypeScript bumped to `~6.0.3`** across all workspaces
- **Production-deps group bump** (10 updates) and **development-deps group
  bump** (16 updates)
- **Pre-release security sweep** — `EAMIG-003`, `EAMIG-046` security overrides
  applied
- **CI dependencies** — `pnpm/action-setup` 6.0.0 → 6.0.3, `setup-node` 6.3.0 →
  6.4.0, `trufflehog` 3.94.3 → 3.95.2, `trivy-action` 0.35.0 → 0.36.0,
  `setup-regal` 1.0.0 → 2.0.0
- **`cargo-deny`** version pinned at 0.19.4

## [0.3.3-beta]

### Distribution & Release Engineering

- **WinGet distribution pipeline** — Windows release automation now emits and
  submits WinGet manifests for tagged releases, extending the binary
  distribution surface beyond direct install scripts and Homebrew formulae
- **Windows signing groundwork** — Authenticode signing path wired into the
  release pipeline via Azure Trusted Signing and SSL.com integration so Windows
  artefacts can move to signed distribution once identity provisioning clears
- **Release automation hardening** — `scripts/release.sh` tightened around
  preflight validation, bundled test execution, remote state checks, and
  manifest handoff to the release skill
- **Public release promotion** — release automation now flips production GitHub
  releases to `Latest` consistently instead of leaving beta-tagged artefacts
  hidden behind cargo-dist defaults

### CLI & TUI

- **Windows input handling** — Ratatui/crossterm event handling on Windows now
  filters to key-press events only, removing duplicate input in onboarding and
  discovery flows
- **Discovery surface repair** — two-panel layout restored with predictable
  scrolling behaviour and a reliable onboarding reset path
- **Tutorial completion fixes** — tutorial exit code handling, `husky` flow, and
  verify-step sentinel behaviour corrected so scripted onboarding paths complete
  deterministically
- **Installer UX polish** — post-install output now prints a branded next-steps
  block with colour support and direct pointers to `anvil auth login` and
  `anvil welcome`
- **Admin CLI surface expansion** — the admin command set moved from endpoint
  groundwork to an operational CLI with `list`, `show`, `approve`, `invite`,
  `audit`, `revoke`, and `send-migration` flows layered over the beta-user
  service APIs
- **CLI hardening wave** — admin command paths now validate flags earlier,
  detect TTY state from stdin and stderr together, sanitise control characters
  in rendered tables, align audit types to the server contract, and make error
  handling more testable and explicit

### API & Operations

- **Admin list endpoints** — read-only waitlist and audit-list surfaces added in
  support of the in-progress admin CLI module (`ADMINCLI-001`–`ADMINCLI-004`)
- **Licence key boot probe** — `anvil-api` now validates the ES256 signing key
  during startup and reports status through `/health`, surfacing secret/config
  failures before auth traffic hits runtime paths
- **Admin approval collision handling** — approval flow now retries `user_code`
  uniqueness collisions and accepts longer codes to reduce back-to-back approval
  failures
- **Structured auth logging** — waitlist and auth routes emit more consistent,
  structured operational logs for support and production debugging
- **DBCON groundwork** — database consolidation module introduced for the Neon
  project merge, including operator-only waitlist pause controls and bridge
  migration work
- **Email correction path** — auth UX now handles email mismatch more clearly,
  and the admin API exposes an email-update endpoint so operators can repair
  beta-user addresses without direct database edits
- **Migration operations** — admin migration sending now has an operator
  runbook, correct dry-run semantics, and non-zero failure exits for automation
  safety
- **DBCON follow-on work** — option-B reset flow started,
  `ANVIL_API_DATABASE_URL` rename introduced, and verification/snapshot steps
  hardened for the next Neon cutover stage

### CI, Benchmarking & Security

- **Nightly stress benchmarks** — benchmark runner added to CI to catch native
  engine performance regressions outside the tagged release path
- **Dependency remediation** — `follow-redirects` pinned to a non-vulnerable
  range to close a known supply-chain issue
- **ADR coverage** — ADR-024 published for the literate-core agent harness; KERN
  and BENCH modules archived after completion
- **Toolchain refresh** — pnpm, Cargo crates, and selected GitHub Actions moved
  forward during the release window to keep the admin CLI and release pipeline
  on current dependency baselines

## [0.3.2-beta]

### CLI Surface

- **Self-update command** — `anvil update` added as an in-place binary updater
  with version detection, asset download, and verification flow (`RCLI`)
- **Admin invite command** — `anvil admin invite` shipped with dual-mode invite
  flow (email plus approval path), extending beta-user operations from the CLI
- **Welcome/onboarding completion** — all WELCOME tasks closed, finishing the
  first-run path with discovery mode, executable tutorial steps, live watch
  demo, fix flow, and hook installation guidance

### Release & Platform

- **Interactive release script** — `scripts/release.sh` now orchestrates
  preflight, branching, tagging, and workflow kickoff, and writes
  `.release/manifest.json` as a handoff contract for the release skill
- **Feature flag operations docs** — feature-flag inventory and governance
  guides published to make ad-hoc flags auditable across runtime surfaces
- **Windows target expansion** — `aarch64-pc-windows-msvc` added to cargo-dist
  configuration, with updater support explicitly deferred pending upstream
  binary availability

### Reliability & Codebase Maintenance

- **OTP query determinism** — `ORDER BY` restored in `findActiveOtpCodes` to
  prevent non-deterministic code selection under concurrent auth traffic
- **SQL centralisation** — inline API-route SQL moved into `db/queries.ts` to
  make data access easier to audit and less error-prone
- **Tutorial/TUI fixes** — tutorial commands brought back in sync with the Rust
  CLI and long audit result lists fixed to scroll correctly
- **Install flow repair** — installer next-step output now prints reliably; the
  Homebrew tap publish path is triggered automatically during release
- **CI stability** — Semgrep version pinned to avoid upstream breakage and OSSF
  Scorecard restricted to the default branch to reduce noisy failures

### Planning & Governance

- **Versioning decision recorded** — ADR-020 published for release/versioning
  policy
- **Decision log introduced** — `DECISION-LOG.md` added as the single-entry
  index for ADR discovery
- **APS maintenance** — completed modules archived and APS workflow rules
  tightened to keep release and planning state aligned
- **Coverage uplift** — 59 unit tests added for previously under-covered
  `anvil-cli` modules (`TCOV`)

## [0.3.1-beta]

### Infrastructure

- **Feature flags module** — shared feature flagging system across TypeScript
  and Rust surfaces (`FLAGS-001`–`FLAGS-009`)
  - Contract schema with JSON Schema validation
  - Runtime resolver with environment-aware flag evaluation
  - Snapshot system for point-in-time flag state capture
  - Telemetry hooks for flag evaluation tracking
  - Exemplar test fixtures
  - Kernel-side feature flag types, resolver, and snapshot mirroring TS surface
  - Feature flag governance, inventory, and reference guides
  - ADR-019: flags–observability alignment decision
- **CI composite actions** — `setup-workspace` action extracted to deduplicate
  Node/pnpm/Nx setup across workflows; `detect-changes` action for path-based
  job filtering
- **CI workflow fixes** — 8 issues resolved: checkout ordering, clippy/rustfmt
  failures, formatting in setup-workspace action
- **Docs-shell app** — Next.js shell application for docs domain proxy with auth
  callback, login, logout routes, JWT/cookie/state libraries, and unit tests
- **Docs upstream scaffolding** — Docusaurus apps for private and public docs
  with middleware, sidebar configs, and Vercel deployment configuration
- **Vercel build skip** — `vercel-ignore-build.sh` script for skipping preview
  deploys on non-release branches

### Documentation & Delivery

- **PR template** — durable link requirement for manual testing; rationale moved
  to section comment
- **README** — lowercase brand usage, Windows aarch64 scope clarification,
  'Anvil Check' action name restored
- **Release doc checklist** — public distribution repo section added; broken
  inline code span fixed; oxfmt formatting applied
- **AGENTS.md** — updated with current conventions

### Dependencies

- pnpm dependency upgrades (vitest, vite, @types/node, globals, @nx/eslint,
  @nx/vite, @vitest/coverage-v8, @github/copilot)
- Cargo dependency upgrades via Cargo.lock refresh
- lint-staged configuration updated

### Tooling

- `aps-cleanup.service` — systemd service for APS status lifecycle automation
- `nx.json` configuration updates

## [0.3.0-beta]

### Platform Foundations

- **Language and runtime baseline** — TypeScript moved to 6.0 across workspace
  packages and the Node engine floor was raised to `>=22`, reducing divergence
  between local and CI environments (`MAINT-011`)
- **Rust toolchain uplift** — toolchain advanced to 1.94.0 with Windows and
  macOS cross-compilation support aligned to the release matrix
- **Linting and formatting refresh** — oxlint adopted as the first-pass linter
  and oxfmt replaced Prettier for the primary formatting path
- **Documentation platform refresh** — Docusaurus upgraded to 3.10 to keep the
  docs stack current with the Rust CLI release

### Performance & Verification

- **Kernel benchmarking in CI** — Criterion benchmarks for critical kernel paths
  and the stress-test harness were wired into CI, with execution scoped to main
  pushes and manual dispatch where appropriate (`BENCH`)
- **Test coverage uplift** — 59 unit tests added for under-covered `anvil-cli`
  modules alongside an integration suite for the checks crate
- **CI modernisation** — GitHub Actions refreshed to current major versions,
  unused jobs removed, and CodeQL added with path scoping to improve signal and
  maintainability

### Architecture & Dependency Governance

- **Dependency refresh** — key build and runtime dependencies updated, including
  Criterion 0.8, Reqwest 0.13, Dirs 6, and Vite 8
- **Architecture decisions published** — ADR-015 (shared packages restructure)
  and ADR-016 (unified config format) recorded the main design decisions behind
  the release

## [0.2.1-beta]

### Platform & Integration

- **Edda/Ember/Stack integration** — contracts and service-layer work matured
  the project-memory foundation introduced in the 0.2.x beta line

### Security & Hardening

- **Parser and adapter hardening** — validation tightened across parsers,
  adapters, and the APS plan loader to reduce malformed-input and edge-case risk
- **Subprocess execution hardening** — command execution paths further locked
  down to reduce shell-safety regressions
- **Dependency remediation** — vulnerable dependencies patched, including
  `minimatch`, `axios`, `svgo`, and `tar`

## [0.1.3]

### CLI & Delivery

- **CLI stream policy** — stdout/stderr behaviour standardised so automation and
  human-readable output are easier to consume consistently
- **Hook script consolidation** — Git hook scripts moved to a single source of
  truth to reduce drift across local and CI execution
- **Default API endpoint update** — default backend URL moved to
  `eddacraft-api.vercel.app`

### Architecture

- **Rust engine decision recorded** — ADR-011 published the architecture
  decision for the Rust core engine direction
