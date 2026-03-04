<!--
APS Steps: IAC Part 2 (Pulumi Infrastructure as Code)
======================================================
AGENT RULES:
- Checkpoint = ONE observable state, max 12 words
- NO implementation detail — that emerges from patterns + judgment
- "How" ONLY appears in Pattern field referencing existing code
- >8 steps? Split the task first
See: plans/aps-rules.md
-->

# Steps: IAC Part 2 — Pulumi Infrastructure as Code

| Field | Value |
|-------|-------|
| Source | [../modules/pulumi-iac.aps.md](../modules/pulumi-iac.aps.md) |
| Task | IAC — Full module execution (continued) |
| Status | Draft |
| Part 1 | [IAC.steps.md](./IAC.steps.md) (IAC-001 through IAC-006) |

## Prerequisites

- [ ] IAC-001 through IAC-006 completed (see [IAC.steps.md](./IAC.steps.md))
- [ ] `pulumi preview --stack prod` runs without error

---

## IAC-007: Manage Azure DNS zones and records

### 1. Create dns/ directory with zone lookup index

- **Checkpoint:** `infra/src/dns/index.ts` looks up existing Azure DNS zones
- **Validate:** `cd infra && pnpm exec tsc --noEmit`

### 2. Create per-zone file for eddacraft.ai

- **Checkpoint:** `infra/src/dns/eddacraft-ai.ts` declares CNAME/A records for the zone
- **Validate:** `pulumi preview --stack prod --diff | grep "azure-native:network:RecordSet"`
- **Pattern:** Existing Terraform `<domain>.tf` per-zone convention

### 3. Create per-zone file for eddacraft.dev

- **Checkpoint:** `infra/src/dns/eddacraft-dev.ts` declares records for docs domain
- **Validate:** `pulumi preview --stack prod --diff | grep "eddacraft-dev"`

### 4. Verify records match live configuration

- **Checkpoint:** Preview shows no unexpected changes against live Azure DNS
- **Validate:** `dig anvil.eddacraft.ai +short`

---

## IAC-008: Add CI/CD pipeline integration

### 1. Create infrastructure workflow file

- **Checkpoint:** `.github/workflows/infra.yml` defines preview and up jobs
- **Validate:** `test -f .github/workflows/infra.yml`
- **Pattern:** `.github/workflows/ci.yml` (existing workflow conventions)

### 2. Configure preview job for pull requests

- **Checkpoint:** PR trigger runs `pulumi preview` and posts comment
- **Validate:** `grep -q "pull_request" .github/workflows/infra.yml`

### 3. Configure up job for main branch merges

- **Checkpoint:** Push-to-main trigger runs `pulumi up --yes`
- **Validate:** `grep -q "pulumi up" .github/workflows/infra.yml`

---

## IAC-009: Write unit tests

### 1. Create test harness with Pulumi mocking

- **Checkpoint:** Vitest test file uses `pulumi.runtime.setMocks`
- **Validate:** `cd infra && pnpm exec vitest run`
- **Notes:** Pulumi mocking pattern:

```typescript
import * as pulumi from '@pulumi/pulumi';

pulumi.runtime.setMocks({
  newResource(args: pulumi.runtime.MockResourceArgs) {
    return { id: `${args.name}-id`, state: args.inputs };
  },
  call(args: pulumi.runtime.MockCallArgs) {
    return args.inputs;
  },
});
```

### 2. Test Vercel resource creation

- **Checkpoint:** Tests assert correct Vercel project and domain resources
- **Validate:** `cd infra && pnpm exec vitest run -- vercel`

### 3. Test GitHub resource creation

- **Checkpoint:** Tests assert correct GitHub secret and environment resources
- **Validate:** `cd infra && pnpm exec vitest run -- github`

### 4. Test Azure DNS record creation

- **Checkpoint:** Tests assert correct RecordSet resources per zone
- **Validate:** `cd infra && pnpm exec vitest run -- dns`

---

## IAC-010: Import existing resources

### 1. Create import helper script

- **Checkpoint:** `infra/scripts/fetch-vercel-ids.sh` retrieves project IDs from Vercel API
- **Validate:** `bash infra/scripts/fetch-vercel-ids.sh | grep -q "prj_"`
- **Notes:** Script should call `GET /v9/projects` with `$VERCEL_TOKEN`, filter by project name, and output `pulumi import` commands. This reduces the risk of manual ID lookup errors.

### 2. Retrieve existing Vercel project identifiers

- **Checkpoint:** Project IDs recorded in stack configuration
- **Validate:** `pulumi config get website-project-id --stack prod`

### 3. Run import commands for each resource

- **Checkpoint:** `pulumi import` completes without error for all resources
- **Validate:** `pulumi preview --stack prod --expect-no-changes`

### 4. Verify import completeness

- **Checkpoint:** No orphaned resources remain outside Pulumi management
- **Validate:** Compare `pulumi stack export | jq '.deployment.resources | length'` against expected resource count

---

## IAC-011: Document IaC workflow

### 1. Write infra README with contributor guide

- **Checkpoint:** `infra/README.md` covers setup, preview, and apply
- **Validate:** `test -f infra/README.md`

### 2. Update root CONTRIBUTING.md

- **Checkpoint:** CONTRIBUTING.md references infrastructure workflow
- **Validate:** `grep -q "infra" CONTRIBUTING.md`

---

## IAC-012: Document rollback procedures

### 1. Write rollback section in infra README

- **Checkpoint:** README covers rollback via `pulumi up` with previous commit
- **Validate:** `grep -q "rollback" infra/README.md`
- **Notes:** Pulumi has no built-in rollback command. The procedure is: revert the code change → run `pulumi up` to re-apply previous state. Document this explicitly including emergency procedures (manual Vercel dashboard as escape hatch).

### 2. Document state recovery procedures

- **Checkpoint:** README covers `pulumi stack export/import` for state corruption
- **Validate:** `grep -q "stack export" infra/README.md`

---

## Completion

- [ ] All checkpoints validated (IAC-007 through IAC-012)
- [ ] All tasks marked complete in source module
- [ ] `pulumi preview --stack prod --expect-no-changes` passes
- [ ] CI pipeline runs Pulumi preview on PRs
- [ ] Infrastructure state matches live environment
