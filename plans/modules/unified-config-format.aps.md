<!--
APS Module: Unified Config Format
=========================
Consolidate the project config surface on the shipped multi-format
loader: one canonical filename, gate section folded in, architecture
as a delegatable section, one discovery implementation.

Scopes: UCFG (main)

Rewritten 2026-08-12 under ADR-120. The original 2026-04 revision
(TOML-only single .anvilrc, 18 items, ADR-016) was invalidated by
MLP-011/MLP2-040 before any item started; no UCFG item from that
revision was ever executed, so IDs are reallocated cleanly.
-->

# Unified Config Format

| ID   | Owner | Priority | Status   | Progress |
| ---- | ----- | -------- | -------- | -------- |
| UCFG | —     | medium   | Ready    | 0/12     |

**Last reviewed:** 2026-08-12 — module rewritten against ADR-120 (Proposed),
superseding the ADR-016 revision. Verified against current code: `anvil init`
still writes `.anvilrc` (`init.rs:33`) while `start`/`config set` write
`.anvil.<ext>`; gate runs ignore `.anvil/gate-config.json`; policy discovery is
hand-rolled in `hook.rs` and `l4_validate.rs`. Council `council-0851e9cb`
(standard pack, same day) findings folded in: start-vs-gate precedence split,
`config set` legacy writer, policy yml→yaml flip made explicit, delegation
containment hardened, fold confirmation, casing-writer ownership, TUI
tutorial surface, ARCHCFG-007 sequencing. **2026-08-13:** ADR-120 Accepted (operator); module flipped to
Ready against the v0.10.0-beta window.

> **Activation gate: satisfied 2026-08-13.** ADR-120 Accepted (operator);
> scheduled against the **v0.10.0-beta** release window. SETCON reads what
> this module defines and SETPREF names it as the writer contract; both stay
> anchored here.

## Purpose

Give Anvil's project configuration one canonical filename, one key casing, one
discovery/precedence implementation, and no config file the product silently
ignores — by finishing the MLP-011/MLP2-040 direction rather than reversing
it.

**Why:** the drift ADR-016 diagnosed has worsened: five valid filenames for
the main config, two commands seeding different ones, a `gate-config.json`
that gate runs ignore, a separately-parsed `architecture.yaml`, and four
independent precedence implementations. AI agents — Anvil's primary audience —
must correlate all of them to understand configuration.

**ADR:** [120-config-surface-consolidation](../decisions/120-config-surface-consolidation.md)
(supersedes [016](../decisions/016-unified-config-format.md))

## In Scope

- `anvil init` writes canonical `.anvil.<ext>`; `.anvilrc` demoted to legacy —
  no command creates one (`config set` may still edit a discovered legacy file
  in place; migration stays explicit)
- Discover-first precedence made the single truth: `start.rs`'s legacy-first
  probe reconciled to `gate.rs`'s discover-first behaviour
- `anvil migrate` owns rename (`.anvilrc` → `.anvil.<ext>`) and
  `gate-config.json` fold; `anvil doctor` detects dual-truth states
- snake_case canonical key space across formats, camelCase accepted on read
- `gate` section in the main config as the authoritative gate-composition
  store; `anvil gate-config` re-pointed; `.anvil/gate-config.json` retired
- `architecture` section with exclusive, one-level, path-safe source
  delegation (`SectionOrSource<T>` in `crates/anvil-config`); existing
  `.anvil/architecture.yaml` continues working as a delegation target
- Policy discovery in `hook.rs` / `l4_validate.rs` unified onto
  `anvil_config::discover` — with one deliberate, doctor-warned behaviour
  change (`policy.yml`→`policy.yaml` precedence flip in dual-variant repos)
- Docs, skill, runbook, MCP resource, and fixture sweeps to the canonical name

## Out of Scope

