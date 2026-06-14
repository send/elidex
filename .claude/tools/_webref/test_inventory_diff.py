#!/usr/bin/env python3
"""Unit tests for semantic inventory diffing."""
from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref.diff import diff_inventories  # noqa: E402
from _webref.inventory import _tc39_ref_id  # noqa: E402


def _snapshot(items):
    return {
        "schemaVersion": 1,
        "shortname": "html",
        "family": "webref",
        "generatedAt": "2026-01-01T00:00:00Z",
        "itemCount": len(items),
        "items": items,
    }


class DiffInventoryTests(unittest.TestCase):
    def test_added_and_removed_are_key_based(self):
        old = _snapshot([
            {
                "key": "heading:a",
                "kind": "heading",
                "id": "a",
                "number": "1",
                "title": "A",
            },
        ])
        new = _snapshot([
            {
                "key": "heading:b",
                "kind": "heading",
                "id": "b",
                "number": "2",
                "title": "B",
            },
        ])
        result = diff_inventories(old, new)
        self.assertEqual(result["counts"]["added"], 1)
        self.assertEqual(result["counts"]["removed"], 1)

    def test_heading_number_and_title_get_specific_categories(self):
        old = _snapshot([
            {
                "key": "heading:a",
                "kind": "heading",
                "id": "a",
                "number": "1",
                "title": "A",
            },
        ])
        new = _snapshot([
            {
                "key": "heading:a",
                "kind": "heading",
                "id": "a",
                "number": "1.1",
                "title": "A prime",
            },
        ])
        result = diff_inventories(old, new)
        self.assertEqual(result["counts"]["renumbered"], 1)
        self.assertEqual(result["counts"]["retitled"], 1)
        self.assertEqual(result["counts"]["changed"], 0)

    def test_non_heading_field_changes_are_changed(self):
        old = _snapshot([
            {"key": "dfn:a", "kind": "dfn", "id": "a", "linkingText": ["old"]},
        ])
        new = _snapshot([
            {"key": "dfn:a", "kind": "dfn", "id": "a", "linkingText": ["new"]},
        ])
        result = diff_inventories(old, new)
        self.assertEqual(result["counts"]["changed"], 1)

    def test_href_only_move_is_not_changed(self):
        old = _snapshot([
            {"key": "dfn:a", "kind": "dfn", "id": "a", "href": "https://old#a"},
        ])
        new = _snapshot([
            {"key": "dfn:a", "kind": "dfn", "id": "a", "href": "https://new#a"},
        ])
        result = diff_inventories(old, new)
        self.assertEqual(result["counts"]["moved"], 1)
        self.assertEqual(result["counts"]["changed"], 0)


class Tc39InventoryTests(unittest.TestCase):
    def test_clause_ao_uses_id_when_ref_id_is_absent(self):
        self.assertEqual(
            _tc39_ref_id({"type": "clause", "id": "sec-test", "aoid": "Test"}),
            "sec-test",
        )

    def test_non_clause_ao_prefers_ref_id(self):
        self.assertEqual(
            _tc39_ref_id({"id": "op-test", "refId": "sec-test", "aoid": "Test"}),
            "sec-test",
        )


if __name__ == "__main__":
    unittest.main()
