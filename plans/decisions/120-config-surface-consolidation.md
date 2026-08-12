# ADR-120: Config Surface Consolidation on the Multi-Format Loader

## Status

Accepted 2026-08-13 (operator)

## Date

2026-08-12

## Context

ADR-016 (Proposed, 2026-04-03) diagnosed configuration drift across three files
(`.anvilrc`, `.anvil/gate-config.json`, `.anvil/architecture.yaml`) and proposed
consolidating them into a single TOML-only `.anvilrc` with source delegation. It
was never accepted, and two of its premises have since been invalidated by
shipped work:

1. **Format posture reversed.** MLP-011 (Done 2026-05-13) shipped
   `crates/anvil-config` as a multi-format loader: `.anvil.{yaml,yml,json,toml}`
   discovery, yaml-first precedence (`DISCOVER_PRECEDENCE`), bounded parsing,
   canonical JSON hashing, and schema migrations. MLP2-040 moved CLI surfaces to
   `.anvil.<ext>` with `.anvilrc` as a read fallback. TOML-only is no longer a
   greenfield choice; it would be a format reversal against shipped behaviour.
2. **"No users to migrate" no longer holds.** v0.9.0-beta shipped 2026-07-12;
   dogfooding repos, agent harnesses, and test fixtures carry live configs.

Meanwhile the drift ADR-016 targeted has grown worse, not better:

- The main config answers to **five filenames**: `.anvilrc` (still written by
  `anvil init`, `init.rs:33`) plus `.anvil.{yaml,yml,json,toml}` (written by
  `anvil start --format`; `anvil config set` writes back to whichever file it
  discovers — including a legacy `.anvilrc`). Two commands seed different
  filenames for the same logical file; every reader dual-probes both.
- Two live code paths **disagree on which file wins** when both names exist:
  `gate.rs` resolves discover-first (`.anvil.<ext>` beats `.anvilrc`,
  test-pinned) while `start.rs existing_project_config_path` probes `.anvilrc`
  first — the dual-config state has two contradictory answers today.
- Key casing drifts **per writer**, not just per format: `init.rs`
  `yaml_serialise` emits camelCase while its `toml_serialise` emits snake_case
  for the same struct, and `start.rs pre_write_anvil_config_format`
  deliberately emits camelCase across *all* formats (including TOML) so
  MLP2-041's `InitConfigView::from_value` can read it back.
- `.anvil/gate-config.json` still exists but **no gate run reads it** — check
  selection comes from the main config (`gate.rs read_anvilrc_checks`). The
  shipped skill documents it as a "planning surface", i.e. a file whose
  contents the product ignores.
- `.anvil/architecture.yaml` remains a separate file with its own parser
  (`anvil-architecture/src/yaml_parser.rs`).
- `anvil/policy.*` discovery precedence is **hand-rolled in two places**
  (`commands/hook.rs`, `commands/l4_validate.rs`) instead of using
  `anvil_config::discover` — a fourth independent precedence implementation.

Two Proposed modules are blocked on this slot: SETCON declares UCFG its
upstream source of truth for config file layout, and SETPREF names UCFG as the
config writer contract.

## Decision

Consolidate the project config surface **on top of** the shipped multi-format
loader, superseding ADR-016 before acceptance:

1. **Canonical filename is `.anvil.<ext>`** where `<ext>` ∈
   {yaml, yml, json, toml}, format encoded in the name. `anvil init` writes
   `.anvil.yaml` by default (or the ext matching a `--format` choice) and stops
   writing `.anvilrc`. `.anvilrc` becomes **legacy: no command creates one**.
   `anvil config set` may keep editing a *discovered* legacy `.anvilrc` in
   place (no silent rename — migration stays explicit via `anvil migrate`),
   with `anvil doctor` nagging while one exists. Docs name exactly one
   canonical spelling.
2. **Format-agnostic, yaml-first, discover-first.** MLP-011's
   `DISCOVER_PRECEDENCE` stands; no TOML-only mandate. **Discover-first is the
   single precedence truth**: `.anvil.<ext>` beats `.anvilrc` everywhere, as
   `gate.rs` already resolves — `start.rs`'s legacy-first probe is reconciled
   to match (its pinning test flips). One config file per project;
   `anvil doctor` warns (exit 0) when both `.anvilrc` and a `.anvil.<ext>`
   exist, or when multiple `.anvil.<ext>` variants exist, naming the winner.
3. **snake_case is the canonical key space across all formats.** Legacy
   camelCase keys are accepted on read via the anvil-config migration layer and
   rewritten on the next owned write; new writes are snake_case only. This
   binds the actual writer functions (`init.rs` serialisers,
   `start.rs pre_write_anvil_config_format`), not just the anvil-config
   canonical layer, and MLP2-041's `InitConfigView::from_value` reader gains
   snake_case tolerance before any writer flips.
4. **`.anvil/gate-config.json` is retired.** Gate composition state moves into
   a `gate` section of the main config (which gate runs already treat as
   authoritative for check selection). `anvil gate-config` becomes a
   reader/writer over that section. `anvil migrate` folds a legacy
   `gate-config.json` into the section — and because gate runs ignore the JSON
   today, a stale file attracts no review pressure, so **folds that would
   weaken enforcement relative to the effective config** (disabling checks,
   lowering thresholds) **require explicit diff-and-confirm**; the fold report
   names every folded key. `anvil doctor` warns while the stray file remains.
