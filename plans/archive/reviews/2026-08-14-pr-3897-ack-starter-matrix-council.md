# Council Review — PR #3897 acknowledgements kit matrix adopt

**Status:** Converged (no-ship on reviewed head; repairs follow)
**Tier:** mini
**Target:** `tools/starters/acknowledgements/**` (kit contract, CI, release pin)
**Date:** 2026-08-14
**PR:** https://github.com/eddacraft/anvil-001/pull/3897
**Head reviewed:** `ef1b10b8c3571876f783ad5016a98cabffa20337` (`fix/ack-starter-matrix-adopt`)

## Change under review

ATTRIB-035..-038: treat cold-adopt as every canonical ecosystem, not Node-only.
Named-block markers, optional expander consumers, Python Trove→SPDX aliases,
Cargo/npm packaging notes, and Rust/Go/Python cold-start fixtures.

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| operations | no-ship | Kit Self-Tests red on ubuntu (`go-cold-adopt`); pin and Python CI snippet stale |
| adversarial | no-ship | Fail-open Python aliases; leftover `{{BLOCK_NAME}}` error unhelpful; Cargo `include`/`exclude` conflict |
| **judge** | **No-ship** | CI-red plus fail-open policy; do not rebase-merge this head |

## Findings

- **critical operations:** `go-cold-adopt` third-party `LICENSE` is a stub (`Permission is hereby granted, free of charge...`). `go-licenses` cannot classify it (`cannot find known license for …/thirdparty`). Ubuntu **Kit Self-Tests** 20 pass / 1 fail on this head. `tools/starters/acknowledgements/tests/go-cold-adopt.sh:42`
- **major adversarial:** `BSD-2-Clause` and `BSD-3-Clause` both alias `BSD License`; `GPL-3.0` / `GPL-3.0-only` and `LGPL-3.0` / `LGPL-3.0-only` share one Trove string each. A single-variant SPDX allow-list then accepts the generic classifier — fail-open. `tools/starters/acknowledgements/drivers/python-license-aliases.txt:11`
- **major adversarial:** leftover `{{BLOCK_NAME}}` is not a managed marker (`classify()` requires `^[a-z0-9]+(-[a-z0-9]+)*$`). The count gate reports `count: 0` for the real block and never names the placeholder. `tools/starters/acknowledgements/generate-acknowledgements.sh:675`
- **major adversarial:** README shows a closed Cargo `include` *or* `exclude`. Cargo treats them as mutually exclusive; a closed `include` drops crate files not listed. Document exclude-only. `tools/starters/acknowledgements/README.md:126`
- **minor operations:** `go.sh` tells the consumer to add the missing licence to `licences.toml` when `go-licenses` failed to classify a file. `tools/starters/acknowledgements/drivers/go.sh:170`
- **minor operations:** Python freshness snippet has `setup-python` only — no venv, no `pip-licenses`. `tools/starters/acknowledgements/ci-freshness.yml.snippet:31`
- **minor operations:** `MIRROR-README.md` pin example is still `v1.2.0` while this cut is `1.2.2`. `tools/starters/acknowledgements/MIRROR-README.md:56`

## Decision

**No-ship** on `ef1b10b8c`. Blocking repairs required before rebase-merge:

1. Full classifiable MIT `LICENSE` in the Go cold-adopt fixture.
2. Drop ambiguous dual-mapped Trove aliases (fail-closed).
3. Name leftover `{{BLOCK_NAME}}` in the marker-count error.
4. Cargo packaging docs: `exclude = ["tools/starters/**"]` only.
5. Python snippet: commented venv + `pip-licenses`.
6. Mirror pin example: `v1.2.2`.
7. Optional: `go.sh` unclassified-licence hint.

Council-gate green is a protected-path skip, not this review.

## Evidence

- Mini: operations + adversarial on PR #3897 head `ef1b10b8c`.
- GitHub Actions run 31805393100: Kit Self-Tests (ubuntu-latest) **FAILURE** — `go-cold-adopt`.
- `classify()` name regex at `generate-acknowledgements.sh:538`.
- Cargo book: `include` and `exclude` are mutually exclusive; `include` wins.

## Residual risks

- `v1.2.1` was never tagged; pin consumers on `v1.2.0` still miss Node cold-adopt.
- ATTRIB-025 (`--version`) remains Proposed.
- Generic Trove names (`BSD License`, generic GPLv3) will fail closed after the alias drop — correct, and needs a specific alias before those packages can pass.
