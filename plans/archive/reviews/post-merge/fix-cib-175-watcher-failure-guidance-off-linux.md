# Post-merge: fix-cib-175-watcher-failure-guidance-off-linux

PR: #3147
Branch: `fix-cib-175-watcher-failure-guidance-off-linux`
APS: CIB-175
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm CI matrix ran `eddacraft-anvil-kernel` tests green on macOS and
      Windows runners for the merge commit — `watch_limit_guidance()` selects
      copy at runtime via `cfg!` (both branches compile in every build), while
      the per-platform *assertions* in the tests are `#[cfg]`-gated, so the
      non-Linux copy is only asserted on the macOS/Windows runners (agent: yes)
- [ ] On a watch-exhausted Linux box, run `anvil watch` and capture a
      transcript showing the inotify sysctl guidance on hard failure and the
      partial-registration exhaustion warning — validation step named in the
      CIB-175 plan entry (human required)
- [ ] On a macOS or Windows machine, force a watcher start failure (e.g.
      point `anvil watch` at a directory removed mid-start, or revoke read
      permission) and confirm the rendered line names a cause and a next step
      with no inotify/sysctl wording (human required)

## Notes

`failure_guidance(&notify::Error)` and `watch_limit_guidance()` live in
`crates/anvil-kernel/src/watcher/mod.rs`; the CLI appends the guidance as
anyhow context in `crates/anvil-cli/src/commands/watch.rs`, keeping the raw
notify chain intact for `--json`/debug output. The Linux inotify preflight in
`capacity.rs` is untouched. A dev box whose inotify watch limit
(`fs.inotify.max_user_watches`) is already exhausted cannot start `anvil watch`
at all, which is exactly the exhaustion path step 2 exercises.
