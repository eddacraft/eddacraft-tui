# CLI Surface Coherence

**Status:** Draft
**Date:** 2026-05-07
**Companion to:** [`2026-05-07-anvil-multilayer-protection-architecture.md`](./2026-05-07-anvil-multilayer-protection-architecture.md)
**APS:** Will spawn module `cli-coherence` (CLIC) plus extensions in
LAUNCH and MLP for the new commands and renames.
**ADRs cited:** ADR-038 (noise discipline), ADR-001 (planless-first),
ADR-002 (warnings over blocks).

---

## 0. What this document is

The Anvil CLI has accreted ~25 top-level commands across LAUNCH, INTD,
RMCP, RMCPF, RTAI, V050F, and earlier work. They cover the full
lifecycle (setup → daily use → diagnostic → admin). New commands from
the multi-layer protection spec (MLP) and ADRs 037/038/039 expand it
further (`anvil hook ...`, `anvil baseline`, `anvil show`).

This spec pins the **vocabulary, exit codes, verbosity, JSON
schemas, and noise-discipline pointer matrix** that keep the surface
coherent under noise rules. Without it, hooks emit pointers like
`anvil show <id>` that may resolve to inconsistent UX across
commands; tooling can't rely on `--json` schemas; users can't predict
exit codes; documentation forks per command.

The spec is reference, not radical redesign. Most existing commands
keep their names; a small number get renames or alias paths to align.

---

## 1. The four classes of CLI invocation

Every Anvil command falls into one of four classes. The class
determines what's allowed in terms of output, exit codes, latency, and
support for `--json` / `--verbose`.

| Class | Examples | Output discipline | Latency budget |
|---|---|---|---|
| **Background (called by hooks / MCP / supervisor, not user)** | `anvil hook <name>`, `anvil intercept start --foreground`, `anvil mcp serve --stdio` | Per ADR-038: silent on success, single terse line on failure, repeat-suppressed | <500ms p95 |
| **User-explicit (user invokes; UX matters; allowed informative)** | `anvil status`, `anvil doctor`, `anvil show <id>`, `anvil baseline`, `anvil audit` | Informative human-readable; `--json` available; `--verbose` for detail | <2s p95 typical |
| **Setup / onboarding (one-shot user-explicit)** | `anvil start`, `anvil init`, `anvil welcome`, `anvil new`, `anvil wizard` | Guided; multi-line acceptable; progress indicators OK | <60s budgeted |
| **Admin / privileged (rarely used; more output OK)** | `anvil admin`, `anvil auth login`, `anvil policy ...`, `anvil architecture ...` | Detailed; `--json` for audit | varies |

The rules below differ by class. A `Background`-class command MAY NOT
print informative text on success even if `--verbose`; a
`User-explicit`-class command MAY.

---

## 2. Naming pattern: verb top-level, noun-verb subsystem

Existing surface mixes patterns. Going forward:

### 2.1 Top-level verbs (what the user does)

User-explicit and setup commands stay top-level verbs:

```
anvil start          # activate
anvil status         # current state
anvil doctor         # diagnose
anvil show <id>      # explain a finding
anvil baseline       # adopt-existing
anvil audit          # L5 re-scan
anvil check          # one-shot file analysis
anvil watch          # save-time fallback
anvil init           # create config
anvil new            # scaffold project
anvil welcome        # menu / onboarding
anvil tutorial       # guided learning
anvil wizard         # interactive setup
anvil update         # self-upgrade
anvil version        # version + upgrade guidance
anvil validate       # APS plan validation
anvil drift          # architecture drift report
```

### 2.2 Noun-verb subsystems (operations on a specific subsystem)

Subsystem-specific commands group under a noun:

```
anvil intercept start | stop | status | ensure | restart    # daemon lifecycle
anvil mcp serve | install | verify                          # MCP server
anvil hook pre-commit | pre-push | post-commit | bootstrap  # git hooks
anvil auth login | logout | whoami                          # authentication
anvil gate config | run                                     # gate operations
anvil architecture list | add | remove                      # boundary definitions
anvil policy list | apply | check                           # policy management
anvil admin <subcommand>                                    # admin operations
anvil baseline show | refresh | verify                      # baseline subcommands
```

