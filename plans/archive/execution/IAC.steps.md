<!--
APS Steps: IAC (Pulumi Infrastructure as Code)
===============================================
AGENT RULES:
- Checkpoint = ONE observable state, max 12 words
- NO implementation detail — that emerges from patterns + judgment
- "How" ONLY appears in Pattern field referencing existing code
- >8 steps? Split the task first
See: plans/aps-rules.md
-->

# Steps: IAC — Pulumi Infrastructure as Code

| Field | Value |
|-------|-------|
| Source | [../modules/pulumi-iac.aps.md](../modules/pulumi-iac.aps.md) |
| Task | IAC — Full module execution |
| Status | Draft |

## Prerequisites

- [ ] Pulumi CLI installed (`pulumi version` succeeds)
- [ ] Pulumi Cloud account created (free tier)
- [ ] Vercel API token with scopes: `read/write projects`, `read/write domains`, `read/write environment variables` (create at vercel.com/account/tokens)
- [ ] GitHub personal access token with `repo`, `admin:org`, and `admin:repo_hook` scopes
- [ ] Azure subscription with access to DNS resource group
- [ ] Azure authentication configured — one of:
  - Azure CLI: `az login` and `az account set --subscription <id>`
  - Service principal: `ARM_CLIENT_ID`, `ARM_CLIENT_SECRET`, `ARM_TENANT_ID`, `ARM_SUBSCRIPTION_ID` environment variables
- [ ] Existing Vercel project IDs noted for import — retrieve with: `curl -s -H "Authorization: Bearer $VERCEL_TOKEN" https://api.vercel.com/v9/projects | jq '.projects[] | {name, id}'`

---

## IAC-001: Scaffold Pulumi project

### 1. Create infra directory and Pulumi project manifest

- **Checkpoint:** `infra/Pulumi.yaml` declares nodejs runtime with pnpm
- **Validate:** `cat infra/Pulumi.yaml | grep -q "packagemanager: pnpm"`

### 2. Add infra as pnpm workspace member

- **Checkpoint:** `pnpm-workspace.yaml` includes `infra` path
- **Validate:** `pnpm ls --filter infra --depth 0`
- **Pattern:** `pnpm-workspace.yaml` (existing workspace entries)

### 3. Create TypeScript configuration for infra

- **Checkpoint:** `infra/tsconfig.json` extends base config, targets ESM
- **Validate:** `cd infra && pnpm exec tsc --noEmit`
- **Pattern:** `tsconfig.base.json` (project-wide TS settings)

### 4. Install Pulumi providers as dependencies

- **Checkpoint:** `@pulumi/pulumi`, `@pulumiverse/vercel`, `@pulumi/github`, `@pulumi/azure-native` in package.json
- **Validate:** `cd infra && pnpm ls @pulumi/pulumi @pulumi/azure-native`

### 5. Create entry point with empty stack

- **Checkpoint:** `infra/index.ts` exports an empty Pulumi programme
- **Validate:** `cd infra && pulumi preview --stack dev`

---

## IAC-002: Configure state backend and provider authentication

### 1. Authenticate with Pulumi Cloud

- **Checkpoint:** `pulumi whoami` returns authenticated identity
- **Validate:** `pulumi whoami`

### 2. Configure Azure authentication

- **Checkpoint:** Azure CLI or service principal credentials available to Pulumi
- **Validate:** `az account show --query name -o tsv` (CLI) or `test -n "$ARM_CLIENT_ID"` (SP)
- **Notes:** For CI, use service principal env vars in GitHub Actions secrets. For local dev, `az login` is sufficient.

### 3. Create dev and prod stacks

- **Checkpoint:** Both stacks appear in `pulumi stack ls` output
- **Validate:** `pulumi stack ls | grep -c "dev\|prod"`

### 4. Write stack-specific configuration files

- **Checkpoint:** `Pulumi.dev.yaml` and `Pulumi.prod.yaml` contain environment and provider config
- **Validate:** `test -f infra/Pulumi.dev.yaml && test -f infra/Pulumi.prod.yaml`
- **Notes:** Prod stack config must include Azure DNS resource group. Example:

```yaml
# Pulumi.prod.yaml
config:
  azure-native:location: uksouth
  azure-native:subscriptionId: <subscription-id>
  azure-dns:resourceGroupName: rg-dns-prod
  vercel:apiToken:
    secure: <encrypted-token>
  github:token:
    secure: <encrypted-token>
```

---

## IAC-003: Manage website Vercel project

### 1. Declare website project resource

- **Checkpoint:** Pulumi preview shows `vercel:index:Project` for website
- **Validate:** `pulumi preview --stack prod --diff | grep "Project.*website"`

### 2. Declare custom domain resource

- **Checkpoint:** Pulumi preview shows `vercel:index:ProjectDomain` resource
- **Validate:** `pulumi preview --stack prod --diff | grep "ProjectDomain"`

### 3. Declare environment variable resources

- **Checkpoint:** Environment variables managed as Pulumi resources
- **Validate:** `pulumi preview --stack prod --diff | grep "ProjectEnvironmentVariable"`

---

## IAC-004: Manage docs-site Vercel project

### 1. Declare docs-site project resource

- **Checkpoint:** Pulumi preview shows Vercel project for docs-site
- **Validate:** `pulumi preview --stack prod --diff | grep "docs-site"`

### 2. Declare docs-site domain resource

- **Checkpoint:** `eddacraft.dev` domain attached to docs-site project
- **Validate:** `pulumi preview --stack prod --diff | grep "eddacraft.dev"`

---

## IAC-005: Create VercelApp ComponentResource

### 1. Extract common pattern into component class

- **Checkpoint:** `VercelApp` ComponentResource class exists and compiles
- **Validate:** `cd infra && pnpm exec tsc --noEmit`

### 2. Refactor website and docs-site to use component

- **Checkpoint:** Both apps use `VercelApp`; preview output unchanged
- **Validate:** `pulumi preview --stack prod --expect-no-changes`

### 3. Write unit tests for component

- **Checkpoint:** Tests verify resource count and types from component
- **Validate:** `cd infra && pnpm exec vitest run`

---

## IAC-006: Manage GitHub repository configuration

### 1. Declare Actions secrets

- **Checkpoint:** Pulumi preview shows `ActionsSecret` resources
- **Validate:** `pulumi preview --stack prod --diff | grep "ActionsSecret"`

### 2. Declare deployment environments

- **Checkpoint:** Production environment with protection rules appears in preview
- **Validate:** `pulumi preview --stack prod --diff | grep "RepositoryEnvironment"`

### 3. Declare branch protection rules

- **Checkpoint:** Main branch protection rule appears in preview
- **Validate:** `pulumi preview --stack prod --diff | grep "BranchProtection"`

---

## Completion

- [ ] All checkpoints validated (IAC-001 through IAC-006)
- [ ] Continue with [IAC-part2.steps.md](./IAC-part2.steps.md) for IAC-007 through IAC-012
