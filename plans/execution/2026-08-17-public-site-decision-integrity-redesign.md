# Public Site Decision Integrity Redesign Implementation Plan

**Goal:** Redesign eddacraft.ai around the shipping anvil control point and its honest progression into Decision Integrity.
**Architecture:** The existing Next.js landing page remains product-led and composes focused server-rendered sections. Interactive behaviour stays isolated to the existing early-access dialog, terminal demonstration and restrained diagram motion; copy and claim boundaries are guarded by a deterministic validation script.
**Tech Stack:** Next.js 16, React 19, TypeScript 6, Tailwind CSS 4, Radix UI, Node.js 24, Nx, Vercel

---

## File map

- Modify: apps/website/app/page.tsx — ordered landing-page composition.
- Modify: apps/website/app/layout.tsx — IBM Plex Sans, canonical metadata and social metadata.
- Modify: apps/website/app/globals.css — canonical brand tokens, diagram primitives and reduced motion.
- Modify: apps/website/app/opengraph-image.tsx — new public positioning social image.
- Modify: apps/website/app/twitter-image.tsx — new public positioning social card.
- Modify: apps/website/components/navbar.tsx — company/product hierarchy and section links.
- Modify: apps/website/components/hero-section.tsx — current-product hero and unchanged access flow.
- Modify: apps/website/components/terminal-window.tsx — current, verifiable decision sequence.
- Delete: apps/website/components/feature-grid.tsx — replaced by the four-stage product architecture.
- Create: apps/website/components/shipping-proof.tsx — current proof strip.
- Create: apps/website/components/trust-gap.tsx — category problem and current-to-destination bridge.
- Create: apps/website/components/decision-integrity-flywheel.tsx — accessible responsive flywheel.
- Create: apps/website/components/product-stages.tsx — Understand, Build, Decide and Learn.
- Create: apps/website/components/delivery-boundary.tsx — single public current/future boundary.
- Create: apps/website/components/decision-model.tsx — intent/evidence/policy kernel model and independence.
- Create: apps/website/components/company-band.tsx — eddacraft company mission.
- Modify: apps/website/components/cli-footer.tsx — conversion copy only; waitlist behaviour unchanged.
- Modify: apps/website/package.json — positioning contract command.
- Create: apps/website/scripts/check-positioning.mjs — required/retired copy and lowercase-brand contract.

### Task 1: Add a failing public-message contract

**Files:**
- Create: apps/website/scripts/check-positioning.mjs
- Modify: apps/website/package.json

- [ ] Add a Node check over page.tsx, layout.tsx and all marketing components.
- [ ] Require the approved hero, bridge and company phrases.
- [ ] Reject capitalised brand names and the old “FORCE PROBABILISTIC TOOLS” hero.
- [ ] Add “test:positioning”: “node scripts/check-positioning.mjs”.
- [ ] Run pnpm --dir apps/website test:positioning and verify failure.
- [ ] Commit: git commit -m "test(website): add positioning contract"

Contract implementation:

~~~js
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('..', import.meta.url);
const componentsDir = new URL('../components/', import.meta.url);
const files = [
  new URL('../app/page.tsx', import.meta.url),
  new URL('../app/layout.tsx', import.meta.url),
  ...readdirSync(componentsDir)
    .filter((name) => name.endsWith('.tsx'))
    .map((name) => new URL('../components/' + name, import.meta.url)),
];
const content = files.map((file) => readFileSync(file, 'utf8')).join('\n');

const required = [
  'TRUST THE CODE',
  'PROTECTION IS THE ENTRY POINT',
  'DECISION INTEGRITY IS THE SYSTEM AROUND IT',
  'TRUST INFRASTRUCTURE',
];

const forbidden = [
  'FORCE PROBABILISTIC TOOLS',
  'Anvil',
  'EddaCraft',
  'THE LOOP IS ALREADY RUNNING',
];

const failures = [
  ...required.filter((value) => !content.includes(value)).map((value) => 'missing: ' + value),
  ...forbidden.filter((value) => content.includes(value)).map((value) => 'retired: ' + value),
];

if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}

console.log('website positioning contract: ok');
~~~

### Task 2: Align the brand foundation and metadata

**Files:**
- Modify: apps/website/app/globals.css
- Modify: apps/website/app/layout.tsx

- [ ] Add the canonical off-white, ghost-grey, brick-red, dull-amber and border-strong aliases.
- [ ] Remove unused shadcn colour values from the active marketing surface or remap them to approved tokens.
- [ ] Replace Inter with IBM Plex Sans while retaining JetBrains Mono.
- [ ] Add global focus-visible and prefers-reduced-motion rules.
- [ ] Update title, description, Open Graph and Twitter copy to the approved current-product position.
- [ ] Run pnpm nx build website.
- [ ] Commit: git commit -m "style(website): align canonical brand foundation"

Required font stacks:

~~~css
--font-sans: 'IBM Plex Sans', ui-sans-serif, system-ui, sans-serif;
--font-mono: 'JetBrains Mono', 'IBM Plex Mono', ui-monospace, monospace;
~~~

