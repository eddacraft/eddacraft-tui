# Infrastructure (Pulumi IaC)

Infrastructure as Code for the Anvil monorepo using
[Pulumi](https://www.pulumi.com/) with TypeScript.

## Prerequisites

- **Pulumi CLI**: `curl -fsSL https://get.pulumi.com | sh`
- **Azure CLI**: `az login` (for DNS management and Key Vault access)
- **Node.js**: >= 20.0.0
- **pnpm**: >= 10.20.0

## Architecture

- **State backend**: Azure Blob Storage (`azblob://pulumi-state`)
- **Secrets provider**: Azure Key Vault (`kv-iac-anvil`) for Pulumi config
  encryption
- **Application secrets**: All stored in Azure Key Vault, read at deploy time
  via `@azure/keyvault-secrets`

| Azure Resource  | Name                 | Resource Group | Purpose                     |
| --------------- | -------------------- | -------------- | --------------------------- |
| Storage Account | `stiacstateprod`     | `rg-iac-state` | Pulumi state files          |
| Blob Container  | `pulumi-state`       | `rg-iac-state` | State container             |
| Key Vault       | `kv-iac-anvil`       | `rg-iac-state` | Secrets + encryption key    |
| Key Vault Key   | `pulumi-secrets-key` | `rg-iac-state` | Pulumi secrets provider key |

## Setup

### Bootstrap (first time only)

Provision the Azure resources for state and secrets:

```bash
export ARM_CLIENT_ID=<service-principal-client-id>
bash scripts/bootstrap-backend.sh
```

The script creates the storage account, blob container, Key Vault, encryption
key, and RBAC assignments. It outputs the storage account key — store it as a
GitHub Secret (`AZURE_STORAGE_KEY`).

### Store secrets in Key Vault

```bash
az keyvault secret set --vault-name kv-iac-anvil --name vercel-token --value '<VERCEL_TOKEN>'
az keyvault secret set --vault-name kv-iac-anvil --name anvil-api-database-url --value '<DATABASE_URL>'
az keyvault secret set --vault-name kv-iac-anvil --name resend-api-key --value '<RESEND_API_KEY>'
```

### Local development

```bash
pnpm install

# Set storage credentials for state access
export AZURE_STORAGE_ACCOUNT=stiacstateprod
export AZURE_STORAGE_KEY=<key>

# Azure CLI login (for Key Vault access)
az login

# Select a stack
export PATH="$HOME/.pulumi/bin:$PATH"
pulumi stack select dev   # for preview/dry-run
pulumi stack select prod  # for production
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

The CI workflow authenticates to Azure via a service principal, fetches the
Vercel token from Key Vault, and uses Azure Blob Storage for state.

## What's managed

| Resource                                                                          | Provider               | File                      |
| --------------------------------------------------------------------------------- | ---------------------- | ------------------------- |
| Vercel projects (website, anvil-api, docs-shell, docs-public, anvil-docs-private) | `@pulumiverse/vercel`  | `src/vercel.ts`           |
| Vercel domains and env vars                                                       | `@pulumiverse/vercel`  | `src/vercel.ts`           |
| Azure DNS records (eddacraft.ai)                                                  | `@pulumi/azure-native` | `src/dns/eddacraft-ai.ts` |

GitHub secrets and branch protection are managed via the GitHub UI, not Pulumi.

## Secrets

All application secrets are stored in Azure Key Vault (`kv-iac-anvil`) and read
at deploy time using `@azure/keyvault-secrets`. The Pulumi code never stores
secret values in stack config files.

| Key Vault Secret         | Used By         | Purpose                        |
| ------------------------ | --------------- | ------------------------------ |
| `vercel-token`           | CI workflow     | Vercel API auth                |
| `anvil-api-database-url` | `src/vercel.ts` | Neon DB connection (anvil-api) |
| `resend-api-key`         | `src/vercel.ts` | Resend email API               |

To add a new secret:

1. Store it in Key Vault:
   `az keyvault secret set --vault-name kv-iac-anvil --name my-secret --value '...'`
2. Read it in code: `import { getSecret } from './keyvault.js';`

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
  gitRepo: 'eddacraft/anvil-001',
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

## State migration from Pulumi Cloud

If migrating from Pulumi Cloud to the Azure Blob backend:

```bash
cd infra

# 1. Export from Pulumi Cloud
pulumi login
pulumi stack select dev && pulumi stack export --file dev-state-backup.json
pulumi stack select prod && pulumi stack export --file prod-state-backup.json

# 2. Switch to Azure Blob
export AZURE_STORAGE_ACCOUNT=stiacstateprod
export AZURE_STORAGE_KEY="<from bootstrap>"
pulumi login azblob://pulumi-state

# 3. Create stacks with KeyVault secrets provider
pulumi stack init dev \
  --secrets-provider="azurekeyvault://kv-iac-anvil.vault.azure.net/keys/pulumi-secrets-key"
pulumi stack init prod \
  --secrets-provider="azurekeyvault://kv-iac-anvil.vault.azure.net/keys/pulumi-secrets-key"

# 4. Import state
pulumi stack select dev && pulumi stack import --file dev-state-backup.json
pulumi stack select prod && pulumi stack import --file prod-state-backup.json

# 5. Set config and verify
pulumi stack select prod
pulumi config set azure-native:location uksouth
pulumi config set azure-native:subscriptionId 290aa167-2d41-45aa-9b36-8ef5b9be99e0
pulumi config set azure-dns:resourceGroupName rg-prd-ap-public-web
pulumi config set keyvault:vaultName kv-iac-anvil
pulumi preview --expect-no-changes
```

## Troubleshooting

| Problem                                   | Solution                                                   |
| ----------------------------------------- | ---------------------------------------------------------- |
| `pulumi preview` shows unexpected changes | Run `pulumi refresh` to sync state with live resources     |
| Provider auth error                       | Check `az account show`, verify RBAC roles on Key Vault    |
| State lock stuck                          | Delete lock file in `pulumi-state` blob container          |
| Key Vault access denied                   | Verify service principal has `Key Vault Secrets User` role |
| Import fails                              | Verify resource name in code matches import target         |

## GitHub Secrets

| Secret                  | Value                | Purpose              |
| ----------------------- | -------------------- | -------------------- |
| `ARM_CLIENT_ID`         | Service principal ID | Azure auth           |
| `ARM_CLIENT_SECRET`     | SP secret            | Azure auth           |
| `ARM_TENANT_ID`         | Azure tenant ID      | Azure auth           |
| `ARM_SUBSCRIPTION_ID`   | Azure subscription   | Azure auth           |
| `AZURE_STORAGE_ACCOUNT` | `stiacstateprod`     | Pulumi state backend |
| `AZURE_STORAGE_KEY`     | Storage account key  | Pulumi state backend |
