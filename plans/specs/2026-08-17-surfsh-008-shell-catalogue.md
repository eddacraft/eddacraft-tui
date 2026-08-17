# SURFSH-008 — shared shell-only command-safety rules

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for SURFSH-008 design | [SURFSH](../modules/surface-shell.aps.md) | Accepted | 2026-08-18 — Council BLOCK repairs |

| Upstream | Downstream |
| -------- | ---------- |
| [Language and coverage](./2026-04-08-language-and-coverage-design.md) §8.3 row 4; [command-safety surfaces](./2026-04-21-command-safety-surfaces-design.md); [SURFSH](../modules/surface-shell.aps.md); SURFDOCK pipe-to-shell matcher | SURFSH-008 Ready item; later SURFSH-009 (unquoted variables) |

**Execution authority** is SURFSH-008. This document is the design contract
that item implements. It does not authorise product-code merge without the
usual branch → validation → Council → PR path.

## 1. Problem

SURFSH-002 shipped a static `.sh` scan that reuses
`command_safety::{parse_compound_command, analyse_command}` against
`default_filesystem_rules()`. That catalogue is single-command (`rm` / `chmod`
/ `dd` / `mkfs`). Both consumers then drop pipeline operators, so
`curl … | sh` becomes `curl` and `sh` and matches nothing.

The language-coverage catalogue still owes pipe-to-shell, `eval` on
user-controlled input, and `chmod 777`. Unquoted variables in destructive
contexts need quote tracking the tokenizer does not keep — they are parked.

## 2. Approach

One catalogue, two consumers. No `CommandRule` schema change.

- New `default_shell_rules()` (`CommandCategory::Shell`) holds `eval` and
  `chmod 777` as ordinary `CommandRule` rows.
- Pipe-to-shell is a **compound** check on `(commands, operators)`.
- Both consumers call one `analyse_compound` helper instead of looping
  segments and discarding operators.
- Runtime `load_rules` concatenates git + filesystem + **shell** rules.
- SURFSH `run_surfsh_check` uses the same shell rules and the same helper.

Dockerfile keeps its local string matcher in this slice. A later cleanup may
call the shared helper; that is not SURFSH-008.

## 3. Rules

| Rule id | Pattern | Runtime action | Severity |
| ------- | ------- | -------------- | -------- |
| `pipe-to-shell` | Fetcher then a pipe into a shell | **Block** | Error |
| `eval-dynamic` | `eval` with a dynamic argument | **Warn** | Warning |
| `chmod-777` | Numeric `777` / `0777` | **Warn** | Warning |

SURFSH remains warn-only: it already maps Block and Warn to findings.
Runtime command-safety is hard-pinned; the class cannot be disabled.
`load_rules` honours per-id `disabled` / `overrides` when
`CommandSafetyConfig` is supplied. **`anvil gate` does not pass that
config today**, so `.anvilrc` per-rule disable is not a live gate
rollback. Skip one invocation with `--skip-command-safety`. SURFSH
rollback is `# @anvil-ignore SURFSH-002: <reason>` or
`ANVIL_TRACK_SURFACE_SH=0`.

Existing `chmod-recursive-777` and `chmod-777-sensitive` stay as they are.

### 3.1 Pipe-to-shell

Parsed SURFDOCK contract:

- Unwrap wrappers (`sudo`, `env`, `command`, …) on each segment first.
- In a pipe-connected chain (operator `|`, not `||`), some command is `curl`
  or `wget` and a later command is `sh` / `bash` / `ash` / `dash` / `zsh`
  (basename; `/bin/sh` counts).
- Does **not** fire on `curl … || sh`, `curl … | tar`, or `cat file | sh`.
- Also fires on download-exec equivalents: `eval "$(curl …)"`,
  `bash <(curl …)`, `bash -c "$(wget …)"`.
- Destination `sh -c` / `bash -lc` is identified from the stage head
  (not after peeling the shell). `|&` and `2>&1 |` count as pipes.
- Fail-closed when one side is a known fetcher/shell and the other is
  an unresolved expansion (`$FETCH | sh`, `curl | $SHELL`). Two unknown
  expansions (`$A | $B`) do not fire.

### 3.4 Known limitations

- `$FETCH | $SHELL` (both sides unknown) is not flagged.
- `timeout`/`busybox`/`exec` prefixes on a fetcher or shell are handled;
  arbitrary unlisted wrappers are not.
- Pretty-printed pipelines without `\` are joined when a line ends with
  `|` or the next line starts with `|`.

### 3.2 Dynamic eval

Fire when any `eval` argument contains `$`, backticks, or command
substitution. Do **not** fire on a static literal (`eval 'echo ok'`,
`eval echo hello`).

This is the static approximation of “eval on user-controlled input”.
Anvil dogfood hits (`scripts/agent/guidance.sh`,
`scripts/cache/anvil-target-evict.test.sh`) are true positives: suppress
with `# @anvil-ignore SURFSH-002: <reason>`.

### 3.3 chmod 777

Fire on numeric mode `777` or `0777`. Do **not** add symbolic forms (`a+w`,
`ugo+rwx`) or other world-writable modes.

## 4. Acceptance

- Unit tests per family: positives, named negatives, wrapper unwrap for
  pipe-to-shell.
- SURFSH scanner picks the new rules up via the shared helper (no SURFSH-only
  duplicate matcher).
- Runtime `run_command_safety_check` Blocks pipe-to-shell and Warns on the
  other two.
- FP re-check on Anvil + ripgrep: target still < 1% FP. Suppressed Anvil
  evals are TP, not FP. Heredoc fixtures in `docs-check.test.sh` stay
  unscanned.

## 5. Non-goals

- Unquoted variables in destructive contexts (SURFSH-009).
- Parser quote tracking.
- Zsh/Fish-only patterns; Makefile / Justfile recipes.
- Migrating the Dockerfile matcher.
- Flagging `cat file | sh` or symbolic chmod.
