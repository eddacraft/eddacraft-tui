# Post-merge: feat-kds-001-daemon-sink

PR: #2897
Branch: `feat/kds-001-daemon-sink`
APS: KDS
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [x] Mark KDS-001 + KDS-003 **Merged** in
      `plans/modules/kindling-daemon-sink.aps.md` (status + `Merged 2026-06-24 via
      PR #2897`) and bump the header / index count to 2/5 — done via PR #2898
- [x] Notify kindling (eddacraft/kindling) that the PORT-011 acceptance landed —
      done via **[eddacraft/kindling#124](https://github.com/eddacraft/kindling/issues/124)**.
      The remaining in-repo kindling flips (PORT-011 → Merged in
      `plans/modules/05-rust-port.aps.md`, the box in
      `plans/reviews/post-merge/feat-kinteg-001-publish-readiness.md`, and the index
      success criterion "anvil emits observations directly via `kindling-client` —
      no TS bridge" for the `command.invoked` path) are kindling's action, tracked
      in that issue (human required, kindling repo)
- [ ] Confirm `command.invoked` rows land in the Kindling daemon end-to-end with a
      real `kindling` binary on PATH and `ANVIL_KINDLING_SINK=daemon` (the in-process
      test daemon proves the wire contract; this confirms cold-spawn) (agent: no)

## Follow-ups (not blockers — tracked, out of PORT-011 scope)

- **KDS-002** — promote the env opt-in (`ANVIL_KINDLING_SINK=daemon`) to the full
  `daemon | ndjson | off` sink-selection surface, resolve `repo_id` from the
  emitting workspace root (not the client cwd default), and decide the D-035
  reconciliation (reword vs short ADR).
- **Spool growth cap** — the `SpooledClient` spool has no age/size trim like the
  NDJSON sidecar's council-T5 7-day / 64 MiB bound. Trimming without dropping
  un-delivered rows belongs in `SpooledClient` (`SpoolConfig` reserves the knob);
  raise upstream (kindling) and wire once available. Documented in the KDS module
  Risks. Default-off + opt-in bounds exposure meanwhile.
- **`gate.evaluated` parity** — route `gate_evaluated(save-time)` through the daemon
  sink (KDS-001 continuation); currently the daemon sink no-ops `try_emit`.
- **KINTEG-002 (upstream)** — daemon-side dedup-on-id makes the at-least-once spool
  exactly-once; removes the crash-mid-flush replay-duplicate edge.

## Notes

PORT-011 minimum scope was `command.invoked` only via `KindlingDaemonSink` in
`anvil-cli` + a parity test (NDJSON-vs-daemon stored row) and a spool-down →
restart → flush → retrieve replay test. The sink is **opt-in and default-off**
(`ANVIL_KINDLING_SINK=daemon`); the default capture path is unchanged NDJSON, so
the privacy contract is preserved. The parity / spool tests live as a
`#[cfg(test)]` module beside the sink (bin-only crate — no `tests/` library
target) and use an in-process `kindling-server` on a temp UDS.
