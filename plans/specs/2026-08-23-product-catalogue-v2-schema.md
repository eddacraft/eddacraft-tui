# Product catalogue v2 physical schema

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for FLAGCAT-011 physical design | [FLAGCAT](../modules/feature-flag-catalogue.aps.md) | Accepted | 2026-08-25 — FLAGCAT-015 adds `planAvailability` on product features; FLAGCAT-011 physical nouns unchanged |

| Upstream | Downstream |
| -------- | ---------- |
| [ADR-076](../decisions/076-feature-catalogue-surface-registry.md); [ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md); `flags/surfaces.json`; `packages/anvil/contracts/src/schemas/feature-flags.schema.ts`; `packages/anvil/flags-catalogue/src/manifest.ts` | FLAGCAT-011 implementation; FLAGCAT-012 host completeness gates; FLAGCAT-013 flag linkage; FLAGCAT-014 generated views; [FLAGCAT-015 plan availability](2026-08-25-product-catalogue-plan-availability.md) |

**Execution authority** remains FLAGCAT-011. This specification records the
operator-approved physical design delegated by ADR-076. It does not authorise
runtime availability enforcement, product-plan mapping, or the dependent
FLAGCAT-012..015 work.

## Approved outcome

Replace the v1 registry's conflated `categories[]` and `surfaces[]` model with
a normalised v2 catalogue containing four independently validated noun
collections plus a delivery-identity migration ledger:

- `productFeatureGroups[]` — stable customer-value or capability families;
- `productFeatures[]` — the smallest independently packageable or gateable
  capabilities;
- `deliverySurfaces[]` — separately identified host entry points that deliver
  those features;
- `excludedDeliverySurfaces[]` — reviewed internal plumbing only.
- `deliverySurfaceMigrations[]` — explicit retired-source to active-target
  mappings for delivery splits and merges.

The dedicated `PRODUCT_CATALOGUE_SCHEMA_VERSION` is `2`. It is independent
of `FEATURE_FLAG_SCHEMA_VERSION`; migrating the product catalogue must not
force unrelated operational-flag inventories to change version.

All v2 objects are strict. Unknown fields, unresolved references, duplicate
keys, invalid defaults, invalid posture, and cyclic hard dependencies fail
validation.

## Physical shape

The canonical document remains `flags/surfaces.json` under ADR-076 despite its
legacy filename:

```jsonc
{
  "schemaVersion": 2,
  "deliverySurfaceMigrations": [],
  "productFeatureGroups": [
    {
      "key": "governance",
      "name": "Governance engine",
      "defaultSurfacePosture": {
        "access": "licence"
      },
      "status": "active"
    }
  ],
  "productFeatures": [
    {
      "key": "check",
      "name": "Project checks",
      "groupKey": "governance",
      "owner": "RCLI2",
      "status": "active",
      "requires": [],
      "planAvailability": {
        "plan-free": "undecided",
        "plan-beta": "undecided",
        "plan-pro": "undecided",
        "plan-enterprise": "undecided"
      }
    }
  ],
  "deliverySurfaces": [
    {
      "key": "cli.check",
      "featureKey": "check",
      "locator": {
        "kind": "cli",
        "commandPath": ["check"]
      },
      "posture": {
        "invocation": "user",
        "mustAlwaysBeOpen": false
      },
      "status": "active"
    }
  ],
  "excludedDeliverySurfaces": []
}
```

Product-feature-group defaults carry only delivery-posture defaults. Product
features own lifecycle, ownership, and hard `requires` edges. Delivery
surfaces own host identity, locator, access/audience posture, invocation
context, and recovery-floor markers.

The current catalogue has no retired split or merge sources, so
`deliverySurfaceMigrations` is empty. Future entries use the strict shape
`{ "fromKeys": ["cli.old"], "toKeys": ["cli.new"] }`.

The v2 lifecycle vocabulary is deliberately `active | retired`. Expansion is
additive only after demonstrated product need.

## Delivery identities and locators

A delivery-surface key is immutable and independent of its mutable locator. A
command, route, or display name may change without renaming the delivery
identity.

Keys use a host-prefixed namespace:

- `cli.<identity>`;
- `mcp-tool.<identity>` and `mcp-resource.<identity>`;
- `api.<identity>`;
- `daemon.<identity>`;
- `dashboard.<identity>`;
- `docs.<identity>`;
- `hook.<identity>`;
- `integration.<identity>`.

Locators are a strict discriminated union:

- `{ kind: "cli", commandPath: string[] }`;
- `{ kind: "mcp-tool", name: string }`;
- `{ kind: "mcp-resource", uri: string }`;
- `{ kind: "api-route", method: string, path: string }`;
- `{ kind: "daemon-rpc", method: string }`;
- `{ kind: "dashboard-route", path: string }`;
- `{ kind: "docs-route", pathPrefix: string }`;
- `{ kind: "hook", hook: string }`;
- `{ kind: "integration", integrationId: string, capability: string }`.