### 2.3 What changes

| Before | After | Rationale |
|---|---|---|
| `anvil mcp-config --target X` (top-level kebab) | `anvil mcp config` (subsystem) | Mcp-config is an MCP operation, not a top-level concept |
| `anvil gate-config` (top-level kebab) | `anvil gate config` (subsystem) | Same — gate operations group |
| `anvil hooks` (top-level plural) | `anvil hook` singular subsystem with subcommands | Aligns with new `anvil hook pre-commit` etc.; plural becomes hidden alias for one-release deprecation window |
| `anvil login` (hidden alias for `auth login`) | Keep as is (already hidden) | Alias for muscle memory; no change |

`anvil mcp-config` and `anvil gate-config` get **deprecation aliases**
that print one terse line on use:
`anvil: 'mcp-config' is deprecated; use 'mcp config'` then proceed
normally. Removal in a later release.

### 2.4 What does NOT change

These existing commands are correct as-is:

- `anvil start`, `anvil status`, `anvil doctor`, `anvil audit`,
  `anvil check`, `anvil watch`, `anvil init`, `anvil new`,
  `anvil welcome`, `anvil update`, `anvil drift`
- `anvil intercept ...` subsystem
- `anvil mcp ...` subsystem (with `mcp-config` folding in as
  `mcp config`)
- `anvil auth ...` subsystem
- `anvil hook ...` subsystem (replacing plural `hooks`)

---

## 3. Exit code matrix (reconciled)

Existing main.rs constants are preserved. New codes extend up.

| Code | Constant | Meaning | Used by |
|---|---|---|---|
| 0 | `EXIT_OK` | Success / no findings | All success paths |
| 1 | `EXIT_ERROR` | Generic error / unexpected failure | All error paths |
| 2 | `EXIT_GATE_FAIL` | Validation block decision (commit refused / push refused / gate failed) | `gate run`, hook commands when blocking, `audit` when findings exceed threshold |
| 3 | `EXIT_AUTH_REQUIRED` | Authentication required for command | Commands that hit Anvil cloud |
| 4 | `EXIT_CONFIG_ERROR` | Config malformed / invalid (e.g., hard-pinned class disable attempt) | Config parser |
| 5 | `EXIT_CROSS_BOUNDARY` *(new)* | `cross-boundary-detected` or `cross-boundary-mixed` | `doctor`, `status` when detected |
| 6 | `EXIT_DAEMON_DOWN` *(new)* | Daemon not running and embedded fallback unavailable | `doctor`, `status`, `intercept ensure` failure |
| 7 | `EXIT_VERSION_MISMATCH` *(new)* | `proto-version-mismatch` between CLI and daemon | `intercept ensure`, hooks when daemon proto doesn't match |
| 10 | `EXIT_DISCOVERY_FAILED` *(new)* | lstat-ladder violation; runtime dir untrusted | `doctor`, `intercept ensure` |

CI / scripts can fail-fast on `2`, `5`, `7`, `10` (real problems) and
treat `0` / `1` as soft-pass / soft-fail per their own logic.
`3`, `4`, `6` are recoverable (sign in, fix config, start daemon) and
typically not what CI guards against.

The reconciliation: I had earlier proposed `2 = cross-boundary` which
would conflict with the existing `EXIT_GATE_FAIL = 2`. Cross-boundary
moves to 5. Validation-block stays at 2 (it IS a gate failure; same
semantics as the existing `gate run` exit code).

---

## 4. Verbosity matrix

Three knobs, two from the existing surface plus one new:

