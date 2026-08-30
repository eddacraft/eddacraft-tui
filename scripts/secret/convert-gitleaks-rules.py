#!/usr/bin/env python3
"""ADR-136 §5 — convert pinned gitleaks rules into Anvil's vendored tier-1 data.

Called by `scripts/secret/refresh-gitleaks-ruleset.sh`; not a user-facing entry
point. Reads the upstream `gitleaks.toml` that the shell script has already
fetched *and digest-verified*, selects the rule ids named in `tier1-rules.txt`,
and emits `tier1.json`.

Conversion is deliberately dumb, because a clever conversion is a place for
upstream semantics to change without anybody noticing:

* the upstream `regex` is copied **verbatim** — never rewritten, widened, or
  "simplified";
* the reported span is narrowed to the upstream capture group when the rule has
  exactly one, so a finding covers the credential and not the trailing quote or
  delimiter gitleaks matches around it (`secret_group`);
* a rule with more than one capture group is an error, not a guess;
* a missing id is an error — a silently dropped rule is a silent loss of
  detection.

Output is sorted by rule id and written with a stable separator so `--check`
diffs are meaningful.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

RULE_ID = re.compile(r"^id = \"([^\"]+)\"", re.M)
RULE_REGEX = re.compile(r"^regex = '''(.*?)'''\s*$", re.M | re.S)
RULE_DESC = re.compile(r"^description = \"(.*?)\"\s*$", re.M)

# A literal, provider-specific prefix is the whole tier-1 criterion: the leading
# run of non-metacharacters a match is pinned to. `.` is included because
# upstream leaves some dots unescaped (`hooks.slack.com`); it is a wildcard
# there, but the surrounding literal run still does the anchoring.
LITERAL_RUN = re.compile(r"[A-Za-z0-9_./:=-]+")

MIN_PREFIX_LEN = 4


class ConversionError(RuntimeError):
    pass


def parse_rules(config_text: str) -> dict[str, dict[str, str]]:
    """Split the upstream config into `id -> {regex, description}`.

    Parsed with a line-anchored scan rather than `tomllib` because upstream
    regexes are `'''…'''` literals whose contents are not our business — we want
    the exact source bytes, not a value round-tripped through a TOML decoder.
    """
    rules: dict[str, dict[str, str]] = {}
    for block in config_text.split("[[rules]]")[1:]:
        ident = RULE_ID.search(block)
        pattern = RULE_REGEX.search(block)
        if not ident or not pattern:
            continue
        description = RULE_DESC.search(block)
        rules[ident.group(1)] = {
            "regex": pattern.group(1),
            "description": description.group(1) if description else "",
        }
    return rules


# `(?P<name>…)` and `(?<name>…)` are *capturing* groups in both Python and the
# Rust `regex` crate. `(?<=…)` and `(?<!…)` are lookbehind and are not. Treating
# a named group as non-capturing would undercount, which is unsafe in both
# directions here: a rule with a named credential group plus another group would
# slip past the >1 rejection, and a rule whose only group is named would lose
# its `secret_group` narrowing and report the scaffolding as the credential.
NAMED_GROUP_OPEN = re.compile(r"\(\?P?<(?![=!])")


def opens_capture_group(pattern: str, index: int) -> bool:
    """Whether the `(` at `index` opens a capturing group."""
    if not pattern.startswith("(?", index):
        return True
    return NAMED_GROUP_OPEN.match(pattern, index) is not None


def count_capture_groups(pattern: str) -> int:
    """Count capturing groups — numbered and named — ignoring escapes, classes,
    and non-capturing `(?…)` constructs."""
    count = 0
    index = 0
    in_class = False
    while index < len(pattern):
        char = pattern[index]
        if char == "\\":
            index += 2
            continue
        if in_class:
            if char == "]":
                in_class = False
            index += 1
            continue
        if char == "[":
            in_class = True
        elif char == "(" and opens_capture_group(pattern, index):
            count += 1
        index += 1
    return count


LEADING_NOISE = re.compile(r"\A(?:\(\?i\)|\\b|\^|\(\?:[^()]*\)[?*]?)+")


def literal_prefix(pattern: str) -> str:
    """The literal provider prefix a match is anchored on, or `""`.

    Looked for at the start of the regex, and — for rules gitleaks wraps in a
    provider-keyword gate — at the start of the capture group. A keyword gate
    makes a rule *more* precise, not less, so it does not disqualify the rule;
    what matters is that the credential itself carries the prefix.
    """
    candidates = [pattern]
    for index, char in enumerate(pattern):
        if char == "(" and opens_capture_group(pattern, index):
            # Skip past the group opener: `(` for a numbered group, the whole
            # `(?P<name>` / `(?<name>` for a named one.
            named = NAMED_GROUP_OPEN.match(pattern, index)
            if named:
                close = pattern.find(">", named.end())
                if close == -1:
                    break
                candidates.append(pattern[close + 1 :])
            else:
                candidates.append(pattern[index + 1 :])
            break
    best = ""
    for candidate in candidates:
        stripped = LEADING_NOISE.sub("", candidate)
        found = LITERAL_RUN.match(stripped)
        if found and len(found.group(0)) > len(best):
            best = found.group(0)
    return best


def read_membership(path: Path) -> list[str]:
    ids: list[str] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            ids.append(line)
    duplicates = {name for name in ids if ids.count(name) > 1}
    if duplicates:
        raise ConversionError(f"duplicate rule id(s) in membership list: {sorted(duplicates)}")
    return ids


def convert(upstream: dict[str, dict[str, str]], wanted: list[str]) -> list[dict[str, object]]:
    converted: list[dict[str, object]] = []
    for rule_id in sorted(wanted):
        rule = upstream.get(rule_id)
        if rule is None:
            raise ConversionError(
                f"rule id {rule_id!r} is named in tier1-rules.txt but is absent from the "
                f"pinned upstream config; upstream renamed or removed it — fix the "
                f"membership list deliberately rather than dropping detection silently"
            )
        pattern = rule["regex"]
        groups = count_capture_groups(pattern)
        if groups > 1:
            raise ConversionError(
                f"rule {rule_id!r} has {groups} capture groups; the reported span would be a "
                f"guess. Review it by hand and either narrow it upstream or drop it from tier 1"
            )
        prefix = literal_prefix(pattern)
        if len(prefix) < MIN_PREFIX_LEN:
            raise ConversionError(
                f"rule {rule_id!r} has no literal provider prefix of at least "
                f"{MIN_PREFIX_LEN} characters (found {prefix!r}); it is not prefix-anchored "
                f"and does not belong in tier 1"
            )
        converted.append(
            {
                "id": rule_id,
                "name": rule_id,
                "description": rule["description"],
                "pattern": pattern,
                "secret_group": groups if groups == 1 else None,
                "prefix": prefix,
            }
        )
    return converted


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--membership", required=True, type=Path)
    parser.add_argument("--pin", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()

    pin = tomllib.loads(args.pin.read_text(encoding="utf-8"))
    upstream = parse_rules(args.source.read_text(encoding="utf-8"))
    try:
        rules = convert(upstream, read_membership(args.membership))
    except ConversionError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1

    document = {
        "schema": 1,
        "generated_by": "scripts/secret/refresh-gitleaks-ruleset.sh",
        "note": (
            "GENERATED FILE - do not hand-edit. Regenerate with "
            "scripts/secret/refresh-gitleaks-ruleset.sh; verify with --check."
        ),
        "ruleset": "gitleaks",
        "tier": 1,
        "version": pin["tag"],
        "commit": pin["commit"],
        "licence": pin["licence"],
        "rule_count": len(rules),
        "rules": rules,
    }
    args.out.write_text(json.dumps(document, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    print(f"converted {len(rules)} tier-1 rule(s) from {pin['tag']} ({pin['commit'][:12]})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
