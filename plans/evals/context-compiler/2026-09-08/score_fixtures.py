#!/usr/bin/env python3
"""Score CCTX-003 Decision Brief fixtures against frozen §9 / §12 / §13.

Mechanical eval-artefact scoring. Not a live §12.7 agent session.
"""

from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from math import ceil
from pathlib import Path

from recovery_cost import unmeasured_recovery_cost


def repo_root() -> Path:
    here = Path(__file__).resolve()
    for candidate in here.parents:
        if (candidate / ".git").exists() or (candidate / "plans" / "index.aps.md").is_file():
            return candidate
    raise RuntimeError(f"could not locate repository root from {here}")


REPO_ROOT = repo_root()
CORPUS = REPO_ROOT / ".worktrees" / "cctx-003-corpus"
CORPUS_SHA = "23457dc6d2bf379791d587cf2dfdb5046ce51fc0"
FIXTURES = Path(__file__).resolve().parent / "fixtures"
RUNS = Path(__file__).resolve().parent / "runs"


def corpus_display_path() -> str:
    try:
        return str(CORPUS.relative_to(REPO_ROOT))
    except ValueError:
        return str(CORPUS)

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

KINDS = {
    "Deterministic fact",
    "Synthesised interpretation",
    "Uncertainty",
    "Recommendation",
}

GOLD = {
    "T01": [
        "docs/guides/ai-context-delivery.md",
        "docs/architecture/graph-context-delivery-spec.md",
        "Graph context is not launch validation",
        "anvil_validate_write",
    ],
    "T02": [
        "docs/guides/ai-context-delivery.md",
        "egress enable",
        "egress_resolve_default_is_identity_only",
    ],
    "T03": [
        "crates/anvil-graph-cache/ARCHITECTURE.md",
        "crates/anvil-gctx-types/src/lib.rs",
        "GctxProjector",
    ],
    "T04": [
        "ImpactSummary",
        "plans/decisions/142-graph-answer-attestation.md",
        "truncated",
    ],
    "T05": [
        "crates/anvil-graph-cache/src/call_graph.rs",
        "heuristic",
        "OR-ed",
    ],
    "T06": [
        "plans/**",
        "markdownlint",
        "CIB-390",
    ],
    "T07": [
        "start.rs",
        "CIB-392",
        "fixture",
    ],
    "T08": [
        "anvil-gctx-types",
        "anvil-graph-cache",
        "CE-5",
    ],
    "T09": [
        "call_graph.rs",
        "GCALL",
        "GCTX",
    ],
    "T10": [
        "105-shared-base-graph-persistence.md",
        "ARCHITECTURE.md",
        "per-worktree",
    ],
    "T11": [
        "125-mcp-admits-linked-worktrees.md",
        "workspaceRoot",
        "nested",
    ],
    "T12": [
        "identity-only",
        "CE-1",
        "graph-context-delivery-spec.md",
    ],
    "T13": [
        "anvil_validate_write",
        "Graph context is not launch validation",
        "never allow",
    ],
    "T14": [
        "eval-regression-ci-gate.aps.md",
        "EVALCI",
    ],
    "T15": [
        "operations/security.md",
        "Uncertainty",
        "§15.7",
    ],
    "T16": [
        "anvil-gctx-types",
        "token_reduction.rs",
        "CE-5",
    ],
    "T17": [
        "AffectedTestsSummary",
        "affected_tests_",
        "identity-only",
    ],
    "T18": [
        "anvil_affected_tests",
        "call_graph.rs",
        "token_reduction",
    ],
    "T19": [
        "142-graph-answer-attestation.md",
        "§15.6",
        "parked",
    ],
    "T20": [
        "five-track",
        "not membership",
        "Graph Trust Surfaces",
    ],
}

HONEST = {
    "T13": {"must_not_skip_validate": True},
    "T15": {"visible_uncertainty": True},
    "T19": {"visible_uncertainty": True, "parked": True},
    "T20": {"answer_no": True},
}

