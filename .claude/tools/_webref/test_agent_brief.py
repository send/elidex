#!/usr/bin/env python3
"""Unit tests for agent-brief repository impact scanning."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref.commands.agent_brief import _changed_entries, _scan_impacts  # noqa: E402
from _webref.commands.agent_brief import _print_markdown  # noqa: E402
from _webref.diff import diff_inventories  # noqa: E402


def _snapshot(items):
    return {
        "schemaVersion": 1,
        "shortname": "html",
        "family": "webref",
        "itemCount": len(items),
        "items": items,
    }


class AgentBriefScanTests(unittest.TestCase):
    def test_scan_matches_anchor_and_section_number(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            doc = root / "docs" / "note.md"
            doc.parent.mkdir()
            doc.write_text(
                "HTML §4.8.5 cites #the-iframe-element.\n",
                encoding="utf-8",
            )
            old = _snapshot([
                {
                    "key": "heading:the-iframe-element",
                    "kind": "heading",
                    "id": "the-iframe-element",
                    "number": "4.8.5",
                    "title": "The iframe element",
                },
            ])
            new = _snapshot([
                {
                    "key": "heading:the-iframe-element",
                    "kind": "heading",
                    "id": "the-iframe-element",
                    "number": "4.8.6",
                    "title": "The iframe element",
                },
            ])
            diff = diff_inventories(old, new)
            impacts = _scan_impacts(root.resolve(), ["docs"], diff)
            self.assertEqual(len(impacts), 1)
            self.assertEqual(impacts[0]["matches"][0]["path"], "docs/note.md")
            self.assertEqual(impacts[0]["matches"][0]["line"], 1)

    def test_scan_matches_old_number_after_renumbering(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            doc = root / "docs" / "note.md"
            doc.parent.mkdir()
            doc.write_text("HTML §4.8.5 changed upstream.\n", encoding="utf-8")
            old = _snapshot([
                {
                    "key": "heading:the-iframe-element",
                    "kind": "heading",
                    "id": "the-iframe-element",
                    "number": "4.8.5",
                    "title": "The iframe element",
                },
            ])
            new = _snapshot([
                {
                    "key": "heading:the-iframe-element",
                    "kind": "heading",
                    "id": "the-iframe-element",
                    "number": "4.8.6",
                    "title": "The iframe element",
                },
            ])
            diff = diff_inventories(old, new)
            impacts = _scan_impacts(root.resolve(), ["docs"], diff)
            self.assertEqual(len(impacts), 1)
            self.assertIn("§4.8.5", impacts[0]["matches"][0]["needles"])

    def test_scan_keeps_all_matches_for_json_output(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            doc = root / "docs" / "note.md"
            doc.parent.mkdir()
            doc.write_text("#the-iframe-element\n" * 55, encoding="utf-8")
            old = _snapshot([])
            new = _snapshot([
                {
                    "key": "heading:the-iframe-element",
                    "kind": "heading",
                    "id": "the-iframe-element",
                    "number": "4.8.5",
                    "title": "The iframe element",
                },
            ])
            diff = diff_inventories(old, new)
            impacts = _scan_impacts(root.resolve(), ["docs"], diff)
            self.assertEqual(len(impacts[0]["matches"]), 55)
            self.assertTrue(impacts[0]["truncated"])

    def test_changed_entries_deduplicates_combined_heading_changes(self):
        old = _snapshot([
            {
                "key": "heading:the-iframe-element",
                "kind": "heading",
                "id": "the-iframe-element",
                "number": "4.8.5",
                "title": "The iframe element",
            },
        ])
        new = _snapshot([
            {
                "key": "heading:the-iframe-element",
                "kind": "heading",
                "id": "the-iframe-element",
                "number": "4.8.6",
                "title": "The iframe element updated",
            },
        ])
        diff = diff_inventories(old, new)
        self.assertEqual(diff["counts"]["renumbered"], 1)
        self.assertEqual(diff["counts"]["retitled"], 1)
        entries = _changed_entries(diff)
        self.assertEqual(len(entries), 1)

    def test_markdown_reports_medium_size_omissions(self):
        matches = [
            {"path": "docs/note.md", "line": i, "needles": ["#x"]}
            for i in range(1, 13)
        ]
        brief = {
            "old": {"shortname": "html"},
            "new": {"shortname": "html"},
            "counts": {
                "added": 1,
                "removed": 0,
                "renumbered": 0,
                "retitled": 0,
                "moved": 0,
                "changed": 0,
            },
            "impacts": [{
                "key": "heading:x",
                "summary": "§1 X",
                "matches": matches,
                "truncated": False,
            }],
        }
        lines: list[str] = []
        with patch("builtins.print", lambda *args, **_kwargs: lines.append(" ".join(map(str, args)))):
            _print_markdown(brief)
        self.assertTrue(any("docs/note.md:11" in line for line in lines))
        self.assertFalse(any("more matches omitted" in line for line in lines))


if __name__ == "__main__":
    unittest.main()
