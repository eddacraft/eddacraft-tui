# Git Hook Compatibility Policy

| Type  | Authority     | Owner | Status | Freshness                                                                                                      |
| ----- | ------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | GHOOK | Live   | Last reviewed 2026-05-25 against `plans/archive/modules/git-config-hooks.aps.md` and Git 2.54 config-hooks API |

| Upstream                                                                                                                                       | Downstream                                                                                                      |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `plans/archive/modules/git-config-hooks.aps.md`, `crates/anvil-cli/src/commands/hooks.rs`, `crates/anvil-tui/src/surfaces/onboarding/hooks.rs` | `docs/public/anvil/tutorials/ci.md`, `docs/public/anvil/guides/agent-harness.md`, hook-related CLI/TUI guidance |

This document is the compatibility baseline and rollout policy for Git 2.54
native config-based hooks across this repository, Anvil's CLI/TUI surfaces, and
the published product. It is the artefact produced by `GHOOK-001` (see
`plans/modules/git-config-hooks.aps.md`) and is the reference target for any
hook-related guidance that follows.

> **Status:** Accepted (2026-04-26). Supersedes ad-hoc statements in
> `docs/public/anvil/tutorials/ci.md`,
> `docs/public/anvil/guides/agent-harness.md`, and the in-tree audit notes
> captured below.

## Why this exists

Git 2.54 added native, config-driven hook execution via `[hook.<name>]` blocks
(`hook.<name>.event`, `hook.<name>.command`, `hook.<name>.enabled`). This lets
multiple commands compose under a single hook event without fighting over
`.git/hooks/<name>` or `.husky/<name>` files. Before we change repo tooling or
extend `anvil hooks` to manage these blocks, we need an explicit compatibility
position so we do not silently raise the floor for contributors, CI, or end
users.

## Hook-dependent surfaces (audit)

`rg -n "husky|\.husky|core\.hooksPath|pre-commit|pre-push|git hook" crates docs package.json`
confirmed the following surfaces currently assume file-based hooks. Each is in
scope for later GHOOK work but is **not** modified by GHOOK-001.

### Repo workflow

- `package.json` — `prepare: husky` script, `husky` and `lint-staged`
  devDependencies.
- `.husky/pre-commit` — invokes `lint-staged` and re-checks staged files via
  `oxfmt`.

### Anvil CLI surfaces

- `crates/anvil-cli/src/commands/hooks.rs` — installs and uninstalls file hooks
  under `.husky/` or `.git/hooks/`, with `--husky` auto-detection via
  `detect_husky`.
- `crates/anvil-cli/src/commands/status.rs` — reports `.husky/pre-commit`,
  `.husky/pre-push`, `.husky/post-merge`, and falls back to
  `.git/hooks/pre-commit`.
- `crates/anvil-cli/src/commands/doctor.rs` — checks for `.husky/pre-commit`,
  recommends `npx husky init`, and links the Husky troubleshooting page.

### Anvil TUI surfaces

- `crates/anvil-tui/src/surfaces/onboarding/hooks.rs` — detects `.husky/` and
  `.pre-commit-config.yaml`; husky takes priority over lefthook.
- `crates/anvil-tui/src/surfaces/tutorial/paths.rs` — pre-commit hook copy
  mentions Husky auto-detection and the `--husky` flag.
- `crates/anvil-tui/src/surfaces/doctor/{mod,render}.rs` — surface the
  `npx husky init` remediation.
- `crates/anvil-tui/src/surfaces/status/{mod,render}.rs` — surface
  `.husky/pre-commit` and `.husky/commit-msg` as canonical locations.
- `crates/anvil-tui/examples/demo.rs` — demo data references
  `.husky/{pre-commit,pre-push,post-merge}`.

### Public docs

- `docs/public/anvil/tutorials/ci.md` — pre-commit example uses
  `.husky/pre-commit (or .git/hooks/pre-commit)`.
