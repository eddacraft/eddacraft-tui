# Watch Warm-Up and Advisory Rule Modes

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | MLP / Activation UX follow-up | Draft | Captured 2026-05-13 from beta-user incident screenshots and `crates/anvil-cli/src/commands/{audit,start,watch}.rs`, `crates/anvil-kernel/src/watch.rs` |

| Upstream | Downstream |
| -------- | ---------- |
| Beta-user incident timeline, `install.sh`, `crates/anvil-cli/src/commands/audit.rs`, `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/commands/watch.rs`, `crates/anvil-kernel/src/watch.rs`, `crates/anvil-kernel/src/watcher/filter.rs` | Future APS module for watch warm-up UX, config/rule-mode work, public docs for first-run activation |

## Context

A macOS beta user exercised Anvil's first-run path in a large repository and hit
several related but distinct UX failures. The observed timeline was:

1. User already had Anvil installed via Homebrew.
2. User ran the curl installer while trying to upgrade.
3. The curl installer ran the standalone installer and only reported a shadowed
   PATH / install conflict at completion.
4. User remembered the Homebrew install and upgraded through `brew`.
5. User ran `anvil audit` and saw many `.env` findings, including files under
   `.claude/worktrees/...`.
6. User ran `anvil start`.
7. User ran `anvil watch`; it appeared to do nothing.
8. About ten minutes later the watch pane appeared.
9. User noticed `anvil - anvil_validate_write (MCP)` showing in their BMAD agent
   workflow.
10. About one hour later the watch pane flashed a failing state with messages
    like `new public symbol 'buildExcelExport' expands API surface`.

Screenshots from the incident showed:

- `audit`: project `core`, `82,871` files, `22,642` current issues, high-severity
  `.env` findings under `.claude/worktrees/...`.
- `watch` hang: terminal only showed the `anvil watch` command.
- initial `watch` pane: `0 files watched`, idle, no pending changes.
- later `watch` pane: `27,374 files watched`, status `Failing`, many API-surface
  and dependency diagnostics.

## Root Cause Model

### Installer Upgrade Path

The curl installer did not detect an existing Homebrew `anvil` before running
the standalone installer. On macOS, Homebrew commonly exposes `anvil` as:

- `/opt/homebrew/bin/anvil` on Apple Silicon
- `/usr/local/bin/anvil` on Intel
- a symlink into `Cellar/anvil/.../bin/anvil`

The installer should detect that state before download/install and tell the user
to use `brew upgrade eddacraft/tap/anvil`.

### Audit Noise

`audit` intentionally does not apply `.gitignore`, because security scanning
must not hide committed or local secret files only because they are ignored by
Git. However, it still needs a built-in noise boundary for generated artefacts,
cache directories, and local agent worktree/tool state.

The beta repo contained `.claude/worktrees/...` inside the project root. Scanning
those directories multiplied file counts and surfaced `.env` files that belong
to local agent execution state rather than the user's project source.

### Watch Startup Silence

`anvil watch` performed expensive setup before the TUI could show useful state:

- directory discovery
- watcher registration
- initial parse / symbol graph build

In a large repo, especially one containing local worktree/tool-state directories,
that made the command look hung. The user had no progress indication until the
watch pane appeared.

### Initial Snapshot False Failures

The watch initial scan built the graph, then evaluated every scanned file as a
delta containing `added_symbols`. Policies such as `public-api-expansion` then
interpreted existing exported symbols as new API surface.

That is why the user saw messages like:

```text
new public symbol 'buildExcelExport' expands API surface
```

Those were not necessarily caused by a new edit. They were initial-snapshot
false positives.

## Product Principles

### Watch Opens Immediately

`anvil watch` should enter the TUI immediately. Slow setup must be visible as a
state, not experienced as a hung terminal.

Recommended states:

| State | Meaning |
| ----- | ------- |
| `Starting` | command launched, workspace/config resolving |
| `Warming up` | file discovery, watcher registration, or graph build is in progress |
| `Watching` | ready and monitoring for future changes |
| `Warnings` | advisory findings exist |
| `Action failed` | a configured action such as `gate` failed |
| `Blocked` | an enforced policy prevented an operation |
| `Error` | watcher setup/runtime failure |

### Initial Scan Is Baseline, Not Failure

The initial scan should build state and emit a readiness snapshot. It must not
emit API-surface/dependency findings as if the existing repo was a new change.

Future file changes after the initial ready point should still be evaluated and
shown.

### Findings Are Advisory by Default

Default watch findings should not imply enforcement. In particular:

| Rule | Default mode |
| ---- | ------------ |
| public API expansion | warn |
| new external dependency | warn |
| privilege/trust expansion | warn |
| cross-layer import | warn unless architecture config marks it enforced |
| parse/read error | error, not policy failure |
| `--action gate` failure | action failed |
| MCP validate-write block | blocked |

`Failing` is too strong for advisory diagnostics. It should be reserved for a
failed configured action or enforced policy outcome.

### Configuration Must Be Visible

If the user has only run `anvil start`, they need to know what was configured.
The start output should include a concise config summary, for example:

```text
config: .anvilrc created with default advisory rules

rules:
  public API expansion: warn
  new dependency: warn
  cross-layer import: warn
  privilege expansion: warn

edit .anvilrc to change a rule from warn to enforce
```

