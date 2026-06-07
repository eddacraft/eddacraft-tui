# Side-by-Side Candidate Install (`ANVIL_HOME`) — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                        |
| ------- | ------------- | ------ | ------ | ------------------------------------------------ |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-31 for DISTRIB-006 (ADR-060) |

| Upstream                                                                                                                                                                                                                                                                    | Downstream                                                                                            |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| [`plans/archive/modules/distribution-and-update.aps.md`](../../plans/archive/modules/distribution-and-update.aps.md) (DISTRIB-006), [ADR-060](../../plans/decisions/060-anvil-home-install-root-override.md), [ADR-036](../../plans/decisions/036-daemon-scope-discovery-and-boundaries.md) | Boring-Week candidate testing, [adoption runbook](anvil-adoption.md), `anvil status --json` consumers |

This runbook shows an internal developer how to run a pre-release Anvil
**candidate** alongside the **production** install — testing a new version
against a real project without cutting a release — using the `ANVIL_HOME` /
`--anvil-home` install-root override.

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
ANVIL_HOME="$HOME/.anvil-candidate" anvil status --json \
  | jq '{install_root, project_writes_gated}'
# { "install_root": "/home/you/.anvil-candidate", "project_writes_gated": true }
```

## Procedure

### 1. Pick a prefix and point the candidate at it

```bash
export ANVIL_HOME="$HOME/.anvil-candidate"
install -d -m 700 "$ANVIL_HOME"   # the daemon requires a 0700, user-owned prefix
```

> The daemon binds its socket and PID file directly under the prefix and
> enforces an owner-only `0700` directory (per ADR-036's runtime-dir hardening).
> A world-readable `mkdir -p` (umask 0755) prefix is **refused at bind time** —
> use `install -d -m 700` (or `chmod 700 "$ANVIL_HOME"` on an existing dir).

Run the candidate binary with the env var exported, or pass `--anvil-home`
per-command (the flag takes precedence over the env var):

```bash
./anvil-candidate --anvil-home "$HOME/.anvil-candidate" status
```

`--anvil-home` is applied by re-execing the candidate once with `ANVIL_HOME` set
in the child environment (the binary forbids `unsafe` env mutation), so the
override reaches every resolver and the spawned daemon coherently.

### 2. Authenticate the candidate (its user state is separate)

The candidate's credentials live under `<ANVIL_HOME>/user/`, so it does not see
your prod login. Either log in again under the prefix, or supply a token via
`ANVIL_LICENSE` for a non-interactive candidate.

### 3. Run the candidate against your real project

```bash
cd ~/work/my-real-repo
anvil status          # reads — unrestricted, against the real repo
anvil check           # reads — unrestricted
anvil watch           # exercises the candidate daemon on its own socket
```

The candidate daemon runs on `<ANVIL_HOME>/intercept.sock`, concurrent with the
prod daemon on its default socket. On an already-activated repo, `anvil start` /
`anvil watch` run normally against the real project state (reads only). On a
repo the candidate has never activated, activation runs **read-only** — it
prints a one-line notice and does **not** seed `.anvilrc`, `anvil/project-id`,
`.gitattributes`, or workflows. Pass `--touch-project-state` if you intend the
candidate to perform that first-run seeding.

### 4. If you need the candidate to write project state

```bash
anvil baseline --refresh --touch-project-state
```

Only do this when you intend the candidate's baseline/witness to become the
project's real state.

### 5. Tear down

```bash
# Stop the candidate daemon by its PID file (there is no `intercept stop`
# subcommand yet); the foreground daemon also stops on Ctrl-C.
kill "$(cat "$HOME/.anvil-candidate/intercept.pid")" 2>/dev/null || true
rm -rf "$HOME/.anvil-candidate"                              # remove candidate state
```

Production's `~/.config/anvil/`, its daemon socket, and its logs were never
touched.

## Verification checklist

- `ANVIL_HOME="$HOME/.anvil-candidate" anvil status --json | jq .install_root`
  shows the prefix; plain `anvil status --json | jq .install_root` is **absent**
  under prod.
- `anvil intercept status` (prod) and
  `ANVIL_HOME="$HOME/.anvil-candidate" anvil intercept status` (candidate)
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
