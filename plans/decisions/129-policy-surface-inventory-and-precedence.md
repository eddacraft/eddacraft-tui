# ADR-129: Policy Surface Inventory and Intra-Repo Precedence

## Status

Proposed

## Date

2026-08-23

## Context

A team that did not build Anvil cannot change policy behaviour without
discovering seven distinct modification surfaces. Three of those have a
public reference page; the rest appear in changelogs, ADRs, or code. Nothing
states which are supported product surfaces, and nothing states what wins
when two of them appear to disagree.

ADR-120 consolidated the **config file** surface (canonical `.anvil.<ext>`,
discover-first, snake_case) and explicitly left **policy merge semantics**
unowned — pointing at POLLC/ORGHIER, which remain dormant. That carve-out
covered two different questions:

1. **Intra-repo:** which of the shipped surfaces in *this* repository wins
   when they address the same behaviour.
2. **Org/pack overlay:** how organisational, federated, or lifecycle policy
   composes across repositories.

This record answers (1). Question (2) stays with POLFIT-009 / ORGHIER /
POLLC / POLFED.

ADR-098 AD-7 already named two of the surfaces — *acceptance policy*
(`anvil/policy.*`, ADR-037) versus *code policy* (regorus packs, ADR-040) —
and forbade collapsing L4 `on_block` into `enforcement.mode`. This record
keeps that split and places the other five surfaces relative to it.

The seven surfaces were read from shipped code on 2026-08-22
(`origin/main` @ `7524a599b`, binary 0.9.7-beta) as part of POLFIT-001.
This ADR pins that inventory; it does not implement new merge machinery.

## Decision

### D-1 — Inventory

These are the user-modifiable surfaces from which a user can change
policy behaviour or its recorded posture. Whether a given key is live
on every evaluator is stated per row and in D-4. Filename discovery
(ADR-120) is **not** an eighth surface: it is how several of these
files are *found*.

| # | Surface | Where | Question it answers | Engine |
| - | ------- | ----- | ------------------- | ------ |
| 1 | Rule modes | `.anvil.<ext>` `enforcement.rules.<rule>.mode` | Stored per-rule off/warn/enforce for four named kernel-invariant IDs. Writer/reader shipped (`anvil config set`/`show`); evaluators do not yet consult `RuleMode` (ADR-098 AD-3 deferred RuleMode normalisation) | `anvil-config` `RuleModes` |
| 2 | Code-policy packs | `.anvil/policies/**` (Rego + manifest) | Project rules evaluated as the `policy` gate check | `anvil-policy-engine` (regorus, ADR-040) |
| 3 | Architecture definition | `architecture` section **or** `.anvil/architecture.yaml` via `architecture.source` | Which layers may depend on which | import-boundaries check |
| 4 | Acceptance policy | `anvil/policy.*` | Whether a commit/push is accepted (witness / L4) | `anvil-l4` (ADR-037) |
| 5 | Intercept-rule registration | `.anvil.<ext>` `enforcement.intercept-rules` | Which save-time intercept rules run. Boolean keys register wrappers; `path-deny` / `regex-content` author glob/regex bodies | `anvil-intercept-rules` |
| 6 | Enforcement posture | `.anvil.<ext>` `enforcement.mode` | How strictly a block-worthy finding is acted on | kernel-types `EnforcementMode` (ADR-098 AD-3) |
| 7 | Anti-pattern registry resolution | shipped lookup chain (ADR-026 §1) | Which compiled catalogue the scanner loads. **Not** a supported user override until POLFIT-008 | `anvil-checks` `registry_loader` |

Related keys that are **not** extra inventory items:

- `enforcement.observe_only` and `enforcement.on_ambiguous_ownership` live
  in the same `enforcement` block as (6) and are daemon-only. They refine
  the intercept/enforcement surface; they are not a seventh-plus product
  surface.
- `antipattern.exclude` is a scanner-scope glob. It does not author a
  rule and is not a policy surface.
- `ANVIL_POLICY_ENFORCEMENT=off` is the ADR-098 AD-5 kill switch. It
  skips MCP pre-write pack evaluation (D-4.11) and is an out-of-band
  recovery control, not a config surface.
