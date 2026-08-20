# DOCRB corpus inventory and disposition

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for DOCRB-002 inventory | [DOCRB](../modules/docs-rebaseline.aps.md) | Ready | 2026-08-17 — source-pinned to `4588f1be8` (DOCRB-001 tip) |

| Upstream | Downstream |
| -------- | ---------- |
| ADR-123; `plans/specs/2026-08-16-docs-rebaseline.md`; `Cargo.toml` workspace members; `apps/*/package.json`; `packages/**/package.json`; `docs/architecture/**`; `docs/public/**` | DOCRB-003..008 migration, pilots, and diagram rebuilds |

This artefact is the DOCRB-002 ownership map. It does not migrate files,
change sibling-module status, or activate mandatory checks.

## 1. Method

Re-ran the design assessment against `4588f1be8` (the DOCRB-001 stack tip,
itself on `origin/main` `7a2ecf28f`).

- **Crate roots:** unique `Cargo.toml` workspace members under `crates/` (36).
  `crates/anvil-plan-read-model` exists on disk with its own `Cargo.toml` but
  is **not** a workspace member.
- **App roots:** `apps/*` directories with `package.json` (9).
- **Package roots:** `packages/**` directories with `package.json` (19), plus
  grouping directories `packages/anvil`, `packages/libs`, `packages/shared`,
  `packages/tooling`.
- **Central architecture:** every `.md` and `.drawio` under
  `docs/architecture/`.
- **Public page families:** top-level directories under `docs/public/` (91
  Markdown pages; matches the 2026-08-16 count).
