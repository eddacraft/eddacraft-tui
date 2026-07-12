# JOURNEY-006 — v0.9.0-beta Release Execution Journal

**Goal:** Execute the operator-approved `v0.9.0-beta` cut (ESC-002, 2026-07-12)
through the `release` skill's deterministic steps, with evidence on tracking
issue [#3305](https://github.com/eddacraft/anvil-001/issues/3305).
**Outcome:** Published on both repos 2026-07-12T17:06Z; all three required
signing targets (both installers and the provenance manifest) signed and
independently verified by 20:13Z; verification and closeout recorded; durable
record at
[`plans/releases/v0.9.0-beta.md`](../releases/v0.9.0-beta.md).

---

## Timeline (all times UTC, 2026-07-12)

| Time   | Step                                                                                                                                                                                                                  |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 07:28  | `prepare.sh` metadata recorded on #3305 (prep commit `29c754baa` on source `9780769cb`).                                                                                                                              |
| 07:37  | Re-cut promotion PR #3306 opened (supersedes #3301); preflight 11/13 with the two documented host-environment test exceptions (inotify saturation + live-daemon leakage; hermetic runs and CI green).                 |
| ~09–10 | Changelog curation on the release branch: `2315c3952` (promote + curate) and `6b0ed1d1d` (Copilot-caught duplicate persistence bullet) — defect filed as CIB-196.                                                     |
| 10:35  | Operator merged #3306.                                                                                                                                                                                                |
| 10:36  | `release-readiness.yml` green on the source SHA (run 29189390970, headSha `6b0ed1d1d`).                                                                                                                               |
| 11:14  | Tag `v0.9.0-beta` pushed at `6b0ed1d1d`; release run 29190475570 attempt 1 — all six build legs + global artifacts green; `host` **failed pre-publish** at "Generate build provenance manifest": `gh: Bad credentials (HTTP 401)`. |
| 11:57  | Blocker documented on #3305: `ANVIL_RELEASES_TOKEN` expired (secret untouched since 2026-04-01). No partial state — nothing published on either repo.                                                                 |
| 13:12  | Recovery attempt 2 (`gh run rerun --failed`) reused retained artefacts; same 401. Parked as ESC-004 (PR #3308, later closed unmerged as moot).                                                                        |
| 17:04  | Operator rotated the `ANVIL_RELEASES_TOKEN` repo secret.                                                                                                                                                              |
| 17:06  | Attempt 3 green end-to-end: both releases published (public marked latest, `prerelease=false` per beta convention), Homebrew tap bumped (`Formula/anvil.rb` → `0.9.0-beta`), Scoop + WinGet legs green, announce ran. |
| ~17:5x | Verification evidence recorded on #3305 ([comment](https://github.com/eddacraft/anvil-001/issues/3305#issuecomment-4952168186)): releases + assets, provenance binding (`6b0ed1d1d` + run URL), installer 200, tap state. Comms parked. |
| ~18:00 | Closeout: final summary on #3305, public release marked latest, #3305 closed (completed), `release/v0.9.0-beta` branch confirmed deleted (removed at #3306 merge).                                                    |
| ~18–20 | Signing recovery hardened the workflow through PRs #3309, #3312, #3313, #3314, #3318, and #3319: immutable run/tag binding, permissions, dispatch validation, parser-compatible comments, portable key normalisation, raw binary key compatibility, and explicit passwordless signing. |
| 20:13  | Signing run 29207277673 succeeded: three `.minisig` assets self-verified, uploaded privately, and mirrored publicly. Independent local `rsign verify` passed for all three pairs; evidence recorded on #3305. |

## Verification evidence (summary)

- Private and public `v0.9.0-beta` releases published with the full cargo-dist
  and signature asset set (25 / 26 assets incl.
  `anvil-v0.9.0-beta-provenance.json`, its signature, and
  `release-evidence-v0.9.0-beta.md`).
- Provenance binds private commit `6b0ed1d1d…`, workflow run 29190475570
  (attempt 3), and public ref-at-publish `0a8c93a36`.
- `https://install.eddacraft.ai` → HTTP/2 200.
- `eddacraft/homebrew-tap` formula at `0.9.0-beta` (commit `b02b2d9a9`).
- Signing run 29207277673 self-verified both installer signatures and the
  provenance signature. Private/public `.minisig` SHA-256 digests match, and
  independent local verification with the configured public key passed; see
  the [signing closeout comment](https://github.com/eddacraft/anvil-001/issues/3305#issuecomment-4952610802).

## Follow-ups filed

- **CIB-196** — `prepare.sh` changelog promotion needs manual curation.
- **PR #3309** — publication-recovery hardening (merged).
- **Issue #3310** — deeper fake-`gh` integration coverage for the signing
  workflow.
- **Archive cascade** — modules shipped in this tag advance to
  `Released/Shipped` + archive in follow-up PRs (per the APS archive
  cascade); see the v0.10.0-beta window's closeout-hygiene phase in
  [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md).
