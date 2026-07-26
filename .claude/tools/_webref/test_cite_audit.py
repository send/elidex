#!/usr/bin/env python3
"""Unit tests for the `cite-audit` adapter command.

The command owns a citation sweep's *exit criterion*, so its own failure
modes are load-bearing. Three of the four classes below are regressions
for defects found by exercising the tool rather than reading it:

  - `--strict` was a no-op (`cli.main` discards return values)
  - `--prefix` used string matching, so `4.10.2` swept `4.10.2x`
  - every bare `§N.N` was resolved against one spec, reporting
    cross-spec citations and non-spec pointers as drift
"""
from __future__ import annotations

import argparse
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from tempfile import TemporaryDirectory

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref.commands import cite_audit  # noqa: E402
from _webref import spec_labels  # noqa: E402


def _args(**kw):
    base = dict(
        spec="html", root="crates", glob="*.rs", prefix=None,
        summary=True, format="text", strict=False, show_unattributed=False,
    )
    base.update(kw)
    return argparse.Namespace(**base)


class _TreeCase(unittest.TestCase):
    """Base: build a throwaway source tree and run the command over it."""

    def _tree(self, files: dict[str, str]) -> Path:
        td = TemporaryDirectory()
        self.addCleanup(td.cleanup)
        for name, body in files.items():
            p = Path(td.name) / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(body, encoding="utf-8")
        return Path(td.name)

    def _run(self, root: Path, **kw) -> tuple[str, int]:
        """Run the command over `root`, returning (stdout, exit_code)."""
        buf = io.StringIO()
        code = 0
        try:
            with redirect_stdout(buf):
                cite_audit.cmd_cite_audit(_args(root=str(root), **kw))
        except SystemExit as e:  # `--strict` signals via sys.exit
            code = e.code if isinstance(e.code, int) else 1
        return buf.getvalue(), code


