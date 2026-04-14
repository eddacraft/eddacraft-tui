# Anvil Update Command

**Date:** 2026-04-13
**Status:** Approved
**Author:** @aneki

## Summary

Add an `anvil update` subcommand that checks for and installs new releases
from the same GitHub Releases location used by the install scripts. Uses a
hybrid strategy: shell out to the cargo-dist sidecar binary when available,
fall back to the `axoupdater` crate as a library.

## Motivation

Cargo-dist already bundles an `eddacraft-anvil-update` sidecar binary via
`install-updater = true`, but users must know it exists and run it manually.
An `anvil update` subcommand provides the natural UX that users expect.

## Command Surface

```
anvil update              # check + install latest
anvil update --check      # check only, print available version
anvil update --version X  # install specific version
anvil update --force      # reinstall even if on latest
```

Global flags `--json`, `--verbose`, `--no-tui` apply as usual.

## Auth

**Bypass** — same group as `doctor`, `welcome`, `init`, etc.

Rationale: release artefacts live on the public `eddacraft/anvil` repo.
Requiring auth to update would lock out users whose sessions have expired,
which is precisely when they may need to update (e.g. if the update fixes an
auth bug).

## Update Strategy (Hybrid)

The command follows this resolution order:

### 1. Detect Homebrew Install

Check whether the current binary path is under a Homebrew prefix
(`/opt/homebrew/`, `/usr/local/Cellar/`, `/home/linuxbrew/`). If so, print:

```
anvil was installed via Homebrew. Run `brew upgrade eddacraft/tap/anvil` instead.
```

Exit with a non-error code (0) since nothing is broken — just a different
update path.

### 2. Try Sidecar Binary

Look for `eddacraft-anvil-update` in:
1. The same directory as the current `anvil` binary (`std::env::current_exe()` parent)
2. `PATH`

If found, shell out to it, forwarding relevant flags:
- `--check` → sidecar's check-only mode
- `--version X` → sidecar's version pin
- No `--force` mapping needed — sidecar reinstalls unconditionally when
  given a version

Stream sidecar stdout/stderr to the user. Exit with the sidecar's exit code.

### 3. Fall Back to axoupdater Library

If no sidecar is found, use `axoupdater` as a Rust library dependency:
- Construct an `AxoUpdater` pointed at `eddacraft/anvil` GitHub Releases
- Query latest release tag
- Compare against current version (`env!("CARGO_PKG_VERSION")`)
- If `--check`: print version info and exit
- Otherwise: download and replace the binary

### 4. Both Fail

If the sidecar isn't found and axoupdater can't determine the install
source (e.g. built from source with `cargo build`), print:

```
Could not determine install method. To update manually:
  - From source: cargo build --release
  - Install script: curl -fsSL https://install.eddacraft.ai | sh
```

Exit with `EXIT_ERROR`.

## Output

### Plain text (default)

```
Current version: 0.3.1-beta
Latest version:  0.3.2-beta
Downloading eddacraft-anvil v0.3.2-beta...
Updated successfully.
```

### --check (no update available)

```
Current version: 0.3.2-beta
Already up to date.
```

Exit code 0.

### --check (update available)

```
Current version: 0.3.1-beta
Latest version:  0.3.2-beta
Update available. Run `anvil update` to install.
```

Exit code 1 (useful for CI: `anvil update --check || echo "outdated"`).

### --json

```json
{
  "current_version": "0.3.1-beta",
  "latest_version": "0.3.2-beta",
  "update_available": true,
  "action": "check"
}
```

## New Files

| File | Purpose |
|------|---------|
| `crates/anvil-cli/src/commands/update.rs` | Subcommand implementation |

## Modified Files

| File | Change |
|------|--------|
| `crates/anvil-cli/src/commands/mod.rs` | Add `pub mod update;` |
| `crates/anvil-cli/src/main.rs` | Add `Update` variant to `Commands`, match arm, auth-bypass |
| `crates/anvil-cli/Cargo.toml` | Add `axoupdater` dependency |

## New Dependency

| Crate | Purpose | Notes |
|-------|---------|-------|
| `axoupdater` | Library fallback for self-update | Already used by cargo-dist sidecar; adding as direct dep |

## No Changes To

- `dist-workspace.toml` — sidecar already enabled
- Release workflow — no new artefacts
- Install scripts — update path is the same

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (updated, already up-to-date, or Homebrew detected) |
| 1 | Update available (`--check` mode), or error during update |

## Risks

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| axoupdater API changes across versions | Low | Pin to compatible range; test in CI |
| Sidecar binary name changes in future cargo-dist | Low | Name is derived from package name, stable |
| Self-replacement fails mid-write (corrupt binary) | Very low | axoupdater handles atomic replacement |
| Homebrew prefix detection misses edge cases | Low | Cover the three standard prefixes; users can always run brew directly |

## Testing

- Unit tests for Homebrew prefix detection
- Unit tests for sidecar path resolution
- Integration test with `--check` flag (safe, read-only)
- `requires_auth` test confirming update bypasses auth
