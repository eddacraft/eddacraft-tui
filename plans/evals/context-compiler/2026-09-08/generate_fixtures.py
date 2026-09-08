#!/usr/bin/env python3
"""Author CCTX-003 V3/V4 Decision Brief fixtures against frozen spec §9.

Eval artefacts only. Not a product compiler, runtime schema, or flag.
"""

from __future__ import annotations

from pathlib import Path

CORPUS = "23457dc6d2bf379791d587cf2dfdb5046ce51fc0"
AUTHORED = "2026-09-08T14:30:00Z"
SYNTHESIS = "cctx-003-fixture-2026-09-08"
SPEC = "plans/specs/2026-09-08-context-compiler.md"
OUT = Path(__file__).resolve().parent / "fixtures"

AUTHORITY = """A Decision Brief is **advisory context**. It is never policy
authority. It is **never** allow, warn, or block. It is **never** a substitute
for `anvil_validate_write` or the deterministic policy / check engine.
Enforcement must keep working if Context Compiler is unavailable, stale, or
wrong. A fluent brief that says an edit is safe does not permit skipping
launch validation (corpus task T13)."""

REQUIRED_SECTIONS = [
    "Task interpretation",
    "Scope",
    "Authority",
    "Relevant components",
    "Relationships",
    "Invariants and applicable policy",
    "Likely blast radius",
    "Tests and validation",
    "Prior ADRs",
    "Risks and unknowns",
    "Claims",
    "Freshness",
    "Completeness",
    "Confidence",
    "Drill-down handles",
]


def claim(cid: int, kind: str, statement: str, evidence: list[str], depends: str | None = None) -> str:
    lines = [
        f"### Claim {cid}",
        "",
        f"- **Id:** {cid}",
        f"- **Kind:** {kind}",
        f"- **Statement:** {statement}",
        "- **Evidence:**",
    ]
    for e in evidence:
        lines.append(f"  - {e}")
    if depends:
        lines.append(f"- **Depends on:** {depends}")
    return "\n".join(lines)


def brief(
    *,
    tid: str,
    cls: str,
    prompt: str,
    scope: str,
    components: str,
    relationships: str,
    invariants: str,
    blast: str,
    tests: str,
    adrs: str,
    risks: str,
    claims: list[str],
    completeness: str,
    completeness_note: str,
    v3_handles: list[str],
    v4_gctx: list[str],
    recall_only: list[str],
    invalidation: str = "current",
) -> str:
    claims_md = "\n\n".join(claims)
    v3 = "\n".join(f"- `{h}`" for h in v3_handles)
    v4 = "\n".join(f"- `{h}`" for h in v4_gctx) or "- (none beyond §12.2 default set)"
    recall = "\n".join(f"- `{h}` — recall key, not a V3 selected input" for h in recall_only) or "- none"
    return f"""# Decision Brief {tid}

Eval fixture for spike variants **V3** (brief only) and **V4** (brief plus
listed GCTX drill-down). Not a product compiler. Synthesis version
`{SYNTHESIS}/{tid}`.

## Task interpretation

- **Task id:** `{tid}`
- **Task class:** {cls}
- **Restated prompt:** {prompt}

## Scope

{scope}

## Authority

{AUTHORITY}

## Relevant components

{components}

## Relationships

{relationships}

## Invariants and applicable policy

{invariants}

## Likely blast radius

{blast}

## Tests and validation

{tests}

The brief itself is **not** a validation gate.

## Prior ADRs

{adrs}

## Risks and unknowns

{risks}

## Claims

{claims_md}

## Freshness

| Field | Value |
| ----- | ----- |
| Source revision | `{CORPUS}` (spec §12.1 corpus freeze) |
| Synthesis version | `{SYNTHESIS}/{tid}` |
| Age / authored-at | `{AUTHORED}` |
| Invalidation state | `{invalidation}` |
| Evidence dependencies | Handles listed on each claim and in drill-down. If a supporting path changes after the corpus revision, this brief is `stale` or `partial` — do not quietly fall back. |

## Completeness

**{completeness}.** {completeness_note}

## Confidence

Qualitative only (spec §15.4 parked): **unmeasured**. No numeric score.

## Drill-down handles

Handles are not permission. V3 may open **only** the V3 paths. V4 may also
call the listed GCTX tools / `graph://` resources from spec §12.2, identity-only
unless a run record states egress consent. Unbounded repository search remains
forbidden. `anvil_validate_write` remains available on every variant.

### V3 (paths the agent may open)

{v3}

### V4 (additional GCTX handles)

{v4}

Default V4 GCTX surface (spec §12.2): `anvil_search_symbols`,
`anvil_find_dependents`, `anvil_impact_of_change`, `anvil_affected_tests`,
`anvil_find_callers`, `anvil_symbol_context` (identity-only), `graph://stats`,
`graph://symbols`, `graph://edges`.

### Recall keys (not V3 selected inputs)

{recall}
"""


TASKS: list[dict] = []


def add(task: dict) -> None:
    TASKS.append(task)


