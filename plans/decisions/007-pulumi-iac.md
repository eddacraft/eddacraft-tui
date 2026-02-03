# ADR-007: Pulumi (Open Source) for Infrastructure as Code

## Status

Accepted

## Date

2026-02-03

## Context

The Anvil monorepo deploys to multiple services — Vercel (website + docs-site),
npm (CLI), and GitHub (Actions, secrets, environments) — but all infrastructure
configuration lives in vendor dashboards or is implicit. This creates several
problems:

1. **No reproducibility.** If the Vercel project is misconfigured, there is no
   way to reconstruct the correct state from source control.
2. **No auditability.** Changes to GitHub secrets, branch protection, or Vercel
   environment variables leave no trace in the repository history.
3. **No review process.** Infrastructure changes bypass the PR workflow that
   governs all code changes.
4. **Bus factor.** Only people with dashboard access can diagnose or reproduce
   the deployment configuration.

### Options Considered

1. **Terraform (HCL)**
   - Pro: Most widely adopted IaC tool; extensive provider ecosystem
   - Pro: Terraform Vercel provider is official (maintained by Vercel)
   - Con: HCL is a separate language — another syntax for a TypeScript-first team
   - Con: State management requires S3 + DynamoDB (or Terraform Cloud)
   - Con: No native TypeScript type safety; schema validation is limited

2. **Pulumi (TypeScript, open source)**
   - Pro: Infrastructure written in TypeScript — same language as the monorepo
   - Pro: Full type safety, autocompletion, and IDE support
   - Pro: Pulumi Cloud free tier handles state, locking, and secrets encryption
   - Pro: Apache 2.0 licence — fully open source
   - Pro: Natural fit for pnpm workspace member
   - Con: Vercel provider is community-maintained (`@pulumiverse/vercel`)
   - Con: Smaller ecosystem than Terraform

3. **AWS CDK / CloudFormation**
   - Pro: First-party AWS support
   - Con: Only manages AWS resources — cannot manage Vercel or GitHub
   - Con: Overkill when the project has no AWS compute

4. **No IaC (status quo)**
   - Pro: Zero additional tooling
   - Con: All the problems listed above persist

## Decision

**Use Pulumi (open source) with TypeScript for infrastructure as code.**

The `infra/` directory will be a pnpm workspace member containing a single
Pulumi project with `dev` and `prod` stacks. Pulumi Cloud free tier will serve
as the state backend.

### Provider Stack

| Provider | Package | Maintainer | Purpose |
| --- | --- | --- | --- |
| Pulumi core | `@pulumi/pulumi` | Pulumi Inc. | Core SDK |
| Vercel | `@pulumiverse/vercel` | Pulumiverse (community) | Projects, domains, env vars |
| GitHub | `@pulumi/github` | Pulumi Inc. | Secrets, environments, branch protection |
| Azure Native | `@pulumi/azure-native` | Pulumi Inc. | Azure DNS zones and record sets |

### Deployment Model

- **Vercel project configuration** is managed by Pulumi (project settings,
  domains, environment variables, framework presets)
- **Actual deployments** remain handled by Vercel's Git integration
  (auto-deploy on push to main, preview deployments on PRs)
- **`pulumi preview`** runs on PRs via GitHub Actions to show infrastructure
  diffs
- **`pulumi up`** runs on merge to main to apply infrastructure changes

## Rationale

### 1. TypeScript Consistency

The monorepo is TypeScript-first (strict mode, ESM, Zod schemas). Pulumi lets
infrastructure definitions live in the same language with full type safety:

```typescript
import * as vercel from '@pulumiverse/vercel';

const website = new vercel.Project('website', {
  name: 'anvil-website',
  framework: 'nextjs',
  rootDirectory: 'apps/website',
  buildCommand: 'pnpm nx build website',
});
```

This is more natural for the team than learning HCL or YAML-based IaC.

### 2. Monorepo Integration