- `docs/public/anvil/guides/agent-harness.md` — pre-commit pattern uses
  `.git/hooks/pre-commit`.
- `docs/public/anvil/operations/config.md` — points at `lint-staged` / Husky for
  hook setup.

The common assumption across these surfaces: **a hook is a file at a fixed
path**. Native config-hooks invalidate that assumption only when Git itself is
recent enough to honour them.

## Compatibility baseline

The minimum Git versions Anvil targets, and the fallback for older Git, are:

| Audience                                | Required Git | Why                                    | Fallback                                                                                                                                |
| --------------------------------------- | ------------ | -------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Repo contributors                       | **>= 2.54**  | We may rely on `[hook.<name>]` locally | Run Husky bootstrap (`pnpm install`) — keeps contributors green                                                                         |
| Repo CI                                 | **>= 2.54**  | Match contributor floor for parity     | Use a runner image with Git >= 2.54 (current `ubuntu-24.04` image already ships ≥ 2.54), or install/upgrade Git as part of the workflow |
| Anvil end users                         | **>= 2.30**  | Existing public floor; do not bump yet | File-hook install (`anvil hooks install`) without `--config`                                                                            |
| Anvil end users opting into config-mode | **>= 2.54**  | Required for native config-hooks       | `anvil hooks install --config` refuses with a clear error                                                                               |

`git --version` against the development runner reports `2.51.0` today;
contributors on that or older versions must update before working on
hook-touching code (GHOOK-002 onward). The bump is captured in `package.json`
via an advisory `engines.git` field — not enforced by `pnpm` (which only
enforces `node` / `pnpm`), but discoverable by tools like `npm-check-engines`
and `setup-claude-config.sh`.

This baseline is intentionally **two-tier**: the repo can move first, end users
follow only when GHOOK-002..-006 ship a working config-mode install path and
migration affordance.

## Rollout policy

The following rules apply to every change in the GHOOK module until this doc is
superseded.

### Default install mode

- `anvil hooks install` defaults to **file mode** (`.husky/<name>` if detected,
  otherwise `.git/hooks/<name>`). Default behaviour does not change in
  GHOOK-001.
- `anvil hooks install --config` (introduced in GHOOK-002) opts into Git 2.54
  native config-hook mode. It MUST refuse to run when `git --version` is below
  2.54 with an actionable message pointing here.
- `anvil init` and onboarding flows continue to recommend file mode until
  GHOOK-003 ships first-class detection.

### Coexistence

- Anvil never edits `.git/hooks/` directly to wire native hooks. All config-hook
  work goes through `git config --add hook.<name>.command` and friends.
- When both a `.git/hooks/<name>` (or `.husky/<name>`) file **and** a
  `hook.<name>.command` entry exist, Anvil treats this as **duplicate execution
  risk**. The CLI MUST warn (not block) by default, per
  `docs/vision/anvil-scope-guard.md` ("warnings over blocks").
- `core.hooksPath` overrides `.git/hooks/`. When set, file-mode installs follow
  it; config-mode installs are unaffected because they never touch the
  filesystem.
- Third-party managers (lefthook, pre-commit, lint-staged) keep their current
  detection logic. We do not migrate other managers' config.

### Migration trigger

Contributor migration off Husky is gated on GHOOK-005 and not initiated by
GHOOK-001. The trigger conditions are:

1. GHOOK-002 has shipped and is exercised by `cargo test --workspace`.
2. GHOOK-003 has shipped detection in doctor/status/onboarding.
3. GHOOK-004 has documented coexistence behaviour and added test coverage.

Until all three are met, this repo keeps Husky and `lint-staged` exactly as
today.

### Reversibility

- The `engines.git` advisory in `package.json` is informational and can be
  relaxed without coordination if we discover a contributor cohort on Git < 2.54
  that we want to keep.
- The compatibility tier table above can be loosened (e.g. drop the end-user
  opt-in tier) without superseding this doc; tightening it (e.g. raising the
  end-user floor) requires either an ADR or a new GHOOK task.