add(
    dict(
        tid="T01",
        cls="orientation",
        prompt=(
            "Before editing, explain how graph context differs from launch "
            "validation. When do you call `anvil_validate_write` versus a GCTX tool?"
        ),
        scope=(
            "Covers the GCTX versus launch-validation split on the MCP surface. "
            "Refuses product-compiler architecture and any reading of a GCTX "
            "answer as allow / warn / block."
        ),
        components=(
            "- `docs/guides/ai-context-delivery.md` — operator guide; heading "
            "`Graph context is not launch validation`.\n"
            "- `docs/architecture/graph-context-delivery-spec.md` — GCTX projection contract.\n"
            "- `anvil_validate_write` — enforcement gate; not a GCTX tool."
        ),
        relationships=(
            "GCTX tools are read-only projections over the resident graph. "
            "`anvil_validate_write` is a separate enforcement half of the same "
            "MCP server. Complementary sequence: understand with GCTX, then "
            "validate a write."
        ),
        invariants=(
            "A GCTX answer is context, never a decision, and never blocks. "
            "Launch validation vocabulary is four-valued (`allow`, `warn`, "
            "`block`, `gateUnavailable`). Crossing the split is the failure "
            "mode Context Compiler exists to avoid."
        ),
        blast=(
            "No code change is required to answer. If an agent skipped "
            "`anvil_validate_write` because a brief looked safe, that would "
            "be authority leakage (T13), not this orientation task."
        ),
        tests=(
            "- Read the guide heading and the delivery spec default posture.\n"
            "- Keep `anvil_validate_write` available; do not treat this brief as a gate."
        ),
        adrs="none in selected inputs for the split itself (GCTX-032 / guide). ADR-083 is MCP delivery affinity.",
        risks=(
            "Agents shorthand launch validation as 'RMCP validation'. That does "
            "not merge the two halves. Spec §15 remains parked and is out of scope here."
        ),
        claims=[
            claim(
                1,
                "Deterministic fact",
                "The AI context delivery guide splits launch validation (`anvil_validate_write`) from graph context: the former is the enforcement gate; the latter is read-only projection that returns context, never a decision, and never blocks.",
                [
                    "`docs/guides/ai-context-delivery.md` heading `Graph context is not launch validation`",
                    "`docs/architecture/graph-context-delivery-spec.md`",
                ],
            ),
            claim(
                2,
                "Deterministic fact",
                "Call `anvil_validate_write` when about to change code; call GCTX tools when trying to understand the codebase.",
                ["`docs/guides/ai-context-delivery.md` (rule of thumb under that heading)"],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Before editing, pull GCTX context if needed, then keep `anvil_validate_write` on the write path. Do not treat this brief as allow / warn / block.",
                ["`docs/guides/ai-context-delivery.md`", "this brief Authority section"],
                depends="1, 2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Gold paths are on the §12.3 allowlist.",
        v3_handles=[
            "docs/guides/ai-context-delivery.md",
            "docs/architecture/graph-context-delivery-spec.md",
        ],
        v4_gctx=["anvil_search_symbols (identity-only locator: anvil_validate_write / GctxProjector)"],
        recall_only=[],
    )
)

add(
    dict(
        tid="T02",
        cls="orientation",
        prompt="How does a workspace opt in to GCTX snippet egress, and what is the default if it does not?",
        scope=(
            "Covers CE-1 identity-only default and the consented `anvil gctx egress enable` "
            "path. Refuses flipping the default to snippets-on (that is T12) and any "
            "assumption that this eval session has egress consent."
        ),
        components=(
            "- `docs/guides/ai-context-delivery.md` — enable/disable/status CLI.\n"
            "- `crates/anvil-gctx-types/src/lib.rs` — `egress_resolve_default_is_identity_only`.\n"
            "- `docs/architecture/graph-context-delivery-spec.md` — CE-1."
        ),
        relationships=(
            "Snippet text is off unless the workspace records consent. Default "
            "eval posture for this spike is identity-only (spec §12.2)."
        ),
        invariants=(
            "CE-1: identity-only default. Kill-switch (`ANVIL_GCTX_EGRESS=0`) is "
            "a distinct source from never-enabled default so status can tell them apart."
        ),
        blast="Opt-in is per-workspace consent, not a GCTX DTO change. No compiler, no flag flip in this spike.",
        tests=(
            "- `egress_resolve_default_is_identity_only` in `crates/anvil-gctx-types/src/lib.rs`.\n"
            "- Do not claim this eval run has egress consent unless a run record says so."
        ),
        adrs="ADR-083 (MCP delivery) affinity; CE-1 lives in the GCTX spec.",
        risks="Snippet egress without recorded consent would violate CE-1. This spike does not record consent.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "With no env overlay and no persisted consent, `resolve_snippet_egress` returns `(SnippetEgress::IdentityOnly, EgressSource::Default)`.",
                ["`crates/anvil-gctx-types/src/lib.rs` test `egress_resolve_default_is_identity_only`"],
            ),
            claim(
                2,
                "Deterministic fact",
                "A workspace opts in with `anvil gctx egress enable` (prints consequence, asks for confirmation) and reverts with `anvil gctx egress disable`.",
                ["`docs/guides/ai-context-delivery.md`"],
            ),
            claim(
                3,
                "Recommendation",
                "Treat this eval session as identity-only unless a run record states `anvil gctx egress enable` was consented.",
                ["spec §12.2 GCTX surface pin", "`docs/architecture/graph-context-delivery-spec.md` CE-1"],
                depends="1, 2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="CLI command name is a gold recall key; the guide on the allowlist documents it.",
        v3_handles=[
            "docs/guides/ai-context-delivery.md",
            "docs/architecture/graph-context-delivery-spec.md",
            "crates/anvil-gctx-types/src/lib.rs",
        ],
        v4_gctx=["anvil_symbol_context (identity-only; snippet text off)"],
        recall_only=["anvil gctx egress enable"],
    )
)

add(
    dict(
        tid="T03",
        cls="orientation",
        prompt="Where does the resident graph live, and which crate owns the sealed egress DTOs and `GctxProjector`?",
        scope=(
            "Covers crate ownership on the GCTX-010 spine. Refuses re-implementing "
            "GCTX or treating projector output as enforcement."
        ),
        components=(
            "- `crates/anvil-graph-cache` — resident `SymbolGraph` / mutation.\n"
            "- `crates/anvil-gctx-types` — sealed egress DTOs; CE-5 no-leak tests.\n"
            "- `crates/anvil-gctx-egress` — `GctxProjector` (recall key; not a V3 selected input)."
        ),
        relationships=(
            "ARCHITECTURE.md: the cache crate owns graph mutation and algorithms; "
            "MCP projection stays in `anvil-gctx-egress` and `anvil-cli` MCP. "
            "`anvil-gctx-types` is graph-free sealed DTOs (ADR-084 split)."
        ),
        invariants=(
            "`anvil-gctx-types` must not depend on `anvil-graph-cache`. Wire and MCP "
            "consumers link the types crate so they cannot name graph internals."
        ),
        blast="Naming the wrong crate for a new GCTX tool leaks internals or source text (see T08).",
        tests=(
            "- CE-5 structural no-leak tests in `crates/anvil-gctx-types/src/lib.rs`.\n"
            "- Projector type is in egress; DTOs are in types."
        ),
        adrs="ADR-084 (graph-handle access / crate split). ADR-083 (MCP delivery).",
        risks="Gold path `crates/anvil-gctx-egress/src/lib.rs` is not on the §12.3 allowlist; V3 must not open it. The brief states the ownership fact.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "The resident semantic graph lives in `crates/anvil-graph-cache` (`SymbolGraph` plus derived `DependencyGraph`).",
                ["`crates/anvil-graph-cache/ARCHITECTURE.md` Scope and boundaries"],
            ),
            claim(
                2,
                "Deterministic fact",
                "`crates/anvil-gctx-types` owns sealed egress DTOs and the CE-5 no-leak tests; it depends on `anvil-kernel-types` and serde only — never on `anvil-graph-cache`.",
                ["`crates/anvil-gctx-types/src/lib.rs` crate docs"],
            ),
            claim(
                3,
                "Deterministic fact",
                "`GctxProjector` is defined in `crates/anvil-gctx-egress/src/lib.rs`. MCP projection does not live in the cache crate.",
                [
                    "`crates/anvil-graph-cache/ARCHITECTURE.md` (MCP projection stays in anvil-gctx-egress)",
                    "recall: `crates/anvil-gctx-egress/src/lib.rs` (`pub struct GctxProjector`)",
                ],
            ),
        ],
        completeness="partial",
        completeness_note="Egress crate path is a gold recall key outside §12.3; V3 must not open it. Ownership is stated from ARCHITECTURE.md plus types crate docs.",
        v3_handles=[
            "crates/anvil-graph-cache/ARCHITECTURE.md",
            "crates/anvil-gctx-types/src/lib.rs",
            "plans/decisions/084-gctx-graph-handle-access.md",
        ],
        v4_gctx=["graph://stats", "anvil_search_symbols (identity-only locator: GctxProjector)"],
        recall_only=["crates/anvil-gctx-egress/src/lib.rs"],
    )
)

