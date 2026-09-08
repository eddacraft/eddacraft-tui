---
name: dev-loop-router
description: >-
  Tiny byte-identical router for the canonical agentic loop. Resolve the active
  harness from shell-verified process identity, then load exactly one uniquely
  named native adapter. Contains no harness choreography itself.
---

# Development-loop router

This skill exists only to preserve safe cross-root loading. Its body may be
projected byte-identically into every harness skill root.

## Route

Use the session's **current native shell** to read only these process-environment
signals. Do not read secret-bearing configuration and do not infer identity from
file layout or model self-report.

POSIX shell:

```sh
printf 'CLAUDECODE=%s\nCODEX_THREAD_ID=%s\nOPENCODE=%s\nGROK_AGENT=%s\n' \
  "${CLAUDECODE:-}" "${CODEX_THREAD_ID:-}" "${OPENCODE:-}" "${GROK_AGENT:-}"
```

PowerShell:

```powershell
"CLAUDECODE=$env:CLAUDECODE"
"CODEX_THREAD_ID=$env:CODEX_THREAD_ID"
"OPENCODE=$env:OPENCODE"
"GROK_AGENT=$env:GROK_AGENT"
```

Exactly one positive signal is required:

| Signal | Adapter |
| --- | --- |
| `CLAUDECODE` | `dev-loop-adapter-claude` |
| `CODEX_THREAD_ID` | `dev-loop-adapter-codex` |
| `OPENCODE` | `dev-loop-adapter-opencode` |
| `GROK_AGENT` | `dev-loop-adapter-grok` |

Anything other than exactly one signal is a **fail-closed** identity error. Do
not guess and do not fall back to a default harness. Report the matching
diagnostic and stop:

| Condition | Diagnostic |
| --- | --- |
| No signal | `dev-loop-router: no harness identity signal detected` |
| More than one signal | `dev-loop-router: ambiguous identity` |
| Shell unavailable or denied | `dev-loop-router: cannot-run` |

Name the variables checked in the report. A signal that contradicts the others is
treated as **hostile, never as a tiebreaker**: tracked files such as `.envrc` or
`devcontainer.json` can set these variables too. Nested sessions that inherit
multiple signals stay ambiguous until the operator relaunches with the unintended
parent signal removed (`env -u <PARENT_SIGNAL> …`).

Where repository policy defines `harnessIdentityCheck`, `off` skips the probe
**only** alongside an explicit `pinnedAdapter`. Without a pin the router still
fails closed.

This section is the whole contract. Everything the router needs is inlined
above, because a distribution may ship it without the catalogue's reference
files. Where `dev-loop/references/harness-identity-check.md` is present it adds
the probe evidence and the measured duplicate-name hazard.
Its absence changes nothing about the rules above.

A downstream distribution may replace the probe mechanism with a reviewed native
identity command (for example Delivery Shadow's `shadow harness identity`) while
preserving the same exactly-one/fail-closed contract.

Load `agentic-loop`, then exactly the selected adapter. Adapter selection grants
no scope, write, verification, or merge authority.

## Loader invariant

The deprecated `dev-loop-executor` shim is held to the same identical-bytes rule
until it is retired under NEUT-010.

Never create differently authored copies of `dev-loop-router` under the same
name across roots. Identical projected bytes are the sanctioned exception to the
measured multi-root duplicate-name hazard. Harness-specific behaviour belongs in
the uniquely named adapter skills, where multi-root discovery is harmless.
