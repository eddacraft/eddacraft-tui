# Post-merge: feat-cib-184-unticked-mcp-picker

PR: #3279
Branch: `feat/cib-184-unticked-mcp-picker`
APS: CIB-184
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm `cargo test -p eddacraft-anvil --bins --
      mcp_picker_options_default_every_candidate_unticked
      demand_picker_enter_without_tick_writes_nothing
      demand_picker_ticked_selection_installs_only_ticked
      demand_picker_never_offers_unsafe_drift` passes on `main` post-merge
      (agent: yes)
- [ ] Manual TTY verification (unit tests cover `mcp_picker_options` and the
      injected-picker seam, not the rendered `demand::MultiSelect`): in a
      scratch repo with no `mcpServers.anvil` entries, run `anvil start` in an
      interactive terminal, reach the MCP client picker, and confirm every
      candidate renders unticked with the "Nothing is selected by default"
      description; press Enter without ticking and confirm no editor MCP
      config file is created or modified (agent: no — needs an interactive
      terminal; same acknowledged gap as CIB-165)

## Notes

CIB-184 (first-run council review C-009): the live plain-mode MCP demand
picker now matches the TUI consent posture — `NotPresent` / `SafeDrift`
candidates start unticked, Enter with nothing ticked writes no MCP config.
ADR-044's pinned pre-selected default was amended in the same PR
(2026-07-11 amendment section) and the activation as-built drift-policy
table updated. Non-interactive auto-install and the `UnsafeDrift` refusal
are unchanged.
