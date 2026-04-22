# Onboarding Language Gap Audit

## Purpose

This document is the `CLAR-004` output. It audits today's onboarding,
welcome, tutorial, and adjacent docs against the canonical model in
`plans/specs/2026-04-21-anvil-quality-language-design.md`.

It also records forward-looking language risk in `RCLI2` and `RCLI3`, since
those modules will expand the command surface and can easily entrench further
drift if not aligned now.

## Audit Standard

The canonical model says onboarding should teach in this order:

1. what Anvil sees
2. what checks are
3. what findings are
4. what a gate decides
5. how commands and modes expose that loop

Any surface that leads with command names, implementation jargon, or parallel
noun systems before teaching that model is considered a gap.

## Summary

Current onboarding and tutorial surfaces are much better than the older static
flows, but they still over-index on commands and implementation nouns before
the model is explicit. The biggest gaps are:

- onboarding/config presents a check list that is known not to match gate-runner
  check names
- discovery uses `findings`, which is good, but the surrounding flow does not
  explicitly connect findings to checks and gates
- tutorial path descriptions are command-oriented and subsystem-oriented rather
  than model-oriented
- docs mix `architecture`, `boundary`, `violation`, and `check` without a clear
  hierarchy
- forward-looking CLI plans (`RCLI2`, `RCLI3`) introduce many new commands with
  no shared wording contract yet

## Gap Matrix

| Surface | What it teaches today | Gap against canonical model | Severity |
| --- | --- | --- | --- |
| Guided init defaults | Project has enabled `checks` named `secret-detection`, `import-boundaries`, `antipattern-scan`, `architecture`, `policy` | Check names are not aligned with gate-runner names; user is not told whether these are checks, check families, or aliases | Critical |
| Welcome discovery flow | Anvil scans project for findings | Strong start, but the flow does not explicitly say scans feed checks and gates | Medium |
| Welcome quick actions | Run Gate, Start Watch, Run Audit, Run Doctor, Run Tutorial | Presents commands/modes as peers before teaching their relationship | High |
| Tutorial path chooser | Policy, Architecture, Drift, CI Integration | Organises learning by subsystem, not by core mental model; no initial “how Anvil works” frame | High |
| Architecture tutorial doc | boundaries, layers, architecture file, `anvil check --all`, architecture violation | Uses boundary language well, but still relies on `check`/`violation` without explaining how they fit into findings and gates | Medium |
| `RCLI2` plan | adds `check`, `validate`, `drift`, `gate-config`, `policy-debug`, `policy-watch`, `pr-comment`, `exception` | Large future command expansion with no shared product-language guardrails | High |
| `RCLI3` plan | adds `edda`, `ember`, `plan validate`, `plan load`, `plan status` | Expands the binary into governance/workflow domains, increasing risk of `validate`, `status`, and workflow terms diverging from the quality model | Medium |

## Detailed Findings

### 1. Guided init has a known broken conceptual contract

Source:

- `crates/anvil-cli/src/commands/defaults.rs`

Observed behaviour:

- the file explicitly states that onboarding/config check names are not the same
  vocabulary as gate-runner dispatch names
- guided init writes `.anvilrc` using `secret-detection` and
  `import-boundaries`, while gate execution uses different identifiers

Why this is a gap:

- first-run setup is where users form their first stable map of the product
- if setup says “these are your checks” but execution later exposes different
  names, the user learns drift rather than the model

Required correction:

- onboarding must not present a check catalogue that is conceptually separate
  from the gate/check system unless the distinction is explicit and intentional

Severity: Critical

### 2. Discovery scan is promising but under-explained

Source:

- `crates/anvil-cli/src/commands/welcome.rs`

Observed behaviour:

