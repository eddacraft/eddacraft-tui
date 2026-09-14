# New-User Journey Audit — `anvil 0.6.2-beta` (Homebrew)

> **Date:** 2026-05-21
> **Owner:** Wow-start activation verification
> **Purpose:** Walk the brand-new-user experience end-to-end against the
> shipped Homebrew binary (`anvil 0.6.2-beta`) and surface gaps between
> the documented wow-start journey (see `docs/public/beta/quickstart.md`,
> `README.md`) and the actual binary behaviour.
> **Scope:** What a developer sees from `brew install` through `anvil start`
> through the MCP pre-write catch path. Not a release-blocking audit; a
> friction inventory.

## Method

Sandboxed run with `HOME` redirected to a tmp dir so `~/.cursor/mcp.json`
and `~/.claude.json` mutations stayed isolated:

```bash
SANDBOX=$(mktemp -d /tmp/anvil-newuser-XXXXXX)
export HOME="$SANDBOX/home"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_STATE_HOME="$HOME/.local/state"
export XDG_CACHE_HOME="$HOME/.cache"
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"
mkdir -p "$SANDBOX/repo" && cd "$SANDBOX/repo"
git init -q -b main
git config user.email "test@example.com"
git config user.name  "Test User"
# tiny TS demo: package.json, src/index.ts, src/db.ts
```

Authentication is gated end-to-end on the shipped binary; without an invite,
every interesting command returns `Authentication required`. To verify the
post-auth journey, `ANVIL_DEV=1` was used (documented local override on
`cli.licence-gate`; see `crates/anvil-cli/src/feature_flags.rs:13`).

Commands exercised:

```
anvil --version
anvil welcome --no-tui                      # pre-bypass
anvil doctor  --no-tui                      # pre-bypass (works)
anvil status  --no-tui                      # pre-bypass
anvil start   --no-tui                      # pre-bypass (EXIT=3)
anvil init    --no-tui                      # pre-bypass
anvil check src/   --no-tui                 # pre-bypass

ANVIL_DEV=1 anvil welcome  --no-tui
ANVIL_DEV=1 anvil start    --no-tui
ANVIL_DEV=1 anvil status   --no-tui
ANVIL_DEV=1 anvil check src/{smelly,index}.ts \
              [--include-opt-in] [--severity info|warning] --no-tui
ANVIL_DEV=1 anvil check --changed --staged --no-tui
ANVIL_DEV=1 anvil gate    --list-profiles --no-tui
ANVIL_DEV=1 anvil gate    --no-tui
ANVIL_DEV=1 anvil gate -p ai --no-tui
ANVIL_DEV=1 anvil audit   --no-tui
ANVIL_DEV=1 timeout 4 anvil watch --no-tui

# MCP catch path
ANVIL_DEV=1 anvil mcp serve --stdio   # driven via JSON-RPC tools/call
```

Synthetic smell file (`src/smelly.ts`) contained both an `sk-…` hardcoded
API key on line 1 and an `eval(input)` antipattern. A separate
`src/aws.ts` contained the textbook AWS example access-key pair
(`AKIAIOSFODNN7EXAMPLE` / `wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY`).

## Wins (worth keeping)

- **`anvil doctor`** works without auth and triages cleanly: each warning
  carries the exact `run:` / `fix:` / `docs:` command. Best command in
  the surface today.
- **`anvil start` (second run on fresh repo, with auth)** prints the
  documented `ACTIVATION` panel with the protection-state literal
  (`ready_restart_required`), MCP install paths, baseline state, and a
  per-language coverage table.
- **`anvil gate`** caught the planted `sk-…` secret in `src/smelly.ts`
  via `secret-detection`. Scored result, clear failing check.
- **MCP `anvil_validate_write` (with creds/bypass)** returned
  `decision: block` for the smelly `proposedContent` write with a
  diagnostic id, location, remediation hint, and `do-not-write` safe
  default. Marketed catch path is real.
- **`anvil watch`** streams `[violation]` and `[snapshot]` events
  per the documented contract.

## Findings (filed)

Each finding has a corresponding GitHub issue cross-referenced below.

**Status legend:**

- `Open` — GitHub issue open, no code work merged yet.
- `In Progress` — branch / PR open against the owning module.
- `Merged` — fix merged to `main`.
- `Released` — fix shipped in a tagged release; cross-reference the
  release record under `plans/releases/`.

The Status column reflects the lifecycle from the new-user-journey
audit's point of view; the owning APS module's task carries the
authoritative status for the implementation work.

