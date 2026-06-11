# @eddacraft/anvil-website

Next.js marketing and documentation site for eddacraft, deployed to
[eddacraft.ai](https://eddacraft.ai) via Vercel.

## Status

Active

## Stack

- Next.js 16 (App Router)
- React 19
- Tailwind CSS 4
- Radix UI primitives
- Vercel Analytics

## Routes

| Route            | Description                                        |
| ---------------- | -------------------------------------------------- |
| `/`              | Landing page                                       |
| `/auth/activate` | Tombstone — activation moved to `anvil auth login` |
| `/privacy`       | Privacy policy                                     |
| `/security`      | Security policy                                    |

OG images are generated dynamically via `next/og`.

## Consumers

- Public-facing website at eddacraft.ai

## Development

```bash
pnpm --filter @eddacraft/anvil-website dev   # http://localhost:3000
pnpm --filter @eddacraft/anvil-website build
pnpm --filter @eddacraft/anvil-website lint
```
