# ADR-038: Hook Surface and Noise Discipline

## Status

Proposed

## Date

2026-05-07

## Context

Anvil's L3 (pre-commit) and L4 (pre-push, client-side) layers depend
on git hooks. Hook design has two distinct concerns:

1. **Functional surface:** which hooks does Anvil install? What does
   each one do? How do they integrate with frameworks the user
   already runs (husky / lefthook / pre-commit-framework)?

2. **Behavioural discipline:** how do these hooks behave when things
   go wrong, when validation fails, when Anvil itself is unavailable,
   when the user's pre-existing chain is broken?

User pain that drove this ADR:

> *"What we must absolutely avoid is that in a failure state the
> user's terminal is a wash of anvil error messages as they will just
> turn anvil off (like I did with serena)."*

This is the **Serena rule.** A flaky tool that produces a wall of
errors gets disabled by users; a disabled tool provides zero
protection. So the worst failure mode for Anvil is not "Anvil
crashes" but "Anvil annoys the user enough that they disable it."

This ADR pulls noise discipline up to a first-class governance
principle and pins the hook surface that obeys it.

## Decision

### D-1 — The Serena rule (noise discipline as governance)

Failure must reduce noise, not increase it. Concretely:

- **Silent on success.** A successful hook prints nothing. Not "✓
  validated." Not "42 rules passed." Nothing. Silence is the win
  signal.
- **One terse line on warning.** Format:
  `anvil: <count> warning(s) (commit allowed) — anvil show <id>`.
  The pointer (`anvil show <id>`) lets the user dig in if they want.
- **One terse line on block.** Format:
  `anvil: <count> finding(s) (block) — anvil show <id>`. Hook exits
  non-zero. Commit refused. User fixes, retries, or `--no-verify`.
- **One terse line on internal error.** Format:
  `anvil: <component> errored (anvil doctor for details)`. Hook
  exits zero (commit proceeds; witness records the error; L4 picks
  up). User isn't held hostage to Anvil being healthy.
- **Repeat-suppression.** Same class+detail won't re-emit in the
  same session. Daemon-down message fires once per session, not 82
  times during a sub-agent burst.
- **Detail goes to log files.** `~/.local/state/anvil/intercept.log`
  for operational; `intercept-panic.log` for crash dumps. Never
  stack traces to stderr.
- **No colour escalation by default.** Reserve loud formatting for
  genuine block decisions. Transient infra warnings stay calm.

### D-2 — Output discipline applies to ALL Anvil surfaces

| Surface | Allowed output during normal operation |
|---|---|
| Daemon (when run via `ensure`) | Detached; logs to file; never to user terminal |
| MCP shim (`anvil mcp serve --stdio`) | stdout reserved for protocol frames; stderr only for protocol-shape errors editors surface |
| Hooks (pre-commit / pre-push / others) | Per D-1 noise rules |
| `anvil status` (user-invoked) | Allowed informative; structured human-readable; `--json` available |
| `anvil doctor` (user-invoked) | Allowed detailed; bounded probe times; `--json` available |
| `anvil show <id>` (user-invoked) | Detailed diagnostic; the user explicitly asked |
| Editor driver | Surfaces diagnostics via `textDocument/publishDiagnostics` / `anvil/publishDiagnostics`; not `window/showMessage` |

### D-3 — Hook surface (which hooks Anvil installs)

| Hook | Purpose | v1? | Time budget |
|---|---|---|---|
| `pre-commit` | L3 validation; witness append; chain integrity | **v1** | <500ms p95 |
| `post-commit` | Kindling `action_executed`; daemon chain-head cache update | **v1** | <50ms p95 |
| `pre-push` | L4 client-side validation; chain integrity across pushed commits | **v1** | <2s p95 |
| `post-merge` | Witness chain merge-join recording; Kindling | **v1** | <100ms p95 |
| `post-rewrite` | Regenerate witnesses for amended/rebased commits | **v1** | <500ms × commit-count p95 |
| `prepare-commit-msg` | Inject task_id / agent attribution trailer | **v1.5** | — |
| `commit-msg` | Validate commit message style; check `@anvil-ignore` citations | **v1.5** | — |

Anything else (post-checkout, pre-rebase) is unnecessary — daemon
observes via watcher, no hook needed. Hook minimalism principle:
"Can this be done passively by the daemon? Then don't add a hook
for it."

### D-4 — Framework integration (non-destructive)

`anvil start` / `anvil hook bootstrap` detect the user's existing
hook framework and integrate as one MORE thing in the chain, never
as a replacement:

