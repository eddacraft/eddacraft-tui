# Post-merge verification: fix/cib-072 (CIB-072)

**APS:** CIB-072 — clear `ready_restart_required` on Windows when daemon attestation is Unreachable  
**Issue:** [#2609](https://github.com/eddacraft/anvil-001/issues/2609)  
**Branch:** `fix/cib-072`

## Automated (CI)

- [x] `cargo test -p eddacraft-anvil-intercept --lib ensure` — Linux host, 19 passed
- [x] `cargo test -p eddacraft-anvil -- activation` — Linux host, 304 passed
- [ ] Windows runner: `cargo test -p eddacraft-anvil -- activation::daemon_evidence::tests::end_to_end_against_real_named_pipe_promotes_to_live_validation`
- [ ] Windows runner: full `rust.yml` green

## Manual (Windows / Scoop / PowerShell)

Requires a Windows machine with Anvil installed and MCP client configured.

1. [ ] Run interactive `anvil start` — confirm daemon auto-starts (no `platform_unsupported` lifecycle line)
2. [ ] Restart editor so MCP client handshake completes
3. [ ] Run `anvil start --verify` — confirm `Protecting` / `LiveValidation` (not stuck `ready_restart_required`)
4. [ ] If daemon cannot start, confirm DLIFE-006 terminating copy (not open-ended restart loop)

## Closeout

- [x] Mark CIB-072 **Merged** in `plans/modules/continuous-improvement-backlog.aps.md` and bump count to 56/101
- [ ] Close #2609 with PR link