# Runbooks

Operational runbooks for repeatable admin/support workflows.

## Available Runbooks

- [Waitlist Email Operations](../waitlist-email-operations.md) — preview
  templates, test delivery, and force resend confirmations via admin endpoint.
- [Neon DB Operations](neon-db-operations.md) — diagnose and recover Neon
  database incidents.
- [Observability Triage](observability-triage.md) — first-15-minutes incident
  triage flow.
- [Post-Deploy Smoke Check](post-deploy-smoke-check.md) — verify critical paths
  after deployments.

## Runbook Standard (lightweight)

Each runbook should include:

1. **Purpose**
2. **When to use**
3. **Required access/env vars**
4. **Exact commands**
5. **Expected success output**
6. **Failure modes + recovery**
7. **Rollback / safety notes**

Keep runbooks task-oriented and copy/paste friendly.
