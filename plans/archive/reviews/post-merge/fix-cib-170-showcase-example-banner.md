# Post-merge test plan — CIB-170 showcase example banner in discovery

PR: #3127
Branch: `fix/cib-170-showcase-example-banner`
APS: CIB-170
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## What / why

On a clean repo (or scan failure) the discovery surface substitutes curated
fake findings distinguished only by an inline `[Example]` title prefix, so a
user could believe the demo secret at `src/services/auth.rs:42` is a real leak
in their own code.

The fix adds `is_showcase: bool` to `ScanResults` (preserved through
`filter_by_domain`, defaulted false at every construction site, `true` at both
`welcome.rs` showcase fallbacks). When set, `render_findings_list` swaps the
panel title for an "Example findings — your scan found no issues" banner and
prefixes each row with a reversed bold `EXAMPLE` badge. Real-scan renders are
unchanged; the `[Example]` title prefix in `showcase.rs` is kept for copy
robustness.

Touched:

- `crates/anvil-tui/src/surfaces/tutorial/discovery.rs` — `is_showcase` field,
  `filter_by_domain` preservation
- `crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs` — banner title
  + per-row badge, render tests
- `crates/anvil-cli/src/commands/welcome.rs` — sets the flag at both showcase
  fallbacks

## Gate commands run (pre-PR, all green)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p eddacraft-anvil-tui
cargo test -p eddacraft-anvil welcome
pnpm run format:check
```

## Steps

- [ ] Confirm the new render tests run green in the standard CI suite on
      `main` (push-event rust-tests job) (agent: yes)
- [ ] Visual spot check in a real terminal: run the welcome flow on a clean
      repo and confirm the discovery panel shows the "Example findings" banner
      and reversed `EXAMPLE` badges on every row; also try an 80-column
      terminal — the em-dash banner (~44 cols) clips in the 50% left panel,
      confirm ratatui truncates it gracefully with no garbling (human
      required)
- [ ] Real-scan regression check: run discovery on a repo with genuine
      findings and confirm the plain `Findings (n/m)` title renders with no
      `EXAMPLE` badge on any row (human required)

## Notes

The render test uses a 140-column `TestBackend` because the banner clips at
80 columns; the 80-column case is therefore only covered by the manual step
above. Crate package name is `eddacraft-anvil-tui` (not `anvil-tui`).
