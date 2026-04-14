# Docs Shell Landing Page Design

## Goal

Replace the placeholder docs-shell landing page with a properly styled docs
hub aligned to the Nordic Terminal design system used across eddacraft.ai.

## Architecture

The docs-shell (`apps/docs-shell`) is a Next.js 16 proxy layer. Its only
rendered pages are the landing page (`/`) and the `/auth/*` flow. All doc
content is proxied from upstream Docusaurus apps. The landing page is plain
CSS — no Tailwind, no shadcn/ui — using the same design tokens as the website.

## Design Tokens

Replace `globals.css` with the Nordic Terminal palette shared across all
eddacraft apps:

| Token             | Value     | Usage                    |
|-------------------|-----------|--------------------------|
| `--void`          | `#0d0d0f` | Page background          |
| `--structure`     | `#2a2a2e` | Borders, dividers        |
| `--surface`       | `#141416` | Card backgrounds         |
| `--text-primary`  | `#ebebeb` | Primary text             |
| `--text-muted`    | `#85858a` | Secondary text, subtitles|
| `--anvil`         | `#cc5500` | Anvil product accent     |
| `--aps`           | `#64748b` | APS product accent       |
| `--kindling`      | `#c2410c` | Kindling product accent  |

### Typography

- **Headings / nav:** JetBrains Mono, uppercase, `letter-spacing: 0.03em`
- **Body:** Inter, regular weight
- **Border radius:** `0` everywhere (sharp corners)

Fonts loaded via `next/font/google` in `layout.tsx` (same approach as
`apps/website`).

## Layout

### Header

Full-width bar, no background border. Content centred at `max-width: 1200px`.

- **Left:** "eddacraft" wordmark (lowercase, JetBrains Mono, links to
  `https://eddacraft.ai`)
- **Right:** "Blog" link (`/blog`) + "eddacraft.ai" external link
- Small text, uppercase, `--text-muted` colour, `--text-primary` on hover

### Hero

Centred block below header. Compact — no CTA buttons.

- `h1`: "DOCUMENTATION" — JetBrains Mono, uppercase, `clamp(2rem, 5vw, 3rem)`,
  `--text-primary`
- Subtitle: "The forge for governed AI-assisted work." — Inter,
  `--text-muted`, `1.125rem`
- Top padding: `4rem`, bottom: `2rem`

### Product Cards

3-column grid, `max-width: 960px`, centred. `gap: 1.5rem`.
Responsive: single column below `640px`.

Each card:

- Background: `--surface`
- Border: `1px solid var(--structure)`
- Left accent border: `4px solid` product colour
- Border radius: `0`
- Padding: `1.5rem`

Card content:

- **Product name:** JetBrains Mono, uppercase, `--text-primary`
- **Description:** Inter, `--text-muted`, one line
- **Link:** "Read docs >" in product accent colour

Hover state: background shifts to product colour at 5% opacity.

Cards (left to right):

| Product    | Accent      | Description                                          | Link               |
|------------|-------------|------------------------------------------------------|---------------------|
| Anvil      | `--anvil`   | Governed code-gen pipelines for engineering teams.    | `/anvil/overview`   |
| APS        | `--aps`     | Declarative implementation plans for AI-assisted work.| `/aps/overview`     |
| Kindling   | `--kindling` | Observation capture and memory substrate.            | `/kindling/overview`|

### Footer

Centred, small text, `--text-muted`. Generous top margin (`4rem`+).

```
copyright 2026 eddacraft
```

(Lowercase "eddacraft" to match wordmark.)

## Files Changed

| File                              | Change                                           |
|-----------------------------------|--------------------------------------------------|
| `apps/docs-shell/app/globals.css` | Replace with Nordic Terminal tokens + layout CSS  |
| `apps/docs-shell/app/page.tsx`    | Rewrite landing page markup                       |
| `apps/docs-shell/app/layout.tsx`  | Add JetBrains Mono + Inter via `next/font/google` |

No new files. No new dependencies.

## Out of Scope

- Font change across all sites (deferred to a coordinated update)
- edda-stack card (not launched yet — add when ready)
- Blog landing page styling (proxied from docs-public)
- Auth flow page styling (separate concern)
