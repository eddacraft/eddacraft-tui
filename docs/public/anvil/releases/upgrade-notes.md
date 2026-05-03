---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between anvil versions.

## Current Version: 0.5.1-beta

## Upgrading to 0.5.1-beta

CLI upgrade from `0.5.0-beta`. This release focuses on scanner signal quality,
TUI interaction fixes, incremental graph correctness, and release workflow
hardening. TypeScript package consumers should note that archived scanner-era
subpath exports were removed from `@eddacraft/anvil-core` and
`@eddacraft/anvil-runtime`.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.5.1-beta

- **Secret scanner false-positive reductions** — generic secret matching now
  requires a stronger right-hand-side shape, credit-card detection rejects UUID
  fragments, and entropy matching focuses on secret-shaped quoted values.
- **Antipattern suppression fixes** — `AP-*` checks now honour local
  `eslint-disable` directives and avoid reporting guarded `Map.get` after
  `has`/`set` flows as `GS-001`.
- **Audit path filtering** — audit scans skip broader environment-template files
  while still reporting real `.env` files regardless of directory.
- **TUI interaction polish** — audit, status, and watch surfaces support
  zooming; doctor acknowledges `f` to fix; tutorial path selection has more room
  for wrapped options.
- **Incremental graph correctness** — watch updates now avoid synthetic import
  ID collisions and preserve import-source ID `0`, preventing missed import
  edges in refreshed symbol graphs.
- **TypeScript package subpath cleanup** — archived scanner-era subpaths for
  antipattern, suppression, drift, gate, and export flows are no longer
  exported; use the Rust CLI surfaces instead.
- **Release safety** — the PR base guard workflow now detects release-sensitive
  PRs targeting the wrong branch when repository branch protection requires the
  check.

## Upgrading to 0.5.0-beta

Drop-in upgrade from `0.4.0-beta`. There are no breaking changes; every new
behaviour below is opt-in.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.5.0-beta

- **Git config-mode hooks (opt-in)** — install Anvil-owned hook commands through
  Git 2.54 native config with `anvil hooks install --config` and remove them
  with `anvil hooks uninstall --config`. File-mode hooks remain the default and
  Husky stays as the contributor bootstrap; both surfaces detect and warn about
  file/config coexistence and `core.hooksPath` overrides.
- **AI guardrail profile** — `anvil gate --profile ai` runs the AI-focused check
  set, treats missing or invalid governance configuration as blocking, and emits
  the canonical `anvil.diagnostic.v1` JSON envelope by default for agent and MCP
  consumers.
- **AI-001 reasoning rule** — a new info-severity rule that flags source
  comments justifying code with authority, social proof, or deflection rather
  than technical reasoning. Suppress per occurrence with
  `// @anvil-ignore AI-001` and a short reason.
- **`.env` and `.envrc` scanning (`SURFENV-001`)** — `.env`, `.env.*`, and
  `.envrc` files are parsed as key/value files; leaked secret values are
  reported with the variable name and source line. Suppress with
  `# @anvil-ignore SURFENV-001`.
- **`anvil mcp-config`** — generates, verifies, and writes Claude Code, Cursor,
  Windsurf, and VS Code MCP server configuration. Use `--write` to apply
  changes, `--verify` to diff against the on-disk config, and rely on the
  path-safety prompts before atomic writes overwrite an existing file. See
  [MCP Integration](../integrations/mcp.md) for the supported transports and
  per-client paths.
- **Scan performance cap** — first-run scans honour `ANVIL_SCAN_THREADS`
  (default `min(num_cpus, 4)`) so the parallel walk does not starve TUI or
  editor work; oversized lines are skipped before regex evaluation to eliminate
  the previous ReDoS risk.
- **Doctor outside git repos** — running `anvil doctor` outside a Git repository
  now produces a structured `git-repo` warning instead of failing the whole run.

### Operator-side: API migration runner

