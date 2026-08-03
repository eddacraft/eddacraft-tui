# eddacraft docs site

| Type   | Authority | Owner | Status | Freshness                                                                                |
| ------ | --------- | ----- | ------ | ---------------------------------------------------------------------------------------- |
| README | Derived   | DSITE | Live   | Reviewed 2026-08-03 against `package.json`, `docusaurus.config.ts`, and `docs/public/**` |

| Upstream                                        | Downstream                            |
| ----------------------------------------------- | ------------------------------------- |
| `docusaurus.config.ts`, `docs/public/**`, DSITE | Local development and deployment work |

This app builds [docs.eddacraft.ai](https://docs.eddacraft.ai) with Docusaurus
3.10.1. It is a shared host: public product content lives under
`../../docs/public/`, while this directory owns routing, navigation, the landing
page, styling, and deployment configuration.

## Develop

Use the repository's pinned Node and pnpm versions, then run from the repository
root:

```bash
pnpm install
pnpm --filter @eddacraft/docs-site start
```

The development server serves the anvil, APS, kindling, and Edda Stack sections.
The beta section is routable but deliberately absent from the global navigation.

## Validate

```bash
pnpm docs:public:check
pnpm docs:public:commands
pnpm docs:public:aps-commands
pnpm docs:check
pnpm --filter @eddacraft/docs-site typecheck
pnpm --filter @eddacraft/docs-site build
```

The production build fails on broken Docusaurus links. The root docs checks add
public-only language, navigation, command-contract, metadata, source-reference,
and generated-index validation.

## Ownership

- Host configuration: `docusaurus.config.ts`, `sidebars/`, `src/`, `static/`,
  and `vercel.json`.
- Product content: `../../docs/public/<product>/`.
- Public anvil editorial work: DOCSYNC.
- Shared host and sibling registration: DSITE.

Read [`AGENTS.md`](AGENTS.md) before editing this app and the root
[`documentation governance guide`](../../docs/guides/documentation-governance.md)
before closing documentation work.
