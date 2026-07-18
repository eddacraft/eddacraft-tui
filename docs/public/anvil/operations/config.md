---
id: config
title: Configuration reference
description:
  Inspect, change, convert, and review anvil project configuration safely.
---

# Configuration reference

anvil reads project configuration from `.anvilrc` or a supported `.anvil.yaml`,
`.anvil.yml`, `.anvil.json`, or `.anvil.toml` file.

## Inspect effective configuration

```text
anvil config show
```

For machine-readable output:

```text
anvil config show --json
```

## Change a rule mode

Use the installed command help so the accepted modes and identifiers match your
version:

```text
anvil config set --help
```

Prefer a focused command over hand-editing unfamiliar fields.

## Convert formats

Preview a conversion on standard output:

```text
anvil config convert --help
```

Review the result before replacing the existing file. Keep one project
configuration format unless the installed help explicitly documents precedence.

## Choose a format during first activation

```text
anvil start --format yaml
```

Supported values are shown by `anvil start --help`.

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
