"""Minimal python fixture for the editor-coexistence harness."""

from __future__ import annotations


def greet(name: str) -> str:
    """Return a greeting that pyright + ruff agree is clean."""
    return f"hello, {name}"
