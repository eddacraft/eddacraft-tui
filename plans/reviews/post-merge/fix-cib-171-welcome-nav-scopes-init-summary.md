# Post-merge: fix-cib-171-welcome-nav-scopes-init-summary

PR: #3131
Branch: `fix/cib-171-welcome-nav-scopes-init-summary`
APS: CIB-171
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Manual TUI smoke on a real terminal (agent shell has no TTY, so the
      interactive paths below were verified by unit/snapshot tests only):
  - [ ] `anvil welcome` → pick a hub entry that reaches the discovery screen →
        press `Esc` — must return to the hub menu, NOT advance into the
        tutorial.
  - [ ] From the hub, open gate, audit, and doctor — footers must read
        "esc menu / q quit anvil" (not "esc/q quit").
  - [ ] Run gate/audit/doctor standalone (e.g. `anvil doctor`) — footer copy
        unchanged from before this PR.
  - [ ] Complete the init wizard for each format choice — the landing summary
        must name `.anvilrc` (the file actually written), and the format picker
        labels must describe the in-file serialisation, not fictional
        `.anvil.yaml`/`.anvil.json`/`.anvil.toml` files.
- [ ] Reconcile CIB-171 status in the CIB module to
      `Merged YYYY-MM-DD via PR #NNN` (agent: no — parent reconciles, shared
      hot file).

## Notes

CIB-171 fixed three navigation/copy traps in the welcome flow:

- `Esc` (`SurfaceExit::Back`) on the discovery screen backed out via a new
  `discovery_outcome` extraction instead of silently advancing into the
  tutorial (`crates/anvil-cli/src/commands/welcome.rs`).
- `generate_config` now returns `GeneratedConfig { config_path,
  gitignore_updated }`; the summary and `ConfigFormat` labels derive from a
  single `CONFIG_FILE_NAME` (always `.anvilrc`), so the wizard no longer
  promises files it never writes (`crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-tui/src/surfaces/init/mod.rs`).
- Gate/audit/doctor states gained an `embedded()` flag so hub-hosted footers
  name the honest `q`/`Esc` scope while standalone copy is unchanged
  (`crates/anvil-tui/src/surfaces/{gate,audit,doctor}/mod.rs`).

Automated coverage: unit tests on key handling and `help_text` for each
surface, plus two regenerated init snapshots for the relabelled formats. Red
was proven before each fix. The manual smoke above exists because the footer
rendering and end-to-end Esc navigation are interactive paths this
environment cannot drive.
