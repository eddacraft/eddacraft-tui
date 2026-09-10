# JOURNEY-015 — Pinned-main journey rehearsal and existing release handoff

**Date:** 2026-09-10 (AWST)
**Pinned source:** `3f8890e15cc498b60b681ff67d1b4254c39f0adf` (`origin/main` at evidence collection)
**Platform (local):** Linux x86_64 (deus)
**Claim:** #4572
**Conductor command:** `node scripts/journey/verify.mjs --no-build --require-upgrade`
**APS lint:** `node scripts/aps/active-lint.mjs` — 140 files checked, all clean
**Release plan check:** `node scripts/docs/check-release-plan.mjs` — ok (active window `v0.9.8-beta`, not yet tagged)
**Identities:**
- Local (upgrade): [2026-09-10-journey-015-identity.json](./2026-09-10-journey-015-identity.json)
- CI (ubuntu): [2026-09-10-journey-015-ci-identity.json](./2026-09-10-journey-015-ci-identity.json) from CI run `34423562600`

## Purpose

Verify what a user will install from a recorded main SHA using the repository's
supported build/CI process and the existing public release process — without
building an internal release channel. Passing this gate unlocks JSIMP /
simplification planning; it does **not** freeze a version claim or authorise
publication.

## Binary identity (local Linux)

| Field | Value |
| ----- | ----- |
| Binary | `/home/aneki/.cache/anvil-targets/journey-015/debug/anvil` (`anvil 0.9.7-beta`) |
| Binary sha256 | `0b9a23100e759ead3fb43ff7af8b1444b685df922f1adcb7bd170f66e8c5c05f` |
| Previous public | `/home/linuxbrew/.linuxbrew/bin/anvil` (Homebrew Cellar `anvil/0.9.7-beta`) |
| Platform | linux / x64 |
| Executed at | `2026-09-10T01:19:41.416Z` (09:19 AWST) |

## Journey verification matrix

| Scenario | Local (deus) | CI (ubuntu, run 34423562600) |
| -------- | ------------ | ---------------------------- |
| daemon-identity | pass | pass |
| save-time-driver-recovery | pass | pass |
| mcp-protection-claim | pass | pass |
| mcp-stdio | pass | pass |
| e2e-cli | pass | pass |
| e2e-smoke | pass | pass |
| upgrade-previous-public | **pass** (`--require-upgrade`) | not-supplied (optional in CI) |

Local conductor: **pass (7 scenarios)** including the upgrade leg against the
previous public Homebrew binary.

## Interactive / client-call coverage

Automated e2e on the pinned binary exercised real CLI surfaces rather than
config/PID presence alone, including:

- MCP config install and Claude allow rules (activation)
- `--no-mcp` spine activation without MCP writes
- Daemon repair guidance when MCP is wired but daemon evidence is absent
- Save-time driver / smoke paths (e2e-smoke + cargo save-time recovery)
- MCP stdio serve and protection-claim cargo tests

Human first-timer navigation observation remains owned by **JOURNEY-013** and is
not re-run here.

## Cross-platform matrix

| Platform | Same-pin journey:verify | Notes |
| -------- | ----------------------- | ----- |
| Linux | **pass** (local + CI) | Authoritative for this gate |
| macOS | not executed on pin | CI Platform Smoke / Release Gate skipped on this docs-path push |
| Windows | not executed on pin | Same — no same-pin matrix job in this lane |

**Operator waiver (recorded by merge of this PR):** for JOURNEY-015 handoff only,
macOS and Windows same-pin `journey:verify` legs are waived. Linux conductor
evidence plus CI journey-verify on `3f8890e15` is accepted. Cross-platform
Release Gate / Platform Smoke remain required before any publication cut of
`v0.9.8-beta` (or successor). A platform pass is never inferred from Linux.

## Release disposition (existing process)

Recorded against [RELEASE-PLAN.md](../../RELEASE-PLAN.md) active window
`v0.9.8-beta`:

| Gate | State after JOURNEY-015 |
| ---- | ----------------------- |
| Pinned-main rehearsal | **Pass** (this record) |
| Claim freeze | **Not started** — theme/IDs still TBD |
| Changelog curation | **Not started** |
| Standing release gates | **Not claimed** by this gate |
| Publication authority | **Not granted** — no tag, no cut |
| Simplification start | **Permitted** (JSIMP may begin; not started in this lane) |

No internal release channel was used. No version bump. Version string remains
`anvil 0.9.7-beta` on the pinned debug binary.

## Validation commands

```text
node scripts/journey/verify.mjs --no-build --require-upgrade
node scripts/aps/active-lint.mjs
node scripts/docs/check-release-plan.mjs
```

CI journey-verify on push `3f8890e15` (run `34423562600`) also green.
Full local validate:full was not re-run end-to-end in this short lane; the green CI
push on the pinned SHA plus the conductor gate above is the recorded substitute.

## Decision

**JOURNEY-015 passes** under the operator platform waiver above. JSIMP remains
unstarted in this lane. JOURNEY-016 stays gated on JSIMP delivery.
