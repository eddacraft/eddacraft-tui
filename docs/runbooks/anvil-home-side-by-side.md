# Rolling `main` Dogfood Channel — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                                                                                                                                          |
| ------- | ------------- | ------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Runbook | Authoritative | @aneki | Live   | Last reviewed 2026-09-12 for the explicit rolling `anvil-main` promotion path and isolated harness configuration. First filed 2026-05-31 for DISTRIB-006 (ADR-060) |

| Upstream                                                                                                                                                                                                                                                                                    | Downstream                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| [`plans/archive/modules/distribution-and-update.aps.md`](../../plans/archive/modules/distribution-and-update.aps.md) (DISTRIB-006), [ADR-060](../../plans/decisions/060-anvil-home-install-root-override.md), [ADR-036](../../plans/decisions/036-daemon-scope-discovery-and-boundaries.md) | Boring-Week candidate testing, [adoption runbook](anvil-adoption.md), `anvil status --json` consumers |

This runbook shows an internal developer how to promote and run the exact
fetched `origin/main` commit as `anvil-main` alongside the published `anvil`
install. This is a rolling dogfood channel, not a stamped release candidate.
Promotion is always explicit.

It replaces the old workaround (stop the prod daemon, symlink an `anvil-beta`
binary, test only under `/tmp`, and accept that user state leaks into the prod
install). With `ANVIL_HOME` the candidate and prod coexist cleanly.

## What `ANVIL_HOME` re-roots (and what it deliberately does not)

`ANVIL_HOME` re-roots only **install-owned** state under the prefix, so the
candidate never collides with production:

| State                    | Default location                        | Under `ANVIL_HOME=<P>` |
| ------------------------ | --------------------------------------- | ---------------------- |
| Daemon socket            | `$XDG_RUNTIME_DIR/anvil/intercept.sock` | `<P>/intercept.sock`   |
| Daemon PID file          | `$XDG_RUNTIME_DIR/anvil/intercept.pid`  | `<P>/intercept.pid`    |
| User state (credentials) | `~/.config/anvil/`                      | `<P>/user/`            |
| Kernel logs (panic log)  | `~/.local/state/anvil/`                 | `<P>/cache/`           |

Because the daemon keys its single-instance rule off the socket/PID path
([ADR-036](../../plans/decisions/036-daemon-scope-discovery-and-boundaries.md):
one daemon per `(uid, os)`), two distinct prefixes yield **two concurrent
daemons** with no clash.

What it does **not** re-root (per
[ADR-060](../../plans/decisions/060-anvil-home-install-root-override.md) Option
a): **per-project state** — `<repo>/.anvil/` (baseline, cache, witness) and
`<repo>/anvil/project-id` stay rooted at the project, so a candidate test runs
against the _real_ repo with witness continuity and baseline durability intact.
That is the whole point — a sandbox shadow would test nothing.

## The write-guard

Because per-project state is shared, an unreleased candidate could otherwise
silently overwrite a real project's baseline or witness chain. To prevent that,
under a non-default `ANVIL_HOME` durable per-project **mutations** are gated:

- **Gated by default (read-only / dry-run):** every durable per-project write —
  baseline refresh/write, witness append, cutoff pinning, project-identity
  mint/seed (`anvil/project-id`), `.gitattributes` witness lines, GitHub Actions
  workflow install, `.anvilrc` seed, the detected-agents cache, and git-hook
  install (`anvil hook bootstrap`), config rewrites (`anvil init`,
  `anvil migrate format`/`schema --apply`), `.anvil/` fixes
  (`anvil doctor --fix`), and drift snapshots (`anvil drift snapshot`). Explicit
  mutation commands (`anvil baseline`, `anvil init`, `anvil hook bootstrap`,
  `anvil migrate --apply`, `anvil doctor --fix`, `anvil drift snapshot`) are
  **refused** with a message naming the opt-in; incidental writes during
  `anvil start` / first-run `anvil watch`, commit-hook witness appends, the
  `anvil audit-chain` kindling sidecar, and the `anvil welcome` first-run marker
  are **skipped** (activation prints a one-line read-only notice; the commit is
  never blocked).
