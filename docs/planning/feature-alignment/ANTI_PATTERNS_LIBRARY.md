# ANTI_PATTERNS_LIBRARY — v1 Seed Catalogue

Principles:
- Default severity: high-confidence warning
- Every rule includes deterministic detection, short explanation, deterministic suggestions, and suppression support with reason.

Seed rules:
- AP-001 Broad eslint disable without reason
- AP-002 @ts-ignore/@ts-nocheck without reason
- AP-003 New `any` proliferation (threshold-based)
- AP-004 Blanket catch swallowing errors
- AP-005 TODO/FIXME as correctness escape hatch
- AP-006 Premature over-generalised abstraction (heuristic; start conservative)

Repo/org extensions:
- enable/disable rules, adjust thresholds, override messages/suggestions, add new rules
