# anvil documentation shell

| Type   | Authority     | Owner           | Status | Freshness                                                                                           |
| ------ | ------------- | --------------- | ------ | --------------------------------------------------------------------------------------------------- |
| README | Authoritative | DOCRB/DSITE gap | Live   | Last reviewed 2026-08-20 against `d6c8b565c`, `apps/docs-shell/proxy.ts`, and `infra/src/vercel.ts` |

| Upstream                                                                                                                                          | Downstream                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `apps/docs-shell/**`, `infra/src/vercel.ts`, ADR-123, BAUTH's `docs/architecture/auth-as-built.md`, and `docs/guides/documentation-governance.md` | Production docs requests and `apps/docs-shell/ARCHITECTURE.md` |

This Next.js application is the live public entrypoint at `docs.eddacraft.ai`.
It authenticates `/anvil/*` and proxies those requests to
`apps/anvil-docs-private`; public APS, kindling, edda-stack, and blog paths are
proxied to `apps/docs-public`. `apps/docs-site` is rollback-only and is not the
live renderer.

The ownership field is intentionally a gap, not a joint assignment. DOCRB owns
this documentation pilot; DSITE still owns recorded legacy host work and has not
adopted the live shell. ADR-123 and documentation governance record that
unresolved boundary without changing DSITE status.

## Entry points

- [`proxy.ts`](proxy.ts) performs path classification, session verification,
  upstream routing, header filtering, and response handling.
- [`app/auth/login/route.ts`](app/auth/login/route.ts) starts GitHub OAuth with
  encrypted state and a nonce.
- [`app/auth/callback/route.ts`](app/auth/callback/route.ts) exchanges the
  callback through the BAUTH API and establishes the docs session.
- [`lib/jwt.ts`](lib/jwt.ts) verifies the ES256 licence and access tier.
- [`lib/bauth.ts`](lib/bauth.ts) calls the hosted authentication authority.

## Local validation

```bash
pnpm --filter @eddacraft/docs-shell test
pnpm --filter @eddacraft/docs-shell typecheck
pnpm --filter @eddacraft/docs-shell build
```

## Architecture and authorities

Read the source-linked [local architecture](ARCHITECTURE.md) for the routing and
trust flow. BAUTH's [auth as-built](../../docs/architecture/auth-as-built.md)
owns authentication meaning.
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
and the
[documentation governance guide](../../docs/guides/documentation-governance.md)
own production topology and the DOCRB/DSITE gap. This pilot does not replace
those central authorities.
