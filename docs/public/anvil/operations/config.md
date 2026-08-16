---
id: config
title: Configuration reference
description:
  Inspect, change, convert, and review anvil project configuration safely.
owner: UCFG
upstream:
  - crates/anvil-config/src/discover.rs
  - crates/anvil-config/src/format.rs
  - crates/anvil-cli/src/commands/config.rs
verified_against: 0.9.4-beta
---

# Configuration reference

The canonical project configuration file is `.anvil.<ext>` in the project root,
where the extension names the format: `.anvil.yaml` (the default), `.anvil.yml`,
`.anvil.json`, or `.anvil.toml`. `anvil init` writes `.anvil.yaml` by default
(the interactive wizard can pick JSON or TOML, which write `.anvil.json` /
`.anvil.toml`). Keys are `snake_case` (for example `schema_version`,
`planning_dir`).

`.anvilrc` is the legacy name. It is still read as a fallback everywhere, but no
command creates one; convert it with `anvil migrate format` or
`anvil config convert --to yaml`.

## Migration bridges

| Situation                                    | Bridge                                                                                                                                                                                                                                                                              |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Legacy `.anvilrc` file                       | `anvil migrate format` or `anvil config convert --to yaml` writes `.anvil.yaml` with `snake_case` keys (add `--remove-old` to delete the source file)                                                                                                                               |
| Legacy `camelCase` keys (`schemaVersion`, …) | Still accepted on read; any owned write (`anvil config set`, `anvil migrate format`) rewrites them. (`anvil migrate schema --apply` applies registered migrations by version delta — the casing migration fires for pre-`0.10.0-beta` projects once running `0.10.0-beta` or later) |
| Retired `.anvil/gate-config.json`            | Ignored by gate runs; `anvil migrate gate-config --apply` folds it into this file and removes it                                                                                                                                                                                    |
| Standalone `.anvil/architecture.yaml`        | Still valid as a legacy fallback; `anvil migrate architecture --apply` records it as the `architecture.source`                                                                                                                                                                      |

`migrate schema`, `migrate gate-config`, and `migrate architecture` preview by
default and only write with `--apply`; `migrate format` writes immediately (it
refuses to overwrite an existing `.anvil.<ext>` without `--force`).
`anvil doctor` reports the legacy states above: its `config-variants` check
warns when more than one config file exists (naming the winner under
discover-first precedence), and legacy `camelCase` keys surface as a deprecation
note on both `anvil config show` and doctor's `config-valid` check. On a TTY
(not `--json`, and not CI or a git hook) doctor then offers to migrate a lone
`.anvilrc`, remove the shadowed leftover, fold `.anvil/gate-config.json`, or
record `architecture.source`. A single healthy canonical file is not prompted.

A malformed `gate` section is a loud error on both `anvil gate` and
`anvil check` — the shared reader refuses to run rather than silently skipping
your composition, and the error names the offending key with a dotted path (for
example `invalid gate.checks`).

## Inspect effective configuration

```text
anvil config show
```

For machine-readable output:

```text
anvil config show --json
```

Stdout is then a single JSON document and nothing else: `config` (the discovered
file, or `defaults`), `rule_modes` keyed by rule name, and `note` — the
legacy-key deprecation warning, or `null` when none applies.

## Change a rule mode

Use the installed command help so the accepted modes and identifiers match your
version:

```text
anvil config set --help
```

Prefer a focused command over hand-editing unfamiliar fields.

## Convert formats

Write the discovered project config as another canonical file (never
`.anvilrc`):

```text
anvil config convert --to json
anvil migrate format --format toml
```

Both commands share the same writer. `--remove-old` deletes the source when the
destination is a different path. `--force` overwrites an existing destination.
`--stdout` on `config convert` prints the converted text and writes nothing (the
previous default).

```text
anvil config convert --help
anvil migrate format --help
```

Keep one project configuration format unless you are mid-conversion.

## Choose a format during first activation

```text
anvil start --format yaml
```

Supported values are shown by `anvil start --help`.

## Exclude generated files from anti-pattern scanning

Committed generated files — for example router or GraphQL codegen output — often
carry blanket `eslint-disable` or `@ts-nocheck` headers that would otherwise
trip anti-pattern rules. anvil already skips files it recognises as generated by
a `*.gen.*` or `*.generated.*` name, a `generated`, `.generated`, or
`__generated__` directory, or a `@generated` / "Code generated by" banner in the
opening lines.

For generators whose output does not match those conventions, declare the paths
explicitly. Either mark them in `.gitattributes` with the standard
`linguist-generated` attribute:

```text
src/api/client.ts linguist-generated=true
*.pb.ts linguist-generated=true
```

or list workspace-relative globs under `antipattern.exclude` in your project
configuration:

```yaml
antipattern:
  exclude:
    - 'src/generated/**'
    - '*.pb.ts'
```

Both apply to `anvil check` and `anvil gate`. Only the anti-pattern scan is
affected: secret detection still inspects these files, so a credential committed
to a generated file is still caught.

## Review rules

- Commit project configuration only after checking the diff.
- Never commit credentials, tokens, personal paths, daemon state, or caches.
- Explain suppressions narrowly and review them like code.
- Use the [generated rule catalogue](../reference/rules.md) for current IDs.
- Run `anvil doctor` when configuration is rejected.

## User-level state

Set `ANVIL_HOME` only when you deliberately need an isolated user state area,
such as testing a candidate beside a normal install. Project state remains in
the project unless the command says otherwise.

## Next step

Read [local data and security](security.md).
