# Docs Auth — SPA Gap Remediation

**Date:** 2026-04-11
**Status:** Draft
**Supersedes (in part):** `plans/specs/2026-04-03-docs-auth-gating-design.md` (DOCSAUTH)
**Module target:** DOCSAUTH2 (new) — treats current DOCSAUTH as Phase 0

## Context

The existing DOCSAUTH module (Complete, 7/7) placed a Vercel edge middleware in
front of `/anvil/:path*` on `docs.eddacraft.ai`. Verification on production
reveals two gaps:

1. **SPA bypass.** `docs.eddacraft.ai` is a Docusaurus SPA. The homepage and
   other public sections contain client-side `<Link>` components to
   `/anvil/overview` and `/anvil/quickstart`. Clicking them does not issue an
   HTTP request for the route; Docusaurus uses React Router `history.pushState`
   and loads the view from an already-shipped route table in `main.js`. The
   middleware never runs. Worse, the code-split chunks under `/assets/js/*` are
   unauthenticated by design — matching them would break the entire public
   site — so an adversary who extracts chunk hashes from `main.js` can fetch
   any Anvil page's React component directly.
2. **Adjacent plugin leak.** A `beta` Docusaurus plugin registered at
   `routeBasePath: 'beta'` sits outside the middleware matcher and serves an
   unlisted beta quickstart unauthenticated. `TOGGLING-DOCS.md` is stale and
   also claims Kindling and edda-stack are disabled — they are not, though
   those are intentionally public.

The original DOCSAUTH design asserted *"content never leaves the edge without
auth"*. That assertion is false under Docusaurus SPA semantics. No amount of
middleware matcher tuning on the existing single-build architecture can fix
it without breaking the public site.

### Threat model — what actually matters

Anvil is pre-traction. Launch is 1–3 months out, but the window in which
cloning would actually harm eddacraft extends through traction — potentially
6–18 months after launch. Today ~10 people are aware of the product, 1 of
whom is an NDAd outsider, and existing public exposure of Anvil-specific
content is near zero.

The real worry driving this work is **category/clone risk**: a competitor,
analyst, or motivated clone-builder using the high-quality Anvil
documentation as a specification to rebuild the product before eddacraft
establishes brand and market presence. This risk does not end at launch —
a public launch with gated docs is viable and desirable, and the gate is
expected to remain on indefinitely for attribution and ToS leverage even
after the clone-risk window closes.

Relevant adversary classes:

| Adversary | Affected by SPA bypass? | Mitigation |
| --- | --- | --- |
| Search crawlers (Googlebot) | No — they honour HTTP 302 | 302 + `robots.txt` + `noindex` |
| Archive crawlers (Common Crawl, archive.org) | No — they honour 302 | 302 + `robots.txt` |
| Static LLM training crawlers (GPTBot, ClaudeBot, CCBot) | No — they honour 302 | 302 + `llms.txt` + `robots.txt` |
| JS-executing agents (ChatGPT Browse, Perplexity, Claude web fetch) | **Yes** — they execute JS and follow SPA navigations | Real auth at origin |
| Human researchers using a browser | **Yes** — Anvil content is in the bundled JS | Real auth at origin |
| Motivated clone-builder | **Yes** — will extract chunks, read everything | Real auth at origin |

**The fix must actually prevent Anvil content bytes from reaching an
unauthenticated client, not merely hide them visually.**

### Policy posture

The architecture in this spec is orthogonal to **who is allowed in**. BAUTH's
existing `beta_users.status = pending | active` field is the policy dial.
Three postures are supported with zero architectural change:

| Posture | Default `status` on signup | Effect |
| --- | --- | --- |
| **A. Open free tier** | `active` | Self-service signup, instant access. Readers are attributed and ToS-bound but not vetted. |
| **B. Waitlist / invite-only** | `pending` | Admin must approve each signup. Functionally invite-only, self-service wrapper. |
| **C. No gate** | n/a | Docs public. Rejected — incompatible with clone-risk threat model. |

**Current posture:** B (waitlist). Approved users today = the ~10 known
people + any explicit additions.

**Future trajectory:** flip to A at traction, not at launch. The gate itself
(attribution, ToS, rate-limit, LLM/crawler exclusion) is expected to remain
on indefinitely regardless of posture.

Policy flips are a one-line change in the BAUTH signup path and an
operational decision, not a spec change. This spec is written so either A or
B works unchanged.

