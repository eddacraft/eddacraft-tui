---
id: config
title: Configuration fields
description:
  Keys anvil reads and writes in .anvil.yaml, cited from product sources.
owner: DOCDEF
upstream:
  - crates/anvil-cli/src/commands/init.rs
  - crates/anvil-config/src/gate_section.rs
  - crates/anvil-config/src/migrations.rs
  - crates/anvil-config/src/discover.rs
  - crates/anvil-cli/src/commands/config.rs
verified_against: 0.9.6-beta
---

# Configuration fields

There is no published typed schema for project configuration. anvil parses the
file as an open document. This catalogue is the union of keys the init writer
emits, keys the gate and rule-mode readers recognise, keys migrate and discover
honour, keys tests assert, and the checked-in fixture of the file `anvil init`
writes.

`anvil config show --json` is **not** a key census. It returns a file **label**,
the four rule modes, and an optional deprecation note. See
[Inspection contract](#inspection-contract).

For inspect, convert, and migrate steps, use
[Inspect and migrate configuration](../operations/config.md).

## File and discovery

| Field          | Value                                                                                            |
| -------------- | ------------------------------------------------------------------------------------------------ |
| Canonical name | `.anvil.yaml` (also `.yml` / `.json` / `.toml`)                                                  |
| Written by     | `anvil init` writes `.anvil.yaml` by default; the wizard can write `.anvil.json` / `.anvil.toml` |
| Discovery      | First existing file, in order: `.anvil.yaml`, `.anvil.yml`, `.anvil.json`, `.anvil.toml`         |
| Legacy name    | `.anvilrc` (read-only fallback; no command creates one)                                          |
| No file        | Commands use in-memory defaults; `anvil config show` labels that state `defaults`                |
| Key case       | `snake_case` on write; legacy `camelCase` accepted on read                                       |

## The file `anvil init` writes

This is the checked-in fixture of a default YAML init:

```text
schema_version: "1.0.0"
planning_dir: "plans"
format: "yaml"
checks:
  - "secret-detection"
  - "import-boundaries"
  - "antipattern-scan"
```

Those four keys are the complete default document. Other keys in this catalogue
are added by later commands, migrate, or hand-edit.

## Top-level keys

### `schema_version`

| Field        | Value                                         |
| ------------ | --------------------------------------------- |
| Path         | `.anvil.yaml` → `schema_version`              |
| Type         | string                                        |
| Written by   | `anvil init`                                  |
| Read by      | project-config load; `anvil migrate schema`   |
| Default      | `"1.0.0"` (init writer and init-file fixture) |
| Legacy names | `schemaVersion`                               |

Allowed values: only `"1.0.0"` is observed in the init writer and the fixture.
Do not invent a version range.

### `planning_dir`

| Field        | Value                               |
| ------------ | ----------------------------------- |
| Path         | `.anvil.yaml` → `planning_dir`      |
| Type         | string                              |
| Written by   | `anvil init`                        |
| Read by      | project-config load                 |
| Default      | `"plans"` (init writer and fixture) |
| Legacy names | `planningDir`                       |

### `format`

| Field        | Value                                                          |
| ------------ | -------------------------------------------------------------- |
| Path         | `.anvil.yaml` → `format`                                       |
| Type         | string                                                         |
| Written by   | `anvil init`; `anvil config convert` may rewrite this metadata |
| Read by      | project-config load; convert / migrate format                  |
| Default      | `"yaml"` (init writer and fixture)                             |
| Legacy names | none                                                           |

Allowed values observed in the convert surface: `yaml`, `yml`, `json`, `toml`.
Init writes `yaml`, `json`, or `toml` to match the chosen file. Convert refuses
`.anvilrc` as a destination. Embedded metadata uses the stable spelling `yaml`
for both `.yaml` and `.yml` files.

### `checks`

| Field        | Value                                                                 |
| ------------ | --------------------------------------------------------------------- |
| Path         | `.anvil.yaml` → `checks`                                              |
| Type         | list of strings                                                       |
| Written by   | `anvil init`                                                          |
| Read by      | `anvil check`, `anvil gate` (check **selection**)                     |
| Default      | `secret-detection`, `import-boundaries`, `antipattern-scan` (fixture) |
| Legacy names | none                                                                  |

Use canonical check names from the [check catalogue](checks.md). `anvil check`
still runs only `secret-detection` and `antipattern-scan`, even when other names
appear here. Surface checks (`sql-migrations`, `github-actions`, `dockerfile`,
`shell-scripts`) are **not** list-editable through `checks:`.

When this list is absent or empty, `gate.checks` key presence becomes the
selection list. When this list is present and non-empty, it is authoritative.

## Nested objects

### `antipattern.exclude`

| Field        | Value                                                     |
| ------------ | --------------------------------------------------------- |
| Path         | `.anvil.yaml` → `antipattern.exclude`                     |
| Type         | list of workspace-relative globs                          |
| Written by   | hand-edit                                                 |
| Read by      | `anvil check` and `anvil gate` (anti-pattern engine only) |
| Default      | unset (empty list)                                        |
| Legacy names | none                                                      |

Files matching these globs are skipped by the anti-pattern scan. Secret
detection still inspects them. anvil already skips files it recognises as
generated; use this list for generators that do not match those conventions.
How-to:
[Exclude generated files from anti-pattern scanning](../operations/config.md#exclude-generated-files-from-anti-pattern-scanning).

### `architecture.source`

| Field        | Value                                                                  |
| ------------ | ---------------------------------------------------------------------- |
| Path         | `.anvil.yaml` → `architecture.source`                                  |
| Type         | string path                                                            |
| Written by   | `anvil migrate architecture --apply`                                   |
| Read by      | architecture validate / show; save-time watch                          |
| Default      | unset; standalone `.anvil/architecture.yaml` remains a legacy fallback |
| Legacy names | none                                                                   |

Records the standalone architecture file after migrate. Architecture
**definition** fields (`layers`, `patterns`, `depends_on`) are not this
catalogue; see [Define architecture boundaries](../first-project.md).

### `gate.version`

| Field        | Value                                              |
| ------------ | -------------------------------------------------- |
| Path         | `.anvil.yaml` → `gate.version`                     |
| Type         | optional unsigned integer                          |
| Written by   | `anvil migrate gate-config --apply` (when folding) |
| Read by      | gate-section reader (informational)                |
| Default      | unset                                              |
| Legacy names | none                                               |

A schema marker carried from retired `.anvil/gate-config.json`. It does not
select checks.

### `gate.thresholds`

| Field        | Value                               |
| ------------ | ----------------------------------- |
| Path         | `.anvil.yaml` → `gate.thresholds`   |
| Type         | reserved table of unsigned integers |
| Written by   | migrate fold; hand-edit             |
| Read by      | gate-section reader (stored only)   |
| Default      | unset                               |
| Legacy names | none                                |

Reserved and unused: no gate run consumes these values. Tests observe names such
as `overall_score`; that is not shipped behaviour. Do not treat a threshold as
an enforcement control.

### `gate.global_config`

| Field        | Value                                |
| ------------ | ------------------------------------ |
| Path         | `.anvil.yaml` → `gate.global_config` |
| Type         | open table                           |
| Written by   | migrate fold; hand-edit              |
| Read by      | gate-section reader (open table)     |
| Default      | unset                                |
| Legacy names | none                                 |

The reader accepts any keys in this table. Tests include `strict` as an example
boolean; no named key in this table has documented shipped behaviour.

### `gate.checks`

| Field        | Value                                                                              |
| ------------ | ---------------------------------------------------------------------------------- |
| Path         | `.anvil.yaml` → `gate.checks.<check-name>`                                         |
| Type         | table of per-check tables                                                          |
| Written by   | `anvil gate-config`; migrate fold; hand-edit                                       |
| Read by      | gate-section reader; selection **only when** top-level `checks` is absent or empty |
| Default      | unset                                                                              |
| Legacy names | none                                                                               |

Each check name maps to a table, or to empty / null meaning “selected, no
config”. Nested keys inside a per-check table are an open map; tests use
`max_findings` as an example and that is not a published control.

Unknown keys **inside** `gate` are ignored. A malformed `gate` section is a loud
error on both `anvil gate` and `anvil check`.

### `enforcement.rules`

| Field        | Value                                           |
| ------------ | ----------------------------------------------- |
| Path         | `.anvil.yaml` → `enforcement.rules.<rule>.mode` |
| Type         | four named rules, each with a string `mode`     |
| Written by   | `anvil config set` **only**                     |
| Read by      | `anvil config show`; rule-mode reader           |
| Default      | all four `warn`                                 |
| Legacy names | none                                            |

See [Rule modes](#rule-modes-anvil-config-set). This is the only nested object
`anvil config set` writes.

## Rule modes (`anvil config set`)

`anvil config set <rule> <mode>` sets **rule modes only**. It does not write
`schema_version`, `checks`, `gate.*`, or any other key.

| Rule                          | Path                                                 |
| ----------------------------- | ---------------------------------------------------- |
| `public-api-expansion`        | `enforcement.rules.public-api-expansion.mode`        |
| `new-dependency-introduction` | `enforcement.rules.new-dependency-introduction.mode` |
| `cross-layer-violation`       | `enforcement.rules.cross-layer-violation.mode`       |
| `privilege-expansion`         | `enforcement.rules.privilege-expansion.mode`         |

Canonical modes: `off`, `warn`, `enforce`. Defaults are `warn`.

Accepted aliases on parse: `off` also `disabled` / `none`; `warn` also
`warning`; `enforce` also `block` / `error`. Owned writes store the canonical
spelling.

```text
anvil config set public-api-expansion warn
```

## Inspection contract

```text
anvil config show
anvil config show --json
```

`--json` is a single JSON document with this **shape**, not a dump of file keys:

```json
{
  "config": ".anvil.yaml",
  "rule_modes": {
    "public-api-expansion": "warn",
    "new-dependency-introduction": "warn",
    "cross-layer-violation": "warn",
    "privilege-expansion": "warn"
  },
  "note": null
}
```

| Field        | Meaning                                                              |
| ------------ | -------------------------------------------------------------------- |
| `config`     | Discovered **file label** (`.anvil.yaml`, `.anvilrc`, or `defaults`) |
| `rule_modes` | The four rules above, each `off`, `warn`, or `enforce`               |
| `note`       | Legacy-key deprecation warning, or `null`                            |

Human `anvil config show` prints the same three facts as prose. Convert and
migrate stay on [Inspect and migrate configuration](../operations/config.md).

## Not in this catalogue

- Unknown keys the open parser accepts and ignores
- Flag-driven surface checks (`sql-migrations`, `github-actions`, `dockerfile`,
  `shell-scripts`) — not list-editable via `checks:`
- Behaviour for nested keys inside `gate.checks.*` or `gate.global_config`
  beyond “open table”
- Architecture definition fields (`layers`, `patterns`, `depends_on`)
- User-level `ANVIL_HOME` state
- Anything not observed in the cited sources or the init-file fixture

## Related

- Journey: [Inspect and migrate configuration](../operations/config.md)
- Definition: [Check catalogue](checks.md)
- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)
- CLI: [`anvil config`](cli.md)
