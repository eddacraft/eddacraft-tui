# Adoption — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                                     |
| ------- | ------------- | ------ | ------ | ------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-18 as N4 doc-lane closure for v0.7.0-beta |

| Upstream                                                                                                                                                                                                                                                           | Downstream                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`plans/archive/modules/adoption-friction.aps.md`](../../plans/archive/modules/adoption-friction.aps.md) (ADOPT-001..-006), [`plans/archive/modules/adoption-trust-surface.aps.md`](../../plans/archive/modules/adoption-trust-surface.aps.md) (ADTRUST-001..-006) | [`docs/policies/resource-budget.md`](../policies/resource-budget.md), [`docs/policies/editor-coexistence.md`](../policies/editor-coexistence.md), [hook coexistence runbook](anvil-hook-coexistence.md), `anvil uninstall` |

This runbook walks an operator (and the senior engineer they support) through
the first hour, first day, and first week of using Anvil on an existing project.
It exists because the ADOPT module's success test — three internal users run
Anvil on their normal work for a calendar week and none of them disables any
check, suppresses without resolution, or bypasses a hook — fails when surprises
show up after the install screen clears.

## Audience and success criteria

**Operator audience.** Tech lead, platform engineer, or principal who is
introducing Anvil to a team that already has opinions about its tooling. You
probably already have a hook manager, a pre-commit suite, an LSP config you
tuned, and an AI coding tool wired into your editor.

**Success criteria.**

1. Existing hook manager (lefthook, husky, pre-commit-framework) keeps working
   after `anvil start`.
2. CPU and RSS stay inside the documented budget on your normal workload (see
   "Resource budget" below).
3. Editor / LSP / formatter stack runs exactly as it did pre-install (see
   "Editor coexistence" below).
4. `anvil uninstall` returns tracked files to **canonical form** — single
   trailing newline, marker-bounded blocks cleanly removed. See the
   [hook coexistence runbook](anvil-hook-coexistence.md) "Round-trip guarantee"
   section for the precise exceptions (notably: Husky files with non-canonical
   trailing whitespace are canonicalised, and user-added `extends:` / `repos:`
   entries in Lefthook or pre-commit-framework configs are not auto-removed).

If any of these break, surface it before the team-wide rollout — the ADOPT
module is owned and we want the bug, not the workaround.

## First hour: install and activate

### 1. Install the binary

Install paths are interchangeable; pick the one your platform tooling prefers:

```bash
brew install eddacraft/tap/anvil           # macOS / Linux via Homebrew
winget install eddacraft.anvil             # Windows
scoop install anvil                        # Windows scoop
sh <(curl -fsSL https://anvil.dev/install) # curl (Homebrew-aware)
```

The curl installer detects an existing Homebrew-managed binary and steps aside
with `brew upgrade eddacraft/tap/anvil` instead of overwriting it (WATCHUX-001).
WinGet / Scoop / Homebrew installs are managed by their package manager; the
curl installer manages its own install dir at `$HOME/.local/share/anvil/`.

Verify:

```bash
anvil version
```

You should see install-method-aware output naming Homebrew, WinGet, Scoop, the
installer, or "dev build" — and a recommended upgrade command for the surface
you used.

### 2. Activate the repo

In the repo root:

```bash
anvil start
```

This:

- Mints a `project_uuid` and writes `anvil/project-id`.
- Pre-positions `.gitattributes` with `merge=union -text` on
  `anvil/witness/active.ndjson`.
- Installs `pre-commit` / `post-commit` / `pre-push` / `post-merge` /
  `post-rewrite` hooks **under your existing host hook manager** if one is
  detected (lefthook, husky, pre-commit-framework), or under `.git/hooks/` if
  none is detected. See the
  [hook coexistence runbook](anvil-hook-coexistence.md) for the full
  per-framework behaviour.

> **Important — Lefthook or pre-commit-framework users:** install completes
> without error, but Anvil hooks will not actually run until you add a single
> manual `extends:` (Lefthook) or `repos:` (pre-commit-framework) entry. The
> coexistence runbook documents the exact line to add per framework. Skipping
> this leaves the install in a silent-failure state — no error at commit time,
> but pre-push will refuse for missing witnesses on the next push. Husky and
> Plain installs are fully automatic.

- Prepares the protection surfaces used by AI coding tools. The detector
  primitive for Claude Code, Cursor, Aider, Windsurf, and Codex exists, but
  `anvil start` does not yet cache those results automatically; pass
  `--tool <name>` to `anvil-run` when wrapping a launch.

### 3. Confirm the protection claim

```bash
anvil status                                       # human-readable
anvil status --json | jq '.claim.worktree_state'   # → "full"
```

The protection claim is on the plain `anvil status` flow under the `.claim` key
(`anvil status --verify` is a separate activation-diagnostic path; it does not
render the claim).

A clean activation produces a `"full"` worktree state on every render surface
(CLI status `.claim`, doctor `.protection_claim`, MCP shim, TS driver). Any
degraded surface prints a one-line reason and the remediation command — read the
line, run the command, re-run `anvil status`.

```bash
anvil doctor
```

