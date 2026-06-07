<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# LLM Provider Semantic Pack (Track 4 — Phase 1 + Phase 2)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| PACKLLM | —     | Draft  |

**Last reviewed:** 2026-04-26

> Note (2026-04-26): "TS / Python substrate" = the languages being analysed.
> The pack itself ships as a Rust crate `crates/anvil-pack-llm-provider/` per
> [ADR-027](../decisions/027-pack-architecture.md).

## Purpose

Per [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§8.4 row 4 and §8.4.1, this is **the most strategically valuable pack in the
entire design**. It is the pack that proves Anvil's "AI/ML governance" story
from the TS side without waiting for Python to reach T3, and it is the
single pack that can be demo'd live to a prospect.

- **Phase 1**: TS substrate. Targets `openai`, `@anthropic-ai/sdk`, and
  `ai` (Vercel AI SDK) imports. Demand: 1 (User B).
- **Phase 2**: Python substrate extension. Targets `openai`, `anthropic`,
  `langchain` family. Demand: 1 (User C). Unblocks after `lang-python`
  reaches T2 (spec §9 step 11). Reuses the rule catalogue where semantics
  align; substrate-specific rules added only where the language forces
  them (e.g. `langchain` has no TS equivalent).

**Warn-only by default** per council finding C-010 — static PII detection
in LLM calls is heuristic, and a false positive could break a production
call path, not just a build. Projects opt into hard-fail per-rule via the
standard policy-hook mechanism once their FP profile is known.

## In Scope

- Substrate languages: TS (Phase 1), Python (Phase 2).
- Minimum substrate tiers: TS T3 (Phase 1), Python T2+ (Phase 2).
- Pack activation: detect imports of named provider SDKs.
- Rule catalogue (per spec §8.4 row 4):
  - PII in prompt construction (heuristic — emails, phone numbers, IDs
    in template strings)
  - Hardcoded system prompts conflicting with declared policy
  - Uncapped `max_tokens` on provider calls
  - Unsanitised response rendering (injection-shaped risks for downstream
    HTML / SQL / shell)
  - Missing streaming cancellation (orphan stream risk)
  - Tool calls without JSON schemas
- Warn-only severity by default; opt-in to hard-fail per rule.
- One-pack-multiple-substrates default per spec §12.9 — one `pack-llm-provider`
  module covers both TS and Python rule catalogues. Final
  one-pack-vs-two decision documented inside this module if it becomes
  load-bearing.
- Live-demo target: paste a Vercel AI SDK code sample, watch Anvil flag
  PII. This is the demonstrability requirement for §8.4.1 and §14.6.

## Out of Scope

- Real PII classification beyond heuristic patterns (no NER / ML models).
- Provider response-content scanning (not in static analysis scope).
- LLM-output content moderation.
- Token-usage telemetry / cost analysis.
- Prompt-quality / prompt-engineering linting.

## Interfaces

**Depends on:**

- [`lang-ts-audit`](../archive/modules/lang-ts-audit.aps.md) — Phase 1 substrate.
- [`lang-python`](./lang-python.aps.md) — Phase 2 substrate (T2+).
- [`pack-pulumi`](./pack-pulumi.aps.md) — first consumer of the pack
  architecture; this pack is second to ship.
- [ADR-027](../decisions/027-pack-architecture.md) — pack architecture
  (symbol-graph access required for the PII heuristic and the
  call-construction-vs-invocation distinction).
- Existing OPA pipeline.

**Exposes:**

- LLM Provider rule catalogue (the AI/ML governance story made concrete).
- The first multi-substrate pack — sets the pattern for any future
  cross-language pack.

## Prerequisites

- `lang-ts-audit` complete (Phase 1).
- `lang-python` at T2+ before Phase 2 work starts.
- [ADR-027](../decisions/027-pack-architecture.md) Accepted.

## Ready Checklist

**Phase 1 (TS substrate)** — change status to **Ready** when:

- [ ] LANGTS complete.
- [ ] ADR-027 Accepted; PACKPUL crate skeleton landed (PACKPUL-001) so
      this pack inherits the registry + activation pattern.
- [ ] Warn-only-by-default policy-hook mechanism confirmed.
- [ ] Acceptance bar agreed (FP rate < N% on a real LLM-using TS codebase
      AND demo-able to a prospect).
- [ ] Owner named.

**Phase 2 (Python extension)** — change status to **Ready** when:

- [ ] PYLAN at T2+.
- [ ] Phase 1 shipped and FP profile known.
- [ ] Decision recorded: one pack with two substrates, or two packs.

## Work Items

Anticipated (Phase 1):

- PACKLLM-001: TS-side provider-import detection (`openai`,
  `@anthropic-ai/sdk`, `ai`).
- PACKLLM-002: PII-in-prompt heuristic.
- PACKLLM-003: Token / cancellation / tool-schema rule set.
- PACKLLM-004: Hardcoded-system-prompt vs declared-policy rule.
- PACKLLM-005: Warn-only severity wiring + per-rule hard-fail opt-in.
- PACKLLM-006: Live-demo fixture and validation against a real TS LLM
  codebase.

Anticipated (Phase 2):

- PACKLLM-101: Python-side provider-import detection (`openai`,
  `anthropic`, `langchain`).
- PACKLLM-102: Reuse rule catalogue from Phase 1 where semantics align;
  add Python-specific rules where forced.
- PACKLLM-103: One-pack-vs-two decision recorded.
- PACKLLM-104: Validation against User C's codebase.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| PII heuristic FPs break production call paths if not warn-only (council C-010) | Critical | Warn-only by default — non-negotiable; hard-fail is opt-in only |
| One-pack-vs-two decision deferred indefinitely | Medium | Decide in Phase 2 Ready check; document rationale |
| Demo fragility — pack must work live in front of prospects (§8.4.1) | High | Curated demo fixture; rehearse before prospect-facing demos |
| Pack must distinguish provider client construction vs invocation in heuristic | Medium | Symbol-graph access (per pack architecture ADR) makes this tractable |

## Open Questions

- [ ] One module covers TS + Python, or split into PACKLLM-TS / PACKLLM-PY?
- [ ] Heuristic PII patterns — start narrow (regex shapes only) and expand,
      or include common identifier formats (SSN-shaped, etc.)?
- [ ] Should this pack interop with policy-hooks for "PII patterns are
      defined per-org" customisation?