class TestSpecAttribution(_TreeCase):
    """Bucket (a) explicit / (b) inherited / (c) unattributed."""

    def test_explicit_label_selects_the_named_spec(self):
        td = self._tree({"a.rs": "/// WHATWG Fetch §2.2.2 safelist\n"})
        out, _ = self._run(td)
        # Attributed to fetch, so an html audit must not count it.
        self.assertIn("attributed to html: 0 distinct sections", out)
        self.assertIn("attributed to another spec (not audited here): 1", out)

    def test_explicit_html_label_is_audited(self):
        td = self._tree({"a.rs": "/// HTML §4.10.13 the element\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 1 distinct sections, 1 cites", out)

    def test_longest_label_wins(self):
        """`WHATWG HTML` must not be parsed as the bare `HTML` alias."""
        td = self._tree({"a.rs": "/// WHATWG HTML §4.10.13\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 1 distinct sections", out)

    def test_bare_cite_inherits_within_the_comment_block(self):
        td = self._tree({"a.rs": "/// HTML §4.10.5 input\n/// see also §4.10.6\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 2 distinct sections, 2 cites", out)
        self.assertIn("UNATTRIBUTED (bare § with no spec in its comment "
                      "block): 0 cites", out)

    def test_non_comment_line_ends_the_block(self):
        td = self._tree({"a.rs": "/// HTML §4.10.5\nfn f() {}\n// bare §4.10.6\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 1 distinct sections, 1 cites", out)
        self.assertIn("UNATTRIBUTED (bare § with no spec in its comment "
                      "block): 1 cites", out)

    def test_unattributed_is_reported_not_silently_audited(self):
        """`§0.5` is a plan-memo pointer, not a spec citation."""
        td = self._tree({"a.rs": "// Spec references via D-17b §0.5 table\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 0 distinct sections", out)
        self.assertIn("UNATTRIBUTED (bare § with no spec in its comment "
                      "block): 1 cites", out)

    def test_show_unattributed_lists_them(self):
        td = self._tree({"a.rs": "// plan-memo §0.5\n"})
        out, _ = self._run(td, summary=False, show_unattributed=True)
        self.assertIn("UNATTRIBUTED cites", out)
        self.assertIn("a.rs:1", out)


class TestPrefixIsDottedComponent(_TreeCase):
    def test_prefix_does_not_string_match_across_components(self):
        td = self._tree({"a.rs": "/// HTML §4.10.2 and §4.10.20.3 and §4.10.2.7\n"})
        out, _ = self._run(td, prefix="4.10.2")
        # `4.10.2` and `4.10.2.7` match; `4.10.20.3` must NOT.
        self.assertIn("attributed to html: 2 distinct sections", out)

    def test_prefix_matches_the_exact_section(self):
        td = self._tree({"a.rs": "/// HTML §4.10\n"})
        out, _ = self._run(td, prefix="4.10")
        self.assertIn("attributed to html: 1 distinct sections", out)


class TestStrictExitCode(_TreeCase):
    def test_strict_exits_nonzero_when_a_section_is_unresolved(self):
        td = self._tree({"a.rs": "/// HTML §4.10.20.3 bogus\n"})
        _, code = self._run(td, strict=True)
        self.assertEqual(code, 1)

    def test_strict_exits_zero_when_all_resolve(self):
        td = self._tree({"a.rs": "/// HTML §4.10.13 progress\n"})
        _, code = self._run(td, strict=True)
        self.assertEqual(code, 0)

    def test_without_strict_unresolved_is_not_a_failure(self):
        td = self._tree({"a.rs": "/// HTML §4.10.20.3 bogus\n"})
        _, code = self._run(td, strict=False)
        self.assertEqual(code, 0)


class TestSharedSpecLabelMap(unittest.TestCase):
    """`spec_labels` is the single source for three consumers.

    It replaced three hand-maintained copies (`coverage_map`,
    `cite_audit`, and the plan-review `preflight.py`) that could drift
    apart — the same partial-enumeration failure `cite-audit` exists to
    detect, so a duplicate here would be self-refuting.
    """

    def test_both_directions_round_trip(self):
        for short, label in spec_labels.SHORTNAME_TO_LABEL.items():
            self.assertEqual(spec_labels.shortname_for(label), short)
            self.assertEqual(spec_labels.label_for(short), label)

    def test_every_shortname_is_its_own_alias(self):
        for short in spec_labels.SHORTNAME_TO_LABEL:
            self.assertEqual(spec_labels.shortname_for(short), short)

    def test_lookup_is_case_and_space_insensitive(self):
        self.assertEqual(spec_labels.shortname_for("  whatwg html "), "html")
        self.assertEqual(spec_labels.shortname_for("HTML"), "html")

    def test_unknown_label_is_none_not_a_guess(self):
        self.assertIsNone(spec_labels.shortname_for("WHATWG Nonesuch"))
        self.assertIsNone(spec_labels.label_for("nonesuch"))

    def test_aliases_do_not_collide_across_specs(self):
        """A label must map to exactly one shortname."""
        seen: dict[str, str] = {}
        for entry in spec_labels.SPECS:
            for label in entry[1:]:
                key = label.lower()
                self.assertNotIn(key, seen, f"{label} claimed by two specs")
                seen[key] = entry[0]

    def test_cite_audit_consumes_the_shared_map(self):
        """Regression: the attribution map must not be re-inlined."""
        self.assertIs(cite_audit.LABEL_TO_SHORTNAME,
                      spec_labels.LABEL_TO_SHORTNAME)


class TestResolverExactness(_TreeCase):
    def test_prefix_tolerant_resolver_is_pinned_to_an_exact_match(self):
        """`lookup_section` is prefix-tolerant and has no `exact` kwarg.

        A cited `§4.10.20.3` must report UNRESOLVED rather than silently
        passing because `§4.10.2` exists — the `hit[0] == section` guard.
        """
        td = self._tree({"a.rs": "/// HTML §4.10.20.3\n"})
        out, _ = self._run(td)
        self.assertIn("UNRESOLVED sections: 1", out)


if __name__ == "__main__":
    unittest.main()
