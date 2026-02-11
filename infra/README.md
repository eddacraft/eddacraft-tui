# Infrastructure (Pulumi IaC)

Infrastructure as Code for the Anvil monorepo using
[Pulumi](https://www.pulumi.com/) with TypeScript.

## Prerequisites

- **Pulumi CLI**: `curl -fsSL https://get.pulumi.com | sh`
- **Pulumi Cloud account**: Free tier at [pulumi.com](https://app.pulumi.com)
- **Azure CLI**: `az login` (for DNS management)
- **Vercel token**: Create at
  [vercel.com/account/tokens](https://vercel.com/account/tokens)
- **Node.js**: >= 20.0.0
- **pnpm**: >= 10.20.0

## Setup

```bash
# Install dependencies
pnpm install

# Log in to Pulumi Cloud
export PATH="$HOME/.pulumi/bin:$PATH"
pulumi login

# Select a stack
pulumi stack select dev   # for preview/dry-run
pulumi stack select prod  # for production

# Set provider tokens (encrypted in stack config)
pulumi config set --secret vercel:apiToken $VERCEL_TOKEN
pulumi config set --secret vercel-apps:website-database-url $DATABASE_URL
```

## Stacks

| Stack  | Purpose                                                     |
| ------ | ----------------------------------------------------------- |
| `dev`  | Local preview/dry-run only — never applied                  |
| `prod` | Production infrastructure — applied via CI on merge to main |

Both stacks target the same Azure subscription and Vercel account. The `dev`
stack is for safe previewing without risk of applying changes.

## Workflow

### Preview changes

```bash
cd infra
pulumi preview
```

### Apply changes (local — use with caution)

```bash
cd infra
pulumi up
```

### CI/CD

- **Pull requests**: `pulumi preview` runs automatically, posts diff as PR
  comment
- **Merge to main**: `pulumi up` runs automatically via
  `.github/workflows/infra.yml`

## What's managed

| Resource                             | Provider               | File                      |
| ------------------------------------ | ---------------------- | ------------------------- |
| Vercel projects (website, docs-site) | `@pulumiverse/vercel`  | `src/vercel.ts`           |
| Vercel domains and env vars          | `@pulumiverse/vercel`  | `src/vercel.ts`           |
| Azure DNS records (eddacraft.ai)     | `@pulumi/azure-native` | `src/dns/eddacraft-ai.ts` |

GitHub secrets and branch protection are managed via the GitHub UI, not Pulumi.

## Adding resources

### New DNS record

Edit `src/dns/eddacraft-ai.ts`:

```typescript
new azure.dns.RecordSet('my-record-eddacraft-ai', {
  relativeRecordSetName: 'subdomain',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'CNAME',
  ttl: 3600,
  cnameRecord: { cname: 'target.example.com' },
});
```

### New Vercel project

Edit `src/vercel.ts`:

```typescript
export const myApp = new VercelApp('my-app', {
  name: 'my-app',
  framework: 'nextjs',
  rootDirectory: 'apps/my-app',
  gitRepo: 'EddaCraft/anvil-001',
  domains: ['my-app.eddacraft.ai'],
});
```

## Importing existing resources

Use the import script to bring existing Vercel projects under Pulumi management:

```bash
export VERCEL_TOKEN=<token>
bash scripts/fetch-vercel-ids.sh
# Follow the generated pulumi import commands
pulumi preview --expect-no-changes  # verify state matches
```

## Rollback

Pulumi has no built-in rollback. To revert infrastructure changes:

1. **Revert the code change**: `git revert <commit>`
2. **Apply previous state**: `pulumi up`

For emergencies, use the provider dashboards directly (Vercel, Azure Portal).

## State recovery

```bash
# Backup state
pulumi stack export > backup.json

# Restore state
pulumi stack import < backup.json

# Sync state with live resources (detect drift)
pulumi refresh
```

## Troubleshooting

| Problem                                   | Solution                                                  |
| ----------------------------------------- | --------------------------------------------------------- |
| `pulumi preview` shows unexpected changes | Run `pulumi refresh` to sync state with live resources    |
| Provider auth error                       | Check tokens: `pulumi config`, `az account show`          |
| State lock stuck                          | Unlock via [Pulumi Cloud console](https://app.pulumi.com) |
| Import fails                              | Verify resource name in code matches import target        |