The exact config format may remain the existing `.anvilrc` format initially, but
rule modes should be explicit in configuration rather than implicit in code.

Example TOML shape for a future config format:

```toml
[rules.public_api_expansion]
mode = "warn"

[rules.new_dependency]
mode = "warn"

[rules.cross_layer]
mode = "warn"

[rules.privilege_expansion]
mode = "warn"
```

Valid modes should be narrow and predictable:

- `off`
- `warn`
- `enforce`

Optional later extension:

- `error` for non-blocking but high-severity diagnostics

## Config Format Commands

`anvil start` should not switch config format. It should remain an activation /
probe command and should be idempotent.

Do not add `anvil start --toml` as a config conversion mechanism.

Preferred command model:

```bash
anvil init --format toml
anvil config show
anvil config convert --to toml
anvil config set rules.public_api_expansion.mode enforce
```

If a user tries a future `anvil start --toml`, reject it with guidance rather
than silently converting:

```text
--toml only applies to anvil init; existing config is .anvilrc.
Run anvil config convert --to toml to migrate.
```

## Warm-Up Design

### Trigger Points

First-run actions may start a background warm-up:

- `anvil start`
- `anvil welcome`
- `anvil init`
- `anvil tutorial`
- possibly `anvil audit`

Recommended rollout:

1. Start with `anvil start`, because it is already activation-oriented and writes
   Anvil state.
2. Extend to `welcome` and `tutorial` once the cache contract is proven.
3. Let `audit` reuse file-discovery state if useful, but do not require it to
   reuse the symbol graph.

### Warm-Up Outputs

Warm-up should cache or prepare:

- discovered file count
- watchable directory count
- ignored directory policy version
- language profile summary
- initial symbol graph or graph seed
- architecture/config hash
- Anvil version
- timestamp

The cache is a startup accelerator only. It is not security authority.

Possible cache path:

```text
.anvil/cache/watch-warmup.json
```

### Invalidation

The warm-up cache must be invalidated when any of these change:

- Anvil version
- config hash
- architecture config hash
- ignored-directory policy version
- relevant parser/schema version
- cache age threshold
- repo root / project identity

`watch` should always reconcile with the filesystem after opening, even when a
warm-up cache exists.

### Watch TUI Progress Events

The kernel/watch layer should emit coarse startup progress before fine-grained
progress exists:

```text
watch_setup_started
watch_setup_progress { phase, completed, total? }
initial_scan_started
initial_scan_progress { files_done, files_total? }
initial_scan_complete { files_watched, files_scanned }
watch_ready
```

TUI rendering should use existing visual affordances where possible:

- spinner when total is unknown
- progress bar when total is known
- phase label such as `Discovering files`, `Registering watchers`,
  `Building graph`, `Ready`
- large-repo hint if warm-up exceeds a short threshold, such as 10 seconds

Changes during warm-up should be queued and replayed after the initial graph is
ready, or explicitly ignored with a clear rescan. Silent loss is unacceptable.

## Implementation Slices

### Slice 1: Beta Hotfix

Goal: remove the most visible false failures and noise without a new cache
contract.

Expected changes:

- Installer detects Homebrew before running standalone install.
- `audit` skips generated/cache/local-agent worktree directories.
- `watch` prints immediate startup feedback before blocking setup.
- `watch` initial scan builds graph and emits snapshot but not violations.
- TUI stops mapping advisory diagnostics to `Failing` once rule modes exist.

### Slice 2: Progressive Watch Startup

Goal: open watch immediately and show warm-up state inside the TUI.

Expected changes:

- Move watcher setup and initial scan behind progress events.
- Add `Starting` / `Warming up` / `Watching` display states.
- Show spinner/progress bar and phase text.
- Preserve queued changes during warm-up.

### Slice 3: Configurable Rule Modes

Goal: make advisory vs enforced behaviour explicit and user-controlled.

Expected changes:

- Add rule-mode config schema.
- Render config summary from `anvil start` / `anvil status`.
- Map `warn` findings to `Warnings`, not `Failing`.
- Reserve `Failing`, `Blocked`, and `Action failed` for enforced outcomes.
- Add `anvil config show`, `anvil config set`, and eventually config conversion.

### Slice 4: Warm-Up Cache

Goal: reuse first-action preparation so later `watch` opens quickly.

Expected changes:

- Write `.anvil/cache/watch-warmup.json` from `start`.
- Add invalidation keys.
- Make `watch` consume warm-up opportunistically.
- Reconcile after opening.

## Open Questions

1. Should all local tool-state directories be ignored by default for security
   scans, or should `audit --include-tool-state` exist for users who want to
   inspect agent state?
2. Should `cross-layer` default to `warn` even when explicit architecture layers
   exist, or should architecture config be treated as opt-in enforcement?
3. Should public API expansion warnings be scoped to package boundaries, export
   files, or all public symbols?
4. Should warm-up be one shared cache for `audit`, `watch`, and `status`, or a
   watch-specific cache first?
5. What is the maximum acceptable warm-up time before the UI should suggest
   narrowing scope with `--source`, `--patterns`, or config excludes?
