#!/usr/bin/env python3
"""Tests for CCTX eval scoring: recovery cost and T06/T07 structural partial-brief.

Not a live §12.7 session. Not a product compiler.
"""

from __future__ import annotations

import unittest

from recovery_cost import UNMEASURED, score_recovery_cost, unmeasured_recovery_cost
from score_fixtures import (
    STRUCTURAL_PARTIAL_BRIEF,
    CORPUS,
    FIXTURES,
    off_allowlist_gold_in_v3_handles,
    score_one,
)


class RecoveryCostTests(unittest.TestCase):
    def test_no_events_is_unmeasured_not_zero(self) -> None:
        result = score_recovery_cost({"task": "T01", "variant": "V1"})
        self.assertEqual(result["status"], "unmeasured")
        self.assertEqual(result["tokens_after_wrong_path"], UNMEASURED)
        self.assertEqual(result["time_after_wrong_path_ms"], UNMEASURED)
        self.assertEqual(result["re_reads_after_wrong_path"], UNMEASURED)
        self.assertNotEqual(result["tokens_after_wrong_path"], 0)
        self.assertIn("unmeasured", result["reason"].lower())

    def test_explicit_unmeasured_honoured(self) -> None:
        result = score_recovery_cost(
            {
                "task": "T01",
                "variant": "V1",
                "recovery_cost_status": "unmeasured",
                "reason": "Harness did not emit per-event tokens",
                "events": [{"t_ms": 10, "kind": "read", "path": "README.md", "tokens": 3}],
            }
        )
        self.assertEqual(result["status"], "unmeasured")
        self.assertEqual(result["tokens_after_wrong_path"], UNMEASURED)
        self.assertIn("Harness did not emit", result["reason"])

    def test_no_wrong_path_is_none_not_unmeasured(self) -> None:
        result = score_recovery_cost(
            {
                "task": "T01",
                "variant": "V1",
                "gold_paths": ["docs/guides/ai-context-delivery.md"],
                "events": [
                    {
                        "t_ms": 100,
                        "kind": "read",
                        "path": "docs/guides/ai-context-delivery.md",
                        "tokens": 40,
                    }
                ],
            }
        )
        self.assertEqual(result["status"], "none")
        self.assertEqual(result["tokens_after_wrong_path"], 0)
        self.assertEqual(result["time_after_wrong_path_ms"], 0)
        self.assertEqual(result["re_reads_after_wrong_path"], 0)

    def test_wrong_path_then_gold_is_measured(self) -> None:
        result = score_recovery_cost(
            {
                "task": "T05",
                "variant": "V1",
                "gold_paths": ["crates/anvil-graph-cache/src/call_graph.rs"],
                "events": [
                    {
                        "t_ms": 50,
                        "kind": "read",
                        "path": "docs/guides/ai-context-delivery.md",
                        "tokens": 20,
                    },
                    {
                        "t_ms": 80,
                        "kind": "read",
                        "path": "docs/guides/ai-context-delivery.md",
                        "tokens": 10,
                    },
                    {
                        "t_ms": 200,
                        "kind": "read",
                        "path": "crates/anvil-graph-cache/src/call_graph.rs",
                        "tokens": 30,
                    },
                ],
            }
        )
        self.assertEqual(result["status"], "measured")
        self.assertEqual(result["tokens_after_wrong_path"], 20 + 10 + 30)
        self.assertEqual(result["time_after_wrong_path_ms"], 200 - 50)
        self.assertEqual(result["re_reads_after_wrong_path"], 1)
        self.assertEqual(len(result["wrong_path_events"]), 1)
        self.assertEqual(
            result["wrong_path_events"][0]["path"],
            "docs/guides/ai-context-delivery.md",
        )

    def test_events_without_tokens_or_time_stay_unmeasured(self) -> None:
        result = score_recovery_cost(
            {
                "task": "T01",
                "variant": "V1",
                "gold_paths": ["docs/guides/ai-context-delivery.md"],
                "events": [{"kind": "read", "path": "README.md"}],
            }
        )
        self.assertEqual(result["status"], "unmeasured")
        self.assertEqual(result["tokens_after_wrong_path"], UNMEASURED)