Anvil API deploys now ship with a first-party SQL migration runner with dry-run
support and drift detection. Operators running `anvil-api` should review the
migration runbook before the next deploy; CLI users do not need to take action.

## Upgrading to 0.4.0-beta

Drop-in upgrade from `0.3.3-beta` for most users. Three behavioural changes
require attention:

- **`anvil watch --exclude` now takes glob patterns, not bare directory names.**
  A previous `--exclude vendor` no longer excludes files under `vendor/`; use
  `--exclude 'vendor/**'` instead. The CLI prints a warning when a
  likely-bare-name pattern is detected.
- **`anvil doctor --json` envelope changed** from a bare array to
  `{ "checks": [...], "notifications": [...], "schema_version": "2.0.0" }`, and
  every check now carries a structured `remediation` object
  (`{ summary, command?, doc_url? }`). Consumers iterating the array must switch
  to `data.checks`; consumers that schema-validated the prior shape must accept
  the `remediation` field on every check and the new `schema_version` envelope
  field. Branch on `schema_version` to gate compatibility — pass / skipped
  checks emit `remediation: { summary: "" }`, fail / warn checks always populate
  `summary` and at least one of `command` or `doc_url`.
- **`anvil check`, `anvil gate`, `anvil audit` JSON outputs now include a
  `notifications[]` field** alongside their existing payloads. Consumers pinned
  to the prior shape will see an additional ignorable field; nothing is removed.
  The notification envelope shape is shared with `anvil doctor`.

### Operator-side: per-operator admin keys

If you're running the `anvil-api` backend and want to enable the new
per-operator admin key flow shipped in this release (replacing the single shared
admin key), set:

- `ADMIN_PER_OPERATOR_KEYS=1` — turns on per-operator key resolution
- `ADMIN_KEY_PEPPER=<random-32-byte-hex>` — pepper for the peppered-hash lookup;
  must be set before any per-operator keys can authenticate

When `ADMIN_PER_OPERATOR_KEYS=1` is set without a non-empty `ADMIN_KEY_PEPPER`,
the middleware falls back to the legacy shared-key auth and logs an error
server-side. CLI requests will not see the misconfiguration directly. Provision
both via your secret manager (Pulumi handles this for the EddaCraft-managed
deployment) before rolling operators onto per-operator keys.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.4.0-beta

- **`anvil watch --patterns / --exclude`** — user-supplied glob filter on the
  watch loop. Previously the flags were declared but never read; watch silently
  used a hardcoded scope.
- **Post-init auto-analysis** — `anvil init` now runs an inline first scan and
  surfaces a real signal (top warnings + counts) rather than pointing at
  `anvil doctor`.
- **Doctor structured remediation** — every `anvil doctor` check emits a
  concrete remediation field (link, command, or auto-fix prompt); no check
  terminates at "see README".
- **`anvil watch` startup banner** — prints active include / exclude scope so
  the active filter is visible at a glance.
- **Workspace hardening** — cargo-hakari workspace-hack, cargo-deny policy,
  third-party notices via cargo-about (RUSTNX).

## Upgrading to 0.3.3-beta

Drop-in upgrade from `0.3.2-beta`. No configuration migration is required.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.3.3-beta

- **Windows distribution** — WinGet landed and Scoop became part of the
  documented install/upgrade story.
- **Admin operations** — the separate `anvil-admin` operator CLI gained
  list/show/invite/audit/revoke and migration tooling.
- **Windows UX fixes** — onboarding, discovery, and key handling improved.

## Upgrading to 0.3.2-beta

Drop-in upgrade from `0.3.1-beta`. No configuration migration is required.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

## Upgrading to 0.3.1-beta

Drop-in upgrade from 0.3.0-beta. No configuration changes required.

```bash
# Upgrade via the installer (overwrites existing binary)
curl -fsSL https://install.eddacraft.ai | sh

# Or via Homebrew
brew upgrade eddacraft/tap/anvil

# Or via the built-in updater
anvil update
```

### What's New in 0.3.1-beta

