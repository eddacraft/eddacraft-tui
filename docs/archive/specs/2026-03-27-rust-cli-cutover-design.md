# Rust CLI Cutover Design

**Date:** 2026-03-27 **Status:** Draft **Scope:** RCLI module — cutover,
archival, distribution

## Summary

Ship the Rust `anvil` binary as the sole CLI, archive the Node.js CLI to
`archive/`, and distribute via `cargo-dist` to a public GitHub Releases repo.
Beta users install via curl script and activate via existing BAUTH device code
flow.

This is a **functional replacement** release — core workflow works, known gaps
are documented. Not full feature parity with the Node.js CLI.

## Blocking Work Items

### 1. Auth Credential Migration (RCLI-015a)

**Problem:** Rust CLI only reads `~/.config/anvil/credentials.json`. Existing
beta users have credentials at `~/.anvil/auth.json`, `~/.anvil/license`, or
`ANVIL_LICENSE` env var. After switchover they appear logged out.

**Solution:** Credential loader checks in order:

1. `$XDG_CONFIG_HOME/anvil/credentials.json` (current)
2. `~/.anvil/auth.json` (legacy)
3. `~/.anvil/license` (legacy)
4. `ANVIL_LICENSE` env var

On first successful load from a legacy path, copy to XDG location and print a
one-line notice:

```
Migrated credentials from ~/.anvil/auth.json → ~/.config/anvil/credentials.json
```

**Files:** `crates/anvil-cli/src/auth/credentials.rs`

### 2. Pre-Action Auth Enforcement (RCLI-015b)

**Problem:** Commands execute unconditionally even without auth.
`EXIT_AUTH_REQUIRED = 3` is defined but never triggered.

**Solution:** Add pre-dispatch middleware in `main.rs`:

- **Require auth:** gate, watch, status, admin, export
- **Bypass auth:** doctor, tutorial, init, hooks, new, wizard, `--version`,
  `--help`

When credentials are missing or expired, return exit code 3 with message:

```
Authentication required. Run `anvil auth login` to authenticate.
```

**Files:** `crates/anvil-cli/src/main.rs`

### 3. Wire Remaining Gate Checks (RCLI-013a)

**Problem:** 4 of 7 gate checks hard-fail with "not yet implemented": coverage,
dependency, architecture, policy. Plan-scoped gating and `--no-cache` are dead
code.

**Solution:** Implement the 4 checks at a functional level (not full parity with
Node.js — those crates are still maturing):

- **Coverage:** look for `coverage/lcov.info` or `coverage/cobertura.xml` in
  project root. If present, parse total line coverage and compare against
  threshold from `.anvil.yaml` (default 80%). If no report found, skip with
  notice rather than fail
- **Dependency:** scan `package-lock.json` and/or `Cargo.lock` for packages
  matching a known-vulnerable list (shipped as a JSON fixture, updated per
  release). Not a full advisory database — a curated blocklist for beta
- **Architecture:** load `.anvil/architecture.yaml`, parse layer definitions,
  check import edges against layer boundaries using the kernel's dependency
  graph. Delegates to `anvil-architecture` crate's `validate()` function (exists
  but needs wiring)
- **Policy:** load policy bundle from `.anvil/policies/`, evaluate against
  current file set using `anvil-policy` crate's `evaluate()` function (exists
  but needs wiring). If no policy bundle configured, skip with notice

Wire `plan` positional arg for plan-scoped gating (filter checked files to those
referenced in the plan). Wire `--no-cache` to skip result cache.

**Files:** `crates/anvil-cli/src/commands/gate.rs` **Dependencies:**
`crates/anvil-policy/`, `crates/anvil-architecture/`

**Note:** The `anvil-policy` and `anvil-architecture` crates (RCLI-017,
RCLI-019) are currently scaffolds. This work item includes adding enough
implementation to those crates for the gate checks to function. Full crate
maturity is not required — the gate checks need `validate()` and `evaluate()`
entry points, not the complete API surface.

### 4. Output Formatters (RCLI-022)

**Problem:** `--json` and `--no-tui` flags exist but produce no output. CI users
need machine-readable output.

**Solution:** Create `output::plain` and `output::json` modules:

- **Plain:** indented lists, ASCII tables, colour-free. Used when `--no-tui` is
  set or stdout is not a TTY
- **JSON:** serialise the same data structures surfaces consume. Used when
  `--json` is set

All commands route through a shared `Output` trait that dispatches to TUI,
plain, or JSON based on flags + TTY detection.

**Files:** `crates/anvil-cli/src/output/`

### 5. Welcome Menu Parity (RCLI-030)

**Problem:** Welcome menu lacks gate/watch launch options that the Ink CLI had.

**Solution:** Add gate and watch as menu items in the welcome surface. Selecting
them launches the respective surface as a sub-surface. Esc returns to the
welcome menu (RCLI-026 already implemented this navigation pattern).

**Files:** `crates/anvil-tui/src/surfaces/welcome/mod.rs`,
`crates/anvil-cli/src/commands/welcome.rs`

## Archival (RCLI-023)

### Pre-Archival

