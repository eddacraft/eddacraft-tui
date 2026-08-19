# anvil documentation shell architecture

| Type         | Authority | Owner           | Status | Freshness                                                                                                                                                    |
| ------------ | --------- | --------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Architecture | Derived   | DOCRB/DSITE gap | Live   | Last reviewed 2026-08-20 against `d6c8b565c`, `apps/docs-shell/proxy.ts`, `apps/docs-shell/app/auth/**`, `apps/docs-shell/lib/**`, and `infra/src/vercel.ts` |

| Upstream                                                                                                                                          | Downstream                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `apps/docs-shell/**`, `infra/src/vercel.ts`, ADR-123, BAUTH's `docs/architecture/auth-as-built.md`, and `docs/guides/documentation-governance.md` | Docs-shell maintainers and production readers |

> **DOCRB-004 pilot:** this source-linked component map does not close the
> DOCRB/DSITE ownership gap or supersede ADR-123, documentation governance, or
> BAUTH's [auth as-built](../../docs/architecture/auth-as-built.md). DOCRB-005
> owns central migration, not this pilot.

## Scope and boundaries

The shell owns request classification, session-cookie handling, and proxy
transport at the live `docs.eddacraft.ai` entrypoint.
[`infra/src/vercel.ts`](../../infra/src/vercel.ts) is deployment truth:
`apps/anvil-docs-private` and `apps/docs-public` are live upstream renderers,
while `apps/docs-site` is retained only for rollback. Authentication semantics
belong to BAUTH and are consumed here.

## Authentication and renderer routing

This diagram owns the shell's request-to-renderer concern.

```mermaid
flowchart LR
    Request[docs.eddacraft.ai request] --> Route{path class}
    Route -->|public path| Public[docs-public renderer]
    Route -->|anvil path| Session{valid docs session?}
    Session -->|no| Login[GitHub OAuth and BAUTH exchange]
    Login --> Session
    Session -->|yes| Private[anvil-docs-private renderer]
    Public --> Response[filtered proxy response]
    Private --> Response
```

Routing and proxy transport trace to [`proxy.ts`](proxy.ts). The login and
callback flow traces to [`app/auth/login/route.ts`](app/auth/login/route.ts),
[`app/auth/callback/route.ts`](app/auth/callback/route.ts), and
[`lib/bauth.ts`](lib/bauth.ts); licence verification traces to
[`lib/jwt.ts`](lib/jwt.ts). In prose: public paths go directly to the public
renderer. An anvil path requires a valid docs session; otherwise the reader is
sent through GitHub OAuth and the BAUTH exchange before the private renderer is
used. Both renderer responses pass through the same filtered proxy boundary.

## Invariants, failure, and fallback

- ES256 licence verification checks issuer, audience, subject, and an allowed
  tier. Verification errors fail closed and clear an invalid session.
- OAuth state is encrypted and tied to a short-lived nonce; the callback
  validates its next path before redirecting.
- The proxy forwards only an allowlist of request headers, injects the upstream
  secret, and strips sensitive or hop-by-hop response headers.
- Upstream redirects may be rewritten only from a known renderer origin;
  redirects into `/auth/` are forbidden.
- Upstream requests time out after 15 seconds and return an honest 503 on
  timeout or transport failure.
- `apps/docs-site` is rollback-only. It is not selected by the live routing flow
  and must not be depicted as an active renderer.

Production topology and the unresolved owner remain authoritative in
[documentation governance](../../docs/guides/documentation-governance.md).
