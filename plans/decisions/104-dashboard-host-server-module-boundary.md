# ADR-104: Dashboard host, server, and module authority boundary

## Status

Accepted 2026-07-09

## Date

2026-07-09

## Context

The active DASH plan was Ready for a browser dashboard inside
`apps/website/` using Next.js 16 App Router, Next API routes, shadcn/ui, and
Recharts. Before implementation, the owner proposed aligning DASH with the
newly ratified eddacraft app stack: React, Vite 8, TanStack Router, TanStack
Query, TanStack Table, shadcn/ui, Tailwind v4, React Hook Form, Zod, OpenAPI
generated clients, json-render first, Recharts as fallback chart primitives,
and Better Auth as the future OIDC-ready auth layer.

The planning council agreed that the stack change is not a local implementation
detail. Moving from Next API routes to Vite changes the API boundary, local
file-read trust boundary, routing model, dashboard module model, and downstream
DASHCORE/DASHARCH/DASHOPS assumptions. DASH must therefore be amended before
implementation resumes.

Anvil's scope guard also requires dashboard surfaces to stay tied to
enforcement, evidence, and provenance rather than becoming a generic metrics
product. The dashboard architecture must make the authority boundary explicit:
UI modules may request actions, but kernel capabilities decide whether actions
are allowed.

## Decision

Adopt a dedicated dashboard architecture for DASH:

- `apps/dashboard/` is the browser dashboard host, built with React, Vite 8,
  TanStack Router, TanStack Query, TanStack Table, shadcn/ui, Tailwind v4, Zod,
  and json-render-first composition.
- `crates/anvil-dashboard-server/` is introduced in Wave 1 as the local
  loopback-bound read-only API server for dashboard data. It owns `.anvil/` and
  workspace artefact access through kernel/crate APIs, not through browser code.
- The dashboard seam is:

  ```text
  Rust API
    -> OpenAPI
    -> generated TypeScript client
    -> TanStack Query
    -> dashboard modules
  ```

- Dashboard modules are UI adapters over kernel capabilities. A module owns
  navigation, layout, rendering, interaction, local UI state, and schema-driven
  views. It does not own permissions, workflow state, audit, evidence, or policy
  decisions.
- Action-capable modules may request actions through typed command envelopes,
  but kernel/server code decides `allowed`, `denied`, `accepted`, or `rejected`
  and emits audit/evidence records. Wave 1 remains read-only; write/action
  execution is a later explicit design gate.
- Wave 1 includes at least two proof modules:
  - Protection Overview: user-facing, planless view of save-time protection
    state, latest runs, warnings, affected files, and evidence.
  - Plan Driver: internal/dogfood module over APS plans, runs, and evidence.
- `apps/website/` remains the existing Next.js website unless a separate
  migration decision moves marketing/docs surfaces. Future target shape is:
  `apps/dashboard` for the dashboard, `apps/docs` for Astro/Starlight docs, and
  `apps/marketing` for Astro marketing.
- `apps/anvil-api/` remains the hosted cloud/user/auth API unless separately
  re-scoped. It must not become the local `.anvil/` artefact reader by accident.
- Better Auth, Monaco, xterm.js, real-time channels, write endpoints, and
  multi-user auth are deferred out of DASH Wave 1.

## Rationale

The dedicated Vite dashboard host matches the owner-approved eddacraft app
stack and avoids coupling the operational dashboard to the current website's
Next/RSC assumptions. The Rust dashboard server keeps the highest-risk local
file access boundary out of the browser and gives the generated TypeScript
client a stable OpenAPI seam.

The module model preserves Anvil's product boundary. Dashboard plugins make
kernel capabilities visible and usable; they do not become policy or workflow
authorities. This supports future modules without allowing UI state to become a
source of truth.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Dedicated `apps/dashboard` + `crates/anvil-dashboard-server` | Aligns with the new app stack; makes the local API trust boundary explicit; keeps website and dashboard lifecycles separate; supports generated client and module model | Requires DASH APS rewrite; introduces a new app and crate in Wave 1 |
| Keep DASH in `apps/website` with Next API routes | Lowest short-term migration cost; existing app already has React/Tailwind/shadcn setup | Bakes in a stack the owner no longer wants for eddacraft apps; couples dashboard to website/RSC; makes Next routes the accidental local API |
| Migrate website, docs, marketing, and dashboard together | One large frontend reset | Too broad for DASH; would delay the user-facing protection dashboard and mix unrelated migrations |
| Pure Vite SPA without local server | Simple frontend scaffold | Browser cannot safely read local `.anvil/` artefacts; no authority boundary for action requests |
| Hosted `apps/anvil-api` reads local artefacts | Reuses an existing API app | Violates the local artefact trust boundary unless the deployment model is explicitly redesigned |

## Consequences

- **Positive:** DASH gets a clear app boundary, explicit local API boundary,
  generated-client seam, and capability-oriented module model.
- **Positive:** The first user-facing module can focus on Anvil's core promise:
  "is this repo protected right now?"
- **Positive:** Future docs/marketing migration remains possible without
  blocking DASH.
- **Negative:** The existing Ready DASH plan and downstream route/file paths
  must be amended before implementation.
- **Negative:** Wave 1 now includes a server crate and OpenAPI generation path,
  increasing foundation work before visual pages land.
- **Risks:** Local API path traversal, symlink traversal, CORS/origin leakage,
  and accidental write/action authority.
- **Mitigations:** Bind the dashboard server to loopback, enforce workspace
  root containment and canonical path checks, keep Wave 1 read-only, validate
  responses with Zod/OpenAPI fixtures, and require explicit future ADR/APS
  gates for auth, write actions, real-time channels, Monaco, and xterm.js.

## References

- Related ADRs: ADR-001, ADR-002, ADR-073
- APS modules: DASH, DASHCORE, DASHARCH, DASHOPS, DASHAI
- Planning council: 2026-07-09 DASH stack review
- External: Vite, TanStack Router, TanStack Query, TanStack Table, json-render,
  Better Auth