- **Docs domain cutover** — `docs.eddacraft.ai` now served via a dedicated proxy
  with a Nordic terminal-themed landing page.
- **Welcome screen fixes** — first-user onboarding flows restored after
  regressions in 0.3.0-beta.
- **Auth error messages** — clearer error messages during login and device-code
  flows.
- **TUI version display** — shell footer now shows the correct version.

No breaking changes. All existing configuration, credentials, and workflows
continue to work without modification.

## Upgrading to 0.3.0-beta

`0.3.0-beta` was the release where anvil became a native Rust binary. Current
docs assume a fresh install on the Rust CLI rather than a staged migration from
the legacy Node.js package.

```bash
# Install the native binary
curl -fsSL https://install.eddacraft.ai | sh
```

If an older npm-installed `anvil` is still earlier on your `PATH`, remove
`@eddacraft/anvil-cli` and re-run `anvil --version` so you know the native
binary is the command being executed.

Your `.anvilrc` and `.anvil/` directory work without changes.

For full details, see [The Switch to Rust](./rust-rewrite.md).

### What's New

- **Native binary** — 5–10x faster scanning, 80% less memory in watch mode, no
  Node.js dependency.
- **Kernel engine** — foreground watch mode, incremental parsing, and real-time
  semantic graph updates in the native Rust runtime.
- **Ratatui TUI** — rebuilt interactive surfaces with the eddacraft Terminal
  Standard design system.
- **Welcome & onboarding** — first-run interactive experience; run
  `anvil welcome` anytime.
- **New commands** — `anvil new`, `anvil wizard`, `anvil audit`, `anvil drift`,
  `anvil validate`, `anvil gate-config`.
- **Structured exit codes** — `0` (pass), `1` (error), `2` (gate fail), `3`
  (auth required), `4` (config error).
- **Beta auth** — device-flow and OTP authentication with OS keychain storage.

### Breaking Changes

- **Installation method** — install anvil as a native binary via the installer,
  Homebrew, WinGet, or Scoop.
- **CI workflows** — replace `pnpm anvil` / `npx anvil` with direct `anvil`
  calls. Remove Node.js setup steps if anvil was the only reason they existed.
- **Docs access** — the `/anvil` documentation is now gated behind GitHub OAuth
  for beta users. Sign in with the GitHub account tied to your beta invite.
  Public eddacraft docs (APS, Kindling, edda-stack) remain open.

## Upgrading to 0.2.1-beta

Drop-in upgrade from any previous 0.2.x version. No configuration changes
required.

### What's New in 0.2.1

- **Project memory** — anvil now tracks patterns and decisions in your codebase
  via the Edda memory system and Ember proposal engine.
- **Security hardening** — input validation and subprocess execution
  improvements across the platform.
- **Dependency patches** — minimatch, axios, svgo, tar, and others.

No breaking changes. The new memory features are opt-in and do not affect
existing scanning behaviour.

## Upgrading to 0.1.2-beta

This was the first public beta. No breaking migrations from alpha beyond the
configuration key change below.

### Note for Early Alpha Testers

If you used an internal alpha build, the top-level configuration key changed
from `"checks"` to `"gates"`:

```json
// Old (alpha)
{
  "checks": {
    "architecture": { ... }
  }
}

// Current (0.1.x-beta)
{
  "gates": {
    "architecture": { ... }
  }
}
```

Run `anvil init --force` to regenerate your configuration, or rename the key
manually in `.anvilrc`.

## Future Versions

Upgrade guides are added here as new versions ship.

## Getting Help

If you encounter upgrade issues:

1. Check the [Troubleshooting guide](/anvil/operations/troubleshooting)
2. Search [existing issues](https://github.com/eddacraft/anvil/issues)
3. Open a new issue with:
   - Old version
   - New version
   - Error message
   - Steps to reproduce

---

**See also:** [Changelog](/anvil/releases/changelog),
[The Switch to Rust](/anvil/releases/rust-rewrite)
