# CIB-047 Mini Council — Watch TUI Daemon Fallback Indicator

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

CIB-047 surfaces the DSV-007 save-time daemon fallback warning inside the watch
TUI. Before this change, the plain/non-TUI path warned once per disconnect, but
TUI mode suppressed stderr while the alt-screen was active.

Changed surfaces:

- `crates/anvil-cli/src/commands/watch.rs`
- `crates/anvil-cli/src/commands/tutorial.rs`
- `crates/anvil-cli/src/commands/welcome.rs`
- `crates/anvil-tui/src/surfaces/watch/mod.rs`
- `crates/anvil-tui/src/surfaces/watch/event_adapter.rs`
- `crates/anvil-tui/src/surfaces/watch/render.rs`
- watch demo/sample initialisers under `crates/anvil-tui/`

## Council result

Operations reviewer returned PASS. The adversarial reviewer returned no content
before closeout, so deterministic test coverage and the operations review are the
binding evidence for this slice.

## Implemented behaviour

- `ActionResultLine` now carries an optional `DaemonNotice` update.
- On the first `SaveTimeDecision::FellBack { warned: true, .. }` in TUI mode,
  the dispatcher sends `DaemonNotice::Fallback` on the existing action-result
  channel instead of writing to stderr.
- Consecutive fallback cycles do not re-send the notice because the existing
  `warned` latch remains authoritative.
- On the next daemon `Validated` verdict, the dispatcher sends
  `DaemonNotice::ClearFallback` so the TUI removes the notice.
- Non-TUI behaviour is unchanged: the existing `tracing::warn!` and human stderr
  advisory remain behind `!self.tui_parent`, with the existing JSON guard for the
  unstructured advisory line.
- The TUI renders the notice in the footer strip with warning/error emphasis and
  prioritises it above update/insights hints while active.

## Validation evidence

RED evidence:

```text
cargo test -p eddacraft-anvil-tui daemon_ -- --nocapture
```

The new tests failed before implementation because `DaemonNotice`,
`daemon_notice`, and `daemon_fallback_notice` did not exist.

Final Rust validation passed:

```text
cargo test -p eddacraft-anvil-tui surfaces::watch:: -- --nocapture
cargo test -p eddacraft-anvil commands::watch::tests
cargo test -p eddacraft-anvil watch_save_time::tests
cargo fmt --check
cargo clippy -p eddacraft-anvil-tui --all-targets -- -D warnings
cargo clippy -p eddacraft-anvil --all-targets -- -D warnings
```

Observed results:

- `eddacraft-anvil-tui surfaces::watch::`: 59 passed.
- `eddacraft-anvil commands::watch::tests`: 78 passed.
- `eddacraft-anvil watch_save_time::tests`: 17 passed.
- Clippy passed for both touched crates.

## Notes

The action-result isolation invariant was updated: action outcomes may now
mutate only footer/action state (`last_action` and the daemon fallback notice),
not kernel-derived status/history/stats.
