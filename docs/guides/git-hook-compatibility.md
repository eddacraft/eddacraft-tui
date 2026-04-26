# Git Hook Compatibility Policy

This document is the compatibility baseline and rollout policy for Git
2.54 native config-based hooks across this repository, Anvil's CLI/TUI
surfaces, and the published product. It is the artefact produced by
`GHOOK-001` (see `plans/modules/git-config-hooks.aps.md`) and is the
reference target for any hook-related guidance that follows.

> **Status:** Accepted (2026-04-26). Supersedes ad-hoc statements in
> `docs/public/anvil/tutorials/ci.md`, `docs/public/anvil/guides/agent-harness.md`,
> and the in-tree audit notes captured below.

## Why this exists

Git 2.54 added native, config-driven hook execution via `[hook.<name>]`
blocks (`hook.<name>.event`, `hook.<name>.command`, `hook.<name>.enabled`).
This lets multiple commands compose under a single hook event without
fighting over `.git/hooks/<name>` or `.husky/<name>` files. Before we
change repo tooling or extend `anvil hooks` to manage these blocks, we
need an explicit compatibility position so we do not silently raise the
floor for contributors, CI, or end users.

## Hook-dependent surfaces (audit)

`rg -n "husky|\.husky|core\.hooksPath|pre-commit|pre-push|git hook"
crates docs package.json` confirmed the following surfaces currently
assume file-based hooks. Each is in scope for later GHOOK work but is
**not** modified by GHOOK-001.

### Repo workflow

- `package.json` — `prepare: husky` script, `husky` and `lint-staged`
  devDependencies.
- `.husky/pre-commit` — invokes `lint-staged` and re-checks staged files
  via `oxfmt`.

### Anvil CLI surfaces

- `crates/anvil-cli/src/commands/hooks.rs` — installs and uninstalls
  file hooks under `.husky/` or `.git/hooks/`, with `--husky`
  auto-detection via `detect_husky`.
- `crates/anvil-cli/src/commands/status.rs` — reports `.husky/pre-commit`,
  `.husky/pre-push`, `.husky/post-merge`, and falls back to
  `.git/hooks/pre-commit`.
- `crates/anvil-cli/src/commands/doctor.rs` — checks for
  `.husky/pre-commit`, recommends `npx husky init`, and links the Husky
  troubleshooting page.

### Anvil TUI surfaces

- `crates/anvil-tui/src/surfaces/onboarding/hooks.rs` — detects `.husky/`
  and `.pre-commit-config.yaml`; husky takes priority over lefthook.
- `crates/anvil-tui/src/surfaces/tutorial/paths.rs` — pre-commit hook
  copy mentions Husky auto-detection and the `--husky` flag.
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
- `docs/public/anvil/operations/config.md` — points at `lint-staged` /
  Husky for hook setup.

The common assumption across these surfaces: **a hook is a file at a
fixed path**. Native config-hooks invalidate that assumption only when
Git itself is recent enough to honour them.

## Compatibility baseline

The minimum Git versions Anvil targets, and the fallback for older Git,
are:

| Audience           | Required Git | Why                                    | Fallback                                                        |
| ------------------ | ------------ | -------------------------------------- | --------------------------------------------------------------- |
| Repo contributors  | **>= 2.54**  | We may rely on `[hook.<name>]` locally | Run Husky bootstrap (`pnpm install`) — keeps contributors green |
| Repo CI            | **>= 2.54**  | Match contributor floor for parity     | Pin `actions/checkout@v4` runner image (already 2.54+)          |
| Anvil end users    | **>= 2.30**  | Existing public floor; do not bump yet | File-hook install (`anvil hooks install`) without `--config`    |
| Anvil end users opting into config-mode | **>= 2.54** | Required for native config-hooks       | `anvil hooks install --config` refuses with a clear error       |

`git --version` against the development runner reports `2.51.0` today;
contributors on that or older versions must update before working on
hook-touching code (GHOOK-002 onward). The bump is captured in
`package.json` via an advisory `engines.git` field — not enforced by
`pnpm` (which only enforces `node` / `pnpm`), but discoverable by tools
like `npm-check-engines` and `setup-claude-config.sh`.

This baseline is intentionally **two-tier**: the repo can move first,
end users follow only when GHOOK-002..-006 ship a working config-mode
install path and migration affordance.

## Rollout policy

The following rules apply to every change in the GHOOK module until
this doc is superseded.

### Default install mode

- `anvil hooks install` defaults to **file mode** (`.husky/<name>` if
  detected, otherwise `.git/hooks/<name>`). Default behaviour does not
  change in GHOOK-001.
- `anvil hooks install --config` (introduced in GHOOK-002) opts into
  Git 2.54 native config-hook mode. It MUST refuse to run when
  `git --version` is below 2.54 with an actionable message pointing
  here.
- `anvil init` and onboarding flows continue to recommend file mode
  until GHOOK-003 ships first-class detection.

### Coexistence

- Anvil never edits `.git/hooks/` directly to wire native hooks. All
  config-hook work goes through `git config --add hook.<name>.command`
  and friends.
- When both a `.git/hooks/<name>` (or `.husky/<name>`) file **and** a
  `hook.<name>.command` entry exist, Anvil treats this as **duplicate
  execution risk**. The CLI MUST warn (not block) by default, per
  `docs/vision/anvil-scope-guard.md` ("warnings over blocks").
- `core.hooksPath` overrides `.git/hooks/`. When set, file-mode installs
  follow it; config-mode installs are unaffected because they never
  touch the filesystem.
- Third-party managers (lefthook, pre-commit, lint-staged) keep their
  current detection logic. We do not migrate other managers' config.

### Migration trigger

Contributor migration off Husky is gated on GHOOK-005 and not initiated
by GHOOK-001. The trigger conditions are:

1. GHOOK-002 has shipped and is exercised by `cargo test --workspace`.
2. GHOOK-003 has shipped detection in doctor/status/onboarding.
3. GHOOK-004 has documented coexistence behaviour and added test
   coverage.

Until all three are met, this repo keeps Husky and `lint-staged`
exactly as today.

### Reversibility

- The `engines.git` advisory in `package.json` is informational and can
  be relaxed without coordination if we discover a contributor cohort
  on Git < 2.54 that we want to keep.
- The compatibility tier table above can be loosened (e.g. drop the
  end-user opt-in tier) without superseding this doc; tightening it
  (e.g. raising the end-user floor) requires either an ADR or a new
  GHOOK task.

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

The `rg` audit is reproduced in this doc (under "Hook-dependent
surfaces") so it does not need to be re-run to consume the policy. The
`git --version` check is intentionally a contributor responsibility —
CI does not block on it today, by design (warnings over blocks).

## Cross-references

- `plans/modules/git-config-hooks.aps.md` — module spec for GHOOK.
- `plans/execution/GHOOK-001.steps.md` — execution checklist this doc
  fulfils.
- `docs/public/anvil/operations/git-hooks.md` — user-facing summary
  that links here for the policy detail.
- `docs/public/anvil/tutorials/ci.md` — pre-commit examples; references
  this doc for the version baseline.
- `docs/public/anvil/guides/agent-harness.md` — pre-commit pattern;
  references this doc for the version baseline.
