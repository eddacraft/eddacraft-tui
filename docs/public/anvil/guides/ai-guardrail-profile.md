---
id: ai-guardrail-profile
title: AI Guardrail Profile
description:
  Curated structural-governance checks for AI-assisted workflows, with a stable
  JSON envelope for tool consumption.
sidebar_position: 4
---

# AI Guardrail Profile

The AI guardrail profile (`anvil gate --profile ai`) bundles the
structural-governance checks an AI-assisted workflow needs — secret detection,
import boundaries, antipatterns, OPA policy, command safety — into a single
invocation with a stable JSON envelope. External AI tools (Claude Code, MCP
servers, custom agent harnesses) call into it the same way every time and get
the same shape back.

This guide covers what the profile does, how to wire it into an AI workflow, and
the contract you can safely build against.

## When to Use It

Reach for `--profile ai` when:

- You want a single command that validates AI-generated changes against the
  structural rules anvil enforces, with no per-call configuration.
- You're integrating anvil into an MCP server, agent harness, or CI step that
  consumes JSON.
- You want strict failure semantics — missing or invalid configuration should
  block the run, not silently skip checks.

For everyday local runs without an AI in the loop, prefer
`anvil gate --profile dev` or the default profile.

## What Runs

The profile selects a curated allow-list rather than the inverse skip list other
profiles use, so the curated rule set is the floor — even a project `.anvilrc`
that enables additional checks doesn't widen the run.

| Check               | Purpose                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------- |
| `secret-detection`  | Catches leaked credentials and API keys before they hit a remote.                           |
| `import-boundaries` | Enforces architectural layering declared in `.anvil/architecture.yaml`.                     |
| `antipattern-scan`  | Flags reasoning antipatterns AI tools regularly emit (`any` types, swallowed errors, etc.). |
| `policy`            | Evaluates OPA policy bundles in `.anvil/policies/`.                                         |
| `command-safety`    | Inspects fenced shell-script blocks in plan files for dangerous commands.                   |

Toolchain-shaped checks (`lint`, `test`, `coverage`, `dependency`) are
deliberately excluded — host projects already enforce those, and including them
would push the profile past the < 5s budget the AI guardrail acceptance criteria
pin.

## Strict Configuration

Under `--profile ai`, missing or invalid project configuration is **blocking**,
not a soft skip. A repository with no `.anvil/architecture.yaml` will fail the
architecture check rather than pass it with a "no config, skipping" message. The
same applies to the policy bundle. `command-safety` analyses fenced shell-script
blocks from a supplied plan file; running the gate without a plan, or with the
check disabled, is also treated as a config gap and blocks under `--profile ai`.

**Host-tooling gaps are not config gaps.** Missing OPA on the runner is not
elevated under strict mode — it is reported as a host environment issue rather
than a project posture failure. Install OPA (`anvil doctor` will tell you how)
to actually run the policy check, but the absence of a binary will not by itself
fail an AI-guardrail run.

Why: an AI tool that asked anvil "is this codebase governed?" needs a truthful
answer. Silently passing because no config exists makes the guardrail look
effective when it isn't. Strict mode surfaces the gap so the operator can either
configure the check or explicitly opt out via `--skip-checks`.

## JSON Envelope

`--profile ai` defaults to JSON output. The wire format is the canonical
`anvil.gate-result.v1` envelope, which wraps a list of `anvil.diagnostic.v1`
payloads — the inner shape pinned by the [diagnostic envelope coordination
spec][envelope-spec] and exported from
`crates/anvil-kernel-types/src/diagnostics.rs`.

[envelope-spec]:
  https://github.com/eddacraft/anvil-001/blob/main/plans/specs/2026-04-26-diagnostic-envelope-coordination.md

### Example Invocation

```bash
anvil gate --profile ai
```

### Example Output

```json
{
  "schema": "anvil.gate-result.v1",
  "exit_code": 2,
  "summary": {
    "total": 1,
    "by_severity": { "error": 1, "warning": 0, "info": 0 },
    "by_category": { "secret": 1 },
    "overall_passed": false,
    "score": 0.0
  },
  "diagnostics": [
    {
      "schema_version": "anvil.diagnostic.v1",
      "id": "diag_gate_secret-detection",
      "severity": "error",
      "summary": "Potential secrets found in 1 location(s):",
      "location": { "file": "<workspace>" },
      "category": "secret",
      "source": {
        "rule_id": "gate-secret-detection",
        "source_module": "anvil-cli::gate::secret-detection"
      },
      "remediation_hint": "Potential secrets found in 1 location(s):\nsrc/api/client.ts:42 [AWS Secret Key]",
      "mode": "gate"
    }
  ],
  "duration_ms": 420
}
```

