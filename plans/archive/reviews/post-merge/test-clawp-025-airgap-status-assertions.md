# Post-merge: test-clawp-025-airgap-status-assertions

PR: #NNN
Branch: `test/clawp-025-airgap-status-assertions`
APS: CLAWP-025
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance CLAWP-025 status to `Merged YYYY-MM-DD via PR #N` in
      `plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md` (agent: yes — parent
      reconciles; the shared count cell in `plans/index.aps.md` is owned by the
      parent because sibling CLAWP branches share it)
- [ ] Confirm `cargo test -p eddacraft-anvil --test air_gapped` passes on `main`
      after merge (agent: yes)

## Notes

Pure-code test-hardening unit. No production code changed —
`status --verify --json` already runs the local activation probe and honours the
air-gap contract; the harness-real tests pass against actual production output,
so `foundRealBug = false`.

What changed (`crates/anvil-cli/tests/air_gapped.rs`, +130 / -0):

- `anvil_status_verify_json_exits_cleanly_with_no_network` and
  `anvil_status_verify_json_skips_auth_refresh_with_expired_credentials` now
  parse stdout as the `activation::render::render_json` diagnostic and assert
  the closed-set local-probe shape (`state`/`config`/`watch`/`baseline_present`
  keys present, `state` ∈ the six `ProtectionState::label` values, no
  network/auth cause in the structured `last_error`).
- Both tests assert the ABSENCE of network/auth/update error markers
  (`authentication_required`, `auth_check_failed`, `auth_error`, `device-flow`)
  on both stdout and stderr.
- No existing assertion was weakened: the "not killed by signal" exit check and
  the pre-existing `!stderr.contains("authentication_required")` check are both
  retained; the new content assertions are added on top.

Why this is positive evidence, not just absence-of-crash: a network attempt that
hangs and times out before the test budget would still exit cleanly and pass the
old assertions. It cannot, however, synthesise the local activation-diagnostic
JSON object — that comes from local state only — so a parse + closed-set shape
match proves the local probe ran without reaching outward.

A socket/connect-attempt detector in the harness (the "gold standard" the issue
mentions) was intentionally NOT added: the harness already strips the network
namespace via `unshare -n -r`, and adding a socket interposer risks flakiness on
this box for marginal extra coverage over the reliable JSON-shape +
no-error-marker assertions.

Gates run locally before commit (crate `eddacraft-anvil`):
`cargo fmt -p eddacraft-anvil`; `cargo fmt --all --check` (clean);
`cargo clippy -p eddacraft-anvil --all-targets -- -D warnings` (clean);
`cargo test -p eddacraft-anvil --test air_gapped` (4 passed).

APS bookkeeping deferred to the parent per the unit's policy: do NOT flip
CLAWP-025 status or touch `plans/index.aps.md` / the clawpatch module from this
branch — the count cell is shared with sibling CLAWP branches.
