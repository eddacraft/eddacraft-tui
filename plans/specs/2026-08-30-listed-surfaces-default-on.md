# Listed surfaces default-on

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for the listing/default-on product rule, the FLAGCAT listed-implies-on check, and the first hide slice | [FLAGCAT](../modules/feature-flag-catalogue.aps.md) / CLI | Accepted | 2026-08-30 — listed boolean + flag-derived on/off |

| Upstream | Downstream |
| -------- | ---------- |
| Operator design (2026-08-30); [ADR-001](../decisions/001-planless-first.md); [ADR-002](../decisions/002-warnings-over-blocks.md); [ADR-076](../decisions/076-feature-catalogue-surface-registry.md) §5 `requires`; [ADR-127](../decisions/127-always-on-cheap-catalogue.md); FLAGCAT-012/013/014 Merged; [IMPV-002](../archive/modules/tui-impact-view.aps.md); [GTAO](../archive/modules/gate-time-always-on.aps.md); `flags/surfaces.json`; `flags/manifest.json` | FLAGCAT-019 (listed field + listed-implies-on static check); next-free ADR; clap `hide` as first consumer; GTAO-004/006/007 stay GTAO-owned |

**Design approved 2026-08-30.** This specification does not authorise product code on its own. Execution follows APS items filed from it. The durable product rule is an ADR in the same change as the catalogue field.

## The problem

Anvil's advertised product is: run `anvil` / `anvil start`, then protection happens on save. A second, better product exists only if the user already knows an incantation (`anvil gate --profile ci`, `ANVIL_IMPACT=1`, `ANVIL_DASHBOARD_WEB=1`, `anvil gctx egress enable`, `anvil watch --action gate`).

`--help` lists surfaces whose controlling flags are default-off. FLAGCAT-013 already links flags to features. It does not yet forbid that combination.

## Promotion test

Default a behaviour when **either**:

1. **Claim test.** A green save / `anvil status` would be a lie without it.
2. **Expectation test.** A reasonable user would already expect it to be on.

Listing creates expectation. If a surface is listed, it works without a secret flag, env var, or specialist command.

Keep genuine contract opt-in, where the command already works and the flag *changes* the contract: `--fail-on-warnings`, `anvil gctx egress enable`, `--anvil-home` / `--touch-project-state`, full gate-on-save (ADR-127).

## Two bits, one illegal state

On each **delivery surface**:

- **`listed`** — boolean. New catalogue field. Default `true` for existing entries.
- **`on`** — not stored. Derived from FLAGCAT-013 `flagLinkage`: linked operational flag `defaultVariant` is enabled, or the feature is `unflagged`.

| Listed | On | Meaning |
| ------ | -- | ------- |
| true | true | Product. In `--help`, works. |
| false | false | Unfinished. Hidden, gated. First hide slice. |
| false | true | Alias / internal. Works if you know the name. |
| true | false | **Illegal.** Listed, then `ANVIL_*=1`. |

**Listed implies on.** Promoting a surface flips `listed` to true in the same change as the controlling flag becoming default-on. Do not list first.

Do not add a second operational flag for listing. Do not copy on/off onto the surface. One flag may still serve mixed listing (`anvil impact` unlisted, MCP `anvil_impact_of_change` listed).

The operator list is a query: `listed == false` and derived on == false. Generate it from the catalogue (extend FLAGCAT-014). Not a markdown index.

## This is a FLAGCAT dependency check

ADR-076 §5 already has feature-to-feature `requires` as declared data plus static existence/acyclicity. Runtime cascade-off stays deferred.

Listed-implies-on is a **cross-noun static check** on that same idea: a delivery surface's `listed: true` requires its product feature to be on (linked flag default-on, or unflagged). FLAGCAT-013 made the join keys; it did not constrain them.

That check is **FLAGCAT-019** (filed Draft in the FLAGCAT module), not a CLI-only edit and not a hand list:

1. `listed` on `DeliverySurfaceSchema` (default true).
2. Derive on/off from `flagLinkage` + `flags/manifest.json` `defaultVariant`.
3. CI fails `listed && !on`.
4. Visible clap `--help` matches `listed` (extends FLAGCAT-012's host projection; hidden commands remain in the completeness set).
5. Generated view of unlisted-and-off surfaces.
6. First consumers: mark `cli.impact`, `cli.dashboard-web`, `cli.plan-dashboard` `listed: false`; clap `hide` them (and `anvil plan` if it has no remaining visible child). Flags stay default-off. MCP `anvil_impact_of_change` stays listed.

Clap hide without the catalogue field would recreate a shadow list. The hide is the proof consumer of FLAGCAT-019, in the same change.

This supersedes IMPV-002 **for listing only**. `impact.view` remains default-off.

## Protection-claim track (not FLAGCAT-019)

GTAO remains the owner of claim-test work already filed: GTAO-004, GTAO-006, GTAO-007.

## Non-goals

- Ungating `impact`, `--web`, or `plan dashboard` in this work
- Runtime cascade-off (ADR-076 §5 still deferred)
- Deriving `CLI_GATED_COMMANDS` from the catalogue (FLAGCAT-009 still deferred)
- A new operational flag per surface for listing
- Defaulting `--fail-on-warnings` or full gate-on-save
- Hiding working terminal `anvil dashboard`
- A markdown hidden-commands index

## Execution shape

1. FLAGCAT-019 is Draft until the operator promotes. ADR in the same implementation change.
2. Schema + loader + `listed && !on` gate + generated view.
3. Mark the three CLI surfaces unlisted; clap hide; help-alignment tests.
4. Leave GTAO and runtime cascade alone.