- Format changes (multi-format yaml-first stands; no TOML-only)
- `workspace.yaml` daemon confinement (ADR-094 trust domain)
- `anvil/exceptions/` store (ADR-100 committed authority)
- `.anvil/suppressions.json`, baselines, dashboards, `gates.json` (state, not
  config)
- `flags/manifest.json` (FLAGCAT) and policy merge semantics (POLLC/ORGHIER)
- New sections beyond `gate` and `architecture`
- Removing legacy read fallbacks (deferred ≥1 minor release after migrate
  ships)

## Interfaces

**Depends on:**

- `crates/anvil-config` — existing discover/parse/canonical/migrations layer
  (MLP-011); gains `SectionOrSource<T>` and casing canonicalisation
- `crates/anvil-architecture` — `ArchitectureDefinition` types and template
  defaults (types reused; YAML file becomes a delegation target)
- `crates/anvil-cli` — init, migrate, doctor, gate, gate-config, watch,
  architecture, config, MCP resources

**Exposes:**

- One canonical config filename + documented precedence for all consumers
- `gate` and `architecture` section schemas in the main config
- Delegation contract consumed by SETCON's resolver and SETPREF's writer

**Coordinates with:**

- [settings-truth-contract](./settings-truth-contract.aps.md) (SETCON) —
  reads the file layout this module fixes
- [settings-safe-preferences](./settings-safe-preferences.aps.md) (SETPREF) —
  writer behaviour contract
- ARCHCFG / ADR-102 — architecture authoring commands sit atop the resolved
  section; command surface decisions stay with ADR-102. **Sequencing:**
  ARCHCFG-007 (`anvil architecture init`, Ready) may ship before UCFG-007 —
  its standalone `.anvil/architecture.yaml` scaffold stays valid as a
  delegation target; once UCFG-007 lands, the scaffold switches to the
  `architecture` section with an explicit `source` line (owned by UCFG-008's
  command sweep)

## Constraints

- Deterministic; warnings over blocks (doctor/gate exit 0 on drift findings)
- Atomic writes for all config mutations; no key-reordering churn beyond the
  owning write
- Delegation is exclusive (inline XOR source), one level deep,
  workspace-relative; rejected: `../` traversal, absolute and Windows
  drive/UNC paths, symlink escapes (canonicalised target must stay under the
  workspace root), and self-reference. Delegated targets are read via
  anvil-config's hardened bounded path (size cap, YAML alias rejection, depth
  cap — ADR-046) regardless of format
- No behaviour change for unmigrated repos beyond new warnings — except the
  ADR-120 pt 6 policy precedence flip, which is deliberate and doctor-warned
- Migrate folds only fields absent from the main config and reports every
  folded key; folds that weaken enforcement relative to the effective config
  require explicit diff-and-confirm (never resurrects stale
  `gate-config.json` intent silently — a main config with no `gate` section
  makes every JSON field "absent", which is exactly the confirm case)

## Ready Checklist

Change status to **Ready** when:

- [x] ADR-120 Accepted 2026-08-13, operator (ADR-016 Rejected with pointer)
- [x] Release window named: v0.10.0-beta
- [x] Work items reviewed against current `main` (council `council-0851e9cb`,
      2026-08-12)

---

## Work Items

### Phase 1 — Canonical Filename

#### UCFG-001: init writes canonical `.anvil.<ext>`

- **Status:** Proposed
- **Intent:** `anvil init` writes `.anvil.yaml` by default (ext follows the
  chosen format); stops writing `.anvilrc`. TUI init surface labels updated
  (it currently suppresses `.anvil.*` labels because init never writes them).
  Depends on UCFG-003 so the first canonical file is snake_case from its
  first write — never a camelCase `.anvil.yaml`
- **Expected Outcome:** fresh `anvil init` produces a snake_case
  `.anvil.<ext>` that every reader discovers first; no command creates a new
  `.anvilrc`
- **Validation:** init in a temp dir; `discover` finds the file and its keys
  are snake_case; grep confirms no `.anvilrc` creation path remains
