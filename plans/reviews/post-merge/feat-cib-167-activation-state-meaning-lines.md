# Post-merge: feat-cib-167-activation-state-meaning-lines

PR: <!-- pinned after PR creation -->
Branch: `feat/cib-167-activation-state-meaning-lines`
APS: CIB-167
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Manual state walkthrough on a real machine (the three new `meaning:`
      arms are environment-dependent; unit tests pin substrings but cannot
      drive the real diagnostic):
  - [ ] Fresh repo, no config: `anvil start --verify` → `needs_action` must
        carry a `meaning:` line saying the repo is not set up yet and naming
        `anvil start`.
  - [ ] Repo with the MCP entry written but the editor not restarted:
        `anvil start --verify` → `needs_action` `meaning:` must acknowledge
        the entry was written (no "has not written" claim) and point at
        restart + `anvil start --verify`.
  - [ ] Repo with only unsupported file types: `anvil start --verify` →
        `unsupported` `meaning:` must read as honest no-action ("not an
        error", "no rules").
  - [ ] Save-time-watch-only repo (no daemon attestation): `watching`
        `meaning:` must describe save-time watch as the weaker fallback and
        name the MCP graduation path.
  - [ ] Daemon-attested worktree (intercept spine active, no MCP client):
        `watching` `meaning:` must credit the daemon-backed spine, not
        save-time watch, and frame MCP as an optional upgrade.
  - [ ] `ready_restart_required` output byte-identical to pre-PR (its arm is
        untouched).
- [ ] Owner decision on Draft CIB-180: rename vs render-time gloss vs
      document for the `restart_handshake_verified` / `server_startable`
      tier tokens (rendered contract consumed by `--verify` scripts —
      deliberately out of CIB-167 scope).
- [ ] Reconcile CIB-167 status in the CIB module to
      `Merged YYYY-MM-DD via PR #NNN` (agent: no — parent reconciles, shared
      hot file).

## Notes

CIB-167 gives terminal-first users a plain-language `meaning:` line on the
activation states that previously had none (`crates/anvil-cli/src/activation/render.rs`):

- `needs_action` — new `needs_action_meaning(d)` branches on
  `ConfigStatus` + `highest_mcp_tier()`, mirroring
  `why_summary_for_needs_action`, so the copy never denies a written MCP
  entry (Council major on the first draft).
- `unsupported` — honest no-action copy: no registry-supported languages
  seen, explicitly not an error.
- `watching` — new `watching_meaning(d)` branches on
  `daemon_attestation.attests_worktree()`, so a daemon-attested worktree
  credits the intercept spine instead of save-time watch (Council major on
  the first draft).
- `ReadyRestartRequired` copy is untouched — `--verify` output for that
  state stays byte-identical. `Protecting`/`Error` intentionally emit no
  `meaning:` line. Replacing the `_ => None` catch-all with explicit arms
  keeps the match exhaustive if a new state is added.

Automated coverage: 5 render tests (one per new arm, plus the written-entry
NeedsAction and daemon-backed Watching honesty cases), each proven red
first. The manual walkthrough exists because the diagnostic inputs (real
editors, daemon attestation, unsupported-only repos) cannot be driven in
this environment.
