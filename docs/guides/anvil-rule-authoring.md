# Authoring `.anvil` Rules

| Type  | Authority     | Owner | Status | Freshness                                                                                             |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | SCAN  | Live   | Last reviewed 2026-08-23 against `crates/anvil-checks/src/antipattern/registry_loader.rs` and ADR-131 |

| Upstream                                                                               | Downstream                                  |
| -------------------------------------------------------------------------------------- | ------------------------------------------- |
| `patterns/`, `patterns/compiled/registry.json`, `crates/anvil-checks/src/antipattern/` | Rule authors, release reviewers, DOCGOV-006 |

This guide walks through adding a new anti-pattern to Anvil's detection
catalogue. All rules live as `.anvil` files under `patterns/` and are compiled
into `patterns/compiled/registry.json` by `scripts/compile-patterns`.

`registry.json` is the **contract the Rust scanner consumes**. The scanner in
`crates/anvil-checks` is authoritative per [ADR-026]; the TypeScript scanner was
archived under [ADR-033]. The active TypeScript in
`packages/anvil/core/src/anvil-format/` is only the `.anvil` source compiler,
not a scanner runtime. Authoring a rule means editing the `.anvil` source and
recompiling the registry — the scanner does not carry a separate hand-written
pattern table.

[ADR-026]: ../../plans/decisions/026-rust-scanner-authoritative.md
[ADR-033]: ../../plans/decisions/033-park-ide-mcp-retire-ts-scanner.md

## Layout

```
patterns/
  <family>/
    definition.anvil      # family-level narrative (why these rules group)
    <RULE-ID>.anvil       # individual rule
```

Existing families:

- `guardrail-suppression` — disabling tools that were there to help
- `type-system-evasion` — escape hatches around the type system
- `error-visibility` — hiding failures that should surface
- `responsibility-laundering` — shifting blame or deferring review
- `deferred-debt` — recording work the author won't do now

Pick the family that best captures the _meta-issue_ your rule exemplifies. If
none fit, a new family is allowed — propose it in an ADR first.

## Rule ID conventions

| Prefix     | Meaning                                                 |
| ---------- | ------------------------------------------------------- |
| `AP-`      | Legacy numbering, still used where natural              |
| `GS-`      | Guardrail-suppression family (new rules)                |
| `RL-`      | Responsibility-laundering                               |
| `DD-`      | Deferred-debt                                           |
| `TE-`      | Type-system-evasion (reserved)                          |
| `EV-`      | Error-visibility (reserved)                             |
| `AI-`      | AI reasoning category (e.g. AI-001 appeal-to-authority) |
| `SURFENV-` | Surface scanning for `.env`/`.envrc` key-value parsing  |

Pick the next free three-digit suffix within the prefix (`GS-002`, `RL-007`,
etc.). Rule IDs are globally unique across the registry.

## Rule file shape

```yaml
---
id: GS-002
type: rule
family: guardrail-suppression
title: Short descriptive title
version: 1

severity: warning          # error | warning | info
confidence: medium         # high | medium | low
spectrum_position: 3       # relative severity within the family (1 = most severe)

targets: [source]          # source | pr-description | commit-message | agent-output
file_extensions: [.ts, .tsx]
allowlist: ['**/*.test.ts', '**/__tests__/**']

detection:
  type: regex              # or ast
  pattern: 'your regex'
  flags: i                 # optional; `g` is always added

related: [AP-004]
enabled: true
opt_in: false
---

Narrative markdown body. Keep it short — everything above the first H2
heading becomes the `nudge`:

1. **What to do instead** — concrete, short, action-focused.
2. **Why it matters** — the reasoning, for humans and AI reviewers who
   navigate from the warning to the family definition.

Optionally, a rule-level explanation that replaces the family's:

## Why It's Harmful

Why this specific rule matters, when the family definition's explanation
is too broad to be useful here.
```

### Fields

- `id` — globally unique rule ID (see prefix table above).
- `family` — must match an existing directory name under `patterns/`.
- `severity` / `confidence` — consumed by the scanner and gate runner.
- `spectrum_position` — integer ordering within the family, where `1` is the
  most severe violation and higher numbers are milder variants. Matches the
  schema contract in `packages/anvil/core/src/anvil-format/schemas.ts`
  (`1 = most severe`). Used by reviewers to reason about escalation.
- `targets` — which artifact kinds the rule scans. Most source-code rules are
  `[source]`; PR-description rules like the RL family use `[pr-description]` or
  combinations.
- `file_extensions` — scanner only invokes the rule for matching files. Omit for
  artifact targets like `pr-description`.
- `allowlist` — glob patterns where the rule is silent (e.g., test files for
  rules that are noisy in tests).
