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
- **Watching already ships.** `anvil watch --run none` is documented in
  `crates/anvil-cli/src/commands/watch.rs` as "an architecture/dependency-only
  watch with no code-quality scan"; `--run gate` runs the gate (including
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
| `watch` | **Redirect** (ARCHCFG-009 → Draft, guide fix folds into ARCHCFG-008) | `anvil watch --run none` already is the architecture/dependency-only watch; `--run gate` includes import-boundaries. No second file-watching loop. |
| `visualise` | **Build** (ARCHCFG-010 → Ready) | Renderer over the definition `show` already parses; `--format mermaid` only in the first pass. A new renderer, not a new data path. |
| `list` | **Reject** (ARCHCFG-011 → Draft, rejected) | Guide-invented synonym: `show` already lists layers, patterns, dependencies, and rule count. Guide corrected to `show` via ARCHCFG-008. |
| `impact` | **Defer** (ARCHCFG-012 → Draft) | If ever built it must be a projection over the existing graph impact tools, but the `--rule "<string>"` parsing shape is unproven and there is no demand evidence. Revisit after `init`/`validate` adoption. |
| `export` | **Defer** (ARCHCFG-013 → Draft) | `show --json` already serves machine consumers; a Markdown export should reuse ARCHCFG-010's renderer plumbing once that exists rather than grow parallel rendering code now. |
| `debug` | **Defer** (ARCHCFG-014 → Draft) | The real troubleshooting need ("which files match this layer") is better served as an explain flag on `validate`/`show` if demand appears; no standalone case today. |

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
  (`gate --only-checks architecture`, `watch --run none`) are less
  discoverable than a dedicated subcommand.
- **Risks:** deferred items (`impact`, `export`, `debug`) linger as Draft
  zombies; the rejected `list` could be re-proposed without new evidence.
- **Mitigations:** each ARCHCFG item carries its verdict inline with a pointer
  here; re-opening a deferred/rejected item requires demand evidence and an
  amendment to this ADR.

## References

- Related ADRs: ADR-039 (baseline policy), ADR-056 (`--format` output
  selector conventions, relevant to `visualise`), ADR-101 (save-time watch
  drivers)
- APS modules: ARCHCFG-006..014
  (`plans/modules/architecture-config-validation.aps.md`)
- Code: `crates/anvil-cli/src/commands/architecture.rs`,
  `crates/anvil-cli/src/commands/check_catalog.rs`,
  `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/commands/baseline.rs`
- Docs: `docs/guides/custom-architecture-policies.md`
