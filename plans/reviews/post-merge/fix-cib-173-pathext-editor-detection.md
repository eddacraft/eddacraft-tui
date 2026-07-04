# Post-merge: fix-cib-173-pathext-editor-detection

PR: #NNN
Branch: `fix/cib-173-pathext-editor-detection`
APS: CIB-173
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm `cargo test -p eddacraft-anvil --bin anvil -- pathext binary_in_dir`
      passes on `main` post-merge — 5 tests green (agent: yes)
- [ ] On a Windows machine (or the Windows CI leg), verify the rewired
      `cfg(windows)` branch of `RealDetectionEnv::has_binary` end-to-end:
      install an editor CLI that ships as a `.cmd` shim (e.g. VS Code's
      `code.cmd`), run `anvil init`/detection, and confirm the editor is
      detected and its MCP config is written — this branch only compiles on
      Windows, so the Linux dev box could not exercise it (agent: no —
      needs a Windows host)
- [ ] Reconcile CIB-173 status in
      `plans/modules/continuous-improvement-backlog.aps.md` to
      `Merged YYYY-MM-DD via PR #NNN` — omitted from this PR because the CIB
      count cell is shared with sibling branches (agent: no — parent
      reconciles batch status)

## Notes

CIB-173: `detect_agents.rs` only tried the bare binary name plus a hardcoded
`.exe` fallback on Windows, so editor CLIs shipped as `.cmd`/`.bat` shims were
missed and their MCP config was silently not written (default install is
detection-gated).

Fix: two pure, platform-independent helpers keep the lookup unit-testable
without mutating process env (`unsafe_code = "forbid"` blocks `set_var`):

- `pathext_candidates` — parses `PATHEXT`, bounds it case-insensitively to the
  standard executable set (`.exe`, `.cmd`, `.bat`, `.com`) preserving order,
  and falls back to the full set when unset/empty/no-intersection.
- `binary_in_dir` — per-directory lookup over a candidate list, keeping the
  `accept_bare` spoof guard for extensionless files.

The `cfg(windows)` branch of `RealDetectionEnv::has_binary` now iterates the
PATHEXT-derived candidates. Cross-platform regression coverage lands with the
PR (5 unit tests using temp-dir shims); the Windows-only wiring is the residual
risk this plan covers.