- `detection` — either a regex (`type: regex`, `pattern`, optional `flags`) or
  an AST query (`type: ast`, `ast_query`). Regex is far more common.
- `related` — other rule IDs worth reading alongside; shown to humans.
- `enabled` — `true` ships the rule; `false` keeps it in the registry but
  doesn't run it. Use this to stage rollout.
- `opt_in` — `true` requires `--include-opt-in`. Use for rules that are noisy in
  many codebases but valuable in specific ones.

### Body

The markdown body is the _rich definition_ — what a reviewer should read when
they want to understand the rule beyond the one-line title. Keep it
action-oriented: "Use X instead of Y", not "this is bad".

The compiler splits the body on H2 headings. An H2 is a line starting with two
hashes followed by a space or tab (`extractSections` matches `/^##[ \t]/`), so
`##Heading` with no space is body text, not a heading — and a heading indented
by even one space is not recognised either:

- **Everything before the first H2 becomes the `nudge`** — the inline text shown
  on a finding. It is the whole preamble, not just the first paragraph; several
  short paragraphs are fine.
- **`## Why It's Harmful` overrides the family-level `explanation`** for this
  rule only. Use it when the family definition's explanation does not fit — a
  mixed-topic family such as `python-reliability` covers suppression comments,
  bare `except:`, wildcard imports and `eval()`, so its family explanation is
  boilerplate for any one of them. Without the override, a rule inherits the
  family text verbatim.
- **Any other H2 is silently discarded.** It is parsed, then read by nothing: it
  does not reach `nudge`, `explanation`, or any other compiled field, and the
  compiler emits no error. Do not put content you need under `## Examples`,
  `## References`, or similar in a _rule_ body — only family `definition.anvil`
  files have a required-section contract.
- **Do not start a rule body with an H2.** With no preamble the `nudge` falls
  back to the entire raw body, so literal `##` markdown leaks into the inline
  text a developer sees on a finding. Always write the nudge paragraphs first.

The last two behaviours are characterised by tests in
`packages/anvil/core/src/anvil-format/compile.test.ts` and are pinned as
current-behaviour, not desired-behaviour — CIB-334 covers making the compiler
reject an unrecognised or leading rule-body H2 instead of dropping content
silently.

### Example: AI-001 reasoning rule

The 0.5.0-beta cycle introduced the `AI-` prefix for the AI reasoning category.
`AI-001` flags source comments that justify code with authority, social proof,
or deflection ("ChatGPT said this was fine", "Claude wrote this") rather than
technical reasoning. It is registry-authored just like every other rule:

```yaml
---
id: AI-001
type: rule
family: reasoning
title: Appeal-to-authority justification in code comments
version: 1

severity: info # info — annotation, not a failure
confidence: medium
spectrum_position: 3

targets: [source]
file_extensions: [.ts, .tsx, .js, .jsx, .rs]

detection:
  type: regex
  pattern: '(ChatGPT|Claude|Copilot|Gemini)\s+(said|told|wrote|generated)'
  flags: i

enabled: true
opt_in: false
---
```

Two contracts matter for AI-001 specifically:

- **Comment-region only.** The Rust scanner restricts AI-001 matching to comment
  regions (line and block comments). It does not flag matches inside string
  literals or normal code.
- **`@anvil-ignore AI-001` semantics.** Per-occurrence suppression uses the
  standard `// @anvil-ignore AI-001 -- <reason>` marker. The reason is required
  — bare `// @anvil-ignore AI-001` is itself a finding, the same way it is for
  every other rule. AI-001 emits at info severity, so it does not fail gates by
  default; it is annotation.

When you author a new reasoning-category rule, follow the same shape: scope to
comment regions, keep severity at info or warning, and rely on the
registry-authored detection rather than scanner-side special-casing.

## Compile and verify

```bash
# Compile .anvil sources into registry.json
pnpm --filter @eddacraft/anvil-core patterns:compile

# Run the core test suite (expects the new rule to appear)
pnpm --filter @eddacraft/anvil-core test

# Optional: typecheck
pnpm -r typecheck
```

The compiler validates:

- Each rule references an existing family definition.
- No two rules share an `id`.
- `targets` / `severity` / `confidence` / `detection.type` are valid enum
  values.
- Regex patterns parse.

A failing compile step points at the offending file and field; fix and rerun.

### Engine compatibility

The Rust scanner uses the `regex` crate (RE2-style: no backtracking, no
lookaround). Patterns that rely on lookaround are accepted by the `.anvil`
compiler but may not compile under `regex`. The Rust scanner handles known
legacy cases in two steps:

1. `flags: "i"` on the registry entry is honoured by the Rust loader via an
   inline `(?i)` prefix, so `RL-*` and `DD-004` match case-insensitively in both
   scanner (SPG-001).