| Detected | Action |
|---|---|
| `.husky/pre-commit` (Husky) | Append `anvil hook pre-commit "$@"` to existing chain |
| `lefthook.yml` / `.toml` | Add Anvil step to lefthook config |
| `.pre-commit-config.yaml` | Add Anvil entry to repo hooks |
| `.cargo-husky/hooks/` | Append to its hook |
| `.githooks/` with `core.hooksPath` set | Append at end of pre-commit |
| Nothing detected | Install at `.git/hooks/pre-commit` directly |

Anvil's hook line uses no `|| true` — the binary itself decides exit
codes. Adding `|| true` in the user's hook chain would swallow
legitimate block decisions; the binary's own panic catcher converts
crashes into exit-0 + log so a panicked Anvil doesn't break the
user's chain via `set -e`.

### D-5 — Hook is a self-contained binary, not a shell script

Hook scripts are 3 lines:

```sh
#!/bin/sh
command -v anvil >/dev/null 2>&1 || exit 0
exec anvil hook pre-commit "$@"
```

(`command -v` check ensures uninstalled-Anvil is silent; the hook
becomes a no-op if the binary is missing.)

The actual logic lives in the Rust binary. This:

- Eliminates pnpm / Node / husky-runtime dependencies. The binary
  works in any worktree even if `pnpm install` hasn't run.
- Keeps the shell surface to one line per hook (less for shell to
  break).
- Centralises noise discipline in code, not in fragile shell.
- Makes the panic catcher possible (`std::panic::set_hook` converts
  any internal crash to a single stderr line + log file write +
  witness record).

### D-6 — Failure-mode taxonomy (what each hook does on each failure)