- **Files:** `crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-tui/src/surfaces/init/`
- **Confidence:** High
- **Priority:** High
- **Dependencies:** UCFG-003

---

#### UCFG-002: migrate renames `.anvilrc`, doctor flags dual configs

- **Status:** Proposed
- **Intent:** `anvil migrate` renames `.anvilrc` → `.anvil.<ext>` preserving
  the embedded format (atomic). `anvil doctor` warns when both names exist,
  or multiple `.anvil.<ext>` variants exist, naming which file wins.
  Establish discover-first as the single precedence truth: reconcile
  `start.rs existing_project_config_path` (today `.anvilrc`-first) to match
  `gate.rs`'s discover-first resolution, flipping its pinning test; `config
  set`'s writable-path keeps editing a discovered legacy `.anvilrc` in place
  but never creates one
- **Expected Outcome:** one migration command moves a legacy repo to the
  canonical name; every code path answers the dual-config question the same
  way; dual-truth states are visible, not silent
- **Validation:** migrate round-trip per format; doctor warning text asserts
  the winning path; dual-file fixture proves start/gate/config-set agree;
  exit 0 throughout
- **Files:** `crates/anvil-cli/src/commands/migrate.rs`,
  `crates/anvil-cli/src/commands/doctor.rs`,
  `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/commands/config.rs`
- **Confidence:** High
- **Priority:** High
- **Dependencies:** UCFG-001

---

#### UCFG-003: snake_case canonical key space

- **Status:** Proposed
- **Intent:** canonicalise keys to snake_case across yaml/json/toml in
  `anvil-config`; accept legacy camelCase on read via the migrations layer;
  owned writes emit snake_case only. This owns the actual divergent writer
  functions — `init.rs` `yaml_serialise` (camelCase today) /
  `toml_serialise`, and `start.rs pre_write_anvil_config_format` (camelCase
  across all formats today) — and gives MLP2-041's
  `InitConfigView::from_value` snake_case read tolerance before any writer
  flips
- **Expected Outcome:** identical logical config in any format parses to the
  same canonical form; camelCase-era files still load; no writer in the tree
  emits camelCase
- **Validation:** cross-format equivalence tests incl. canonical-hash
  stability; legacy-casing fixture loads with a deprecation note;
  `InitConfigView` round-trip against snake_case output
- **Files:** `crates/anvil-config/src/{canonical,migrations,parse}.rs`,
  `crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-cli/src/commands/start.rs`
- **Confidence:** Medium — casing rules must not disturb canonical hashing
  consumers
- **Priority:** High
- **Dependencies:** None

---

### Phase 2 — Retire gate-config.json

#### UCFG-004: `gate` section schema in the main config

- **Status:** Proposed
- **Intent:** define the `gate` section holding what `gate-config.json`
  carried (check enablement, `overall_score`, global config), reconciled with
  the `checks` key gate runs already read
- **Expected Outcome:** one schema answers "which checks run and how is the
  gate composed"; round-trips in all formats
- **Validation:** round-trip tests; gate check selection unchanged for
  existing configs
- **Files:** `crates/anvil-config/src/`, `crates/anvil-cli/src/commands/gate.rs`
- **Confidence:** High
- **Priority:** High
- **Dependencies:** UCFG-003

---

#### UCFG-005: gate-config command re-pointed; legacy file folded

- **Status:** Proposed
- **Intent:** `anvil gate-config --list/--enable/--disable` reads/writes the
  main-config `gate` section. `anvil migrate` folds a legacy
  `gate-config.json` (only fields absent from the main config; every folded
  key reported; folds that weaken enforcement relative to the effective
  config — disabled checks, lowered thresholds — require explicit
  diff-and-confirm), then the stray file is a doctor warning
- **Expected Outcome:** no code path writes `.anvil/gate-config.json`; the
  file the product ignores no longer exists on migrated repos; a stale
  weakened JSON cannot become live enforcement silently
- **Validation:** enable/disable round-trip via main config; fold test with
  conflicting stale JSON proves main config wins; weakened-stale-JSON fixture
  with no `gate` section in the main config proves the fold stops for
  confirmation; grep zero writers
- **Files:** `crates/anvil-cli/src/commands/gate_config.rs`,
  `crates/anvil-cli/src/commands/{migrate,doctor}.rs`
- **Confidence:** Medium — in-place section editing must not reorder
  unrelated keys
- **Priority:** High
- **Dependencies:** UCFG-004

---

### Phase 3 — Architecture as a Delegatable Section

#### UCFG-006: `SectionOrSource<T>` in anvil-config

- **Status:** Proposed
- **Intent:** implement exclusive inline-XOR-source delegation with clear
  errors (both present, neither, nested delegation, missing target, `../`
  traversal, absolute or Windows drive/UNC path, symlink escaping the
  workspace root after canonicalisation, self-reference back to the main
  config), one level deep, workspace-relative, format-agnostic targets.
  Delegated targets are read via `read_to_string_bounded` and parsed via
  anvil-config's hardened path (size cap, YAML alias rejection, depth cap —
  ADR-046); the legacy `yaml_parser` is never handed a delegated path
- **Expected Outcome:** actionable error strings for every invalid topology;
  no panic on any malformed input; alias-bomb and deep-nesting payloads in a
  delegated target are rejected identically to the main config
- **Validation:** unit tests for all topologies incl. `../`, absolute-path,
  symlink-escape, and self-reference rejection; alias-bearing and
  deeply-nested delegated-yaml fixtures; property/fuzz pass over the
  delegation resolver
- **Files:** `crates/anvil-config/src/delegation.rs`
- **Confidence:** Medium — custom deserialisation across three formats
- **Priority:** High
- **Dependencies:** UCFG-003

---

#### UCFG-007: `architecture` section resolution + migrate

- **Status:** Proposed
- **Intent:** `architecture` becomes a main-config section
  (`SectionOrSource<ArchitectureDefinition>`). Existing
  `.anvil/architecture.yaml` keeps working as a delegation target;
  `anvil migrate` writes the explicit `source` line; template defaults
  reused from `anvil-architecture`
- **Expected Outcome:** inline and delegated architecture resolve to the same
  definition; unmigrated repos behave as before plus a doctor note
- **Validation:** resolved-equality test inline vs delegated; migrate adds
  `source` without touching other keys
- **Files:** `crates/anvil-config/src/`, `crates/anvil-architecture/src/`,
  `crates/anvil-cli/src/commands/migrate.rs`
- **Confidence:** Medium
- **Priority:** Medium
- **Dependencies:** UCFG-006

---

#### UCFG-008: gate / watch / architecture commands read the resolved section

- **Status:** Proposed
- **Intent:** replace direct `.anvil/architecture.yaml` reads in `gate.rs`,
  `watch.rs`, and `architecture*.rs` with the resolved section (inline or
  delegated); ADR-102 command-surface semantics unchanged
- **Expected Outcome:** all architecture consumers work identically with
  inline or delegated config; delegated file edits are watched
- **Validation:** existing architecture/gate/watch tests pass against both
  topologies
- **Files:** `crates/anvil-cli/src/commands/{gate,watch,architecture}.rs`,
  `crates/anvil-architecture/src/yaml_parser.rs`
- **Confidence:** Medium — gate.rs carries implicit config assumptions
- **Priority:** Medium
- **Dependencies:** UCFG-007

---

### Phase 4 — One Discovery Layer, One Story

#### UCFG-009: policy discovery via anvil_config::discover

- **Status:** Proposed
- **Intent:** replace the hand-rolled `anvil/policy.*` candidate lists in
  `hook.rs` and `l4_validate.rs` with `anvil_config::discover`. One
  deliberate behaviour change (ADR-120 pt 6): the hand-rolled lists are
  yml-first (test-pinned by
  `load_policy_prefers_yml_over_other_extensions`), `DISCOVER_PRECEDENCE` is
  yaml-first — dual-variant repos flip winner; the pinning test flips with
  the decision. `anvil doctor` warns when multiple `anvil/policy.<ext>`
  variants exist, naming the winner. ADR-100 authority semantics untouched
- **Expected Outcome:** one precedence implementation in the tree; identical
  file chosen for every single-variant fixture; the dual yml+yaml case
  resolves yaml-first with a doctor warning available
- **Validation:** parity test over fixture matrices (each ext
  present/absent) against old and new resolution, including the
  both-yml-and-yaml case asserting the new winner; doctor multi-variant
  warning test
- **Files:** `crates/anvil-cli/src/commands/{hook,l4_validate}.rs`,
  `crates/anvil-cli/src/commands/doctor.rs`
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** None

---

#### UCFG-010: MCP resources, config summary, doctor on the unified surface

- **Status:** Proposed
- **Intent:** `anvil://config`-class MCP resources, `config_summary`, and
  doctor render the resolved unified config (canonical name, gate section,
  resolved architecture, delegation provenance)
