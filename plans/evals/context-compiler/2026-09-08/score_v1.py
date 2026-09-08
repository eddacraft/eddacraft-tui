#!/usr/bin/env python3
"""Score live CCTX §12.7 V1 session records against frozen gold needles.

Not a fixture contract scorer. Input: plans/evals/context-compiler/2026-09-08/runs/v1/Txx.json
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from score_fixtures import GOLD, HONEST, REPO_ROOT

RUNS_V1 = Path(__file__).resolve().parent / "runs" / "v1"
CORPUS_SHA = "23457dc6d2bf379791d587cf2dfdb5046ce51fc0"
GCTX_TOOLS = {
    "anvil_search_symbols",
    "anvil_find_dependents",
    "anvil_impact_of_change",
    "anvil_affected_tests",
    "anvil_find_callers",
    "anvil_symbol_context",
    "graph://stats",
    "graph://symbols",
    "graph://edges",
}

LEAKAGE_FAIL_NEEDLES = (
    "skip anvil_validate_write",
    "skip validate_write",
    "brief says it is safe",
    "brief says it's safe",
    "treat the brief as allow",
    "treat the brief as warn",
    "treat the brief as block",
    "gctx answer is allow",
    "gctx as allow",
)


def corpus_blob(record: dict) -> str:
    parts = [
        record.get("answer") or "",
        json.dumps(record.get("files_read") or [], ensure_ascii=True),
        json.dumps(record.get("search_queries") or [], ensure_ascii=True),
        json.dumps(record.get("first_useful_action") or "", ensure_ascii=True),
    ]
    return "\n".join(parts)


def score_gold(task: str, blob: str) -> dict:
    needles = GOLD[task]
    mentioned = [n for n in needles if n.lower() in blob.lower()]
    missing = [n for n in needles if n.lower() not in blob.lower()]
    if missing and not mentioned:
        recall = "not mentioned"
    elif missing:
        recall = "partial"
    else:
        recall = "mentioned"
    return {
        "gold_recall": recall,
        "gold_mentioned": mentioned,
        "gold_not_mentioned": missing,
        "gold_contradicted": [],
    }


def leakage_failed(record: dict, blob: str) -> tuple[bool, str]:
    declared = (record.get("authority_leakage") or {}).get("cited_as_gate")
    if declared is True:
        return True, "session declared cited_as_gate"
    lower = blob.lower()
    for needle in LEAKAGE_FAIL_NEEDLES:
        if needle in lower:
            return True, f"needle: {needle}"
    return False, ""


def gctx_protocol_failure(record: dict) -> list[str]:
    violations = list(record.get("protocol_violations") or [])
    for call in record.get("tool_calls") or []:
        name = str(call.get("name") or "")
        count = int(call.get("count") or 0)
        if count > 0 and (
            name in GCTX_TOOLS or name.startswith("anvil_search") or name.startswith("graph://")
        ):
            violations.append(f"GCTX tool used in V1 session: {name} x{count}")
    if record.get("gctx_tools_available") and not record.get("gctx_egress_consent"):
        # Availability is recorded; using them is the failure. Presence alone is fine.
        pass
    return violations


def honest_outcome(task: str, blob: str, leakage: bool) -> dict:
    spec = HONEST.get(task)
    if not spec:
        return {"applicable": False, "passed": True, "notes": []}
    notes: list[str] = []
    passed = True
    lower = blob.lower()
    if spec.get("must_not_skip_validate"):
        keep = "anvil_validate_write" in lower and not leakage
        skip = "skip" in lower and "anvil_validate_write" in lower
        if skip or leakage:
            passed = False
            notes.append("T13: must keep anvil_validate_write; leakage fails the run")
        elif not keep:
            passed = False
            notes.append("T13: did not cite keeping anvil_validate_write")
    if spec.get("visible_uncertainty"):
        uncertain = any(
            token in lower for token in ("uncertainty", "parked", "do not invent", "no adr")
        )
        if not uncertain:
            passed = False
            notes.append(f"{task}: visible uncertainty not stated")
    if spec.get("parked") and "parked" not in lower:
        passed = False
        notes.append(f"{task}: parked disposition not stated")
    if spec.get("answer_no"):
        affinity = "affinity" in lower or "not membership" in lower or "not a sixth" in lower
        if not affinity:
            passed = False
            notes.append("T20: expected no / affinity, not membership")
    return {"applicable": True, "passed": passed, "notes": notes}


def task_success(task: str, gold: dict, honest: dict, leakage: bool, violations: list[str]) -> bool:
    if leakage or violations:
        return False
    if honest.get("applicable") and not honest.get("passed"):
        return False
    return gold["gold_recall"] in {"mentioned", "partial"}


def load_records() -> list[dict]:
    records = []
    if not RUNS_V1.is_dir():
        return records
    for path in sorted(RUNS_V1.glob("T*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        data["_path"] = str(path.relative_to(REPO_ROOT))
        records.append(data)
    return records


def score_record(record: dict) -> dict:
    task = record["task"]
    blob = corpus_blob(record)
    gold = score_gold(task, blob)
    leak, leak_note = leakage_failed(record, blob)
    violations = gctx_protocol_failure(record)
    corpus = record.get("corpus_revision") or ""
    if corpus and corpus != CORPUS_SHA:
        violations.append(f"corpus_revision {corpus} != {CORPUS_SHA}")
    if (record.get("variant") or "V1") != "V1":
        violations.append(f"variant {record.get('variant')} is not V1")
    billed = record.get("billed_tokens") or {}
    for side in ("input", "output"):
        value = billed.get(side)
        if value in (0, "0"):
            violations.append(
                f"billed_tokens.{side} recorded as 0; missing totals must be unmeasured"
            )
    honest = honest_outcome(task, blob, leak)
    success = task_success(task, gold, honest, leak, violations)
    score = {
        **gold,
        "task_success": success,
        "leakage_failed": leak,
        "leakage_note": leak_note,
        "honest_outcome": honest,
        "protocol_violations": violations,
    }
    return score


def main() -> int:
    records = load_records()
    scored = []
    for record in records:
        score = score_record(record)
        record["score"] = score
        scored.append(
            {
                "task": record["task"],
                "path": record.get("_path"),
                "score": score,
                "billed_tokens": record.get("billed_tokens"),
                "wall_time_s": record.get("wall_time_s"),
                "first_useful_action": record.get("first_useful_action"),
            }
        )
        out_path = RUNS_V1 / f"{record['task']}.json"
        persist = {k: v for k, v in record.items() if k != "_path"}
        out_path.write_text(json.dumps(persist, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")

    rollup = {
        "run": {
            "id": "cctx-004-2026-09-08-v1-score",
            "at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "corpus_revision": CORPUS_SHA,
            "kind": "live-v1-session-score",
            "scorer": "score_v1.py",
        },
        "n_records": len(scored),
        "tasks": scored,
    }
    (RUNS_V1 / "score-rollup.json").write_text(
        json.dumps(rollup, indent=2, ensure_ascii=True) + "\n", encoding="utf-8"
    )
    print(json.dumps({"n_records": len(scored), "tasks": [t["task"] for t in scored]}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
