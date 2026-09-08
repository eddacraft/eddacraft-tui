#!/usr/bin/env python3
"""§13 recovery-cost metric for CCTX eval sessions.

Live V1 (and later V2/V3/V4) sessions record tokens and time spent after a
wrong path, including re-reads. Missing instrumentation is honest
`unmeasured`, never silent zero.

Not a product compiler. Spec §15 remains parked.
"""

from __future__ import annotations

from typing import Any


UNMEASURED = "unmeasured"

# File-like event kinds that can be a wrong path or a re-read.
PATH_EVENT_KINDS = frozenset({"read", "search", "open"})


def unmeasured_recovery_cost(reason: str) -> dict[str, Any]:
    """First-class §13 recovery-cost row when the session cannot be scored."""
    return {
        "status": "unmeasured",
        "metric": "recovery_cost",
        "tokens_after_wrong_path": UNMEASURED,
        "time_after_wrong_path_ms": UNMEASURED,
        "re_reads_after_wrong_path": UNMEASURED,
        "wrong_path_events": [],
        "first_wrong_path_at_ms": None,
        "reason": reason,
    }


def _norm_path(path: str | None) -> str:
    if not path:
        return ""
    return path.strip().strip("`")


def _is_gold(path: str, gold_paths: list[str]) -> bool:
    """True when ``path`` is a gold file, not a superstring of a gold needle.

    ``start.rs`` matches ``.../start.rs`` and not ``.../restart.rs``.
    """
    if not path:
        return False
    path_n = path.replace("\\", "/").lower()
    base = path_n.rsplit("/", 1)[-1]
    for gold in gold_paths:
        g = gold.strip().strip("`").replace("\\", "/").lower()
        if not g:
            continue
        if path_n == g or path_n.endswith("/" + g):
            return True
        if "/" not in g and base == g:
            return True
    return False


def score_recovery_cost(session: dict[str, Any]) -> dict[str, Any]:
    """Score one live-session record for spec §13 recovery cost.

    Session shape (scaffold; extra keys ignored):

    - ``task``, ``variant``
    - ``gold_paths``: recall keys for this task
    - ``events``: ordered list of ``{t_ms, kind, path, tokens}``
    - ``recovery_cost_status``: if ``unmeasured``, honour that even when events exist

    A wrong path is a read/search/open whose path is not a gold key.
    Recovery is every event from the first wrong path through the first later
    gold hit (inclusive), or through session end if gold is never reached.
    Re-reads are path-events that repeat a path already seen in the recovery
    window *before* that event.
    """
    explicit = session.get("recovery_cost_status")
    reason = session.get("reason") or session.get("recovery_cost_reason")
    if explicit == "unmeasured":
        return unmeasured_recovery_cost(
            reason or "Session declared recovery cost unmeasured."
        )

    events = session.get("events")
    if not events:
        return unmeasured_recovery_cost(
            reason
            or "No per-event tokens or timestamps were recorded; recovery cost is unmeasured, not zero."
        )

    gold_paths = [str(p) for p in session.get("gold_paths") or []]
    first_wrong_idx: int | None = None
    wrong_events: list[dict[str, Any]] = []
    has_token = False
    has_time = False

    for i, raw in enumerate(events):
        kind = str(raw.get("kind") or "").lower()
        path = _norm_path(raw.get("path"))
        if raw.get("tokens") is not None:
            has_token = True
        if raw.get("t_ms") is not None:
            has_time = True
        if kind not in PATH_EVENT_KINDS:
            continue
        if path and not _is_gold(path, gold_paths):
            if first_wrong_idx is None:
                first_wrong_idx = i
            if path not in {w["path"] for w in wrong_events}:
                wrong_events.append(
                    {
                        "index": i,
                        "kind": kind,
                        "path": path,
                        "t_ms": raw.get("t_ms"),
                        "tokens": raw.get("tokens"),
                    }
                )

    if first_wrong_idx is None:
        if not has_token and not has_time:
            return unmeasured_recovery_cost(
                reason
                or "Events exist but carry neither tokens nor timestamps; recovery cost is unmeasured."
            )
        return {
            "status": "none",
            "metric": "recovery_cost",
            "tokens_after_wrong_path": 0,
            "time_after_wrong_path_ms": 0,
            "re_reads_after_wrong_path": 0,
            "wrong_path_events": [],
            "first_wrong_path_at_ms": None,
            "reason": "No wrong-path file event; recovery cost is zero for this session.",
        }

    if not has_token or not has_time:
        return unmeasured_recovery_cost(
            reason
            or "Wrong-path events exist but tokens or timestamps are missing; recovery cost is unmeasured, not zero."
        )

    window_end = len(events)
    for j in range(first_wrong_idx + 1, len(events)):
        ev = events[j]
        kind = str(ev.get("kind") or "").lower()
        path = _norm_path(ev.get("path"))
        if kind in PATH_EVENT_KINDS and path and _is_gold(path, gold_paths):
            window_end = j + 1
            break

    window = events[first_wrong_idx:window_end]
    tokens = 0
    seen_paths: list[str] = []
    re_reads = 0
    for ev in window:
        tok = ev.get("tokens")
        if tok is None:
            return unmeasured_recovery_cost(
                "Recovery window is missing token counts; unmeasured, not zero."
            )
        tokens += int(tok)
        kind = str(ev.get("kind") or "").lower()
        path = _norm_path(ev.get("path"))
        if kind in PATH_EVENT_KINDS and path:
            if path in seen_paths:
                re_reads += 1
            else:
                seen_paths.append(path)

    t0 = events[first_wrong_idx].get("t_ms")
    t1 = events[window_end - 1].get("t_ms")
    if t0 is None or t1 is None:
        return unmeasured_recovery_cost(
            "Recovery window is missing timestamps; unmeasured, not zero."
        )

    return {
        "status": "measured",
        "metric": "recovery_cost",
        "tokens_after_wrong_path": tokens,
        "time_after_wrong_path_ms": int(t1) - int(t0),
        "re_reads_after_wrong_path": re_reads,
        "wrong_path_events": wrong_events,
        "first_wrong_path_at_ms": int(t0),
        "reason": (
            "Recovery window is first wrong-path event through first later gold "
            "hit (inclusive), or session end if gold was never reached."
        ),
    }
