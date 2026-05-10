# Planning Council

Use this skill when APS planning needs multi-role judgement before it becomes
execution authority, or when repo reality invalidates an existing plan.

Planning Council follows the operating-model lifecycle:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

It provides judgement evidence only. APS files remain the authority for intent
and readiness; deterministic checks remain validation authority.

Reference documents:

- `plans/aps-rules.md`
- `plans/specs/2026-05-09-plan-build-release-operating-model.md`
- `plans/specs/2026-05-09-council-agent-skill-change-proposal.md`

## Modes

| Mode | Playbook | When |
| --- | --- | --- |
| Create | `playbooks/plan-create.md` | New module, spec, or multi-item plan. |
| Direction validate | `playbooks/direction-validate.md` | Before Draft/Proposed planning becomes Ready. |
| Pre-execution validate | `playbooks/pre-execution-validate.md` | Before non-trivial Ready work starts. |
| Amend | `playbooks/plan-amend.md` | Repo reality, review, or validation changes the plan. |

## Role Lenses

Store stable role names separately from runtime agent IDs.

| Stable role | Runtime agent |
| --- | --- |
| `planning-synthesizer` | `plan-synthesizer` |
| `pragmatic` | `pragmatic-lead` |
| `operations` | `operations-reviewer` |
| `security` | `council-reviewer` |
| `adversarial` | `adversarial-reviewer` |

- `planning-synthesizer`: synthesises the plan and proposes APS updates.
- `pragmatic`: checks proportionality, sequencing, and execution cost.
- `operations`: checks CI, release, recovery, and observability impact.
- `security`: checks trust boundaries, secrets, policy, and abuse cases.
- `adversarial`: challenges assumptions, dependencies, and failure paths.

## Required Outputs

Every Planning Council pass should produce:

- decision: `proceed`, `amend`, `split`, `replan`, or `block`
- APS items reviewed
- repo reality checked: base branch, changed files, relevant specs/docs
- risks and unresolved questions
- required deterministic checks
- plan file updates required before execution, if any

If the decision is not `proceed`, stop execution and update APS first.