## Goals

- No Anvil documentation bytes (HTML, JS chunks, MDX, images, or search index)
  served to unauthenticated clients from any public origin.
- Cross-links between public docs (Kindling, APS, edda-stack, blog) and
  private Anvil docs feel native — same top nav, same origin, same cookie.
- Public docs remain fully public, crawlable, and indexable.
- BAUTH integration reuses `POST /api/v1/auth/github/callback` (DOCSAUTH-001).
- Auth flow survives an eventual Better-Auth migration without re-architecting
  the gate.
- Deployable within ~2–3 working days.

## Non-goals

- Per-page ACLs inside `/anvil/*`. All-or-nothing.
- Migrating Anvil docs to MDX-in-Next.js (considered and rejected — too slow).
- Replacing BAUTH with Clerk/Auth0 as part of this work.
- Adding search to the shell landing page.
- Refresh token rotation in the browser (BAUTH JWT is stateless, 7-day
  expiry — re-auth via GitHub is instant if the GitHub session is warm).

## Architecture

A small Next.js "shell" app owns the `docs.eddacraft.ai` origin. Two
Docusaurus builds live behind it as separate Vercel projects, reached via
Vercel rewrites from the shell. Routing Middleware on the shell gates every
`/anvil/*` request — HTML and assets — against the BAUTH JWT cookie.

```
                  docs.eddacraft.ai  (Next.js shell — new Vercel project)
                  │
                  ├─ /                          → Next.js landing
                  ├─ /auth/login                → Next.js route
                  ├─ /auth/callback             → Next.js route (calls BAUTH)
                  ├─ /auth/logout               → Next.js route
                  ├─ /auth/pending              → Next.js route (403 UX)
                  │
                  ├─ /anvil/:path*              → [MIDDLEWARE GATE]
                  │                                │
                  │                                ▼  (rewrite)
                  │                           anvil-docs-private.vercel.app
                  │                           (Docusaurus, baseUrl: '/anvil')
                  │                           serves /anvil/overview,
                  │                                  /anvil/assets/js/*, etc.
                  │
                  ├─ /kindling/:path*           → rewrite → docs-public.vercel.app
                  ├─ /aps/:path*                → rewrite → docs-public
                  ├─ /edda-stack/:path*         → rewrite → docs-public
                  ├─ /blog/:path*               → rewrite → docs-public
                  │
                  ├─ /robots.txt                → Next.js static
                  └─ /llms.txt                  → Next.js static
```

### Why Next.js shell, not a bare rewrite project

A framework-agnostic Vercel project with `middleware.ts` + `vercel.json` would
also work. Next.js is chosen because:

- **Auth routes as server components.** `/auth/login`, `/auth/callback`, and
  `/auth/logout` are simpler as App Router server components than as raw
  `api/*.ts` functions. Error states, pending approval, and "session expired"
  interstitials get real React pages instead of inline HTML strings.
- **Landing page.** `docs.eddacraft.ai/` deserves a small index instead of
  404-ing or defaulting into one of the two doc sets.
- **Better-Auth migration path.** The planned future auth system has
  first-class Next.js integration. Landing at a Next.js shell now means that
  migration is a swap of the auth helpers, not an architecture change.
- **Shared top nav (optional, Phase 2).** Next.js can inject a unified navbar
  via middleware response rewriting or via a sidecar header render. Not in
  scope for the first cut, but cheap if we want it later.

Cost of the choice: ~0.5–1 day of extra scaffolding vs. a bare shell. Worth
it for the above.

## The critical detail — Docusaurus `baseUrl`

Docusaurus's `baseUrl` config prefixes **all** emitted URLs, including
`/assets/js/*` chunks, `/assets/css/*`, static images, and the Algolia/local
search index. Setting `baseUrl: '/anvil'` on the private build causes every
asset to ship at `/anvil/assets/...`, which means the middleware matcher
`/anvil/:path*` catches them all.

```ts
// anvil-docs-private/docusaurus.config.ts
const config: Config = {
  baseUrl: '/anvil/',   // <-- the critical change
  url: 'https://docs.eddacraft.ai',
  plugins: [
    ['@docusaurus/plugin-content-docs', {
      id: 'anvil',
      path: 'anvil',
      routeBasePath: '/',   // relative to baseUrl, so effective = /anvil/
      sidebarPath: require.resolve('./sidebars/anvil.ts'),
    }],
  ],
  // Kindling / APS / edda-stack / beta plugins REMOVED from this config
};
```