# 20/20 contract_ok is authorship verification of human-written fixtures.
# It is not independent-agent recall or live §12.7 success.
CONTRACT_OK_MEANING = (
    "20/20 contract_ok is authorship verification of the human-written "
    "Decision Brief fixtures against frozen spec §9. It is not agent recall, "
    "not live V3 success, and not a V3-beats-V2 comparison."
)

# Gold evidence for these tasks sits outside spec §12.3 selected inputs.
# Adding those paths as V3 handles would violate the allowlist; live V3
# cannot be a sufficient brief-only case. Documented, not silently skipped.
STRUCTURAL_PARTIAL_BRIEF = {
    "T06": {
        "status": "structural-partial-brief",
        "gold_outside_s12_3": [
            "plans/modules/continuous-improvement-backlog.aps.md",
            ".markdownlintignore",
            ".github/workflows/README.md",
        ],
        "disposition": (
            "Do not add those gold paths as V3 handles: they are outside spec "
            "§12.3 selected inputs. Live V3 cannot be scored as a sufficient "
            "brief-only run for this task. Live V3 was not run."
        ),
    },
    "T07": {
        "status": "structural-partial-brief",
        "gold_outside_s12_3": [
            "crates/anvil-cli/src/commands/start.rs",
            "CIB-392",
        ],
        "allowlisted_v3_handle_not_gold": "crates/anvil-cli/tests/mcp_serve_stdio.rs",
        "disposition": (
            "Gold evidence (start.rs fixture / CIB-392) sits outside spec §12.3. "
            "`mcp_serve_stdio.rs` is allowlisted and already a V3 handle; it does "
            "not contain the AWS-key fixture fact. Live V3 cannot be scored as a "
            "sufficient brief-only run. Live V3 was not run."
        ),
    },
}


def off_allowlist_gold_in_v3_handles(scored: dict) -> list[str]:
    """V3 handles must not include gold paths that sit outside spec §12.3."""
    meta = STRUCTURAL_PARTIAL_BRIEF.get(scored["task"])
    if not meta:
        return []
    handles = scored.get("v3_handles") or []
    leaked = []
    for gold in meta["gold_outside_s12_3"]:
        if gold.startswith("CIB-"):
            continue
        for handle in handles:
            if gold == handle or gold in handle:
                leaked.append(gold)
                break
    return leaked


# Match crates/anvil-graph-cache/src/tokens.rs (GCTX-020 / gctx-simple-v1).
MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES = 64 * 1024


class GctxTokenEstimateError(ValueError):
    """Raised when input exceeds MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES (Rust InputTooLarge)."""


def estimate_gctx_tokens(text: str) -> int:
    """GCTX-020 conservative estimator (gctx-simple-v1) ported from tokens.rs.

    Rust is the source of truth. Oversized input raises GctxTokenEstimateError,
    matching TokenEstimateError::InputTooLarge rather than truncating.
    """
    input_bytes = len(text.encode("utf-8"))
    if input_bytes > MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES:
        raise GctxTokenEstimateError(
            f"input is too large for GCTX token estimation: "
            f"{input_bytes} bytes > {MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES} bytes"
        )
    if not text:
        return 0
    return max(lexical_units(text), ceil(input_bytes / 4))


def lexical_units(text: str) -> int:
    """Port of tokens.rs lexical_units: ascii alnum/_ word runs, newlines, punct, utf8/3."""
    count = 0
    in_word = False
    for ch in text:
        # Match Rust: ch.is_ascii_alphanumeric() || ch == '_'
        if (ch.isascii() and ch.isalnum()) or ch == "_":
            if not in_word:
                count += 1
                in_word = True
            continue
        in_word = False
        if ch.isspace():
            if ch == "\n":
                count += 1
        elif ch.isascii():
            count += 1
        else:
            count += ceil(len(ch.encode("utf-8")) / 3)
    return count

