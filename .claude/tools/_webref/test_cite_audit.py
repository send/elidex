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
import json
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


class TestLabelWrapCarry(_TreeCase):
    """A label left dangling at end-of-line attributes the next line.

    Regression for a false negative found *inside* an audited set:
    `form_data.rs:181-182` reads "… WHATWG XHR §4.3 constructor + WHATWG
    HTML\n/// §4.10.21.3 step 7", so the wrapped cite inherited `xhr`
    from earlier on the previous line. A mis-bucketed cite is worse than
    an unattributed one, because nothing reports it.
    """

    def test_dangling_label_attributes_the_next_line(self):
        td = self._tree({"a.rs": "/// WHATWG XHR §4.3 ctor + WHATWG HTML\n"
                                 "/// §4.10.21.3 step 7\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 1 distinct sections, 1 cites", out)
        self.assertIn("attributed to another spec (not audited here): 1", out)

    def test_carry_applies_to_the_first_cite_only(self):
        """A second cite on the carried line falls back to the block."""
        td = self._tree({"a.rs": "/// see WHATWG HTML\n"
                                 "/// §4.10.5 and §4.10.6\n"})
        out, _ = self._run(td)
        # First inherits the carry; the second inherits it via block_spec.
        self.assertIn("attributed to html: 2 distinct sections, 2 cites", out)

    def test_carry_does_not_leak_two_lines_down(self):
        td = self._tree({"a.rs": "/// WHATWG HTML\n/// prose only\n"
                                 "/// §4.10.5\n"})
        out, _ = self._run(td)
        # Line 2 clears the carry; line 3 still inherits block_spec=None.
        self.assertIn("UNATTRIBUTED (bare § with no spec in its comment "
                      "block): 1 cites", out)


class TestPrefixScopesEveryReportedClass(_TreeCase):
    """`--prefix` must scope the UNATTRIBUTED list too, not just sections.

    It previously filtered `by_section` only, so a `--prefix`-scoped run
    still dumped the whole tree's unattributed cites.
    """

    def test_prefix_scopes_the_unattributed_count(self):
        td = self._tree({"a.rs": "// bare §4.10.5\n// bare §9.9.9\n"})
        out, _ = self._run(td, prefix="4.10")
        self.assertIn("UNATTRIBUTED (bare § with no spec in its comment "
                      "block): 1 cites", out)

    def test_prefix_scopes_the_other_spec_count(self):
        td = self._tree({"a.rs": "// WHATWG Fetch §4.10.5\n"
                                 "// WHATWG Fetch §9.9.9\n"})
        out, _ = self._run(td, prefix="4.10")
        self.assertIn("attributed to another spec (not audited here): 1", out)


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
            # index 2 is the cli help blurb, not a parse key — skip it.
            for label in (entry[0], entry[1], *entry[3:]):
                key = label.lower()
                # A key repeating within ONE spec is fine and expected —
                # `html`'s shortname and its `HTML` alias fold to the same
                # key. Only a key claimed by two DIFFERENT specs is a bug.
                self.assertEqual(seen.get(key, entry[0]), entry[0],
                                 f"{label!r} claimed by two specs")
                seen[key] = entry[0]

    def test_module_leaves_no_temporaries_to_delete(self):
        """Regression: the map is built by comprehension, not a loop.

        An earlier form accumulated into a dict with `for _entry in
        SPECS:` and ended `del _entry, _short, _labels, _label`. Pyright
        flagged all four as possibly-unbound, correctly: with an empty
        `SPECS` the loop never binds them and the `del` raises
        `NameError` **at import** — and this module is imported at load
        time by `cite-audit`, `coverage-map`, and the plan-review
        preflight gate, so one bad edit would take out all three.
        """
        leftovers = [
            n for n, v in vars(spec_labels).items()
            if n.startswith("_") and not n.startswith("__")
            and not callable(v)  # private helpers are fine; bound loop vars are not
        ]
        self.assertEqual(leftovers, [], f"module-level temporaries: {leftovers}")

    def test_empty_specs_would_still_import(self):
        """Re-exec the REAL module source with `SPECS` emptied.

        The earlier form re-implemented the comprehension inside the test,
        so it passed even if `spec_labels.py` were deleted — a test that
        survives the deletion of its subject is worse than no test,
        because it reads as coverage. This executes the shipped source.
        """
        src = Path(spec_labels.__file__).read_text(encoding="utf-8")
        # Neutralise the `SPECS = (...)` literal without touching anything
        # else, then run the module top-level exactly as import would.
        start = src.index("SPECS: tuple[tuple[str, ...], ...] = (")
        end = src.index("\n)\n", start) + len("\n)\n")
        src = src[:start] + "SPECS: tuple[tuple[str, ...], ...] = ()\n" + src[end:]
        ns: dict = {"__name__": "spec_labels_empty"}
        exec(compile(src, spec_labels.__file__, "exec"), ns)  # noqa: S102
        self.assertEqual(ns["LABEL_TO_SHORTNAME"], {})
        self.assertEqual(ns["SHORTNAME_TO_LABEL"], {})
        self.assertEqual(ns["SHORTNAME_TO_BLURB"], {})

    def test_all_three_consumers_derive_from_specs(self):
        """Derivation test — mechanism-agnostic, covers all three consumers.

        The earlier guard asserted `cite_audit.LABEL_TO_SHORTNAME is
        spec_labels.LABEL_TO_SHORTNAME`, which is true by construction of
        `from … import` and therefore only caught a literal re-inline in
        `cite_audit` — while `preflight` is the copy that empirically
        drifted, and no test imported it at all, so its `sys.path` shim
        was never exercised. This asserts the OUTPUT of each consumer
        instead, so any re-inlining or fallback drift fails regardless of
        mechanism. It also pins finding 2's round-trip break
        (`coverage_map`'s old `.upper().replace("-", " ")` fallback
        emitted labels `shortname_for` could not read back).
        """
        sys.path.insert(
            0, str(Path(__file__).resolve().parents[2] / "skills"
                   / "elidex-plan-review")
        )
        import importlib

        coverage_map = importlib.import_module("_webref.commands.coverage_map")
        preflight = importlib.import_module("preflight")

        for entry in spec_labels.SPECS:
            short, label = entry[0], entry[1]
            self.assertEqual(coverage_map._spec_label(short), label,
                             f"coverage_map drifted for {short}")
            self.assertEqual(preflight.shortname_from_label(label), short,
                             f"preflight drifted for {label}")

    def test_coverage_map_fallback_round_trips(self):
        """A label the generator emits must be one the gate can read back."""
        for short in ("css-text-3", "css-values-4"):
            label = coverage_map_label(short)
            self.assertIsNotNone(spec_labels.shortname_for(label),
                                 f"{short} -> {label!r} does not round-trip")


def coverage_map_label(short: str) -> str:
    import importlib

    return importlib.import_module("_webref.commands.coverage_map")._spec_label(short)


class TestAnnexGrammar(_TreeCase):
    """Finding 1: the detector must see annex citations, not only digits.

    Real ones exist — `elidex-api-crypto/src/rsa.rs` cites RFC 3447
    `§A.1.1` (a cross-spec mis-attribution, the class this tool
    advertises) and `events_modern/mod.rs` cites a plan-memo `§F.2` (the
    UNATTRIBUTED class). A digits-only grammar made both invisible.
    """

    def test_annex_cite_is_discovered(self):
        td = self._tree({"a.rs": "/// HTML §A.1.1 annex\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 1 distinct sections, 1 cites", out)

    def test_bare_annex_lands_in_unattributed(self):
        td = self._tree({"a.rs": "// plan-memo §F.2\n"})
        out, _ = self._run(td)
        self.assertIn("UNATTRIBUTED (bare § with no spec in its comment "
                      "block): 1 cites", out)

    def test_annex_sorts_without_raising(self):
        """`sec_number_key` handles annexes; `tuple(int(p) …)` would raise."""
        td = self._tree({"a.rs": "/// HTML §4.10.5 and §A.1 and §B.2.1\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 3 distinct sections", out)


class TestAnchorRefsAreNotPhantomSections(_TreeCase):
    """`re.IGNORECASE` applies to the whole pattern, so a naive `[\\dA-Z]+`
    number half turned anchor-style refs into phantom sections
    (`§attr-fs-method` → `attr`) and internal markers into one-letter ones
    (`§Deferred` → `D`). They were reported UNRESOLVED, so `--strict`
    failed partly on citations that do not exist.
    """

    def test_anchor_style_ref_is_not_a_section(self):
        td = self._tree({"a.rs": "/// HTML §attr-fs-method and §dom-document-title\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 0 distinct sections", out)

    def test_internal_marker_is_not_a_one_letter_section(self):
        td = self._tree({"a.rs": "// see §Deferred and §C1 and §C7\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 0 distinct sections", out)
        self.assertIn("UNATTRIBUTED (bare § with no spec in its comment "
                      "block): 0 cites", out)

    def test_real_annexes_still_parse(self):
        """The finding-1 widening must survive the case-scoping."""
        td = self._tree({"a.rs": "/// HTML §A.1.1 and §F.2 and §4.10.5\n"})
        out, _ = self._run(td)
        self.assertIn("attributed to html: 3 distinct sections", out)


class TestMissingRootExits(_TreeCase):
    def test_not_a_directory_exits_nonzero(self):
        """Finding 4: the same `return 1` defect `--strict` had, one site on."""
        _, code = self._run(Path("/nonexistent-cite-audit-root"))
        self.assertEqual(code, 1)


class TestJsonOutput(_TreeCase):
    def test_section_mark_is_not_escaped(self):
        """Finding 5: a tool about `§` must not emit `\u00a7`."""
        td = self._tree({"a.rs": "/// HTML §4.10.5 input\n"})
        out, _ = self._run(td, format="json", summary=False)
        self.assertNotIn("u00a7", out)
        self.assertIn("§", out)

    def test_json_records_carry_relative_paths_for_both_classes(self):
        """Finding 11: only one of the two renderers relativized before."""
        td = self._tree({"a.rs": "/// HTML §4.10.5\n", "b.rs": "// bare §9.1\n"})
        out, _ = self._run(td, format="json", summary=False)
        payload = json.loads(out)
        self.assertEqual(payload["unattributed_cites"], 1)
        self.assertEqual(payload["distinct_sections"], 1)


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
