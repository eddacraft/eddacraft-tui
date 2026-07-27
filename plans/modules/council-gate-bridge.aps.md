# Council Gate Bridge

| ID    | Owner  | Status | Progress |
| ----- | ------ | ------ | -------- |
| CGBDG | @aneki | Ready  | 0/6      |

**Status:** Ready — the blocker is cleared (2026-06-24). MLP-002 (witness chain)
shipped **Done 2026-05-13** (archived via `v0.7.0-beta`), and the
witness-schema-stabilising follow-ups it was waiting on are now terminal:
**MLP2-011 (DAG-aware merge verification) Released/Shipped** and **MLP2-012
(manifest event stream, the MLP-002b follow-up) Merged** — so the witness format
the bridge would target is stable. Discovery (CGBDG-001..006, all Ready) may now
begin. **Re-evaluate scope first:** CGBDG may collapse to "council emits witness
lines" over the now-stable witness chain rather than a separate attestation
bridge — CGBDG-001 and CGBDG-004 own that call.

**Programme (2026-07-28):** Active track of
[Graph Trust Surfaces](../specs/2026-07-28-graph-trust-surfaces.md). Index hub
row + NBI rank 2. Option B promotion: module stays **Ready** (already
executable); discovery is authorised to start without further APS status
change. Follow-on **implementation** still requires CGBDG-006's discovery
report and a separate implement work-item set — this module remains
discovery-only until then.

**Last reviewed:** 2026-07-28 — Graph Trust Surfaces programme affiliation;
unchanged Ready disposition from 2026-06-24 unblock. Discovery-only;
attestation work still lives at `packages/anvil/core/src/provenance/`. If/when
attestation moves to a Rust crate (e.g. `crates/anvil-checks`), update
CGBDG-001 accordingly.

## Purpose

Investigate and define how the council review flow (LLM-powered dev-time code
review) connects to Anvil's deterministic gate at runtime. The goal is a bridge
that turns a council-judge JSON verdict into an Anvil-format attestation — so
the development-time review and the runtime gate speak the same language, and
there is a provenance chain from council review → attestation → production.

This module is discovery-only. No implementation without a follow-on spec.

## Background

The council flow (`/council-full`) runs five Claude Code reviewer personas in
parallel, supervises their output for quality, debates contradictions, and
synthesises a BLOCK/WARN/PASS verdict with a structured JSON output. This
output currently lives only in the Claude Code session — it is not persisted,
not signed, and not connected to Anvil.

Anvil's existing attestation format (Rust core, provenance-service) is
deterministic and signed. The question is whether a thin bridge layer can
consume the council-judge JSON and emit a valid Anvil attestation that the
runtime gate can verify.

## Boundaries

**In scope:**
- Understand Anvil's existing attestation schema and signing model
- Understand the council-judge JSON output format
- Map severity taxonomy (council: critical/major/minor/nit → Anvil: policy violation levels)
- Identify the minimal bridge interface (what goes in, what comes out)
- Identify where the bridge lives (code-env script, Rust binary, Anvil plugin)
- Identify whether the planned PocketFlow TS adapter (documented but not yet
  vendored — see `docs/architecture/references/pocketflow-vendoring.md`) is the
  right orchestration layer for the full council flow, or whether the current
  Claude Code agent approach is sufficient

**Out of scope:**
- Implementing the bridge
- Modifying Anvil's existing attestation format
- Adding LLMs to Anvil's runtime enforcement path (Anvil stays deterministic)

## Work Items

| ID          | Task                                          | Status  |
|-------------|-----------------------------------------------|---------|
| CGBDG-001   | Read provenance-service and attestation schema | Ready   |
| CGBDG-002   | Document council-judge output format           | Ready   |
| CGBDG-003   | Map severity taxonomies                        | Ready   |
| CGBDG-004   | Define bridge interface (inputs/outputs)       | Ready   |
| CGBDG-005   | Evaluate PocketFlow TS adapter vs agent approach | Ready   |
| CGBDG-006   | Write discovery report + follow-on spec        | Ready   |

### CGBDG-001 — Read provenance-service and attestation schema

- **Checkpoint:** Attestation schema fields and signing mechanism documented
- **Validate:** Discovery doc has schema summary

### CGBDG-002 — Document council-judge output format

- **Checkpoint:** council-judge JSON fields mapped against attestation schema
- **Validate:** Field mapping table exists in discovery doc

### CGBDG-003 — Map severity taxonomies

- **Checkpoint:** council severity levels (critical/major/minor/nit) mapped to Anvil policy violation levels
- **Validate:** Mapping table in discovery doc, edge cases noted

### CGBDG-004 — Define bridge interface

- **Checkpoint:** Bridge interface defined: input schema, output schema, signing approach
- **Validate:** Interface definition in discovery doc

### CGBDG-005 — Evaluate PocketFlow TS adapter vs agent approach

- **Checkpoint:** Recommendation documented: PocketFlow TS adapter (planned, not yet vendored), Claude Code agents, or hybrid — with rationale
- **Validate:** Recommendation in discovery doc with tradeoffs

### CGBDG-006 — Write discovery report + follow-on spec

- **Checkpoint:** Discovery report written, follow-on implementation spec drafted if warranted
- **Validate:** `plans/specs/YYYY-MM-DD-council-gate-bridge-discovery.md` exists

## Dependencies

- **Blocks on:** MLP-002 (witness chain) — **met (2026-06-24)**. MLP-002 is
  `Done` (2026-05-13) and the witness-schema follow-ups MLP2-011 (DAG-aware
  verification, Released/Shipped) and MLP2-012 (manifest stream, Merged) are
  terminal, so the witness format is stable. Witness lines may replace the
  bridge's attestation target — confirm in CGBDG-001/-004.
- Existing: `packages/anvil/core/src/provenance/` (attestation schema)
- Existing: `.claude/commands/council.md` and `.claude/agents/plan-synthesizer.md` (council output format)
- Planned: PocketFlow TS adapter (`packages/kindling-adapter-pocketflow/`; documented in `docs/architecture/references/pocketflow-vendoring.md` but not yet vendored)

## Risks

- The attestation schema may require signing keys that don't exist in the dev
  environment — if so, the bridge may need to emit unsigned dev attestations
  with a flag, not production attestations
- The council flow produces probabilistic LLM output; Anvil's gate is
  deterministic. The bridge must never allow the LLM path to override a
  deterministic policy decision — it can add context, not override verdicts
