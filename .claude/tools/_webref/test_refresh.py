#!/usr/bin/env python3
"""Unit tests for refresh snapshot path handling."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref.commands.refresh import _unique_snapshot_path  # noqa: E402


class RefreshPathTests(unittest.TestCase):
    def test_unique_snapshot_path_avoids_existing_collision(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            first = _unique_snapshot_path(root, "dom")
            first.write_text("{}\n", encoding="utf-8")
            second = _unique_snapshot_path(root, "dom")
            self.assertNotEqual(first, second)
            self.assertFalse(second.exists())


if __name__ == "__main__":
    unittest.main()
