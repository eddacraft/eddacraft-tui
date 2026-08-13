# Editor Surface Coexistence

| Type  | Authority     | Owner                                                                                                            | Status | Freshness                                                                                                                |
| ----- | ------------- | ---------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | ADOPT ([`plans/archive/modules/adoption-friction.aps.md`](../../plans/archive/modules/adoption-friction.aps.md)) | Live   | Last reviewed 2026-08-13 against `tools/test-harness/editor-coexistence/` and `.github/workflows/editor-coexistence.yml` |

| Upstream                                                                                                                                    | Downstream                                                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `crates/anvil-kernel/src/watcher/filter.rs` (canonical ignore set, ADOPT-004), `crates/anvil-hook/src/coexistence.rs` (host manager wiring) | `.github/workflows/editor-coexistence.yml`, `tools/test-harness/editor-coexistence/run-harness.sh` |

Anvil's adoption test is that a senior user with an editor already configured to
their taste can install Anvil, run their normal workflow for a week, and notice
no LSP misfires, no formatter races, no false-positive Anvil events caused by
editor caches, and no broken hook chain. This policy pins the behaviour Anvil
promises and the gate that prevents drift.

## What "coexistence" means here

The repository's editor surface is the set of long-lived processes the user
already runs against the working tree:

- **Language servers** — `rust-analyzer`, `tsserver`, `pyright`. These read
  source files continuously, write to their own caches, and react to file
  events.
- **Formatters / linters** — `ruff`, `prettier`, `eslint`. These run on save or
  via editor command and rewrite source files.
- **Editor processes** — VS Code, Cursor, JetBrains IDEA, Neovim. These hold
  inotify / FSEvents watches and may issue rapid sequential writes during
  refactor or format-on-save.

Anvil is also long-lived (`anvil watch`, `anvil-hook`, `anvil-run`) and walks
the same tree. Coexistence is the property that running both at once produces no
observable interaction beyond what either would produce alone.

## Coexistence Matrix (v1)

Verification key:

- **HEADLESS** — verified by the editor-coexistence harness on every PR.
- **BORING WEEK** — manual spot-check during the Boring Week dogfood window.
  Failures escalate to a HEADLESS cell or a documented exclusion.
- **N/A** — combination not exercised because the toolchain is not relevant to
  that editor's default config.

| Editor / Toolchain                                  | rust-analyzer | tsserver    | pyright     | ruff        | prettier    | eslint      |
| --------------------------------------------------- | ------------- | ----------- | ----------- | ----------- | ----------- | ----------- |
| **VS Code**                                         | BORING WEEK   | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK |
| **Cursor**                                          | BORING WEEK   | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK |
| **JetBrains IDEA / RustRover / WebStorm / PyCharm** | BORING WEEK   | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK |
| **Neovim (nvim-lspconfig)**                         | BORING WEEK   | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK | BORING WEEK |
| **headless (LSP / CLI)**                            | HEADLESS      | HEADLESS    | HEADLESS    | HEADLESS    | HEADLESS    | HEADLESS    |

The headless row is the CI gate: it spawns each language server or formatter
against the harness fixture _with `anvil watch` running on the same tree_ and
asserts both produce their expected output, both processes exit cleanly, and no
file lock / `EBUSY` / panic markers appear in either log. The finding-count
assertion is reported informationally today and tightens to a hard check once
`anvil watch --json` lands (tracked in ADOPT-006 follow-up). Failures block the
candidate.

The desktop-editor rows are manual because GUI editors are not headless-
runnable in CI without a fragile xvfb / display-server harness that would spend
more bug-hours than it would catch. Each cell is spot-checked once per Boring
Week against the same fixture used by the headless harness.

## Coexistence semantics Anvil promises

These are the invariants the harness checks and the BORING WEEK pass exists to
confirm under realistic editor load.

### Read-only on source files

`anvil watch`, `anvil audit`, and `anvil-run` open source files for reading
only. They never hold exclusive locks. A formatter writing to the same path
mid-walk is reflected on the next file-event tick, not lost or merged.

### Source-tree writes are confined to `.anvil/`

The only filesystem paths Anvil writes to in a working tree are under `.anvil/`
(plus the host's hook-manager surface per ADOPT-001). It never edits source
files, `target/`, `node_modules/`, `__pycache__/`, `.venv/`, or editor-managed
metadata.

### Editor-managed caches are ignored

The shared ignore set lives at
`crates/anvil-kernel/src/watcher/filter.rs::IGNORE_DIRS` (ADOPT-004). It already
covers the directories LSPs and editors write into without operator control:
`.git/`, `target/`, `node_modules/`, `__pycache__/`, `.venv/`, `.idea/`,
`.vscode/`, `.zed/`. New surface that walks the tree consumes that const
directly — it cannot drift.

### Hook chain is non-destructive

Anvil hooks register under the host's pre-commit manager (lefthook, husky,
pre-commit-framework) via `crates/anvil-hook/src/coexistence.rs`. They do not
overwrite `.git/hooks/*` when a host manager is detected, and `anvil uninstall`
removes only Anvil-owned entries (ADOPT-005).

