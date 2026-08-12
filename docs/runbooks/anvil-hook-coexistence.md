# Hook Coexistence — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                  |
| ------- | ------------- | ------ | ------ | ------------------------------------------ |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-15 alongside ADOPT-001 |

| Upstream                                                                                                                                                                                                                                                                                | Downstream                                                                                                                                                                                                                                                     |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [ADOPT-001 in `adoption-friction.aps.md`](../../plans/archive/modules/adoption-friction.aps.md), [ADR-038](../../plans/decisions/038-hook-surface-and-noise-discipline.md) `plans/archive/modules/adoption-friction.aps.md`, `plans/decisions/038-hook-surface-and-noise-discipline.md` | [`crates/anvil-hook/src/coexistence.rs`](../../crates/anvil-hook/src/coexistence.rs), [`crates/anvil-cli/src/commands/hooks.rs`](../../crates/anvil-cli/src/commands/hooks.rs), [`docs/guides/git-hook-compatibility.md`](../guides/git-hook-compatibility.md) |

Anvil installs pre-commit, post-commit, pre-push, post-merge, and post-rewrite
hooks. When another hook manager is already present Anvil registers as a managed
entry in that manager's config instead of overwriting `.git/hooks/`. This
runbook explains what Anvil does per framework, how to verify it, and how to
recover when something goes wrong.

## Detection precedence

`anvil hook bootstrap` (and the `anvil start` install path) probes the repo root
for framework marker files in this order:

1. `.husky/` directory → **Husky**
2. `lefthook.yml`, `lefthook.toml`, or `lefthook.yaml` → **Lefthook**
3. `.pre-commit-config.yaml` or `.pre-commit-config.yml` → **pre-commit
   framework**
4. `.cargo-husky/hooks/` directory → **cargo-husky** (coexistence not supported;
   see below)
5. `core.hooksPath` set in git config → **CoreHooksPath** (coexistence not
   supported; see below)
6. Nothing matched → **Plain** (Anvil writes directly to `.git/hooks/`)

The first match wins. If you have both `.husky/` and `lefthook.yml`, Husky is
detected and Lefthook is ignored for coexistence purposes.

## Per-framework behaviour

### Plain (no manager detected)

Anvil writes shell scripts to `.git/hooks/<event>`. Each script is marked
executable. On uninstall the scripts are removed; the directory is left as-is.

### Husky

Anvil appends a marker-bounded block to `.husky/<event>` for each enabled hook
kind. If the file does not yet exist Anvil creates it with the standard Husky
shell preamble (`#!/usr/bin/env sh` + `husky.sh` source guard).

The managed block looks like:

```sh
# >>> anvil-managed (do not edit) >>>
if command -v anvil >/dev/null 2>&1; then
  anvil hook pre-commit "$@" || exit $?
fi
# <<< anvil-managed <<<
```

The `command -v anvil` guard means the hook silently skips if `anvil` is not on
PATH (e.g. a teammate who has not installed Anvil). Anvil is invoked without
`exec` so host commands after the marker still run when validation succeeds; a
non-zero status exits immediately and blocks Git.

On uninstall Anvil removes only the marker-bounded block. Surrounding user
content is preserved byte-exact for canonical input (files ending with a single
`\n`).

### Lefthook

Lefthook supports a single top-level `extends:` key and Anvil cannot safely
inject a second one. The install plan therefore has two parts:

1. **`.anvil-lefthook.yml`** (Anvil-owned) — a fully managed YAML file
   containing one stanza per hook kind:

   ```yaml
   # anvil-managed lefthook configuration.
   # Do not edit by hand — re-run `anvil hook bootstrap` to regenerate.

   pre-commit:
     commands:
       anvil:
         run: anvil hook pre-commit
   pre-push:
     commands:
       anvil:
         run: anvil hook pre-push
   ```

2. **`lefthook.yml`** (user-owned) — Anvil injects a marker-bounded comment
   block pointing to `.anvil-lefthook.yml`. The `extends:` splice is a manual
   step:

   Add `.anvil-lefthook.yml` to your `lefthook.yml` `extends:` list:

   ```yaml
   extends:
     - .anvil-lefthook.yml
   ```

   If you already have an `extends:` key, append to the list rather than
   replacing it. The comment block Anvil injects into `lefthook.yml` reminds you
   of this step.

On uninstall Anvil removes `.anvil-lefthook.yml` and its marker block from
`lefthook.yml`. The `extends:` entry you added manually is not removed — remove
it yourself if you want a clean config.

### pre-commit framework

The pre-commit framework does not support config inclusion. Anvil generates a
snippet file and requires a one-time manual merge:

1. **`.anvil-pre-commit-config.local.yaml`** (Anvil-owned) — contains a `local`
   repo entry for each hook kind:

   ```yaml
   # anvil-managed snippet for `.pre-commit-config.yaml`.
   # Do not edit by hand — re-run `anvil hook bootstrap` to regenerate.
   repos:
     - repo: local
       hooks:
         - id: anvil-pre-commit
           name: anvil hook pre-commit
           entry: anvil hook pre-commit
           language: system
           stages: [pre-commit]
           pass_filenames: false
         - id: anvil-pre-push
           name: anvil hook pre-push
           entry: anvil hook pre-push
           language: system
           stages: [pre-push]
           pass_filenames: false
   ```

2. **`.pre-commit-config.yaml`** — Anvil injects a marker-bounded comment
   pointing to the snippet. The `repos:` merge is manual:

   Copy the `- repo: local` stanza from `.anvil-pre-commit-config.local.yaml`
   into the `repos:` list in your existing `.pre-commit-config.yaml`.

On uninstall Anvil removes `.anvil-pre-commit-config.local.yaml` and its marker
block from `.pre-commit-config.yaml`. The merged `repos:` entry is not removed
automatically — remove it yourself.

### Unsupported frameworks (cargo-husky, core.hooksPath)

Anvil falls back to the **Plain** install path (direct `.git/hooks/` writes) for
these two cases. The coexistence report surfaces a warning when either is
detected alongside existing file-mode hooks. No data is lost; you may need to
reconcile hook execution order manually.

## Verifying the coexistence report

`anvil hooks install --config` and `anvil hooks uninstall --config` (the
install-time CLI; note the plural — `anvil hook` is the runtime hook-entry
namespace) both print a coexistence report. The report surfaces four signals per
hook event:

| Signal                   | Meaning                                                                                                                                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `file_mode_paths`        | Hook script files on disk for this event (`.git/hooks/`, `.husky/`, `core.hooksPath` dir). Multiple entries means duplicate-execution risk. |
| `third_party_managers`   | Managers detected repo-wide (Husky, Lefthook, pre-commit).                                                                                  |
| `foreign_config_entries` | Count of `hook.<event>.command` git-config entries not owned by Anvil.                                                                      |
| `core_hooks_path`        | Value of `core.hooksPath` if set; `None` otherwise.                                                                                         |

A healthy post-install report for a Husky repo looks like:

```
pre-commit coexistence
  file-mode paths : .husky/pre-commit
  managers        : husky
  foreign config  : 0
  core.hooksPath  : (not set)
```

## Round-trip guarantee

Install followed by uninstall returns every modified file to its canonical form
(single trailing `\n`, no leading whitespace removed). For Husky files with
non-canonical trailing whitespace (e.g. multiple trailing newlines), the
post-uninstall file is canonicalised — this is documented behaviour and will
show as a one-line diff in `git diff`.

## Troubleshooting

**`anvil hooks install` says "framework not supported"** The repo uses
cargo-husky or `core.hooksPath`. Anvil falls back to plain `.git/hooks/` writes.
Verify with `git config core.hooksPath`.

**Duplicate hook execution after install** Two hook scripts exist for the same
event. Check `file_mode_paths` in the coexistence report. Remove the extra
script or consolidate via your hook manager.

**Lefthook does not run Anvil hooks** `.anvil-lefthook.yml` exists but is not in
`lefthook.yml`'s `extends:` list. Add it manually (see Lefthook section above).

**pre-commit framework does not run Anvil hooks** The snippet was not merged
into `.pre-commit-config.yaml`. Copy the `local` repo stanza from
`.anvil-pre-commit-config.local.yaml` into your config.

**Uninstall left a stale `extends:` entry (Lefthook) or `repos:` stanza
(pre-commit)** These are user-owned and not auto-removed. Delete them manually.

**`git diff` shows a trailing-whitespace change after uninstall** The original
file had non-canonical trailing whitespace. The diff is cosmetic and safe to
commit. See the round-trip guarantee above.

## See also

- [Adoption runbook](anvil-adoption.md) — first-week operator journey that this
  runbook supports.
- [Witness chain runbook](anvil-witness-chain.md) — what the pre-commit /
  pre-push hooks Anvil installs actually write.
- [`anvil-run` manpage](anvil-run.md) — the wrapped-launch ingress that
  cooperates with these hooks.
- [v0.6.x → v0.7.0-beta migration note](../archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md)
  — what changes for hook installs across the upgrade.
- [ADR-038](../../plans/decisions/038-hook-surface-and-noise-discipline.md) —
  doctrine anchor for the entire hook surface.

## Provenance

- Filed 2026-05-15 alongside the ADOPT-001 closure.
- `See also` + Provenance metadata added 2026-05-18 alongside the N4 doc-lane
  closure so this runbook matches the structural bar set by the rest of the
  runbook set.
- Doctrine anchor: ADR-038 (Hook Surface and Noise Discipline).
- Implementation: `crates/anvil-hook/src/coexistence.rs`,
  `crates/anvil-cli/src/commands/hooks.rs`.