1. Tag the repo `pre-rust-cli` for rollback reference
2. Verify Rust CLI builds and all commands dispatch: `cargo build --release`

### Archival Steps

1. `git mv apps/anvil-cli/ archive/anvil-cli-node/`
2. `git mv apps/anvil-cli/src/tui/ archive/anvil-tui-ink/`
3. Remove `apps/anvil-cli` from pnpm workspace (`pnpm-workspace.yaml`)
4. Remove Nx project config for `@eddacraft/anvil-cli`
5. Remove any `@eddacraft/anvil-cli` references from other packages'
   dependencies
6. Update root `package.json` scripts if they reference the Node.js CLI
7. Clean up `tsconfig` references pointing at the archived package

### Post-Archival Validation

- `cargo build --workspace` passes
- `pnpm install && pnpm build` passes (no broken workspace references)
- `pnpm nx run-many -t test` passes (same pass count as before, minus any
  anvil-cli-specific tests)
- `anvil --help` works from the Rust binary

## Distribution (RCLI-024)

### Strategy: cargo-dist + Public Releases Repo

**Why public repo:** The main repo is private. Beta users don't have GitHub
access — they authenticate via BAUTH (device code + OTP). Binaries must be
downloadable without GitHub auth.

**Solution:** Create `eddacraft/anvil-releases` (public repo, contains only
release assets and the install script). The CI workflow in the private repo
cross-compiles, then pushes release assets to the public repo.

### Setup

1. `cargo dist init` in the private repo — generates release workflow config,
   target matrix, and install script template
2. Configure 4 targets:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
3. Modify the generated workflow to push assets to `eddacraft/anvil-releases`
   instead of the private repo's releases
4. Host install script at `install.eddacraft.ai` (static page, can be Vercel or
   a raw GitHub URL from the public repo)

### Release Profile

```toml
[profile.release]
strip = "symbols"
lto = true
codegen-units = 1
panic = "abort"
```

Produces small, stripped binaries. No debug symbols, no recoverable source.

### Install Flow (User Perspective)

```bash
curl -sSf https://install.eddacraft.ai | sh
# Downloads correct binary for OS/arch → ~/.local/bin/anvil or /usr/local/bin/anvil

anvil auth login
# Device code flow → activates licence → ready to use
```

### Release Flow (Maintainer Perspective)

1. Bump version in `crates/anvil-cli/Cargo.toml`
2. `git tag v0.3.0-beta`
3. `git push --tags`
4. CI builds 4 binaries, pushes to `eddacraft/anvil-releases` as a GitHub
   Release with the same tag
5. Install script auto-resolves to latest release

## Known Gaps (Ship With Documentation)

These are documented in release notes. Users see clear "not yet implemented"
messages, not silent failures.

| Gap                                                               | Impact                      | Tracking             |
| ----------------------------------------------------------------- | --------------------------- | -------------------- |
| `watch --file/--action/--patterns/--exclude` parsed but not wired | Power users only            | RCLI-014a            |
| Hooks generate `doctor --no-tui` only, no enforcement             | Teams miss plan/gate checks | RCLI-021a            |
| Export: APS markdown rejected, constraint formatters bail         | APS workflow users          | RCLI-021b, RCLI-021c |
| Policy/architecture subcommands shallow                           | Advanced users              | RCLI-018a, RCLI-020a |
| Audit viewport doesn't scroll for large lists                     | Many-issue projects         | RCLI-027             |
| `login`/`logout`/`whoami` top-level aliases missing               | Convenience only            | RCLI-015c            |
| Doctor `--fix` doesn't execute fixes                              | Manual fix needed           | RCLI-028             |

## Release Sequence

1. Complete 5 blocking work items (RCLI-015a, 015b, 013a, 022, 030)
2. Triage 12 e2e test failures — fix regressions, skip pre-existing
3. Decide MAINT-011 (TS 6.0): finish or defer (not blocking)
4. RCLI-023: tag `pre-rust-cli`, archive Node.js CLI
5. RCLI-024: `cargo dist init`, configure targets, set up public releases repo
6. Tag `v0.3.0-beta`, CI builds and publishes
7. Write release notes with known gaps table
8. Update install script, verify end-to-end install flow
9. Notify beta users

## Success Criteria

- [ ] `curl -sSf https://install.eddacraft.ai | sh` installs correct binary
- [ ] `anvil auth login` completes device code flow
- [ ] Existing beta users' credentials are migrated transparently
- [ ] `anvil gate` runs all 7 checks with real logic
- [ ] `anvil watch` shows live TUI with kernel events
- [ ] `anvil --json gate` produces parseable JSON for CI
- [ ] `cargo build --workspace` passes without Node.js CLI
- [ ] `pnpm build` passes without Node.js CLI
- [ ] All Rust tests pass (520+)
- [ ] All TS unit tests pass (1461+)

## Out of Scope

- crates.io publishing (post-beta)
- Windows support (Linux + macOS only)
- Homebrew tap (cargo-dist can add later)
- Full feature parity with Node.js CLI (tracked via deferred RCLI items)
- MCP server migration (stays Node.js)
- Dashboard/website changes
