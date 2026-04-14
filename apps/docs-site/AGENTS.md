# Docs Site - AI Agent Instructions

> **Docusaurus 3.9 documentation hub for the eddacraft product suite**

## Overview

This is the documentation site at `eddacraft.dev`, built with **Docusaurus
3.9**. It hosts documentation for multiple products using **multi-instance
docs** — each product (Anvil, APS, Kindling, Edda Stack) has its own docs plugin
instance with independent routing and sidebars.

This is _not_ the marketing site (that lives in `apps/website`).

## Structure

```
apps/docs-site/
├── docs/                         # All documentation content
│   ├── start-here/               # Entry point (what-is-eddacraft, choose-your-path, glossary)
│   ├── anvil/                    # Anvil product docs (concepts, guides, integrations, operations)
│   ├── aps/                      # APS specification docs (spec, schemas, examples, tooling)
│   ├── kindling/                 # Kindling memory capture docs (concepts, adapters, commands)
│   └── edda-stack/               # Edda Stack roadmap docs (components, principles)
├── blog/                         # Blog posts
├── sidebars/                     # Sidebar configs (one per product)
│   ├── start-here.ts
│   ├── anvil.ts
│   ├── aps.ts
│   ├── kindling.ts
│   └── edda-stack.ts
├── src/
│   ├── pages/
│   │   ├── index.tsx             # Custom homepage (product tiles, value props, quick links)
│   │   └── index.module.css      # Homepage styles
│   ├── css/
│   │   └── custom.css            # Nordic Terminal theme (all global styling)
│   └── data/
│       └── changelog.json        # Release notes data
├── static/img/                   # Logos, favicons, social card
├── docusaurus.config.ts          # Main configuration
├── project.json                  # Nx project config
├── tsconfig.json                 # TypeScript config (editor only)
└── TOGGLING-DOCS.md              # Guide for enabling/disabling doc sections
```

## Commands

```bash
# Development
nx start docs-site               # Dev server (port 3000)
nx build docs-site                # Static build to /build

# Or from this directory
pnpm start
pnpm build
pnpm serve                       # Serve pre-built static content
pnpm clear                       # Clear Docusaurus cache
pnpm typecheck                   # TypeScript validation
```

## Multi-Instance Docs Architecture

Each product has its own `@docusaurus/plugin-content-docs` instance. This gives
each section independent routing, sidebars, and (future) versioning.

| Instance   | Content path (plugin `path`)   | URL route     | Sidebar config           |
| ---------- | ------------------------------ | ------------- | ------------------------ |
| start-here | `../../docs/public/start-here` | `/start-here` | `sidebars/start-here.ts` |
| anvil      | `../../docs/public/anvil`      | `/anvil`      | `sidebars/anvil.ts`      |
| aps        | `../../docs/public/aps`        | `/aps`        | `sidebars/aps.ts`        |
| kindling   | `../../docs/public/kindling`   | `/kindling`   | `sidebars/kindling.ts`   |
| edda-stack | `../../docs/public/edda-stack` | `/edda-stack` | `sidebars/edda-stack.ts` |

**To add a new doc section**, see `TOGGLING-DOCS.md` or:

1. Add a new plugin instance in `docusaurus.config.ts`
2. Create the sidebar file in `sidebars/`
3. Create the content directory under `docs/`
4. Add navbar and footer links

**To toggle sections off** (e.g., during development):

```bash
DOCS_KINDLING=false DOCS_EDDA_STACK=false pnpm start
```

## Writing Documentation

Each markdown file uses Docusaurus front-matter:

```yaml
---
id: unique-identifier
title: Display Title
description: Short summary for SEO
sidebar_position: 1
---
```

- Content lives in `docs/<product>/` directories
- Sidebar ordering is controlled by `sidebar_position` in front-matter and the
  sidebar config files in `sidebars/`
- Cross-product links use full paths: `/anvil/overview`
- Broken links will **fail the build** (configured to `throw`)

## Styling — Nordic Terminal Theme

All styling is in `src/css/custom.css` using CSS custom properties. There is
**no Tailwind** in this app — it is pure CSS.

**Core colours (same palette as website):**

| Token               | Value     | Usage             |
| ------------------- | --------- | ----------------- |
| `--ec-void`         | `#0d0d0f` | Page background   |
| `--ec-surface`      | `#141416` | Card/container bg |
| `--ec-structure`    | `#2a2a2e` | Borders, dividers |
| `--ec-text-primary` | `#ebebeb` | Primary text      |
| `--ec-text-muted`   | `#85858a` | Secondary text    |

**Product accent colours:**

| Product  | Colour    | Token           |
| -------- | --------- | --------------- |
| Anvil    | `#cc5500` | `--ec-anvil`    |
| APS      | `#64748b` | `--ec-aps`      |
| Kindling | `#c2410c` | `--ec-kindling` |
| Edda     | `#2e8b57` | `--ec-edda`     |

**Design rules:**

- Sharp corners (0px border-radius)
- Dark mode by default
- Elevation through borders, not shadows
- Monospace font for code and headings (JetBrains Mono)
- Inter for body text
- Product tiles have a 4px left-side coloured border

## Homepage

The homepage (`src/pages/index.tsx`) is a custom React component, not a markdown
doc. It includes:

- **Product tiles** — cards for each product with status badges
  (available/coming-soon) and product-specific accent colours
- **Value proposition cards** — four key selling points
- **Quick links** — categorised links to key documentation pages

Styles are in `src/pages/index.module.css`.

## Configuration Reference

| File                      | Purpose                                        |
| ------------------------- | ---------------------------------------------- |
| `docusaurus.config.ts`    | Site metadata, plugins, navbar, footer, themes |
| `sidebars/*.ts`           | Sidebar structure per product                  |
| `src/css/custom.css`      | All global styling and theme tokens            |
| `src/data/changelog.json` | Structured release notes and roadmap           |
| `project.json`            | Nx project config (name: `docs-site`)          |
| `TOGGLING-DOCS.md`        | Guide for enabling/disabling doc sections      |

## Key Differences from apps/website

| Aspect    | docs-site                  | website                    |
| --------- | -------------------------- | -------------------------- |
| Framework | Docusaurus 3.9             | Next.js 16                 |
| Content   | Markdown docs              | React components           |
| Styling   | Pure CSS custom properties | Tailwind CSS 4 + shadcn/ui |
| Output    | Static HTML                | SSR/SSG hybrid             |
| Purpose   | Product documentation      | Marketing landing page     |