- Top-level `checks:` / `gate.checks` can omit `policy` or
  `import-boundaries` and so unload catalogues (2) and (3) at gate.
  That is check selection, not a policy surface.

### D-2 — Complementary layers, not a single merge stack

The seven surfaces do not form one overlay. Most apparent disagreements
are different questions. Merge only when two surfaces address the **same**
behaviour.

Three axes:

1. **Catalogue** — what exists to fire: (2) packs, (3) architecture
   definition, (5) intercept-rule registration, (7) anti-pattern registry.
2. **Posture** — how strictly a finding is acted on: (6) `enforcement.mode`.
   (1) is the *intended* per-rule posture for four kernel invariant IDs
   whose catalogue is architecture (3); it is stored, not yet evaluated
   (D-4.3).
3. **Lifecycle** — when it fires: intercept / MCP pre-write, `anvil check`,
   `anvil gate`, L4 push acceptance.

A pack finding and an architecture finding are both findings. Neither
suppresses the other. A gate check and an L4 rule can both refuse the
same commit for different reasons.

### D-3 — Classification

| Surface | Classification | Public home today |
| ------- | -------------- | ----------------- |
| (1) Rule modes | **Supported** writer/reader; evaluation unwired (D-4.3) | `docs/public/anvil/reference/config.md` |
| (2) Code-policy packs | **Supported** | `docs/public/anvil/concepts/policy-model.md` |
| (3) Architecture definition | **Supported** structural-constraint surface (not a regorus policy) | `docs/public/anvil/concepts/boundaries.md` |
| (4) Acceptance policy | **Supported** | ADR-037; changelog note only on the public site |
| (5) Intercept-rule registration | **Supported** (field catalogue is POLFIT-006) | none yet |
| (6) Enforcement posture | **Supported** (field catalogue is POLFIT-006) | none yet |
| (7) Registry resolution | **Internal** until POLFIT-008 classifies it. Not a supported override | ADR-026 §1 only |

None of the seven is deprecated. Legacy **filenames** (`.anvilrc`, a
standalone `.anvil/architecture.yaml` without `architecture.source`) are
ADR-120 config-file legacy, not deprecated policy surfaces.

### D-4 — Intra-repo overlap rules

These rules are the "policy merge semantics" ADR-120 left unowned, scoped
to one repository.

1. **Different questions do not merge.** Catalogue, posture, and lifecycle
   compose. A finding from one catalogue is not cancelled by silence in
   another.

2. **`enforcement.mode` is action-time posture, not a catalogue switch.**
   It projects a block-worthy finding onto `ControlDecision` (off → allow,
   warn → warn, fence → fence, interrupt → interrupt). It does not unload
   packs, architecture, or the anti-pattern registry. It does not rewrite
   L4 `on_block` / `on_warn` (ADR-098 AD-7).

3. **`enforcement.rules` is a closed four-ID table, not a live
   evaluator control.** The four IDs
   (`public-api-expansion`, `new-dependency-introduction`,
   `cross-layer-violation`, `privilege-expansion`) are the kernel
   invariant IDs whose catalogue is architecture (3). The intended
   overlap is catalogue (3) plus per-rule posture (1) on those findings.
   **Shipped today:** `anvil config set`/`show` persist and echo the
   modes; kernel invariant evaluators do not read `RuleMode`. `off`
   does not suppress those findings; `enforce` does not raise global
   `enforcement.mode`. Treating this table as a working override is
   the unwired gap ADR-098 AD-3 deferred, not a decided merge.
   Hard-pinned classes `secrets` and `command-safety` (ADR-039) cannot
   be set to `off` in the same map; unknown keys are ignored by
   `RuleModes::from_value`.

4. **Daemon project vs user posture is stricter-wins, and it is not
   intra-repo.** `EnforcementMode::stricter` (`off < warn < fence <
   interrupt`) applies when the intercept daemon merges project
   `.anvil.yaml` with user-level config. MCP pre-write does not merge
   user config. This is extra-repo daemon behaviour, not the ADR-120
   intra-repo carve-out, and not org overlay (D-5).