| Failure class | Hook output | Witness behaviour | Commit outcome |
|---|---|---|---|
| Validation passed (clean) | *(silent)* | `L3: {status: "ok"}` | Proceed |
| Validation found warn-level | One terse line | `L3: {status: "warn", findings: [...]}` | Proceed |
| Validation found block-level | One terse line; exit 1 | No witness (no commit happens) | Refused |
| Daemon unreachable, embedded fallback ran | One terse line, suppressed if already shown this session | `L3: {status: "ok", backend: "embedded"}` | Proceed |
| Daemon unreachable AND embedded errored | One terse line | `L3: {status: "error", backend: "embedded-failed"}` | Proceed (L4 catches) |
| Hook didn't fire (worktree not bootstrapped) | *(no output — hook didn't run)* | No witness for the commit | L4 detects on push, prompts `anvil hook bootstrap` |
| `--no-verify` used | *(nothing — git's own warning suffices)* | No witness | L4 detects on push, applies `on_no_verify` policy |
| Hash chain break detected | One terse line + pointer | Refuse the commit OR append `chain-broken-recovery` line per policy | Refused by default |
| Witness file write failed (disk full / perms) | One terse line | None | Refused — we don't claim what we can't witness |
| Slow validation timed out | One terse line | `L3: {status: "warn", findings: [...], partial: true, timed_out_rules: [...]}` | Proceed (with partial witness) |

The pattern: **failure that's Anvil's fault doesn't block the user;
failure that's a real violation does.** Anvil being broken should not
cost the user their work.

### D-7 — Anvil binary's panic catcher

The binary's `main()` registers a `std::panic::set_hook` that:

1. Writes the panic info + backtrace to `intercept-panic.log`.
2. Writes a single stderr line: `anvil: L3 hook errored (anvil doctor for details)`.
3. Writes a witness line tagged `L3: {status: "error", reason: "panic", "log_path": "..."}`.
4. Calls `std::process::exit(0)` — proceed; let L4 catch.

This makes "Anvil panicked" indistinguishable from "Anvil's daemon
unreachable" from the user's perspective: one line, suppression on
repeat, commit proceeds, L4 picks up.

## Rationale

### Why noise discipline is governance, not UX

User feedback is direct: tools that flood the terminal get disabled.
A disabled tool has zero protection. So Anvil's primary failure mode
isn't "Anvil missed a violation"; it's "user disabled Anvil because
of repeated annoyances." Avoiding that mode is a security property,
not a usability nicety — disabled-Anvil is the same as
no-Anvil.

This ADR pulls noise discipline up to ADR-level so any future
contribution can be reviewed against it, not negotiated against
in-line.

### Why a self-contained binary, not a shell script

Real-world data: this very repo's husky chain breaks on fresh
worktrees because `.husky/_/` requires `pnpm install`. If Anvil's
hook depended on the husky runtime, fresh worktrees would silently
fail to validate (Anvil never fires), and L4 would catch on push —
but the user might not realise the hook isn't running until then.

A self-contained Rust binary works in any worktree the moment
`anvil` is on PATH. No `.husky/_/` dependency. No pnpm dependency.
No husky-version coupling. The hook script is so simple it can't
break.

### Why "no `|| true`" in the hook chain

It's tempting to wrap Anvil's invocation in `|| true` so a panicked
Anvil doesn't break the user's chain. But `|| true` swallows
legitimate exit-1 (block decisions). The right answer is:

- The binary itself decides exit codes (panic → exit 0; block → exit
  1; pass → exit 0).
- The husky chain trusts those exit codes.
- Panics are caught internally and demoted to "internal error,
  proceed."

This way the user's `set -e` chain breaks ONLY on real block
decisions, and Anvil itself takes care of not breaking under its own
internal failures.

### Why limit the hook surface to 5 v1 hooks (not 12)

Each installed hook is a new code path, a new test surface, a new
opportunity to fail noisily. The selection is justified by:

- "Can this be done passively by the daemon?" — branch-switch,
  checkout, etc., observable via daemon's `.git/HEAD` watcher;
  no hook needed.
- "Does it need to mutate or refuse at a specific moment?" —
  pre-commit (refuse), pre-push (refuse), post-commit / post-merge
  / post-rewrite (record state). Five.

`prepare-commit-msg` and `commit-msg` are v1.5 because they're
optional richness, not load-bearing for protection.

## Consequences

- **Positive — Anvil isn't disabled by users.** Bounded noise budget
  means real violations stand out; transient infra issues are
  whispered, not shouted.
- **Positive — Hook fragility is decoupled from Anvil.** Husky /
  lefthook / pcf / no-framework all work the same way. Anvil's hook
  is the same one-line script regardless.
- **Positive — Worktree bootstrap is robust.** Self-contained binary
  works the moment `anvil` is on PATH; daemon's silent self-heal
  handles the husky `.husky/_/` regeneration when needed.
- **Positive — Panic doesn't break the user.** Internal Anvil
  failures cost nothing; L4 picks up.
- **Positive — The hook is auditable in 3 lines.** Users who care
  can read the hook script and understand it instantly.
- **Negative — Repeat-suppression state is per-session.** A user who
  hits the same daemon-down condition across 100 sessions sees the
  message 100 times. Mitigation: doctor command can show "last 30
  days of repeated errors" for users who want the aggregate view.
- **Negative — `validation.backend` field on every L3 witness.**
  Mandatory metadata expense: ~40 bytes per witness. Acceptable.
- **Negative — Some users will WANT loud success output.** "Show me
  every check that passed" is a valid debugging mode. Solution:
  `ANVIL_VERBOSE=1` env var or `--verbose` flag for explicit opt-in.
- **Risk — Repeat-suppression false-negative.** If a user
  legitimately has a recurring issue, suppression hides it.
  Mitigation: doctor command surfaces suppressed events; reset
  via `anvil doctor --reset-suppressions`.
- **Risk — A future hook contribution forgets noise discipline.**
  Mitigation: this ADR is the test. Any new hook contribution
  reviews against §D-1 / D-6.

## References

- **Spec:** [`2026-05-07-anvil-multilayer-protection-architecture.md`](../specs/2026-05-07-anvil-multilayer-protection-architecture.md) §6
- **Brainstorm:** [`2026-05-07-anvil-multilayer-protection-brainstorm.md`](../brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md) §1.6 (Serena rule), §2 principle 1
- **Companion ADRs:**
  - ADR-036 — Daemon scope (parent: where the daemon's I/O surface lives)
  - ADR-037 — Witness chain (companion: hooks write the witness)
  - ADR-039 — Baseline policy (companion: baseline runs at hook-install time)
- **APS modules:**
  - `plans/modules/multilayer-protection.aps.md` — MLP-003 (pre-commit hook), MLP-004 (pre-push hook), MLP-005 (post-* hook handlers)
- **Related ADRs:**
  - ADR-001 — Planless-first (hooks installed by `anvil start`; user doesn't author them)
  - ADR-002 — Warnings over blocks (default; aligns with hook noise discipline)
  - ADR-031 — Validation latency rubric (owns hook time budgets)
- **External patterns:**
  - Husky 9 hook chain shape (`core.hooksPath = .husky/_`)
  - Lefthook YAML
  - pre-commit-framework `.pre-commit-config.yaml`
