<!--
APS Module: Pulumi IaC
======================
AGENT RULES:
- Purpose/Scope = WHAT this module does (not how)
- Tasks = execution authority (add only when status=Ready)
- Task fields: Intent + Outcome + Validation (required)
- NO implementation detail in tasks
See: plans/aps-rules.md
-->

# Pulumi Infrastructure as Code

| ID | Owner | Status |
|----|-------|--------|
| IAC | @eddacraft | Ready |

## Purpose

Codify all infrastructure that supports the Anvil monorepo — Vercel projects,
GitHub repository settings, DNS records, and CI/CD secrets — into a single
Pulumi TypeScript project so that environments are reproducible, auditable, and
version-controlled alongside the application code they serve.

## In Scope

- Pulumi project scaffold inside the monorepo (`infra/` workspace member)
- Vercel project configuration for `apps/website` (Next.js) and `apps/docs-site` (Docusaurus)
- Vercel custom domains and environment variables
- GitHub Actions secrets and repository environment configuration
- GitHub branch protection rules
- Azure DNS zone and record management (per-zone file organisation mirroring existing Terraform pattern)
- CI/CD pipeline integration (`pulumi preview` on PRs, `pulumi up` on merge)
- Two stacks: `dev` (preview) and `prod` (production)
- Pulumi state backend configuration (Pulumi Cloud free tier)
- Reusable ComponentResource abstractions for Vercel app patterns
- Unit tests for infrastructure code (vitest)

## Out of Scope

- Application build or deployment logic (Vercel Git integration handles deploys)
- npm package publishing pipeline (remains in `publish.yml`)
- Database provisioning (no databases in current architecture)
- Azure compute resources — Anvil is Vercel-hosted (Azure is used for DNS only)
- Pulumi Cloud paid features (Team/Enterprise editions)
- Monitoring and alerting infrastructure (Vercel built-in analytics suffice)
- Docker or container orchestration (no containers in current architecture)

## Interfaces

**Depends on:**

- Vercel — project hosting, domain management, deployment runtime
- GitHub — repository, Actions secrets, environments, branch protection
- Azure DNS — DNS zone hosting and record management (existing zones)
- Azure subscription — authentication via service principal or Azure CLI
- Pulumi Cloud — state storage, locking, secrets encryption (free tier)

**Exposes:**

- `infra/` workspace package — Pulumi project with `preview` and `up` Nx targets
- `pulumi-preview` GitHub Action job — runs on PRs to show infrastructure diff
- `pulumi-up` GitHub Action job — applies infrastructure changes on merge to main
- Vercel project outputs (project IDs, URLs) available via Pulumi stack outputs

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined

## Tasks

### IAC-001: Scaffold Pulumi project in the monorepo

- **Intent:** Establish the `infra/` directory as a Pulumi TypeScript project integrated with the Nx workspace
- **Expected Outcome:** `infra/` directory exists with `Pulumi.yaml`, `package.json`, `tsconfig.json`, and `index.ts` entry point; `pnpm install` succeeds; `pulumi preview` runs without error against an empty stack
- **Validation:** `cd infra && pulumi preview --stack dev 2>&1 | grep -q "0 unchanged"`
- **Files:** `infra/Pulumi.yaml`, `infra/package.json`, `infra/tsconfig.json`, `infra/index.ts`, `pnpm-workspace.yaml`
- **Dependencies:** None
- **Confidence:** high

### IAC-002: Configure Pulumi state backend and provider authentication

