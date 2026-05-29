# Cargo Target Eviction Runbook

| Type    | Authority | Owner  | Status | Freshness                                                             |
| ------- | --------- | ------ | ------ | --------------------------------------------------------------------- |
| Runbook | Advisory  | DEVENV | Live   | Created 2026-05-29 for DEVENV-004 against ADR-057 (dev-env hardening) |

| Upstream                                                                                  | Downstream                                   |
| ----------------------------------------------------------------------------------------- | -------------------------------------------- |
| `scripts/cache/anvil-target-evict.sh`, `plans/decisions/057-dev-environment-hardening.md` | Operators of the shared concurrent-agent box |

## What this is

DEVENV-002 relocates each worktree's Rust `target/` onto `/home` at
`$ANVIL_TARGET_BASE/<worktree-name>` (default `~/.cache/anvil-targets/<name>`).
Those accumulate. `scripts/cache/anvil-target-evict.sh` reclaims the
least-recently-used ones when the target filesystem crosses a high-water mark.

## Safety model

The PreToolUse Bash hooks are no-ops, so the script is self-enforcing:

- **Fail-closed prefix guard** — it only deletes direct children of the resolved
  `$ANVIL_TARGET_BASE`; any path that resolves outside it (symlink, mis-set env)
  exits non-zero without deleting. It also refuses an unsafe base (`$HOME`,
  `/`).
- **Never evicts a live build** — skips a dir whose `.cargo-lock` is held (cargo
  holds it for the whole build/check/test/clippy) or whose newest file was
  touched within `--freshness-mins` (default 30).
- **Dry-run by default** — prints what it _would_ evict; only `--apply` deletes.

## Preview, then run

```bash
# Preview (no deletion):
scripts/cache/anvil-target-evict.sh
scripts/cache/anvil-target-evict.sh --json   # machine-readable

# Real run once you trust the preview:
scripts/cache/anvil-target-evict.sh --apply
# Tunables: --high-water 80 --low-water 70 --freshness-mins 30
```

## Install the timer (operator, optional)

Ship the dry-run by hand first for a cycle; flip to the timer once you've
confirmed it never selects a building dir.

```bash
mkdir -p ~/.config/systemd/user
for u in anvil-target-evict.service anvil-target-evict.timer; do
  sed "s#__ANVIL_REPO__#$HOME/Projects/src/anvil-001#" \
    "$HOME/Projects/src/anvil-001/scripts/cache/systemd/$u" > "$HOME/.config/systemd/user/$u"
done
systemctl --user daemon-reload
systemctl --user enable --now anvil-target-evict.timer
systemctl --user list-timers anvil-target-evict.timer   # verify schedule
journalctl --user -u anvil-target-evict.service -n 50    # inspect a run
```

Disable any time: `systemctl --user disable --now anvil-target-evict.timer`.

## Orphaned in-tree targets (manual, deliberate)

A shell that builds with neither `direnv` nor `wt` writes to the in-tree
`./target` on the full Projects mount. This is **not** auto-swept (a blind
cross-worktree sweep is too dangerous). Reclaim one deliberately, per worktree:

```bash
cargo clean            # from inside the worktree, or:
rm -rf ./target        # if cargo is unavailable
```