Pulumi projects are standard Node.js packages. The `infra/` directory becomes a
pnpm workspace member with its own `package.json` and `tsconfig.json`, fitting
naturally into the existing Nx build graph. Nx targets (`preview`, `up`) can be
defined just like any other project.

### 3. State Management

Pulumi Cloud free tier provides:

- Unlimited state updates
- Built-in state locking (no DynamoDB needed)
- Built-in secrets encryption (no KMS setup needed)
- Web console for state inspection
- 500 deployment minutes per month (sufficient for this project)

This eliminates the S3 + DynamoDB bootstrapping problem that Terraform
self-managed backends require.

### 4. Open Source Licence

Pulumi CLI and SDK are Apache 2.0, compatible with the project's licence. The
free tier of Pulumi Cloud is sufficient — no paid features are required.

### 5. Vercel Provider Assessment

The `@pulumiverse/vercel` provider is community-maintained but bridges the
official Vercel Terraform provider, so resource coverage is solid (v3.15.x).
The risk is mitigated by:

- Using Vercel's Git integration for actual deployments (the most complex part)
- Only managing project configuration via Pulumi (lower rate of change)
- The Terraform provider upstream being official and well-maintained

## Consequences

### Positive

- Infrastructure is version-controlled alongside application code
- Changes go through PR review before application
- Environments are reproducible from a single `pulumi up` command
- TypeScript type safety catches configuration errors at compile time
- Contributors can understand infrastructure without learning a new language
- State is securely managed with zero bootstrapping effort

### Negative

- Adds Pulumi CLI as a development dependency for infrastructure work
- Community-maintained Vercel provider may lag behind Vercel API changes
- Contributors need a Pulumi Cloud account (free) for state access
- Additional CI/CD minutes for `pulumi preview` on PRs

### Mitigations

- Pulumi CLI installation documented in `infra/README.md`
- Provider versions pinned in `package.json` to avoid drift
- Vercel Git integration handles deployments (reduces Pulumi surface area)
- `pulumi preview` is lightweight and fast for small resource counts

### Rollback Strategy

Pulumi has no built-in rollback command. The procedure is:

1. **Revert the code change** — `git revert <commit>` or fix forward
2. **Run `pulumi up`** — re-applies the previous infrastructure state
3. **Emergency escape hatch** — Vercel dashboard remains accessible for manual overrides

State recovery: `pulumi stack export > backup.json` before risky operations; `pulumi stack import < backup.json` to restore.

## Authentication & Token Requirements

| Provider | Auth Method (local) | Auth Method (CI) | Required Scopes |
| --- | --- | --- | --- |
| Pulumi Cloud | `pulumi login` | `PULUMI_ACCESS_TOKEN` | State read/write |
| Vercel | `VERCEL_TOKEN` env var | GitHub Actions secret | Read/write: projects, domains, env vars |
| GitHub | `GITHUB_TOKEN` env var | GitHub Actions secret | `repo`, `admin:org`, `admin:repo_hook` |
| Azure | `az login` | Service principal (`ARM_CLIENT_ID`, `ARM_CLIENT_SECRET`, `ARM_TENANT_ID`, `ARM_SUBSCRIPTION_ID`) | DNS Zone Contributor on resource group |

## Project Structure

```
infra/
├── Pulumi.yaml          # Runtime: nodejs, packagemanager: pnpm
├── Pulumi.dev.yaml      # Dev stack configuration
├── Pulumi.prod.yaml     # Prod stack configuration
├── package.json         # Dependencies: @pulumi/pulumi, @pulumiverse/vercel, @pulumi/github, @pulumi/azure-native
├── tsconfig.json        # Extends tsconfig.base.json
├── index.ts             # Entry point — imports and composes all resources
└── src/
    ├── components/
    │   └── vercel-app.ts        # Reusable VercelApp ComponentResource
    ├── vercel.ts                # Website + docs-site project definitions
    ├── github.ts                # Secrets, environments, branch protection
    ├── dns/
    │   ├── index.ts             # Zone lookup helpers, re-exports
    │   ├── eddacraft-ai.ts     # Records in eddacraft.ai zone
    │   └── eddacraft-dev.ts     # Records in eddacraft.dev zone
    └── __tests__/
        ├── vercel.test.ts       # Unit tests for Vercel resources
        ├── github.test.ts       # Unit tests for GitHub resources
        └── dns.test.ts          # Unit tests for DNS records
```

