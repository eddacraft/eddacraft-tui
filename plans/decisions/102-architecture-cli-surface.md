# ADR-102: Architecture CLI Surface — Resolve the Documented-but-Unbuilt Commands

## Status

Proposed

## Date

2026-07-06

## Context

`docs/guides/custom-architecture-policies.md` documents eight
`anvil architecture` subcommands — `init`, `check`, `watch`, `visualise`,
`list`, `impact`, `export`, `debug` — plus baseline-accepting flags
(`check --fix`, `check --baseline-all`). Its frontmatter claims the guide is
"Live" and reviewed against `crates/anvil-cli/src/commands/architecture.rs`.
Only `validate` and `show` exist there. Every quickstart step in the guide
begins with a command that does not run.

ARCHCFG-006 is the design gate requiring a build / defer / reject /
redirect-to-existing verdict for each candidate before ARCHCFG-007..014
proceed. The ground truth found during the gate:

- **No scaffold exists.** `resolve_arch_config` errors with "Create
  `.anvil/architecture.yaml` manually" — the guide's quickstart and every
  tutorial step downstream assume `init` exists.
- **Dependency analysis already ships** as the `import-boundaries` gate check
  (canonical name `import-boundaries`, legacy alias `architecture`, defined in
  `crates/anvil-cli/src/commands/check_catalog.rs`), run via
  `anvil gate --only-checks architecture`. ARCHCFG's own Out of Scope excludes
  "Dependency analysis and gate evaluation".
- **Watching already ships.** `anvil watch --action none` is documented in
  `crates/anvil-cli/src/commands/watch.rs` as "an architecture/dependency-only
  watch with no code-quality scan"; `--action gate` runs the gate (including
  import-boundaries) on change, and the save-time daemon (ADR-101) supervises
  durable watchers.
- **The structured definition is already exposed.**
  `anvil architecture show --json` returns template, layers, patterns,
  `depends_on`, and rules count.
- **Graph impact analysis exists** as the MCP tools `anvil_impact_of_change`
  and `anvil_find_dependents`.
- **Baseline machinery exists** (`anvil baseline`, baseline policy per
  ADR-039).

## Decision

Per-command verdicts:

| Command | Verdict | Shape / target |
| --- | --- | --- |
| `init` | **Build** (ARCHCFG-007 → Ready) | Non-interactive scaffold that writes a `.anvil/architecture.yaml` passing `anvil architecture validate` unmodified; optional `--template <layered\|hexagonal>`, default `layered`. The guide's "interactive wizard" framing is dropped. |
| `check` | **Redirect** (ARCHCFG-008 → Ready, rescoped to guide reconciliation) | The guide points at `anvil gate --only-checks architecture`. No wrapper command: a second entry point would need its own baseline, exit-code, and output semantics kept in sync with gate — drift by construction. |
| `check --fix` / `--baseline-all` | **Redirect** | Baseline acceptance belongs to the existing baseline machinery (`anvil baseline`, ADR-039), never an architecture-local baseline writer. |
| `watch` | **Redirect** (ARCHCFG-009 → Draft, guide fix folds into ARCHCFG-008) | `anvil watch --action none` already is the architecture/dependency-only watch; `--action gate` includes import-boundaries. No second file-watching loop. |
| `visualise` | **Build** (ARCHCFG-010 → Ready) | Renderer over the definition `show` already parses; `--format mermaid` only in the first pass. A new renderer, not a new data path. |
| `list` | **Reject** (ARCHCFG-011 → Draft, rejected) | Guide-invented synonym: `show` already lists layers, patterns, dependencies, and rule count. Guide corrected to `show` via ARCHCFG-008. |
| `impact` | **Defer** (ARCHCFG-012 → Draft) | *(Reframed by the 2026-07-06 amendment.)* Not a graph-tools projection: `anvil_impact_of_change` answers the inverse question (changed paths → dependents). The honest shape is a config dry-run diff — run the import-boundaries analysis under the current and a proposed `.anvil/architecture.yaml` and diff the violation sets (`impact --file <proposed.yaml>`), dropping the `--rule "<string>"` DSL entirely. Deferred behind the ARCHCFG-015 usage gate: it needs the check to accept an injected config, and its consumer (a team evolving a mature config) cannot exist before `init` ships. |
| `export` | **Defer** (ARCHCFG-013 → Draft) | `show --json` already serves machine consumers, and *(2026-07-06 amendment)* a substantial top-level `anvil export` already exists (plan conversion + constraint export), so the real question is which surface owns architecture-as-Markdown: extend `anvil export`, a namespace-local `architecture export`, or `visualise --format markdown`. Decide at the ARCHCFG-015 usage gate, after ARCHCFG-010 ships the renderer plumbing two of those options would share. |
| `debug` | **Defer** (ARCHCFG-014 → Draft) | The underlying need is real and cheap — there is no way today to see which files a layer's globs actually capture, the first confusion users will hit once `init` ships. *(2026-07-06 amendment)* If promoted at the ARCHCFG-015 usage gate it is a flag, not a subcommand: `show --layer <name> --files` (preferred) or `validate --explain`. |

