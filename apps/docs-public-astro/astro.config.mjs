// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// SPIKE: Astro 7 + Starlight rebuild of apps/docs-public (APS section only).
//
// This config exists to prove the docs-shell proxy contract end-to-end, not to
// ship. The shell (apps/docs-shell/proxy.ts) is a path-prefix auth proxy in
// front of docs.eddacraft.ai: it forwards an x-docs-upstream-secret header and
// only proxies a fixed set of path prefixes. Two settings below exist purely to
// satisfy that contract — see the inline notes.
export default defineConfig({
  site: 'https://docs.eddacraft.ai',

  build: {
    // PROXY CONTRACT (the sharp edge from the migration review): Astro emits
    // hashed assets to `/_astro/` by default, which the shell proxy matcher
    // does NOT forward (it lists `/assets/:path*` and `/img/:path*`). Left at
    // the default, CSS/JS would 404 in production while looking fine locally.
    // Emitting to `assets/` lands them under the proxied `/assets/` prefix.
    assets: 'assets',
    // Docusaurus served directory-style URLs (/aps/overview/); keep that so the
    // proxied paths and any existing inbound links don't redirect-loop.
    format: 'directory',
  },

  // The shell rewrites `/aps/*` to this upstream; cross-section links like
  // `/kindling/*` resolve only at runtime through the proxy, so don't fail the
  // build on them (Docusaurus used `onBrokenLinks: 'log'` for the same reason).
  trailingSlash: 'ignore',

  integrations: [
    starlight({
      title: 'eddacraft',
      // Pagefind is on by default and indexes ONLY this app's built output.
      // Because this app contains public APS content only, the index is
      // intrinsically public — the Path A isolation property the review called
      // out. (Search still needs `/pagefind/:path*` added to the shell matcher;
      // done in apps/docs-shell/proxy.ts in this spike.)
      pagefind: true,
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/eddacraft' }],
      // Mirrors apps/docs-public/sidebars/aps.ts. Content lives canonically in
      // docs/public/aps and is symlinked into src/content/docs/aps.
      sidebar: [
        { label: 'Overview', slug: 'aps/overview' },
        {
          label: 'Specification',
          items: ['aps/spec/taxonomy', 'aps/spec/file-layout', 'aps/spec/determinism'],
        },
        {
          label: 'Schemas',
          collapsed: true,
          items: ['aps/schemas/json-schema', 'aps/schemas/examples'],
        },
        {
          label: 'Examples',
          items: ['aps/examples/minimal-plan', 'aps/examples/multi-module'],
        },
        {
          label: 'Tooling',
          collapsed: true,
          items: ['aps/tooling/validation'],
        },
      ],
    }),
  ],
});