5. **Absent `enforcement.mode` is a same-key split, not a different
   question.** The shared type defaults to `Warn` (ADR-002). When the
   key is missing or unreadable, MCP pre-write falls back to
   `Interrupt` (historical veto-on-error) and the intercept daemon
   falls back to `Warn`. A repo with no `enforcement.mode` therefore
   vetoes MCP writes and warns on daemon save-time. This ADR records
   that shipped split as a decided exception rather than pretending
   the surfaces do not disagree. The MCP Interrupt fallback is
   fail-closed write-safety and must not be merged down to Warn.
   Picking one absent-key winner is a follow-up (UCFG / POLFIT-006);
   it is not decided here.

6. **Architecture is exclusive, not merged.** Inline `architecture` XOR
   `architecture.source` (ADR-120). A standalone
   `.anvil/architecture.yaml` remains a legacy fallback until migrate
   writes the `source` line. Two architecture documents are not overlaid.

7. **Filename discovery is winner-takes-all.** ADR-120 discover-first,
   yaml-first. Multiple `.anvil.<ext>` or `anvil/policy.<ext>` variants:
   one winner, `anvil doctor` warns. That is lookup, not merge.

8. **Registry lookup is Internal implementation, not a supported
   override.** Today's first-found chain (explicit path,
   `ANVIL_REGISTRY_PATH`, cwd walk, executable walk, embedded
   catalogue; set-but-missing is `OverrideMissing`) is ADR-026 §1
   behaviour: unsigned, no hash or signature, a found path silently
   replaces the embedded catalogue. Stricter-wins (D-4.4) does **not**
   apply to catalogue lookup. This ADR describes the loader; it does
   **not** freeze it as a product merge rule. POLFIT-008 decides
   whether it becomes a stated surface with a trust boundary or a
   closed one; closing it will amend this record.

9. **Intercept wrappers do not replace `anvil check` / `anvil gate`.**
   `secret-detection` defaults on; `antipattern` defaults off.
   `path-deny` / `regex-content` author additional save-time rule
   bodies and are not a second pack surface. Gate and check keep their
   own invocations. Duplicate secret-detection hits across save-time
   and check are two lifecycles, not a precedence bug.

10. **Packs are one logical surface with one admission contract.**
    `.anvil/policies/` is the supported authoring *target* (POLFIT-002's
    falsifier). Gate and MCP pre-write must admit the same pack the same
    way; today's divergence is a defect owned by POLFIT-004, not a
    decided split.

11. **The pack-evaluation kill switch skips pack eval on AD-5
    surfaces, not project config as a whole.**
    `ANVIL_POLICY_ENFORCEMENT=off` (or `0`) skips MCP pre-write pack
    evaluation so a broken pack cannot trap recovery inside the
    interrupt path (ADR-098 AD-5). It is not a posture setter, is
    not read by `anvil gate` or L4, and does not disable intercept,
    architecture, registry, rule modes, or the MCP Interrupt fallback
    for non-pack diagnostics. It is a process-environment recovery
    control, not a project-config key.

### D-5 — Out of scope (deliberate)

- Organisational / federated / lifecycle overlay (ORGHIER, POLLC, POLFED).
  POLFIT-009 makes their dormant posture honest; this ADR does not invent
  a merge function for modules that do not ship.
- Authoring on-ramp (ACTAX vs OPAE-013..017 vs pack scaffolding) —
  POLFIT-002.
- Whether the registry override becomes a public surface or a closed
  one — POLFIT-008.
- Field-catalogue entries for `enforcement.mode` and
  `enforcement.intercept-rules` — POLFIT-006 / DOCDEF-007.
- The public pointer at the unshipped `authoring-anvil-policy` skill —
  POLFIT-003.
- Ratifying reader drift as a product contract. MCP pre-write **and**
  the intercept daemon currently probe only `.anvil.yaml` for
  `enforcement.mode`, while intercept-rules registration uses ADR-120
  `discover`. That is implementation debt on surface (6), not an
  eighth surface and not the D-4.7 winner-takes-all rule.
- Unifying the MCP Interrupt vs daemon Warn absent-key split (D-4.5)
  onto one fallback. Recording the split is in scope; picking a
  single winner is not.

### D-6 — Public inventory home

