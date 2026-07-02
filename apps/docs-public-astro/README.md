# docs-public-astro (SPIKE — not for production)

A throwaway **Astro 7 + Starlight 0.41** rebuild of
[`apps/docs-public`](../docs-public) (Docusaurus), scoped to the **APS section
only**, built to de-risk a full Docusaurus → Astro migration before committing
to it. Sits alongside the Docusaurus app so the two can be diffed directly.

> Migration context and the Path A vs Path B decision live in the PR
> description. This README is the "what the spike proves" record.

## What this proves

Built green on Astro `7.0.2` / Starlight `0.41.0` (Rust compiler + Rust Markdown
pipeline are the stable default in 7), 9 APS pages + Pagefind index in ~2.4s:

1. **Content portability** — `docs/public/aps` is symlinked into
   `src/content/docs/aps`, so content stays **canonically authored under
   `docs/public/`** (owned by the DSITE/DOCSYNC APS modules), unchanged. The
   existing Docusaurus frontmatter (`id`, `sidebar_position`, `title`,
   `description`) is accepted as-is — Starlight only requires `title` and
   ignores the rest. Zero content edits.
2. **Routing parity** — pages emit at `/aps/<...>/` directory URLs, matching the
   prefix the shell proxy rewrites to this upstream.
3. **The asset-path sharp edge is handled** — `build.assets: 'assets'`
   (`astro.config.mjs`) forces hashed assets under `/assets/` instead of Astro's
   default `/_astro/`, so they fall under a prefix the shell proxy matcher
   already forwards. Verified: `dist/` has no `_astro/`, and built pages
   reference `/assets/*.css|js`.
4. **Search, scoped to public content** — Pagefind builds from this app's output
   only. Because the app contains public APS content alone, the index is
   intrinsically public — the Path A isolation property. (Docusaurus shipped
   **no** search; this is a net upgrade.) Required one additive change to the
   shell: `/pagefind/:path*` added to the proxy matcher in
   `apps/docs-shell/proxy.ts`.
5. **Auth contract survives untouched** — `middleware.ts` is carried over
   verbatim from `docs-public`; it's a Vercel routing middleware, framework-
   agnostic, so the `x-docs-upstream-secret` gate is unaffected by the framework
   swap.

## What this deliberately does NOT cover

- **Other sections** — `kindling`, `edda-stack`, the `start-here`/`beta` trees.
  Same mechanism (symlink + sidebar), not yet wired.
- **Blog + RSS/Atom** — the real feature gap. Docusaurus gives blog + RSS + Atom
  for free; Starlight has no native blog. Needs the community Starlight Blog
  plugin + `@astrojs/rss` (Atom is manual). Decision deferred.
- **Theme parity** — uses Starlight defaults. Navbar/footer/custom CSS from
  `docs-public` not reproduced.
- **The private app** (`anvil-docs-private`, the gated `/anvil/*` upstream) —
  out of scope; Path A migrates it the same way as a second step.
- **`docs:check` / DOCGOV** — unchanged; it globs `docs/**`, framework-agnostic.

## Run it

```sh
pnpm --filter @eddacraft/docs-public-astro dev     # local dev server
pnpm --filter @eddacraft/docs-public-astro build   # static build + pagefind index → dist/
```