- **Intent:** Store Pulumi state securely with locking and secrets encryption using Pulumi Cloud free tier; configure Azure, Vercel, and GitHub provider authentication for both local development and CI
- **Expected Outcome:** `pulumi login` connects to Pulumi Cloud; `dev` and `prod` stacks are created; stack configs (`Pulumi.dev.yaml`, `Pulumi.prod.yaml`) exist with provider config including Azure DNS resource group, subscription, and encrypted tokens; Azure auth verified via CLI or service principal
- **Validation:** `pulumi stack ls 2>&1 | grep -q "dev"` and `pulumi stack ls 2>&1 | grep -q "prod"` and `pulumi config get azure-dns:resourceGroupName --stack prod`
- **Files:** `infra/Pulumi.dev.yaml`, `infra/Pulumi.prod.yaml`
- **Dependencies:** IAC-001
- **Confidence:** high
- **Notes:** Vercel token requires scopes: read/write projects, domains, environment variables. GitHub token requires: repo, admin:org, admin:repo_hook. Azure auth: `az login` for local dev, service principal env vars (`ARM_CLIENT_ID`, etc.) for CI.

### IAC-003: Manage Vercel project configuration for website

- **Intent:** Declare the `apps/website` Vercel project, its custom domain, framework settings, and environment variables as Pulumi resources
- **Expected Outcome:** `pulumi preview` shows a `vercel.Project`, `vercel.ProjectDomain`, and `vercel.ProjectEnvironmentVariable` resources for the website; `pulumi up` creates/imports them without manual Vercel dashboard changes
- **Validation:** `pulumi preview --stack prod --diff 2>&1 | grep -q "vercel:index:Project"`
- **Files:** `infra/src/vercel.ts`
- **Dependencies:** IAC-001, IAC-002
- **Confidence:** high

### IAC-004: Manage Vercel project configuration for docs-site

- **Intent:** Declare the `apps/docs-site` Vercel project, its custom domain, and build settings as Pulumi resources
- **Expected Outcome:** `pulumi preview` shows Vercel resources for the docs-site; Docusaurus framework preset, build command, and root directory are configured
- **Validation:** `pulumi preview --stack prod --diff 2>&1 | grep -q "docs-site"`
- **Files:** `infra/src/vercel.ts`
- **Dependencies:** IAC-001, IAC-002
- **Confidence:** high

### IAC-005: Create reusable VercelApp ComponentResource

- **Intent:** Avoid duplication by abstracting the common Vercel project + domain + env var pattern into a reusable Pulumi ComponentResource
- **Expected Outcome:** A `VercelApp` class exists that accepts app name, framework, root directory, domains, and environment variables; IAC-003 and IAC-004 use this abstraction; `pulumi preview` output is unchanged
- **Validation:** `cd infra && pnpm exec vitest run --reporter=verbose 2>&1 | grep -q "VercelApp"`
- **Files:** `infra/src/components/vercel-app.ts`
- **Dependencies:** IAC-003, IAC-004
- **Confidence:** high

### IAC-006: Manage GitHub repository configuration

- **Intent:** Codify GitHub Actions secrets, deployment environments, and branch protection rules so they are reproducible and auditable
- **Expected Outcome:** `pulumi preview` shows `github.ActionsSecret`, `github.RepositoryEnvironment`, and `github.BranchProtectionV3` resources; secrets for `VERCEL_TOKEN`, `NPM_TOKEN`, and `CLAUDE_CODE_OAUTH_TOKEN` are managed
- **Validation:** `pulumi preview --stack prod --diff 2>&1 | grep -q "github:index:ActionsSecret"`
- **Files:** `infra/src/github.ts`
- **Dependencies:** IAC-001, IAC-002
- **Confidence:** high

### IAC-007: Manage Azure DNS zones and records

- **Intent:** Declare Azure DNS records across zones using a per-zone file organisation (one file per DNS zone, mirroring the existing Terraform `<domain>.tf` convention) so that DNS changes are version-controlled and follow a familiar pattern
- **Expected Outcome:** `pulumi preview` shows `azure-native.network.RecordSet` resources for CNAME, A, and TXT records across all managed zones; each zone has its own source file under `infra/src/dns/`; records point to Vercel CNAMEs for web apps
- **Validation:** `pulumi preview --stack prod --diff 2>&1 | grep -q "azure-native:network:RecordSet"`
- **Files:** `infra/src/dns/index.ts`, `infra/src/dns/eddacraft-ai.ts`, `infra/src/dns/eddacraft-dev.ts`
- **Dependencies:** IAC-001, IAC-002
- **Confidence:** high
- **Notes:** Mirrors existing Terraform pattern where each zone is a separate file (e.g., `maindomain.com.tf`). Uses `@pulumi/azure-native` to reference existing Azure DNS zones via `getZone` and creates record sets within them. Naming convention for resources follows `<subdomain>_<zone>` pattern.

