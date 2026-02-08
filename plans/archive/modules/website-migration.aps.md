# Website Migration

> **Module ID:** WEB
> **Status:** Complete
> **Release:** v1.2
> **Dependencies:** monorepo-migration (MONO)

## Overview

Migrate the EddaCraft landing page from the standalone `EddaCraft/eddacraft-landing-page`
repository into `apps/website/` in the anvil monorepo. The v0 repo stays alive as a
design sandbox; the monorepo copy becomes the production path forward.

**Why now:** The monorepo migration (MONO) is complete and `apps/website/` already exists
as a placeholder. The landing page is a self-contained Next.js 16 app — no shared code
with anvil packages — so migration is a copy-and-adapt operation.

## Current State

```
EddaCraft/eddacraft-landing-page (standalone repo)
├── app/                  # Next.js App Router pages
│   ├── globals.css       # Tailwind v4 + Nordic terminal theme
│   ├── layout.tsx        # Root layout (JetBrains Mono + Inter, Vercel Analytics)
│   ├── page.tsx           # Home (Navbar, Hero, FeatureGrid, CLIFooter)
│   ├── opengraph-image.tsx # Dynamic OG image (edge runtime)
│   ├── twitter-image.tsx
│   ├── privacy/page.tsx   # Privacy policy (man-page style)
│   └── security/page.tsx
├── components/            # 6 custom components (navbar, hero, features, footer, terminal, theme)
├── components.json        # shadcn/ui config (new-york, @/ aliases)
├── lib/utils.ts           # cn() utility
├── public/                # Static assets (icons, brand SVGs, OG image)
├── styles/globals.css     # Default shadcn globals (unused duplicate)
├── next.config.mjs        # ignoreBuildErrors, unoptimized images
├── postcss.config.mjs     # @tailwindcss/postcss
├── tsconfig.json          # Standard Next.js config with @/ paths
└── package.json           # Next.js 16, React 19, shadcn/ui, Radix, Tailwind v4
```

**v0 connection:** Auto-syncs from [v0.app/chat/hawD8UZx6KC](https://v0.app/chat/hawD8UZx6KC)
to the standalone repo. Vercel deploys from that repo at `v0-anvil-landing-page`.

```
apps/website/ (monorepo — placeholder)
└── README.md              # "Status: Placeholder for v1.1+"
```

## Target State

```
apps/website/              # Fully populated Next.js 16 app
├── app/                   # Unchanged from source
├── components/            # Unchanged from source
├── components.json        # Unchanged (preserves v0/shadcn compat)
├── lib/                   # Unchanged
├── public/                # Unchanged
├── styles/                # Unchanged
├── next.config.mjs        # Unchanged
├── postcss.config.mjs     # Unchanged
├── tsconfig.json          # Unchanged (self-contained, not extending base)
├── package.json           # Renamed to @eddacraft/anvil-website
└── project.json           # Nx project config (new)
```

## Boundaries

### In Scope

- Copying source files from standalone repo into `apps/website/`
- Renaming package to `@eddacraft/anvil-website`
- Adding Nx project configuration
- Installing dependencies via monorepo root
- Verifying dev server and build work

### Out of Scope

- Changing the internal structure of the Next.js app
- Modifying component code, styles, or theme
- Removing unused shadcn dependencies
- Setting up CI/CD or Vercel deployment from monorepo
- Decommissioning the v0 repo
- Fixing TypeScript errors (ignoreBuildErrors stays)

## Constraints

- **v0 compatibility:** Internal structure must stay identical to v0's expected layout
  (`@/` path aliases, `components.json` at project root, standard Next.js App Router)
  so v0-generated components can be pasted in without modification
- **Self-contained tsconfig:** Must NOT extend `tsconfig.base.json` — Next.js requires
  `jsx: "preserve"`, `moduleResolution: "bundler"`, and the `"next"` plugin, which
  conflict with the monorepo base config
- **No shared dependencies:** The website has no imports from `@eddacraft/anvil-*`
  packages and none should be introduced in this migration

## Tasks

### WEB-001: Copy source files into apps/website/

- **Intent:** Populate the placeholder with the landing page source
- **Expected Outcome:** All source files from the standalone repo exist in `apps/website/`
- **Validation:** `ls apps/website/app/page.tsx apps/website/components.json apps/website/package.json`
- **Confidence:** high
- **Non-scope:** `.git/`, `node_modules/`, `pnpm-lock.yaml`, `.next/`, `.gitignore`, `README.md`

### WEB-002: Adapt package.json for monorepo conventions

- **Intent:** Rename package to follow monorepo naming convention
- **Expected Outcome:** Package name is `@eddacraft/anvil-website`, private, scripts preserved
- **Validation:** `grep '"@eddacraft/anvil-website"' apps/website/package.json`
- **Confidence:** high
- **Non-scope:** Dependencies, scripts, version — all stay as-is

### WEB-003: Add Nx project configuration

- **Intent:** Wire website into Nx so it can be targeted via `nx run`
- **Expected Outcome:** `project.json` exists with dev, build, start targets
- **Validation:** `nx show project anvil-website`
- **Confidence:** high

### WEB-004: Install dependencies and verify workspace recognition

- **Intent:** Monorepo lockfile includes website dependencies
- **Expected Outcome:** `pnpm install` succeeds, website package is discoverable
- **Validation:** `pnpm -F @eddacraft/anvil-website exec -- echo "found"`
- **Confidence:** high

### WEB-005: Smoke test dev and build

- **Intent:** Website works in the monorepo context
- **Expected Outcome:** `next dev` starts, `next build` completes
- **Validation:** `pnpm -F @eddacraft/anvil-website build`
- **Confidence:** medium
- **Risks:** Next.js 16 + React 19 may conflict with monorepo-level TypeScript or Node version constraints

### WEB-006: Commit migration

- **Intent:** Record the migration as a single atomic commit
- **Expected Outcome:** Clean commit with all website files staged
- **Validation:** `git log --oneline -1` shows website migration commit
- **Confidence:** high
- **Dependencies:** WEB-001, WEB-002, WEB-003, WEB-004, WEB-005

## v0 Workflow (Post-Migration)

1. Continue designing in v0.app — changes push to the standalone repo as before
2. When v0 produces changes worth keeping, copy changed files into `apps/website/`
3. When ready to go live from the monorepo, point the domain to a new Vercel project
4. Archive the standalone repo

## Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Next.js 16 conflicts with monorepo Node/TS version | Medium | Low | Self-contained tsconfig, ignoreBuildErrors |
| pnpm hoisting issues with Next.js/React 19 | Medium | Low | Test install, add shamefully-hoist if needed |
| v0 auto-sync breaks | None | N/A | We're not touching the standalone repo |
| Unused Radix deps bloat install | Low | Certain | Intentional — cleanup is future work |