The public build keeps `baseUrl: '/'` and registers only the public plugins
(Kindling, APS, edda-stack, blog). The `beta` plugin is **removed** — its
contents either merge into Anvil's private docs or are deleted.

**Verification step before committing:** a throwaway deploy of the private
build must confirm that (a) the homepage at `/anvil/` renders, (b)
view-source shows all script tags referencing `/anvil/assets/js/*.js`, and
(c) no asset path escapes the `/anvil/` prefix. If any asset leaks to
`/assets/*`, the whole approach is compromised — fallback to Approach 1
(subdomain split) from the brainstorming options.

## Middleware

Lives in the Next.js shell app at `middleware.ts`:

```ts
import { NextResponse, type NextRequest } from 'next/server';
import { jwtVerify, importSPKI } from 'jose';

const COOKIE_NAME = 'anvil-docs-session';
let cachedKey: CryptoKey | null = null;

async function getPublicKey() {
  if (cachedKey) return cachedKey;
  cachedKey = await importSPKI(process.env.LICENSE_PUBLIC_KEY!, 'ES256');
  return cachedKey;
}

export async function middleware(request: NextRequest) {
  const token = request.cookies.get(COOKIE_NAME)?.value;
  if (!token) return redirectToLogin(request);

  try {
    await jwtVerify(token, await getPublicKey(), { algorithms: ['ES256'] });
    return NextResponse.next();
  } catch {
    const response = redirectToLogin(request);
    response.cookies.delete(COOKIE_NAME);
    return response;
  }
}

function redirectToLogin(request: NextRequest) {
  const loginUrl = new URL('/auth/login', request.url);
  loginUrl.searchParams.set('next', request.nextUrl.pathname);
  return NextResponse.redirect(loginUrl, 302);
}

export const config = {
  matcher: ['/anvil/:path*'],
};
```

`matcher: ['/anvil/:path*']` covers HTML routes **and** `/anvil/assets/*`
chunks because `baseUrl` pushed the chunks there. The matcher is the entire
SPA-bypass fix — it works only because of the `baseUrl` decision above.

## Auth routes (Next.js App Router)

Replaces the current `apps/docs-site/api/auth/*.ts` serverless functions.

| Route | Purpose |
| --- | --- |
| `app/auth/login/route.ts` | Build GitHub OAuth URL with encrypted `state` containing the validated `next` param; 302 to GitHub |
| `app/auth/callback/route.ts` | Verify state, exchange `code` at `POST /api/v1/auth/github/callback` (existing BAUTH endpoint), set `anvil-docs-session` cookie, 302 to `next` |
| `app/auth/logout/route.ts` | Clear cookie, 302 to `/` |
| `app/auth/pending/page.tsx` | Server component — "Your access is pending approval" |
| `app/auth/error/page.tsx` | Server component — OAuth denied, state mismatch, BAUTH error |

Cookie attributes: `HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=604800`
(7 days, matches JWT expiry).

`next` param validation: must start with `/anvil/` and must not contain
protocol or `//`. Reject otherwise.

State encryption: AES-256-GCM with `DOCS_STATE_SECRET` (already in Key Vault
from DOCSAUTH-004), using Web Crypto (`crypto.subtle`) so it runs on the Edge
runtime. The nonce is regenerated per request and tracked via an `oauth-nonce`
cookie scoped to `/auth/callback` with `Max-Age=600` (10-minute TTL).
Single-use: the callback compares the cookie nonce to the state nonce and
clears the cookie on success. AEAD tag validation provides tamper detection;
GCM's IV is random-12 per encrypt.

## Public doc build rewrites

`docs.eddacraft.ai` shell project `next.config.ts`:

```ts
const config: NextConfig = {
  async rewrites() {
    return [
      // Private Anvil docs — middleware gates before rewrite runs
      { source: '/anvil/:path*', destination: 'https://anvil-docs-private.vercel.app/anvil/:path*' },
      // Public doc sections
      { source: '/kindling/:path*', destination: 'https://docs-public.vercel.app/kindling/:path*' },
      { source: '/aps/:path*',      destination: 'https://docs-public.vercel.app/aps/:path*' },
      { source: '/edda-stack/:path*', destination: 'https://docs-public.vercel.app/edda-stack/:path*' },
      { source: '/blog/:path*',     destination: 'https://docs-public.vercel.app/blog/:path*' },
    ];
  },
};
```