### IAC-008: Add Pulumi CI/CD pipeline integration

- **Intent:** Run `pulumi preview` on pull requests and `pulumi up` on merge to main so infrastructure changes are reviewed before application
- **Expected Outcome:** A new GitHub Actions workflow (or additions to `ci.yml`) runs `pulumi preview` on PRs and posts a comment with the diff; `pulumi up` runs automatically on merge to main; the workflow uses `PULUMI_ACCESS_TOKEN` from GitHub secrets
- **Validation:** `.github/workflows/infra.yml` exists and `act -l 2>&1 | grep -q "pulumi"` (or manual PR test)
- **Files:** `.github/workflows/infra.yml`
- **Dependencies:** IAC-001, IAC-002, IAC-006
- **Confidence:** high

### IAC-009: Write unit tests for infrastructure code

- **Intent:** Ensure infrastructure definitions are correct before deployment by testing Pulumi resource outputs with vitest
- **Expected Outcome:** Unit tests validate that the correct number and type of resources are created; tests run as part of the Nx `test` target for the `infra` project; tests pass in CI
- **Validation:** `cd infra && pnpm exec vitest run --reporter=verbose`
- **Files:** `infra/src/__tests__/vercel.test.ts`, `infra/src/__tests__/github.test.ts`, `infra/src/__tests__/dns.test.ts`
- **Dependencies:** IAC-003, IAC-004, IAC-005, IAC-006, IAC-007
- **Confidence:** high

### IAC-010: Import existing Vercel resources into Pulumi state

- **Intent:** Bring existing Vercel projects and domains under Pulumi management without recreating them
- **Expected Outcome:** Import helper script retrieves project IDs from Vercel API; `pulumi import` commands successfully import the live website and docs-site projects; subsequent `pulumi preview` shows no pending changes (state matches live)
- **Validation:** `pulumi preview --stack prod --expect-no-changes`
- **Files:** `infra/scripts/fetch-vercel-ids.sh`
- **Dependencies:** IAC-003, IAC-004
- **Confidence:** medium
- **Risks:** Import requires Vercel project IDs — mitigated by helper script that calls `GET /v9/projects` with `$VERCEL_TOKEN` to generate import commands automatically

### IAC-011: Document IaC setup and contributor workflow

- **Intent:** Ensure contributors understand how to preview and apply infrastructure changes
- **Expected Outcome:** `infra/README.md` documents prerequisites (Pulumi CLI, Pulumi Cloud login), stack selection, preview/up workflow, and secret management; root CONTRIBUTING.md references the infra workflow
- **Validation:** `test -f infra/README.md && wc -l infra/README.md | awk '{print ($1 > 20)}'`
- **Files:** `infra/README.md`
- **Dependencies:** IAC-001, IAC-008
- **Confidence:** high

### IAC-012: Document rollback procedures

- **Intent:** Ensure contributors know how to revert infrastructure changes and recover from state corruption
- **Expected Outcome:** `infra/README.md` documents rollback procedure (revert code → `pulumi up`), emergency manual procedures (Vercel dashboard), and state recovery (`pulumi stack export/import`)
- **Validation:** `grep -q "rollback" infra/README.md`
- **Files:** `infra/README.md`
- **Dependencies:** IAC-011
- **Confidence:** high

## Execution

Steps: [../execution/IAC.steps.md](../execution/IAC.steps.md)