- **Unrestricted:** reads — `status`, `check`, `audit`, `watch` render — and the
  daemon path. These are exactly what you want to exercise during a candidate
  test.
- **Opt in** with `--touch-project-state` (or `ANVIL_TOUCH_PROJECT_STATE=1`)
  when you deliberately want the candidate to write the real project.

Check the posture at any time:

```bash
ANVIL_HOME="$HOME/.anvil-main" anvil-main status --json \
  | jq '{install_root, project_writes_gated}'
# { "install_root": "/home/you/.anvil-main", "project_writes_gated": true }
```

## Promotion

Prerequisites are Bash, Git, Cargo, and Python 3 on Linux or macOS. Python is
used for physical-path checks and atomic link replacement. `systemd-run` is
optional; the helper falls back to a startup-checked background process.

From any worktree in this repository:

```bash
scripts/dev/promote-main.sh
```

The helper fetches `origin/main`, resolves its full SHA, builds that detached
commit with the locked dependency graph, and installs it under
`~/.local/lib/anvil-main/<sha>/`. That immutable directory contains both the
binary and its `provenance.env`. Only after both are complete does the helper
atomically move `~/.local/bin/anvil-main` to that immutable binary;
`~/.local/lib/anvil-main/current` follows afterwards as a convenience link. A
failed build or provenance write therefore leaves the previous dogfood binary
and provenance selected together. The provenance records the source SHA, binary
hash, version output, and promotion time. Promotions are serialised, and build
artefacts are separated by source SHA.

It creates `~/.anvil-main` with mode `0700` and recycles only the daemon rooted
there. On systemd-based developer machines it owns a transient
`anvil-main-intercept.service`; elsewhere it uses the portable foreground daemon
entrypoint in the background. It does not stop or replace the published daemon.
Candidate daemon commands explicitly clear an ambient
`ANVIL_NO_SAVE_TIME_DRIVER`; the rolling channel must exercise that driver. Skip
the recycle when needed with `--no-restart-daemon`.

Check provenance without fetching or rebuilding:

```bash
scripts/dev/promote-main.sh --status
```

Status verifies that the channel resolves to the provenance-named immutable
binary and that its hash and version match before comparing the promoted SHA
with the local `origin/main` tracking ref. Run promotion again when it reports
`behind origin/main`; promotion performs the fetch that makes this comparison
authoritative.

### Authenticate before reconnecting harnesses

The candidate's credentials live under `<ANVIL_HOME>/user/`, so it deliberately
does not reuse the published install's login. On first use, complete a separate
device login, then promote once more so the authenticated candidate daemon is
restarted and its save-time drivers are exercised:

```bash
ANVIL_HOME="$HOME/.anvil-main" anvil-main auth login
scripts/dev/promote-main.sh
ANVIL_HOME="$HOME/.anvil-main" anvil-main auth whoami --json
ANVIL_HOME="$HOME/.anvil-main" anvil-main intercept status --json
```

Do not reconnect harnesses until `auth whoami` succeeds and candidate worktrees
report an active save-time driver rather than `failed`. For a non-interactive
candidate, `ANVIL_LICENSE` is the supported explicit token override. The helper
refuses to recycle the daemon while candidate authentication is missing.

## Harness configuration

Each dogfood harness must make both the binary and its isolated state explicit:

```text
command = "/home/you/.local/bin/anvil-main"
ANVIL_HOME = "/home/you/.anvil-main"
ANVIL_MCP_PREFERRED = "/home/you/.local/bin/anvil-main"
ANVIL_NO_SAVE_TIME_DRIVER = ""
```

Apply those values to the harness's existing anvil MCP entry without replacing
its unrelated settings. Do not export either variable globally in the shell:
that would make the published `anvil` command use candidate state. Do not run
`anvil mcp refresh` for this channel because refresh owns release-oriented MCP
entries; preserve the explicit dogfood command. Reconnect the harness after a
promotion so a new MCP process loads the promoted binary.