class StructuralPartialBriefTests(unittest.TestCase):
    def test_t06_and_t07_are_declared_structural_partial(self) -> None:
        self.assertIn("T06", STRUCTURAL_PARTIAL_BRIEF)
        self.assertIn("T07", STRUCTURAL_PARTIAL_BRIEF)
        self.assertNotIn("T01", STRUCTURAL_PARTIAL_BRIEF)

    def test_t06_fixture_labels_structural_partial_brief(self) -> None:
        text = (FIXTURES / "T06.md").read_text(encoding="utf-8")
        self.assertIn("structural-partial-brief", text)
        scored = score_one(FIXTURES / "T06.md")
        self.assertEqual(scored["structural_partial_brief"]["status"], "structural-partial-brief")
        self.assertTrue(scored["contract_ok"] or not CORPUS.is_dir())
        self.assertEqual(scored["recovery_cost"]["status"], "unmeasured")

    def test_t07_fixture_labels_structural_partial_brief(self) -> None:
        text = (FIXTURES / "T07.md").read_text(encoding="utf-8")
        self.assertIn("structural-partial-brief", text)
        scored = score_one(FIXTURES / "T07.md")
        self.assertEqual(scored["structural_partial_brief"]["status"], "structural-partial-brief")
        if CORPUS.is_dir():
            self.assertTrue(scored["contract_ok"])

    def test_off_allowlist_gold_must_not_be_v3_handles(self) -> None:
        t06 = score_one(FIXTURES / "T06.md")
        t07 = score_one(FIXTURES / "T07.md")
        self.assertEqual(off_allowlist_gold_in_v3_handles(t06), [])
        self.assertEqual(off_allowlist_gold_in_v3_handles(t07), [])
        self.assertNotIn(".markdownlintignore", t06["v3_handles"])
        self.assertNotIn("crates/anvil-cli/src/commands/start.rs", t07["v3_handles"])

    def test_ordinary_task_is_not_structural_partial(self) -> None:
        scored = score_one(FIXTURES / "T01.md")
        self.assertIsNone(scored["structural_partial_brief"])
        if CORPUS.is_dir():
            self.assertTrue(scored["contract_ok"])


class AuthorshipNotRecallTests(unittest.TestCase):
    def test_contract_ok_is_labelled_authorship_not_recall(self) -> None:
        from score_fixtures import CONTRACT_OK_MEANING

        self.assertIn("authorship", CONTRACT_OK_MEANING.lower())
        self.assertIn("not", CONTRACT_OK_MEANING.lower())
        self.assertIn("recall", CONTRACT_OK_MEANING.lower())

    def test_unmeasured_helper_never_returns_zero(self) -> None:
        row = unmeasured_recovery_cost("fixture contract score")
        self.assertEqual(row["tokens_after_wrong_path"], UNMEASURED)
        self.assertNotEqual(row["tokens_after_wrong_path"], 0)


class LiveScaffoldSmokeTests(unittest.TestCase):
    def test_score_live_session_file(self) -> None:
        from score_live_session import score_live_session

        session = {
            "task": "T01",
            "variant": "V1",
            "corpus_revision": "23457dc6d2bf379791d587cf2dfdb5046ce51fc0",
            "gold_paths": ["docs/guides/ai-context-delivery.md"],
            "events": [
                {"t_ms": 10, "kind": "search", "path": "src/unrelated.rs", "tokens": 8},
                {
                    "t_ms": 40,
                    "kind": "read",
                    "path": "docs/guides/ai-context-delivery.md",
                    "tokens": 12,
                },
            ],
        }
        scored = score_live_session(session)
        self.assertEqual(scored["recovery_cost"]["status"], "measured")
        self.assertEqual(scored["recovery_cost"]["tokens_after_wrong_path"], 20)
        self.assertEqual(scored["variant"], "V1")
        self.assertNotIn("live_v3_ran", scored)


if __name__ == "__main__":
    raise SystemExit(unittest.main())
