# Harness identity check

The catalogue ships one identity router, `dev-loop-router`, projected
byte-identically into every harness skill root an emitter populates
(`.claude/skills/`, `.agents/skills/`, `.opencode/skills/`, `.grok/skills/`).
The router therefore cannot learn which harness is running from where it was
loaded: OpenCode and Grok Build read other harnesses' roots as compatibility
input (see `docs/agent-runtimes.md`), and the same bytes are in all of them by
design. Identity is resolved at runtime from shell-verified environment
signals instead, and the router then loads exactly one uniquely named
`dev-loop-adapter-*` skill. This file holds the mechanics and the evidence; the
router inlines the same check so it stays self-contained when projected alone.

Read `harnessIdentityCheck` from repository policy
(`references/policy-contract.md`); default `auto`.

- `off` — skip the probe. Valid only alongside an explicit `pinnedAdapter`
  naming the adapter to load. Without a pin the router still fails closed.
- `auto` (default) or `strict` — run the shell-verified check below before
  any routed behaviour. The values are equivalent; `strict` is kept so existing
  policy files stay valid.

## Shell-verified identity check

Using the session's shell tool, run exactly:

```sh
sh -c 'printf "CLAUDECODE=%s\nCODEX_THREAD_ID=%s\nOPENCODE=%s\nGROK_AGENT=%s\n" "${CLAUDECODE:-}" "${CODEX_THREAD_ID:-}" "${OPENCODE:-}" "${GROK_AGENT:-}"'
```

The command output in the transcript is the identity evidence. Identity comes
from shell-verified environment signals only — never substitute memory, tool
inventory, context markers, or repository content for this evidence: tracked
files (`.envrc`, `devcontainer.json`, task-runner configuration) can set these
variables too, so a signal that contradicts the others is treated as hostile,
never as a tiebreaker.

If no shell tool is available, or the command is denied, STOP and report:
`dev-loop-router: cannot-run — identity is unverifiable because the shell tool
is unavailable or denied`. This is a restricted configuration, not an attack,
and it is a different diagnostic from ambiguity.

Count the signals whose values are non-empty:

| Signal            | Harness     |
| ----------------- | ----------- |
| `CLAUDECODE`      | Claude Code |
| `CODEX_THREAD_ID` | Codex       |
| `OPENCODE`        | OpenCode    |
| `GROK_AGENT`      | Grok        |

- **Exactly one signal** — load that harness's `dev-loop-adapter-*` skill.
- **No signal** — STOP and report `dev-loop-router: no harness identity
signal detected`, naming the four variables checked. Do not fall back to a
  default harness.
- **More than one signal** — STOP and report `dev-loop-router: ambiguous
identity`, naming every signal found. This is the common nested-session case:
  a harness launched from inside another harness's session inherits the
  parent's variables. No proven disambiguation rule exists — ask the operator
  to state the intended harness, or relaunch with the inherited parent signal
  removed (`env -u <PARENT_SIGNAL> …`).

Misidentification must degrade safely: take no destructive action while
identity is unresolved or mismatched, and if the resolved route's mechanisms
are absent from the actual runtime, stop and re-run the check rather than
falling back destructively.

## Projection invariant

The runtime check is only sound because every projected copy of the router
is the same bytes. Mechanical evidence (below, F1) shows that when a
multi-root reader has to dedup two _differently authored_ skills sharing one
name, which body actually executes is not reliably deterministic. Identical
bytes make that choice irrelevant; a drifted copy, or a retired per-harness
sibling (`dev-loop-claude`, `dev-loop-codex`, `dev-loop-grok`,
`dev-loop-opencode`) coexisting with the router, reintroduces the hazard.

Harness-specific choreography is deliberately kept out of the projected router
and placed in the uniquely named adapter skills, where multi-root discovery is
harmless because no two roots author the same name differently.

`scripts/check-executor-projection.sh` verifies both rules over a checkout or
any install tree and runs in CI, for the router and for the deprecated
`dev-loop-executor` shim that older emissions still load. Never edit a projected
copy; edit `skills/eddacraft/dev-loop-router/SKILL.md` and re-project.

## Evidence

The four signals and the nested-session hazard above are mechanically
confirmed, not just reasoned from documentation: a sister project's
instrumented probe (`delivery-shadow`, CAP-006, 2026-07-23) captured `env`
output directly from real sessions rather than relying on model self-report.
Versions probed: Claude Code 2.1.218, Codex codex-cli 0.145.0, OpenCode
1.17.18, Grok 0.2.106 (bde89716f6) — treat as the staleness reference and
re-check after a harness upgrade. Full findings, including the duplicate-name
nondeterminism finding cited above (F1): `docs/agent-runtimes.md`, "Cross-harness
discovery hazards" section. The collapse to one runtime-routed binding is
recorded in ADR-0023; ADR-0025 moved the surviving invariant to the small
`dev-loop-router` and retired the executor as architecture.