| Knob | Default | Effect |
|---|---|---|
| (none) | Default | Per ADR-038 noise discipline. Background-class silent on success; user-explicit informative human-readable |
| `--verbose` / `-v` / `ANVIL_VERBOSE=1` | Off | Adds detail lines. Background-class **still silent on success** (the verbose flag adds DETAIL on existing output, not new lines on success) |
| `--quiet` / `-q` / `ANVIL_QUIET=1` *(new)* | Off | User-explicit class becomes silent on success; only errors print |
| `--json` | Off | Structured JSON to stdout; human output suppressed; `--verbose` inert when `--json` set |

`--verbose` and `--quiet` are mutually exclusive. `--json` and
`--verbose` are also mutually exclusive (verbose is for humans).

Background-class commands (hooks, daemon, MCP shim) **ignore
`--verbose`** for success output specifically — silent-on-success is
the contract. Verbose may be used internally for log file detail.

---

## 5. `--json` schema versioning

Every user-explicit command that supports `--json` emits a versioned
schema with a top-level `schema` field:

```jsonc
{
  "schema": "anvil.<command>.v1",
  // command-specific fields
}
```

Stable schemas (existing, contracted):

| Command | Schema | Notes |
|---|---|---|
| `anvil status --json` | `anvil.status.v1` | Already shipped (LAUNCH) — extends with MLP fields additively |
| `anvil intercept status --json` | `anvil.intercept.status.v1` | Already shipped (INTD-011) |
| `anvil audit --json` | `anvil.audit.v1` | Existing format; pin schema field |
| `anvil drift --json` | `anvil.drift.v1` | Existing |

New schemas (MLP-introduced):

| Command | Schema | Used for |
|---|---|---|
| `anvil doctor --json` | `anvil.doctor.v1` | CI gating; reports per-probe outcome + exit code reasoning |
| `anvil show <id> --json` | `anvil.diagnostic.v1` (existing) | Wraps the canonical `anvil.diagnostic.v1` envelope; reuses, doesn't redefine |
| `anvil baseline show --json` | `anvil.baseline.v1` | Same as the on-disk `anvil/baseline.json` file (reuse the schema) |
| `anvil baseline verify --json` | `anvil.baseline.verify.v1` | Verification outcome |
| `anvil hook bootstrap --json` | `anvil.hook.bootstrap.v1` | Recovery outcome (frameworks detected, files written, witnesses retroactively recorded) |
| `anvil project status --json` | `anvil.project.status.v1` | Aggregated rollup across surfaces (MLP project-level state) |

**Schema stability guarantee:** Field additions are minor-version
safe (consumers must ignore unknown fields). Field removals or type
changes require a new major version (`v2`). Bumps go through ADR.
MLP-009 contract test suite pins each schema with a fixture.

---

## 6. The noise-discipline pointer matrix

Every error/warning line emitted by a background-class command (hook,
daemon, MCP shim) ends with an actionable pointer to a user-explicit
command. The matrix:

| Error class | Pointer | Resolves to |
|---|---|---|
| Validation found block-level finding | `anvil show <id>` | `anvil show` looks up the diagnostic by Kindling `gate_eval_id` or `action_id`; renders the full finding with rule, file, line, suggested fix |
| Validation found warn-level finding | `anvil show <id>` | Same — `anvil show` is the universal "explain this" command |
| Daemon unreachable, embedded fallback ran | `anvil doctor` | Doctor reports daemon state, suggests `anvil intercept ensure` or `restart` |
| Embedded validation errored | `anvil doctor` | Same |
| Hook didn't fire (worktree not bootstrapped) | (no terminal output) — but L4's rejection message: `anvil hook bootstrap` | Bootstrap recovery |
| Hash chain broken | `anvil doctor --explain-chain` | Chain-specific diagnostic |
| Cross-boundary detected | `anvil doctor --explain-boundary` | Boundary-specific diagnostic |
| `proto-version-mismatch` | `anvil intercept restart` | After updating the binary |
| Witness file write failed (disk / perms) | `anvil doctor` | Doctor surfaces the FS issue |
| `degraded:fence-cascade` | `anvil unfence --review` | Operator review of recent fences |
| `degraded:baseline-suspicious` | `anvil baseline diff` | Diff view of last refresh |

