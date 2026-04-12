# Docs Shell Landing Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder docs-shell landing page with a Nordic Terminal-styled docs hub.

**Architecture:** Three files modified in `apps/docs-shell/app/` — layout gets font loading, globals.css gets the design tokens, page.tsx gets the new markup. Plain CSS, no new dependencies.

**Tech Stack:** Next.js 16, `next/font/google` (JetBrains Mono + Inter), plain CSS custom properties.

---

## File Structure

| File | Responsibility |
|------|---------------|
| `apps/docs-shell/app/layout.tsx` | Root layout — font loading, CSS variables on `<body>`, metadata |
| `apps/docs-shell/app/globals.css` | Nordic Terminal design tokens + all layout/component styles |
| `apps/docs-shell/app/page.tsx` | Landing page markup — header, hero, product cards, footer |

All three files exist and will be fully rewritten. No new files created.

---

### Task 1: Replace globals.css with Nordic Terminal tokens and layout styles

**Files:**
- Modify: `apps/docs-shell/app/globals.css` (full rewrite)

- [ ] **Step 1: Replace globals.css**

```css
/* Nordic Terminal — shared design tokens */
:root {
  --void: #0d0d0f;
  --structure: #2a2a2e;
  --surface: #141416;
  --text-primary: #ebebeb;
  --text-muted: #85858a;

  --anvil: #cc5500;
  --aps: #64748b;
  --kindling: #c2410c;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

html,
body {
  margin: 0;
  padding: 0;
  background: var(--void);
  color: var(--text-primary);
  line-height: 1.6;
}

body {
  font-family: var(--font-sans), system-ui, -apple-system, sans-serif;
}

a {
  color: inherit;
  text-decoration: none;
}

/* ---- Header ---- */

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  max-width: 1200px;
  margin: 0 auto;
  padding: 1.5rem 2rem;
}

.wordmark {
  font-family: var(--font-mono), monospace;
  font-size: 0.875rem;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-primary);
}

.header-nav {
  display: flex;
  gap: 1.5rem;
  align-items: center;
}

.header-link {
  font-family: var(--font-mono), monospace;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
  transition: color 0.15s;
}

.header-link:hover {
  color: var(--text-primary);
}

/* ---- Hero ---- */

.hero {
  padding: 4rem 2rem 2rem;
  text-align: center;
  max-width: 960px;
  margin: 0 auto;
}

.hero h1 {
  font-family: var(--font-mono), monospace;
  font-size: clamp(2rem, 5vw, 3rem);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  margin: 0 0 0.75rem;
}

.hero p {
  font-size: 1.125rem;
  color: var(--text-muted);
  margin: 0;
}

/* ---- Product Cards ---- */

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 1.5rem;
  max-width: 960px;
  margin: 0 auto;
  padding: 2rem;
}

.card {
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--structure);
  border-radius: 0;
  border-left-width: 4px;
  transition: background 0.15s;
}

.card[data-accent="anvil"] {
  border-left-color: var(--anvil);
}

.card[data-accent="aps"] {
  border-left-color: var(--aps);
}

.card[data-accent="kindling"] {
  border-left-color: var(--kindling);
}

.card[data-accent="anvil"]:hover {
  background: rgba(204, 85, 0, 0.05);
}

.card[data-accent="aps"]:hover {
  background: rgba(100, 116, 139, 0.05);
}

.card[data-accent="kindling"]:hover {
  background: rgba(194, 65, 12, 0.05);
}

.card h3 {
  font-family: var(--font-mono), monospace;
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  margin: 0 0 0.5rem;
}

.card p {
  font-size: 0.9375rem;
  color: var(--text-muted);
  margin: 0 0 1rem;
}

.card-link {
  font-family: var(--font-mono), monospace;
  font-size: 0.8125rem;
  letter-spacing: 0.02em;
}

.card[data-accent="anvil"] .card-link {
  color: var(--anvil);
}

.card[data-accent="aps"] .card-link {
  color: var(--aps);
}

.card[data-accent="kindling"] .card-link {
  color: var(--kindling);
}

/* ---- Footer ---- */

.footer {
  text-align: center;
  padding: 4rem 2rem 2rem;
  font-size: 0.75rem;
  color: var(--text-muted);
}

/* ---- Responsive ---- */

@media (max-width: 640px) {
  .cards {
    grid-template-columns: 1fr;
  }

  .header {
    flex-direction: column;
    gap: 1rem;
  }
}
```

- [ ] **Step 2: Verify the file was saved**

Run: `head -5 apps/docs-shell/app/globals.css`
Expected: first 5 lines show the Nordic Terminal comment and `:root` opening.

- [ ] **Step 3: Commit**

