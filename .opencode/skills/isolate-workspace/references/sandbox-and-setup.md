# Sandbox and setup reference

Supporting detail for `isolate-workspace` Steps 3–4c. Read this before
selecting a setup command, and before running in a sandboxed or autonomous
harness. The hard rules in SKILL.md remain authoritative; nothing here relaxes
them.

## Per-ecosystem setup detection examples

The following are detection examples, not an instruction to try each fallback:

- Node/JS: `pnpm install` for `pnpm-lock.yaml`, `yarn install` for `yarn.lock`,
  or `npm ci` for `package-lock.json`; stop when no lockfile proves the manager.
- Rust: choose a target directory per "Choosing cache and temp locations" below,
  export it as `CARGO_TARGET_DIR`, then run `cargo fetch`.
- Python: run only the repository-documented command for `requirements.txt` or
  `pyproject.toml`; never try a fallback chain.
- Go: run `go mod download` when `go.mod` is present.

## Hermetic defaults for sandboxed runs

Create worktree-local directories for `HOME`, `XDG_CONFIG_HOME`,
`XDG_CACHE_HOME`, `XDG_RUNTIME_DIR`, `TMPDIR`, `CARGO_TARGET_DIR`, and
`PNPM_HOME`, then set those values through the harness command environment. In
PowerShell use `$env:NAME = <value>`; in a POSIX shell use
`export NAME=<value>`.

Do not inherit a shared `CARGO_TARGET_DIR`, package-manager home, or `/tmp` when
the sandbox makes those paths read-only or capacity-constrained. Choose the
replacement paths deliberately — see the next section. If native deps select a
missing compiler (for example `g++-11`), classify as environment setup, not
product failure, and stop with the missing toolchain named.

## Choosing cache and temp locations

Hermetic does not mean `/tmp`. A socket directory and a build cache are
constrained by different things, and conflating them is what leaves
multi-gigabyte debris behind after the run.

**Adopt the repository's convention first.** Check `.envrc`, worktree lifecycle
hooks (for example `.config/wt.toml`), `.cargo/config.toml`, and `AGENTS.md` or
`CONTEXT.md` before choosing anything yourself. A repository that already
relocates its build cache did so for a reason, usually a constrained mount, and
an ad-hoc override reintroduces the problem that convention solved. Adopt the
documented path; do not invent a parallel one.

**Short paths are a `TMPDIR` constraint, not a cache one.** Unix domain socket
paths are capped by `sun_len` (typically 108 bytes), so a deep hermetic
`TMPDIR` or `XDG_RUNTIME_DIR` breaks daemons, test harnesses, and language
servers with errors that do not name the cause. Keep those two short. No such
limit applies to a build cache, so "it needs to be short" is not a reason to
put one on `/tmp`.

**Build caches go on disk, never `/tmp`.** `CARGO_TARGET_DIR`, package-manager
stores, and compiler caches are large, long-lived, and often tens of gigabytes.
On most Linux hosts `/tmp` is a RAM-backed tmpfs, so a build cache there
consumes memory and evicts nothing; and unlike a per-run temp directory,
nothing cleans it up when the session ends. Prefer a disk-backed location under
`$HOME` keyed by worktree, such as `$HOME/.cache/<project>-targets/<slug>`, and
confirm the chosen filesystem has room before building.

**Remove what you created.** A hermetic directory created for a run is yours to
delete when the run ends. If you deliberately keep one — an incremental cache
worth preserving, for instance — say so in the report and say where it is.

## Write-gate probe (Step 4b detail)

If the runtime exposes a write gate or patch validator (for example an MCP tool
bound to a trusted workspace root), confirm it accepts the selected worktree path
before relying on patch-mode validation.

| Result                                   | Action                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------- |
| Worktree accepted                        | Record `write-gate: patch-mode` and continue                            |
| Worktree rejected but content reads work | Record `write-gate: content-mode`; validate by reading files + commands |
| Worktree rejected and validation blocked | Stop; ask to register the worktree path or choose another workspace     |

Do not abandon isolation just to satisfy a main-checkout-only write gate. The
degradation belongs in the report and checkpoint.

## Read-only tool probe (Step 4c detail)

Run read-only/orientation commands only after checking whether they create temp
merge trees, fixture updates, caches, registrations, or lockfiles. If a supposed
read-only command mutates or fails with `EROFS` / `permission denied`, classify it
as `tooling-sandbox-failure`, record the attempted path, and switch to a safer
content/Git inspection path.