2. Lookaround-bearing rules (DD-001, DD-002, DD-003, GS-001, RL-001, RL-005) are
   translated into a Rust-side post-filter paired with a base regex in
   `crates/anvil-checks/src/antipattern/scanner.rs::prepare_pcre_rewrite`
   (SPG-003).

Any other rule whose pattern fails to compile in the Rust scanner is surfaced by
`anvil doctor` as a `registry-patterns-compile` warning (SPG-002) — the
silent-drop path is observable. The old TS/Rust parity harness was archived with
the TypeScript scanner. When possible, rewrite to avoid lookaround; when not
possible, add a new arm to `prepare_pcre_rewrite` and Rust scanner fixtures for
the positive and negative cases.

### Registry integrity

The compiled registry (`patterns/compiled/registry.json`) is a trust boundary.
The Rust loader resolves it in this order (see
`crates/anvil-checks/src/antipattern/registry_loader.rs::resolve_registry_path`,
ADR-131):

1. `LoadRegistryOptions.registry_path` (explicit override, tests and API).
2. The `ANVIL_REGISTRY_PATH` environment variable.
3. The compile-time embedded catalogue.

There is no cwd or executable-directory walk. A cloned
`patterns/compiled/registry.json` does not replace the scanner catalogue. Rule
authors who need an unrebuild catalogue must set `ANVIL_REGISTRY_PATH` to the
compiled file, then rebuild (or keep the env var) so the binary matches the
source.

**`ANVIL_REGISTRY_PATH` is a trust boundary.** If it is set, the scanner will
load whatever JSON lives at that path with no integrity check on the payload. A
poisoned registry — for example one with every rule's `enabled` flipped to
`false`, or with detection patterns rewritten to match nothing — silently
disables scanning without any other signal that something is wrong.

Treat the env var accordingly:

- **CI jobs should rely on the in-tree registry compiled at the checked-out
  SHA.** Do not set `ANVIL_REGISTRY_PATH` to a path outside the repo.
- **Never let external input (a PR body, a webhook payload, an untrusted build
  argument) flow into `ANVIL_REGISTRY_PATH`.** It is intended for local
  development overrides and test fixtures only.
- **Scanner self-check.** Running `anvil doctor` prints the
  `registry-patterns-compile` check; if a poisoned registry has also broken the
  regex shape, that check will flag the affected rules. It does not yet verify
  the registry's byte hash — a `--expect-registry-hash` CLI flag is a possible
  follow-up (ADR material, not part of this module). Compile checks detect
  _shape_ failures only. A registry that is syntactically valid but semantically
  poisoned — every rule's `enabled` flipped to `false`, every pattern rewritten
  to `.*` so nothing meaningful surfaces — will not be caught by this check. Pin
  the registry's hash in CI if that matters.
- **Symlinks.** `resolve_registry_path` does not canonicalise the env-var value.
  If `ANVIL_REGISTRY_PATH` points into a writable directory whose contents can
  be swapped by another process (e.g. a shared `/tmp`), that same write surface
  owns the scanner catalogue for the process's lifetime. Use absolute paths
  inside the repo or a trusted config root.

### ReDoS and Untrusted Artifact Targets

Rules targeting `pr-description`, `commit-message`, or `agent-output` scan text
that may be supplied by untrusted contributors or automation. Treat those inputs
like untrusted user input even though the Rust scanner uses the non-backtracking
`regex` crate today.

- Keep patterns linear and bounded; avoid broad nested alternations that make
  reviews and future engine ports hard to reason about.
- Prefer explicit phrase families over catch-all `.*` bridges across large text
  blocks.
- Add positive and negative fixtures for PR-body and commit-message examples
  when a rule targets those artifact kinds.
- Keep the `max_line_bytes` style guards in scanner code when adding new
  artifact readers; never let a single PR body or commit message become an
  unbounded scan unit.

## Testing the rule

**Enumerate the threat model before you write the regex.** List the defect or
attack shapes the rule exists to catch — the things that must fire — and derive
every positive fixture from that list, never from the branches of the pattern
you happen to have written. A suite derived from the pattern is a mirror of the
implementation: each fixture exercises a branch the regex already takes, so the
suite structurally cannot fail when the regex narrows. A suite derived from the
threat model is a specification: when the pattern stops covering a shape, the
shape's fixture goes red.

For PY-008 (dynamic `eval`/`exec`/`compile` arguments) the threat model is:
plain identifier, attribute access, indexing, composed payloads via every
operator family _including keyword operators reached across a space_ (`if`,
`and`, `or`, `not`, `is`, `in`, `await`), kwargs, walrus, unpacking, every
interpolating f-string prefix ordering (`f`, `rf`, `fR`, ...), non-ASCII
identifiers, and wrapped calls whose first argument ends the line. Note that
none of those entries mentions a regex construct — that is the test: if a list
item only makes sense with the pattern open in the next pane, it is describing
the implementation, not the threat.