add(
    dict(
        tid="T04",
        cls="localised bug",
        prompt="`ImpactSummary.truncated` is a single bool. What can a consumer not tell from it, and why does GATT exist?",
        scope=(
            "Covers the counts-only `truncated` flag versus GATT's reason for in-band "
            "limits. Refuses implementing GATT, forking attestation, or deciding spec §15.6."
        ),
        components=(
            "- `ImpactSummary` in `crates/anvil-gctx-types/src/lib.rs`.\n"
            "- ADR-142 — why a single bool is not enough.\n"
            "- GATT module (recall) — affinity only."
        ),
        relationships=(
            "`truncated: bool` means a result cap bound the report (affected-symbol "
            "set or dependent-closure walk). It does not say which bound, how much was "
            "dropped, or per-edge fidelity. GATT exists so graph answers state those limits in-band."
        ),
        invariants="Do not treat `truncated: false` as 'the graph is complete'. Do not fork GATT from this spike.",
        blast="Consumers that treat a single bool as a full disclosure will misread caps as completeness (spec §14 false completeness).",
        tests="- Read `ImpactSummary` field docs in `crates/anvil-gctx-types/src/lib.rs`. GATT behaviour is not executed here.",
        adrs="ADR-142 (Accepted). ADR-135 is CEG affinity; do not consume CEG.",
        risks="§15.6 (GATT versus distinct brief evidence) remains parked. T19 owns that novel question.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "`ImpactSummary.truncated` is a single bool meaning a result cap bound the report — either the affected-symbol set or the dependent-closure walk hit its node budget.",
                ["`crates/anvil-gctx-types/src/lib.rs` `struct ImpactSummary`"],
            ),
            claim(
                2,
                "Synthesised interpretation",
                "From that bool alone a consumer cannot tell which cap fired, what was omitted, or per-edge call-resolution fidelity — the counts-only summary is CE-5 safe and deliberately coarse.",
                ["`crates/anvil-gctx-types/src/lib.rs`", "`plans/decisions/142-graph-answer-attestation.md`"],
                depends="1",
            ),
            claim(
                3,
                "Deterministic fact",
                "GATT / ADR-142 exists because `ImpactSummary` and `AffectedTestsSummary` count only what was returned and carry a single `truncated: bool`; graph answers need in-band limits (caps, per-edge fidelity, cost self-report).",
                ["`plans/decisions/142-graph-answer-attestation.md`"],
                depends="1",
            ),
            claim(
                4,
                "Uncertainty",
                "Whether Decision Brief evidence should reuse GATT, extend it, or stay distinct is spec §15.6 and remains parked (see T19). This brief does not fork GATT.",
                ["spec §15.6", "`plans/decisions/142-graph-answer-attestation.md`"],
            ),
        ],
        completeness="partial",
        completeness_note="GATT APS module path is a gold recall key outside §12.3; ADR-142 on the allowlist is the selected input.",
        v3_handles=[
            "crates/anvil-gctx-types/src/lib.rs",
            "plans/decisions/142-graph-answer-attestation.md",
        ],
        v4_gctx=["anvil_impact_of_change (identity-only; inspect summary.truncated — not permission)"],
        recall_only=["plans/modules/graph-answer-attestation.aps.md"],
    )
)

add(
    dict(
        tid="T05",
        cls="localised bug",
        prompt=(
            "In the call graph, is a caller marked `heuristic` per edge or OR-ed across "
            "all of that caller's edges? What must a consumer not treat as an exact call?"
        ),
        scope="Covers CALL-1 heuristic OR-across-edges. Refuses treating heuristic callers as exact calls or changing GCALL substrate.",
        components="- `crates/anvil-graph-cache/src/call_graph.rs` — `callers_of` walk.\n- ADR-086 — symbol call graph.",
        relationships=(
            "GCALL produces `EdgeType::Calls` in the resident graph. GCTX-014 projects "
            "`anvil_find_callers` over `callers_of`. Heuristic is a producer-side flag."
        ),
        invariants=(
            "A caller reachable via several frontier nodes in one hop is recorded once, "
            "with `heuristic` OR-ed across all its edges. A consumer must not treat a "
            "`heuristic` caller as an exact call (GCALL-007 CALL-1)."
        ),
        blast="Wrong exact-call treatment on fan-out overloads. GCTX projection must keep the flag visible.",
        tests="- `fan_out_caller_is_marked_heuristic` in `call_graph.rs`.",
        adrs="ADR-086. GATT affinity for disclosure; not a GATT implementation task.",
        risks="Per-edge fidelity beyond this bool is GATT territory (parked for CCTX).",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "`heuristic` is OR-ed across all of a caller's edges in a hop: a caller is heuristic if any of its calls into the frontier is an overload fan-out.",
                ["`crates/anvil-graph-cache/src/call_graph.rs` (`heuristic` OR-across-edges comment and `entry.1 |= heuristic`)"],
            ),
            claim(
                2,
                "Deterministic fact",
                "A consumer must not treat a `heuristic` caller as an exact call (GCALL-007 CALL-1).",
                ["`crates/anvil-graph-cache/src/call_graph.rs` field docs on `heuristic`"],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "When reading callers, keep the heuristic flag visible; do not collapse it into an exact call graph for enforcement.",
                ["`crates/anvil-graph-cache/src/call_graph.rs`", "this brief Authority section"],
                depends="2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Gold path is on the §12.3 test allowlist.",
        v3_handles=[
            "crates/anvil-graph-cache/src/call_graph.rs",
            "plans/decisions/086-symbol-call-graph-substrate.md",
        ],
        v4_gctx=["anvil_find_callers (identity-only locator: call_graph.rs / heuristic)"],
        recall_only=[],
    )
)

add(
    dict(
        tid="T06",
        cls="localised bug",
        prompt="You ran markdownlint on a `plans/` APS module and it exited 0. Is that a pass?",
        scope=(
            "Covers CIB-390 shape A: a check that examined nothing must not read as a pass. "
            "Refuses treating markdownlint exit 0 on ignored `plans/**` as lint-clean."
        ),
        components=(
            "- `.markdownlintignore` excludes `plans/**` (recall; not a V3 selected input).\n"
            "- CIB-390 in the continuous-improvement backlog (recall).\n"
            "- `.github/workflows/README.md` `Reading PR readiness (CIB-390)` (recall)."
        ),
        relationships=(
            "markdownlint with `--ignore-path .markdownlintignore` on an excluded path "
            "receives an empty file list, prints a usage banner, and exits 0. Nothing was examined."
        ),
        invariants="A check that has not reached a verdict must not read as a pass (CIB-390).",
        blast="False confidence on APS module markdown. PR-readiness readers have the same class of defect for unreported checks.",
        tests="Do not treat this brief as proving markdownlint ran. Ask a deterministic reader for PR checks (`scripts/ci/pr-required-status.mjs`).",
        adrs="none in selected inputs. CIB-390 is backlog, not an ADR.",
        risks=(
            "Selected §12.3 inputs do not include `.markdownlintignore` or the CIB module. "
            "This brief carries the corpus fact so V3 need not open those paths. Completeness is partial."
        ),
        claims=[
            claim(
                1,
                "Deterministic fact",
                "`.markdownlintignore` excludes `plans/**`. markdownlint naming only `plans/` files lints nothing and exits 0.",
                [
                    "recall: `.markdownlintignore`",
                    "recall: `plans/modules/continuous-improvement-backlog.aps.md` CIB-390",
                ],
            ),
            claim(
                2,
                "Synthesised interpretation",
                "Exit 0 on a `plans/` APS module is not a lint pass; it is CIB-390 shape A (examined nothing).",
                ["recall: CIB-390", "recall: `.github/workflows/README.md` Reading PR readiness"],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Do not report markdownlint clean for `plans/**` from that exit 0. For PR readiness, use the deterministic required-status reader, not a missing-check-as-pass reading.",
                ["recall: `.github/workflows/README.md`"],
                depends="2",
            ),
        ],
        completeness="partial",
        completeness_note="All gold paths sit outside §12.3 selected inputs. Facts are stated in-brief as recall keys; V3 must not open them.",
        v3_handles=[
            "docs/guides/ai-context-delivery.md",
        ],
        v4_gctx=["anvil_search_symbols (identity-only locator: markdownlint)"],
        recall_only=[
            "plans/modules/continuous-improvement-backlog.aps.md",
            ".markdownlintignore",
            ".github/workflows/README.md",
        ],
        invalidation="partial",
    )
)