## Repo Husky migration decision (GHOOK-005)

> **Status:** Accepted (2026-04-27). Recommendation: **Option A — keep Husky as
> the contributor bootstrap.** Revisit when the trigger conditions below flip.

GHOOK-001 deferred the keep/replace call on Husky until GHOOK-002, GHOOK-003,
and GHOOK-004 had landed. GHOOK-002 has shipped (PR #1119); GHOOK-003 and
GHOOK-004 are in review (PRs #1122 and #1125). GHOOK-005 is the decision task
that closes the loop. This section captures the decision so the next change to
repo hook tooling is a mechanical flip rather than a re-litigation.

### The two real options

- **Option A — Keep Husky as the contributor bootstrap.** `pnpm install` keeps
  invoking `prepare: husky`, which installs Husky's hook-execution wiring and
  activates the tracked `.husky/pre-commit` hook (via Husky's setup /
  `core.hooksPath`) so `lint-staged` plus the post-stage `oxfmt` check run
  exactly as today. Native config-mode hooks are still available to power-users
  via `anvil hooks install --config`, run after `pnpm install`, but they are not
  the default repo posture.
- **Option B — Replace Husky entirely.** `.husky/` is removed. Repo pre-commit /
  pre-push hooks live as `hook.<event>.command` entries materialised by a
  contributor-run bootstrap step (`pnpm setup:hooks` or
  `scripts/setup-git-hooks.sh`) that wraps `anvil hooks install --config` (or
  the equivalent raw `git config --add hook.<event>.command ...` writes). The
  `prepare` script is replaced or chained so `pnpm install` still ends with a
  working hook chain on contributor machines that meet the Git floor.

### Criteria the decision turns on

- **(a) Contributor Git version distribution.** The repo declares a contributor
  floor of Git ≥ 2.54 in `engines.git` and in this doc's tier table, but the
  floor is advisory — `pnpm` only enforces `node` and `pnpm`. The development
  runner used for this work reports `git --version` = `2.51.0` _today_. Option B
  hard-requires every contributor (and every fresh clone, including CI base
  images that have not yet been bumped) to be on Git ≥ 2.54 before the bootstrap
  step succeeds. Option A degrades gracefully on Git < 2.54 because Husky's
  file-mode hooks work on every Git version we support, and the config-mode
  opt-in already refuses below 2.54 with a clear error per GHOOK-002.
- **(b) `lint-staged` integration.** Husky's value to _this_ repo is the
  `lint-staged` glue, not the hook execution itself. Native config-mode hooks
  invoke commands directly and have no notion of staged-file filtering; Option B
  keeps `lint-staged` available and replaces the `.husky/pre-commit` shell
  wrapper with an equivalent `hook.pre-commit.command` such as
  `pnpm exec lint-staged && <oxfmt staged check>`. That is mechanically
  straightforward but moves the executable surface from a tracked shell file
  (reviewable in PRs) to a `git config` entry materialised by a script
  (reviewable only via the bootstrap script and its smoke test).
- **(c) Post-clone bootstrap UX.** Option A ships zero new contributor steps:
  `pnpm install` triggers `prepare: husky`, which is universal and runs on every
  clone. Option B requires either chaining `pnpm install` → `pnpm setup:hooks`
  via the existing `prepare` script, or documenting a separate
  `pnpm setup:hooks` invocation. The chained variant has the same UX as today;
  the separate variant adds a contributor footgun (forgetting the second step
  yields a repo with no pre-commit at all).
- **(d) Reversibility.** Option B is reversible: reinstating the
  `prepare: husky` script and the `.husky/pre-commit` file (both already tracked
  in git history) restores file-mode hooks. Option A is also reversible — the
  work to migrate later is small once the Git floor is met across contributors.
  Neither direction is a one-way door.

### Recommendation

**Option A — Keep Husky as the contributor bootstrap.** The decisive factor is
criterion (a): the dev runner is on Git 2.51 today, which is below the
contributor floor declared by GHOOK-001 and below the floor Option B requires to
function. Anvil's user-facing posture is Git ≥ 2.30, the repo's contributor
floor is advisory ≥ 2.54, but the operative number is 2.51 _on the machine
shipping this work right now_. Migrating off Husky before that gap closes would
either break the shipping runner (Option B refuses on 2.51) or force a same-PR
runner upgrade that is out of scope for a hook-tooling decision. Husky's
`lint-staged` glue (criterion b) and zero-step bootstrap (criterion c) are
secondary but reinforcing reasons; reversibility (criterion d) means picking A
costs us nothing except a future small migration when the contributor Git
distribution catches up.

We will revisit and flip to Option B when **all three** of the following are
true:

1. The dev runner image and contributor baseline both report Git ≥ 2.54 (i.e.
   the advisory `engines.git` becomes the operational reality, not just the
   declared floor).
2. The `.husky/pre-commit` script's `lint-staged` + post-stage oxfmt logic has a
   documented config-mode equivalent that has been smoke-tested in CI.
3. GHOOK-006 docs for native hooks are landed so contributors have a single
   reference for the new bootstrap.

### Mechanical scaffolding for Option A

No structural change is required: `package.json` keeps `prepare: husky`, the
`husky` and `lint-staged` devDependencies, and `.husky/pre-commit` exactly as
they are. The only addition is contributor guidance, captured here:

> Contributors who want to test config-mode hooks locally can run
> `anvil hooks install --config` _after_ `pnpm install`. This is additive and
> does not remove the file-mode hooks Husky installed; the resulting duplicate
> execution is reported by `anvil hooks status` (per GHOOK-004) so it is visible
> rather than silent.

This guidance is also referenced from `package.json` via a top-level `huskyNote`
string sibling to `engines` and `scripts`, so a contributor reading
`package.json` discovers the opt-in path and the link to this guide without
having to open it first.

### Out-of-scope guard

`.husky/` is **not** removed in this PR. If we ever flip to Option B, the
`.husky/` removal must be its own PR (or its own commit, called out explicitly
in the PR description) so the change is auditable and revertible in isolation.
Bundling `.husky/` removal with anything else is forbidden by this decision.

### How a contributor reproduces the chosen setup from a fresh clone

1. `git clone …`
2. `pnpm install` — runs `prepare: husky`, installs Husky and wires Git to run
   the tracked `.husky/pre-commit` hook.
3. (Optional) `anvil hooks install --config` — additionally wires
   `hook.pre-commit.command` for power-user testing of native config-mode.
4. (Optional) `anvil hooks status` — confirms the hook chain Anvil sees.

No other steps are required. CI inherits the same path because the CI image runs
`pnpm install` before any hook-relevant target.

## Testing this policy

Step validations from `plans/execution/GHOOK-001.steps.md`:

```bash
# Step 1 — Audit current hook assumptions.
rg -n "husky|\.husky|core\.hooksPath|pre-commit|pre-push|git hook" \
  crates docs package.json

# Step 2 — Confirm the contributor's Git version against the baseline.
git --version

# Step 4 — Lint published Markdown.
pnpm lint:md
```

The `rg` audit is reproduced in this doc (under "Hook-dependent surfaces") so it
does not need to be re-run to consume the policy. The `git --version` check is
intentionally a contributor responsibility — CI does not block on it today, by
design (warnings over blocks).

## Cross-references

- `plans/modules/git-config-hooks.aps.md` — module spec for GHOOK.
- `plans/execution/GHOOK-001.steps.md` — execution checklist this doc fulfils.
- `docs/public/anvil/operations/git-hooks.md` — user-facing summary that links
  here for the policy detail.
- `docs/public/anvil/tutorials/ci.md` — pre-commit examples; references this doc
  for the version baseline.
- `docs/public/anvil/guides/agent-harness.md` — pre-commit pattern; references
  this doc for the version baseline.