- **Expected Outcome:** agents see one config surface with provenance; doctor
  validates all delegation topologies
- **Validation:** MCP resource snapshot tests for inline + delegated;
  doctor topology matrix
- **Files:** `crates/anvil-cli/src/mcp/resources/`,
  `crates/anvil-cli/src/config_summary.rs`,
  `crates/anvil-cli/src/commands/doctor.rs`
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** UCFG-005, UCFG-007

---

#### UCFG-011: documentation sweep to one canonical name

- **Status:** Proposed
- **Intent:** config.md names exactly one canonical filename and the legacy
  fallback story; first-project, quickstart, agent-harness, using-anvil
  skill, and cli-surface runbook updated; gate-config "planning surface"
  concession removed once UCFG-005 lands
- **Expected Outcome:** no doc presents five filenames as co-equal; no doc
  references `gate-config.json` as a live surface
- **Validation:** `pnpm docs:check`; grep gates for retired paths in
  docs/public
- **Files:** `docs/public/anvil/operations/config.md`,
  `docs/public/anvil/{first-project,quickstart}.md`,
  `docs/public/anvil/guides/agent-harness.md`,
  `crates/anvil-cli/assets/skills/using-anvil/SKILL.md`,
  `docs/runbooks/cli-surface.md`,
  `crates/anvil-tui/src/surfaces/tutorial/{mod,paths}.rs` (interactive
  tutorial copy + `Verify::FileExists(".anvil/architecture.yaml")` checks)
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** UCFG-002, UCFG-005, UCFG-008