| # | Severity | Finding | Issue | Module · Task | Status |
|---|---|---|---|---|---|
| 1 | High | Every interesting command (`welcome`, `status`, `start`, `init`, `check`, `gate`, `audit`, `watch`) returns `Authentication required` before any output. The product thesis is "planless-first / value without config", but the binary is gated end-to-end. `welcome` arguably should never demand auth. | [#1795](https://github.com/eddacraft/anvil-001/issues/1795) | FLAGCAT · FLAGCAT-008 | Open |
| 2 | High | After `anvil start` installs the MCP entries, the MCP server returns `decision: block, code: authentication-required` for **every** `anvil_validate_write` call until the user runs `anvil auth login`. Editors that honor the published instructions ("Honour `block` decisions") would refuse to write any file. | [#1796](https://github.com/eddacraft/anvil-001/issues/1796) | MLP2 · MLP2-072 (Group Q) | Open |
| 3 | High | `anvil check src/smelly.ts` (planless) reports **no warnings** on a file with a hardcoded API key and `eval(input)`. JSON shows `checksRun: ["architecture"]` — `secret-detection` and `antipattern-scan` listed in `.anvilrc` do not run under the planless `check` path even though they run fine under `gate` and over MCP. | [#1797](https://github.com/eddacraft/anvil-001/issues/1797) | CIB · CIB-008 | Open |
| 4 | Medium | `anvil audit` says "0 issues — project looks clean" on the same repo where `anvil gate` reports the secret as a failure. Audit and gate disagree about the same files. | [#1798](https://github.com/eddacraft/anvil-001/issues/1798) | CIB · CIB-009 | In Progress |
| 5 | Medium | MCP pre-write summary double-counts findings (one secret on one line returns `summary.total = 2` with two identical diagnostics). | [#1799](https://github.com/eddacraft/anvil-001/issues/1799) | MLP2 · MLP2-073 (Group Q) | Open |
| 6 | Medium | The textbook AWS access-key pair (`AKIAIOSFODNN7EXAMPLE` + secret-key) is **allowed** by the MCP gate (0 diagnostics). Detector trips on `sk-…` via high-entropy heuristic but misses AWS-prefix patterns. **Fixed by PR #1815: high-confidence shape patterns (AWS, GitHub, Slack, Stripe, …) now bypass the `looks_like_code` filter and the keyword allowlist that were silently dropping `AKIA…` matches; STS (`ASIA…`), Anthropic, and OpenAI patterns added.** | [#1800](https://github.com/eddacraft/anvil-001/issues/1800) | SEC · SEC-008 | Merged |
| 7 | Low | `eval(input)` antipattern not caught on TS by any of `check`, `audit`, `gate`, watch, or MCP. | [#1801](https://github.com/eddacraft/anvil-001/issues/1801) | LANGTS · LANGTS-006 | Open |
| 8 | Low | `anvil watch` flags every existing exported symbol as `public-api-expansion` on a never-baselined repo (e.g. a one-line `greet()` helper). Conflicts with the "new edges only" principle for first scan. **Already fixed post-`v0.6.2-beta` by WATCHUX-001 (`8242affb`/`2929847c`) — the old `evaluate_baseline` path was removed so the initial graph is treated as baseline. Audit ran against the shipped Homebrew binary which still carried the bug. First release that ships the fix: `v0.6.3-beta`.** | [#1802](https://github.com/eddacraft/anvil-001/issues/1802) | CIB · CIB-010 | Released |
| 9 | Low | `anvil gate -p ai` reports 3 failures (`import-boundaries`, `policy`, `command-safety`) solely because their config files don't exist. New user activating with the suggested AI profile sees a 1/5 score with no next-step guidance. | [#1803](https://github.com/eddacraft/anvil-001/issues/1803) | CIB · CIB-011 | Open |
| 10 | Low | `anvil check --staged` errors with `required arguments were not provided: --changed`. `--staged` should imply `--changed`. | [#1804](https://github.com/eddacraft/anvil-001/issues/1804) | CIB · CIB-012 | Open |

**Progress:** 1 Released · 1 Merged · 1 In Progress · 7 Open _(as of 2026-05-21)_.

## Not-findings (initially flagged, dropped on re-test)

- **Silent first run of `anvil start`** — original observation could not
  be reliably reproduced. Repro attempt on a fresh repo with redirected
  `HOME` showed `EXIT=3` + auth message on the no-bypass run (writing
  nothing), and `EXIT=0` + full activation output on the bypass run.
  Original measurement likely a `tee`-pipeline artifact (captured
  `$?` of `echo`, not of the pipeline). Dropping.

## Linked memories

- A personal auto-memory note (`reference_anvil_validate_write_auth`,
  local to the auditor's workstation, not in this repo) already records
  the MCP auth-gate behaviour as "gate-unavailable, not a content
  veto". Finding #2 above is the user-facing side of that same
  behaviour: real users without an in-memory escape hatch will see
  their AI editor refuse to write.
