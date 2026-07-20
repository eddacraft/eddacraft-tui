---
id: installation
title: Install, update, or migrate APS
description:
  Choose an APS installation path and maintain an existing project safely.
sidebar_position: 3
---

# Install, update, or migrate APS

Use the [first-plan tutorial](getting-started.md) for the canonical first
installation. This page covers alternatives and maintenance.

## Supported platforms

The native `aps` binary provides the complete command surface on macOS, Linux,
and Windows. Windows users do not need WSL or Git Bash. The legacy shell runtime
is only a fallback when a release binary is unavailable.

## Install only the CLI

Use `--cli` when you want the command on the machine without creating project
files:

```bash
curl -fsSL https://raw.githubusercontent.com/eddacraft/anvil-plan-spec/main/scaffold/install | bash -s -- --cli
```

On Windows PowerShell, append `--cli` to the scriptblock command shown in the
quickstart.

## Install with a package tool

Build from source with Cargo:

```bash
cargo install aps-cli
```

Install the prebuilt release with cargo-binstall:

```bash
cargo binstall aps-cli
```

On Windows, Scoop can install the published manifest:

```powershell
scoop install https://raw.githubusercontent.com/eddacraft/anvil-plan-spec/main/packaging/scoop/aps.json
```

Use one ownership method for upgrades. Do not overlay a script installation with
Cargo or Scoop unless you first remove the old binary from `PATH`.

## Initialise without the wizard

For automation or a terminal without interactive input:

```bash
aps init --non-interactive --profile solo --shape single
```

Change `solo` to `team` or `agent-operator`, and `single` to `monorepo`, when
those choices match the project. Run `aps init --help` for optional templates,
paths, tools, hooks, and components.

## Add an optional integration

```bash
aps setup
```

The picker explains available additions. A direct tool setup is also valid:

```bash
aps setup codex
```

Supported tool keys are shown by `aps setup --help` and during the picker.

## Update generated APS files

```bash
aps update
```

`aps update` reconciles APS-owned templates and installed skills. It does not
rewrite your plan content.

To update the global binary, repeat the installation method that owns it. The
`aps update` command updates a project, not the machine-wide executable.

## Migrate an older project

First inspect the project without changing it:

```bash
aps doctor
aps migrate --dry-run
```

If the preview is correct, apply it:

```bash
aps migrate --apply
```

Migration backs up files before removing an old vendored runtime and adjusts
known generated paths. Review the reported backup location before deleting any
old files yourself.

## Project configuration

`.aps/config.yml` records the CLI version and project paths. Project-scoped
commands find it by walking up from the current directory, so they work from a
subdirectory without repeated path flags.

Use `--strict` in continuous integration when a CLI-version mismatch must fail
instead of warn:

```bash
aps --strict lint
```