## Azure DNS Per-Zone Pattern

DNS records are managed in Azure DNS. The existing Terraform convention uses one
file per DNS zone (e.g., `maindomain.com.tf`), with records referencing the zone
via `data.azurerm_dns_zone`. The Pulumi equivalent preserves this pattern:

### Terraform (existing pattern)

```hcl
# maindomain.com.tf
resource "azurerm_dns_cname_record" "go_maindomain_com" {
  name                = "go"
  zone_name           = data.azurerm_dns_zone.maindomain_com.name
  resource_group_name = data.azurerm_dns_zone.maindomain_com.resource_group_name
  ttl                 = 3600
  record              = "cname.tinyurl.com"
}
```

### Pulumi equivalent

```typescript
// infra/src/dns/eddacraft-ai.ts
import * as azure from '@pulumi/azure-native';
import { zone } from './index';

// Each file manages one DNS zone — mirrors <domain>.tf convention

const goRecord = new azure.network.RecordSet('go-eddacraft-ai', {
  relativeRecordSetName: 'go',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName: zone.eddacraftAi.resourceGroupName,
  recordType: 'CNAME',
  ttl: 3600,
  cnameRecord: {
    cname: 'cname.tinyurl.com',
  },
});
```

```typescript
// infra/src/dns/index.ts
import * as azure from '@pulumi/azure-native';
import * as pulumi from '@pulumi/pulumi';

const config = new pulumi.Config('azure-dns');
const resourceGroupName = config.require('resourceGroupName');

// Look up existing zones (not managed by Pulumi — they already exist)
export const zone = {
  eddacraftAi: azure.network.getZoneOutput({
    zoneName: 'eddacraft.ai',
    resourceGroupName,
  }),
  eddacraftDev: azure.network.getZoneOutput({
    zoneName: 'eddacraft.dev',
    resourceGroupName,
  }),
};
```

**Key design decisions:**

- **One file per zone** — `eddacraft-ai.ts`, `eddacraft-dev.ts` — same
  organisational principle as the Terraform `<domain>.tf` files
- **Zones are looked up, not created** — uses `getZoneOutput` to reference
  existing Azure DNS zones (avoids managing zone lifecycle in Pulumi)
- **Resource naming convention** — `<subdomain>-<zone>` (e.g.,
  `go-eddacraft-ai`) for Pulumi resource names, following a similar pattern
  to the Terraform `<subdomain>_<zone>` convention (hyphens instead of underscores)
- **TTL defaults to 3600** — consistent with existing Terraform templates

## References

- [Pulumi TypeScript SDK](https://www.pulumi.com/docs/iac/languages-sdks/javascript/)
- [Pulumiverse Vercel Provider](https://www.pulumi.com/registry/packages/vercel/)
- [Pulumi GitHub Provider](https://www.pulumi.com/registry/packages/github/)
- [Pulumi Azure Native Provider](https://www.pulumi.com/registry/packages/azure-native/)
- [Azure Native DNS Zone](https://www.pulumi.com/registry/packages/azure-native/api-docs/network/zone/)
- [Azure Native RecordSet](https://www.pulumi.com/registry/packages/azure-native/api-docs/network/recordset/)
- [Pulumi State and Backends](https://www.pulumi.com/docs/iac/concepts/state-and-backends/)
- [Pulumi Cloud Pricing](https://www.pulumi.com/pricing/)
- [IaC Best Practices: Structuring Projects](https://www.pulumi.com/blog/iac-best-practices-structuring-pulumi-projects/)
- [Pulumi in Nx Monorepos](https://www.pulumi.com/blog/nx-monorepo/)