---

#### UCFG-012: fixture and CI sweep

- **Status:** Proposed
- **Intent:** convert test fixtures and CI steps referencing `.anvilrc`,
  `gate-config.json`, or direct `architecture.yaml` reads to the canonical
  surface (legacy-fallback fixtures kept deliberately and labelled)
- **Expected Outcome:** CI green on all platforms with canonical fixtures;
  remaining legacy references are intentional fallback coverage only
- **Validation:** full workspace test run; grep audit distinguishing
  intentional legacy fixtures from stragglers
- **Files:** `.github/workflows/`, `crates/*/tests/`, fixture directories
- **Confidence:** High
- **Priority:** Low
- **Dependencies:** UCFG-005, UCFG-008

---

## Parallel Execution

```
Phase 1: UCFG-003 → UCFG-001 → UCFG-002
Phase 2: UCFG-003 → UCFG-004 → UCFG-005
Phase 3: UCFG-003 → UCFG-006 → UCFG-007 → UCFG-008
Phase 4: UCFG-009 (anytime)
         UCFG-010 (after 005+007)
         UCFG-011 (after 002+005+008)
         UCFG-012 (after 005+008)
```

UCFG-003 is the root of every casing-bearing chain: the first canonical
`.anvil.<ext>` file init produces must never carry camelCase keys.