```bash
git add apps/docs-shell/app/globals.css
git commit -m "style(docs-shell): replace globals.css with nordic terminal tokens"
```

---

### Task 2: Add font loading to layout.tsx

**Files:**
- Modify: `apps/docs-shell/app/layout.tsx` (full rewrite)

- [ ] **Step 1: Rewrite layout.tsx**

```tsx
import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { JetBrains_Mono, Inter } from 'next/font/google';
import './globals.css';

const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
});

const inter = Inter({
  subsets: ['latin'],
  variable: '--font-sans',
});

export const metadata: Metadata = {
  title: 'eddacraft docs',
  description: 'The forge for governed AI-assisted work',
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body className={`${jetbrainsMono.variable} ${inter.variable}`}>
        {children}
      </body>
    </html>
  );
}
```

- [ ] **Step 2: Verify typecheck passes**

Run: `cd apps/docs-shell && pnpm typecheck`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/docs-shell/app/layout.tsx
git commit -m "feat(docs-shell): add jetbrains mono and inter font loading"
```

---

### Task 3: Rewrite landing page markup

**Files:**
- Modify: `apps/docs-shell/app/page.tsx` (full rewrite)

- [ ] **Step 1: Rewrite page.tsx**

```tsx
export default function HomePage() {
  return (
    <>
      <header className="header">
        <a href="https://eddacraft.ai" className="wordmark">
          eddacraft
        </a>
        <nav className="header-nav">
          <a href="/blog" className="header-link">
            Blog
          </a>
          <a
            href="https://eddacraft.ai"
            className="header-link"
            rel="noopener"
          >
            eddacraft.ai &rarr;
          </a>
        </nav>
      </header>

      <section className="hero">
        <h1>Documentation</h1>
        <p>The forge for governed AI-assisted work.</p>
      </section>

      <section className="cards">
        <a href="/anvil/overview" className="card" data-accent="anvil">
          <h3>Anvil</h3>
          <p>Governed code-gen pipelines for engineering teams.</p>
          <span className="card-link">Read docs &gt;</span>
        </a>

        <a href="/aps/overview" className="card" data-accent="aps">
          <h3>APS</h3>
          <p>Declarative implementation plans for AI-assisted work.</p>
          <span className="card-link">Read docs &gt;</span>
        </a>

        <a href="/kindling/overview" className="card" data-accent="kindling">
          <h3>Kindling</h3>
          <p>Observation capture and memory substrate.</p>
          <span className="card-link">Read docs &gt;</span>
        </a>
      </section>

      <footer className="footer">
        &copy; {new Date().getFullYear()} eddacraft
      </footer>
    </>
  );
}
```

- [ ] **Step 2: Verify typecheck passes**

Run: `cd apps/docs-shell && pnpm typecheck`
Expected: no errors.

- [ ] **Step 3: Verify dev server renders correctly**

Run: `cd apps/docs-shell && pnpm dev`

Open `http://localhost:3100` in a browser. Verify:
- Header shows "eddacraft" wordmark (left) and "Blog" + "eddacraft.ai →" links (right)
- Hero shows "DOCUMENTATION" in uppercase monospace with muted subtitle
- Three product cards with coloured left borders (orange, steel, ember)
- Cards hover state brightens background
- Footer shows "© 2026 eddacraft" centred
- Responsive: cards stack at narrow viewport

- [ ] **Step 4: Commit**

```bash
git add apps/docs-shell/app/page.tsx
git commit -m "feat(docs-shell): rewrite landing page as nordic terminal docs hub"
```

---

## Self-Review

**Spec coverage:**
- [x] Design tokens (void, structure, surface, text-primary, text-muted, product accents) — Task 1
- [x] Typography (JetBrains Mono headings, Inter body, sharp corners) — Task 1 + 2
- [x] Header with wordmark + Blog/eddacraft.ai links — Task 3
- [x] Hero with "DOCUMENTATION" heading + subtitle — Task 3
- [x] Three product cards (Anvil, APS, Kindling) with accent borders and hover — Task 1 + 3
- [x] Footer with lowercase "eddacraft" — Task 3
- [x] Responsive single-column below 640px — Task 1
- [x] No new dependencies — confirmed (next/font/google is built-in)
- [x] Out of scope items (edda-stack, blog styling, auth pages, font change) not included

**Placeholder scan:** No TBD, TODO, or vague steps found.

**Type consistency:** `--font-mono` and `--font-sans` CSS variables set in layout.tsx, referenced in globals.css. Class names match between page.tsx and globals.css (`header`, `wordmark`, `header-nav`, `header-link`, `hero`, `cards`, `card`, `card-link`, `footer`). `data-accent` attribute values match CSS selectors (`anvil`, `aps`, `kindling`).
