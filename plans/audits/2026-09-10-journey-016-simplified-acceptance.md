# JOURNEY-016 — Simplified journey acceptance and documentation closeout

**Date:** 2026-09-10 (AWST)
**Pinned source:** `5489c6112e61e9691b5c34e53db84bd73ccae4a3` (`origin/main` at evidence collection)
**Platform (local):** Linux x86_64 (deus)
**Claim:** #4613
**Conductor command:** `node scripts/journey/verify.mjs --no-build --require-upgrade`
**APS lint:** `node scripts/aps/active-lint.mjs`
**Docs check:** `pnpm docs:check`
**Identity:** [2026-09-10-journey-016-identity.json](./2026-09-10-journey-016-identity.json)
**Decision:** **pass** — JSIMP contract is implemented, documented and verified. No splash. No always-on app. No dashboard. Publication is **not** authorised.

## Purpose

Prove first-time and returning users experience one understandable journey after
JSIMP-001..006, against the same pinned build as `journey:verify`. Passing this
gate is simplification **acceptance**. It does **not** freeze a version claim or
authorise publication.

## Binary identity (local Linux)

| Field | Value |
| ----- | ----- |
| Binary | `/home/aneki/.cache/anvil-targets/journey-016/debug/anvil` (`anvil 0.9.7-beta`) |
| Binary sha256 | `f0e156701ce9021657fdfdca3fa1ab7bb5d23ed84b98ac6d542bd2c5c5104817` |
| Previous public | `/home/linuxbrew/.linuxbrew/bin/anvil` (Homebrew Cellar `anvil/0.9.7-beta`) |
| Platform | linux / x64 |
| Executed at | `2026-09-10T12:45:23.912Z` (20:45 AWST) |

Host intercept at `/run/user/1000/anvil/intercept.sock` was stopped for the
`daemon_identity` fixtures (they refuse to run against a live canonical
socket). Restore after this gate with bare `anvil` / `anvil start`.

## Journey verification matrix

JREL-012's non-skipping conductor, with JSIMP-006 e2e/help coverage on the same
catalog:

| Scenario | Local (deus) |
| -------- | ------------ |
| daemon-identity | pass |
| save-time-driver-recovery | pass |
| mcp-protection-claim | pass |
| mcp-stdio | pass |
| e2e-cli | pass |
| e2e-smoke | pass |
| upgrade-previous-public | **pass** (`--require-upgrade`) |

Local conductor: **pass (7 scenarios)** including the upgrade leg against the
previous public Homebrew binary.

Supporting pins on the same SHA:

- `cargo test -p eddacraft-anvil-tui --lib multi_section_consent_help` — pass
  (JOURNEY-013 help bar still advertised; no splash)
- `cargo test -p eddacraft-anvil --bin anvil -- intercept_has_no_ensure_or_restart_subcommand` — pass
- Intercept help names daily ensure as bare `anvil` and recycle as
  `anvil mcp refresh --daemon restart`

## First-time observation

Operator/agent observation of the **production CLI**, not a shadowed human
first-timer. Isolated `HOME` / `XDG_*` so MCP writes cannot touch the host
editor configs. Entitled legs used `ANVIL_DEV=1` (licence fixture only; no
`ANVIL_HOME`, so project state persists). Unsigned legs used a hermetic
`ANVIL_HOME` so credentials cannot resolve.

| Step | Command | Result |
| ---- | ------- | ------ |
| Install identity | `$BIN --version` | `anvil 0.9.7-beta` |
| Root help | `anvil --help` | Leads with `anvil welcome` / `anvil start` / daily `anvil`. Exit 0. |
| Never-activated unsigned | bare `anvil` | Exit **1**. Names `anvil start` and `anvil welcome`. No licence wall. No project write. |
| Never-activated JSON | `anvil --json` | Exit **1**. `surface: ensure`, `config: absent`, `next: run \`anvil start\` to activate`. Not an auth envelope. |
| Optional learning | `anvil welcome --no-tui` (no skip) | Exit 0. Names tutorial/doctor/`start --verify` without sign-in; next is `anvil start` then daily bare `anvil` and a closing receipt. |
| Ungated probe | `anvil start --verify` unsigned | Exit 0. `state: needs_action`, `config: absent`. Next names activation. |
| Gated JSON | `anvil start --json` unsigned | Exit **3** auth envelope. JSON stays read-only. |
| First useful proof | `anvil start --no-tui --no-daemon --no-mcp` entitled | Exit 0. Writes `.anvil.yaml`, hooks, baseline. First scan is honest clean. Receipt: project, coverage, `client: none (omitted)`, last proof `secret-detection caught 1 finding(s)`, next names `anvil start` to wire MCP or `--watch`. |

