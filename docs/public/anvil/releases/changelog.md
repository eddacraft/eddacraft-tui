---
id: changelog
title: Current release notes
description:
  Current anvil beta changes and links to the complete release archive.
owner: DOCSYNC
upstream:
  - CHANGELOG.md
  - RELEASE-PLAN.md
verified_against: 0.9.7-beta
---

# Current release notes

This page summarises user-visible beta changes. Internal work items, source
paths, and implementation notes are deliberately excluded. For the full
version-by-version history and downloadable artefacts, use the
[GitHub release archive](https://github.com/eddacraft/anvil/releases).

## 0.10.0-beta — 13 September 2026

### Fixed

- **A missing Windows named pipe is reported as daemon unavailable.** Connecting
  when no intercept daemon is listening no longer looks like a wrong-owner
  refusal, so clients can retry instead of treating "daemon down" as a hostile
  pipe.

- **`anvil welcome` restores the terminal on every exit.** Success, cancel,
  ordinary errors, partial setup failures, and panics all leave raw mode, mouse
  capture, and the alternate screen cleaned up.

- **Guided setup stays on the repository you selected.** Discovery, preview,
  apply, tutorial state, and completion markers follow the wizard selection,
  including nested directories and linked worktrees.

- **Learning-path progress is kept across projects.** Completing a path records
  progress when the final step finishes — quitting the completion screen no
  longer looks like an early Back. A repository without a local first-run marker
  still loads your global progress.

- **L4 activation uses Git's effective hook path.** Installation, removal, and
  status honour `core.hooksPath` (relative, absolute, or tilde) and linked
  worktrees, instead of assuming `.git/hooks`.

- **Daily ensure preserves explicit MCP launch choices.** Your chosen
  executable, environment, and per-entry options survive repair. Only obsolete
  managed command paths are migrated; entries you overrode stay as you left
  them.

- **MCP serve pins to the admitted Git worktree.** Starting from a package
  subdirectory, linked worktree, or symlink shares one canonical project with
  activation, status, graph, and validation. Launches outside a repository are
  refused with reconnection guidance.

- **Concurrent daemon ensure no longer fails on a short lock wait.** When
  another `ensure` already holds the rendezvous or start lock, the waiter
  retries until the lifecycle budget is spent, re-probes for a live peer-started
  daemon, and reuses it when one answers. A genuinely stuck holder still fails
  closed when the budget expires.

- **Windows doctor and config paths share one identity.** Delegated
  `architecture.source` and leftover checks no longer mix `\\?\` canonical forms
  with ordinary drive paths, so a healthy migrated config is not reported as
  dual-truth shadowing (Warn vs Pass plus a leftover offer).

- **Windows named-pipe clients authenticate the daemon before traffic.** After
  the existing ownership gate, the driver checks the pipe server process SID and
  refuses a mismatched server before handlers attach. Trusted Windows config
  files whose DACL grants write to another principal are refused the same way
  Unix refuses world-writable config.

- **Provenance and debug output no longer keep credential-shaped material.**
  Token-shaped session ids, Git remote userinfo, SCP-style remotes, prompt text,
  and nested structured debug values are redacted before they reach Git notes,
  stored provenance, or the console. Ordinary remotes, messages, and exact Git
  object ids stay intact.

- **False-positive secrets can be marked without silencing real keys.** Codex's
  `anvil_suppress` comments (`@anvil-ignore-until DATE ID: reason`) are now
  parsed. Entropy, generic-secret, API-key, and credit-card matches honour
  `@anvil-ignore` / `SECRET-DETECTION` on the previous line. High-confidence
  shapes (AWS keys, GitHub PATs, Stripe live keys, private keys, vendored
  provider rules) still cannot be ignored inline. A withheld false positive
  stays visible as an info diagnostic and a local `report-fp` record so the
  scanner can be tightened.

- **Graph-context MCP tools no longer accept a nested directory as the workspace
  root.** `anvil_search_symbols`, `anvil_find_callers`, `anvil_find_dependents`,
  `anvil_impact_of_change`, `anvil_affected_tests`, and `anvil_symbol_context`
  now require `workspaceRoot` to be the MCP server root itself or a linked Git
  worktree root of the same repository. Pointing a graph tool at a subdirectory
  used to rebuild the graph from that directory, so a sensitive path such as
  `secrets/token.ts` could surface as `token.ts` past the sensitive-path filter.
  Check, gate, status, and write-validation tools still accept directories
  inside the server root.

- **Generated record identifiers are no longer flagged as high-entropy
  secrets.** Content-addressed ids with a lowercase prefix, dashed segments, and
  a hex tail stay quiet. Mixed-case opaque tokens and real credentials still
  fire.

- **Secret scans no longer report clean when they could not read the input.**
  Oversize lines, unreadable files, and scanner panics block a clean pass;
  extension skips stay advisory. `anvil audit` and planless `anvil check` report
  that coverage gap the same way `anvil gate` already did. The two commands draw
  the failing line differently: `check` exits non-zero on any coverage note,
  while `audit` exits non-zero only when a _file_ went unread and reports an
  unscanned oversize _line_ on a passing run — read `coverageNotes` rather than
  inferring coverage from the exit code. The scan limit rises from 1 MiB to 8
  MiB and the reader now holds one line at a time instead of the whole file;
  files at or over the limit are still unscanned, but they are reported as a
  coverage failure instead of passing silently.

- **Gate and drift now report when the architecture check did not run.** Without
  architecture config, gate JSON sets `skipped` and drift snapshots set
  `boundary_analysis_skipped`. Both are additive flags beside the existing
  values, not replacements: gate still reports `passed: true` with a score of
  100 for a check that never ran, and drift still reports
  `boundary_violations: 0`. A consumer that branches on `passed` or the score
  alone sees no change, and must read the new field to tell "not measured" from
  "clean".

- **`DO_NOT_TRACK` now stops save-time and fence usage rows as well as the CLI
  beacon.** Any non-empty value other than `0`/`false` is a hard-off.

- **GitHub release notes advertise the Homebrew tap formula
  `eddacraft/tap/anvil`.** cargo-dist used the crate name (`eddacraft-anvil`) in
  the Homebrew stanza while the tap formula is `anvil`. Copy-paste of the
  v0.9.7-beta release-notes install line would not resolve. (#4077)

- **GCTX and `anvil intercept status` find a daemon bound under the other Unix
  socket path.** A client with `XDG_RUNTIME_DIR` set no longer reports
  `unavailable` when the intercept daemon was started without that variable (and
  vice versa). `anvil intercept status` prints the socket it actually used in
  its human-readable output (the `--json` document is unchanged). `anvil doctor`
  warns on the split; `anvil doctor --fix` stops the sibling-path daemon and
  starts one at this process's canonical socket. `ANVIL_HOME` prefixes stay
  exclusive. Socket paths that are symlinks are still refused.

- **Unix daemon rendezvous now follows live listeners through their full
  lifecycle.** Stale socket inodes no longer hide a live sibling daemon, and a
  long-running `anvil watch` re-resolves the candidate paths after fallback or
  daemon relocation. The liveness check is the connection the request is then
  sent on: the daemon sees one connection per save-time scan, MCP graph query,
  or LSP scan, not a dropped probe followed by a second connect. Doctor holds
  the start and PID locks before removing a proven-stale socket. Version
  recycling waits every daemon it signalled, then starts the current binary at
  the canonical socket; leftover sibling PID-file metadata does not abort that
  restart. `anvil intercept stop` reports the deduplicated worktrees losing
  protection across all stopped daemons, or says the impact is unknown when it
  cannot query one safely. JSON still lists every candidate, including
  unresolved sibling PID files, without failing the command.

- **`anvil policy eval-regression` detects rules that go silent on frozen
  fixtures.** A finding that appears or disappears now reports that the fixture
  output changed, rather than calling a disappearing finding an improvement.
  `--fail-on-regression` exits non-zero for that change; report-only runs remain
  non-blocking.

- **Antipattern scan no longer panics on a multibyte character at a keyword
  boundary.** A `§` (or any 2-, 3-, or 4-byte glyph) sitting where the masker
  split a word used to crash `anvil init`, `anvil baseline`, and
  `anvil check <file>` with exit 101.

- **`anvil workspace allow` refuses Git Bash drive-relative paths.** `D:repo`
  (the form MSYS produces when it eats the slash after the drive letter) is
  rejected instead of being stored as `{cwd}\repo`. `anvil workspace list` flags
  entries the daemon cannot resolve.

- **`anvil mcp refresh` no longer fails on Windows while other anvil processes
  hold lock files in `%LOCALAPPDATA%\anvil`.** Opening the existing state
  directory no longer requests DELETE. Replacing the refresh generation file
  also succeeds while a live `mcp serve` process holds it open mid-poll: the
  held file is renamed aside and removed once its last reader closes, the same
  swap `anvil update` uses for the binary. `anvil doctor` fails `mcp-heal` when
  the generation poke itself errors, instead of ticking Pass over the failure.

- **`anvil update` completes on Windows while the daemon and editors hold
  `anvil.exe`.** The running binary is renamed aside so the installer can write
  a fresh one — the same swap rustup uses — so updating no longer requires
  `anvil intercept stop`, closing editors whose MCP servers hold the file, or
  running the installer by hand. Processes holding the old binary keep running
  it; the next `anvil` or `anvil start` recycles a version-skewed daemon. Parked
  copies (`anvil.exe.old-<pid>`) are cleaned up by a later `anvil update` once
  their holders exit. If the rename itself fails, the manual recipe (PowerShell
  installer after `anvil intercept stop`) is printed as before.

- **Watch events match the bound project root on Windows.** Short 8.3 names and
  `\\?\` verbatim paths no longer look like a different tree than the
  dunce-canonical root `anvil watch` bound, so TUI file-change handling (and
  dwell-clear) still runs for the project you opened. (#4659)

### Changed

- **`anvil status` reports L4 `on` only when a pre-push hook and a parseable
  acceptance policy are both present.** A file-mode hook without
  `anvil/policy.*` is `partial`, matching the silent no-op both L4 entry points
  take when no policy exists.

- **Protection status now proves a client is actually attached.**
  `anvil mcp serve` registers a live session with the intercept daemon,
  identified from the editor's own MCP handshake. Live pre-write status is
  reported only for a client with fresh, independently attributed evidence, so
  an editor that is configured but closed no longer appears to be protecting
  your writes, and one client's activity can no longer make another look live. A
  registered worktree with no attached editor still reports the daemon as
  enforcing.

- **First-use bare `anvil` skips the licence wall when nothing is activated
  yet.** A never-activated checkout gets the existing not-activated report
  (exit 1) instead of the licence prompt. Once project config exists, daily
  ensure stays entitled as before. `anvil welcome` remains optional ungated
  learning and does not start MCP, hook, daemon, or workflow mutations while
  unsigned.

- **Unattended `anvil start` no longer treats missing MCP as consent.** New MCP
  installs need `--mcp-client` and/or `--all-mcp-clients`. `--no-tui` on a real
  TTY is interactive plain consent with pickers; `--json` and `--verify` stay
  read-only.

- **Healthy bare `anvil` restores remembered integration coverage quietly.**
  Client, scope, executable, and optional protection choices are inferred from
  owned artefacts. Unresolved faults after ensure has already tried restore name
  `anvil doctor`. `anvil start` remains the deliberate reconsider path.

- **Graph-context answers attest their own bounds.** MCP graph envelopes report
  which budget bound each section, per-edge call-resolution fidelity (Exact vs
  Heuristic), and an estimated token cost. Oversized envelopes omit cost rather
  than truncating the estimate. Skills and the AI-context guide say how to route
  on those disclosures.

- **MCP updates no longer discard an accepted request.** A skewed MCP process
  may replace itself before its first stdin read. Once a session is established,
  it completes accepted and pipelined requests on the current image and asks for
  a targeted MCP reconnect to use the preferred binary.

- **Ember is inactive pending its Rust migration.** The historical
  `anvil ember list` reader is hidden and default-off behind `ember.enabled`.
  Explicit `ANVIL_EMBER=1` allows historical reads only; generation remains
  unavailable. Existing databases are preserved. The unused JavaScript Ember,
  runtime watch and concurrency entry points have been retired.

- **Docs and shipped skills say when AST actually runs.** Pre-write and
  save-time stay regex plus secrets. `anvil check` / MCP `anvil_check` add AST.
  Watch may print a non-blocking AST follow-up after allow. `anvil gate` is the
  merge judgement; default watch is not. The save-time guide and the
  checks/gates page now say the same split.

- **`anvil watch` reports background AST findings in one line.** After a
  save-time allow, the cheap-catalogue follow-up still does not block the write.
  When it finds something, stderr prints a single `AST follow-up` warning so a
  green save is not read as fully clean. Empty results stay silent.

- **`--help` no longer advertises unfinished opt-in commands.** `anvil impact`,
  `anvil plan`, and `anvil dashboard --web` stay in the binary and stay
  default-off; they leave the visible list until they can be default-on. Known
  names plus `ANVIL_IMPACT=1`, `ANVIL_DASHBOARD_WEB=1`, or `ANVIL_DEV=1` still
  invoke them. MCP `anvil_impact_of_change` is unchanged.

- **`anvil start` help bar leads with arrows and hides dead Enter.** Consent and
  the result screen advertise `↑/↓` and, when there is more than one consent
  step, `←/→ next section`. `h`/`j`/`k`/`l` still work as aliases. Enter is
  listed only when it does something on the focused row (unsafe-drift confirm,
  or toggle a verdict section).

- **Custom anti-pattern registries are explicit-only.** The catalogue embedded
  in the binary remains the default even when a checkout contains
  `patterns/compiled/registry.json`. Set `ANVIL_REGISTRY_PATH` (or provide an
  explicit API path) to replace it; a cloned repository no longer changes what
  anvil flags merely because that file is present.

- **`anvil insights --json` now defaults to `anvil.insights.v3`.** Six of the
  seven weekly metrics are not instrumented and were emitted as `0`,
  indistinguishable from a measured zero, so any consumer of the default
  document recorded six fabricated results. v3 emits those fields as `null`
  instead. Pass `--schema v1` for the previous document if you are pinned to
  that shape. The human-readable output is unchanged — it has always said "not
  yet measured".

### Added

- **Fresh `anvil init` writes `anvil/policy.yml`.** The file matches the default
  L4 branch posture (main and `*` require `l4_or_l3`; `dependabot/*` requires
  `l4_only`). Existing `anvil/policy.{yaml,yml,json,toml}` files are left
  unchanged, including under `--force`. `anvil start` does not create a policy
  on a repo that already has project config.

- **First-value closing receipt after setup.** When selected MCP and save-time
  coverage are proven, the receipt names the project, selected coverage,
  connected or pending client, policy mode, last proof, and one next step.
  Incomplete setup names one owner instead of claiming success. `anvil status`
  and `anvil doctor` consume the same receipt facts (status JSON adds an
  additive `receipt` object).

- **`anvil start` installs managed skills for chosen MCP clients.** Ticking a
  client (or passing `--mcp-client`) now writes the bundled skills at that
  client's skill root, not only the MCP config. Clients with no skill location
  (for example Grok) are skipped without failing activation.

- **`anvil skill install` also writes `using-anvil`.** The managed installer
  already shipped `anvil-developer-functions`. The companion setup/CLI skill was
  vendored in the binary but never installed, so agents could not load the
  hand-off it names. Repeat install now writes both skills to each selected
  client root, with the same provenance and refuse-unmanaged rules.

- **`anvil check` writes last-run reports under `.anvil/`.** Every run that
  produces a result overwrites `.anvil/last-check.txt` (the same human report as
  plain stdout) and `.anvil/last-check.json` (the existing check JSON schema).
  Open the text file in an editor, then hand the JSON to an agent without
  re-running the scan. `--format` still selects stdout only; a write failure
  warns and does not change the check result.

- **`anvil impact` maps the current repository in the terminal.** The read-only
  view starts from the crate-level used-import graph, drills into a crate's
  neighbourhood or internal modules, and restores the prior view when you back
  out. Keyboard and mouse navigation are supported; `--json` and `--no-tui`
  provide non-interactive output. It needs a warm graph snapshot and names cold
  or unrenderable states instead of showing an empty canvas. The command is
  default-off and hidden from `--help` until that experience is hardened; opt in
  with `ANVIL_IMPACT=1` or `ANVIL_DEV=1`.

- **Gate-time Python scans catch multiline swallowed exceptions.** The new
  `PY-010` rule flags a named `except` handler whose body is only `pass`. MCP
  `anvil_check` and target-file `anvil_gate` calls now include AST findings,
  matching the CLI check path. The latency-sensitive interactive pre-write path
  remains regex-only.

- **`anvil conformance check` evaluates a PR declaration against an exact Git
  range.** Advisory only: missing, malformed, or incomplete evidence is never
  treated as conformant. The command is licence-gated like `anvil check` and
  accepts `--format json|sarif`.

- **`anvil-control-examples` is a second bundled policy pack.** Install it with
  `anvil policy install anvil-control-examples`. `anvil policy members` lists
  and toggles members; the overlay survives `install --force`.
  `crypto-human-signoff` hard-stops MCP writes until a human records a grant.
  This is an engineering-control template, not a compliance certification.

- **`anvil drift snapshot --no-save` computes a snapshot without writing it.**
  Isolated `ANVIL_HOME` no longer needs `--touch-project-state` just to print
  the JSON.

- **Secret detection includes vendored gitleaks tier-1 provider rules.**
  Prefix-anchored credentials (GitLab, Slack, Stripe, and similar) now match on
  the built-in path. anvil still compiles the rules itself; no third-party
  scanner enters the product.

## 0.9.7-beta — 21 August 2026 — First-session honesty

First-session welcome, gate progress, learning path, and Audit Next Steps tell
the truth after the published 0.9.6-beta run.

### Fixed

- **MCP tools accept linked Git worktrees of the same repository.** Agent
  sessions that start `anvil mcp serve` from the primary checkout can pass a
  sibling Worktrunk or harness worktree as `workspaceRoot`. Other repositories
  and unregistered directories are still refused.

- **`anvil gate` and `anvil check` no longer hang while excluding generated
  files.** Anti-pattern scanning still honours `.gitattributes`
  `linguist-generated`. On a large repo it used to stall inside `git check-attr`
  and ignore Ctrl+C. Repos that never set the attribute skip that lookup.

- **Unsigned `anvil welcome` no longer dead-ends on gated Policy or Architecture
  steps.** Those commands still need a signed-in session. When you are not
  signed in, the path names `anvil auth login` first and does not present the
  gated command as a runnable check. Free `--verify` probes stay runnable.

- **Hub "Review gate decision" shows live progress.** On a large repo the hub no
  longer sits on a frozen "Running quality checks..." line. The loading line
  updates as the workspace is scanned and as each check starts.

- **"Choose a learning path" opens the path picker.** Hub and first-run no
  longer run a discovery scan before the picker. Guided setup can still wow with
  discovery before offering the menu.

- **Audit Next Steps jump to the matching issue.** Enter on a Next Steps row
  focuses and expands that issue. One large-file step lands on that file. The
  footer no longer promises expand on a panel that cannot expand.

## 0.9.6-beta — 18 August 2026 — Beta field fixes and shell command-safety

After 0.9.5-beta, hooks and warnings tell the truth again, and shell scripts get
the same dangerous-command coverage runtime already had.

### Added

- **Type-assertion rules for TypeScript.** Chained `as unknown as T` laundering
  and unvalidated boundary casts are now called out in the compiled catalogue
  (`GS-002`, `TE-001`).

- **Shared shell command-safety rules.** Runtime command-safety and the
  default-on `shell-scripts` surface now share `pipe-to-shell` (Block at
  runtime), `eval-dynamic` (Warn), and `chmod-777` (Warn). The shell-scripts
  surface stays warn-only. The `command-safety` class remains hard-pinned;
  per-rule `.anvilrc` disabling is not wired through `anvil gate` today — skip
  one invocation with `--skip-checks=command-safety`, suppress a script line
  with `# @anvil-ignore <rule-id> -- <reason>`, or set
  `ANVIL_TRACK_SURFACE_SH=0`.

### Fixed

- **`eval-dynamic` now flags `$1`, `$@`, and `$name/suffix`.** Static literals
  and ANSI-C `$''` leftovers still stay quiet.

- **Pipe-to-shell analysis uses the shared compound helper again.** Runtime and
  shell-scripts share one analyser for pipelines and nested shells, matching the
  accepted shell catalogue design. The three shared rules are unchanged.

- **Default Git hooks now record the L3 witness chain.** `anvil hooks install`
  and Git-hook setup through `anvil start` keep the pre-commit quality gate, add
  the commit witness, and install a managed post-commit step that binds the
  resulting `HEAD`. Existing anvil-managed gate-only hooks are upgraded without
  `--force`, and `anvil hooks status` names the witness step.

- **Hook witness fallback is quiet and observable when the daemon is down.**
  Commits no longer dump a raw pipe or socket error before falling back to the
  embedded witness writer. `anvil doctor` and `anvil start --verify --why` now
  explain the degraded path; hooks still do not auto-start the daemon or block
  the commit solely because it is unavailable.

- **`--fail-on-warnings` now fails the four warn-only surface checks.**
  `dockerfile`, `shell-scripts`, `sql-migrations`, and `github-actions` still
  pass by default when they only have warnings. With `--fail-on-warnings` or
  `ANVIL_FAIL_ON_WARNINGS=1`, an unsuppressed finding on those checks fails the
  check and the gate exits non-zero. Anti-pattern behaviour is unchanged.

- **Bare document filenames no longer look like high-entropy secrets.** Names
  such as `quarterly-revenue-forecast-summary.md` are recognised as document
  paths even without a directory separator. Opaque high-entropy values and
  vendor-specific secret formats are still reported.

## 0.9.5-beta — 16 August 2026 — MCP live-heal and config unification

This window is about staying attached after an upgrade: daily `anvil`,
`anvil start`, and `anvil doctor` rewrite owned MCP configs and poke live
children. Typical upgrades keep sessions attached; a restart remains a residual
fallback. Use `anvil mcp refresh` if you need the full cascade now. Pin with
`anvil mcp pin` if you do not want auto-heal.

The same tag also unifies the project config anvil reads — one canonical file,
one key casing, and migrate/doctor paths for the older names — plus honesty
fixes so Claude's project MCP install, first-wave handshake, live-validation
status, graph search on large repos, and several checks match reality.

### Changed

- **Managed MCP install writes a PATH-stable `anvil` command.** New and
  rewritten client entries use `anvil mcp serve --stdio` instead of a versioned
  Homebrew Cellar (or similar) absolute path, so the next upgrade does not leave
  configs pointing at a deleted binary. Pass `--command` if you need a
  side-by-side override. Re-run `anvil mcp install` for a client that was
  installed the old way.

- **Project config has one canonical filename.** Fresh `anvil init` writes
  `.anvil.yaml` by default (or `.anvil.json` / `.anvil.toml` if you pick those
  formats). `.anvilrc` is still read as a fallback, but no command creates one.
  Convert with `anvil migrate format` or `anvil config convert --to yaml` (pass
  `--remove-old` to delete the source when dest is a different path).

- **Config keys are `snake_case`.** Owned writes (`anvil init`,
  `anvil config set`, migrate) emit keys such as `schema_version`. Older
  `camelCase` files still load; `anvil config show` and `anvil doctor` mention
  the legacy casing.

- **Gate composition lives in the project config.** `anvil gate-config` reads
  and writes a `gate` section in that file. A leftover `.anvil/gate-config.json`
  is no longer used by gate runs; fold it with
  `anvil migrate gate-config --apply`. A malformed `gate` section is now a loud
  error on `anvil gate` and `anvil check` instead of being skipped.

- **Architecture can sit in the project file or a pointed-to file.** You can
  keep `.anvil/architecture.yaml` and record it with
  `anvil migrate architecture --apply`, or put the definition inline. Unmigrated
  standalone files still work.

- **When both `anvil/policy.yaml` and `anvil/policy.yml` exist,
  `anvil/policy.yaml` wins.** `anvil doctor` warns when more than one policy or
  project-config variant is present and names the winner.

- **`--json` is a CLI-wide promise.** When a command accepts `--json`, success
  stdout is exactly one JSON document (or a documented machine stream such as
  watch NDJSON). Both `anvil --json <command>` and `anvil <command> --json`
  work. Human output without the flag is unchanged. Interactive-only paths
  (`anvil tutorial`) and contradictory combinations refuse with a structured
  error instead of silently printing prose.

### Added

- **Daily `anvil` / `anvil start` / `anvil doctor` heal MCP unless you pin.**
  When owned client entries, the CLI, or the daemon are stale, those paths
  rewrite the entries and poke live `mcp serve` children. `anvil mcp pin` (or
  `ANVIL_MCP_PIN`) freezes daily heal and in-process recycle; `anvil mcp unpin`
  or `ANVIL_MCP_PIN=0` resumes. First-time install on `anvil start` still works
  while pinned.

- **`anvil mcp refresh` is the emergency cascade.** It rewrites anvil-owned
  client entries to the PATH-stable `anvil` command, recycles a version-skewed
  intercept daemon, and always bumps a refresh generation so live children
  re-check on the next tool call — even when heal is pinned. `--dry-run`
  previews; `--json` prints the report. Default `--processes report` only lists
  children by parent. `--processes orphan-reap` sends SIGTERM to same-user
  orphans whose parent is gone (Unix). Children of live sessions are never
  signalled.

- **`anvil doctor` reports leftover MCP shims and `--fix` reaps them.**
  Parentless `anvil mcp serve --stdio` processes are listed; `--fix` sends
  SIGTERM to those orphans only. Live editor and agent children are left alone.

- **`anvil status` and `--verify` split attach from graph readiness.** Human and
  `--json` output can show MCP process inventory, `mcp_skew`, and separate
  `protecting` / `agent_ready` / `graph_ready` claims. A current MCP binary no
  longer implies the graph is ready. Skewed children print `anvil mcp refresh`
  as the recovery, not “restart your editor”.

- **Migration and doctor coverage for the older config names.**
  `anvil migrate format` and `anvil config convert --to` write any discovered
  project file to `.anvil.yaml` / `.yml` / `.json` / `.toml` (never `.anvilrc`).
  `--remove-old` deletes the source when the dest is a different path;
  `--stdout` on convert still prints only. `anvil migrate gate-config` and
  `anvil migrate architecture` preview by default and write with `--apply`.
  `anvil doctor` reports dual-file and legacy-key states. On a TTY (not
  `--json`, not CI or git hooks) it then offers to migrate a lone `.anvilrc`,
  remove a shadowed leftover file, fold `.anvil/gate-config.json`, or record
  `architecture.source`. A single healthy `.anvil.yaml` / `.yml` / `.json` /
  `.toml` is not prompted.

- **GitHub Actions checks see compact YAML mappings.** Trigger and `uses:` forms
  written as `{ pull_request_target: … }` or `- { uses: owner/repo@branch }` are
  no longer invisible to the supply-chain rules.

- **History secret scan covers lockfile URL credentials.** Recognised lockfiles
  such as `Cargo.lock` and `yarn.lock` are checked for credentials embedded in
  dependency URLs, instead of being skipped because the path ended in `.lock`.

- **Command-safety unwraps `env -S`.** A wrapped line such as
  `env -S "rm -rf /"` is treated as the inner command, so the same
  dangerous-command rules apply.

### Fixed

- **PY-008 human and TUI help names eval/exec/compile.** The rule-specific nudge
  already shown in JSON now appears as the finding help text instead of the
  generic Python-family suggestion (type-ignore, `import *`, `Any`).

- **Invalid project configuration exits 4.** Parse and schema failures in the
  discovered project file now use the documented configuration-error exit code
  on `check`, `gate`, `gate-config`, `watch` startup, and `architecture`.
  Runtime and missing-file failures still exit 1.

- **`anvil config convert` rewrites destination format metadata.** Converting
  YAML to JSON no longer leaves `"format": "yml"` (and the same for other
  pairs). Requested `.yml` / `.yaml` filenames are preserved; owned metadata
  uses one canonical yaml spelling. `migrate format` shares the rule.

- **`anvil watch` enforces architecture on save and reloads policy.** A new
  forbidden dependency now appears in watch the same way as
  `anvil gate --only-checks architecture`. Editing the active architecture
  source reloads or fails closed instead of keeping a stale layer.

- **MCP status does not call a missing `anvil` live protection.** A client whose
  configured command is not on that client's PATH is not `live_validation` or
  `protecting`. Status names the unresolvable command.

- **Empty-host URLs no longer skip credit-card detection.** A string such as
  `https:///accounts/4111…/events` is not treated as a safe URL path. Valid
  host-bearing HTTP(S) paths still get the path exemption.

- **Activation recovery is one story.** A missing daemon no longer both
  recommends and rejects an editor restart. Stale-image warnings print once.

- **Read-only MCP diagnostics work before you log in.** `mcp install --dry-run`,
  `mcp install --verify`, and `mcp-config --verify` (and the no-`--write`
  preview) no longer require account authentication. Real install and `--write`
  still do.

- **Claude Code project MCP install writes `.mcp.json`.** Project-scoped install
  used to write workspace `.claude.json`, which Claude does not load for MCP.
  User and local scope stay on `~/.claude.json`. Re-run
  `anvil mcp install --client claude-code` (or activation) in a project that was
  installed the old way.

- **Live-validation status is per client.** When several MCP clients are
  connected, a participating surface for one of them no longer marks the others
  as live-validated. Status now follows the client that was actually observed.

- **`anvil start --verify` handshakes every first-wave MCP client.** Codex,
  OpenCode, Gemini CLI, Grok, Warp, Zed, VS Code, and the other installable
  clients can reach the same attach/attestation ladder as Claude Code and
  Cursor. A live Grok or Codex session is no longer reported as a leftover
  Cursor/Claude restart.

- **Graph search on a large repo stays usable after a timed-out scan.** When the
  first full scan hits its time budget, tools serve the partial graph as
  ready-but-stale instead of looping on "warming". The daemon also reuses an
  existing graph-base artefact (or names a real spawn failure) instead of
  silently serving cold.

- **Windows install has a stable PowerShell short URL.**
  `irm https://install.eddacraft.ai/windows | iex` always fetches the latest
  official installer.

- **Snippet-egress consent is no longer a file in the repo.** A committed or
  checkout-controlled consent file cannot turn source-text snippets on. Consent
  lives in your user state and is bound to that workspace; use
  `anvil gctx egress enable` / `disable` as before.

- **Husky coexistence writes the hook files Git actually runs.** After the Husky
  runtime directory was missing, bootstrap could leave hooks silently dead. The
  generated `.husky/_/<hook>` entrypoints now run.

- **Commit-protection verify binds the attested range to the evidence.** A
  substituted range or an unrelated valid witness chain no longer passes as if
  it belonged to that capsule.

- **Windows `anvil update` is honest about decline and what to run next.** A
  declined update no longer exits 0 or prints both WinGet and the installer. It
  uses the same current/not-current comparison as `--check`, exits non-zero when
  it does not update, and prints only the remedy for how this copy was
  installed.

- **Credit-card checks ignore 16-digit runs in `https` URL paths.** Facebook
  reel IDs and similar path segments no longer trip the card rule. A standalone
  card number still fires.

- **Python dynamic-execution (PY-008) is more precise.** It no longer treats
  `something.compile(...)` as the builtin `compile`, and it does catch f-string
  and attribute forms that used to slip through.

- **`anvil report-fp` accepts the rule IDs `anvil check` prints.** IDs such as
  `PY-008` or `SECRET-*` record against the owning check. Unknown IDs still
  error with a suggestion.

- **A skewed intercept daemon recycles to the current CLI.** When `anvil` and
  the save-time daemon report different versions, bare `anvil`, `anvil start`,
  and `anvil mcp refresh` stop the old daemon, wait for it to exit, and start
  the current binary. Agent sessions are not restarted.

- **Long-lived `mcp serve` recycles to the preferred binary on Unix.** After an
  upgrade, the next `initialize`, `tools/list`, or `tools/call` can replace the
  child process in place so the harness pipe stays up. Windows reports the skew
  instead of re-execing.

- **Concurrent auth refresh no longer revokes your session.** Parallel `anvil`
  processes that rotate the same refresh token now take turns instead of looking
  like token theft to the server.

## 0.9.4-beta — 10 August 2026 — Clearer install advice and quieter false alarms

This patch is about trusting what anvil tells you: how you installed it, whether
registration really stuck, and fewer “secret” false alarms on ordinary file
paths — plus leaner MCP answers when a write is fine.

### Changed

- **Clean MCP allow responses are shorter by default.** When a pre-write check
  passes with nothing to report, the agent gets a small “allowed” answer instead
  of a full payload every time. Ask for full detail when you need the complete
  envelope. Checks themselves are unchanged.

### Fixed

- **Official installs on Windows and macOS report the right install method.**
  After installing with the installer, `anvil version` could still say
  `cargo install` and suggest a Rust rebuild you do not need. It now matches
  `anvil update`. Linux was already correct.

- **Windows upgrade advice is PowerShell, not a Unix pipe.** Official Windows
  installs are pointed at the PowerShell installer line (the same one
  `anvil update` prints there), not `curl … | sh`.

- **Workspace register no longer says “failed” when it actually worked.**
  Registration could succeed and then immediately look missing for a moment.
  anvil now waits briefly and checks again before giving up.

- **File paths are less likely to be flagged as secrets.** Path-like strings in
  docs or configs no longer trip the “looks like a random secret” check as
  often.

- **Commit protection notes stay tied to the commit they belong to**, so they
  are not mixed up with a different HEAD.

### Added

- **More Python dynamic-execution patterns** in reliability checks, so more “run
  code at runtime” shapes show up in audit and gate.

## 0.9.3-beta — 7 August 2026 — Honesty and Windows path

### Fixed

- **Windows PowerShell install runs again on clean machines.** The dual-install
  guard (WinGet/Scoop vs cargo-dist) was injected in a way that exited before
  the installer body ran, so `irm … | iex` could do nothing on a machine with no
  package-manager anvil. The guard now runs after cargo-dist's parameters and
  only refuses a true dual install; clean installs complete as normal.

- **Public installers no longer show private GitHub issue numbers.** Earlier
  Windows installer banners and help text embedded internal tracker ids. Anyone
  who downloaded the asset could see those numbers even though the private
  issues stay inaccessible. Ship artefacts now use neutral product wording only
  — no private issue-number branding in installer copy.

- **`anvil update --check` and install method work for cargo-dist installs.**
  Official installer receipts live under the `eddacraft-anvil` app name. anvil
  loads that layout (with a legacy `anvil` fallback), reports the install method
  honestly, and can check GitHub for updates instead of misclassifying the
  install as a plain `cargo install`.

- **Install and welcome no longer promise save-time from bare `anvil start`.**
  The post-install banner and welcome next-step copy described start as daily
  save-time protection. A bare start activates and reports state; it does not
  attach save-time to the worktree on its own. Banner and welcome now describe
  activation honestly, lead newcomers at `anvil welcome` where appropriate, and
  name save-time only where a path actually requests it (for example `--watch`).

- **Project-scoped `anvil start` can install MCP.** Interactive start with
  `--mcp-scope project` no longer claims "MCP installation disabled" solely
  because the scope is project. Consent can offer and install project-scoped
  clients like the headless path already could.

- **Start MCP copy states what was observed, not who wrote it.** Meanings that
  used to say the editor "has seen" anvil or that "anvil has written" the entry
  now say an MCP entry is present (and whether restart is still needed). The
  same honesty applies to `start --verify`, which is a non-mutating probe.

- **No false "log in again" nag for people who already have a session.**
  Ordinary beta or pro credentials are not treated as edicts. Only an explicit
  edict forces the edict re-verify path, so a transient network check no longer
  demands `anvil auth login` when you are already authenticated.

- **Workspace register no longer claims success when nothing stuck.** If
  registration does not leave a durable entry that `workspace list` can see,
  anvil says so instead of printing `Registered …` with exit 0.

- **`workspace list --json` returns JSON.** Machine-readable list output is
  honoured instead of falling back to plain text, so scripts can parse workspace
  membership reliably.

- **`anvil status` no longer creates project cache as a side effect.** Pure
  status leaves never-activated trees clean; what's-new markers write only when
  `.anvil/cache` already exists.

- **`anvil start` result screen shows one help bar and a full next step.** The
  activation verdict no longer draws a second key legend that disagreed with the
  shell bar (arrows vs `j/k`). The `next:` guidance line is promoted out of the
  tree and wraps at typical console widths so the whole next step stays
  readable.

- **Welcome autoplay failures stay in the TUI.** Auth or demo failures during
  autoplay surface recovery inside the interface instead of dumping you to a
  bare shell with little context.

- **File paths read the same way everywhere.** `check`, the pre-commit gate,
  `audit`, and `skill install` each had their own idea of how to print a
  location, so one run could show you `src/app.py`, `/.env`, and
  `\\?\C:\project\...` for the same kind of finding. Paths inside your project
  now always print relative to it, anything outside prints as an ordinary
  absolute path, and Windows paths no longer arrive with the `\\?\` prefix or
  mixed slashes.

- **No more line `0`.** `audit` reported findings about a whole file — a
  committed `.env`, say — as `.env:0`, which read like a line number counting
  from zero. Whole-file findings now show just the filename, and `--format json`
  reports `"line": null` for them. Every line number anvil prints is 1-based.

- **Secrets found outside your project keep their real path.** A secret detected
  outside the workspace could be reported with its leading `/` removed, naming a
  file that was not the one scanned.

  On Windows only, because the paths in SARIF output change, code-scanning
  alerts for secret findings are fingerprinted afresh: expect existing alerts to
  close and reappear once after upgrading.

- **Init and doctor recognise `.anvil.yaml` (and the other supported names).**
  Discovery already accepted alternate config basenames; init's already-exists
  error and doctor's probe now match, so a YAML-named config is not treated as
  missing.

- **`validate` reads one link at a time in plan indexes.** When an index line
  held an ordinary link before a module link — a design doc, then the module it
  points at — anvil read the whole span between them as a single path. Two
  things went wrong: a perfectly clean module path could be reported as broken,
  and an unsafe one could slip through, because the `..` in it was no longer a
  path component of its own. Each link destination is now read on its own, so a
  parent-directory traversal is reported wherever it appears on the line.

### Changed

- **Activation Install shows the assistants you chose.** The verdict Install
  block lists this-run outcomes for every client you selected in consent, not a
  parade of Cursor/Claude Code "not selected" / "already up to date" rows. Other
  detected clients you left alone collapse to one summary line.

- **Consent is grouped with plain-language "what is this".** Project, hooks,
  workflows, and MCP clients are separate steps with short blurbs on why anvil
  wants each write and what happens if you skip it. Selections persist across
  steps; submit is still explicit.

- **Start value receipt names its evidence scope.** When save-time or witness
  lines appear on a repeat start, they say whether the evidence is machine-wide
  or for this repository, so two machines or two clones are not mistaken for one
  picture.

- **`audit`, `gate`, and `check` say what they actually cover.** Chain and
  secret summaries disclose coverage and that the secret domain is not the same
  as a full `check` of every file type. Green `gate` / `check` results name the
  secret and antipattern file-type domains so a pass is not read as full-tree
  coverage. The same scope statement is carried through TUI, SARIF, and
  plain/JSON audit output.

- **Init first-scan copy matches the sample.** A clean first scan names the
  anti-pattern sample rather than sounding like a whole-project all-clear. When
  nothing matches the scan's language coverage, the hint says so instead of
  implying an empty repository failed.

- **Init summary names `.gitignore` when it changes ignore entries.** The plain
  and TUI success summaries list every path touched; the gitignore line is
  omitted when entries were already present.

- **`status` and `insights` name next steps and domains.** Protection:warming
  points at a next step (or refuses that label when it cannot). Zero counts in
  insights say which domain was counted so an empty score is not read as "all
  clear everywhere".

- **Open admission is disclosed, not flipped.** Where admission is intentionally
  open, surfaces say so instead of implying a closed default you never set.

- **Start flag and non-git honesty.** Combining `--no-mcp` with explicit MCP
  client flags fails loudly. `--format` warns when config already exists and the
  flag will not rewrite it. Non-git init vs worktree registration messages no
  longer contradict each other.

- **After upgrade, recovery paths are clearer.** When the daemon or MCP binary
  looks older than the CLI you just installed, status/update copy points at the
  usual recovery steps for that install method.

- **Gate copy: "blocking" means threshold, not severity=warning.** Banner
  wording no longer reads as if only low-severity "warnings" can block; it is
  findings that meet the block threshold.

- **Pre-commit gate labels pre-existing tree debt.** When the full-tree gate
  blocks on already-committed problems (for example a checked-in `.env`) while
  your staged change is clean, those findings are labeled as pre-existing so the
  current commit is not blamed for the whole yard. Full-tree scanning is
  unchanged.

- **Config-mode hooks report honestly in doctor and status.** Config-mode-only
  hooks no longer silent-Pass doctor or claim L3/L4 on. Fire is labeled
  unverified (or impossible on older Git), and file-mode remains the green
  default path.

- **Watch save-time verdicts are scoped and timed clearly.** Daemon-backed
  save-time checks state their family scope; partial or stale evidence dominates
  a clean snapshot. `--no-daemon` is a hard no-contact path. The TUI watch
  dashboard shows relative ages for queue and history so operators outside UTC
  do not misread recent work as hours stale (JSON timestamps stay absolute).

- **Architecture and CI completion copy in the TUI stays within verified
  writes.** Architecture summary names the verified config write and on-demand
  check/gate commands without claiming editor or commit-review wiring. CI
  summary no longer claims the pipeline is live after guidance-only steps.

### Docs

- **Public CLI docs match current flags and auth exit code 3.** Reference copy
  no longer implies only Claude Code and Cursor; skill install docs cover
  multi-client `--client` and moving files outside a skills directory.
- **Tutorial and antipattern-scan scope.** Non-interactive tutorial refusal
  exits non-zero with accurate copy. Antipattern-scan naming is clarified
  against the built-in rule catalogue. Tutorial autoplay keeps Escape and
  command effects inside the path picker's workspace safety chain.

## 0.9.2-beta — 3 August 2026 — MCP 2.0 reconnect

### Fixed

- **Codex and similar assistants work with anvil again.** After the last MCP
  update, some clients that send normal progress metadata were rejected as if
  they spoke a newer protocol, so tool lists and tool calls failed. Those
  clients connect and call tools again; broken modern requests still fail
  clearly.

### Added

- **Prove it from `anvil start`.** On the result screen, press `t` for
  **Prove**. anvil runs a real secret check on a sample and tells you whether it
  caught the secret — a 30-second reality check, not a placeholder toast. If
  Prove cannot run here (no secret checks configured, unsupported languages), it
  says so plainly. It never pretends your editor's live save guard is working
  when only the check engine was tested.

### Changed

- **`anvil status` and `anvil start` tell the same story.** When one says
  "warming" while the assistant path is already live (or save-time is still
  catching up), status adds one plain line of meaning so you are not left
  reconciling two different truths.

- **Cleaner keyboard help on activation.** Consent and the result screen share
  one help bar — the key list no longer fights itself.

## 0.9.1-beta — 2 August 2026 — Daily Path Polish and MCP 2.0 support

The daily minute and assistant wiring. After setup, plain `anvil` is the
on-switch; `anvil start` opens consent-first activation on a real terminal by
default; install into the assistants you already use. Browser dashboard remains
flag-gated and is not a claim of this release.

### Added

- Plain `anvil` (no subcommand) is the daily on-switch after activation —
  healthy daemon, this worktree, and already-connected assistants, without
  reinstall prompts. Use `anvil start` for first-time setup and reconfigure.
- Optional anonymous usage telemetry (opt-out). Inspect with `anvil telemetry`;
  turn off with `anvil telemetry off`, `ANVIL_TELEMETRY=off`, or
  `DO_NOT_TRACK=1`. See [anonymous usage telemetry](../operations/telemetry.md).
- `anvil mcp install --client <id>` configures twelve AI clients (Claude Code,
  Cursor, Codex, OpenCode, Gemini CLI, Antigravity, OpenClaw, VS Code, Copilot
  CLI, Grok, Warp, Zed), with `--verify` and `--dry-run`. Interactive start
  lists them with nothing ticked until you choose; scripts can use
  `--all-mcp-clients` / `ANVIL_ALL_MCP_CLIENTS`.
- Newer MCP protocol support without rewriting older client configs.
- `anvil skill install` ships the managed developer-functions skill;
  `anvil doctor` reports whether it is up to date or needs a reinstall.
- `anvil update` uses Homebrew, Scoop, or WinGet when that is how you installed,
  after explicit consent.
- `anvil lsp --stdio` exposes advisory graph context over Language Server
  Protocol for editors that prefer LSP.
- Anti-pattern scanning flags UI that is invisible until an entrance animation
  runs, and can skip generated trees via `linguist-generated` or
  `antipattern.exclude`.

### Changed

- On a real terminal, `anvil start` opens the consent-first activation screen by
  default; use `--no-tui` or `ANVIL_NO_TUI=1` for plain text.
- Warning-level anti-pattern findings no longer fail `anvil gate` unless you opt
  in with `--fail-on-warnings` or `ANVIL_FAIL_ON_WARNINGS`. Broken ciphers and
  JWT `none` still fail the gate.

### Fixed

- Remaining title-case product-name strings in user-facing copy are lowercase
  `anvil`.

## 0.9.0-beta — 12 July 2026

### Added

- `anvil welcome` provides an account-free first result on a user's own project.
- `anvil start` gives a quieter repeat-activation result and explicit protection
  state.
- `anvil insights --share` creates a reviewable local scorecard.
- Python joins the primary analysed language set.
- Read-only graph context is available to supported AI clients after explicit
  configuration.
- Infrastructure checks cover selected workflow, shell, container, and migration
  files.

### Changed

- Authentication-required action commands exit **`3`** for safer automation;
  read-only `anvil status` stays exit **`0`** with an informational envelope.
- Interactive activation choices begin unselected; continuing without a choice
  writes nothing.
- Save-time activation manages the local daemon on supported platforms while
  read-only and non-interactive commands remain non-mutating.
- Graph persistence is enabled by default for faster warm starts and stores
  structural identity rather than source text.

### Fixed and hardened

- Secret scanning reduces low-value findings in common environment and lock
  files.
- Dashboard navigation and authentication output are more consistent.
- Workspace allowlists enforce only explicitly allowed roots.
- Credential, client configuration, and daemon state handling received
  additional platform hardening.

## Earlier beta releases

Earlier releases introduced the native Rust binary, local checks and gates,
architecture boundaries, save-time watching, Git hooks, machine-readable output,
and the public installer.

For artefact-level history and checksums, use
[GitHub Releases](https://github.com/eddacraft/anvil/releases).

## Upgrade

Read the [current upgrade guide](upgrade-notes.md) before changing a scripted or
team installation.
