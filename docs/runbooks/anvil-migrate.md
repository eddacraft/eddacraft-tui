# anvil-migrate(1)

| Type    | Authority     | Owner  | Status | Freshness                                                                 |
| ------- | ------------- | ------ | ------ | ------------------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-26 as the DISTRIB-005 `anvil migrate schema` doc lane |

| Upstream                                                                                                           | Downstream                                                                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`plans/modules/distribution-and-update.aps.md`](../../plans/modules/distribution-and-update.aps.md) (DISTRIB-005) | [`crates/anvil-cli/src/commands/migrate.rs`](../../crates/anvil-cli/src/commands/migrate.rs), [`crates/anvil-config/src/migrations.rs`](../../crates/anvil-config/src/migrations.rs) |

## NAME

**anvil migrate** — migrate Anvil configuration. Two subcommands: `format`
(legacy `.anvilrc` → multi-format `.anvil.<ext>`) and `schema` (reconcile an
existing config across anvil versions).

## SYNOPSIS

```
anvil migrate format [--format <yaml|yml|json|toml>] [--force] [--remove-old]
anvil migrate schema [--apply]
anvil migrate            # back-compat alias for `format`
```

## DESCRIPTION

`anvil migrate` has two distinct jobs, separated into subcommands so each is
explicit:

- **`format`** (shipped in `v0.7.0-beta`, MLP2-040) is a _filename and encoding_
  migration. It reads a legacy `.anvilrc` and writes the equivalent
  `.anvil.<ext>` file in the multi-format surface from MLP-011. This is a
  one-time bridge; existing `.anvilrc` projects keep working through the gate's
  fallback reader regardless.
- **`schema`** (DISTRIB-005) is a _content_ migration. It reconciles an existing
  `.anvil.<ext>` config when a newer anvil minor version changes the config
  schema, so operators do not hand-edit files to stay current.

Bare `anvil migrate` (no subcommand) runs `format` and prints a one-line notice
pointing at the explicit subcommands. Prefer the explicit form in scripts.

## `anvil migrate schema`

### How the version delta is computed

`schema` compares two versions:

- **origin** — the anvil version that created this project, read from
  `created_by_version` in `anvil/project-id` (the identity file written at
  `anvil start` / baseline time).
- **installed** — the version of the `anvil` binary you are running.

It then selects every registered schema migration introduced _after_ the origin
version and _up to and including_ the installed version, applies them
oldest-first, and (with `--apply`) writes the result back in the config's
original format via an atomic replace.

Versions follow semver pre-release ordering: a migration introduced in `0.7.0`
does **not** apply on a `0.7.0-beta` binary (`0.7.0-beta` < `0.7.0`). Beta
testers receive it once they run the stable `0.7.0` tag or later.

### Dry-run by default

`anvil migrate schema` previews changes and writes nothing. Re-run with
`--apply` to write:

```
$ anvil migrate schema
anvil: 1 schema migration(s) apply to .anvil.yaml (0.6.0 → 0.8.0):
  • 0.8.0: rename `oldkey` to `newkey`
anvil: dry-run — no changes written. Re-run `anvil migrate schema --apply` to write them.

$ anvil migrate schema --apply
anvil: migrated .anvil.yaml (0.6.0 → 0.8.0, 1 step(s) applied).
```

### Outcomes

| Situation                                                      | Result                                                                                                  |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| No migration registered for the delta (the common case today)  | Prints `no migration needed for <origin> → <installed>` and exits 0. Nothing is read or written.        |
| Origin version unknown (no `created_by_version` in project-id) | Prints manual-review guidance and exits 0. Run `anvil start` to establish identity, or review manually. |
| A migration applies but no `.anvil.<ext>` config exists        | Errors — run `anvil init` or `anvil migrate format` first.                                              |
| `created_by_version` is not valid semver                       | Errors with the offending version, so a hand-edited project-id is caught rather than silently skipped.  |

### Current state: the registry is empty

There are currently **no registered schema migrations** — no shipped anvil
version has yet changed the config schema. Every real project therefore resolves
to "no migration needed". The command, the version-delta detection, and the
dry-run/apply path are wired and tested so the seam is exercised end-to-end; a
future schema change registers its transform in
[`crates/anvil-config/src/migrations.rs`](../../crates/anvil-config/src/migrations.rs)
and `anvil migrate schema` applies it without further wiring.

## `anvil migrate format`

Convert a legacy `.anvilrc` to a multi-format config file:

```
$ anvil migrate format --format yaml
anvil: migrated .anvilrc → .anvil.yaml (legacy file kept; pass --remove-old to delete)
```

- `--format <yaml|yml|json|toml>` — target format (default `yaml`).
- `--force` — overwrite an existing `.anvil.<ext>` target.
- `--remove-old` — delete `.anvilrc` after writing the new file.

The source format is auto-detected (JSON, then TOML, then YAML). The migrated
file round-trips through `anvil-config::discover` + `parse_file`, so the gate
reads it back identically.

## EXIT STATUS

`0` on success, including the benign "no migration needed" and "origin version
unknown" outcomes. Non-zero on I/O errors, a malformed version, an unsupported
`--format`, or a `schema` migration with no config to apply to.

## SEE ALSO

`anvil-init(1)`, `anvil-start(1)`,
[`docs/policies/release-cadence.md`](../policies/release-cadence.md).