`--no-mcp` is the documented declined-integrations path, not a hidden switch.
MCP files under isolated HOME stayed absent (`.cursor/mcp.json`, `.claude.json`,
settings).

JOURNEY-013 already closed consent-picker navigation as fixed on a build that
carries `e586b6e53`. This item does not re-authorise a splash.

## Returning user and second repository

Same isolated HOME, later invocation, then a second git repository:

| Step | Result |
| ---- | ------ |
| Later session, repo A, bare `anvil` | Exit 0. Compact ensure: `protection: watching`, daemon started, worktree registered. MCP stays `not installed`. No picker. |
| Later session, repo A, `start --no-tui --no-daemon --no-mcp` | Exit 0. Compact. Receipt coverage: `mcp: not live; daemon: attesting worktree; save-time: attached`. Client remains omitted. |
| Repo B, unsigned bare `anvil` | Exit 1. Same never-activated pointer. Repo A config is untouched. |
| Repo B, entitled `start --no-mcp` | Exit 0. Own `.anvil.yaml` + closing receipt. MCP still omitted. Daemon reused. |
| Repo A `status --verify` after repo B | Exit 0. `state: watching`. Receipt still names repo A, omitted client, save-time demonstrated. |

Declined MCP stayed declined across resume and the second repository. Healthy
daily invocation did not re-offer a picker.

## Machine contracts

| Contract | Observed |
| -------- | -------- |
| Unsigned never-activated JSON | Existing ensure document, exit 1, no `authRequired` |
| `start --json` unsigned | Licence wall, exit 3, one JSON document, no writes |
| Entitled `start --json` after `--no-mcp` | Read-only `state: watching`; MCP clients remain `config_absent` |
| `--verify` | Non-mutating; unsigned skip stays ungated |

Compatibility matches [ADR-145](../decisions/145-continuous-command-journey.md)
and the [transition matrix](../specs/2026-09-10-continuous-command-journey.md).

## Residual usability (non-blocking)

None of these is a splash, always-on app, or dashboard, and none blocks
acceptance:

1. **JOURNEY-013 leftovers** — in-section ↑/↓ wrap, `a` versus Enter, 80-column
   quit truncation. Already recorded; not a key rewrite.
2. **Receipt `next` after save-time is armed** sometimes names
   `anvil intercept status` rather than daily `anvil`. Coverage and the next
   owner are still on-screen. Editorial; CIB-353 may absorb copy depth later.
3. **`status --verify` lists host MCP version skew** (live editor processes)
   even from an isolated HOME. Honest, noisy on a developer machine; not a
   first-timer block.
4. **Unsigned `start --verify` next** can say `anvil init` then `anvil start`.
   Direct `anvil start` already writes config. Editorial.

No blocking residual. No undocumented repair step was required to finish the
observed path.

## Release disposition (existing process)

Recorded against [RELEASE-PLAN.md](../../RELEASE-PLAN.md) active window
`v0.9.8-beta`:

| Gate | State after JOURNEY-016 |
| ---- | ----------------------- |
| Simplification acceptance | **Pass** (this record) |
| Claim freeze | **Not started** — theme/IDs still TBD |
| Changelog curation | **Not started** |
| Standing release gates | **Not claimed** by this gate |
| Publication authority | **Not granted** — no tag, no cut |
| Splash / always-on / dashboard | **Rejected** (unchanged) |

No internal release channel was used. No version bump. Version string remains
`anvil 0.9.7-beta` on the pinned debug binary.

## Validation commands

```text
node scripts/journey/verify.mjs --no-build --require-upgrade
cargo test -p eddacraft-anvil-tui --lib multi_section_consent_help
cargo test -p eddacraft-anvil --bin anvil -- intercept_has_no_ensure_or_restart_subcommand
```

`pnpm docs:check` and `pnpm aps:active-lint` run on the closeout change set.

## Decision

**JOURNEY-016 passes.** First-time and returning users can identify current
coverage and their next action from on-screen chrome. Setup resumes. Declined
integrations stay declined. Healthy daily `anvil` stays quiet. Machine contracts
have explicit compatibility treatment. JSIMP is not reopened. No splash.
Publication remains a separate authorised cut.