Next.js middleware runs **before** rewrites, so the gate fires for `/anvil/*`
before the request is proxied to the private build.

**Root path ownership.** `/` has no rewrite entry, so it is handled by the
shell's Next.js landing page. The public Docusaurus build must therefore
**not** register anything at `routeBasePath: '/'` and must not ship an
`src/pages/index.tsx` — its homepage is gone; visitors arrive via the shell.
The current `apps/docs-site/src/pages/index.tsx` content migrates to
`apps/docs-shell/app/page.tsx`. Existing Docusaurus homepage CSS moves with
it (or is rewritten against shadcn/Tailwind, author's choice during
implementation).

The private and public Docusaurus Vercel projects are marked
**Deployment Protection: "Only Accessible from Vercel Rewrites"** (or
equivalent — requires Vercel plan support; otherwise a shared secret header
the shell attaches and the targets require). This prevents an adversary from
hitting `anvil-docs-private.vercel.app/anvil/overview` directly.

## Cross-linking between public and private

Because everything lives on one origin, internal links are same-origin
absolute paths:

- Public Docusaurus sidebar can link to `/anvil/overview` — renders as a
  normal link; unauthenticated users get 302'd by the shell middleware.
- Private Docusaurus (Anvil) links back to `/kindling/overview`, etc.
- Both builds configure `url: 'https://docs.eddacraft.ai'` so the canonical
  URL reflects the user-facing origin, not the upstream Vercel URL.

Unified top-nav is **out of scope for this spec**. Each Docusaurus keeps its
own navbar. A future iteration can inject a shared header via middleware
response rewriting if desired.

## robots.txt and llms.txt

Lives on the shell, served as static Next.js routes:

```
# /robots.txt
User-agent: *
Disallow: /anvil/
Disallow: /auth/

Sitemap: https://docs.eddacraft.ai/sitemap.xml
```

```
# /llms.txt
# Anvil documentation is private (closed beta).
# Public sections: /kindling, /aps, /edda-stack, /blog
User-agent: *
Disallow: /anvil/
```

Also set `X-Robots-Tag: noindex, nofollow` header on all `/anvil/*` responses
(middleware adds it on the 302 redirect path; the private Docusaurus config
adds it to the authenticated HTML via a `<meta>` tag).

These are belt-and-braces — the 302 already prevents indexing — but costless
and help against misbehaving crawlers.

## Infrastructure changes

### New Vercel projects

1. `docs-shell` — Next.js App Router, new project, domain
   `docs.eddacraft.ai` (reassigned from current `docs-site`).
2. `anvil-docs-private` — Docusaurus, Anvil plugin only, `baseUrl: '/anvil/'`.
3. `docs-public` — Docusaurus, public plugins only, `baseUrl: '/'`. Can reuse
   the existing `docs-site` project if we strip its Anvil + beta plugins and
   rename.

### Pulumi (`infra/src/vercel.ts`)