### Watcher capacity is reported

`anvil watch` inspects the kernel-level inotify / FSEvents capacity at start. On
Linux it reads `/proc/sys/fs/inotify/max_user_watches` and emits a startup
warning when the headroom against the project's likely watch count is tight, so
the user can raise the limit before the editor's own watches are starved.
`anvil doctor` reports the current margin. (Enforcement — refusing to register
watches above a hard headroom — is planned but not yet wired; today this is a
warning, not a block.)

### Debounce covers typical format-on-save bursts

The default file-event debounce window is 300 ms, chosen to absorb the span of a
normal format-on-save burst from `ruff`, `prettier`, or `eslint --fix` on modern
development hardware: "format saved file" should arrive as a single Anvil event,
not five. Long-running formatters on slow hardware can still split a burst
across multiple events; users with that profile raise the window via
`anvil watch --debounce <ms>`.

## Verification protocol

`tools/test-harness/editor-coexistence/run-harness.sh` is the entry point. For
each target in `required-targets.txt` it:

1. Copies the matching deterministic fixture from
   `tools/test-harness/editor-coexistence/fixtures/<lang>/` into a scratch
   directory and runs `git init` so anvil treats it as a workspace.
2. Starts `anvil watch --source` against the scratch fixture with `ANVIL_DEV=1`,
   redirecting stdout / stderr to a per-target log, and waits a settle window
   for the initial scan to complete.
3. Invokes the per-target runner script
   (`targets/<name>.sh --run-against <scratch-repo>`) — a short non-mutating
   exercise (LSP probe, `tsc --noEmit`, `pyright`, `ruff check`,
   `prettier --check`, `eslint`) against the fixture.
4. Sends `SIGTERM` to `anvil watch`, waits for the process to exit, and computes
   the verdict cells from: the runner's exit code, the OS-level conflict markers
   (`EBUSY` / `EAGAIN` / lock-contention / `panicked`) in either log, and the
   wall-clock duration. The `anvil_events` cell is currently reported as `0` for
   visibility only; it tightens to a hard gate once `anvil watch --json` lands
   (tracked in ADOPT-006 follow-up).
5. Emits the per-target JSON verdict and a roll-up.

A target is **PASS** when the language server completes its session with exit
code 0 and no `EBUSY` / lock-contention error or panic is logged from either
side. `anvil_events` is reported in the verdict for visibility but is not yet
load-bearing — `anvil watch`'s output is human-readable today, so parsing it
would couple the harness to a moving format. When a structured
`anvil watch --json` mode lands the gate tightens to "zero false-positive
findings against the clean fixture baseline"; tracked in ADOPT-006 follow-up.

A target is **SKIP** when its binary is not on PATH on the runner. CI fails if
more than `editor-coexistence.required` targets skip — that threshold is pinned
in `tools/test-harness/editor-coexistence/required-targets.txt`.

A target is **FAIL** for any other terminal state and blocks the candidate.

The verdict JSON is uploaded as a CI artifact (`editor-coexistence-verdict`) on
every run and printed inline so headroom and skip counts are visible on green
builds.

```jsonc
{
  "schema_version": 1,
  "targets": [
    {
      "name": "rust-analyzer",
      "status": "pass" | "skip" | "fail",
      "duration_ms": 1842,
      "anvil_events": 0,
      "language_server_exit": 0,
      "notes": "" // populated on fail / skip
    }
  ],
  "required_targets_present": 6,
  "required_targets_threshold": 4,
  "threshold_met": true
}
```

`schema_version` is bumped on any field change; CI must read it before parsing.

## Boring Week protocol

For each desktop-editor row in the matrix, during the Boring Week candidate
window a maintainer:

1. Installs the candidate `anvil` binary on a clean checkout of the fixture
   repo.
2. Opens the fixture in the editor, lets the language server fully index.
3. Performs the editor-specific format-on-save burst documented at
   `tools/test-harness/editor-coexistence/manual-protocol.md`.
4. Records the result in `plans/releases/<version>-boring-week.md` under the
   editor-coexistence section. A pass is "no Anvil event fired, no editor error
   toast, format completed normally". Any other observation is a fail.

A single fail blocks the release candidate. The intent is that we discover
GUI-editor incompatibilities here, escalate them into the HEADLESS row where
feasible, and document an exclusion where not.

## Out of scope (filed separately)

- Remote-development / SSH-host configurations (file watchers behave differently
  across NFS; a separate ADOPT slice will own remote-mode if a user complaint
  appears).
- Editor-side Anvil plugins (a downstream layer; this policy is about not
  needing one).
- Windows path-length and reserved-name edge cases — covered by INTL-006.

## Cross-references

- Coexistence harness — `tools/test-harness/editor-coexistence/`
- CI gate — `.github/workflows/editor-coexistence.yml`
- Ignore-policy source of truth — `crates/anvil-kernel/src/watcher/filter.rs`
- Hook-manager coexistence — `crates/anvil-hook/src/coexistence.rs`
- APS — `plans/archive/modules/adoption-friction.aps.md` (ADOPT-006)
