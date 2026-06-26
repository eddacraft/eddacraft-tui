#!/usr/bin/env python3
"""Conservative Worktrunk worktree cleanup assistant.

Dry-run is the default. The helper classifies git worktrees against a freshly
pruned origin/main and only marks clean disposable branches eligible when the
branch is proven merged/equivalent to origin/main. Apply mode requires explicit
branch names and delegates deletion to Worktrunk's own safety checks.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import sys
from dataclasses import dataclass, asdict
from typing import Iterable

DISPOSABLE_PREFIXES = ("feat/", "fix/", "docs/", "chore/", "test/")
PERMANENT_BRANCHES = {"main", "dev"}
IGNORED_ALLOWLIST = {"node_modules", "target", ".direnv", ".next", ".turbo"}


@dataclass
class Entry:
    path: str
    branch: str | None
    head: str | None
    detached: bool
    current: bool
    clean: bool
    upstream: str | None
    upstream_gone: bool
    proof: str | None
    eligible: bool
    reason: str


def run(
    args: list[str],
    *,
    cwd: str | pathlib.Path | None = None,
    check: bool = False,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=check,
    )


def git(repo: pathlib.Path, *args: str, check: bool = False) -> subprocess.CompletedProcess[str]:
    return run(["git", *args], cwd=repo, check=check)


def repo_root() -> pathlib.Path:
    cp = run(["git", "rev-parse", "--show-toplevel"], check=True)
    return pathlib.Path(cp.stdout.strip()).resolve()


def common_dir(repo: pathlib.Path) -> pathlib.Path:
    cp = git(repo, "rev-parse", "--git-common-dir", check=True)
    p = pathlib.Path(cp.stdout.strip())
    if not p.is_absolute():
        p = repo / p
    return p.resolve()


def fetch_origin_main(repo: pathlib.Path) -> None:
    git(repo, "fetch", "--prune", "origin", "main", check=True)


def parse_worktrees(repo: pathlib.Path) -> list[dict[str, object]]:
    cp = git(repo, "worktree", "list", "--porcelain", "-z", check=True)
    records: list[dict[str, object]] = []
    current: dict[str, object] = {}
    for raw in cp.stdout.split("\0"):
        if raw == "":
            if current:
                records.append(current)
                current = {}
            continue
        if raw.startswith("worktree "):
            current["path"] = raw[len("worktree ") :]
        elif raw.startswith("HEAD "):
            current["head"] = raw[len("HEAD ") :]
        elif raw.startswith("branch "):
            ref = raw[len("branch ") :]
            current["branch_ref"] = ref
            current["branch"] = ref.removeprefix("refs/heads/")
        elif raw == "detached":
            current["detached"] = True
    return records


def status_clean(path: pathlib.Path) -> tuple[bool, str | None]:
    cp = run(["git", "-C", str(path), "status", "--porcelain=v1", "--untracked-files=all"])
    if cp.returncode != 0:
        return False, "status failed"
    if cp.stdout.strip():
        return False, "working tree has staged/modified/deleted/renamed/untracked files"
    ignored = run(["git", "-C", str(path), "status", "--porcelain=v1", "--ignored=matching"])
    if ignored.returncode != 0:
        return False, "ignored-file scan failed"
    blocked: list[str] = []
    for line in ignored.stdout.splitlines():
        if not line.startswith("!! "):
            continue
        rel = line[3:].strip().rstrip("/")
        first = rel.split("/", 1)[0]
        if first not in IGNORED_ALLOWLIST:
            blocked.append(rel)
    if blocked:
        sample = ", ".join(blocked[:3])
        return False, f"ignored files outside cache allowlist would be deleted: {sample}"
    return True, None


def upstream_state(repo: pathlib.Path, branch: str) -> tuple[str | None, bool]:
    cp = git(repo, "rev-parse", "--abbrev-ref", f"{branch}@{{upstream}}")
    if cp.returncode != 0:
        return None, True
    upstream = cp.stdout.strip()
    exists = git(repo, "rev-parse", "--verify", "--quiet", upstream).returncode == 0
    return upstream, not exists


def branch_proof(repo: pathlib.Path, branch: str) -> tuple[str | None, str | None]:
    if git(repo, "merge-base", "--is-ancestor", branch, "refs/remotes/origin/main").returncode == 0:
        return "ancestor-of-origin-main", None
    cherry = git(repo, "cherry", "refs/remotes/origin/main", branch)
    if cherry.returncode != 0:
        return None, "safety proof failed"
    plus = [line for line in cherry.stdout.splitlines() if line.startswith("+")]
    if not plus:
        return "patch-equivalent-to-origin-main", None
    return None, "branch adds changes not present on origin/main"


def classify(repo: pathlib.Path) -> list[Entry]:
    root_common = common_dir(repo)
    cwd = pathlib.Path.cwd().resolve()
    out: list[Entry] = []
    for item in parse_worktrees(repo):
        raw_path = str(item.get("path", ""))
        path = pathlib.Path(raw_path).resolve()
        branch = item.get("branch")
        branch = branch if isinstance(branch, str) else None
        detached = bool(item.get("detached", False)) or not branch
        current = path == cwd or cwd.is_relative_to(path)
        head = item.get("head") if isinstance(item.get("head"), str) else None
        clean = False
        upstream = None
        upstream_gone = False
        proof = None

        def entry(reason: str, eligible: bool = False) -> Entry:
            return Entry(raw_path, branch, head, detached, current, clean, upstream, upstream_gone, proof, eligible, reason)

        if not path.exists():
            out.append(entry("worktree path is missing; manual review required"))
            continue
        if path in {pathlib.Path("/"), pathlib.Path.home(), repo.resolve()}:
            out.append(entry("path is protected; skipped"))
            continue
        cd = run(["git", "-C", str(path), "rev-parse", "--git-common-dir"])
        if cd.returncode != 0:
            out.append(entry("cannot verify git common-dir"))
            continue
        listed_common = pathlib.Path(cd.stdout.strip())
        if not listed_common.is_absolute():
            listed_common = path / listed_common
        if listed_common.resolve() != root_common:
            out.append(entry("git common-dir mismatch; manual review required"))
            continue
        if current:
            out.append(entry("current worktree is never swept"))
            continue
        if detached:
            out.append(entry("detached worktree requires explicit path cleanup"))
            continue
        assert branch is not None
        sym = run(["git", "-C", str(path), "symbolic-ref", "--short", "HEAD"])
        if sym.returncode != 0 or sym.stdout.strip() != branch:
            out.append(entry("branch/path mismatch; manual review required"))
            continue
        if branch in PERMANENT_BRANCHES:
            out.append(entry("permanent branch is excluded"))
            continue
        if branch.startswith(("release/", "hotfix/")):
            out.append(entry("release/hotfix branches require explicit lifecycle cleanup"))
            continue
        if not branch.startswith(DISPOSABLE_PREFIXES):
            out.append(entry("branch prefix is not in the disposable cleanup allowlist"))
            continue
        clean, dirty_reason = status_clean(path)
        if not clean:
            out.append(entry(dirty_reason or "working tree is not clean"))
            continue
        upstream, upstream_gone = upstream_state(repo, branch)
        proof, proof_reason = branch_proof(repo, branch)
        if proof is None:
            out.append(entry(proof_reason or "no merge/equivalence proof"))
            continue
        remote_note = "upstream gone" if upstream_gone else "branch adds no changes to origin/main"
        out.append(entry(f"eligible: {proof}; {remote_note}", True))
    return out


def print_table(entries: Iterable[Entry]) -> None:
    for e in entries:
        status = "ELIGIBLE" if e.eligible else "skip"
        branch = e.branch or "(detached)"
        print(f"{status:8} {branch:45} {e.path} — {e.reason}")


def apply(repo: pathlib.Path, entries: list[Entry], branches: list[str], *, test_confirmed: bool) -> int:
    by_branch = {e.branch: e for e in entries if e.branch}
    rc = 0
    wt_bin = os.environ.get("WT_BIN", "wt")
    for branch in branches:
        e = by_branch.get(branch)
        if e is None:
            print(f"ERROR {branch}: no such listed worktree branch", file=sys.stderr)
            rc = 1
            continue
        if not e.eligible:
            print(f"ERROR {branch}: not eligible — {e.reason}", file=sys.stderr)
            rc = 1
            continue
        path = pathlib.Path(e.path)
        clean, dirty_reason = status_clean(path)
        if not clean:
            print(f"ERROR {branch}: no longer clean — {dirty_reason}", file=sys.stderr)
            rc = 1
            continue
        proof, proof_reason = branch_proof(repo, branch)
        if proof is None:
            print(f"ERROR {branch}: no longer safe — {proof_reason}", file=sys.stderr)
            rc = 1
            continue
        if not test_confirmed:
            answer = input(f"Type '{branch}' to remove {e.path}: ")
            if answer != branch:
                print(f"SKIP {branch}: confirmation did not match", file=sys.stderr)
                rc = 1
                continue
        cmd = [wt_bin, "remove", "--foreground", "--format", "json", branch]
        cp = run(cmd, cwd=repo)
        if cp.stdout:
            print(cp.stdout, end="")
        if cp.stderr:
            print(cp.stderr, file=sys.stderr, end="")
        if cp.returncode != 0:
            rc = cp.returncode
    return rc


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Safely list/apply Worktrunk cleanup candidates")
    parser.add_argument("branches", nargs="*", help="Explicit branches to remove in --apply mode")
    parser.add_argument("--apply", action="store_true", help="Remove explicitly named eligible branches")
    parser.add_argument("--dry-run", action="store_true", help="List candidates only (default)")
    parser.add_argument("--json", action="store_true", help="Print classification as JSON")
    parser.add_argument("--confirm-for-test", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    repo = repo_root()
    try:
        fetch_origin_main(repo)
    except subprocess.CalledProcessError as err:
        print(f"ERROR: failed to fetch/prune origin main: {err.stderr or err}", file=sys.stderr)
        return 2
    entries = classify(repo)
    if args.json:
        print(json.dumps([asdict(e) for e in entries], indent=2, sort_keys=True))
    else:
        print_table(entries)
    if args.apply and args.dry_run:
        print("ERROR: --apply and --dry-run cannot be combined", file=sys.stderr)
        return 2
    if args.apply:
        if not args.branches:
            print("ERROR: --apply requires explicit branch names", file=sys.stderr)
            return 2
        return apply(repo, entries, args.branches, test_confirmed=args.confirm_for_test)
    if args.branches:
        print("ERROR: branch arguments are only valid with --apply", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
