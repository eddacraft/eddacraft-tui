import { defineCollection } from 'astro:content';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

// Starlight's Content Layer collection. docsLoader() globs src/content/docs,
// where docs/public/aps is symlinked in as `aps/` — so the canonical content
// stays authored under docs/public/ (owned by APS module DSITE), unchanged.
//
// Docusaurus-specific frontmatter keys (`id`, `sidebar_position`) on those
// files are simply ignored by docsSchema(); Starlight only requires `title`.
export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
};
