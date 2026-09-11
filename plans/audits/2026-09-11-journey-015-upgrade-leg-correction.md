# JOURNEY-015 — Upgrade-leg correction (#4591)

| Type  | Authority     | Owner | Status | Freshness |
| ----- | ------------- | ----- | ------ | --------- |
| Audit | Authoritative | JREL  | Live   | Last reviewed 2026-09-11 for #4591 upgrade-leg gate correction (previous-public invoke + hand-off verify). |

| Upstream | Downstream |
| -------- | ---------- |
| `scripts/journey/verify.mjs`, `plans/modules/release-user-journeys.aps.md`, JOURNEY-015 / JREL-012 | #4591 |

**Date:** 2026-09-11 (AWST)
**Authority:** JREL-012 / JOURNEY-015 residual; GitHub #4591
**Pinned current source:** recorded in
[2026-09-11-journey-015-upgrade-leg-identity.json](./2026-09-11-journey-015-upgrade-leg-identity.json)
**Previous public:** `/home/linuxbrew/.linuxbrew/bin/anvil` (Homebrew Cellar
`anvil/0.9.7-beta`)
**Platform:** Linux x86_64

## What was wrong

The 10 September JOURNEY-015 conductor run recorded `upgrade-previous-public` as
**pass** under `--require-upgrade`. The gate only checked that
`ANVIL_PREVIOUS_PUBLIC_BIN` named an executable, then ran the *current* binary
with `mcp serve --help`. The previous public binary was never executed, so any
executable satisfied the flag. That row is **not** upgrade proof. See the
correction banner on
[2026-09-10-journey-015-pinned-main-rehearsal.md](./2026-09-10-journey-015-pinned-main-rehearsal.md).
The original identity file is retained as the false-pass artefact.

## What the gate now requires

`--require-upgrade` fails unless:

1. The previous path is a distinct executable from the current binary.
2. Previous `--version` identifies as anvil.
3. Previous `mcp-config --json --target grok --scope project --write` produces a
   JSON hand-off (`path` + `wrote: true`) inside an isolated `--workspace`.
4. The current binary `mcp-config --json --verify` consumes that file
   (`ok: true`).

Host `HOME` / `ANVIL_HOME` are left in place so a licensed public binary can
run; the config write is confined to the isolated workspace. Distinguishable
fake binaries in `pnpm test:journey-gate` prove the previous process must run
and create the marker the current process consumes.

## Correction rehearsal

```text
ANVIL_PREVIOUS_PUBLIC_BIN=/home/linuxbrew/.linuxbrew/bin/anvil
node scripts/journey/verify.mjs --no-build --require-upgrade \
  --bin <current-debug-anvil> \
  --catalog plans/audits/2026-09-11-journey-015-upgrade-leg-catalog.json
```

Result: **pass** (`previousPublic.invoked: true`; previous sha256 ≠ current
sha256). Identity:
[2026-09-11-journey-015-upgrade-leg-identity.json](./2026-09-11-journey-015-upgrade-leg-identity.json).

This corrects the upgrade-leg evidence. It does not freeze a version claim or
authorise publication. Remaining JOURNEY-015 disposition (required non-upgrade
legs; platform waiver; no publication) is unchanged.