5. **Architecture config becomes a main-config section with source
   delegation.** ADR-016's `SectionOrSource<T>` survives, format-agnostic:
   `architecture` is either inline or `architecture.source = "<path>"`
   pointing at a file in any supported format. Delegation is exclusive
   (inline XOR source), one level deep, and path-safe: workspace-relative only
   (absolute and Windows drive/UNC paths rejected), the resolved target
   canonicalised and verified to remain under the workspace root after symlink
   resolution, and self-reference (the source resolving to the main config
   itself) rejected. Delegated targets are read through anvil-config's
   hardened bounded path — size cap, YAML alias rejection, depth cap
   (ADR-046) — regardless of format; the legacy `yaml_parser` survives only as
   a type mapper. Existing `.anvil/architecture.yaml` files keep working as
   delegation targets; `anvil migrate` writes the explicit `source` line.
6. **One discovery implementation.** `commands/hook.rs` and
   `commands/l4_validate.rs` replace their hand-rolled `anvil/policy.*`
   candidate lists with `anvil_config::discover` — with **one deliberate
   behaviour change**: the hand-rolled lists probe `policy.yml` before
   `policy.yaml` (test-pinned), while `DISCOVER_PRECEDENCE` is yaml-first, so
   a repo holding both variants flips winner. The flip is adopted explicitly
   (one precedence rule beats a per-basename carve-out), and `anvil doctor`
   warns when multiple `anvil/policy.<ext>` variants exist, naming the winner
   under the unified rule. Policy **authority** semantics (ADR-100:
   committed-to-count) are untouched; this changes only how the file is
   found.
7. **Migration is in scope.** Unlike ADR-016's greenfield assumption,
   `anvil migrate` owns the rename and folds; nothing breaks a repo that has
   not migrated (readers keep legacy fallbacks for at least one minor
   release). Warnings over blocks throughout (ADR-002 posture).

Out of scope: user-level `workspace.yaml` (daemon confinement — different
trust domain, ADR-094), `anvil/exceptions/` (committed-authority store,
ADR-100), `.anvil/suppressions.json` / baselines / dashboards / `gates.json`
(state, not configuration), `flags/manifest.json` (FLAGCAT), policy merge
semantics (POLLC/ORGHIER), and any new config sections beyond `gate` and
`architecture`.

## Rationale

The consolidation problem is real and worsening, but ADR-016's remedy now
costs a second format migration on top of the one MLP-011 already shipped. The
cheapest path to "one file, one name, one key casing, one discovery rule" is
to finish the MLP2-040 direction rather than reverse it. Retiring
`gate-config.json` is mostly deletion — the gate already ignores it, so the
honest move is to make the surface match the behaviour.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Consolidate on multi-format `.anvil.<ext>` (chosen) | Finishes shipped direction; no format reversal; smallest migration; fixes dual-seeding and dead-file honesty gaps | Multi-format surface keeps 4 valid extensions; casing rules must span 3+ formats |
| Resurrect ADR-016 TOML-only `.anvilrc` | Single format, simplest schema story | Reverses shipped MLP-011/MLP2-040; migrates every beta user twice; contradicts yaml-first docs and fixtures |
| Status quo + documentation | Zero code | Leaves two seeding paths writing different filenames, a config file the product ignores, and four precedence implementations |
| One mega-file absorbing policy/exceptions too | Ultimate consolidation | Breaks ADR-100 committed-authority boundary; policy trust model must stay separate |

## Consequences

- **Positive:** one canonical filename and key casing for agents and docs;
  `gate-config.json` honesty gap closed; architecture config reachable through
  the same loader; a single precedence implementation to test and document;
  SETCON/SETPREF get a truthful upstream.
- **Negative:** a deprecation window where readers probe both `.anvilrc` and
  `.anvil.<ext>`; `anvil gate-config` changes its backing store; docs churn
  across config.md, quickstart, first-project, skills, runbooks.
- **Risks:** silent behaviour change for repos with *both* a stale
  `gate-config.json` and main-config `checks` (today the JSON is ignored;
  after folding, migrate must not resurrect stale intent — a main config with
  no `gate` section at all makes *every* JSON field "absent"); the deliberate
  `policy.yml`→`policy.yaml` precedence flip in dual-variant repos; TOML/JSON
  in-place edits may reorder keys on write.
- **Mitigations:** migrate folds only fields absent from the main config,
  reports every folded key, and requires diff-and-confirm for
  enforcement-weakening folds; doctor surfaces dual-truth states (main config
  names *and* `anvil/policy.*` variants) before and after; round-trip tests
  per format; ADR-002 warnings-not-blocks throughout.

## References

- Supersedes: ADR-016 (Proposed, never accepted → mark Rejected with pointer)
- Related ADRs: ADR-002 (warnings over blocks), ADR-046 (hardened YAML parse:
  size cap, alias reject, depth cap — delegated targets inherit it), ADR-094
  (workspace.yaml), ADR-100 (committed authority; policy loading unchanged),
  ADR-102 (ARCHCFG architecture CLI surface — authoring commands sit atop the
  resolved section). Sequencing note: ARCHCFG-007 (`anvil architecture init`,
  Ready) may ship before UCFG-007 — its standalone `.anvil/architecture.yaml`
  scaffold stays valid as a delegation target; once UCFG-007 lands, the
  scaffold switches to the `architecture` section with an explicit `source`
  line
- APS modules: UCFG (rewritten under this ADR), SETCON, SETPREF, MLP2 (MLP-011,
  MLP2-040 as shipped baseline)
- Evidence: council-09fc9567 (original drift findings, via ADR-016);
  `crates/anvil-cli/assets/skills/using-anvil/SKILL.md:139` (gate-config
  "planning surface" concession)