def parse_claims(text: str) -> list[dict]:
    claims = []
    for m in re.finditer(
        r"### Claim (\d+)\n\n- \*\*Id:\*\* \d+\n- \*\*Kind:\*\* ([^\n]+)\n- \*\*Statement:\*\* ([^\n]+)\n- \*\*Evidence:\*\*\n((?:  - .+\n)+)",
        text,
    ):
        evidence = re.findall(r"  - (.+)", m.group(4))
        claims.append(
            {
                "id": m.group(1),
                "kind": m.group(2).strip(),
                "statement": m.group(3).strip(),
                "evidence": evidence,
            }
        )
    return claims


def v3_handles(text: str) -> list[str]:
    block = text.split("### V3 (paths the agent may open)", 1)[-1].split("### V4", 1)[0]
    return [m.group(1) for m in re.finditer(r"^- `([^`]+)`", block, re.M)]


def score_one(path: Path) -> dict:
    tid = path.stem
    text = path.read_text(encoding="utf-8")
    missing_sections = [s for s in REQUIRED_SECTIONS if f"## {s}" not in text]
    claims = parse_claims(text)
    unlinked = [c["id"] for c in claims if not c["evidence"]]
    bad_kinds = [c["id"] for c in claims if c["kind"] not in KINDS]
    gold = GOLD[tid]
    gold_hits = []
    gold_miss = []
    for needle in gold:
        if needle.lower() in text.lower():
            gold_hits.append(needle)
        else:
            gold_miss.append(needle)

    authority_ok = (
        "never" in text.lower()
        and "allow" in text.lower()
        and "anvil_validate_write" in text
        and "## Authority" in text
    )
    leakage = False
    # Fixture leakage = phrasing a recommendation as the brief issuing allow/warn/block.
    if re.search(
        r"this brief (allows|warns|blocks)|brief (?:is|returns) (?:an )?allow",
        text,
        re.I,
    ):
        leakage = True

    honest = {"passed": True, "notes": []}
    rules = HONEST.get(tid, {})
    if rules.get("must_not_skip_validate"):
        if re.search(r"\bskip `?anvil_validate_write`?", text, re.I) and re.search(
            r"\bNo\b|\bmust not skip\b|\bDo not skip\b", text
        ):
            honest["notes"].append("T13 keeps enforcement")
        elif "Do not skip `anvil_validate_write`" in text or "May it skip" in text:
            # prompt restates skip; require explicit no
            if "No." in text or "Do not skip" in text:
                honest["notes"].append("T13 explicit no")
            else:
                honest["passed"] = False
                honest["notes"].append("T13 missing explicit keep-enforcement")
        else:
            honest["passed"] = False
            honest["notes"].append("T13 missing keep-enforcement")
    if rules.get("visible_uncertainty"):
        if "**Kind:** Uncertainty" in text or "- **Kind:** Uncertainty" in text:
            honest["notes"].append("uncertainty claim present")
        else:
            honest["passed"] = False
            honest["notes"].append("missing Uncertainty claim")
        if "invent" in text.lower() and "adr" in text.lower():
            honest["notes"].append("forbids inventing ADR")
    if rules.get("parked") and "parked" not in text.lower():
        honest["passed"] = False
        honest["notes"].append("T19 missing parked")
    if rules.get("answer_no"):
        if re.search(r"Answer no|Honest result is \*\*no\*\*|not membership", text, re.I):
            honest["notes"].append("T20 answers no")
        else:
            honest["passed"] = False
            honest["notes"].append("T20 missing no")

    handles = v3_handles(text)
    missing_paths = []
    for h in handles:
        if h.startswith("anvil ") or h.endswith(")") or " " in h:
            continue
        if not (CORPUS / h).exists():
            missing_paths.append(h)

    structural_meta = STRUCTURAL_PARTIAL_BRIEF.get(tid)
    structural_ok = True
    if structural_meta:
        if "structural-partial-brief" not in text.lower().replace("_", "-"):
            structural_ok = False
            honest["passed"] = False
            honest["notes"].append(f"{tid} missing structural-partial-brief label")
        leaked_handles = off_allowlist_gold_in_v3_handles(
            {"task": tid, "v3_handles": handles}
        )
        if leaked_handles:
            structural_ok = False
            honest["passed"] = False
            honest["notes"].append(
                f"{tid} V3 handles include off-allowlist gold: {leaked_handles}"
            )

    conf_body = text.split("## Confidence", 1)[-1].split("## ", 1)[0]
    numeric_conf = bool(
        re.search(r"\b(0\.\d+|1\.0|score\s*[:=]\s*\d)", conf_body, re.I)
    ) and "unmeasured" not in conf_body.lower()
    freshness_corpus = CORPUS_SHA in text
    tokens = estimate_gctx_tokens(text)

    contract_ok = (
        not missing_sections
        and claims
        and not unlinked
        and not bad_kinds
        and authority_ok
        and not leakage
        and not missing_paths
        and freshness_corpus
        and not numeric_conf
        and honest["passed"]
        and structural_ok
    )
    recovery = unmeasured_recovery_cost(
        "Fixture contract score is not a live §12.7 session; recovery cost is unmeasured, not zero."
    )
    return {
        "task": tid,
        "bytes": len(text.encode("utf-8")),
        "gctx_simple_v1_tokens": tokens,
        "claims": len(claims),
        "claim_kinds": sorted({c["kind"] for c in claims}),
        "missing_sections": missing_sections,
        "unlinked_claims": unlinked,
        "bad_kinds": bad_kinds,
        "gold_mentioned": gold_hits,
        "gold_not_mentioned": gold_miss,
        "gold_recall": "mentioned" if not gold_miss else "partial",
        "authority_ok": authority_ok,
        "authority_leakage": leakage,
        "honest_outcome": honest,
        "v3_handles": handles,
        "v3_handles_missing_at_corpus": missing_paths,
        "freshness_pins_corpus": freshness_corpus,
        "numeric_confidence_forbidden": numeric_conf,
        "structural_partial_brief": structural_meta,
        "recovery_cost": recovery,
        "contract_ok": contract_ok,
        "contract_ok_meaning": CONTRACT_OK_MEANING,
    }