### Envelope Field Reference

| Field                    | Type    | Notes                                                                                          |
| ------------------------ | ------- | ---------------------------------------------------------------------------------------------- |
| `schema`                 | string  | Always `anvil.gate-result.v1` for the AI guardrail profile.                                    |
| `exit_code`              | integer | Mirrors the CLI exit code (`0` = pass, `2` = at least one check failed).                       |
| `summary.total`          | integer | Number of failing diagnostics in the run.                                                      |
| `summary.by_severity`    | object  | Count per severity (`error`, `warning`, `info`).                                               |
| `summary.by_category`    | object  | Count per category (`secret`, `antipattern`, `boundary`, `policy`, `command-safety`, `other`). |
| `summary.overall_passed` | boolean | Convenience derived from `exit_code`.                                                          |
| `summary.score`          | number  | Aggregate score (0–100) across all checks that ran.                                            |
| `diagnostics[]`          | array   | Inner-shape `anvil.diagnostic.v1` payloads.                                                    |
| `duration_ms`            | integer | Wall-clock duration of the run.                                                                |

Each diagnostic carries the canonical fields documented in the envelope spec:
`id`, `severity`, `summary`, `location`, `category`, `source`, optional
`remediation_hint`, and `mode = "gate"`. New optional fields may be added
without bumping the schema version per the spec's forward-compatibility rules;
consumers should ignore unknown fields rather than fail.

## Exit Codes

The profile uses anvil's standard exit codes:

| Code | Meaning                                                            |
| ---- | ------------------------------------------------------------------ |
| `0`  | All gates passed.                                                  |
| `1`  | General error (invalid arguments, IO failure).                     |
| `2`  | One or more gate checks failed — read `diagnostics[]` for details. |
| `4`  | Configuration error (malformed `.anvilrc`, invalid profile).       |

## Wiring It Into an AI Workflow

### MCP Tool Integration

Expose `anvil gate --profile ai` as an MCP tool. The JSON envelope is
self-describing, so the LLM can branch on `summary.by_category` to prioritise
its remediation order without reading every diagnostic.

```jsonc
{
  "name": "anvil_gate_ai",
  "description": "Run anvil's AI guardrail profile on the workspace",
  "inputSchema": { "type": "object", "properties": {} },
}
```

When the tool returns, parse stdout as `anvil.gate-result.v1` and feed `summary`
plus the highest-severity diagnostics back into the model turn.

### Pre-Commit Validation

Run the profile as a pre-commit step in agent-driven workflows:

```bash
anvil gate --profile ai || exit 2
```

The non-zero exit code signals the agent harness to surface the JSON envelope to
the model and request a fix before the commit is finalised.

### CI Integration

Pair the profile with `--no-tui` to force plain output when JSON isn't needed,
or feed the JSON straight into a status reporter:

```bash
anvil gate --profile ai > gate.json
jq '.summary' gate.json
```

## Customising the Run

Most flags compose with the profile:

- `--skip-checks <name>` — opt out of a specific curated check (still validated
  against the catalogue).
- `--only-checks <name>` — intersect with the AI guardrail allow-list; the
  result is still bounded by the curated set.
- `--fail-fast` — stop on the first failing check.
- `--progress` — emit per-check progress lines on stderr; useful when driving a
  long-running agent loop and you want incremental updates.

What you cannot do via the profile:

- Re-enable the toolchain checks (`lint`, `test`, `coverage`, `dependency`). Run
  them explicitly via `--profile ci` or `--only-checks` outside the AI profile.

## Related Reading

- [Agent Harness Patterns](./agent-harness.md) — broader patterns for using
  anvil as a constraint layer around AI agents.
- [Solo Dev Flow](./solo-dev-flow.md) — non-AI everyday workflow.
- [Team Flow](./team-flow.md) — CI integration patterns that compose with the AI
  guardrail.
