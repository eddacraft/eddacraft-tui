# Toggling Documentation Sections

This document explains how to enable/disable specific documentation sections in
the eddacraft Docusaurus site.

## Architecture

The site uses Docusaurus multi-instance docs. Each product section is a separate
plugin:

```
plugins: [
  { id: 'start-here', path: 'docs/start-here' },
  { id: 'anvil', path: 'docs/anvil' },
  { id: 'aps', path: 'docs/aps' },
  { id: 'kindling', path: 'docs/kindling' },
  { id: 'edda-stack', path: 'docs/edda-stack' },
]
```

This means each section can be independently enabled or disabled.

## To Disable a Section

### Step 1: Comment Out the Plugin

In `apps/docs-site/docusaurus.config.ts`, find the plugins array and comment out
the section:

```typescript
plugins: [
    // Keep these
    ['@docusaurus/plugin-content-docs', { id: 'start-here', /* ... */ }],
    ['@docusaurus/plugin-content-docs', { id: 'anvil', /* ... */ }],
    ['@docusaurus/plugin-content-docs', { id: 'aps', /* ... */ }],

    // DISABLED: Kindling docs hidden until 0.4.0 (Edda Stack)
    // ['@docusaurus/plugin-content-docs', { id: 'kindling', /* ... */ }],

    // DISABLED: Edda Stack is placeholder only
    // ['@docusaurus/plugin-content-docs', { id: 'edda-stack', /* ... */ }],
],
```

### Step 2: Remove from Navbar

In the same file, find `themeConfig.navbar.items` and remove/comment the
Products dropdown entry:

```typescript
{
  type: 'dropdown',
  label: 'Products',
  items: [
    { label: 'anvil', to: '/anvil/overview' },
    { label: 'APS', to: '/aps/overview' },
    // { label: 'Kindling', to: '/kindling/overview' },
    // { label: 'Edda Stack', to: '/edda-stack/overview' },
  ],
},
```

### Step 3: Remove from Footer

In `themeConfig.footer.links`, remove the footer entries:

```typescript
{
  title: 'Products',
  items: [
    { label: 'anvil', to: '/anvil/overview' },
    { label: 'APS', to: '/aps/overview' },
    // { label: 'Kindling', to: '/kindling/overview' },
    // { label: 'Edda Stack', to: '/edda-stack/overview' },
  ],
},
```

### Step 4: Update Homepage (Optional)

In `apps/docs-site/src/pages/index.tsx`, you may want to remove or mark the
product tile:

```tsx
// Option A: Remove entirely
// <ProductTile title="Kindling" ... />

// Option B: Mark as coming soon
<ProductTile
  title="Kindling"
  status="coming-soon"
  ...
/>
```

### Step 5: Fix Cross-Links

Search for links to the disabled section and either:

- Remove them
- Redirect to a placeholder
- Add conditional rendering

```bash
# Find all links to kindling docs
grep -r "/docs/kindling" apps/docs-site/docs/
```

## To Re-Enable a Section

Reverse the process:

1. Uncomment the plugin
2. Uncomment navbar entry
3. Uncomment footer entry
4. Update homepage
5. Restore cross-links

## Environment-Based Toggle (Advanced)

For dynamic control without code changes:

```typescript
// docusaurus.config.ts
const enabledDocs = {
  'start-here': true,
  'anvil': true,
  'aps': true,
  'kindling': process.env.DOCS_KINDLING !== 'false',
  'edda-stack': process.env.DOCS_EDDA_STACK !== 'false',
};

const plugins = [
  enabledDocs['start-here'] && ['@docusaurus/plugin-content-docs', { id: 'start-here', ... }],
  enabledDocs['anvil'] && ['@docusaurus/plugin-content-docs', { id: 'anvil', ... }],
  enabledDocs['aps'] && ['@docusaurus/plugin-content-docs', { id: 'aps', ... }],
  enabledDocs['kindling'] && ['@docusaurus/plugin-content-docs', { id: 'kindling', ... }],
  enabledDocs['edda-stack'] && ['@docusaurus/plugin-content-docs', { id: 'edda-stack', ... }],
].filter(Boolean);
```

Then control via environment:

```bash
# Hide Kindling and Edda Stack
DOCS_KINDLING=false DOCS_EDDA_STACK=false pnpm start

# Show everything (default)
pnpm start
```

## Current Section Status

| Section    | Status   | Notes                            |
| ---------- | -------- | -------------------------------- |
| start-here | Disabled | Folded into homepage for go-live |
| anvil      | Enabled  | Primary product                  |
| aps        | Enabled  | OSS spec                         |
| kindling   | Enabled  | Public docs are live             |
| edda-stack | Enabled  | Public docs are live             |

## Checklist for Disabling a Section

- [ ] Comment out plugin in `docusaurus.config.ts`
- [ ] Remove from navbar Products dropdown
- [ ] Remove from footer links
- [ ] Update homepage product tiles
- [ ] Search and fix cross-links from other docs
- [ ] Update "Choose Your Path" page if affected
- [ ] Test build: `pnpm build`
- [ ] Test broken link check: `pnpm build` (fails on broken links by default)