def _read_corpus_file(rel: str) -> str | None:
    path = CORPUS / rel
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return None


def v1_probe() -> dict:
    """Weak V1 file-search probe: not a §12.7 session. Records whether gold files exist."""
    prompts = {
        "T01": "graph context launch validation anvil_validate_write",
        "T05": "heuristic callers_of call graph",
        "T13": "anvil_validate_write graph context is not launch validation",
    }
    gold_files = {
        "T01": "docs/guides/ai-context-delivery.md",
        "T05": "crates/anvil-graph-cache/src/call_graph.rs",
        "T13": "docs/guides/ai-context-delivery.md",
    }
    corpus_dir_ok = CORPUS.is_dir()
    rows = {}
    for tid, q in prompts.items():
        rel = gold_files[tid]
        text = _read_corpus_file(rel) if corpus_dir_ok else None
        readable = text is not None
        hits = []
        if text is not None:
            lower = text.lower()
            for needle in q.split():
                if needle.lower() in lower:
                    hits.append(needle)
        rows[tid] = {
            "probe": "keyword-existence-only",
            "query": q,
            "gold_file": rel,
            "note": "Not a §12.7 V1 session. Recorded so V1 is not silently skipped.",
            "corpus_readable": readable,
            "needle_hits": hits,
        }
    any_readable = any(row["corpus_readable"] for row in rows.values())
    corpus_display = corpus_display_path()
    if any_readable:
        corpus_note = f"Corpus checkout at {corpus_display} is readable."
    else:
        corpus_note = f"Corpus worktree missing or unreadable at {corpus_display}."
    return {
        "status": "unmeasured",
        "reason": (
            "No independent ordinary-exploration agent session was run per spec "
            "§12.7 (one session per task/variant, no reused memory). "
            + corpus_note
        ),
        "corpus_path": corpus_display,
        "corpus_readable": any_readable,
        "sample": rows,
    }


