# Website - AI Agent Instructions

> **Next.js 16 marketing site for anvil, deployed on Vercel**

## Overview

This is the public-facing marketing and landing page for anvil at
`eddacraft.ai`. It is a **Next.js 16** application using the **App Router**,
**React 19**, **Tailwind CSS 4**, and **Radix UI** primitives. The design
follows a **Nordic Terminal** aesthetic — dark, monospace, sharp corners.

This is _not_ the documentation site (that lives at `docs.eddacraft.ai` via
`apps/docs-shell`).

For waitlist email operations and resend testing, see:
`docs/guides/waitlist-email-operations.md`.

## Structure

```
apps/website/
├── app/                        # Next.js App Router
│   ├── layout.tsx              # Root layout (metadata, fonts, analytics)
│   ├── page.tsx                # Landing page
│   ├── globals.css             # Tailwind v4 theme + custom properties
│   ├── privacy/page.tsx        # Privacy policy (man page style)
│   ├── security/page.tsx       # Security docs (man page style)
│   ├── opengraph-image.tsx     # Dynamic OG image (edge runtime)
│   └── twitter-image.tsx       # Dynamic Twitter card (edge runtime)
├── components/
│   ├── navbar.tsx              # Fixed navigation bar
│   ├── hero-section.tsx        # Hero with CTAs and modal
│   ├── feature-grid.tsx        # 6-feature grid with integrations bar
│   ├── cli-footer.tsx          # Interactive email waitlist CLI
│   ├── terminal-window.tsx     # Animated terminal demo
│   └── theme-provider.tsx      # next-themes wrapper
├── lib/
│   └── utils.ts                # cn() helper (clsx + tailwind-merge)
├── public/                     # Static assets (icons, logos, images)
├── components.json             # shadcn/ui configuration
├── next.config.mjs             # Next.js config
├── postcss.config.mjs          # PostCSS (Tailwind v4)
├── project.json                # Nx project config
└── tsconfig.json               # TypeScript config
```

## Commands

```bash
# Development
nx dev website                   # Dev server
nx build website                 # Production build
nx start website                 # Serve production build

# Or from this directory
pnpm dev
pnpm build
pnpm start
```

## Tech Stack

| Layer      | Technology                                             |
| ---------- | ------------------------------------------------------ |
| Framework  | Next.js 16 (App Router, React 19)                      |
| Styling    | Tailwind CSS 4 (`@tailwindcss/postcss`), CSS variables |
| Components | Radix UI primitives, shadcn/ui configured (new-york)   |
| Icons      | Lucide React                                           |
| Forms      | React Hook Form + Zod 4                                |
| Analytics  | Vercel Analytics                                       |
| Fonts      | Inter (sans), JetBrains Mono (mono) via Google Fonts   |
| Deployment | Vercel (static-friendly, edge OG images)               |

## Design System — Nordic Terminal Theme

All styling is defined via CSS custom properties in `app/globals.css`. There is
no separate `tailwind.config` file — Tailwind v4 uses the CSS file directly.

**Core colours:**

| Token            | Value     | Usage             |
| ---------------- | --------- | ----------------- |
| `--void`         | `#0d0d0f` | Page background   |
| `--surface`      | `#141416` | Card/container bg |
| `--structure`    | `#2a2a2e` | Borders, dividers |
| `--text-primary` | `#ebebeb` | Primary text      |
| `--text-muted`   | `#85858a` | Secondary text    |
| `--anvil`        | `#cc5500` | Accent orange     |
| `--edda`         | `#2e8b57` | Accent green      |

**Design rules:**

- Sharp corners everywhere (0px border-radius)
- `font-mono` for CLI/code elements (JetBrains Mono)
- `font-sans` for body text (Inter)
- Dark mode is the default and primary experience
- Elevation through borders, not shadows

## Component Patterns

All interactive components use `'use client'` directive. Key patterns:

- **Typewriter effects** — terminal-window and cli-footer use
  character-by-character animation via `setTimeout` chains
- **Smooth scroll anchors** — hero CTA scrolls to `#waitlist` in the footer
- **Modal dialogs** — docs modal and pre-release notice use state toggles
- **Man page format** — privacy and security pages use Unix manual page styling
  (NAME, SYNOPSIS, DESCRIPTION sections)

## Conventions

- **Path alias**: `@/*` maps to the app root (e.g., `@/components/navbar`)
- **shadcn/ui ready**: `components.json` is configured but components live in
  `components/` directly (not `components/ui/` yet)
- **Server Components by default**: only add `'use client'` when needed
- **Static-friendly**: `images.unoptimized: true` in next.config for static
  export
- **TypeScript build errors ignored**: `typescript.ignoreBuildErrors: true` in
  next.config (Nx handles type checking separately)

## Metadata

```
Title: "anvil — Guardrails that ship with your code"
URL: https://eddacraft.ai
Locale: en_GB
OG: Site name "anvil by eddacraft", type "website"
Twitter: @eddacraft, summary_large_image
```

## Branding

- Product name is **anvil** (lowercase), never "Anvil" — except in all-caps
  contexts where **ANVIL** is acceptable
- Company name is **eddacraft** (lowercase), never "EddaCraft"
- Sentence-initial lowercase is intentional (e.g. "anvil enforces policy…") — do
  not capitalise to "correct" grammar
