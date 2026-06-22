---
id: installation
title: Installation
description:
  Install the APS CLI, scaffold a project, and configure release channels.
sidebar_position: 3
---

# Installation

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

## Quick install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/EddaCraft/anvil-plan-spec/main/scaffold/install | bash
```

In an interactive terminal this shows a **mode picker**:

1. **Install the APS CLI** on this machine
2. **Initialize APS planning** in this repository
3. **Initialize this repository for an AI agent**
4. **Upgrade** an existing APS project
5. **Add a tool integration**

Pick non-interactively with a flag (required in CI):

```bash
curl -fsSL .../scaffold/install | bash -s -- --cli       # CLI only
curl -fsSL .../scaffold/install | bash -s -- --init      # scaffold this repo
curl -fsSL .../scaffold/install | bash -s -- --agent     # minimal agent bootstrap
curl -fsSL .../scaffold/install | bash -s -- --upgrade   # upgrade in place
curl -fsSL .../scaffold/install | bash -s -- --setup claude-code
```

`--init` is **minimal by default**: it writes `plans/` with rules, templates,
`project-context.md`, `issues.md`, and `.aps/config.yml` — nothing else. The
global `aps` binary on your PATH drives the repo.

## Global install

```bash
# Linux/macOS
curl -fsSL https://raw.githubusercontent.com/EddaCraft/anvil-plan-spec/main/scaffold/install | bash -s -- --global

# Windows (PowerShell)
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/EddaCraft/anvil-plan-spec/main/scaffold/install.ps1))) --global
```

This is **binary-first**: `--cli` installs the prebuilt native `aps` binary to
`~/.aps/bin` and adds it to your shell PATH. It falls back to the
bash/PowerShell CLI only when no release binary exists for your platform.

To update:

```bash
curl -fsSL https://raw.githubusercontent.com/EddaCraft/anvil-plan-spec/main/scaffold/update | bash -s -- --global
# Or: aps update --global
```

## Release channels

The native `aps` binary is built for five targets (Linux x86_64/aarch64, macOS
x86_64/aarch64, Windows x86_64) and published to GitHub releases.

```bash
# Pin an exact release
VERSION=0.4.0 curl -fsSL .../scaffold/install | bash -s -- --cli

# cargo-binstall — prebuilt binary from GitHub releases
cargo binstall aps-cli

# cargo install — build from source
cargo install aps-cli
```

On Windows, install via the script or **Scoop**:

```powershell
scoop install https://raw.githubusercontent.com/EddaCraft/anvil-plan-spec/main/packaging/scoop/aps.json
```

## Project config (`.aps/config.yml`)

`aps init` writes the per-repo contract the global `aps` binary reads by walking
up from the current directory:

```yaml
cli_version: 0.4.0 # toolchain semver, stamped from the running binary
plans_dir: plans/ # where plan documents live
docs_dir: docs/ # where generated docs live
tooling_root: .aps/ # APS-owned tooling root
```

- **`cli_version`** pins the toolchain a project expects
- **`plans_dir` / `docs_dir` / `tooling_root`** are runtime defaults — a
  monorepo can set `plans_dir: packages/foo/plans/`
- Unknown keys are ignored for forward compatibility

## Add integrations (`aps setup`)

After minimal `aps init`, add optional pieces:

```bash
aps setup                 # interactive picker
aps setup hooks           # hook scripts into .aps/scripts
aps setup claude-code     # tool integration (skill + agents)
aps setup all --yes       # full footprint
```

Tool names: `claude-code`, `copilot`, `codex`, `opencode`, `gemini`, `generic`.

## Upgrade (remove generated bloat)

Older installs scatter generated files across the repo. `aps upgrade` removes
that bloat safely:

```bash
aps upgrade            # dry run
aps upgrade --apply    # back up to .aps/backup/<timestamp>/, then remove
```

`upgrade` never deletes user content: `plans/**`, `AGENTS.md`, and settings
files are protected.

## Platform support

| Platform    | Authoring (`lint`/`init`)          | Orchestration (`next`/`start`/`complete`) |
| ----------- | ---------------------------------- | ----------------------------------------- |
| **Linux**   | Bash 4.0+                          | Bash 4.0+                                 |
| **macOS**   | Bash 4.0+ via `brew install bash`  | Bash 4.0+ via Homebrew bash               |
| **Windows** | PowerShell 5.1+ (native `aps.ps1`) | Bash 4.0+ via WSL or Git Bash             |

macOS ships Bash 3.2 (too old — APS needs associative arrays). Homebrew's bash
is picked up automatically.

## Next steps

1. Edit `plans/index.aps.md` to define your plan
2. Copy templates to create modules (remove the leading dot from filenames)
3. Point your AI agent at `plans/aps-rules.md`, or run `aps next`

---

**Next:** [Workflow →](./workflow.md)
