# CIB-040 Council — CLIC-010 Help-Text Layout Pass

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

CIB-040 applies the CLIC-010 uniform help layout to every visible `anvil`
command and adds a CI lint that prevents future drift. Implemented across four
commits on `fix/cib`:

- `2aa4e4404` docs(cib): plan CLIC-010 help layout pass (action plan + APS).
- `101f74988` feat(cib): add CLI help-layout command inventory.
- `43d2b9230` feat(cib): add CLIC-010 help-layout lint and close runbook gaps.
- `34a7fde1d` feat(cib): apply CLIC-010 help layout at runtime.
- `a150ab841` feat(cib): strip internal identifiers from user-visible help.

Changed surfaces:

- `crates/anvil-cli/src/help_layout.rs` (new) — runtime augmentation +
  test-only CLIC-010 lints.
- `crates/anvil-cli/src/main.rs` — augmented parse path + lint tests.
- `crates/anvil-cli/src/commands/{capsule,hook,intercept,start,watch,policy/eval}.rs`
  — internal-identifier scrub in clap-visible help.
- `docs/runbooks/cli-surface.md` — new `anvil ember` / `anvil kindling`
  sections; workspace when-to-use scrub.

## Council result

Run as a coordinator pass with adversarial and operations lenses (parallel
sub-agent spawning was unavailable this turn; explorer findings on metadata and
clap-augmentation risk informed the design). Decision: **proceed**.

### Adversarial lens

- Parse/dispatch parity: parsing now flows through `augment_clic_010_help` +
  `try_get_matches_from_mut` + `from_arg_matches_mut`. clap remains the
  parser/validator/help/version authority; the existing `main()` error branch
  (exit codes, `--json` envelope) is unchanged. Verified by
  `augmented_parse_preserves_command_dispatch`, the full 2285-test bin suite,
  and the `update`/`start`/`watch`/`format_flag` help/parse integration tests.
- Footer preservation: `after_long_help` folds any existing `after_help`
  (root EXIT CODES, watch daemon notes, uninstall scope) before appending the
  CLIC-010 footer. Root is not augmented (no when-to-use for the bare binary),
  so EXIT CODES is intact. Verified by
  `augmented_help_preserves_existing_watch_footer` and manual render of
  `--help`, `uninstall --help`.
- Runbook parsing robustness: `extract_when_to_use` collapses soft wraps and
  bounds the field at the next bold marker / paragraph break; returns `None`
  (lint failure, not panic) when absent.
- No internal identifiers: a second lint renders every visible command's long
  help and fails on leaked IDs; all leaks are scrubbed.

### Operations lens

- CI wiring: the CLIC-010 lints are `#[cfg(test)]` in the CLI bin crate and run
  via `cargo test --workspace` in `.github/workflows/rust.yml`
  (`rust.yml` line ~518). No exclusions.
- `include_str!` of `docs/runbooks/cli-surface.md` couples runtime help to the
  runbook; accepted as source-of-truth alignment (single canonical when-to-use
  surface). Noted as a future size/coupling tradeoff if the runbook grows large.

## Acceptance vs. CIB-040

- Every non-hidden command `--help` follows the four-section structure
  (summary + WHEN TO USE + COMMON FLAGS + LEARN MORE): **met** — runtime
  augmentation + coverage lint over all 110 visible paths.
- No internal identifiers / ADR / work-item IDs in user-visible text: **met** —
  enforced by `clic_010_help_excludes_internal_identifiers`.
- CLIC-010 CI lint asserts the layout, no exclusions: **met**.

## Validation evidence

```text
cargo test -p eddacraft-anvil --bin anvil            # 2285 passed
cargo test -p eddacraft-anvil --test start --test watch_daemon_lifecycle \
  --test update_resolution_chain --test policy_eval  # pass
cargo clippy -p eddacraft-anvil --all-targets -- -D warnings  # clean
cargo fmt --check -p eddacraft-anvil                  # clean
pnpm docs:check                                       # 8/8 surfaces pass
pnpm lint:md ; pnpm format:check                      # clean
```