Rule: **the pointer is always a command the user can run NOW that
will give them more information or fix the problem.** Never a
bare URL, never a message file path the user has to cat themselves,
never a Stack Overflow link. The CLI explains itself.

`anvil show` and `anvil doctor` are the two most-pointed-to commands;
they MUST handle every plausible input gracefully:

- `anvil show <id>` — accepts Kindling session_id / gate_eval_id /
  action_id / plan_id (auto-detects scope from id format).
- `anvil show <commit-sha>` — accepts a commit hash; renders the
  witness line for that commit.
- `anvil show <unknown>` — terse error with `anvil doctor` pointer.
- `anvil doctor` — runs all probes; renders state; never hangs;
  returns an exit code per §3.
- `anvil doctor --explain-X` — focused subset (boundary / chain /
  daemon / etc.).

Both must satisfy the User-explicit class budget (<2s p95).

---

## 7. Per-command coherence checklist

Each command must satisfy:

1. **Class assigned** (Background / User-explicit / Setup / Admin).
2. **Exit code mapped** to one of the constants in §3.
3. **Verbosity flags supported** per §4 (or documented as
   intentionally not).
4. **`--json` schema named and versioned** if applicable.
5. **Output discipline** matches its class.
6. **Help text exists** (`--help`); `clap` handles this.
7. **Pointer destinations work** (any error/warning line that names
   a command must point at one that exists and produces useful
   output for the input).

A `cli-coherence` lint runs in CI (proposed CLIC-001) that asserts:
- Every `EXIT_*` reference uses a constant, not a magic number
- Every emitted error message ending with `anvil <cmd>` resolves to a
  real command
- Every `--json` output's `schema` field matches a known versioned
  schema in a manifest

---

## 8. Specific command-by-command notes

### 8.1 `anvil start` (LAUNCH-shipped, extending for MLP)

- **Class:** Setup
- **Exit codes:** 0, 1, 4 (config error if existing config malformed),
  6 (daemon unreachable AFTER start attempt — rare)
- **`--verify`:** read-only probe (existing behaviour preserved)
- **`--watch`:** save-time fallback (existing)
- **MLP additions:** writes `anvil/project-id`, `anvil/witnessed.ndjson`
  + manifest, `anvil/policy.<format>`, `.gitattributes`, hooks, CI
  workflow
- **Output discipline:** Setup-class — multi-line OK; progress for
  the (potentially long) hook installation step

### 8.2 `anvil baseline` (MLP-007 new)

- **Class:** Setup (one-shot per repo) + User-explicit (re-runs)
- **Exit codes:** 0 (clean), 1 (scan failed), 2 (security-class
  finding refuses to grandfather)
- **Subcommands:** `baseline` (default = scan), `baseline show`,
  `baseline refresh`, `baseline verify`, `baseline diff`
- **`--scope <path>`:** partial baseline (v2)
- **`--json`:** `anvil.baseline.v1` (mirrors on-disk `anvil/baseline.json`)
- **Output:** progress line during scan; 4-5 line summary on completion

### 8.3 `anvil audit` (MLP-015 — extends existing)

- **Class:** User-explicit (on-demand) and Background (CI cron)
- **Exit codes:** 0 (clean), 2 (drift exceeds threshold), 1 (scan
  errored)
- **`--since <ref>`:** scope to commits since that ref (default:
  last audit)
- **`--threshold <n>`:** drift count threshold for exit 2
- **`--json`:** `anvil.audit.v1`
- **Note:** Existing `anvil audit` is preserved; MLP-015 extends it
  with the per-commit drift detection model.

### 8.4 `anvil doctor` (existing, extending for MLP)

- **Class:** User-explicit
- **Exit codes:** 0 (healthy), 1 (degraded surface), 2 (gate problem
  found via probes), 5 (cross-boundary), 6 (daemon down), 7 (version
  mismatch), 10 (discovery failure)
