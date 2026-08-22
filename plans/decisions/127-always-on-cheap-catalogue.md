# ADR-127: Always-on cheap catalogue, rare full gate

## Status

Accepted 2026-08-22 (operator, in-session)

## Date

2026-08-22

## Context

ADR-071 put AST anti-pattern detection in `anvil-checks-ast`, a terminal
crate the resident daemon must not link (ADR-064). CLI `anvil check` and
`anvil gate` merge regex and AST. Save-time, MidEdit, and MCP
`anvil_validate_write` stay regex-only so they keep the ADR-031 interactive
budget (80 ms service / 120 ms round-trip p95).

Three coverage holes followed:

1. **MCP `anvil_check` and planless `anvil_gate` skip AST.** They call
   `run_antipattern_check` only. ADR-071 §7 already required both scanners on
   any check/gate invocation. Agents that “do the right thing” never see
   `RS-*`.
2. **Golden-path save-time never runs AST.** After `anvil start` the daemon
   is armed; watch routes `check` through the daemon. AST is dark until a
   human runs `anvil check`/`gate` or a hook fires.
3. **Python AST is unimplemented.** `anvil-checks-ast` is hardcoded to
   `rust_language()`. PY-004/008/009 document regex blinds (block-body
   `except …: pass`, composed/multiline `eval`, `yaml.load` loader
   arguments). CIB-332 named AST as the syntactic fix and recorded it as
   not near-term because the crate is Rust-only.

A fourth hole is the full merge gate: `anvil gate` is opt-in (typed, hook
install, or CI). The shipped adopter workflow runs L4 pre-push, not
`anvil gate --profile ci`. Auto-running the **full** gate on save was tried
as cold `anvil check --all` and cost ~6.55 cores per agent (ADR-061 / RLB-007).

Operator direction 2026-08-22: do not auto-run full `anvil gate` on save; do
auto-run the cheap catalogue (regex + AST) on surfaces that already fire;
keep the named gate as the merge judgement at commit/CI; add bounded
**additive** Python AST companions without converting regex PY-* to AST.

## Decision

1. **Always-on cheap catalogue.** Regex antipatterns plus AST run on changed
   files at gate-time surfaces: CLI `anvil check` / `anvil gate`, MCP
   `anvil_check`, planless MCP `anvil_gate`, and (later, GTAO-003) a
   background CLI subprocess after a daemon `allow`. Interactive save /
   MidEdit / `anvil_validate_write` remain regex-only.

2. **AST stays off the daemon crate.** `anvil-checks-ast` may grow
   `tree-sitter-python` the same way it already depends on `tree-sitter-rust`
   (workspace pin, not `anvil-kernel`). The daemon never links it. Background
   follow-up is a CLI subprocess (ADR-067 precedent).

3. **Interactive verdicts do not wait.** A background follow-up must not
   delay the ADR-031 save/pre-write p95. Failure is fail-safe: the original
   allow/warn stands.

4. **Full `anvil gate` remains a workflow event.** Commit hook (if
   installed), CI, or explicit CLI/MCP with no `targetFiles`. Lint, test,
   coverage, dependency, policy, and npm audit do not run on save.

5. **`anvil watch --action gate` stays opt-in.** Default watch action remains
   `check`.

6. **CI adopter template** should run `anvil gate --profile ci` **in addition
   to** L4 pre-push, not instead of it (GTAO-006; does not close CIB-294).

7. **Kill switch for the background follow-up.** Environment
   `ANVIL_AST_FOLLOWUP=0` (and an equivalent config key when that item lands)
   disables GTAO-003 without touching save-time regex. Given ADR-061 load
   history, the follow-up is changed-path, coalesced, and budget-gated
   (GTAO-005) before it is default-on.

8. **Python AST is additive companion rules** (new `PY-01x` ids) via language
   dispatch in `anvil-checks-ast`. Regex PY-001…009 stay on save-time.
   Comment-level PY-001/002/003 stay regex forever. Do not convert PY-008 to
   `detection: ast`.

## Rationale

The cheap catalogue is single-file and parse-bound; the full gate is a merge
judgement. Mixing them on the hot path recreates the save-storm. Converting
regex PY-008 to AST would take an `error` rule off save-time. Companion ids
keep ERROR coverage live while AST covers documented blinds at gate.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Chosen: cheap catalogue always-on; full gate rare; Python AST additive** | Preserves ADR-031/064/061; closes MCP/AST gap; Python without darkening save-time | Two scan paths; follow-up spawn cost; companion-id catalogue growth |
| Default `anvil watch --action gate` | Full catalogue on every save | Recreates ~6.55-core cold gate; lint/test/audit on keystroke |
| Feature flag `ast` on `anvil-checks` | No new crate | Cargo feature unification links tree-sitter into the daemon (ADR-064) |
| Feed trees into the daemon | AST at save-time in-process | Widens attestation; tree-sitter re-enters daemon |
| Convert PY-008 to `detection: ast` | One id | `eval()` stops firing at save-time |
| New crate for Python AST | Isolation | Duplicate registry/suppression machinery; ADR-071 already language-general |

## Consequences

- **Positive:** MCP check/gate and CLI check/gate agree on AST. Python
  regex-blind shapes become expressible. Save-time latency budget untouched.
- **Negative:** Background follow-up adds a short-lived CLI process after
  allow (spawn dominates parse). Gate on a Python-heavy repo pays a second
  tree-sitter parse (kernel already parsed for the graph).
- **Risks:** Uncoalesced follow-up or `anvil check --all` per save returns to
  the ADR-061 storm. Grammar parse-skip blinds AST files. Companion ids can
  confuse (“save was green, gate has PY-010”).
- **Mitigations:** GTAO-005 budget proof; kill switch; SARIF/output `ast`
  tag; PY-010 rule body names PY-004 as the save-time sibling.

## References

- Related ADRs: ADR-031, ADR-038, ADR-061, ADR-064, ADR-067, ADR-071
- APS modules: GTAO-001…010, PYLAN (grammar only; items not reopened)
- External: CIB-332, CIB-294, RLB-007