add(
    dict(
        tid="T07",
        cls="localised bug",
        prompt="`anvil_validate_write` on `crates/anvil-cli/src/commands/start.rs` reports AWS-key interrupts. Are those live secrets?",
        scope=(
            "Covers CIB-392: fixture / recipe / pin shapes, not live secrets. Refuses "
            "weakening secret-detection generally or treating the interrupt as an allow."
        ),
        components=(
            "- `crates/anvil-cli/src/commands/start.rs` — built-in sample fixture `AKIA…` (recall).\n"
            "- `anvil_validate_write` — still the gate; findings may be pre-existing fixtures."
        ),
        relationships=(
            "First-run recipe runs the ADTRUST-006 secret fixture through real "
            "secret-detection so `anvil start` can prove the check pipeline catches the shape."
        ),
        invariants="An interrupt is not proof of a live credential. It is also not an allow. Keep the gate; interpret the finding.",
        blast="Agents that 'fix' the fixture by deleting it break first-run proof. Agents that ignore all AWS-key findings weaken the gate.",
        tests="`anvil_validate_write` remains required for edits to `start.rs`. This brief does not clear findings.",
        adrs="none in selected inputs. CIB-392 is backlog.",
        risks="Gold path `start.rs` is not a §12.3 selected input. Completeness partial. Do not paste live secrets into briefs.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "`start.rs` embeds a built-in sample fixture `const KEY = \"AKIAQRSTUVWXYZ123456\";` so the first-run recipe can prove secret-detection catches the AWS-key shape. That is not a live credential.",
                [
                    "recall: `crates/anvil-cli/src/commands/start.rs` (CIB-392 / ADTRUST-006 fixture)",
                    "recall: CIB-392",
                ],
            ),
            claim(
                2,
                "Synthesised interpretation",
                "Whole-file `anvil_validate_write` interrupts on that fixture (and recipe/test pins) are pre-existing detection hits, not evidence the editor introduced a secret.",
                ["recall: CIB-392"],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Keep `anvil_validate_write`. Do not skip the gate. Do not treat fixture hits as live secrets, and do not delete the fixture to silence the gate.",
                ["this brief Authority section", "`docs/guides/ai-context-delivery.md`"],
                depends="1, 2",
            ),
        ],
        completeness="partial",
        completeness_note="`start.rs` and CIB-392 are recall keys outside §12.3. The brief states the corpus fact; V3 opens only allowlisted enforcement-split docs.",
        v3_handles=[
            "docs/guides/ai-context-delivery.md",
            "crates/anvil-cli/tests/mcp_serve_stdio.rs",
        ],
        v4_gctx=["anvil_search_symbols (identity-only locator: start.rs / AKIA fixture)"],
        recall_only=["crates/anvil-cli/src/commands/start.rs", "CIB-392"],
        invalidation="partial",
    )
)


add(
    dict(
        tid="T08",
        cls="cross-module change",
        prompt="Which crates and contracts must change to add a new identity-only GCTX MCP tool without leaking source text?",
        scope="Covers the GCTX-010..014 spine for a new identity-only tool. Refuses snippet default-on and any DTO that carries source text without CE-1.",
        components=(
            "- `crates/anvil-gctx-types` — sealed DTO + CE-5 tests.\n"
            "- `crates/anvil-gctx-egress` — projector (recall).\n"
            "- `crates/anvil-graph-cache` — resident graph reads.\n"
            "- `crates/anvil-cli` MCP dispatch."
        ),
        relationships=(
            "New tool: identity DTO in types (CE-5 serialised-shape tests), projection in egress, "
            "graph read in cache or existing read API, MCP dispatch in CLI. Types crate must stay graph-free."
        ),
        invariants="CE-1 identity-only default. CE-5 no-leak hard gate. No source text, byte spans, or session-local ids on the wire unless CE-1 opt-in (not this task).",
        blast="`anvil-gctx-types`, `anvil-gctx-egress`, `anvil-graph-cache`, `anvil-cli` MCP dispatch, CE-5 tests. Missed blast-radius if CLI dispatch or CE-5 tests are omitted.",
        tests="CE-5 structural no-leak tests in `crates/anvil-gctx-types/src/lib.rs` including GCTX-011/012/013/014. `anvil_validate_write` still required for the edit.",
        adrs="ADR-083, ADR-084. ADR-086 if the tool is callers.",
        risks="Gold egress/CLI paths are partly recall-only. Do not leak source to 'make the tool more useful'.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "Sealed identity-only DTOs and CE-5 no-leak tests live in `crates/anvil-gctx-types/src/lib.rs`; that crate must not depend on `anvil-graph-cache`.",
                ["`crates/anvil-gctx-types/src/lib.rs` crate docs"],
            ),
            claim(
                2,
                "Deterministic fact",
                "Resident graph mutation/algorithms live in `crates/anvil-graph-cache`; MCP projection stays in `anvil-gctx-egress` and `crates/anvil-cli` MCP.",
                ["`crates/anvil-graph-cache/ARCHITECTURE.md`"],
            ),
            claim(
                3,
                "Synthesised interpretation",
                "A new identity-only GCTX MCP tool must change types (DTO + CE-5 test), egress projector, any new graph read, and CLI MCP dispatch — omitting CE-5 tests or putting source text on the DTO is a protocol failure for this task.",
                [
                    "`crates/anvil-gctx-types/src/lib.rs`",
                    "`crates/anvil-graph-cache/ARCHITECTURE.md`",
                    "`docs/architecture/graph-context-delivery-spec.md` CE-1 / CE-5",
                ],
                depends="1, 2",
            ),
            claim(
                4,
                "Recommendation",
                "Add the tool identity-only. Keep `anvil_validate_write` on the write path. Do not treat this brief as permission to ship snippets.",
                ["`docs/architecture/graph-context-delivery-spec.md`", "this brief Authority"],
                depends="3",
            ),
        ],
        completeness="partial",
        completeness_note="Egress crate and CLI dispatch are gold recall keys; ARCHITECTURE.md and types crate are selected inputs.",
        v3_handles=[
            "crates/anvil-gctx-types/src/lib.rs",
            "crates/anvil-graph-cache/ARCHITECTURE.md",
            "docs/architecture/graph-context-delivery-spec.md",
            "plans/decisions/084-gctx-graph-handle-access.md",
            "plans/decisions/083-gctx-mcp-delivery-target.md",
        ],
        v4_gctx=[
            "anvil_search_symbols",
            "graph://symbols",
            "anvil_symbol_context (identity-only)",
        ],
        recall_only=["crates/anvil-gctx-egress/src/lib.rs", "crates/anvil-cli MCP dispatch"],
    )
)

