# ADR-056: `--format` value-enum as the canonical output selector

## Status

Accepted (2026-05-29). Amended the same day during `SARIFOUT-001`
implementation — see [Amendment](#amendment-per-command-not-global-2026-05-29).
The three SARIFOUT design decisions (flag surface, module home, shared model)
were ratified by the operator on 2026-05-29.

## Date

2026-05-29

## Context

Anvil's CLI selects its output rendering through
`OutputMode::{Tui, Plain, Json}`, resolved by
`OutputMode::resolve(json, no_tui, is_tty)` in
`crates/anvil-cli/src/output/mod.rs`. That resolver is a two-boolean plus
TTY-detection truth table: `--json` wins, else `--no-tui` or a non-TTY stdout
selects `Plain`, else `Tui`. `--json`, `--no-tui`, and `--verbose` are the
global output flags today; there is no `--format` flag.

CIB-014 (now superseded by the `SARIFOUT` module) adds SARIF 2.1.0 output to the
finding-emitting commands (`anvil check`, `gate`, `audit`) as a pure additive
machine format. SARIF does not fit the existing model cleanly:

- It is **not** TTY-driven and **not** a degrade target. Unlike `Plain` (the
  non-interactive fallback for `Tui`), SARIF is an explicit, opt-in machine
  format that must **never** be auto-selected by TTY detection.
- Bolting a third boolean (`--sarif`) onto the existing resolver produces an
  ambiguous precedence matrix — what does `--json --sarif` mean? — and does not
  scale to any future machine format (e.g. JUnit).

How a CLI selects machine-output formats is a public, user-facing convention
that is hard and expensive to reverse once a flag ships and scripts/CI depend on
it. Per `docs/guides/adr-process.md` (establishing a convention; hard to
reverse), this decision warrants an ADR before the flag surface is implemented
in `SARIFOUT-001`. The operator ratified this direction on 2026-05-29.

## Decision

Introduce a `--format <FORMAT>` value-enum as the canonical output selector on
the finding-emitting commands. Value space: `auto | tui | plain | json | sarif`.
Default: `auto`. (Originally specified as a *global* flag — narrowed to
per-command during implementation; see
[Amendment](#amendment-per-command-not-global-2026-05-29).)

1. `OutputMode` gains a `Sarif` variant. A single resolver replaces the
   boolean truth table. **Precedence:** an explicit `--format` wins; else legacy
   `--json` resolves to `Json`; else `--no-tui` or a non-TTY stdout resolves to
   `Plain`; else `Tui`. `--format auto` (the default) resolves through the
   existing TTY rules and **never** yields `Sarif`.
2. `--json` and `--no-tui` are retained as **documented compatibility aliases**
   — no deprecation, no behaviour change. `--json` means exactly
   `--format json`. The existing `output/mod.rs` resolver tests stay green and
   gain a `--json` / `--format json` parity test plus a `--format` + `--json`
   precedence test.
3. `--format sarif` is valid **only** on the finding-emitting commands
   (`check`, `gate`, `audit`). On any other command, `clap` rejects it with a
   value-parse error rather than silently degrading.
4. **Convention:** future machine-output formats are added as new `--format`
   enum values, not as new top-level booleans.

## Amendment — per-command, not global (2026-05-29)

Implementing `SARIFOUT-001` surfaced a blocker the design pass missed: `--format`
is **already a per-command flag** with unrelated semantics on two existing
commands —

- `anvil export --format <llms.txt|mcp-resource|prompt-fragment>` (constraint
  export target), and
- `anvil validate --format <aps|json|yaml>` (input plan format override).

`clap` rejects a `global = true` argument whose long name collides with a
subcommand-local one (it panics while building the command tree). A global
output `--format` is therefore not implementable without renaming those two
public flags — a breaking change well outside SARIFOUT scope.

**Resolution (operator-ratified 2026-05-29):** add the output `--format`
value-enum as a **per-command flag on the three finding-emitting commands only**
(`check`, `gate`, `audit`), which have no pre-existing `--format`. `--json` /
`--no-tui` stay global and unchanged. Everything else in this ADR — the value
space, the precedence rules, `--json` as an alias, SARIF being opt-in and never
TTY-auto-selected, and the "future formats are enum values" convention — is
unchanged.

Consequences of the narrowing:

- The "reject `--format sarif` on a non-finding command" guarantee now falls out
  of `clap` for free: `--format` simply does not exist on non-finding commands,
  so any `--format …` there is an `unexpected argument` error. No central
  allowlist or pre-dispatch validation is needed (this also removes the
  allowlist-drift risk noted under Risks).
- `--format json|plain|tui` is **not** accepted on non-finding commands; their
  output selector remains `--json` / `--no-tui` (the global aliases). No
  capability is lost — `sarif` is only meaningful where findings are emitted.
- The resolver lives in `output/mod.rs` (`resolve_format` +
  `from_command_format`); `from_global` is retained unchanged for the
  non-finding commands and can never yield `Sarif`.

Future option (not taken now): if a truly universal selector is ever wanted,
a non-colliding `--output-format` global flag could be introduced; deferred
until there is demand.

Known limitation (deferred): `--format tui` forces the TUI even when stdout is
not a terminal, so piping a `--format tui` invocation surfaces a raw crossterm
"enable raw mode" error rather than a friendly message. This is pre-existing
`TerminalGuard` behaviour, now reachable via an explicit flag; forcing the TUI
to a pipe is a misuse and degrading `tui` → `plain` would make `tui` equivalent
to `auto`, so it is left as-is and tracked as a follow-up nicety.

## Rationale

A single explicit `--format` selector with documented precedence is
unambiguous, scales to future formats, keeps SARIF opt-in (it can never be
chosen by TTY detection), and preserves full backward compatibility by folding
the existing booleans in as aliases. The cost is transitional duplication (two
spellings — `--json` and `--format json`) and a one-time, mechanical change to
the resolver signature and its callers.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **`--format` value-enum extends `OutputMode` (chosen)** | One canonical selector; scales to future formats; explicit precedence; SARIF can never be TTY-auto-selected; `--json`/`--no-tui` keep working as aliases | Two spellings for JSON during/after transition; resolver signature change touches every `from_global` caller (mechanical) |
| **A. Add a `--sarif` boolean + `Sarif` variant, keep the boolean resolver** | Smallest diff | Ambiguous precedence (`--json --sarif`?); does not scale to a 3rd/4th format; high risk of a TTY auto-select bug for a format that must stay opt-in |
| **B. Separate `--output sarif` flag, distinct from `--json`/`--no-tui`** | Isolates SARIF from the existing flags | Two parallel output-selection mechanisms on the same command; more confusing than one `--format`; still needs precedence rules against `--json` |
| **C. Replace `--json`/`--no-tui` with `--format` (hard deprecation)** | Single mechanism, no alias duplication | Breaking change for every script/CI invocation using `--json`; unnecessary churn — rejected in favour of keeping the booleans as aliases |

## Consequences

- **Positive:** One canonical, discoverable output selector. Future machine
  formats are additive enum values with no new precedence surface. `--json`
  continues to work unchanged. SARIF is opt-in only and structurally cannot be
  auto-selected.
- **Negative:** Transitional duplication (`--json` ≡ `--format json`). The
  resolver signature changes, touching every `OutputMode::from_global` call
  site (mechanical, compiler-guided).
- **Risks:** Precedence bugs when both `--format` and a legacy boolean are
  passed; the "which commands accept `--format sarif`" allowlist can drift out
  of sync as new finding-emitting commands are added.
- **Mitigations:** Explicit precedence encoded in one resolver with parity and
  conflict unit tests; a single centralised allowlist of finding-emitting
  commands plus a reject test proving `--format sarif` errors on a non-finding
  command.

## References

- Design: [`../specs/2026-05-29-sarif-output-design.md`](../specs/2026-05-29-sarif-output-design.md) (Decision 1 — Flag Surface)
- APS: `SARIFOUT-001` (flag surface + `OutputMode::Sarif` + resolver), `SARIFOUT` module
- Origin: `CIB-014` (superseded by the `SARIFOUT` module)
- Related ADRs: ADR-050 (Anvil keeps its own `clap` tree), ADR-054 (TUI engine home)
- Code: `crates/anvil-cli/src/output/mod.rs` (`OutputMode`, `resolve`, `from_global`)
