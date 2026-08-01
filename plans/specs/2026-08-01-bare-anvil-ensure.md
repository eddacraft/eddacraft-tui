# Bare `anvil` ensure surface — design notes

| Field | Value |
| ----- | ----- |
| Status | Accepted with ADR-114 (2026-08-01); implementing in ONSW for `v0.10.0-beta` |
| Date | 2026-08-01 |
| APS | [ONSW](../modules/bare-ensure.aps.md), [JOURNEY-011](../modules/release-user-journeys.aps.md) |

## Problem

1. Bare `anvil` exits 2 with help (CIB-177). Users who want an on-switch must
   learn `anvil start`.
2. `anvil start` re-offers MCP/workflow installs that were declined (disk still
   `NotPresent`). That is correct for reconfigure and wrong for daily ensure.

## Goals

- Bare `anvil` turns protection **on** for a known worktree without reinstall.
- `anvil start` remains the only interactive install/reconfigure path.
- No new durable "declined forever" preference store in v1.
- Non-interactive paths never hang.

## Non-goals

- Bare as wow-start / welcome replacement (welcome stays explicit).
- Bare installing hooks, workflows, or NotPresent MCP.
- Replacing `anvil status` diagnostics depth.
- Changing ACTTUI consent defaults on `start`.

## Behaviour matrix

| Situation | Bare `anvil` | `anvil start` |
| --------- | ------------ | ------------- |
| Never activated repo | Honest pointer (± daemon ensure only); no MCP/workflow write | Full activation + consent |
| Activated, healthy, MCP present | Ensure daemon + spine + MCP verify/SafeDrift; short confidence | Reconfigure available; may re-offer only if still NotPresent |
| Activated, MCP declined earlier | Ensure daemon + spine; one line "MCP not installed — run `anvil start`" | Re-offers MCP/workflows from disk state |
| Outside git worktree | Short refuse / advisory (pin exit in ONSW-002) | Existing ACTMO "outside worktree" honesty |
| `ANVIL_NO_MCP` | Skip MCP ensure | Skip MCP install |
| `--help` | Full catalogue + role blurb | Unchanged |
| CI / piped | Deterministic ensure, no prompt | Unchanged machine contracts |

## Implementation sketch

```text
main.rs root dispatch:
  if argv is bare (no subcommand) and not --help:
    commands::ensure::run(...)   // new thin module
  else:
    existing subcommand tree
```

`commands/ensure.rs` (name TBD) should call existing primitives only:

- `intercept::ensure_save_time_daemon` / DLIFE ensure
- ACTMO registration/attest helpers used by start (read-only + ensure membership
  without project write consent)
- MCP path: collect candidates → process only `UpToDate` / `SafeDrift`; skip
  `NotPresent` with recovery copy
- Render: reuse JOURNEY-003 compact human summary helpers where possible

Do **not** call `build_tui_consent_plan` or workflow pickers from bare.

## Contracts to pin (tests)

1. Bare interactive ensure exit 0 on healthy protecting worktree.
2. Bare does not create MCP config when NotPresent.
3. Bare does not write workflow files when absent.
4. After user declined MCP on start, bare still does not install; start still can.
5. `--help` still lists commands and names both roles.
6. Non-TTY bare never waits on stdin.
7. CIB-177 test updated or replaced for ensure path.

## Docs touch list

- `docs/runbooks/cli-surface.md` — root / bare section
- `crates/anvil-cli/src/help_layout.rs` — `FIRST_RUN_POINTER`
- Public quickstart / glossary if they claim "run anvil start every day"
- `docs/reviews/cli-command-truth-review.md` — CLICT note for root behaviour

## Open questions — resolved 2026-08-01 (ADR-114 Accepted)

1. **Never activated:** `activation::verify` config status is `Absent`.
2. **Default-on** in the implementing PR (no feature gate).
3. **Bare `--json`:** yes — global `--json` emits a compact ensure document.
4. **Exit codes:** `0` success; `1` not-activated or ensure failure (daemon
   failure included with recovery copy).