`ANVIL_MCP_PREFERRED` also ensures MCP re-exec stays on `anvil-main` if another
`anvil` appears earlier on `PATH`. The explicit empty save-time override
neutralises a stale parent-process opt-out without setting the flag active.

## Use and compare

Run the rolling channel explicitly:

```bash
ANVIL_HOME="$HOME/.anvil-main" \
  ANVIL_MCP_PREFERRED="$HOME/.local/bin/anvil-main" \
  anvil-main status --json
```

Run bare `anvil` to test the latest published release. Its default state and
daemon remain separate. A version string alone is not provenance: use
`promote-main.sh --status` to identify the dogfood SHA.

The daemon binds its socket and PID file directly under the prefix and enforces
an owner-only `0700` directory (per ADR-036's runtime-dir hardening). Wrong
ownership fails with a recovery instruction; do not start another foreground
daemon to work around it.

### Run the candidate against your real project

```bash
cd ~/work/my-real-repo
ANVIL_HOME="$HOME/.anvil-main" anvil-main status
ANVIL_HOME="$HOME/.anvil-main" anvil-main check
ANVIL_HOME="$HOME/.anvil-main" anvil-main watch
```

The candidate daemon runs on `<ANVIL_HOME>/intercept.sock`, concurrent with the
prod daemon on its default socket. On an already-activated repo, `anvil start` /
`anvil watch` run normally against the real project state (reads only). On a
repo the candidate has never activated, activation runs **read-only** — it
prints a one-line notice and does **not** seed `.anvilrc`, `anvil/project-id`,
`.gitattributes`, or workflows. Pass `--touch-project-state` if you intend the
candidate to perform that first-run seeding.

### If you need the candidate to write project state

```bash
ANVIL_HOME="$HOME/.anvil-main" \
  anvil-main baseline --refresh --touch-project-state
```

Only do this when you intend the candidate's baseline/witness to become the
project's real state.

### Stop the candidate daemon

```bash
ANVIL_HOME="$HOME/.anvil-main" anvil-main intercept stop
```

Production's `~/.config/anvil/`, its daemon socket, and its logs were never
touched.

### Recovery

If restart fails, the newly promoted binary remains selected but the helper
returns non-zero. With systemd, inspect
`journalctl --user-unit anvil-main-intercept.service`; for the background
fallback, inspect `${ANVIL_HOME:-$HOME/.anvil-main}/daemon.log`. Fix
authentication or the reported state-directory problem, and run promotion again.
Do not start a second candidate daemon against the same prefix.

If the promoted `main` itself is unsuitable, use bare `anvil` as the published
comparison path until `main` is repaired and promoted again; the helper does not
provide an unverified manual rollback path. If a terminated promotion left
`.promote.lock`, first confirm the recorded PID is no longer running, then
remove only that lock directory and rerun the helper.

## Verification checklist

- `ANVIL_HOME="$HOME/.anvil-main" anvil-main status --json | jq .install_root`
  shows the prefix; plain `anvil status --json | jq .install_root` is **absent**
  under prod.
- `anvil intercept status` (prod) and
  `ANVIL_HOME="$HOME/.anvil-main" anvil-main intercept status` (candidate)
  report two separate running daemons.
- After a gated `anvil baseline`, `git status` in the real repo shows
  `anvil/baseline.json` (and `anvil/project-id`) **unchanged**.
- Unsetting `ANVIL_HOME` returns byte-for-byte default behaviour.

## Limitations

- **Unix-first.** Socket/PID re-rooting targets Unix domain sockets. On Windows
  the daemon uses a named pipe keyed to the user SID; re-rooting two candidate
  daemons by prefix on Windows is a follow-up (the PID file re-roots via the
  prefix today). The documented side-by-side flow is Linux/macOS.
- **`ANVIL_HOME` should be absolute.** A relative value is absolutised against
  the current directory; export an absolute path to avoid ambiguity across the
  CLI and the daemon.
- Cross-version chain _format_ compatibility (a candidate writing a chain a
  different version reads) is out of scope — that is an `anvil migrate` concern
  (DISTRIB-005).