Required motion fallback:

~~~css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
~~~

### Task 3: Rebuild the current-product opening

**Files:**
- Modify: apps/website/components/navbar.tsx
- Modify: apps/website/components/hero-section.tsx
- Modify: apps/website/components/terminal-window.tsx
- Create: apps/website/components/shipping-proof.tsx

- [ ] Keep the early-access request and install-unlock logic byte-for-byte where practical.
- [ ] Make the navigation show eddacraft as company and anvil as active product.
- [ ] Implement the approved current-product hero.
- [ ] Change the terminal sequence to current interception, context, policy, verdict and present receipt/provenance behaviour.
- [ ] Populate the proof strip only after checking release, integrations and benchmark sources in this repository.
- [ ] Run pnpm --dir apps/website test:positioning; expected failure only for unbuilt lower sections.
- [ ] Run pnpm nx build website.
- [ ] Commit: git commit -m "feat(website): rebuild current product opening"

Hero content:

~~~text
// GENERATION_TIME_TRUST
TRUST THE CODE
YOUR AI WRITES.

anvil is the independent, deterministic control point
for AI-assisted software engineering.

Understand the change. Apply your standards.
Stop unsafe work before it reaches review.
~~~

### Task 4: Build the category bridge and flywheel

**Files:**
- Create: apps/website/components/trust-gap.tsx
- Create: apps/website/components/decision-integrity-flywheel.tsx

- [ ] Implement the trust-gap distinction between logs, evidence, policy and receipts.
- [ ] Implement the approved “Protection is the entry point” bridge.
- [ ] Render Understand → Build → Decide → Learn in semantic reading order.
- [ ] Use SVG only for connective paths and arrows.
- [ ] Use solid ember for operating foundations and Structure-grey for the system being completed.
- [ ] Include visible legend and screen-reader summary.
- [ ] Render a vertical sequence below the desktop breakpoint.
- [ ] Verify the diagram with CSS and animation disabled.
- [ ] Commit: git commit -m "feat(website): add decision integrity flywheel"

### Task 5: Replace the feature grid with the system model

**Files:**
- Delete: apps/website/components/feature-grid.tsx
- Create: apps/website/components/product-stages.tsx
- Create: apps/website/components/delivery-boundary.tsx
- Create: apps/website/components/decision-model.tsx
- Create: apps/website/components/company-band.tsx

- [ ] Implement Understand, Build, Decide and Learn as one structural section.
- [ ] Render current capabilities normally and directional capabilities muted.
- [ ] Add the single “control point ships today / trust chain comes next” boundary.
- [ ] Populate the current column strictly from anvil-001.
- [ ] Frame future capabilities as the system being completed.
- [ ] Add the deterministic decision model and independence argument.
- [ ] Add the brief eddacraft company band.
- [ ] Run pnpm --dir apps/website test:positioning; expect success.
- [ ] Commit: git commit -m "feat(website): add decision integrity system sections"

### Task 6: Compose the landing page and conversion path

**Files:**
- Modify: apps/website/app/page.tsx
- Modify: apps/website/components/cli-footer.tsx

- [ ] Compose sections in this order: navbar, hero, shipping proof, trust gap, bridge/flywheel, product stages, delivery boundary, decision model, company band, conversion footer.
- [ ] Update navigation anchors to stable section IDs.
- [ ] Update conversion copy without changing the waitlist request, state machine or analytics.
- [ ] Verify keyboard focus after smooth-scroll actions.
- [ ] Run pnpm nx build website.
- [ ] Commit: git commit -m "feat(website): compose decision integrity narrative"

### Task 7: Update generated social surfaces

**Files:**
- Modify: apps/website/app/opengraph-image.tsx
- Modify: apps/website/app/twitter-image.tsx

- [ ] Use current-product copy, not the target-system claim.
- [ ] Use only Void, Structure, Off-White and anvil ember.
- [ ] Preserve edge-runtime compatibility.
- [ ] Verify generated image routes during local production serving.
- [ ] Commit: git commit -m "feat(website): update decision integrity social cards"

### Task 8: Full verification

**Files:**
- Review all files listed above.

- [ ] Run pnpm --dir apps/website test:positioning.
- [ ] Run pnpm nx build website.
- [ ] Run pnpm exec eslint apps/website.
- [ ] Run the repository changed-file validation command required by current anvil-001 guidance.
- [ ] Inspect 360px, 768px, 1280px and 1536px layouts.
- [ ] Verify keyboard operation, focus visibility and reduced motion.
- [ ] Verify diagrams remain understandable without colour.
- [ ] Verify the install unlock, request-access and waitlist flows.
- [ ] Confirm no NOW claim lacks a repository source.
- [ ] Run the brand-and-design governance diff.
- [ ] Commit corrections: git commit -m "fix(website): complete redesign verification"

Expected core results:

~~~text
website positioning contract: ok
website:build completed successfully
~~~