`anvil doctor` runs the cheap presence + parse + chain-shape checks. Any failed
check prints a `fix:` line that names the next command to run.

## First day: the surfaces a senior user notices first

### Resource budget

`v0.7.0-beta` ships a CI-enforced resource ceiling: steady-state CPU ≤ 5%, peak
RSS ≤ 200 MB on the deterministic reference repository (`crates/anvil-bench`).
Measurement protocol and the pinned numbers are in
[`docs/policies/resource-budget.md`](../policies/resource-budget.md).

**Operator check.** Run `anvil watch` against a representative repo for ten
minutes during normal work and watch your activity monitor. If either ceiling is
exceeded on your machine, file the regression — the CI gate
`.github/workflows/resource-budget.yml` is supposed to catch this and a
regression past it is interesting.

### Editor coexistence

`v0.7.0-beta` runs a CI headless coexistence harness for `rust-analyzer`,
`tsserver`, `pyright`, `ruff`, `prettier`, and `eslint`. Desktop editor coverage
for VS Code, Cursor, JetBrains, and Neovim is a Boring Week/manual validation
lane. The pinned compatibility matrix is at
[`docs/policies/editor-coexistence.md`](../policies/editor-coexistence.md).

**Operator check.** Open your normal editor in a freshly-activated repo. The
LSP, formatter, and language-server stack should behave exactly as it did
pre-install. Anvil does not run a language server, does not register itself as
an LSP, and does not register a formatter. Anything in the matrix that breaks is
a coexistence bug — file it against ADOPT-006.

### Local-noise ignore policy

Watch, audit, baseline, check, drift, and gate all honour the same canonical
ignore list (kernel const, shared by every walking surface after ADOPT-004).
Coverage includes:

- Agent / tool worktree dirs: `.claude`, `.opencode`, `.gemini`, `.serena`,
  `.worktrees`
- Cache / build dirs: `node_modules`, `target`, `dist`, `.venv`, `__pycache__`,
  and others

Per-project additions go in `.anvil.<ext>` under the documented `ignore:` key.
There is no per-surface override list — the policy is canonical so an agent
worktree never gets scanned by `audit` after being correctly excluded from
`watch`.

### AI tool auto-detect

The AI-tool detector primitive enumerates installed tools without configuration.
Detection covers macOS, Linux, and Windows via documented heuristics (binary on
PATH, well-known config paths, env-var hints). The current `anvil start` command
does not write a detected-agent cache, so operator-facing wrapped launches
should pass `--tool <name>` to `anvil-run` explicitly for this release.

If your AI tool is missed, the auto-detect path is in
`crates/anvil-cli/src/activation/detect_agents.rs`. Add a heuristic and file a
PR — the surface is meant to be additive.

Detection is **purely local** — PATH probes and well-known config-path checks
only, no network calls. Air-gap operators can confirm by reading
`detect_agents.rs` directly; the call surface holds no HTTP clients.

## First week: settling in

### Hook output is one terse line

ADR-038 noise-discipline is the law: each hook emits at most one line per event.
Silent on pass; one informative line on warn; one terse refusal line on block.
If you are seeing more than that, the hook is violating policy — file it.

The companion property: `--no-verify` becomes self-defeating. Skipping the
pre-commit hook means no witness line is appended, which the pre-push hook (and
`anvil audit-chain`) refuse. The recovery for a teammate who landed unwitnessed
commits is `anvil hook bootstrap --witness-recent`, documented in the
[witness-chain runbook](anvil-witness-chain.md).

### `anvil insights` weekly summary

The insight rows stay local; a separate, narrow fleet beacon is documented in
[`docs/public/anvil/operations/telemetry.md`](../public/anvil/operations/telemetry.md):

```bash
anvil insights
```

Derived from the witness chain. Schema is pinned at `anvil.insights.v1`
(`schemas/anvil-insights.v1.json`). INSIGHTS-001 (weekly summary) ships with
`v0.7.0-beta`; the suppression-health view, drift sparkline, and first-week
adoption hint (INSIGHTS-002 / -003 / -004) are **Draft** for a follow-up release
— don't expect them in the v0.7.0-beta build.

### Update path is honest

```bash
anvil version --check
```

Advisory only — reports current version, latest version, install method, and the
one upgrade command that will actually work on your machine. Network-gated on
opt-in; default is off. Homebrew, WinGet, Scoop, and the installer all dispatch
correctly.

## Backing out: clean uninstall

```bash
anvil uninstall              # project-scoped
anvil uninstall --global     # also remove user-level state and the daemon
```

Project-scoped removal cleans `.anvil/`, `.anvilrc` (or `.anvil.<ext>`), and the
Anvil-managed git hooks (respecting hook coexistence — lefthook / husky /
pre-commit-framework entries are removed by their marker boundaries, not by
overwriting the whole file).

`--global` additionally removes `~/.anvil/`, Anvil MCP entries from
`~/.claude.json` and `~/.cursor/mcp.json`, stored credentials, and stops the
running daemon.