- **Subcommands / flags:**
  - `anvil doctor` — full report
  - `anvil doctor --explain-boundary` — focused on cross-boundary
  - `anvil doctor --explain-chain` — focused on witness chain
  - `anvil doctor --explain-daemon` — focused on daemon connectivity
  - `anvil doctor --reap` — clean stale runtime files (opt-in)
  - `anvil doctor --reset-suppressions` — clear repeat-suppression
    state
- **`--json`:** `anvil.doctor.v1`
- **Latency:** <2s p95; per-probe timeout (~200ms socket probe; 50ms
  fs reads); never hangs.

### 8.5 `anvil show <id>` (MLP-introduced; extends or replaces the
existing per-scope show commands)

- **Class:** User-explicit
- **Exit codes:** 0 (rendered), 1 (id not found), 4 (id format
  invalid)
- **Auto-detects scope** by id format:
  - `gate-eval-...` → GateQuery
  - `plan-...` → PlanQuery
  - `action-...` → ActionQuery
  - SHA-1/256 hex → commit-witness lookup
  - 26-char ULID → SessionQuery
- **`--json`:** wraps `anvil.diagnostic.v1` for findings; or
  Kindling response shapes for session/plan/action
- **Existing `anvil run show <id>` etc.:** keep as aliases; primary
  surface becomes `anvil show <id>`
- **Required to work in air-gapped mode** (reads local Kindling DB
  + witness file; no network)

### 8.6 `anvil hook <name>` (MLP-003..-008 new)

- **Class:** Background
- **Exit codes:** 0 (proceed), 1 (block), 2 (rare — internal error
  that should be loud)
- **Subcommands:** `pre-commit`, `pre-push`, `post-commit`,
  `post-merge`, `post-rewrite`, `bootstrap`
- **`--json`:** not for hook execution (background); `bootstrap`
  may have `anvil.hook.bootstrap.v1`
- **Output:** Per ADR-038. Silent on success. Single line on
  failure with pointer.
- **Replaces:** `anvil hooks` (plural; existing) — kept as alias
  with deprecation message during one release

### 8.7 `anvil intercept ...` (existing, extending)

- **Class:** Background (start --foreground, ensure) and Admin
  (start, stop, restart, status)
- **Subcommands:** `start`, `stop`, `restart`, `status`, `ensure`
  (new), `reap` (new)
- **`anvil intercept ensure`:** idempotent lazy launcher per
  ADR-036 §D-4
- **`anvil intercept reap`:** clean stale runtime files (per
  `--reap` flag on doctor)
- **`anvil intercept status --json`:** `anvil.intercept.status.v1`
  (existing INTD-011)
- **Output:** background discipline for ensure / start --foreground;
  user-explicit otherwise

### 8.8 `anvil project status` (MLP — new top-level alias)

- **Class:** User-explicit
- **Purpose:** project-level rollup across all surfaces / daemons /
  witnesses
- **Relationship to `anvil status`:** `anvil status` is the
  single-execution-scope view (this machine's daemon, this checkout's
  witnesses); `anvil project status` aggregates across all known
  scopes for the current `project_uuid` (uses git remote refs for
  the L4 view; local Kindling for per-machine view)
- **`--json`:** `anvil.project.status.v1`

### 8.9 `anvil mcp ...` (existing, consolidating)

- **Subcommands:** `serve`, `install`, `verify`, `config` (folded
  in from `anvil mcp-config`)
- **Background-class** for `serve`; **User-explicit** for `install`
  / `verify` / `config`
- **`mcp-config` becomes alias** with deprecation message

### 8.10 Other commands (no significant change)

- `anvil welcome`, `anvil tutorial`, `anvil wizard`, `anvil new`,
  `anvil init`, `anvil update`, `anvil version`, `anvil licenses`,
  `anvil drift`, `anvil check`, `anvil watch`, `anvil validate`,
  `anvil gate`, `anvil architecture`, `anvil policy`, `anvil admin`,
  `anvil auth`, `anvil export` — all keep current shape;
  documentation alignment work only.

---

## 9. Implementation work items

Proposed module: `cli-coherence` (CLIC). Smaller than MLP; mostly
alignment work. Items:

- **CLIC-001:** Exit code constant audit + lint. Replace magic
  numbers; add new constants (5–7, 10).
- **CLIC-002:** Verbosity flag audit + `--quiet` introduction. Pin
  per-command verbosity behaviour against §4.
- **CLIC-003:** `--json` schema manifest + per-command schema
  versioning. CI lint asserts every `--json`-emitting command names
  its schema.
- **CLIC-004:** Noise-discipline pointer audit. Every error message
  resolved to a real command; CI lint asserts.
- **CLIC-005:** `anvil show <id>` unified command (auto-detect
  scope; backward-compat aliases for existing per-scope shows).
- **CLIC-006:** Subsystem rename / alias deprecation:
  - `mcp-config` → `mcp config` (alias keeps working)
  - `gate-config` → `gate config` (alias keeps working)
  - `hooks` → `hook` (alias keeps working)
- **CLIC-007:** `anvil project status` new top-level command.
- **CLIC-008:** `anvil doctor --explain-X` subcommands.
- **CLIC-009:** `anvil intercept reap` + `--reap` doctor option
  consistency.
- **CLIC-010:** Help text consistency pass — every command's
  `--help` follows the same layout (one-line summary, when-to-use
  hint, common flags, pointer to docs).

These dovetail with MLP work — the new `anvil hook` subsystem, the
extended `anvil baseline`, the extended `anvil audit` etc. naturally
follow the coherence rules without needing CLIC-* to land first.
CLIC is alignment / lint / consistency work that runs in parallel.

---

## 10. Documentation deliverable

Single page: `docs/runbooks/cli-surface.md` — the canonical
user-facing index of every command. Format per command:

```
## anvil <command>

**Class:** <Setup | User-explicit | Background | Admin>
**Purpose:** <one line>
**When to use:** <one line>

**Synopsis:** anvil <command> [flags]

**Flags:**
  --json    structured output (schema: anvil.<name>.v1)
  --verbose more detail
  --quiet   suppress success output

**Exit codes:** 0 (ok), <list of others>

**Common errors:**
  - <error message>: try `<pointer command>`

**Examples:**
  $ anvil <example1>
  $ anvil <example2>
```

Every command in §2.1, §2.2, §8 has an entry. Auto-generated from a
single source-of-truth manifest (probably YAML) so the docs / `--help`
/ man pages don't drift.

---

## 11. What this spec does NOT do

- **Does not redesign existing commands.** Most existing commands
  keep their current shape; this spec is alignment, not rewriting.
- **Does not pin internal CLI architecture** (clap structure,
  command dispatch). Internal organisation is engineering choice,
  not user-facing contract.
- **Does not specify dashboard / TUI surfaces.** Those are separate
  surfaces with their own coherence rules.
- **Does not address i18n** — English-only for v1; localisation is
  vNext.
- **Does not address shell completions.** Should exist for v1
  (`anvil completion bash|zsh|fish`); covered by CLIC-001..-010
  collectively but not a separate work item.

---

## 12. Open questions / followups

1. **`anvil show` ambiguity when an id matches multiple scopes.**
   Mitigated by ULID prefix differences but worth a fixture test.
2. **Repeat-suppression scope** — per-session for noise discipline,
   but how does that interact with users opening many short shells
   (e.g., `git` invoking `anvil hook` from a script)? Probably need
   per-(uid, project) state in `~/.local/state/anvil/suppressions.json`.
3. **`anvil show <commit-sha>` vs `anvil log` style commands.**
   Specifically: should we have an `anvil log` for chronological
   browsing of witnesses, separate from `show <id>` for explanation?
   Probably yes, but vNext.
4. **Backward-compat window** for renamed subcommands. One release?
   Two? Removal in `v0.7.0` if v0.6.0 introduces the rename?
5. **Help text generation** — single-source-of-truth manifest format
   (YAML?) and tooling to render to `--help` / man / docs.

These are documented gaps for the future-session input file
[`2026-05-07-remaining-design-gaps.md`](../brainstorms/2026-05-07-remaining-design-gaps.md).
