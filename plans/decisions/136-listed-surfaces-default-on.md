# ADR-136: Listed surfaces must be default-on

## Status

Accepted 2026-08-30 (operator)

## Date

2026-08-30

## Context

`--help`, welcome, status, MCP tool lists, and public docs create expectation.
Several CLI surfaces were listed while their controlling flags were default-off,
so the working product was an incantation (`ANVIL_IMPACT=1`,
`ANVIL_DASHBOARD_WEB=1`, `ANVIL_DEV=1`). FLAGCAT-013 already joins flags to
features. It did not forbid that combination.

Genuine contract opt-in (`--fail-on-warnings`, snippet egress, `--anvil-home`)
is different: those commands already work; the flag changes a contract.

## Decision

1. **Listing is a default.** A listed delivery surface must work without a
   secret flag. Help, welcome, status, MCP tool lists, and public docs are the
   list.

2. **`listed` lives on the delivery surface**, default `true`. On/off is derived
   from FLAGCAT-013 `flagLinkage` plus `flags/manifest.json` `defaultVariant`.
   Do not copy on/off onto the surface. Do not add a listing operational flag.

3. **CLI invocation is off** when the product feature is linked to a boolean
   flag whose default variant is `false` and every delivery of that feature is
   CLI. Unflagged features are on. Features that also ship on another host stay
   on (the flag is a contract on that host, not a clap refusal). Non-CLI hosts
   are out of this check.

4. **Hide until default-on.** Unfinished CLI surfaces stay in the binary and
   stay gated. Clap `hide = true`. Known names plus the existing env /
   `ANVIL_DEV=1` still invoke them. Promote by flipping `listed` to true in the
   same change as the controlling flag becoming default-on.

5. **CI fails `listed && !on` for CLI locators.** The operator list is the
   generated unlisted-and-off view, not a markdown index.

6. **IMPV-002 listing is superseded.** `impact.view` remains default-off.
   MCP `anvil_impact_of_change` stays listed (`gctx.impact-of-change` is a
   different, unflagged product feature).

## Rationale

Implementation cost is not a reason to stay listed-but-off. Mixed-host features
must not hide a working CLI because a sibling host is gated.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Chosen: surface `listed` + derived on/off | One new field; mixed CLI/MCP listing | Must unflag control surfaces such as `gctx-control` |
| `helpListing` enum | Labels pending vs alias | Three values for two bits |
| Listing operational flag | Familiar FLAGCAT object | Rolls out help copy; drifts from invocation |
| Ungate unfinished UI | Listing becomes true | Ships unfinished surfaces |

## Consequences

- **Positive:** `--help` stops advertising incantations. Unlisted-and-off is
  queryable from the catalogue.
- **Negative:** Hidden commands remain a footgun for anyone who memorised them.
- **Risks:** A new CLI-only default-off flag without `listed: false` fails CI.
- **Mitigations:** Load-time integrity in `@eddacraft/anvil-flags-catalogue`.

## References

- Related ADRs: ADR-001, ADR-002, ADR-076, ADR-127
- APS modules: FLAGCAT-019
- Spec: `plans/specs/2026-08-30-listed-surfaces-default-on.md`