> **Blast radius:** the daemon is **user-scoped, not project-scoped**. Running
> `anvil uninstall --global` from one repo stops the daemon serving every other
> Anvil-enabled repo on the machine — active witness sessions in sibling
> worktrees will see the daemon disappear mid-session. Close or pause other
> Anvil work before invoking `--global`.

**The Anvil binary itself is never removed by `anvil uninstall`.** Remove it
with the install method's native command (`brew uninstall`, `winget uninstall`,
`scoop uninstall`, or remove the curl-installer path).

Auth-bypass is built in so stuck installs can be cleaned without logging in
first.

### Round-trip guarantee

For repos whose hook manager is in the supported set (Plain / Husky / Lefthook /
pre-commit-framework), an install followed by an uninstall leaves every modified
file in canonical form (single trailing `\n`, marker-bounded blocks removed
cleanly). The one documented diff: Husky files with non-canonical trailing
whitespace get canonicalised on uninstall — cosmetic, safe to commit.

User-owned merge points (Lefthook `extends:` list, pre-commit framework `repos:`
list) are **not** auto-removed. The marker block that pointed to them is
removed; the manual merge entry is left for you to delete yourself. This is by
design — Anvil does not edit user-owned config blocks on uninstall.

## What this runbook does NOT cover

- **Onboarding for users who have never used Anvil before.** The public
  install + quickstart at [anvil.dev](https://anvil.dev) and the in-product
  tutorial (`anvil welcome`) own that surface. This runbook is for operators
  after install.
- **Authoring rules.** See
  [`docs/guides/anvil-rule-authoring.md`](../guides/anvil-rule-authoring.md).
- **Daemon ops.** See
  [`docs/archive/runbooks/v0.6.0-beta-release-runbook.md`](../archive/runbooks/v0.6.0-beta-release-runbook.md)
  (carry-forward from `v0.6.0-beta`) for foreground-launch + macOS interrupt
  path, both unchanged in `v0.7.0-beta`.
- **Upgrade from a specific prior version.** See the
  [v0.6.x → v0.7.0-beta migration note](../archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md).

## Troubleshooting

**`anvil start` says hooks already exist (Husky/Lefthook/pre-commit).** That is
the expected coexistence path — Anvil installs under your host manager. Verify
with the coexistence report at the end of `anvil hooks install --config` and
read the [hook coexistence runbook](anvil-hook-coexistence.md).

**`anvil status` reports a degraded surface.** Read the one-line reason
(`anvil status` for the human-readable output, or
`anvil status --json | jq '.claim'` for the structured form). Common cases:
daemon not running (`anvil intercept start --foreground`), MCP shim
unauthenticated (`anvil auth login` — note the `anvil_validate_write` gate
returns `block` with `code: authentication-required` until then; the gate
refusal is a tooling state, not a content veto).

**Watch is CPU-noisy on my machine.** Check the resource-budget policy file —
the documented ceiling is steady-state ≤ 5%. If you are over, the regression is
interesting; capture `crates/anvil-bench` measurements with the watch-budget
bench fixture and file it against ADOPT-002.

**Editor goes red on every save.** Anvil should not register as an LSP,
formatter, or language-server. The matrix at
[`docs/policies/editor-coexistence.md`](../policies/editor-coexistence.md)
documents what is tested in CI. Anything outside the matrix is best-effort —
file it against ADOPT-006 with the editor + plugin set.

**Resource budget passing but my workflow feels heavier.** The budget gate is on
the deterministic reference repo. Real-world repos can pin a slower ceiling
under `.anvil.<ext>` (knob documented in the resource-budget policy file). The
CI gate is a floor on Anvil itself, not a ceiling on your machine.

**Witness chain refuses my push.** The pre-push hook ran `verify_chain_dag` and
found a break. Read the diagnostic line, then the corresponding failure-mode
entry in the [witness-chain runbook](anvil-witness-chain.md).

## See also

- [`plans/archive/modules/adoption-friction.aps.md`](../../plans/archive/modules/adoption-friction.aps.md)
  — ADOPT module scope and per-task evidence.
- [`docs/policies/resource-budget.md`](../policies/resource-budget.md) — pinned
  CPU / RSS ceiling and measurement protocol.
- [`docs/policies/editor-coexistence.md`](../policies/editor-coexistence.md) —
  editor / LSP / formatter compatibility matrix.
- [Hook coexistence runbook](anvil-hook-coexistence.md) — install behaviour per
  host hook manager.
- [Witness chain runbook](anvil-witness-chain.md) — chain shape and recovery
  procedures.
- [`anvil-run` manpage](anvil-run.md) — wrapped-launch ingress semantics.
- [v0.6.x → v0.7.0-beta migration note](../archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md)
  — what changes if you are upgrading.

## Provenance

- Filed 2026-05-18 as the N4 doc-lane closure for `v0.7.0-beta` (Wave 4
  release-gate evidence; see [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md)).
- ADOPT module success test (Boring Week, three internal users, no disables /
  suppressions / bypasses) is the behavioural anchor.
- Doctrine anchors: ADR-038 (Hook Surface and Noise Discipline); ADTRUST module
  (Adoption Trust Surface — closed 2026-05-14 with the protection-claim render
  contract).