`docs/public/anvil/concepts/policy-model.md` is the one public page that
enumerates the seven surfaces and states that they are complementary
layers. The Internal registry row names the surface without publishing
the lookup recipe. Per-key catalogue stays `reference/config.md`.
Architecture definition stays `concepts/boundaries.md`. Pack commands
stay `reference/policy.md`. This ADR is the precedence authority.

## Rationale

Pinning shipped behaviour is cheaper and more honest than inventing an
org-style overlay the product does not have. The adoption failure is
discoverability and false conflicts, not missing merge code. Recording
the three axes (catalogue / posture / lifecycle) stops agents and docs
from treating `enforcement.mode: off` as "policy is disabled" or treating
L4 `on_block: reject` as an enforcement-mode synonym.

Classifying the registry chain as Internal, and keeping the lookup
recipe out of the public page, avoids documenting an override that
POLFIT-008 may close. Classifying architecture as a supported
structural-constraint surface, while saying it is not a regorus policy,
matches the public `boundaries.md` distinction already shipped.
Calling rule modes Supported-as-writer without claiming they steer
kernel evaluation keeps the inventory honest about a shipped knob
that is not yet an evaluator control.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Complementary layers + overlap rules (chosen) | Matches shipped code; unblocks POLFIT-003..006 and -008 without pretending org merge exists | Adopters must learn three axes, not one stack |
| Single numbered precedence stack (1 beats 2 beats …) | Simple slogan | Almost every pair is orthogonal; a stack would be a fiction and would mis-order L4 vs packs vs intercept |
| Treat only packs as "policy"; omit the other six | Smaller page | The audit's adoption failure is that users *will* find the other six; omitting them recreates the problem |
| Invent org/pack overlay now | Future-proof | No shipped ORGHIER/POLLC; would decide a dormant programme in the wrong ADR |
| Classify registry override as supported today | Documents what the loader already does | POLFIT-008 exists specifically to choose supported vs closed; promoting it here pre-empts that item |

## Consequences

- **Positive:** one inventory and one intra-repo overlap contract; POLFIT
  follow-ups have a gate they can cite; public docs can name every surface
  without claiming a merge stack; ADR-120's carve-out is split into
  intra-repo (this ADR) and org/pack (still dormant).
- **Negative:** the public concept page grows beyond packs; L4 remains
  thinly documented on the public site (changelog + this inventory, not a
  tutorial); Internal classification of (7) will look like a gap until
  POLFIT-008 lands.
- **Risks:** readers treat D-4 as implemented merge code and expect
  org overlay; MCP's `.anvil.yaml`-only probe is mistaken for the
  decided discovery rule; POLFIT-002 proceeds on the pack surface and
  later has to re-run if that surface is unshipped (the recorded
  falsifier).
- **Mitigations:** D-5 names the dormant overlay; D-5 names the MCP
  probe as debt, not contract; POLFIT-002 already records the pack-surface
  falsifier.

## References

- Related ADRs: ADR-002 (warnings-first default), ADR-026 §1 (registry
  lookup chain), ADR-037 (L4 acceptance policy), ADR-040 (regorus packs),
  ADR-073 / ADR-100 (committed authority vs local config), ADR-098
  (two-axis posture; AD-7 acceptance vs code policy; AD-5 kill switch),
  ADR-120 (config file consolidation; this ADR picks up the intra-repo
  half of its policy-merge carve-out)
- APS modules: POLFIT-001 (this record), POLFIT-002 (authoring on-ramp;
  pack surface remains the supported authoring target), POLFIT-003..006
  and POLFIT-008 (Draft, unblocked to start once this lands), POLFIT-009
  (org-module posture), OPAE, ARCHCFG, DOCDEF, INSEC
- Evidence: POLFIT audit against `origin/main` @ `7524a599b` (0.9.7-beta);
  `crates/anvil-config/src/rule_modes.rs`;
  `crates/anvil-l4/src/policy.rs`;
  `crates/anvil-intercept-rules/src/config.rs`;
  `crates/anvil-kernel-types/src/enforcement.rs`;
  `crates/anvil-cli/src/mcp/enforcement.rs`;
  `crates/anvil-checks/src/antipattern/registry_loader.rs`
