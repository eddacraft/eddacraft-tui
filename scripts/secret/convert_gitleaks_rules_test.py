#!/usr/bin/env python3
"""Focused tests for the vendored-ruleset converter's structural gates.

The converter is deliberately dumb about regex *semantics* — upstream patterns
are taken verbatim — but it is the only place tier-1 membership is enforced, so
its counting has to be right. An undercount is unsafe in both directions: a rule
with a named credential group plus another group would slip past the
"more than one capture group" rejection, and a rule whose only group is named
would lose its `secret_group` narrowing and report scaffolding as the credential.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from types import ModuleType


def load_subject() -> ModuleType:
    path = Path(__file__).with_name("convert-gitleaks-rules.py")
    spec = importlib.util.spec_from_file_location("convert_gitleaks_rules", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


SUBJECT = load_subject()


class CaptureGroupCountingTests(unittest.TestCase):
    def count(self, pattern: str) -> int:
        return SUBJECT.count_capture_groups(pattern)

    def test_numbered_group_counts(self) -> None:
        self.assertEqual(self.count(r"\b(dop_v1_[a-f0-9]{64})"), 1)

    def test_non_capturing_and_flag_groups_do_not_count(self) -> None:
        self.assertEqual(self.count(r"(?i)(?:foo|bar)\b(tok_[a-z]{8})"), 1)

    def test_python_style_named_group_counts(self) -> None:
        self.assertEqual(self.count(r"\b(?P<secret>tok_[a-z]{8})"), 1)

    def test_rust_style_named_group_counts(self) -> None:
        self.assertEqual(self.count(r"\b(?<secret>tok_[a-z]{8})"), 1)

    def test_lookbehind_does_not_count(self) -> None:
        # `(?<=` and `(?<!` open lookbehind, not a named group. Counting them
        # would overcount and reject a legitimate single-group rule.
        self.assertEqual(self.count(r"(?<=x)(tok_[a-z]{8})"), 1)
        self.assertEqual(self.count(r"(?<!x)(tok_[a-z]{8})"), 1)

    def test_escaped_and_class_parens_do_not_count(self) -> None:
        self.assertEqual(self.count(r"\(tok_[a-z(]{8}\)"), 0)

    def test_a_named_group_beside_another_group_is_still_two(self) -> None:
        # The case the old counter got wrong: it read this as one group and
        # would have vendored the rule with a `secret_group` pointing at the
        # wrong span.
        self.assertEqual(self.count(r"(?P<a>foo)(tok_[a-z]{8})"), 2)


class LiteralPrefixTests(unittest.TestCase):
    def test_prefix_found_inside_a_numbered_group(self) -> None:
        self.assertEqual(
            SUBJECT.literal_prefix(r"(?i)\b(dop_v1_[a-f0-9]{64})"), "dop_v1_"
        )

    def test_prefix_found_inside_a_named_group(self) -> None:
        self.assertEqual(
            SUBJECT.literal_prefix(r"(?i)\b(?P<secret>dop_v1_[a-f0-9]{64})"), "dop_v1_"
        )


if __name__ == "__main__":
    unittest.main()