add(
    dict(
        tid="T09",
        cls="cross-module change",
        prompt="Trace `anvil_find_callers` from producer substrate to MCP projection. What is GCTX's job versus GCALL's?",
        scope="Covers GCALL producer versus GCTX-014 consumer. Refuses re-implementing call extraction in CCTX.",
        components=(
            "- GCALL (archived module, recall) — extract calls, lift `EdgeType::Calls`, `callers_of`.\n"
            "- `crates/anvil-graph-cache/src/call_graph.rs` — resident walk.\n"
            "- GCTX-014 — MCP projection of that walk."
        ),
        relationships="GCALL owns producer substrate. GCTX owns sealed egress + MCP tool. CCTX must not become a third call graph.",
        invariants="CALL-1 heuristic flag must remain visible through projection. Identity-only default.",
        blast="Cache call_graph, GCTX types callers DTO, CLI MCP. Archived GCALL/GCTX modules are provenance, not live edit targets unless the task says so.",
        tests="Caller CE-5 tests in `anvil-gctx-types`. `fan_out_caller_is_marked_heuristic` in `call_graph.rs`.",
        adrs="ADR-086. ADR-084.",
        risks="Archived module paths are recall keys outside §12.3.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "GCALL is producer-side call-graph substrate: extraction into `FileSymbols`, `EdgeType::Calls` in the resident graph, bounded `callers_of` read API.",
                [
                    "`plans/decisions/086-symbol-call-graph-substrate.md`",
                    "`crates/anvil-graph-cache/src/call_graph.rs`",
                ],
            ),
            claim(
                2,
                "Deterministic fact",
                "GCTX's job is projection: `anvil_find_callers` is an identity-only MCP tool over that read API, not a second extractor.",
                [
                    "`crates/anvil-graph-cache/ARCHITECTURE.md`",
                    "`docs/architecture/graph-context-delivery-spec.md`",
                ],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Trace callers from `call_graph.rs` through GCTX types/MCP. Do not re-derive calls by searching the repo. Keep `anvil_validate_write` if editing.",
                ["`crates/anvil-graph-cache/src/call_graph.rs`", "this brief Authority"],
                depends="1, 2",
            ),
        ],
        completeness="partial",
        completeness_note="Archived GCALL/GCTX module files are gold recall keys, not V3 inputs.",
        v3_handles=[
            "crates/anvil-graph-cache/src/call_graph.rs",
            "crates/anvil-graph-cache/ARCHITECTURE.md",
            "plans/decisions/086-symbol-call-graph-substrate.md",
            "crates/anvil-gctx-types/src/lib.rs",
        ],
        v4_gctx=["anvil_find_callers", "graph://edges"],
        recall_only=[
            "plans/archive/modules/symbol-call-graph.aps.md",
            "plans/archive/modules/graph-context-delivery.aps.md",
        ],
    )
)

add(
    dict(
        tid="T10",
        cls="cross-module change",
        prompt="After a merge, how does shared base-graph persistence differ from the old per-worktree snapshot?",
        scope="Covers ADR-105 storage-layout successor to ADR-069. Refuses implementing a knowledge store or event-sourced replay.",
        components="- ADR-105.\n- `crates/anvil-graph-cache/ARCHITECTURE.md`.",
        relationships=(
            "Old layout: one sealed snapshot per `WorktreeKey`. New layout: one write-once "
            "content-addressed base per repo per merge-base commit plus live per-worktree overlays."
        ),
        invariants="ADR-105 changes storage layout only; it is not the event-sourced persist-and-trust design ADR-069 rejected. Format/privacy lines inherited.",
        blast="Persistence route, orphan GC over shared bases, not GCTX tool DTOs.",
        tests="No CCTX test. GBASE graduation is historical. `anvil_validate_write` if editing persistence.",
        adrs="ADR-105 (Accepted). ADR-069 amended in the named sections only.",
        risks="Do not treat overlay freshness as CCTX card invalidation (§15.8 parked).",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "ADR-069 persisted a per-`WorktreeKey` snapshot (one file per worktree). ADR-105 replaces that on-disk layout with one shared write-once base per repo per merge-base commit plus live per-worktree overlays.",
                ["`plans/decisions/105-shared-base-graph-persistence.md`"],
            ),
            claim(
                2,
                "Deterministic fact",
                "The cache crate remains the resident-graph owner; persistence is layout, not a second graph product.",
                ["`crates/anvil-graph-cache/ARCHITECTURE.md`", "`plans/decisions/105-shared-base-graph-persistence.md`"],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Describe the post-merge difference as shared base + overlays versus private per-worktree snapshots. Do not invent an event-sourced CCTX store from this ADR.",
                ["`plans/decisions/105-shared-base-graph-persistence.md`", "spec §15.8 parked"],
                depends="1",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Both gold paths are selected inputs.",
        v3_handles=[
            "plans/decisions/105-shared-base-graph-persistence.md",
            "crates/anvil-graph-cache/ARCHITECTURE.md",
        ],
        v4_gctx=["graph://stats"],
        recall_only=[],
    )
)

add(
    dict(
        tid="T11",
        cls="cross-module change",
        prompt="May a GCTX tool accept a nested directory as `workspaceRoot`? What identity/privacy invariant does that protect?",
        scope="Covers CIB-398 / ADR-125 amendment: graph-root must be exact workspace or registered worktree root, not a nested directory.",
        components="- ADR-125 (amended 2026-09-03).\n- `docs/guides/ai-context-delivery.md` graph-root rule.\n- GCTX delivery spec CE-8 / CIB-398 note.",
        relationships=(
            "MCP may admit linked worktrees of the same repo as `workspaceRoot`. Nested "
            "directories inside a root are refused for the six daemon-keyed GCTX tools."
        ),
        invariants=(
            "The daemon keys its graph on the root it is handed and projects root-relative "
            "identities. A nested root rebases `secrets/token.ts` to `token.ts` past the CE-3 deny-list."
        ),
        blast="All six graph tools. Wrong `workspaceRoot` is a privacy bug, not a convenience feature.",
        tests="Guide freshness cites the CIB-398 graph-root rule. Keep `anvil_validate_write` on edits.",
        adrs="ADR-125 (corpus status Proposed, with CIB-398 amendment in the Decision section).",
        risks="ADR-125 status at corpus is Proposed; the amendment text is still the binding graph-root rule in the guide/spec.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "The six daemon-keyed GCTX tools require `workspaceRoot` to be exactly the server cwd or a registered worktree root — not a directory inside either.",
                [
                    "`plans/decisions/125-mcp-admits-linked-worktrees.md` Amended (CIB-398)",
                    "`docs/guides/ai-context-delivery.md`",
                    "`docs/architecture/graph-context-delivery-spec.md`",
                ],
            ),
            claim(
                2,
                "Deterministic fact",
                "That protects CE-3 sensitive-path identity: a nested root would rebase `secrets/token.ts` to `token.ts` past the deny-list.",
                ["`plans/decisions/125-mcp-admits-linked-worktrees.md`"],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Pass the workspace or registered worktree root. Do not 'fix' a tool failure by pointing `workspaceRoot` at a nested folder.",
                ["`docs/guides/ai-context-delivery.md`"],
                depends="1, 2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Gold paths are selected inputs (guide + ADR-125).",
        v3_handles=[
            "plans/decisions/125-mcp-admits-linked-worktrees.md",
            "docs/guides/ai-context-delivery.md",
            "docs/architecture/graph-context-delivery-spec.md",
        ],
        v4_gctx=["anvil_search_symbols (must use graph root, not a nested directory)"],
        recall_only=[],
    )
)

