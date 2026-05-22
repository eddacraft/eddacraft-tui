# Runbook Template

| Type    | Authority     | Owner       | Status | Freshness                                                                                       |
| ------- | ------------- | ----------- | ------ | ----------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | MODULE-CODE | Draft  | Last reviewed YYYY-MM-DD against source paths listed in [Source references](#source-references) |

| Upstream                                         | Downstream                                |
| ------------------------------------------------ | ----------------------------------------- |
| `scripts/example.sh`, `docs/policies/example.md` | Operators, release skills, support guides |

Use this template for operational procedures that humans or agents execute.
Runbooks are authority for procedures, not architecture rationale; link to ADRs,
APS, source code, and as-built docs instead of copying their content.

## When To Use

- Trigger: what event starts this procedure.
- Preconditions: repository state, credentials, command availability, or human
  approval needed before execution.
- Stop conditions: states where the operator must stop and escalate.

## Procedure

```bash
# command from repository root
scripts/example.sh --flag
```

Expected success signal:

```text
example completed
```

## Failure And Recovery

- Failure mode: how the command reports it.
- Recovery: exact retry, rollback, or escalation path.
- Safety note: what must not be skipped.

## Source References

- `scripts/example.sh` — executable procedure.
- `docs/policies/example.md` — policy boundary.
- `plans/modules/example.aps.md` — owning work module.

## Related Docs

- As-built: `docs/architecture/example-as-built.md`
- ADR: `plans/decisions/NNN-example.md`
- Public docs: `docs/public/anvil/example.md`

## How To Write One

1. Keep every command executable from the repository root unless stated
   otherwise.
2. Include expected success output and failure handling.
3. Cite source paths in backticks so `pnpm docs:check` can validate them.
4. Date the freshness row against the reviewed source paths, release record,
   incident, or successful dry-run.