- Add the two new projects with their env vars.
- `docs-shell`: `LICENSE_PUBLIC_KEY`, `DOCS_STATE_SECRET`, `BAUTH_API_URL`,
  `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` (the GitHub OAuth callback URL
  must be updated to the shell's `/auth/callback`).
- `anvil-docs-private` and `docs-public`: no secrets needed; just build config.
- Set deployment protection on the two docs projects so only rewrites from
  the shell can reach them.
- Move `docs.eddacraft.ai` domain binding from `docs-site` to `docs-shell`.

### Key Vault

No new secrets. `license-public-key`, `docs-state-secret`,
`github-oauth-client-id`, `github-oauth-client-secret` already exist
(DOCSAUTH-004).

### GitHub OAuth App

Update callback URL:
- Remove: `https://docs.eddacraft.ai/api/auth/callback` (current)
- Add: `https://docs.eddacraft.ai/auth/callback` (new — same path sans `/api`)

## Migration plan

Executed on `dev` branch in sequence, each step individually shippable:

1. **Scaffold `docs-shell` Next.js app** in `apps/docs-shell/` with landing
   page, auth routes, middleware, `next.config.ts` rewrites pointing at
   temporary placeholder destinations. Deploy to a preview URL. Verify the
   landing page, `/auth/login` → GitHub, and the middleware redirect loop.
2. **Split Docusaurus config.** Create `apps/docs-public/` and
   `apps/anvil-docs-private/` by copying `apps/docs-site/` and pruning
   plugins. Configure `baseUrl` appropriately. Deploy both as standalone
   Vercel projects to preview URLs.
3. **Verify `baseUrl` assumption** — the kill/no-kill checkpoint. On the
   private-build preview, confirm all assets live under `/anvil/`. If any
   leak, stop and fall back to subdomain-split (Approach 1).
4. **Wire rewrites.** Update `docs-shell` `next.config.ts` to point at the
   two doc builds' preview URLs. Test the full flow on a preview deployment:
   unauthenticated `/anvil/overview` → 302 → GitHub → callback → cookie →
   content renders.
5. **Pulumi changes.** Add the three projects, configure env vars, set
   deployment protection on the two upstream builds. `pulumi preview`, then
   apply.
6. **GitHub OAuth callback update.**
7. **DNS cutover.** Move `docs.eddacraft.ai` from `docs-site` to `docs-shell`
   in Vercel. DNS is already pointed at Vercel, so this is an internal
   routing change with instant propagation.
8. **Smoke test production.** Run the full auth matrix (DOCSAUTH test plan).
9. **Retire `docs-site`** — archive the project once prod is confirmed green
   for 48h. Keep the repo directory `apps/docs-site/` in git history but
   delete the files in a cleanup commit.

## Rollback

Each step is independently revertible; the only one-way door is step 7 (DNS
cutover). Rollback is: reassign `docs.eddacraft.ai` back to `docs-site` in
Vercel. The old project continues to work because nothing was deleted until
step 9.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Docusaurus `baseUrl` doesn't fully prefix assets (search index, sitemap, images) | Low | High | Step 3 is a hard kill-switch; fallback to subdomain split |
| Next.js rewrite drops cookies on upstream fetch | Low | Medium | Pre-verified on preview deployment in step 4 |
| Deployment Protection unavailable on current Vercel plan | Medium | Medium | Fallback: shared-secret header check in a Docusaurus plugin or a cloudflare-style origin lock |
| Search (Algolia DocSearch) crawler can't reach private build | High | Low | Acceptable — Anvil docs are private, external search not needed; use local search inside the private build |
| Users bookmark `anvil-docs-private.vercel.app` directly | Low | Low | Deployment protection blocks it; user-facing URL is always `docs.eddacraft.ai` |
| Pre-existing BAUTH JWTs don't work after OAuth callback path change | Low | Low | Users re-auth; 7-day expiry means worst case is 7 days of minor friction |
| Two Docusaurus builds drift in theme/config | Medium | Low | Shared `@eddacraft/docs-theme` package (future), acceptable drift in short term |

## Validation

- **Automated:** existing DOCSAUTH test plan (`plans/specs/2026-04-03-...`)
  rerun against the new shell URL. New tests: (a) unauthenticated fetch of
  `/anvil/assets/js/*.js` returns 302, not 200; (b) authenticated navigation
  via `<Link>` clicks on the homepage lands on gated content; (c)
  `/robots.txt` contains `Disallow: /anvil/`; (d) `X-Robots-Tag` present on
  `/anvil/*` responses.
- **Manual clone-risk check:** attempt to extract Anvil content using
  ChatGPT Browse and Perplexity pointed at `docs.eddacraft.ai/anvil/overview`
  without a cookie. Both should see only the GitHub login redirect.
- **Direct-hit check:** `curl https://anvil-docs-private.vercel.app/anvil/overview`
  should return 401/403 (deployment protection), not 200.

## Out of scope (tracked separately)

- Rotate Pulumi stack passphrase (infra hygiene, unrelated).
- Delete stale `TOGGLING-DOCS.md`.
- Unified top nav across public and private docs (future).
- Better-Auth migration (future).
- Per-page ACLs in `/anvil/*` (future, if ever).

## Kill-switch verification (2026-04-11)

- `baseUrl: '/anvil/'` correctly prefixes: HTML routes, JS chunks, CSS, static images, search index, sitemap.
- No assets escape the `/anvil/` prefix in the build output (except the standard root-level files: index.html redirect, robots.txt, sitemap.xml, 404.html, .nojekyll).
- **Verdict: GREEN — proceeding with Next.js shell architecture.**
