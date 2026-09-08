#!/usr/bin/env python3
"""Live-run scorer scaffold for spec §13 recovery cost (V1 first).

Accepts one session JSON object (file or stdin). Records recovery cost as a
first-class metric, or honest ``unmeasured``. Does **not** run live sessions.
Does **not** claim live V3 was run.

Usage (from repository root)::

    python3 plans/evals/context-compiler/2026-09-08/score_live_session.py SESSION.json

Not a product compiler. Spec §15 remains parked.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from recovery_cost import score_recovery_cost, unmeasured_recovery_cost


def score_live_session(session: dict[str, Any]) -> dict[str, Any]:
    """Score one live §12.7 session record. Never invents a V3 live run."""
    variant = session.get("variant") or "V1"
    recovery = score_recovery_cost(session)
    if session.get("events") is None and session.get("recovery_cost_status") != "unmeasured":
        if recovery["status"] != "unmeasured":
            recovery = unmeasured_recovery_cost(
                session.get("reason")
                or "Live session record has no events; recovery cost unmeasured."
            )
    return {
        "task": session.get("task"),
        "variant": variant,
        "corpus_revision": session.get("corpus_revision"),
        "kind": "live-session-score-scaffold",
        "not": "live V3; this scaffold scores a provided session record only",
        "recovery_cost": recovery,
        "authority_leakage": session.get("authority_leakage", "unmeasured"),
        "billed_tokens": session.get("billed_tokens", "unmeasured"),
        "note": (
            "Recovery cost is a spec §13 metric. Missing provider or event "
            "instrumentation must stay unmeasured, never silent zero. "
            "20/20 fixture contract_ok is authorship verification, not agent recall."
        ),
    }


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if not args or args[0] in {"-h", "--help"}:
        print(__doc__.strip())
        return 0 if args and args[0] in {"-h", "--help"} else 2
    path = Path(args[0])
    if path.name == "-":
        session = json.load(sys.stdin)
    else:
        session = json.loads(path.read_text(encoding="utf-8"))
    scored = score_live_session(session)
    print(json.dumps(scored, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