Cross-cutting principles:

1. **`anvil architecture` stays a thin config-authoring surface** — author
   (`init`), check (`validate`), and render (`show`, `visualise`) the
   definition file. Analysis, watching, baselining, and impact live in their
   existing homes (gate/check catalog, `anvil watch`, `anvil baseline`, graph
   tools). The namespace must not grow parallel engines.
2. **The guide documents only shipped commands.** ARCHCFG-008 reconciles
   `custom-architecture-policies.md` with this ADR (quickstart, CLI Commands
   section, troubleshooting, and the false "Live / reviewed" freshness claim).

## Rationale

The gate's job was to stop documentation-driven scope creep without losing the
genuinely missing capability. `init` and `visualise` are real gaps with no
existing equivalent and a deterministic, warnings-friendly shape. Everything
else the guide describes already exists behind a different name — building it
again under `anvil architecture` would duplicate engines this module's Out of
Scope explicitly excludes.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Chosen: build 2, redirect 3, reject 1, defer 3 | Guide becomes truthful; no duplicate engines; the two builds close real gaps (unusable quickstart, no diagram output) | Users of the old guide text lose documented (but never functional) commands |
| Build all eight as documented | Guide unchanged | Duplicates gate/watch/baseline/graph engines; violates module Out of Scope; large new surface with zero demand evidence |
| Reject all, docs-only correction | Minimal work | Quickstart stays impossible (no scaffold); no diagram renderer; guide reduced to hand-authoring YAML |
| Thin `architecture check` wrapper over the gate check | Familiar command name | Second entry point whose baseline/exit-code/output semantics must track gate forever; the redirect costs one line of docs instead |

## Consequences

- **Positive:** `anvil architecture init` makes the guide's quickstart real;
  the guide stops claiming unshipped surface; ARCHCFG-007..014 have
  unambiguous statuses; the thin-namespace principle gives future reviewers a
  test for new `architecture` subcommand proposals.
- **Negative:** anyone pattern-matching on the published guide text loses
  `check`/`watch`/`list` spellings; the redirect targets
  (`gate --only-checks architecture`, `watch --action none`) are less
  discoverable than a dedicated subcommand.
- **Risks:** deferred items (`impact`, `export`, `debug`) linger as Draft
  zombies; the rejected `list` could be re-proposed without new evidence.
- **Mitigations:** each ARCHCFG item carries its verdict inline with a pointer
  here. Deferred items re-enter only through **ARCHCFG-015**, a usage-gated
  design review that re-verdicts them (and sweeps for any other
  documented-but-missing architecture surface) once shipped commands generate
  adoption signal; its outcome lands as a dated amendment to this ADR.
  Re-opening a rejected or redirected item still requires demand evidence and
  an amendment here.

## Amendments

### 2026-07-06 — deferral corrections and the usage gate

Same-day amendment following a deeper pass on the three deferred commands:

- **`impact` substrate corrected.** The original rationale named
  `anvil_impact_of_change`/`anvil_find_dependents` as the substrate; those
  answer "changed paths → dependents", the inverse of the guide's question
  ("proposed rule → violating edges"). The candidate shape is now a config
  dry-run diff (`impact --file <proposed.yaml>` comparing import-boundaries
  violations under current vs proposed config); the `--rule "<string>"` DSL is
  dropped. Verdict unchanged: defer.
- **`export` collision recorded.** A substantial top-level `anvil export`
  (plan conversion + constraint export,
  `crates/anvil-cli/src/commands/export.rs`) was missed in the original
  survey. The open question is surface ownership, decided after ARCHCFG-010.
  Verdict unchanged: defer.
- **`debug` shape recorded.** If promoted, it is `show --layer <name> --files`
  (or `validate --explain`) — a flag on existing commands, never a
  subcommand. Verdict unchanged: defer.
- **Re-entry mechanism named.** ARCHCFG-015 (usage-gated design review) now
  owns deferral re-entry, replacing per-item "amendment + demand evidence"
  with one explicit gate: it opens only once `init` and `visualise` have
  shipped **and** a concrete usage signal (issue, support question, telemetry,
  explicit request) names a deferred capability.

## References

- Related ADRs: ADR-039 (baseline policy), ADR-056 (`--format` output
  selector conventions, relevant to `visualise`), ADR-101 (save-time watch
  drivers)
- APS modules: ARCHCFG-006..015
  (`plans/modules/architecture-config-validation.aps.md`)
- Code: `crates/anvil-cli/src/commands/architecture.rs`,
  `crates/anvil-cli/src/commands/check_catalog.rs`,
  `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/commands/baseline.rs`,
  `crates/anvil-cli/src/commands/export.rs`
- Docs: `docs/guides/custom-architecture-policies.md`