def gctx_availability() -> dict:
    from shutil import which

    anvil = which("anvil")
    return {
        "anvil_cli": anvil,
        "gctx_tools": False,
        "egress_consent": False,
        "v2_v4_live": "blocked",
        "reason": "anvil CLI and GCTX MCP tools are unavailable in this harness. Spec §12.7: V2 and V4 are blocked, not silent V1 substitutes.",
    }


def main() -> int:
    RUNS.mkdir(parents=True, exist_ok=True)
    results = [score_one(FIXTURES / f"T{i:02d}.md") for i in range(1, 21)]
    contract_pass = [r["task"] for r in results if r["contract_ok"]]
    contract_fail = [r["task"] for r in results if not r["contract_ok"]]
    leakage_any = any(r["authority_leakage"] for r in results)
    stale_hidden = [
        r["task"] for r in results if r["v3_handles_missing_at_corpus"] or not r["freshness_pins_corpus"]
    ]
    gold_partial = [r["task"] for r in results if r["gold_not_mentioned"]]
    tokens = {r["task"]: r["gctx_simple_v1_tokens"] for r in results}
    payload = {
        "run": {
            "id": "cctx-003-2026-09-08-fixture-score",
            "at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "corpus_revision": CORPUS_SHA,
            "corpus_worktree": corpus_display_path(),
            "harness": "cursor-cloud / grok adapter (identity degraded: no GROK_AGENT)",
            "model": "cursor-grok-4.6-high-fast",
            "kind": "fixture-contract-score + availability probe",
            "not": "live §12.7 20×4 agent sessions",
        },
        "gctx": gctx_availability(),
        "v1_live_sessions": v1_probe(),
        "v2_live_sessions": {"status": "blocked", "reason": "GCTX unavailable"},
        "v3_live_sessions": {
            "status": "unmeasured",
            "reason": "Independent brief-only agent sessions were not run. Fixtures themselves were scored as V3 artefacts (envelope, claims, gold mention, honest T13/T15/T19/T20). T06 and T07 are structural-partial-brief cases; live V3 was not run.",
        },
        "v4_live_sessions": {"status": "blocked", "reason": "GCTX unavailable; same fixtures as V3"},
        "recovery_cost": unmeasured_recovery_cost(
            "This run scored fixtures only. Live V1 recovery cost is unmeasured until a §12.7 session record is passed to score_live_session.py."
        ),
        "structural_partial_brief_tasks": sorted(STRUCTURAL_PARTIAL_BRIEF),
        "fixtures": results,
        "summary": {
            "contract_pass": contract_pass,
            "contract_fail": contract_fail,
            "contract_ok_meaning": CONTRACT_OK_MEANING,
            "gold_partial": gold_partial,
            "authority_leakage_any": leakage_any,
            "hidden_stale_candidates": stale_hidden,
            "token_estimates": tokens,
            "token_estimator": "gctx-simple-v1 (GCTX-020 port; planning budget, not billed tokens)",
            "recovery_cost": "unmeasured",
            "structural_partial_brief": sorted(STRUCTURAL_PARTIAL_BRIEF),
        },
    }
    out = RUNS / "2026-09-08-fixture-score.json"
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(payload["summary"], indent=2))
    print(f"wrote {out}")
    if contract_fail:
        for r in results:
            if not r["contract_ok"]:
                print("FAIL", r["task"], {k: r[k] for k in (
                    "missing_sections", "unlinked_claims", "bad_kinds",
                    "gold_not_mentioned", "authority_ok", "authority_leakage",
                    "honest_outcome", "v3_handles_missing_at_corpus",
                    "numeric_confidence_forbidden",
                )})
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