The locator is operational discovery data, not a historical join key. Splits or
merges create new delivery keys and retire the old entries.

## Stable-key and ownership rules

- Existing v1 `categories[].id` values become product-feature-group keys
  unchanged.
- All 46 existing v1 `surfaces[].key` values become product-feature keys
  unchanged, including historically inconsistent forms.
- A product feature moving groups retains its key.
- Display-name, owner, group, locator, and documentation changes retain keys.
- Retired group, feature, and delivery keys remain reserved forever.
- A split or merge creates new keys and records an explicit migration from each
  retired key to its replacement key or keys.
- Migration sides are non-empty and contain unique keys. Sources must exist and
  be `retired`; targets must exist and be `active`. Every retired delivery
  key appears exactly once as a source, and a source key cannot be reused across
  migrations.
- Operational flag keys remain unchanged under ADR-041.
- Owners use existing APS module identifiers. Ownership is curated from current
  repository authority; migration must not guess silently.

Hard `requires` edges move to product features and target product-feature keys.
Existence and acyclicity remain static catalogue checks. Runtime cascade-off
remains out of scope.

## Posture, exclusions, and recovery floor

Delivery posture retains the current `open | licence | admin-key | staff`
access vocabulary, optional audience references, `user | system` invocation,
and `mustAlwaysBeOpen` marker. Effective posture resolves deterministically
from the group default plus a delivery-surface override.

The recovery floor remains independently pinned by exact delivery identity in
tests. It covers CLI credential bootstrap and canonical login/refresh paths,
usable public API login issuance and refresh routes, and the documentation
shell's login/callback routes. A `mustAlwaysBeOpen` delivery surface must
resolve to `open`. System-invoked surfaces and recovery-critical surfaces are
exceptions only to catalogue-derived availability refusal; they never bypass
host-owned authentication, authorisation, credential validation, input
integrity, or issuance checks.

An exclusion is valid only when all of these are present:

- a stable host-prefixed delivery key and typed locator;
- an APS-module owner;
- `classification: "internal-plumbing"`;
- a concrete reason;
- a review reference;
- `status: "active" | "retired"`.

User-visible surfaces are invalid exclusions. The v1 `catalogued: false`
escape hatch does not survive into v2.

## Migration and compatibility

Migration is behaviour-preserving and proceeds in two phases:

1. Add a frozen v1 schema, the strict v2 schema, and a pure v1-to-v2 normaliser.
   The normaliser requires a complete curated map from every v1 feature key to
   its APS owner, immutable delivery key, and typed locator; it fails on missing
   or extra mapping entries and never infers ownership or command paths from
   display names. Prove parity for all 46 legacy feature keys, nine groups,
   effective access postures, audience references, invocation markers,
   recovery-floor entries, and hard-dependency edges.
2. Convert the canonical `flags/surfaces.json` to v2 and back-capture current
   CLI, MCP, API, daemon, dashboard, documentation, hook, and integration
   delivery identities at the smallest independently packageable or gateable
   granularity.

Readers accept v1 and v2 through the first product release that contains v2 and
until all known in-repository consumers prove v2 adoption, whichever is later.
Canonical data and any writer emit v2 only.

`productCatalogue()` is the authoritative v2 accessor.
`flagSurfaces()` remains as a deprecated, deterministic, read-only v1
projection of the legacy CLI subset for the compatibility window. The
projection is explicitly incomplete and must never drive completeness,
entitlement, or runtime enforcement.
The frozen v1 fixture is authoritative for the exact payload returned by that
deprecated accessor during the compatibility window. It is not authoritative
for v2 product, completeness, or enforcement truth; `flags/surfaces.json`
through `productCatalogue()` remains canonical for those concerns.

## Rollback

Before any FLAGCAT-012 or later consumer adopts v2 identities, rollback is an
atomic revert of the schema/loader and canonical JSON to v1. The frozen v1
fixture is parity evidence and the exact compatibility-payload authority for
`flagSurfaces()`; it is not canonical v2 catalogue truth.

After downstream work publishes or consumes v2 delivery identities, the
catalogue does not downgrade. Recovery is repair-forward while the deprecated
v1 projection continues serving known compatibility consumers. Downgrading
would discard non-CLI and one-to-many information.

Rollback evidence must prove the exact 46-key legacy projection, effective
posture, dependency graph, audience references, invocation markers, and
independently pinned recovery floor.

## Non-goals

FLAGCAT-011 does not:

- derive host enforcement or completeness gates;
- link operational flags to product features;
- generate comprehensive prose views;
- map features to commercial product plans;
- add runtime cascade-off, environment plumbing, staff/RBAC resolution, or
  build-time editions;
- change operational flag evaluation or any shipped feature's availability.

Those boundaries remain with FLAGCAT-012..015 and the separate runtime owners
identified by ADR-076.
