#!/usr/bin/env python3
"""Build the TP/FP classification worksheet from a run.sh dump.

For each repo in the requested group it joins every `anvil check` finding back
to its source line (so the reviewer judges true-positive vs false-positive
without re-opening each file by hand) and writes a markdown worksheet per group
under the output dir. The `Verdict` column is left blank for the human/agent
pass; once filled, `--score` recomputes the genuine-FP rate against the
council §16.5 #9 bar.

Usage:
  python3 classify.py <rust|langts|all>          # build/refresh worksheets
  python3 classify.py <rust|langts|all> --score  # recompute FP rate from verdicts

Env: EXT_FP_WORK (clone cache, default /tmp/anvil-ext-fp), EXT_FP_OUT (results).
"""
import json
import os
import sys
import pathlib
import collections

HERE = pathlib.Path(__file__).resolve().parent
with open(HERE / "corpus.json", "r") as fh:
    CORPUS = json.load(fh)
WORK = pathlib.Path(os.environ.get("EXT_FP_WORK", "/tmp/anvil-ext-fp"))
OUT = pathlib.Path(os.environ.get("EXT_FP_OUT", WORK / "out"))


def source_line(repo, relfile, lineno):
    p = WORK / repo / relfile
    try:
        with open(p, "r", errors="replace") as fh:
            for i, line in enumerate(fh, 1):
                if i == lineno:
                    return line.rstrip("\n")
    except OSError:
        return "<source unavailable>"
    return "<line out of range>"


def load_warnings(path):
    if not path.exists():
        return None
    try:
        with open(path, "r") as fh:
            return json.load(fh).get("warnings", [])
    except json.JSONDecodeError:
        return None


def build(group):
    g = CORPUS["groups"][group]
    optin_ids = set(g["opt_in_rules"])
    md = [f"# {group} external-FP worksheet\n",
          "Verdict column: `TP` (true positive), `FP` (false positive), or a",
          "noise note. Default-catalogue findings drive the §16.5 #9 rate;",
          "opt-in findings are characterised separately.\n"]
    for repo in g["repos"]:
        name = repo["name"]
        default = load_warnings(OUT / f"{name}.default.json")
        optin = load_warnings(OUT / f"{name}.optin.json")
        if default is None:
            md.append(f"## {name}\n\n_no parseable default dump — run.sh first_\n")
            continue
        # opt-in-only findings = present with --include-opt-in but not by default
        default_keys = {(w["id"], w["file"], w["line"]) for w in default}
        optin_only = [w for w in (optin or [])
                      if (w["id"], w["file"], w["line"]) not in default_keys]
        by = collections.Counter(w["id"] for w in default)
        md.append(f"## {name}  (`{repo['sha'][:12]}`)\n")
        md.append(f"_{repo['why']}_\n")
        md.append(f"Default findings: **{len(default)}** "
                  f"({', '.join(f'{k}×{v}' for k, v in sorted(by.items())) or 'none'})\n")
        md.append("| Verdict | Rule | Location | Source line |")
        md.append("| --- | --- | --- | --- |")
        for w in sorted(default, key=lambda w: (w["id"], w["file"], w["line"])):
            src = source_line(name, w["file"], w["line"]).strip().replace("|", "\\|")
            md.append(f"|  | {w['id']} | `{w['file']}:{w['line']}` | `{src[:90]}` |")
        if optin_only:
            md.append(f"\n### {name} — opt-in only (not in default rate)\n")
            md.append("| Verdict | Rule | Location | Source line |")
            md.append("| --- | --- | --- | --- |")
            for w in sorted(optin_only, key=lambda w: (w["id"], w["file"], w["line"])):
                src = source_line(name, w["file"], w["line"]).strip().replace("|", "\\|")
                md.append(f"|  | {w['id']} | `{w['file']}:{w['line']}` | `{src[:90]}` |")
        md.append("")
    dest = OUT / f"{group}.worksheet.md"
    dest.write_text("\n".join(md) + "\n")
    print(f"wrote {dest}")


def score(group):
    """Recompute the FP rate by reading filled-in worksheets."""
    path = OUT / f"{group}.worksheet.md"
    if not path.exists():
        print(f"no worksheet at {path} — build it first"); return
    in_optin = False
    total = fp = 0
    for line in path.read_text().splitlines():
        if line.startswith("### ") and "opt-in" in line:
            in_optin = True; continue
        if line.startswith("## "):
            in_optin = False; continue
        if not line.startswith("|") or line.startswith("| ---") or line.startswith("| Verdict"):
            continue
        verdict = line.split("|")[1].strip().upper()
        if in_optin or not verdict:
            continue
        total += 1
        if verdict.startswith("FP"):
            fp += 1
    rate = (fp / total * 100) if total else 0.0
    print(f"{group}: default-catalogue findings classified={total} "
          f"FP={fp} -> genuine-FP rate {rate:.1f}%  (§16.5 #9 bar: RSTLAN-008 = 0%)")


def main():
    if len(sys.argv) < 2:
        print(__doc__); sys.exit(2)
    group = sys.argv[1]
    do_score = "--score" in sys.argv[2:]
    groups = ["rust", "langts"] if group == "all" else [group]
    for grp in groups:
        (score if do_score else build)(grp)


if __name__ == "__main__":
    main()
