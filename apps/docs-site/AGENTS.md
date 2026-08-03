# Docs Site Agent Notes

This app is the Docusaurus 3.10.1 host for
[docs.eddacraft.ai](https://docs.eddacraft.ai). It aggregates separately owned
product content; it is not the marketing site in `apps/website`.

Follow the root [`AGENTS.md`](../../AGENTS.md),
[`plans/project-context.md`](../../plans/project-context.md), and
[`docs/guides/documentation-governance.md`](../../docs/guides/documentation-governance.md).
The host is owned by DSITE. Public anvil content is owned by DOCSYNC; sibling
content remains derived from its owning product.

## Source map

- `docusaurus.config.ts` owns plugin instances, routes, navbar, and footer.
- `sidebars/*.ts` owns per-product navigation.
- `src/pages/index.tsx` and `src/css/` own the host landing page and theme.
- `../../docs/public/anvil` publishes at `/anvil`.
- `../../docs/public/aps` publishes at `/aps`.
- `../../docs/public/kindling` publishes at `/kindling`.
- `../../docs/public/edda-stack` publishes at `/edda-stack`.
- `../../docs/public/beta` publishes at unlisted `/beta` routes.
- `../../docs/public/start-here` is retained content but its plugin is disabled
  in `docusaurus.config.ts`.

Do not create a second content tree under this app. Add or edit product pages in
their owning `docs/public/<product>/` directory, then update the matching
sidebar when navigation changes.

## Commands

From the repository root:

```bash
pnpm --filter @eddacraft/docs-site start
pnpm --filter @eddacraft/docs-site build
pnpm --filter @eddacraft/docs-site typecheck
```

From this directory, the equivalent commands are `pnpm start`, `pnpm build`, and
`pnpm typecheck`.

## Change rules

- Keep Docusaurus front matter and sidebar ids aligned.
- Treat route or heading changes as link-contract changes; update inbound links.
- Section availability is static configuration. Change the plugin, navigation,
  homepage, footer, and affected links together; there are no runtime `DOCS_*`
  section-toggle environment variables.
- Preserve lowercase `anvil`, `eddacraft`, and `kindling` in public prose.
- Never put internal plan ids, source paths, or repository-only instructions in
  published pages.

Before closeout, run the root public-doc checks and a production build:

```bash
pnpm docs:public:check
pnpm docs:public:commands
pnpm docs:public:aps-commands
pnpm docs:check
pnpm --filter @eddacraft/docs-site build
```
