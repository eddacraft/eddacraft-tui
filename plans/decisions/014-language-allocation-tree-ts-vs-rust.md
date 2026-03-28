# ADR-014: Language Allocation Tree — TypeScript vs Rust Kernel

## Status

Proposed

## Date

2026-03-17

## Context

Anvil is now a mixed-language system: TypeScript remains the orchestration and UX layer while Rust kernel capabilities are available for performance-critical paths. New modules risk inconsistent language choices unless an explicit allocation rule exists. The line-level authorship and confidence module is a near-term example where this decision must be durable beyond one implementation cycle.

## Decision

Adopt a stable language allocation tree for new and evolving features.

### Decision Tree

1. **Choose TypeScript by default** when the work is:
   - contract/schema orchestration
   - adapter and integration wiring
   - CLI/API UX surfaces
   - low-to-moderate compute paths that meet latency budgets

2. **Choose Rust kernel** when the work is:
   - CPU-bound and repeated on large inputs
   - hot-path diff/reconciliation/parsing/hashing
   - latency-critical under sustained workload
   - constrained by memory/throughput in TypeScript implementation

3. **Choose hybrid** when both are true:
   - feature needs fast iteration and broad integration (TS)
   - one or more inner loops exceed performance thresholds (Rust)

### Promotion Thresholds (TS -> Rust)

Promote a TypeScript path to Rust when one or more thresholds are breached in realistic workloads:

- p95 command latency exceeds target by >25% for two consecutive benchmarks
- module memory use exceeds 512MB in standard benchmark dataset
- per-PR (1k changed lines) attribution/reconciliation exceeds 2 seconds
- per-line query p95 exceeds 200ms in expected developer workflow

## Consequences

### Positive
- Preserves delivery speed by defaulting to TypeScript for orchestration and UX.
- Prevents premature Rust expansion while retaining clear upgrade criteria.
- Gives product and engineering a shared, testable language-choice policy.

### Negative
- Requires benchmark instrumentation discipline to trigger promotions objectively.
- Hybrid boundaries add interface maintenance overhead.

## Implementation Notes

- Reference this ADR from module plans requiring mixed-language choices.
- Include language choice rationale in module tasks touching hot paths.
- Re-evaluate thresholds quarterly as runtime and hardware assumptions evolve.

## Initial Application

Applies immediately to:
- `plans/modules/lineage-authorship-confidence.aps.md` (LAC)
