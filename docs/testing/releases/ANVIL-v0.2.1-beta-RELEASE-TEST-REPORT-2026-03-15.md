# Anvil v0.2.1-beta Release Test Report (Edda/Ember/Stack + Tutorial)

| Type  | Authority  | Owner   | Status   | Freshness                                                                                       |
| ----- | ---------- | ------- | -------- | ----------------------------------------------------------------------------------------------- |
| Guide | Historical | RELEASE | Archived | Historical release evidence for `v0.2.1-beta`; metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                        | Downstream                       |
| ------------------------------- | -------------------------------- |
| `v0.2.1-beta` release artefacts | Historical release evidence only |

> Historical release evidence for `v0.2.1-beta` only.
>
> This report documents a point-in-time release test run against the old
> TypeScript CLI and its command surface. It is not current test guidance and
> should not be used as the source of truth for today's CLI, release process, or
> supported commands.

Date: 2026-03-15 Release: `v0.2.1-beta — Project Memory & Pattern Detection`
Source: https://github.com/eddacraft/anvil-001/releases/tag/v0.2.1-beta

## Test scope

- `edda` command group
- `ember` command group
- `stack` command group
- new tutorial flow (`anvil tutorial`)

## Environment

- Repo checked out at tag `v0.2.1-beta`
- Built full workspace via `pnpm nx run-many -t build --all` (after
  `pnpm nx sync`)
- CLI executed from local build: `node apps/anvil-cli/dist/index.js ...`

## Results summary

### 1) Command availability and help text

PASS (commands present and wired):

- `anvil edda list|show|promote|retire|trace`
- `anvil ember list|show|promote`
- `anvil stack status|validate`
- `anvil tutorial --list`

Observed output confirms all advertised groups are reachable.

### 2) Runtime behavior without auth token

EXPECTED BLOCK / PASS (guardrails functioning):

- `stack status --json`
- `stack validate --json`
- `edda list --json`
- `ember list --json`

All return: `Your session needs to be refreshed. Run anvil login to continue.`

Interpretation: commands are present, but functional end-to-end validation
requires authenticated session.

### 3) Tutorial flow

PASS (smoke):

- `anvil tutorial --list` shows available tutorials:
  - core, policies, architecture, drift, ci
- Interactive `anvil tutorial` launches correctly in TTY, runs scan step, and
  presents guidance/warnings.
- `anvil tutorial core --reset` works and clears progress artifacts.

### 4) Release-note parity checks

- Claim: Edda commands available → **PASS**
- Claim: Ember commands available → **PASS**
- Claim: Stack health commands available → **PASS**
- Claim: New tutorial flow exists → **PASS**
- Claim: drop-in upgrade with existing auth tokens/settings → **PARTIAL / NOT
  VERIFIED**
  - blocked by lack of authenticated session during this run.

## Notable mismatch / follow-up

Release note text says `anvil edda list` can browse by "type, confidence, or
age". Current help shows options:

- `--type`
- `--status`
- `--limit` No explicit `--confidence` or age filter shown in help.

Action: verify whether confidence/age filtering exists implicitly or release
copy should be corrected.

## Recommended next test pass (when Josh returns)

1. `anvil login`
2. Run full matrix:
   - `edda list/show/promote/retire/trace`
   - `ember list/show/promote`
   - `stack status/validate`
3. Verify persistence and evolution chain (`edda trace`) after a few
   `watch/check` runs.
4. Validate tutorial completion flow and artifact creation in
   `.anvil/tutorial*`.
