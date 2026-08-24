# Product catalogue host-completeness contract

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for FLAGCAT-012 host projections and CI gate | [FLAGCAT](../modules/feature-flag-catalogue.aps.md) | Accepted | 2026-08-23 — derived from ADR-076, the accepted v2 schema, and live host registries |

| Upstream | Downstream |
| -------- | ---------- |
| [ADR-076](../decisions/076-feature-catalogue-surface-registry.md); [product catalogue v2](2026-08-23-product-catalogue-v2-schema.md); live host registries | FLAGCAT-012 implementation and CI gate |

**Execution authority** is FLAGCAT-012. This specification authorises
behaviour-preserving host projections and completeness validation only. It does
not authorise catalogue-derived runtime enforcement, authentication changes,
operational-flag linkage, commercial-plan mapping, or generated product views.

## Required outcome

Every active shipping host exposes a deterministic projection of the delivery
identities it owns. CI compares that projection with the canonical v2 catalogue
by typed locator, not by display name or a minimum-count assertion.

All nine current locator kinds participate:

- CLI command paths;
- MCP tools;
- MCP resources;
- API method and path pairs;
- daemon RPC methods;
- dashboard routes;
- documentation route prefixes;
- git hook names;
- integration client and capability pairs.

A host projection has two separately compared sets:

1. **Product deliveries** must equal the host's active
   `deliverySurfaces[]` locators.
2. **Internal plumbing** must equal the host's active
   `excludedDeliverySurfaces[]` locators.

Comparing only the union is invalid because it would allow a user-visible
delivery to move into the exclusion collection without failing. Retired
identities remain reserved catalogue history and are not part of the current
shipping projection.

## Host authorities

| Host | Runtime authority | Projection rule |
| ---- | ----------------- | --------------- |
| CLI | Clap's `Cli::command()` tree and canonical command classification | Project independently packageable command paths, including supported hidden compatibility aliases; keep namespace-only nodes collapsed and classify `graph-base` as internal |
| MCP tools | `mcp::tools::registry::all()` | Project every registered tool name |
| MCP resources | `mcp::resources::list()` | Project every advertised resource URI |
| API | composed Hono application's registered routes | Project concrete non-middleware method/path pairs after the `/api/v1` base path is applied |
| daemon | canonical accepted-method registry used by IPC dispatch | Project canonical methods; accepted legacy spellings are aliases, not new delivery identities |
| dashboard | TanStack route tree plus dashboard-server OpenAPI/runtime paths | Project the eight browser routes and seven loopback server routes as disjoint owned subsets |
| docs | Next app routes plus proxy matcher | Project independently usable app/proxy prefixes; framework and asset transport stays in the internal set |
| hooks | the installed `HookKind` registry | Project every installed hook filename |
| integrations | `AgentClientId::all()` registry capabilities | Project every supported client/capability pair |

The CLI projection deliberately does not equate every nested help leaf with a
product feature. ADR-076 defines a product feature as the smallest unit anvil
would ship or gate independently; nested operations that share one package and
posture remain collapsed. The host-owned projection must still prove that every
projected command path exists in Clap, so a stale locator cannot pass.

The daemon projection uses the spelling emitted by the current producer where
one exists, and otherwise the host's namespaced protocol constant. IPC
compatibility aliases remain accepted by the host but do not multiply product
identities. Adding a canonical accepted method requires an explicit
product-delivery or internal-plumbing decision.

## Comparison and diagnostics

Each comparison normalises its typed locator into a stable tuple:

- CLI: command segments joined with a NUL-safe structural encoding;
- MCP tool/resource: name or URI;
- HTTP: upper-case method plus exact path;
- daemon: exact canonical method;
- dashboard/docs/hook: exact path, prefix, or filename;
- integration: client identifier plus capability.

Tests fail on all of these conditions:

- a shipping locator is absent from both catalogue collections;
- a catalogue locator is absent from the shipping host;
- a locator appears in both collections;
- a user-visible projection is present only as an exclusion;
- an internal projection is present only as a product delivery;
- duplicate host or catalogue locators collapse during normalisation;
- an active exclusion lacks the schema-required owner, concrete reason,
  internal-plumbing classification, and review reference.

Failure output reports host, set, and missing/extra tuples. A changed count alone
is never accepted as completeness evidence.

## CI contract

A change to `flags/surfaces.json` must run both the TypeScript and Rust host
projection suites. A change to a host registry must run that host's projection
test even when the catalogue file is untouched.

Nx inputs and Rust path filters are targeted to the participating projects;
`flags/**` must not become a global workspace input that makes every project
affected. The required hosted checks reuse the existing Node unit-test and Rust
test conclusions.

The executor validation surface is:

```text
pnpm exec nx test flags-catalogue --skip-nx-cache
pnpm exec nx test @eddacraft/anvil-api --skip-nx-cache
pnpm exec nx test dashboard --skip-nx-cache
pnpm exec nx test docs-shell --skip-nx-cache
cargo test -p eddacraft-anvil --no-fail-fast
cargo test -p eddacraft-anvil-dashboard-server
cargo test -p eddacraft-anvil-hook
cargo test -p eddacraft-anvil-intercept --lib
pnpm test:ci-classify
pnpm validate:changed
```

## Rollback and non-goals

The gate is additive and changes no shipped request, command, flag-resolution,
authentication, or authorisation path. Before any dependent FLAGCAT consumer
lands, rollback is an atomic revert of the projection tests, registry metadata,
and targeted CI wiring. After a dependent consumer adopts the contract,
recovery is repair-forward so catalogue drift cannot be hidden by removing the
gate.

FLAGCAT-012 does not:

- change access posture or host enforcement;
- generate a second catalogue or checked-in projection manifest;
- infer product features from prose, route filenames, or display names;
- treat protocol-only constants or compatibility aliases as distinct product
  deliveries;
- perform FLAGCAT-013 operational-flag linkage or FLAGCAT-014 documentation
  generation.