add(
    dict(
        tid="T12",
        cls="policy-sensitive change",
        prompt="What would it mean to flip GCTX's identity-only default to snippets-on? Which consent and CE-1 rules apply?",
        scope="Policy-sensitive thought experiment only. Refuses actually flipping the default, shipping a flag, or treating this brief as permission.",
        components="- GCTX spec CE-1 / CE-12.\n- Guide consent CLI.\n- This spec §11 trust boundary.",
        relationships="CE-1 identity-only default is a hard Ready gate. Consent (CE-12) is explicit operator action. Kill-switch is distinct from default.",
        invariants="Snippets-on as default would invert CE-1. Eval spike default remains identity-only. Synthesis is never allow / warn / block.",
        blast="Every GCTX tool and resource, consent records, tests `egress_resolve_default_is_identity_only`, privacy review CE-1..CE-12.",
        tests="`egress_resolve_default_is_identity_only` must keep proving the default. This brief is not a change proposal ADR.",
        adrs="none that authorises flipping CE-1. ADR-083/084 affinity.",
        risks="A fluent 'yes, default snippets-on' would be a protocol failure. No ADR is invented here.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "CE-1 default posture is identity-only; snippet text is behind explicit CE-12 consent (`anvil gctx egress enable`), not first-use auto-enable.",
                [
                    "`docs/architecture/graph-context-delivery-spec.md` Default posture: identity-only (CE-1)",
                    "`docs/guides/ai-context-delivery.md`",
                    "`crates/anvil-gctx-types/src/lib.rs` `egress_resolve_default_is_identity_only`",
                ],
            ),
            claim(
                2,
                "Synthesised interpretation",
                "Flipping the default to snippets-on would be a CE-1 / consent-model change affecting every tool and resource, not a local UX tweak.",
                ["`docs/architecture/graph-context-delivery-spec.md`"],
                depends="1",
            ),
            claim(
                3,
                "Uncertainty",
                "Whether a product compiler or future GCTX default should change is not decided by this spike. No ADR is filed here.",
                ["spec §15 parked", "this spec §11"],
            ),
            claim(
                4,
                "Recommendation",
                "Do not flip the default in this spike. Keep identity-only unless a run record has consent. Keep `anvil_validate_write`. This brief is not allow / warn / block.",
                ["this brief Authority", "`docs/architecture/graph-context-delivery-spec.md`"],
                depends="1, 3",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Consent CLI is documented on the allowlisted guide. Spec §11 is recall for trust boundary; guide+GCTX spec carry the same split.",
        v3_handles=[
            "docs/architecture/graph-context-delivery-spec.md",
            "docs/guides/ai-context-delivery.md",
            "crates/anvil-gctx-types/src/lib.rs",
        ],
        v4_gctx=["anvil_symbol_context (identity-only; do not assume snippets)"],
        recall_only=["GCTX-024 consent record", "this spec §11"],
    )
)

add(
    dict(
        tid="T13",
        cls="policy-sensitive change",
        prompt="An agent has a fluent Decision Brief that says an edit is safe. May it skip `anvil_validate_write`?",
        scope="Authority-leakage hard case. Honest result: no. Keep enforcement. The brief is never allow / warn / block.",
        components="- Guide heading `Graph context is not launch validation`.\n- Spec §11 trust boundary (recall).\n- `anvil_validate_write`.",
        relationships="Same split as T01, applied to a fluent brief. Context Compiler may influence what an agent considers; it may not determine whether a write is permitted.",
        invariants="Enforcement must work if Context Compiler is unavailable, stale, or wrong. Skipping the gate because a brief says 'safe' is authority leakage and fails the §13 leakage check.",
        blast="Any edit path. First correct action: keep `anvil_validate_write`.",
        tests="Correct first action is keep enforcement. MCP serve tests still list `anvil_validate_write`.",
        adrs="none required. GCTX split is in the guide.",
        risks="A recommendation that uses allow/warn/block as if the brief issued it is a fixture defect.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "Launch validation (`anvil_validate_write`) is the enforcement gate; graph context and briefs return context, never a decision, and never block.",
                [
                    "`docs/guides/ai-context-delivery.md#graph-context-is-not-launch-validation`",
                    "`crates/anvil-cli/tests/mcp_serve_stdio.rs`",
                ],
            ),
            claim(
                2,
                "Deterministic fact",
                "The Context Compiler may not determine whether a proposed action is permitted. Enforcement must work if it is unavailable.",
                [
                    "`docs/guides/ai-context-delivery.md`",
                    "recall: this spec §11",
                ],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "No. Do not skip `anvil_validate_write` because a fluent Decision Brief says an edit is safe. Keep the gate. This brief is not allow / warn / block.",
                ["this brief Authority", "`docs/guides/ai-context-delivery.md`"],
                depends="1, 2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Spec §11 is a gold recall key; the allowlisted guide states the same split.",
        v3_handles=[
            "docs/guides/ai-context-delivery.md",
            "crates/anvil-cli/tests/mcp_serve_stdio.rs",
        ],
        v4_gctx=["anvil_impact_of_change (context only — not a write permit)"],
        recall_only=["this spec §11"],
    )
)

add(
    dict(
        tid="T14",
        cls="policy-sensitive change",
        prompt="Is the CCTX spike the same eval as `anvil policy eval-regression` in CI?",
        scope="Adjacent-work distinction. Honest result: no. EVALCI is policy-baseline CI; CCTX is an internal task-brief comparison.",
        components="- EVALCI module.\n- Spec §6 adjacent-work table (recall).\n- This spike protocol §12.",
        relationships="EVALCI wires `anvil policy eval-regression` into CI. CCTX scores agent task outcomes against a frozen corpus. Do not conflate.",
        invariants="Different eval. CCTX must not wait on EVALCI and must not change policy-regression CI.",
        blast="None for product code. Wrong merge of the two evals would pull EVALCI into CCTX scope (explicit non-scope).",
        tests="None shared. Do not run policy eval-regression as this spike.",
        adrs="none in selected inputs for this distinction.",
        risks="EVALCI module is a selected policy input. Spec §6 is recall.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "EVALCI owns the policy eval-regression CI gate: `anvil policy eval-regression` against a committed baseline.",
                ["`plans/modules/eval-regression-ci-gate.aps.md`"],
            ),
            claim(
                2,
                "Deterministic fact",
                "CCTX's first spike is an internal baselines-versus-brief-variants comparison, not a policy-baseline CI gate.",
                [
                    "`plans/modules/eval-regression-ci-gate.aps.md`",
                    "recall: this spec §6 adjacent-work table",
                ],
            ),
            claim(
                3,
                "Recommendation",
                "Answer no. Keep EVALCI and CCTX separate. Do not treat this brief as changing CI or policy.",
                ["`plans/modules/eval-regression-ci-gate.aps.md`", "this brief Authority"],
                depends="1, 2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="EVALCI module is a §12.3 selected input.",
        v3_handles=["plans/modules/eval-regression-ci-gate.aps.md", "docs/guides/ai-context-delivery.md"],
        v4_gctx=["anvil_search_symbols (identity-only locator: eval-regression)"],
        recall_only=["this spec §6 adjacent-work table"],
    )
)