Write the enumeration down as labelled data. The shared helper
`assert_rule_fires_on(path, rule, &[(shape_label, fixture)])` in
`crates/anvil-checks/tests/support/mod.rs` takes exactly this list, so the
threat model is reviewable in the diff — a reviewer compares the labels against
the rule body's stated intent, a shape without a fixture is impossible by
construction, and a narrowing reports every missed shape in one run instead of
dying on the first assert. `python_antipatterns.rs`'s PY-008 tests are the
reference consumers.

**Prove every new assertion RED.** An assertion that has never failed proves
nothing — it may be green because the rule works, or because the fixture cannot
reach the rule at all. For each new assertion, apply the mutation it guards
against and watch it fail. For detection rules the mechanism is a patched
registry copy: copy `patterns/compiled/registry.json` somewhere disposable, edit
the rule's `detection.pattern` to the narrowed form (for a regression guard, the
historical bad pattern from git), point `ANVIL_REGISTRY_PATH` at the copy, and
rerun the suite — the loader honours the override before any upward walk (see
[Registry integrity](#registry-integrity); this is the sanctioned test-fixture
use of that env var). The assertions you just wrote must fail; record which
mutation turned which assertion red in the PR description.

### Worked example: the #3880 regression

PR #3880 narrowed PY-008's dynamic-argument arm to an identifier terminated by
one of `,` `)` `(` `.` `[` — a delimiter allowlist. The 33-test suite stayed
green through the change, and "33 passed" was cited as merge evidence, because
every positive fixture (`eval(user_input)`, `eval(input())`,
`eval(user_input[0])`, ...) terminated its first identifier with a character
from that same delimiter class. The tests had been written from the regex, so
the regex could not narrow out from under them. The rule shipped roughly twenty
false-negative shapes at `severity: error` — every operator-composed payload,
`eval(a + b)` included, which is the canonical injection shape.

One threat-model fixture would have caught it: `eval(a + b)` terminates the
identifier with a space, sits in no delimiter allowlist, and goes red the moment
the terminator class narrows. That fixture now lives in
`py008_composed_and_operator_terminated_arguments_fire` alongside the rest of
the enumerated threat model, and replaying the #3880 pattern via
`ANVIL_REGISTRY_PATH` turns 18 of its 19 shapes red — the survivor is the
comma-terminated keyword-argument shape, the one shape the delimiter allowlist
covered.

## Checklist before merge

- [ ] New `.anvil` file lives in the correct family directory.
- [ ] Rule ID uses the right prefix and is globally unique.
- [ ] `family`, `spectrum_position`, and `related` align with the family
      definition.
- [ ] `file_extensions` and `allowlist` match the scope (e.g., don't run a
      TS-only rule on `.html`).
- [ ] Registry recompiled and committed (`patterns/compiled/registry.json`).
- [ ] New assertion or snapshot in `patterns.test.ts` / `scanner.test.ts` if the
      rule introduces novel surface area.
- [ ] Positive fixtures enumerate the rule's threat model as labelled shapes
      (see [Testing the rule](#testing-the-rule)), not the pattern's branches.
- [ ] Prove-RED run recorded in the PR: which mutation of the pattern turned
      which assertion red.
- [ ] **Do not** hand-edit `docs/public/anvil/reference/rules.md`. It is
      generated (`pnpm docs:public:generate`) from the registry **at the last
      released tag**, not from the workspace tree, so a rule that has not
      shipped yet must not appear there — adding it makes the file stale against
      its own source and fails the `public-docs` surface in CI. New rules land
      in the public reference automatically when a release containing them is
      cut. (`overview.md`, `concepts/gates.md` and `operations/config.md` list
      no rule ids at all; the reference table is the only rule listing.)
- [ ] Trap: in a clone without tags the generator logs
      `public release ref     <tag> does not resolve; using workspace tree for all product inputs`
      and generates from the workspace registry instead — which looks correct
      locally and fails in CI. Run `git fetch --tags` before trusting a local
      `docs:check`.
- [ ] `docs/architecture/checks-as-built.md` — update the family/ID table.
      Unlike the public reference this tracks `main`, so it does carry
      unreleased rules.

## Retiring a rule

Prefer `enabled: false` over deleting the file — keeps the rule ID reserved and
the narrative discoverable. Delete only when the rule is outside Anvil's scope
entirely (e.g., the HTML/CSS patterns retired in ANVFMT-014 per decision D-002).
