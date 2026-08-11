#!/usr/bin/env python3
"""Focused tests for benchmark-history catalogue comparability metadata."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


def load_subject() -> ModuleType:
    path = Path(__file__).with_name("to-history.py")
    spec = importlib.util.spec_from_file_location("to_history", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


SUBJECT = load_subject()


class AntipatternCatalogueMetadataTests(unittest.TestCase):
    def write_registry(
        self,
        root: Path,
        *,
        compiled_at: str,
        title: str,
        ts_pattern: str = "any",
        rust_ast_query: str = "unwrap",
        html_pattern: str = "<script",
    ) -> Path:
        registry = {
            "schema_version": 1,
            "compiled_at": compiled_at,
            "patterns": [
                {
                    "id": "TS-001",
                    "version": 1,
                    "title": title,
                    "enabled": True,
                    "opt_in": False,
                    "targets": ["source"],
                    "detection": {"type": "regex", "pattern": ts_pattern},
                    "file_extensions": [".ts"],
                    "allowlist": [],
                },
                {
                    "id": "RS-001",
                    "version": 1,
                    "enabled": True,
                    "opt_in": True,
                    "targets": ["source"],
                    "detection": {
                        "type": "ast",
                        "ast_query": rust_ast_query,
                    },
                    "file_extensions": [".rs"],
                    "allowlist": [],
                },
                {
                    "id": "ALL-001",
                    "version": 2,
                    "enabled": True,
                    "opt_in": False,
                    "targets": ["source"],
                    "detection": {"type": "regex", "pattern": "TODO"},
                    "file_extensions": [],
                    "allowlist": [],
                },
                {
                    "id": "PR-001",
                    "version": 1,
                    "enabled": True,
                    "opt_in": False,
                    "targets": ["pr-description"],
                    "detection": {"type": "regex", "pattern": "later"},
                    "file_extensions": [],
                    "allowlist": [],
                },
                {
                    "id": "HTML-001",
                    "version": 1,
                    "enabled": True,
                    "opt_in": True,
                    "targets": ["source"],
                    "detection": {"type": "regex", "pattern": html_pattern},
                    "file_extensions": [".html"],
                    "allowlist": [],
                },
            ],
        }
        path = root / "registry.json"
        path.write_text(json.dumps(registry))
        return path

    def test_metadata_counts_default_source_rules_by_language(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            registry = self.write_registry(
                Path(directory), compiled_at="first", title="Original title"
            )
            metadata = SUBJECT.antipattern_catalogue_metadata(registry)

        self.assertEqual(metadata["pattern_count"], 5)
        self.assertEqual(metadata["scanner_pattern_count"], 4)
        self.assertEqual(metadata["enabled_scanner_pattern_count"], 4)
        self.assertEqual(metadata["default_scanner_pattern_count"], 3)
        self.assertEqual(
            metadata["default_source_rules"],
            {"typescript": 2, "rust": 1, "python": 1},
        )
        self.assertRegex(metadata["fingerprint"], r"^sha256:[0-9a-f]{64}$")
        self.assertRegex(
            metadata["enabled_scanner_fingerprint"], r"^sha256:[0-9a-f]{64}$"
        )

    def test_fingerprint_ignores_generated_timestamp_and_prose(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = self.write_registry(
                root, compiled_at="first", title="Original title"
            )
            first_fingerprint = SUBJECT.antipattern_catalogue_metadata(first)[
                "fingerprint"
            ]
            second = self.write_registry(
                root,
                compiled_at="second",
                title="Edited prose only",
                rust_ast_query="panic",
            )
            second_fingerprint = SUBJECT.antipattern_catalogue_metadata(second)[
                "fingerprint"
            ]

        self.assertEqual(first_fingerprint, second_fingerprint)

    def test_opt_in_regex_change_only_changes_enabled_workload_fingerprint(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = self.write_registry(
                root, compiled_at="first", title="Original title"
            )
            first_metadata = SUBJECT.antipattern_catalogue_metadata(first)
            second = self.write_registry(
                root,
                compiled_at="second",
                title="Original title",
                html_pattern="<(?:script|iframe)",
            )
            second_metadata = SUBJECT.antipattern_catalogue_metadata(second)

        self.assertEqual(
            first_metadata["fingerprint"], second_metadata["fingerprint"]
        )
        self.assertNotEqual(
            first_metadata["enabled_scanner_fingerprint"],
            second_metadata["enabled_scanner_fingerprint"],
        )

    def test_fingerprint_changes_with_executed_regex_workload(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = self.write_registry(
                root, compiled_at="first", title="Original title"
            )
            first_fingerprint = SUBJECT.antipattern_catalogue_metadata(first)[
                "fingerprint"
            ]
            second = self.write_registry(
                root,
                compiled_at="second",
                title="Original title",
                ts_pattern="(?:any|unknown)",
            )
            second_fingerprint = SUBJECT.antipattern_catalogue_metadata(second)[
                "fingerprint"
            ]

        self.assertNotEqual(first_fingerprint, second_fingerprint)


if __name__ == "__main__":
    unittest.main()
