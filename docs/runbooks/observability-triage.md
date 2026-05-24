# Observability Triage Runbook

| Type    | Authority     | Owner | Status | Freshness                                                                                                                                                           |
| ------- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | OBS   | Live   | Command review 2026-05-25 against the health/waitlist probes below, `docs/observability/namespace-registry.md`, and `plans/modules/observability-foundation.aps.md` |

| Upstream                                                                                                                                    | Downstream                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Health/waitlist probe commands in this runbook, `docs/observability/namespace-registry.md`, `plans/modules/observability-foundation.aps.md` | Incident triage, post-deploy checks, release support |

## Purpose

Provide a standard first-15-minutes triage flow for incidents using Anvil
observability signals.

## When to use

- Errors increase after deploy
- Users report failures but root cause unclear
- Dashboard shows degraded health
- Alert fired for latency/error budget breach

## Required access / env vars

- Access to logs/metrics dashboards
- Access to API and website health endpoints
- Access to Neon + Resend dashboards (if relevant)

## Exact commands

### 1) Establish blast radius quickly

```bash
curl -sS https://<api-host>/health
curl -sS https://<site-host>/api/waitlist \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"email":"triage-test@example.com"}'
```

### 2) Classify failure domain

- **API**: high 5xx, auth failures, timeout spikes
- **DB (Neon)**: connection failures, query latency, transaction errors
- **Email (Resend)**: delivery failures, provider rejection, auth/config errors
- **Frontend**: client errors, endpoint mismatch, UX failure handling

### 3) Capture minimal triage packet

- Timestamp window
- Affected endpoints
- Error signature(s)
- Current severity (sev1 / sev2 / sev3)
- Suspected owner domain (API/DB/Email/Frontend)

### 4) Stabilise, then diagnose

- Apply lowest-risk stabilisation first (traffic shaping, retries, toggles)
- Confirm customer-facing path recovers
- Continue deeper cause analysis after stabilisation

## Expected success output

- Incident domain classified within 15 minutes
- Customer-critical path restored or isolated
- Clear handoff packet available for deeper remediation

## Failure modes + recovery

1. **No clear signal source**
   - Recovery: force synthetic request through full path and trace each hop.

2. **Multiple simultaneous failures**
   - Recovery: isolate by critical path first (signup/auth/core API), defer
     non-critical noise.

3. **False-positive alerting**
   - Recovery: verify against real user path + endpoint health before
     escalation.

## Rollback / safety notes

- No schema migrations during active triage unless explicitly approved.
- Avoid broad config changes without a rollback command prepared.
- Log every mitigation action with timestamp for post-incident review.