add(
    dict(
        tid="T15",
        cls="policy-sensitive change",
        prompt="For a product compiler, may synthesis models send private source off-box? Decide the architecture.",
        scope=(
            "Honest result is **uncertainty**. Spec §15.7 is parked. Do not invent an ADR. "
            "Local-first security doc is evidence to read, not a compiler architecture."
        ),
        components="- `docs/public/anvil/operations/security.md`.\n- Spec §15.7 (recall).",
        relationships="anvil is local-first: source analysis and normal findings are produced on the machine. Off-box synthesis for a product compiler is an open owner call.",
        invariants="Parking is not a decision. A decided-sounding architecture with no ADR is a protocol failure for this run (spec §12.7 step 6).",
        blast="Any off-box synthesis would sit against the local-first data boundary. This spike does not build it.",
        tests="Success **is** visible uncertainty. Do not ship a compiler. Keep `anvil_validate_write` if editing.",
        adrs="none. Do not invent one.",
        risks="Fluent architecture prose is the failure mode. GATT/CEG/EVALCI are unrelated.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "anvil is local-first: source analysis and normal findings are produced on the operator's machine. The public security page is the existing data-boundary description, not a CCTX compiler ADR.",
                ["`docs/public/anvil/operations/security.md`"],
            ),
            claim(
                2,
                "Uncertainty",
                "Spec §15.7 (synthesis models under local-first constraints — what may run on-box, what may not leave the machine) remains parked. This spike does not decide whether a product compiler may send private source off-box, and it does not invent an ADR.",
                [
                    "recall: this spec §15.7",
                    "`docs/public/anvil/operations/security.md`",
                ],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Report uncertainty. Do not design the off-box architecture in this run. Do not treat this brief as allow / warn / block.",
                ["this brief Authority", "`docs/public/anvil/operations/security.md`"],
                depends="2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Honest result is parked §15.7. Security page is a selected input.",
        v3_handles=["docs/public/anvil/operations/security.md", "docs/guides/ai-context-delivery.md"],
        v4_gctx=["graph://stats (identity-only; no source text)"],
        recall_only=["this spec §15.7"],
    )
)

add(
    dict(
        tid="T16",
        cls="test-impact",
        prompt="If `GctxProjector` changes, which tests are the CE-5 no-leak hard gate and which bench must not be treated as this spike?",
        scope="Test-impact on the GCTX-010 spine. Distinguishes CE-5 tests from GCTX-031 `token_reduction` bench. This spike does not replace that bench.",
        components=(
            "- `crates/anvil-gctx-types/src/lib.rs` CE-5 tests.\n"
            "- `crates/anvil-gctx-egress/src/lib.rs` projector (recall).\n"
            "- `crates/anvil-bench/src/scenarios/token_reduction.rs`."
        ),
        relationships="CE-5 is the structural no-leak hard gate on DTO shape. `token_reduction` is a shape regression on `ImpactOutcome` payloads versus file-reading. CCTX spike is a task-outcome comparison beside it.",
        invariants="Do not treat `token_reduction` numbers as this spike's success. Do not skip CE-5.",
        blast="Types crate tests, egress projector, token_reduction bench. Missed blast-radius if CE-5 is omitted.",
        tests="CE-5 tests in types. Bench in anvil-bench. `anvil_validate_write` on edits.",
        adrs="ADR-084. GCTX-031 provenance.",
        risks="Projector path is recall-only for V3.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "CE-5 structural no-leak tests live in `crates/anvil-gctx-types/src/lib.rs` and are the hard gate that GCTX-010 and downstream tools must keep.",
                ["`crates/anvil-gctx-types/src/lib.rs` crate docs and CE-5 tests"],
            ),
            claim(
                2,
                "Deterministic fact",
                "`crates/anvil-bench/src/scenarios/token_reduction.rs` is the GCTX-031 shape bench on `ImpactOutcome` payloads. Spec §12 says this spike sits beside it and does not replace that harness.",
                ["`crates/anvil-bench/src/scenarios/token_reduction.rs`"],
            ),
            claim(
                3,
                "Recommendation",
                "If `GctxProjector` changes, run CE-5 no-leak tests as the hard gate. Do not treat `token_reduction` as the CCTX spike, and do not skip `anvil_validate_write`.",
                [
                    "`crates/anvil-gctx-types/src/lib.rs`",
                    "`crates/anvil-bench/src/scenarios/token_reduction.rs`",
                    "this brief Authority",
                ],
                depends="1, 2",
            ),
        ],
        completeness="partial",
        completeness_note="Egress projector path is a gold recall key outside §12.3.",
        v3_handles=[
            "crates/anvil-gctx-types/src/lib.rs",
            "crates/anvil-bench/src/scenarios/token_reduction.rs",
            "crates/anvil-graph-cache/ARCHITECTURE.md",
        ],
        v4_gctx=["anvil_affected_tests (identity-only; not a substitute for CE-5)"],
        recall_only=["crates/anvil-gctx-egress/src/lib.rs"],
    )
)

add(
    dict(
        tid="T17",
        cls="test-impact",
        prompt="What does `anvil_affected_tests` return, and which tests pin that it is identity-only?",
        scope="GCTX-013 identity-only affected-tests report. Refuses treating the report as execution-verified coverage.",
        components="- `AffectedTestsSummary` / `AffectedTestsReport` in `anvil-gctx-types`.\n- `affected_tests_*` tests in the same file.",
        relationships="Import-derived, not execution-verified (`heuristic`). Counts-only summary is CE-5 safe. Paths are workspace-relative identities, no source text.",
        invariants="Identity-only. `truncated` has the same GATT-shaped coarseness as impact (see T04). Not the CCTX spike and not `token_reduction`.",
        blast="Types crate tests named `affected_tests_*`. Projector/CLI if the tool changes (recall).",
        tests="`affected_tests_report_serialised_keys_are_identity_only`, `affected_tests_report_carries_no_absolute_paths_or_forbidden_concepts`, query/outcome round-trip tests.",
        adrs="ADR-084. ADR-142 affinity for truncated bool.",
        risks="Do not name affected tests from this brief as if GCTX had been called; V4 may call the tool.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "`anvil_affected_tests` returns an identity-only test-attribution report: workspace-relative file paths, no source text; relevance is import-derived, not execution-verified (`heuristic`).",
                ["`crates/anvil-gctx-types/src/lib.rs` `AffectedTestsReport` / `AffectedTestsSummary`"],
            ),
            claim(
                2,
                "Deterministic fact",
                "Identity-only pinning tests include `affected_tests_report_serialised_keys_are_identity_only` and `affected_tests_report_carries_no_absolute_paths_or_forbidden_concepts`.",
                ["`crates/anvil-gctx-types/src/lib.rs`"],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Use the types tests as the identity-only pin. Call `anvil_affected_tests` only on V2/V4 when GCTX is available. This brief is not a test list for a live edit.",
                ["`crates/anvil-gctx-types/src/lib.rs`", "spec §12.2"],
                depends="1, 2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Gold path is the types crate on the allowlist.",
        v3_handles=["crates/anvil-gctx-types/src/lib.rs"],
        v4_gctx=["anvil_affected_tests"],
        recall_only=[],
    )
)