- **Diagrams:** every `.drawio` in-tree and every `docs/**` file containing a
  ` ```mermaid ` fence.

Classification vocabulary (ADR-123):

| Class | Meaning |
| ----- | ------- |
| `component-doc required` | Needs a local `README.md` and, if internals warrant it, `ARCHITECTURE.md` |
| `README only` | Orientation is enough; no local architecture doc |
| `central cross-system authority` | Stays in `docs/architecture/**` |
| `generated/vendor` | Do not hand-author; regenerate or leave vendor-owned |
| `grouping-only` | Directory is a folder, not a documentation unit |
| `historical` | Archive or superseded; do not treat as live |
| `explicit exemption` | Out of the documentation unit on purpose |

Diagram dispositions: `retain`, `move`, `redraw`, `merge`, `retire`.

## 2. Counts at `4588f1be8`

| Unit | Count | Local README | Local ARCHITECTURE.md |
| ---- | ----- | ------------ | --------------------- |
| Workspace crates | 36 | 11 | 0 |
| Non-member crate dir | 1 (`anvil-plan-read-model`) | 0 | 0 |
| Apps | 9 | 5 | 0 |
| Package roots | 19 | 17 | 0 |
| Grouping dirs | 4 | 3 (`anvil`, `shared`, `tooling`) | 0 |
| Central architecture files | 35 (33 Markdown, 2 Draw.io) | n/a | n/a |
| Public page families | 6 (91 pages) | n/a | n/a |
| In-tree Draw.io | 2, both without sibling SVG | n/a | n/a |
| Live Mermaid in `docs/` | 2 (`overview.md`, `quality-model.md`); plus 2 non-authority (`docs/guides/adapters/workflow-guide.md`, archive) | n/a | n/a |

The 2026-08-16 design pinned 37 crate roots / 58 tracked roots / 27 with a
root README or ARCHITECTURE. This re-run finds 36 workspace crates + 1
orphan crate dir, 9 apps, 19 packages, 4 grouping dirs. Co-located
`ARCHITECTURE.md` is still absent everywhere.

## 3. Component roots

### 3.1 Crates

| Root | Class | Owner | Authoritative concern | Target | Required diagram | Disposition |
| ---- | ----- | ----- | --------------------- | ------ | ---------------- | ----------- |
| `crates/anvil-cli` | component-doc required | CLI | Command dispatch, activation, MCP shim, TUI runner | Local README + ARCHITECTURE; keep `cli-tui-runner-as-built.md` / `mcp-shim-as-built.md` as links until merge | Request flow CLI → kernel / MCP | move central as-built detail here |
| `crates/anvil-run` | README only | CLI | Process launcher / air-gapped run | Local README | none | README only |
| `crates/anvil-config` | component-doc required | UCFG | Config discovery, formats, delegation | Local README + ARCHITECTURE | Discovery/precedence | component-doc required |
| `crates/anvil-hook` | README only | HOOK | Hook coexistence | Local README | none unless lifecycle grows | README only |
| `crates/anvil-baseline` | README only | SCAN | Baseline persistence | Local README | none | README only |
| `crates/anvil-capsule` | README only | CAPS | Review-capsule format | Local README; link ADR-074 | none | README only |
| `crates/anvil-sarif` | README only | SCAN | SARIF emitter | Local README | none | README only |
| `crates/anvil-kernel` | component-doc required | KERN | Parse, graph, watch, embedded scan | Local README + ARCHITECTURE | Source → graph → finding | **pilot** |
| `crates/anvil-kernel-types` | README only | KERN | Shared kernel types | Keep existing README | none | README only |
| `crates/anvil-graph-cache` | component-doc required | GV2 | Hot/cold graph cache | Local README + ARCHITECTURE | Hot-read vs persist | component-doc required |
| `crates/anvil-gctx-types` | README only | GCTX | Graph-context types | Local README | none | README only |
| `crates/anvil-gctx-egress` | README only | GCTX | Egress / slice | Local README | trust boundary (link central) | README only |
| `crates/anvil-architecture` | README only | ARCH | Architecture YAML check | Keep existing README | none | README only |
| `crates/anvil-grammar-wat` | explicit exemption | LTW2 | Vendored WAT grammar | crate README optional | none | explicit exemption (vendor/generated parser) |
| `crates/anvil-plan-read-model` | README only | DASH | APS read model (not a workspace member) | Local README; record workspace-membership separately | none | README only; not a docs-unit until it is a member |
| `crates/anvil-checks` | component-doc required | SCAN | Check registry and families | Local README + ARCHITECTURE; slim `checks-as-built.md` to cross-system | registry → finding | move |
| `crates/anvil-checks-ast` | README only | SCAN | AST-aware checks | Local README | none | README only |
| `crates/anvil-checks-napi` | README only | SCAN | NAPI binding | Keep existing README | none | README only |
| `crates/anvil-rules` | README only | POL | Rule packaging | Local README | none | README only |
| `crates/anvil-policy` | README only | POL | Policy facade / exceptions | Keep existing README | none | README only |
| `crates/anvil-policy-engine` | component-doc required | POLENG | regorus engine | Local README + ARCHITECTURE | eval path | component-doc required |
| `crates/anvil-l4` | README only | L4 | L4 policy projection | Local README | none | README only |
| `crates/anvil-witness` | README only | WIT | Witness chain | Local README | none | README only |
| `crates/anvil-attribution` | README only | ATTRIB | Authorship attribution | Local README | none | README only |
| `crates/anvil-observability` | README only | OBS | Local tracing primitives | Local README | none | README only |
| `crates/anvil-intercept` | component-doc required | INTD | Save-time daemon | Local README + ARCHITECTURE | save → validate → fence | **pilot** |
| `crates/anvil-intercept-proto` | README only | INTD | Wire protocol types | Local README | none | README only |
| `crates/anvil-intercept-rules` | README only | INTD | Intercept rules | Local README | none | README only |
| `crates/anvil-intercept-macos` | README only | INTD | macOS adapter | Local README | none | README only |
| `crates/anvil-intercept-win32` | README only | INTD | Windows adapter | Local README | none | README only |
| `crates/anvil-rayon-init` | explicit exemption | INTD | Thread-pool init helper | none required | none | explicit exemption |
| `crates/anvil-tui` | component-doc required | TUI | Anvil TUI surfaces | Local README + ARCHITECTURE | surface/widget boundary | move from `tui-as-built.md` |
| `crates/eddacraft-tui` | component-doc required | TUIR | Shared Ratatui library | Keep README; add ARCHITECTURE if internals stay here | widget composition | retain README; ARCHITECTURE if needed |
| `crates/anvil-dashboard-server` | component-doc required | DASH | Loopback dashboard API | Local README + ARCHITECTURE | capability/auth boundary | **pilot** (with `apps/dashboard`) |
| `crates/anvil-bench` | README only | BENCH | Benchmark harness | Keep README | none | README only |
| `crates/spike` | explicit exemption | — | Spike experiments | existing README is enough | none | explicit exemption |
| `crates/workspace-hack` | generated/vendor | build | cargo-hakari hack | none | none | generated/vendor |

### 3.2 Apps

| Root | Class | Owner | Authoritative concern | Target | Required diagram | Disposition |
| ---- | ----- | ----- | --------------------- | ------ | ---------------- | ----------- |
| `apps/anvil-api` | component-doc required | API | Hosted API, auth, persistence | Local README + ARCHITECTURE | identity/data/trust | **pilot** |
| `apps/admin-cli` | README only | API | Operator CLI | Local README | none | README only |
| `apps/dashboard` | component-doc required | DASH | Local dashboard host | Local README + ARCHITECTURE | surface ↔ server | **pilot** |
| `apps/website` | README only | WEB | Marketing site | Keep README | none (not product docs) | README only |
| `apps/docs-shell` | component-doc required | DOCRB / DSITE gap | Production entrypoint and auth proxy | Local README + ARCHITECTURE | shell → private/public | **pilot** |
| `apps/anvil-docs-private` | README only | DOCRB | Gated Docusaurus renderer | Local README | none (renderer, not content) | README only |
| `apps/docs-public` | README only | DOCRB / DOCSYNC | Public Docusaurus renderer | Local README | none (renderer, not content) | README only |
| `apps/docs-public-astro` | historical | DOCSYNC | Alternate public renderer | Keep README; mark historical | none | historical / compatibility |
| `apps/docs-site` | historical | DSITE | Legacy combined host (rollback) | Keep README/AGENTS; do not restore as live authority | none | historical; DSITE status unchanged |
| `apps/e2e` | README only | TEST | E2E harness | Keep README | none | README only |

### 3.3 Packages

| Root | Class | Owner | Authoritative concern | Target | Required diagram | Disposition |
| ---- | ----- | ----- | --------------------- | ------ | ---------------- | ----------- |
| `packages/anvil` | grouping-only | — | Group folder | group README already present | Rust-first vs TS map is central | grouping-only |
| `packages/anvil/core` | README only | TSCOMPAT | Legacy TS domain | Keep README; retiring surface | none | README only |
| `packages/anvil/contracts` | README only | TSCOMPAT | TS contracts | Local README | none | README only |
| `packages/anvil/policy` | README only | TSCOMPAT | Legacy TS policy | Keep README | none | README only |
| `packages/anvil/ports` | README only | TSCOMPAT | TS ports | Keep README | none | README only |
| `packages/anvil/runtime` | README only | TSCOMPAT | TS runtime for API/archive | Keep README | none | README only |
| `packages/anvil/observability` | README only | OBS | TS observability helpers | Keep README | none | README only |
| `packages/anvil/flags-catalogue` | README only | FLAGCAT | Flag catalogue helpers | Keep README | none | README only |
| `packages/anvil-driver-client` | component-doc required | DRVR | Editor-driver client | Keep README; ARCHITECTURE if protocol detail stays here | wire protocol (link driver as-built) | README + possible ARCHITECTURE |
| `packages/adapters` | README only | OPENSPEC | Plan-format adapters | Keep README/AGENTS | none | README only |
| `packages/aps` | README only | APS | Local APS tooling (not public APS product) | Keep README/AGENTS | none | README only |
| `packages/docs-meta` | README only | DOCRB | Docs metadata parser | Keep README/AGENTS | none | README only |
| `packages/eslint-plugin-anvil` | README only | SCAN | ESLint plugin | Keep README | none | README only |
| `packages/edda-stack` | component-doc required | EDDA | Edda/Ember TS package (retiring) | Keep README; internals link `docs/architecture/edda-stack.md` | none locally | README only until retirement |
| `packages/kindling-integration` | component-doc required | KFIT | Kindling bridge | Keep README | privacy/capture boundary (central) | README only + link |
| `packages/libs` | grouping-only | — | Group folder | none | none | grouping-only |
| `packages/libs/render` | README only | DASH | json-render primitives | Keep README | none | README only |
| `packages/shared` | grouping-only | — | Group folder | Keep group README | none | grouping-only |
| `packages/shared/admin-contracts` | README only | API | Admin contracts | Local README | none | README only |
| `packages/shared/storage` | README only | API | Storage adapters | Keep README | none | README only |
| `packages/tooling` | grouping-only | — | Group folder | Keep group README | none | grouping-only |
| `packages/tooling/eslint-config` | explicit exemption | DEVENV | Shared ESLint config | Keep README | none | explicit exemption |
| `packages/tooling/tsconfig` | explicit exemption | DEVENV | Shared TS configs | Keep README | none | explicit exemption |
| `packages/transactional` | README only | API | Email templates | Keep README | none | README only |

## 4. Central architecture documents

| Document | Class | Owner | Concern | Disposition |
| -------- | ----- | ----- | ------- | ----------- |
| `docs/architecture/README.md` | central cross-system authority | DOCRB | Directory map + production host record | retain (updated in DOCRB-001) |
| `docs/architecture/overview.md` | central cross-system authority | DOCRB | System context and container/component relationships | retain; redraw Mermaid in DOCRB-006 |
| `docs/architecture/quality-model.md` | central cross-system authority | KERN | Check/gate/watch conceptual model | retain |
| `docs/architecture/trust-and-deployment-boundaries.md` | central cross-system authority | DOCRB | Macro local/hosted trust and deployment boundaries | added in DOCRB-006 |
| `docs/architecture/save-to-validation.md` | central cross-system authority | DOCRB | Cross-owner caller-buffer and post-save validation sequence | added in DOCRB-006 |
| `docs/architecture/docs-delivery.md` | central cross-system authority | DOCRB / DSITE gap | Source/build/deploy and shell/private/public flow | added in DOCRB-006 |
| `docs/architecture/rust-architecture-overview.md` | central cross-system authority | KERN | Crate layout | retain; do not duplicate `overview.md` |
| `docs/architecture/oss-surface.md` | central cross-system authority | OSS | Three OSS repos | retain |
| `docs/architecture/jsts-release-surfaces.md` | central cross-system authority | REL | JS/TS release tiers | retain |
| `docs/architecture/edda-stack.md` | central cross-system authority | EDDA | Kindling/Ember/Edda contract | retain |
| `docs/architecture/kernel-as-built.md` | component (misplaced) | KERN | Kernel internals | move → `crates/anvil-kernel/ARCHITECTURE.md`; leave a stub link |
| `docs/architecture/checks-as-built.md` | component (misplaced) | SCAN | Checks pipeline | move → `crates/anvil-checks/ARCHITECTURE.md` |
| `docs/architecture/intercept-as-built.md` | component (misplaced) | INTD | Intercept daemon | move → `crates/anvil-intercept/ARCHITECTURE.md` |
| `docs/architecture/mcp-shim-as-built.md` | component (misplaced) | MCP | MCP shim | merge into `crates/anvil-cli/ARCHITECTURE.md` |
| `docs/architecture/activation-as-built.md` | component (misplaced) | LAUNCH | `anvil start` | merge into `crates/anvil-cli/ARCHITECTURE.md` |
| `docs/architecture/cli-tui-runner-as-built.md` | component (misplaced) | TUI | CLI TUI runner | merge into `crates/anvil-cli/ARCHITECTURE.md` |
| `docs/architecture/tui-as-built.md` | component (misplaced) | TUI | TUI surfaces | move → `crates/anvil-tui/ARCHITECTURE.md` |
| `docs/architecture/widgets-as-built.md` | component (misplaced) | TUI | Widget catalogue | merge into tui ARCHITECTURE or keep as appendix link |
| `docs/architecture/api-as-built.md` | component (misplaced) | API | anvil-api | move → `apps/anvil-api/ARCHITECTURE.md` |
| `docs/architecture/auth-as-built.md` | central / split | BAUTH | Auth across CLI, API, docs-shell | retain as cross-system; strip docs-site-as-live wording in DOCRB-005 |
| `docs/architecture/driver-framework-as-built.md` | component (misplaced) | DRVR | Driver protocol | move toward `packages/anvil-driver-client` / intercept-proto |
| `docs/architecture/observability-as-built.md` | component (misplaced) | OBS | Observability crate | move → `crates/anvil-observability/ARCHITECTURE.md` |
| `docs/architecture/capsule-as-built.md` | component (misplaced) | CAPS | Capsules | move → `crates/anvil-capsule/ARCHITECTURE.md` |
| `docs/architecture/adapter-packages-as-built.md` | component (misplaced) | OPENSPEC | Adapters | move → `packages/adapters/ARCHITECTURE.md` or keep README |
| `docs/architecture/tutorial-as-built.md` | component (misplaced) | TUI | Tutorial engine | merge into `crates/anvil-tui/ARCHITECTURE.md` |
| `docs/architecture/_as-built-template.md` | generated/vendor (template) | DOCRB | Authoring template | retain as template; retarget to component ARCHITECTURE in DOCRB-003 |
| `docs/architecture/rust-kernel-spec.md` | historical | KERN | H1 kernel intent | historical; `kernel-as-built` / local ARCHITECTURE owns shipped truth |
| `docs/architecture/anvil-architecture-evolution.md` | historical | KERN | H1/H2 rollout | historical |
| `docs/architecture/rust-mcp-server-spec.md` | central (active spec) | RMCPF | MCP dual-era | retain until module closes |
| `docs/architecture/kernel-benchmarking-spec.md` | central | BENCH | Bench method | retain |
| `docs/architecture/graph-v2-foundation-spec.md` | historical (frozen) | GV2 | Graph v2 spine | historical / frozen contract |
| `docs/architecture/graph-context-delivery-spec.md` | historical (frozen) | GCTX | GCTX contract | historical / frozen contract |
| `docs/architecture/dev-acceleration-benchmark-spec.md` | central | DEVACC | Acceleration benches | retain |
| `docs/architecture/system-spec.md` | historical / target-state | PFGW | Aspirational topology | historical; not live |
| `docs/architecture/references/*` | historical | — | Advisory notes | historical |

## 5. Diagrams

| Diagram | Format | Location | Disposition | Notes |
| ------- | ------ | -------- | ----------- | ----- |
| System overview | Mermaid | `docs/architecture/overview.md` | retain / redraw | Central C4-like context; DOCRB-006 rebuilds |
| Quality model | Mermaid | `docs/architecture/quality-model.md` | retain | Cross-system conceptual |
| Adapter workflow | Mermaid | `docs/guides/adapters/workflow-guide.md` | retain | Guide-local; not a second system map |
| Monitor feature | Mermaid | `docs/archive/MONITOR_FEATURE.md` | retire (already archived) | do not treat as live |
| System components | Draw.io | `docs/architecture/anvil-system-components.drawio` | retired by DOCRB-006 | Replaced by the Mermaid central set; historical archive references remain history |
| PPTX workflow | Draw.io | `docs/architecture/pptx-workflow.drawio` | retired by DOCRB-006 | No shipped PPTX workflow; historical archive references remain history |
| Public visual layer | Draw.io + SVG | `docs/public/**` | missing | 0 committed pairs; create in DOCRB-007/008 |

Five required DOCRB-006 views, alongside the retained supporting central
authorities:

1. System context — `docs/architecture/overview.md`.
2. Container/component map — `docs/architecture/overview.md`.
3. Trust and deployment boundary —
   `docs/architecture/trust-and-deployment-boundaries.md`.
4. Save-to-validation sequence —
   `docs/architecture/save-to-validation.md`.
5. Docs delivery flow — `docs/architecture/docs-delivery.md`.

Component internals must not be redrawn into those five.

## 6. Public page families

| Family | Pages | Product source (ADR-122) | Owner | Diátaxis placement | Disposition |
| ------ | ----- | ------------------------ | ----- | ------------------ | ----------- |
| `docs/public/anvil/**` | 46 | in-repo product | DOCSYNC + product owners | mixed tutorial/how-to/reference/explanation | retain content; DOCRB-008 labels types and adds curated diagrams |
| `docs/public/beta/**` | 1 | in-repo product | DOCSYNC | how-to | retain |
| `docs/public/start-here/**` | 3 | in-repo | DOCSYNC | tutorial / explanation | retain |
| `docs/public/edda-stack/**` | 7 | in-repo product | DOCSYNC / EDDA | explanation | retain; full ADR-119 triple |
| `docs/public/kindling/**` | 19 | copied external | DOCSYNC | mixed | retain as copy; ADR-122 pin |
| `docs/public/aps/**` | 15 | copied external spec | DOCSYNC | mixed | retain as copy; ADR-122 pin |

No public Draw.io/SVG assets exist. DOCRB-008 selects journeys; this
inventory does not pick public content priorities.

Generated public/reference pages (CLI reference and similar) stay
`generated/vendor` — do not hand-maintain.

## 7. Duplicate-authority pairs

| Concern | Apparent authorities | Resolution |
| ------- | -------------------- | ---------- |
| Live docs host | DSITE module (`apps/docs-site` as shared host); `docs/architecture/README.md` (pre-001); `infra/src/vercel.ts` | ADR-123: vercel.ts is implementation truth; README/governance record the gap; DSITE status unchanged |
| Kernel internals | `kernel-as-built.md` and `rust-kernel-spec.md` | spec is historical; as-built then local ARCHITECTURE |
| System map | `overview.md` Mermaid and retired `anvil-system-components.drawio` | `overview.md` is the sole live engineering authority after DOCRB-006 |
| TUI surfaces | `tui-as-built.md`, `widgets-as-built.md`, `cli-tui-runner-as-built.md` | one crate ARCHITECTURE + links |
| Auth | `auth-as-built.md`, docs-site AGENTS, docs-shell tests, ADR-066 | keep one cross-system as-built; update host names in DOCRB-005 |
| Public Anvil behaviour | `docs/public/anvil/**` vs internal as-builts | public describes shipped product; internal describes implementation; do not copy |
| APS public vs `packages/aps` | public APS pages vs local tooling | ADR-122 already splits these |
| Architecture README vs CONTEXT.md | both list docs apps | CONTEXT stays orientation; README + ADR-123 own production topology |

## 8. Broken or weak discovery paths

- **No component `ARCHITECTURE.md` exists.** Maintainers cannot start beside
  code for internals.
- **25 of 36 workspace crates** have no root README
  (`anvil-attribution`, `anvil-baseline`, `anvil-capsule`,
  `anvil-checks-ast`, `anvil-config`, `anvil-dashboard-server`,
  `anvil-gctx-egress`, `anvil-gctx-types`, `anvil-grammar-wat`,
  `anvil-graph-cache`, `anvil-hook`, `anvil-intercept`,
  `anvil-intercept-macos`, `anvil-intercept-proto`,
  `anvil-intercept-rules`, `anvil-intercept-win32`, `anvil-l4`,
  `anvil-observability`, `anvil-policy-engine`, `anvil-rayon-init`,
  `anvil-rules`, `anvil-run`, `anvil-sarif`, `anvil-witness`,
  `workspace-hack`).
- **Four live docs/dashboard apps** have no README: `anvil-docs-private`,
  `dashboard`, `docs-public`, `docs-shell`.
- **`packages/anvil/contracts`** and **`packages/shared/admin-contracts`**
  have no README.
- **`crates/anvil-plan-read-model`** is on disk but not a workspace member —
  easy to miss in crate indexes.
- **Both Draw.io sources lack SVG**, so they cannot be reviewed or published
  as accessible public assets.
- **`docs/architecture/README.md` (before DOCRB-001)** pointed readers at
  `apps/docs-site` as the public renderer. Fixed on the stack base; inbound
  archive docs still say that.
- **CONTEXT.md** lists `apps/docs-site/AGENTS.md` as the only docs spoke.
  After pilots, add `apps/docs-shell` (DOCRB-004/005), not in this item.

## 9. Pilot set for DOCRB-004

Selected to cover materially different surfaces:

| Pilot | Why |
| ----- | --- |
| `crates/anvil-kernel` | Rust engine / graph |
| `crates/anvil-intercept` | Save-time daemon / MCP-adjacent |
| `apps/dashboard` + `crates/anvil-dashboard-server` | Local UI + API |
| `apps/anvil-api` | Hosted API |
| `apps/docs-shell` | Documentation delivery and live host |

## 10. Out of scope (recorded, not done)

- Moving or deleting any document (DOCRB-005).
- Changing DSITE, DOCFRESH, or DOCSYNC work-item status.
- Activating mandatory CI (DOCRB-009).
- Redrawing public diagrams (DOCRB-007/008).
