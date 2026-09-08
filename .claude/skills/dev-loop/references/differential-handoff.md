# Differential hand-off contract

Use this contract when `dev-loop-differential` delegates an advisor or verifier
role to another model, provider, or harness. Keep the hand-off bounded and
independent. Do not transfer the executor's conversation or reasoning.

## Common envelope

```markdown
## Differential Task

- Run: <run id>
- Target: <ReadyItem id or ad-hoc target>
- Role: advisor | verifier
- Question or claim: <one explicit question or completion claim>
- Governing sources: <accepted ADRs, policy, module spec, ReadyItem>
- Bounded change: <base..head, patch, or explicit paths>
- Allowed tools: <read and validation tools>
- Write boundary: read-only
- Required output: <advisor response or verification evidence bundle>
```

Rules:

1. Provide sources or repository paths the target harness can actually access.
2. Exclude executor messages, reasoning, self-assessment, and preferred answers.
3. Never use session transfer or transcript import for blind verification.
4. Do not widen a read-only task because an adapter defaults to write-capable
   delegation.
5. Treat returned command claims as untrusted until the orchestrator can match
   them to captured output and exit status.

## Advisor payload

Add:

```markdown
- Advisor type: design | risk | decomposition | conflict | taste
- Decision boundary: <what judgement changes>
- Constraints and non-goals:
```

Return:

```markdown
## Advisor Response

- Verdict: proceed | revise | escalate | blocked
- Top risks: <ranked, evidence-linked>
- Specific changes: <concrete>
- Assumptions:
- Confidence: low | medium | high
```

An advisor critiques and recommends. It never implements or advances loop state.

## Verifier payload

Add:

```markdown
- Acceptance criteria: <complete list>
- Required gates and commands:
- Risk: low | standard | high | critical
- Prior open findings: <ids only; omit executor discussion>
```

Return an evidence bundle matching `evidence-bundle.schema.json`. A verifier may
run tests and other validation tools but must not edit implementation. The
orchestrator maps adapter-native findings into the canonical severity and
decision vocabulary without weakening them.

## Adapter result handling

- Empty, malformed, timed-out, or non-zero adapter results are failed dispatches.
- Under `preferred`, try the next candidate, then use the role-specific
  same-harness fallback and record degradation.
- Under `required`, exhaust viable differential candidates, then stop `blocked`.
- Contradictory material results require a fresh differential decision or
  Council; never average them.
