# Documentation delivery

| Type  | Authority     | Owner           | Status | Freshness                                                                                                                                                                                                |
| ----- | ------------- | --------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCRB/DSITE gap | Live   | Last reviewed 2026-08-20 at `97899b00a` against `docs/public/**`, both renderer configs and middleware matchers, `apps/docs-shell/**`, `infra/src/vercel.ts`, and `tools/scripts/vercel-ignore-build.sh` |

| Upstream                                                                                                                                                                                                                                                       | Downstream                                                                                  |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| ADR-123, `docs/public/**`, `apps/anvil-docs-private/docusaurus.config.ts`, `apps/docs-public/docusaurus.config.ts`, `apps/docs-shell/ARCHITECTURE.md`, `infra/src/vercel.ts`, `infra/src/components/vercel-app.ts`, and `tools/scripts/vercel-ignore-build.sh` | Production documentation topology, operations, and public-information-architecture planning |

## Audience, concern, and local authority

This macro view is for documentation maintainers and operators tracing content
from source to the live host. It owns source/build/deployment relationships and
the shell-to-renderer split. Request classification, OAuth state, cookie,
licence, header filtering, redirect, timeout, and response handling remain in
the docs-shell [component architecture](../../apps/docs-shell/ARCHITECTURE.md).
BAUTH owns licence semantics in [authentication as-built](auth-as-built.md).

## Source, build, deployment, and request flow

```mermaid
flowchart LR
    subgraph Source["Repository source"]
        AnvilSource[docs/public/anvil and docs/public/beta]
        PublicSource[docs/public/aps, kindling, edda-stack, and public blog]
        ShellSource[apps/docs-shell]
    end

    subgraph Build["Vercel production builds from main"]
        PrivateBuild[build anvil-docs-private]
        PublicBuild[build docs-public]
        ShellBuild[build docs-shell]
    end

    subgraph Deploy["Protected deployments"]
        Private[anvil-docs-private renderer]
        Public[docs-public renderer]
        Shell[docs.eddacraft.ai docs shell]
    end

    AnvilSource --> PrivateBuild --> Private
    PublicSource --> PublicBuild --> Public
    ShellSource --> ShellBuild --> Shell

    Reader[documentation reader] --> Shell
    Shell -->|/anvil entitlement required; protected upstream secret| Private
    Shell -->|public routes; protected upstream secret| Public
```

In prose: governed Markdown under `docs/public/anvil` and `docs/public/beta` is
built by `apps/anvil-docs-private`. APS, kindling, edda-stack, and blog sources
are built by `apps/docs-public`. The independent Next.js `apps/docs-shell` build
owns the public domain `docs.eddacraft.ai`. Vercel's project-level ignore
commands build production from `main` when the relevant app, shared workspace
inputs, or declared content paths change; preview deployments are disabled for
these projects.

Every reader request enters the shell. `/anvil` and `/anvil/*` require a valid
entitled licence before the private renderer is selected. Other matched
documentation routes use the public renderer. The shell injects
`DOCS_UPSTREAM_SECRET` as `X-Docs-Upstream-Secret`; both renderer middleware
boundaries reject matched direct requests that do not carry the matching secret.
Their matcher excludes `/favicon.ico`, so the shared-secret statement is
deliberately not universal to every renderer path.

`apps/docs-site` was the rollback artefact: no production domain, ignore command
`--always-skip`, no live request edge. It was retired on 2026-07-08
(`847436623`) and **deleted** once the rollback window closed — the shell had
been live for six weeks without a rollback being needed. Its navigation
authority moved to the live hosts (`apps/anvil-docs-private/sidebars/anvil.ts`
and `apps/docs-public/sidebars/aps.ts`), which is what
`scripts/docs/check-public-docs.mjs` now reads. `docs/public/start-here` went
with it: only docs-site rendered that section, so it had been unpublished since
the same date.

## Source trace and ownership gap

- Content mounts and route bases trace to
  `apps/anvil-docs-private/docusaurus.config.ts` and
  `apps/docs-public/docusaurus.config.ts`.
- Build watches, production branch selection, disabled previews, domains,
  renderer hosts, and environment wiring trace to `infra/src/vercel.ts`,
  `infra/src/components/vercel-app.ts`, each app's `vercel.json`, and
  `tools/scripts/vercel-ignore-build.sh`.
- The `/anvil` entitlement branch, public routing branch, and injected
  upstream-secret header trace to `apps/docs-shell/proxy.ts` and
  `apps/docs-shell/lib/jwt.ts`.
- Renderer protection and the explicit `/favicon.ico` matcher exclusion trace to
  `apps/anvil-docs-private/middleware.ts` and `apps/docs-public/middleware.ts`.
- Detailed login, proxy, failure, and fallback behaviour remains in
  `apps/docs-shell/ARCHITECTURE.md`; this macro view does not duplicate it.

The **DOCRB/DSITE ownership gap** is closed on the topology side: DOCRB
documents the live hosts, and the legacy `apps/docs-site` host that DSITE owned
no longer exists. Any remaining DSITE work is recorded history, not a live
surface.
