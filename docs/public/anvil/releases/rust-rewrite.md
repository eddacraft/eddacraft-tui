---
id: rust-rewrite
title: The Native Rust CLI
description:
  How the current Rust Anvil binary is packaged, configured, and operated.
sidebar_position: 10
---

# The Native Rust CLI

Anvil has shipped as a native Rust product since `0.3.0-beta`. The current
release is one `anvil` binary: CLI commands, terminal UI, MCP stdio server,
policy engine, updater, and daemon operations all use the same versioned
artefact.

There is no companion runtime or package to install.

## What ships in the binary

| Surface                          | Entry point                         |
| -------------------------------- | ----------------------------------- |
| Guided discovery                 | `anvil welcome`                     |
| Repository activation            | `anvil start`                       |
| Focused source scan              | `anvil check`                       |
| Workflow judgement               | `anvil gate`                        |
| Save-time feedback               | `anvil watch`                       |
| Native MCP server                | `anvil mcp serve --stdio`           |
| Local daemon operations          | `anvil intercept`                   |
| Rego policy engine               | `anvil policy`                      |
| Terminal dashboards and insights | `anvil dashboard`, `anvil insights` |
| Install-aware update guidance    | `anvil version`, `anvil update`     |

The MCP server and daemon use the same Rust checks, types, and protection-claim
contracts as the terminal commands. That keeps editor, watch, status, and CI
behaviour on one implementation.

## Install

```bash
# macOS / Linux
curl -fsSL https://install.eddacraft.ai | sh

# Homebrew (macOS / Linux)
brew install eddacraft/tap/anvil
```

```powershell
# Windows installer
irm https://install.eddacraft.ai/windows | iex

# WinGet
winget install eddacraft.anvil

# Scoop
scoop bucket add eddacraft https://github.com/eddacraft/scoop-bucket
scoop install anvil
```

Supported release targets are macOS, Linux, and Windows on x86_64 and ARM64. The
standalone installer places the binary under the per-user EddaCraft bin
directory and updates PATH when the platform permits it.

Use one install method per machine. `anvil version` identifies the active method
and prints the matching upgrade command:

```bash
anvil --version
anvil version
```

## Command model

`check` and `gate` answer different questions:

- **`anvil check`** runs the focused source-analysis surface for anti-patterns
  and hardcoded secrets. Use it for quick file or repository scans.
- **`anvil gate`** combines configured analysis, architecture, policy, command,
  build, test, coverage, and dependency checks into a workflow verdict.

For CI, use a gate profile:

```bash
anvil gate --profile ci
```

For active development, activate the repository once and use the daemon-backed
path:

```bash
anvil start
anvil status --verify
```

## Configuration

Anvil discovers `.anvilrc` and the multi-format `.anvil.yaml`, `.anvil.yml`,
`.anvil.json`, or `.anvil.toml` forms. Architecture definitions, drift
snapshots, suppressions, dashboards, and other project-owned state live under
`.anvil/`.

Preview schema reconciliation before writing it:

```bash
anvil migrate schema
anvil migrate schema --apply
```

The Rust policy engine evaluates Rego in process. A standalone OPA binary is
optional for authoring workflows that deliberately run `opa test`; it is not a
runtime requirement for Anvil gate evaluation.

## CI examples

Linux or macOS:

```yaml
- name: Install Anvil
  run: curl -fsSL https://install.eddacraft.ai | sh

- name: Run Anvil gate
  run: anvil gate --profile ci
```

Windows:

```yaml
- name: Install Anvil
  shell: pwsh
  run: irm https://install.eddacraft.ai/windows | iex

- name: Run Anvil gate
  run: anvil gate --profile ci
```

Gate checks may invoke the lint, test, coverage, or policy toolchain already
owned by the repository. Install those project dependencies when the enabled
checks need them; they are not dependencies of the Anvil binary itself.

## Update or reinstall

Use the owner of the current installation:

```bash
anvil update
# or: brew upgrade eddacraft/tap/anvil
```

```powershell
winget upgrade --id eddacraft.anvil
# or: scoop update anvil
```

On Windows, an editor can keep the running MCP binary open. Quit the editor or
stop its `anvil mcp serve` process before replacing the executable.

## Verify a report against the current binary

When reporting a problem, include:

- `anvil --version` and `anvil version` output;
- operating system and architecture;
- install method;
- the exact command and exit code; and
- the smallest output excerpt that explains the failure.

Open product issues at
[github.com/eddacraft/anvil/issues](https://github.com/eddacraft/anvil/issues).