- discovery says “Scanning project for findings...`
- scan results merge secret and antipattern outputs into `Finding`
- showcase mode also uses findings consistently

Why this is a gap:

- `finding` is the right generic noun, but the flow still does not teach that
  scans gather evidence, checks evaluate concerns, and gates summarise whether
  work can advance

Required correction:

- add one short explanatory step or heading that links scan -> checks ->
  findings -> gate

Severity: Medium

### 3. Welcome hub presents modes before relationships

Source:

- `crates/anvil-cli/src/commands/welcome.rs`

Observed behaviour:

- the hub offers gate, watch, docs, audit, doctor, tutorial as sibling actions

Why this is a gap:

- a new user sees a menu of tools, not a model of how the tools fit together
- `gate`, `watch`, `audit`, and `doctor` feel like parallel products instead of
  views over related concepts

Required correction:

- rename or subtitle quick actions in terms of purpose, eg setup health,
  project findings, continuous checks, workflow gate
- add a one-line explanation of how they connect

Severity: High

### 4. Tutorial paths are subsystem-first, not understanding-first

Source:

- `crates/anvil-tui/src/surfaces/tutorial/mod.rs`

Observed behaviour:

- paths are `Policy`, `Architecture`, `Drift`, `CI Integration`
- descriptions include phrases like “Learn to write and test gate policies” and
  “Set up architecture boundary enforcement”

Why this is a gap:

- users are routed straight into subsystems before learning the common model
- `gate policies` is especially confusing because it merges `gate` and `policy`
  without clarifying whether policies are checks, gates, or something else

Required correction:

- add a short foundation step before path selection
- revise path descriptions to anchor them in checks/findings/gates first

Severity: High

### 5. Architecture docs are closest to the desired model, but still incomplete

Source:

- `docs/public/anvil/tutorials/architecture.md`

Observed behaviour:

- strong use of `boundaries`
- clear explanation that Anvil analyses import graphs
- later step switches to `anvil check --all` and reports `architecture violation`

Why this is a gap:

- this is one of the best surfaces for the model, but it still assumes the
  reader already understands where checks, findings, and gates sit

Required correction:

- explain that the architecture tutorial is one family of checks over the code
  graph, and that violations are one kind of finding

Severity: Medium

## Forward-Looking Risk

### `RCLI2`

Source:

- `plans/modules/rust-cli-tier2.aps.md`

Risk profile:

- introduces more commands around the same quality space: `check`, `validate`,
  `drift`, `gate-config`, `policy-debug`, `policy-watch`, `pr-comment`,
  `exception`
- uses phrases like “planless file analysis”, “GateRunner in planless mode”,
  “reports warnings”, and “gate results” without the canonical terminology
  guardrails being stated in the module

Why this matters:

- once Tier 2 ships, these command names become durable user vocabulary
- if they land before language cleanup, they will constrain later fixes

Recommendation:

- add a wording guard to `RCLI2` tasks and spec references: checks are the
  smallest evaluative unit, findings are the generic results noun, gate is the
  workflow judgement

### `RCLI3`

Source:

- `plans/modules/rust-cli-tier3.aps.md`

Risk profile:

- broadens the binary with governance and plan workflows: `plan validate`,
  `plan load`, `plan status`, plus Edda and Ember subsystem commands
- increases the chance that generic nouns like `validate`, `status`, and
  `workflow` mean different things across the same CLI

Why this matters:

- Tier 3 will make `anvil` feel like a larger operating surface, so the shared
  mental model becomes more important, not less

Recommendation:

- document a CLI naming rule before Tier 3 moves forward: quality commands must
  be framed around checks/findings/gates; governance commands must avoid
  accidentally reusing those words with different meanings

## Recommended Follow-On Changes

### Copy and UX

- add a foundation explanation step to welcome/tutorial
- revise quick action labels and subtitles around user goals
- rewrite tutorial path descriptions to reflect the canonical model
- update architecture tutorial intro to place boundaries within checks/findings

### Command and config language

- unify or explicitly map onboarding/config check names to gate check names
- review `gate-config` wording so it is clear whether it configures checks,
  gates, or both

### Plan reconciliation

- update `RCLI2` and `RCLI3` module wording to reference the canonical language
- review active dashboard and policy-governance plans for `gate` overload

## Exit Criteria For CLAR-004

This audit is complete when:

- the major onboarding/tutorial gaps are identified with sources
- forward-looking risk is captured for the main planned CLI expansions
- `CLAR-005` can turn these findings into bounded execution work
