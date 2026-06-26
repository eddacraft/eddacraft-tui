#!/usr/bin/env python3
"""Normalise a pnpm bench artifact dir into benchmarks/history/<date>.json."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

BENCHER_RE = re.compile(
    r"^test (?P<case>.+?) \.\.\. bench:\s+(?P<ns>[\d,]+) ns/iter \(\+/-\s+(?P<var>[\d,]+)\)"
)
GATE_RE = re.compile(
    r"^(?P<case>.+?): samples=(?P<samples>\d+) p50=(?P<p50>[\d.]+)ms "
    r"p95=(?P<p95>[\d.]+)ms p99=(?P<p99>[\d.]+)ms"
)
MIDEDIT_RE = re.compile(
    r"^(?P<case>midedit_(?:service|roundtrip)/empty)\s+time:\s+"
    r"\[[\d.]+ ms (?P<median>[\d.]+) ms [\d.]+ ms\]"
)

# Log stems produced by scripts/bench/run.sh → benches.* keys.
BENCH_LOGS: dict[str, str] = {
    "kernel": "kernel-bench.log",
    "checks": "checks-bench.log",
    "stress": "stress-bench.log",
    "antipattern_scan": "antipattern-scan-bench.log",
    "secret_scan_parallel": "secret-scan-parallel-bench.log",
    "walk_discovery": "walk-discovery-bench.log",
    "ipc_roundtrip": "ipc-roundtrip-bench.log",
    "midedit_roundtrip": "midedit-roundtrip-bench.log",
    "hot_read": "hot-read-bench.log",
    "call_lift": "call-lift-bench.log",
}

RESOURCE_LOGS: dict[str, str] = {
    "watch_resource_budget": "watch-resource-budget.log",
    "mcp_resource_budget": "mcp-resource-budget.log",
    "intercept_resource_budget": "intercept-resource-budget.log",
    "concurrent_resource_budget": "concurrent-resource-budget.log",
}

IPC_CASES = frozenset({"validation.service", "validation.roundtrip"})


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def git_short_commit(cwd: Path) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "--short", "HEAD"], cwd=cwd, text=True
    ).strip()


def rustc_version() -> str:
    return subprocess.check_output(["rustc", "--version"], text=True).strip().split()[1]


def parse_bencher_log(path: Path) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    for line in path.read_text().splitlines():
        match = BENCHER_RE.search(line)
        if not match:
            continue
        entries.append(
            {
                "case": match.group("case"),
                "ns_per_iter": int(match.group("ns").replace(",", "")),
                "variance_ns": int(match.group("var").replace(",", "")),
            }
        )
    return entries


def parse_gate_log(path: Path, *, ipc_only: bool = False) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    for line in path.read_text().splitlines():
        match = GATE_RE.search(line)
        if not match:
            continue
        case = match.group("case")
        if ipc_only and case not in IPC_CASES:
            continue
        entries.append(
            {
                "case": case,
                "samples": int(match.group("samples")),
                "p50_ms": float(match.group("p50")),
                "p95_ms": float(match.group("p95")),
                "p99_ms": float(match.group("p99")),
            }
        )
    return entries


def parse_midedit_log(path: Path) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    for line in path.read_text().splitlines():
        match = MIDEDIT_RE.search(line)
        if not match:
            continue
        entries.append(
            {
                "case": match.group("case"),
                "median": f"{match.group('median')} ms",
            }
        )
    return entries


def parse_budget_json(path: Path) -> dict[str, Any] | None:
    text = path.read_text()
    start = text.find("{")
    if start < 0:
        return None
    obj = json.loads(text[start:])
    if "sample" not in obj:
        return None
    return {
        "status": obj["status"],
        "steady_state_cpu_pct": obj["sample"]["steady_state_cpu_pct"],
        "peak_rss_mib": obj["sample"]["peak_rss_mib"],
        "budget": obj["budget"],
    }


def parse_intercept_budget(path: Path) -> dict[str, Any] | None:
    text = path.read_text()
    start = text.find("{")
    if start < 0:
        return None
    obj = json.loads(text[start:])
    out: dict[str, Any] = {}
    for key in ("idle", "burst"):
        if key not in obj:
            return None
        item = obj[key]
        out[key] = {
            "status": item["status"],
            "steady_state_cpu_pct": item["sample"]["steady_state_cpu_pct"],
            "peak_rss_mib": item["sample"]["peak_rss_mib"],
            "budget": item["budget"],
        }
    return out


def parse_mcp_budget(path: Path) -> dict[str, Any] | None:
    text = path.read_text()
    requests: int | None = None
    for line in text.splitlines():
        match = re.search(r"drove (\d+) tools/call", line)
        if match:
            requests = int(match.group(1))
    budget = parse_budget_json(path)
    if budget is None:
        return None
    if requests is not None:
        budget["requests"] = requests
    return budget


def parse_bench_surface(key: str, path: Path) -> list[dict[str, Any]]:
    if key == "midedit_roundtrip":
        return parse_midedit_log(path)
    if key == "ipc_roundtrip":
        return parse_gate_log(path, ipc_only=True)
    if key in {"hot_read", "call_lift"}:
        return parse_gate_log(path)
    return parse_bencher_log(path)


def infer_date_from_artifact(name: str) -> str | None:
    # benchmark-results/manual-20260626T202506Z → 2026-06-26
    match = re.search(r"manual-(\d{4})(\d{2})(\d{2})T", name)
    if not match:
        return None
    return f"{match.group(1)}-{match.group(2)}-{match.group(3)}"


def build_history(
    artifact_dir: Path,
    *,
    date: str,
    commit: str,
    rustc: str,
    trigger: str,
    source: str,
    host: dict[str, Any],
) -> dict[str, Any]:
    benches: dict[str, list[dict[str, Any]]] = {}
    for key, log_name in BENCH_LOGS.items():
        log_path = artifact_dir / log_name
        if not log_path.is_file():
            continue
        parsed = parse_bench_surface(key, log_path)
        if parsed:
            benches[key] = parsed

    history: dict[str, Any] = {
        "schema_version": 1,
        "run": {
            "date": date,
            "commit": commit,
            "rustc": rustc,
            "trigger": trigger,
            "host": host,
            "source": source,
        },
        "benches": benches,
    }

    for key, log_name in RESOURCE_LOGS.items():
        log_path = artifact_dir / log_name
        if not log_path.is_file():
            continue
        if key == "intercept_resource_budget":
            parsed = parse_intercept_budget(log_path)
        elif key == "mcp_resource_budget":
            parsed = parse_mcp_budget(log_path)
        else:
            parsed = parse_budget_json(log_path)
        if parsed is not None:
            history[key] = parsed

    return history


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Normalise a benchmark-results/manual-* artifact directory "
            "into benchmarks/history/<date>.json."
        ),
        epilog=(
            "Expects log stems from scripts/bench/run.sh "
            "(kernel-bench.log, hot-read-bench.log, …). "
            "Missing logs are skipped; empty parses are omitted."
        ),
    )
    parser.add_argument(
        "artifact_dir",
        type=Path,
        help="Path to benchmark-results/manual-<timestamp>/",
    )
    parser.add_argument(
        "--date",
        help="Run date (YYYY-MM-DD). Defaults to the timestamp in artifact_dir.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Output JSON path (default: benchmarks/history/<date>.json under repo root).",
    )
    parser.add_argument(
        "--commit",
        help="Git commit (short). Defaults to HEAD in the repo.",
    )
    parser.add_argument("--trigger", default="manual pnpm bench (quiet box)")
    parser.add_argument(
        "--source",
        help="Provenance string. Defaults to the artifact_dir path.",
    )
    parser.add_argument("--hostname", help="Host label for run.host.hostname")
    parser.add_argument("--cpus", type=int, help="CPU count for run.host.cpus")
    parser.add_argument(
        "--host-note",
        default="local reference box; sequential quiet-box run via scripts/bench/run.sh surfaces",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print JSON to stdout instead of writing a file.",
    )
    args = parser.parse_args(argv)

    artifact_dir = args.artifact_dir.resolve()
    if not artifact_dir.is_dir():
        parser.error(f"artifact dir not found: {artifact_dir}")

    root = repo_root()
    date = args.date or infer_date_from_artifact(artifact_dir.name)
    if not date:
        parser.error(
            "could not infer --date from artifact dir name; pass --date YYYY-MM-DD"
        )

    output = args.output or (root / "benchmarks" / "history" / f"{date}.json")
    commit = args.commit or git_short_commit(root)
    rustc = rustc_version()
    source = args.source or str(artifact_dir)

    host: dict[str, Any] = {"note": args.host_note}
    if args.hostname:
        host["hostname"] = args.hostname
    if args.cpus is not None:
        host["cpus"] = args.cpus

    history = build_history(
        artifact_dir,
        date=date,
        commit=commit,
        rustc=rustc,
        trigger=args.trigger,
        source=source,
        host=host,
    )

    payload = json.dumps(history, indent=2) + "\n"
    if args.dry_run:
        sys.stdout.write(payload)
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(payload)
    print(f"Wrote {output}")
    for key, entries in sorted(history.get("benches", {}).items()):
        print(f"  benches.{key}: {len(entries)} cases")
    for key in RESOURCE_LOGS:
        if key in history:
            print(f"  {key}: present")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())