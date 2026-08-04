#!/usr/bin/env python3
"""Normalise kindling profile / Criterion artefacts into benchmarks/history/kindling/.

Consumer-side evidence only. Kindling results are filed in the anvil repo so the
public kindling product never carries anvil adoption benchmarks.

Inputs (any combination):
  --standard path/to/standard-profile.json
  --stress   path/to/stress-profile.json
  --criterion path/to/criterion-summary.json

Scratch dirs from local runs typically look like:
  benchmark-results/manual-<ts>-kindling/{kindling-perf,kindling-stress}.json

Output:
  benchmarks/history/kindling/<YYYY-MM-DD>.json
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

# Proposed KFIT-005 performance contract (p95 unless noted). Units match
# workload_id scale: ms for cold start / page / ranked / concurrent / replay
# wall; us for append paths. None = no budget (report-only).
BUDGETS: dict[str, dict[str, float | None]] = {
    "cold-start/runtime-start": {"p95_ms": 50.0},
    "direct-service/append": {"p95_us": 500.0},
    "daemon/spooled-append-warm": {"p95_us": 1000.0},
    "daemon/append-concurrent": {"p95_ms": 5.0},
    "daemon/list-page": {"p95_ms": 10.0},
    # Ranked retrieve budget is meaningful near ~25k rows (standard profile).
    "direct-service/ranked-retrieve": {"p95_ms": 50.0},
    "daemon/ranked-retrieve": {"p95_ms": 50.0},
    "outage-recovery/spool-append": {"p95_us": 5000.0},
    "outage-recovery/spool-append-early": {"p95_us": 5000.0},
    "outage-recovery/spool-append-late": {"p95_us": 5000.0},
    # Replay uses a floor in rows/s rather than p95 latency.
    "outage-recovery/spool-replay": {"min_rows_per_s": 2000.0},
    # Full scans are export/projection rebuild only — no interactive budget.
    "direct-service/list-full-scan": {},
    "daemon/list-full-scan": {},
    "direct-service/list-page": {},
}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def us_to_ms(us: float) -> float:
    return us / 1000.0


def round_num(value: float | None, digits: int = 4) -> float | None:
    if value is None:
        return None
    if isinstance(value, float) and (math.isnan(value) or math.isinf(value)):
        return None
    return round(float(value), digits)


def workload_id(group: str, metric: str) -> str:
    return f"{group}/{metric}"


def verdict_for(
    wid: str,
    *,
    p95_us: float | None,
    rows_per_s: float | None,
    profile: str,
) -> str | None:
    budget = BUDGETS.get(wid)
    if budget is None or not budget:
        return "report"
    if "min_rows_per_s" in budget and budget["min_rows_per_s"] is not None:
        if rows_per_s is None:
            return "unknown"
        return "pass" if rows_per_s >= float(budget["min_rows_per_s"]) else "fail"
    # Ranked budgets only enforced on standard-scale (~25k) profiles.
    if wid.endswith("ranked-retrieve") and profile != "standard":
        return "report"
    if p95_us is None:
        return "unknown"
    if "p95_us" in budget and budget["p95_us"] is not None:
        return "pass" if p95_us <= float(budget["p95_us"]) else "fail"
    if "p95_ms" in budget and budget["p95_ms"] is not None:
        return "pass" if us_to_ms(p95_us) <= float(budget["p95_ms"]) else "fail"
    return "report"


def extract_profile(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text())
    profile_name = data.get("profile", {}).get("name") or path.stem
    system = data.get("system") or {}
    workloads: list[dict[str, Any]] = []
    resources: list[dict[str, Any]] = []

    for group in data.get("groups") or []:
        gname = group.get("name") or "unknown"
        res = group.get("resources") or {}
        if res:
            resources.append(
                {
                    "group": gname,
                    "measurement_scope": res.get("measurementScope"),
                    "peak_rss_mib": round_num(res.get("peakRssMib"), 3),
                    "rss_growth_mib": round_num(res.get("rssGrowthMib"), 3),
                    "peak_threads": res.get("peakThreads"),
                    "peak_fds": res.get("peakFileDescriptors"),
                    "cpu_cores": round_num(res.get("cpuCores"), 4),
                    "read_bytes": res.get("readBytes"),
                    "write_bytes": res.get("writeBytes"),
                    "logical_read_bytes": res.get("logicalReadBytes"),
                    "logical_write_bytes": res.get("logicalWriteBytes"),
                    "storage_bytes": group.get("storageBytes"),
                    "spool_bytes": group.get("spoolBytes"),
                }
            )
        for metric in group.get("metrics") or []:
            mname = metric.get("name") or "unknown"
            lat = metric.get("latency") or {}
            wid = workload_id(gname, mname)
            p50_us = lat.get("p50Us")
            p95_us = lat.get("p95Us")
            p99_us = lat.get("p99Us")
            mean_us = lat.get("meanUs")
            ops = lat.get("operationsPerSecond")
            rows = metric.get("rowsProcessed")
            samples = lat.get("samples")
            rows_per_s = None
            if mname == "spool-replay":
                # Prefer wall time from mean_us (batch duration). Fall back to
                # harness operationsPerSecond when mean_us is missing.
                if rows and mean_us and mean_us > 0:
                    rows_per_s = rows / (mean_us / 1_000_000.0)
                elif ops is not None:
                    rows_per_s = float(ops)

            entry: dict[str, Any] = {
                "id": wid,
                "profile": profile_name,
                "group": gname,
                "metric": mname,
                "samples": samples,
                "operation_count": metric.get("operationCount"),
                "rows_processed": rows,
                "p50_us": round_num(p50_us, 3),
                "p95_us": round_num(p95_us, 3),
                "p99_us": round_num(p99_us, 3),
                "mean_us": round_num(mean_us, 3),
                "p50_ms": round_num(us_to_ms(p50_us), 4) if p50_us is not None else None,
                "p95_ms": round_num(us_to_ms(p95_us), 4) if p95_us is not None else None,
                "ops_per_s": round_num(ops, 3) if ops is not None else None,
            }
            if rows_per_s is not None:
                entry["rows_per_s"] = round_num(rows_per_s, 2)
            entry["verdict"] = verdict_for(
                wid,
                p95_us=p95_us,
                rows_per_s=entry.get("rows_per_s"),
                profile=profile_name,
            )
            budget = BUDGETS.get(wid) or {}
            if budget:
                entry["budget"] = budget
            workloads.append(entry)

    return {
        "profile": data.get("profile"),
        "system": system,
        "generated_at_epoch_ms": data.get("generatedAtEpochMs"),
        "workloads": workloads,
        "resources": resources,
        "source_path": str(path),
    }


def extract_criterion(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text())
    benches: list[dict[str, Any]] = []
    for b in data.get("benchmarks") or []:
        mean_ns = b.get("meanNs")
        median_ns = b.get("medianNs")
        conf = b.get("confidence95") or {}
        benches.append(
            {
                "benchmark": b.get("benchmark"),
                "mean_ns": round_num(mean_ns, 2),
                "median_ns": round_num(median_ns, 2),
                "mean_us": round_num(mean_ns / 1000.0, 3) if mean_ns is not None else None,
                "median_us": round_num(median_ns / 1000.0, 3)
                if median_ns is not None
                else None,
                "confidence95": {
                    "lower_ns": round_num(conf.get("lower_bound"), 2),
                    "upper_ns": round_num(conf.get("upper_bound"), 2),
                }
                if conf
                else None,
            }
        )
    return {
        "harness": data.get("harness"),
        "source_commit": data.get("sourceCommit"),
        "source_state": data.get("sourceState"),
        "generated_at": data.get("generatedAt"),
        "benchmarks": benches,
        "source_path": str(path),
    }


def build_history(
    *,
    date: str,
    standard: Path | None,
    stress: Path | None,
    criterion: Path | None,
    kindling_commit: str | None,
    anvil_commit: str | None,
    host_note: str | None,
    trigger: str,
    source: str | None,
    caveats: list[str],
) -> dict[str, Any]:
    profiles: dict[str, Any] = {}
    workloads: list[dict[str, Any]] = []
    resources: list[dict[str, Any]] = []
    system: dict[str, Any] | None = None

    for label, path in (("standard", standard), ("stress", stress)):
        if path is None:
            continue
        extracted = extract_profile(path)
        profiles[label] = {
            "config": extracted["profile"],
            "generated_at_epoch_ms": extracted["generated_at_epoch_ms"],
            "source_path": extracted["source_path"],
        }
        workloads.extend(extracted["workloads"])
        for r in extracted["resources"]:
            resources.append({"profile": label, **r})
        system = system or extracted["system"]

    criterion_block = extract_criterion(criterion) if criterion else None
    if criterion_block and not kindling_commit:
        kindling_commit = criterion_block.get("source_commit")

    run: dict[str, Any] = {
        "date": date,
        "kindling_commit": kindling_commit,
        "anvil_commit": anvil_commit,
        "trigger": trigger,
        "host": {
            "os": (system or {}).get("os"),
            "architecture": (system or {}).get("architecture"),
            "logical_cpus": (system or {}).get("logicalCpus"),
            "release_build": (system or {}).get("releaseBuild"),
            "note": host_note
            or "local reference box; consumer-side kindling adoption evidence",
        },
        "source": source,
    }
    if caveats:
        run["partial"] = True
        run["caveats"] = caveats

    return {
        "schema_version": 1,
        "suite": "kindling",
        "run": run,
        "profiles": profiles,
        "workloads": workloads,
        "resources": resources,
        "criterion": criterion_block,
        "budgets_note": (
            "Budgets are proposed KFIT-005 contract values (2026-08-03 assessment). "
            "Ranked-retrieve p95 budgets apply to standard-scale only. Full scans "
            "are report-only. Resource RSS is shared-process and directional."
        ),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--date", required=True, help="YYYY-MM-DD for history file name")
    parser.add_argument("--standard", type=Path, help="standard profile JSON")
    parser.add_argument("--stress", type=Path, help="stress profile JSON")
    parser.add_argument("--criterion", type=Path, help="criterion summary JSON")
    parser.add_argument("--kindling-commit", default=None)
    parser.add_argument("--anvil-commit", default=None)
    parser.add_argument("--host-note", default=None)
    parser.add_argument(
        "--trigger",
        default="manual kindling adoption / KFIT evidence",
    )
    parser.add_argument("--source", default=None, help="scratch or audit source path")
    parser.add_argument(
        "--caveat",
        action="append",
        default=[],
        help="comparability caveat (repeatable); sets partial=true",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="output path (default: benchmarks/history/kindling/<date>.json)",
    )
    parser.add_argument(
        "--stdout",
        action="store_true",
        help="print JSON to stdout instead of writing a file",
    )
    args = parser.parse_args(argv)

    if not any([args.standard, args.stress, args.criterion]):
        parser.error("provide at least one of --standard, --stress, --criterion")

    for p in (args.standard, args.stress, args.criterion):
        if p is not None and not p.is_file():
            parser.error(f"not a file: {p}")

    history = build_history(
        date=args.date,
        standard=args.standard,
        stress=args.stress,
        criterion=args.criterion,
        kindling_commit=args.kindling_commit,
        anvil_commit=args.anvil_commit,
        host_note=args.host_note,
        trigger=args.trigger,
        source=args.source,
        caveats=list(args.caveat),
    )

    text = json.dumps(history, indent=2) + "\n"
    if args.stdout:
        sys.stdout.write(text)
        return 0

    out = args.out or (repo_root() / "benchmarks" / "history" / "kindling" / f"{args.date}.json")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text)
    print(f"wrote {out} ({len(history.get('workloads') or [])} workloads)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
