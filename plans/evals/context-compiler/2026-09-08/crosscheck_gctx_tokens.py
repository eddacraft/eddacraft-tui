#!/usr/bin/env python3
"""Cross-check Python gctx-simple-v1 port against Rust estimate_gctx_tokens.

Design-council follow-up (2026-09-08): before any numeric §13 thresholds,
prove the Python port in score_fixtures.py matches crates/anvil-graph-cache
tokens.rs (GCTX-020). Rust is the source of truth.

Usage (from repo root or this directory):
  python3 plans/evals/context-compiler/2026-09-08/crosscheck_gctx_tokens.py

Requires a working Cargo toolchain and builds
`eddacraft-anvil-graph-cache`'s `estimate_gctx_tokens_batch` example.
Writes a machine-readable summary under plans/audits/.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

from score_fixtures import (
    FIXTURES,
    GctxTokenEstimateError,
    MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES,
    estimate_gctx_tokens,
)

HERE = Path(__file__).resolve().parent
AUDITS = HERE.parents[2] / "audits"  # durable estimator xcheck evidence


def repo_root() -> Path:
    for candidate in HERE.parents:
        if (candidate / "Cargo.toml").is_file() and (
            candidate / "crates" / "anvil-graph-cache"
        ).is_dir():
            return candidate
    raise RuntimeError(f"could not locate repository root from {HERE}")


def synthetic_cases() -> list[tuple[str, str]]:
    max_b = MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES
    return [
        ("empty", ""),
        ("ascii_prose", "The quick brown fox jumps over the lazy dog."),
        (
            "code_identifiers",
            "export function alpha(value: string) {\n  return value.trim();\n}\n",
        ),
        ("underscored_idents", "hello_world foo_bar_baz snake_case"),
        ("newlines", "line one\nline two\n\nline four\n"),
        ("crlf", "a\r\nb\r\nc"),
        ("tabs_and_spaces", "a\tb  c"),
        ("ascii_punct", "a.b(c){d}[!];"),
        ("non_ascii_latin", "café naïve résumé"),
        ("non_ascii_cjk", "中文测试 日本語"),
        ("emoji", "ship it 🙂🚀"),
        ("nbsp", "a\u00a0b"),
        ("near_max_bytes", "x" * max_b),
        ("over_max_bytes", "x" * (max_b + 1)),
        (
            "rust_impl_ref",
            "impl Runner {\n    pub fn run(&self) -> Result<()> { Ok(()) }\n}\n",
        ),
        (
            "graph_summary_ref",
            "symbol alpha function public src/a.ts depends_on src/b.ts",
        ),
    ]


def fixture_cases() -> list[tuple[str, str]]:
    # Handful of real brief texts from the frozen CCTX-003 fixture set.
    names = ["T01.md", "T05.md", "T10.md", "T15.md", "T20.md"]
    out: list[tuple[str, str]] = []
    for name in names:
        path = FIXTURES / name
        out.append((f"fixture:{name}", path.read_text(encoding="utf-8")))
    return out


def py_estimate(text: str) -> dict:
    try:
        tokens = estimate_gctx_tokens(text)
        return {
            "ok": True,
            "tokens": tokens,
            "input_bytes": len(text.encode("utf-8")),
        }
    except GctxTokenEstimateError:
        return {
            "ok": False,
            "error": "input_too_large",
            "input_bytes": len(text.encode("utf-8")),
            "max_bytes": MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES,
        }


def rust_batch(root: Path, texts: list[str]) -> dict:
    env = os.environ.copy()
    target = env.get("CARGO_TARGET_DIR") or str(
        Path.home() / ".cache" / "anvil-targets" / root.name
    )
    env["CARGO_TARGET_DIR"] = target
    Path(target).mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "eddacraft-anvil-graph-cache",
            "--example",
            "estimate_gctx_tokens_batch",
            "--quiet",
        ],
        input=json.dumps(texts),
        capture_output=True,
        text=True,
        cwd=root,
        env=env,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"Rust batch oracle failed (exit {proc.returncode}):\n"
            f"stdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return json.loads(proc.stdout)


def compare(name: str, py: dict, rust: dict) -> tuple[bool, str]:
    if py.get("ok") != rust.get("ok"):
        return False, f"ok mismatch py={py} rust={rust}"
    if not py["ok"]:
        if py.get("error") != rust.get("error"):
            return False, f"error kind mismatch py={py} rust={rust}"
        if py.get("input_bytes") != rust.get("input_bytes"):
            return False, f"input_bytes mismatch on error py={py} rust={rust}"
        if py.get("max_bytes") != rust.get("max_bytes"):
            return False, f"max_bytes mismatch py={py} rust={rust}"
        return True, "error agree"
    if py["tokens"] != rust["tokens"]:
        return False, f"tokens {py['tokens']} != {rust['tokens']}"
    if py["input_bytes"] != rust["input_bytes"]:
        return False, f"input_bytes {py['input_bytes']} != {rust['input_bytes']}"
    return True, f"tokens={py['tokens']}"


def main() -> int:
    root = repo_root()
    cases = synthetic_cases() + fixture_cases()
    names = [n for n, _ in cases]
    texts = [t for _, t in cases]

    py_results = [py_estimate(t) for t in texts]
    rust_envelope = rust_batch(root, texts)

    if rust_envelope.get("max_bytes") != MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES:
        print(
            "FAIL: MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES mismatch "
            f"python={MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES} "
            f"rust={rust_envelope.get('max_bytes')}",
            file=sys.stderr,
        )
        return 1

    rust_results = rust_envelope["results"]
    if len(rust_results) != len(py_results):
        print("FAIL: result length mismatch", file=sys.stderr)
        return 1

    rows = []
    passed = 0
    failed = 0
    for name, py, rust in zip(names, py_results, rust_results):
        ok, detail = compare(name, py, rust)
        rows.append(
            {
                "name": name,
                "pass": ok,
                "detail": detail,
                "python": py,
                "rust": {
                    k: rust[k]
                    for k in ("ok", "tokens", "input_bytes", "error", "max_bytes")
                    if k in rust
                },
            }
        )
        status = "PASS" if ok else "FAIL"
        print(f"{status}\t{name}\t{detail}")
        if ok:
            passed += 1
        else:
            failed += 1

    AUDITS.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    out_path = AUDITS / f"{stamp}-gctx-token-estimator-xcheck.json"
    payload = {
        "date_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "estimator": "gctx-simple-v1",
        "source_of_truth": "crates/anvil-graph-cache/src/tokens.rs",
        "python_port": "plans/evals/context-compiler/2026-09-08/score_fixtures.py",
        "max_bytes": MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES,
        "pass": passed,
        "fail": failed,
        "total": passed + failed,
        "cases": rows,
        "intentional_divergences": [],
        "notes": (
            "Python now rejects input above MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES "
            "with GctxTokenEstimateError, matching Rust TokenEstimateError::InputTooLarge. "
            "No numeric §13 thresholds were set."
        ),
    }
    out_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"\nSummary: {passed} PASS / {failed} FAIL / {passed + failed} total")
    print(f"Evidence: {out_path}")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