add(
    dict(
        tid="T18",
        cls="test-impact",
        prompt="You need to know which tests to run after editing `call_graph.rs`. Use GCTX, not this spike, to name affected tests.",
        scope="Contrast: live affected-tests is GCTX-013; this spike is not that harness and not `token_reduction`.",
        components="- `anvil_affected_tests`.\n- `crates/anvil-graph-cache/src/call_graph.rs`.\n- Spec §12 (spike is not the bench).",
        relationships="V2/V4 may call `anvil_affected_tests` when GCTX is available. V3 must not pretend the brief enumerated live tests. If GCTX is unavailable, V2/V4 are blocked, not silent V1.",
        invariants="Do not replace GCTX-031 `token_reduction`. Do not invent a test list from synthesis.",
        blast="`call_graph.rs`, callers tests, GCTX-013 types tests, possibly GCALL. Live list is a graph query, not this fixture.",
        tests="This fixture's success is to send the agent to GCTX (or report blocked), not to name tests.",
        adrs="ADR-086. ADR-084.",
        risks="A fluent invented test list would be false completeness.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "`call_graph.rs` is the resident `callers_of` walk. Editing it can affect caller heuristic behaviour and GCTX-014 projection.",
                ["`crates/anvil-graph-cache/src/call_graph.rs`", "`crates/anvil-graph-cache/ARCHITECTURE.md`"],
            ),
            claim(
                2,
                "Deterministic fact",
                "Naming affected tests after an edit is `anvil_affected_tests` (GCTX-013). This CCTX spike is not that tool and is not the GCTX-031 `token_reduction` bench.",
                [
                    "`crates/anvil-gctx-types/src/lib.rs`",
                    "`crates/anvil-bench/src/scenarios/token_reduction.rs`",
                    "recall: this spec §12",
                ],
            ),
            claim(
                3,
                "Uncertainty",
                "This brief does not list the tests to run. If GCTX tools are unavailable, V2/V4 are blocked for that run — not a silent file-search substitute.",
                ["spec §12.7 step 2", "`crates/anvil-gctx-types/src/lib.rs`"],
                depends="2",
            ),
            claim(
                4,
                "Recommendation",
                "On V4, call `anvil_affected_tests` with `call_graph.rs` as a changed file (identity-only). On V3, do not invent a test list. Keep `anvil_validate_write` for the edit.",
                ["this brief Authority", "V4 drill-down `anvil_affected_tests`"],
                depends="2, 3",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Honest result is use GCTX, not this spike. Tool name is a gold recall key.",
        v3_handles=[
            "crates/anvil-graph-cache/src/call_graph.rs",
            "crates/anvil-gctx-types/src/lib.rs",
            "crates/anvil-bench/src/scenarios/token_reduction.rs",
        ],
        v4_gctx=["anvil_affected_tests", "anvil_impact_of_change"],
        recall_only=["anvil_affected_tests", "this spec §12"],
    )
)

add(
    dict(
        tid="T19",
        cls="novel structural question",
        prompt="Should Decision Brief evidence reuse GATT's in-band limits, extend them, or stay a distinct advisory contract?",
        scope=(
            "Honest result is **parked**. Spec §15.6. Do not fork GATT. Do not invent an ADR. "
            "CCTX-002 froze fixture evidence handles without deciding this."
        ),
        components="- ADR-142.\n- Spec §15.6 (recall).\n- This fixture contract §9.4 GATT note.",
        relationships="GATT attests graph-answer limits. Brief evidence handles are spike-only (path, GCTX locator, allowlisted ADR/spec). They are not an Attestation type.",
        invariants="§15.6 remains parked after CCTX-002. T19 success is visible uncertainty. Inventing an ADR is a protocol failure.",
        blast="None authorised. A silent pick would poison product architecture.",
        tests="Success **is** reporting parked. No GATT field change.",
        adrs="ADR-142 exists for GATT. It does not decide CCTX evidence-format reuse.",
        risks="Quoting a GCTX answer that already carries attestation is reuse of GCTX payload, not a CCTX decision to adopt GATT.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "ADR-142 accepts GATT: graph answers state in-band limits (shared Attestation, tagged counts, per-edge fidelity). That ADR does not authorise a Decision Brief evidence type.",
                ["`plans/decisions/142-graph-answer-attestation.md`"],
            ),
            claim(
                2,
                "Uncertainty",
                "Spec §15.6 remains parked: reuse GATT's in-band limits, extend them, or keep brief evidence as a distinct advisory contract. This spike does not pick an option and does not fork GATT.",
                [
                    "recall: this spec §15.6",
                    "`plans/decisions/142-graph-answer-attestation.md`",
                ],
                depends="1",
            ),
            claim(
                3,
                "Recommendation",
                "Report parked / uncertainty. Do not implement Attestation on briefs. Do not treat this brief as allow / warn / block.",
                ["this brief Authority", "`plans/decisions/142-graph-answer-attestation.md`"],
                depends="2",
            ),
        ],
        completeness="complete-for-scope",
        completeness_note="Honest result is parked §15.6. ADR-142 is a selected input.",
        v3_handles=["plans/decisions/142-graph-answer-attestation.md", "docs/guides/ai-context-delivery.md"],
        v4_gctx=["anvil_impact_of_change (existing GCTX payload may already carry attestation — not a CCTX GATT decision)"],
        recall_only=["this spec §15.6"],
    )
)

add(
    dict(
        tid="T20",
        cls="novel structural question",
        prompt="Is Context Compiler a sixth Graph Trust Surfaces track?",
        scope="Honest result is **no** — affinity, not membership. Operator shortlist remains five tracks.",
        components="- `plans/specs/2026-07-28-graph-trust-surfaces.md` (recall).\n- Index Graph Substrate / Graph Trust Surfaces notes (recall).\n- Spec §6 adjacent-work (recall).",
        relationships="Graph Trust Surfaces five questions: CONF/CEG, LSPNAV, CGBDG, POLCAP, SCA. CCTX sits in Graph Substrate next to GATT/CEG affinity.",
        invariants="Not a sixth track. Not a release claim. Not a product compiler in this spike.",
        blast="None. Wrong membership would pull CCTX into an operator-approved shortlist it is not on.",
        tests="Success is answering no, with affinity. Do not add a track via this brief.",
        adrs="none that adds a sixth track.",
        risks="Selected inputs do not include the GTS spec or index; completeness partial. The allowlisted overview/scope-guard do not name the five tracks. The brief states the corpus filing fact.",
        claims=[
            claim(
                1,
                "Deterministic fact",
                "Graph Trust Surfaces is an operator-approved five-track shortlist: CONF/CEG, LSPNAV, CGBDG, POLCAP, SCA.",
                [
                    "recall: `plans/specs/2026-07-28-graph-trust-surfaces.md`",
                    "recall: `plans/index.aps.md` Graph Trust Surfaces",
                ],
            ),
            claim(
                2,
                "Deterministic fact",
                "Context Compiler is affinity of intent (trustworthy graph answers), not membership in that shortlist. Home is Graph Substrate.",
                [
                    "recall: this spec §6",
                    "recall: `plans/index.aps.md` Graph Substrate CCTX row",
                ],
            ),
            claim(
                3,
                "Recommendation",
                "Answer no. Do not schedule CCTX as a sixth GTS track from this spike. Do not treat this brief as allow / warn / block.",
                ["this brief Authority"],
                depends="1, 2",
            ),
        ],
        completeness="partial",
        completeness_note="GTS spec and index are gold recall keys outside §12.3. The brief states the filing fact so V3 need not open them.",
        v3_handles=["docs/architecture/overview.md", "docs/vision/anvil-scope-guard.md"],
        v4_gctx=["graph://stats"],
        recall_only=[
            "plans/specs/2026-07-28-graph-trust-surfaces.md",
            "plans/index.aps.md",
            "this spec §6",
        ],
        invalidation="partial",
    )
)


def render_all() -> list[tuple[str, str]]:
    files = []
    for task in TASKS:
        invalidation = task.pop("invalidation", "current")
        tid = task["tid"]
        files.append((f"{tid}.md", brief(invalidation=invalidation, **task)))
        task["invalidation"] = invalidation  # restore if reused
    return files


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    ids = []
    for name, text in render_all():
        (OUT / name).write_text(text, encoding="utf-8")
        ids.append(name)
    print(f"wrote {len(ids)} fixtures to {OUT}")
    missing = [f"T{i:02d}.md" for i in range(1, 21) if f"T{i:02d}.md" not in ids]
    if missing:
        raise SystemExit(f"missing {missing}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

