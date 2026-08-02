#!/usr/bin/env python3
"""Unit tests for the shared spec shortname ↔ label map.

Every assertion here is one of the pins the slice memo enumerates (S1-S8
and T-net); the pin name is named in each test's docstring so a failure
points back at the invariant rather than at the assertion.
"""
from __future__ import annotations

import importlib
import re
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref import cli  # noqa: E402
from _webref import spec_labels  # noqa: E402
from _webref.commands import coverage_map  # noqa: E402

# The package is the only tree this suite scans, and it is located from the
# test file itself — never from a repo root. The three pre-existing generic
# suites here all stop at `parents[1]`; reaching further (`parents[3]`, which
# on `origin/main` existed only in the elidex adapter) would make a package
# test depend on where the package is checked out, and would put unrelated
# elidex artifacts under `.claude/tools/` inside a webref unit test's blast
# radius. K2's and K3's CROSS-TREE halves — the wider `.claude/tools/` and
# `.claude/skills/` — are checked by `rederive couplings`, where cross-tree
# assertions belong.
WEBREF_PKG = Path(__file__).resolve().parent

# The reverse map the plan-review gate carried before this module existed,
# vendored as a literal. FROZEN: it is a snapshot taken once, and refreshing
# it would turn a pin into a mirror — the point is to hold the shared map to
# what the gate already resolved (S5).
_VENDORED_GATE_REVERSE = {
    "ECMA-262": "ecma262",
    "ECMA-402": "ecma402",
    "WHATWG HTML": "html",
    "WHATWG DOM": "dom",
    "WHATWG URL": "url",
    "WHATWG Fetch": "fetch",
    "WHATWG Streams": "streams",
    "WHATWG XHR": "xhr",
    "Web Cryptography API": "webcrypto",
    "Web IDL": "webidl",
    "CSS Selectors L4": "selectors-4",
    "Geometry Interfaces L1": "geometry-1",
    "HTML": "html",
    "DOM": "dom",
    "URL": "url",
}

# The `Common shortnames:` block the help blurb carried as a literal before
# it was derived, vendored the same way and for the same reason (S3b). It
# survives the change that deleted the original, which a comparison against
# the live source could not.
_VENDORED_BLURB_BLOCK = """\
  html         HTML LS (Custom Elements / Canvas / Workers / Form / Events — monolithic)
  dom          DOM LS
  selectors-4  CSS Selectors L4
  geometry-1   Geometry Interfaces (DOMRect / DOMMatrix)
  url          URL LS
  fetch        Fetch LS
  streams      Streams LS
  webcrypto    Web Cryptography API (series → current spec webcrypto-2)
  xhr          XMLHttpRequest LS
  webidl       Web IDL
  ecma262      ECMAScript Language Specification (tc39, biblio.json)
  ecma402      ECMAScript Internationalization API (tc39, biblio.json)"""

# The parse aliases this module deliberately does not carry. Each one
# lower-cases to its own shortname, so a column for them would add no key —
# which is what S4 executes rather than asserts.
_OMITTED_PARSE_ALIASES = {
    "HTML": "html",
    "DOM": "dom",
    "URL": "url",
    "Fetch": "fetch",
    "Streams": "streams",
    "WebCrypto": "webcrypto",
    "XHR": "xhr",
    "WebIDL": "webidl",
}


def _text_files(root: Path):
    """Every readable text file under `root`, caches and binaries skipped."""
    for path in sorted(root.rglob("*")):
        if not path.is_file() or "__pycache__" in path.parts:
            continue
        try:
            yield path, path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue


def _scan(root: Path, pattern: re.Pattern) -> list[str]:
    """`path:lineno:line` for every match of `pattern` under `root`."""
    hits = []
    for path, body in _text_files(root):
        for lineno, line in enumerate(body.splitlines(), 1):
            if pattern.search(line):
                hits.append(f"{path.relative_to(root)}:{lineno}:{line.strip()}")
    return hits


class TestSharedSpecLabelMap(unittest.TestCase):
    """`spec_labels` is the single source for two consumers.

    It replaced two hand-maintained copies in the generic tree —
    `coverage_map`'s label map and `cli.py`'s help blurb — which had
    drifted apart, since adding a spec to one never reached the other.
    """

    def test_shortname_for_resolves_labels_and_shortnames(self):
        """S1: both the canonical label and the shortname are parse keys."""
        for short, label, _blurb in spec_labels.SPECS:
            self.assertEqual(spec_labels.shortname_for(label), short)
            self.assertEqual(spec_labels.shortname_for(short), short)

    def test_label_for_resolves_every_pinned_shortname(self):
        """S2: the forward direction, over the same pinned set."""
        for short, label, _blurb in spec_labels.SPECS:
            self.assertEqual(spec_labels.label_for(short), label)

    def test_the_eight_omitted_parse_aliases_are_inert(self):
        """S4: the map is byte-identical with the aliases omitted.

        Adding them back must be a no-op — not "close enough", identical —
        because that is what makes leaving them out a refactor rather than
        a behaviour change.
        """
        widened = dict(spec_labels.LABEL_TO_SHORTNAME)
        for alias, short in _OMITTED_PARSE_ALIASES.items():
            widened[alias.lower()] = short
        self.assertEqual(widened, spec_labels.LABEL_TO_SHORTNAME)

    def test_shortname_for_agrees_with_the_vendored_gate_map(self):
        """S5: every spelling the gate's own reverse map resolved, resolves."""
        for label, short in _VENDORED_GATE_REVERSE.items():
            self.assertEqual(spec_labels.shortname_for(label), short,
                             f"{label!r} no longer resolves to {short!r}")

    def test_both_directions_compose_into_a_round_trip(self):
        """Neither direction is allowed to be lossy, over all 12 rows.

        S1 and S2 each pin one direction against `SPECS`. This composes
        them, which is the property a caller actually relies on: a label
        printed by `coverage-map` reads back as the shortname that printed
        it, and vice versa.
        """
        for short, label, _blurb in spec_labels.SPECS:
            self.assertEqual(
                spec_labels.label_for(spec_labels.shortname_for(label)), label)
            self.assertEqual(
                spec_labels.shortname_for(spec_labels.label_for(short)), short)

    def test_lookup_is_case_and_space_insensitive(self):
        self.assertEqual(spec_labels.shortname_for("  whatwg html "), "html")
        self.assertEqual(spec_labels.shortname_for("HTML"), "html")

    def test_unknown_label_is_none_not_a_guess(self):
        self.assertIsNone(spec_labels.shortname_for("WHATWG Nonesuch"))
        self.assertIsNone(spec_labels.shortname_for(""))
        self.assertIsNone(spec_labels.label_for("nonesuch"))

    def test_empty_specs_would_still_import(self):
        """Re-exec the REAL module source with `SPECS` emptied.

        Re-implementing the comprehension inside the test would pass even
        if the module were deleted — a test that survives the deletion of
        its subject reads as coverage without being it. This executes the
        shipped source, which is what pins the comprehension form: an
        accumulating loop ending in `del _entry, …` raises `NameError` at
        import when `SPECS` is empty, and both consumers import this
        module at load time.
        """
        src = Path(spec_labels.__file__).read_text(encoding="utf-8")
        start = src.index("SPECS: tuple[tuple[str, str, str], ...] = (")
        end = src.index("\n)\n", start) + len("\n)\n")
        src = src[:start] + "SPECS: tuple[tuple[str, str, str], ...] = ()\n" + src[end:]
        ns: dict = {"__name__": "spec_labels_empty"}
        exec(compile(src, spec_labels.__file__, "exec"), ns)  # noqa: S102
        self.assertEqual(ns["LABEL_TO_SHORTNAME"], {})
        self.assertEqual(ns["SHORTNAME_TO_LABEL"], {})
        self.assertEqual(ns["SHORTNAME_TO_BLURB"], {})


class TestConsumersDeriveFromSpecs(unittest.TestCase):
    """Both consumers must produce their output FROM `SPECS`.

    Agreement on today's values is NOT the assertion, because it does not
    discriminate: replayed against `origin/main`'s re-inlined `_spec_label`
    (`_SPEC_LABEL_MAP` plus the same last resort), the value comparison
    passes over all 12 rows. So the pin PERTURBS the canonical map and
    requires the consumer to follow — which the re-inlined body does not.
    """

    def test_coverage_map_label_derives_from_specs(self):
        """S3: `_spec_label` follows `SPECS`, it does not merely agree.

        The perturbation is what makes this a derivation pin: a re-inlined
        copy keeps answering `WHATWG HTML` while the canonical map says
        otherwise, and only the second assertion below sees that.
        """
        for short, label, _blurb in spec_labels.SPECS:
            self.assertEqual(coverage_map._spec_label(short), label,
                             f"coverage_map drifted for {short}")
        sentinel = "SPEC LABEL DERIVATION SENTINEL"
        original = spec_labels.SHORTNAME_TO_LABEL["html"]
        try:
            spec_labels.SHORTNAME_TO_LABEL["html"] = sentinel
            self.assertEqual(
                coverage_map._spec_label("html"), sentinel,
                "coverage_map answered from its own copy, not from SPECS",
            )
        finally:
            spec_labels.SHORTNAME_TO_LABEL["html"] = original
        self.assertEqual(coverage_map._spec_label("html"), original)

    def test_spec_label_covers_pinned_and_non_pinned_shortnames(self):
        """S6: the pinned set, plus the last resort for everything else.

        The last resort is unchanged from what the map replaced. A label
        it emits for a non-pinned spec does NOT read back through
        `shortname_for` — that round-trip is a behaviour change, not a
        refactor, so it is not this slice's to make.
        """
        self.assertEqual(len(spec_labels.SPECS), 12)
        for short, label, _blurb in spec_labels.SPECS:
            self.assertEqual(coverage_map._spec_label(short), label)
        self.assertEqual(coverage_map._spec_label("css-text-3"), "CSS TEXT 3")
        self.assertEqual(coverage_map._spec_label("cssom-view-1"), "CSSOM VIEW 1")
        self.assertIsNone(spec_labels.shortname_for("CSS TEXT 3"))

    def test_cli_blurb_block_reproduces_the_vendored_literal(self):
        """S3b: the derived help block is byte-identical to the old literal."""
        self.assertEqual(cli._SHORTNAME_LINES, _VENDORED_BLURB_BLOCK)
        self.assertIn(_VENDORED_BLURB_BLOCK, cli.COMMON_SHORTNAMES)


class TestSliceBoundary(unittest.TestCase):
    """This package names neither a Slice-B artifact nor an elidex path.

    Both are greps over prose occurrences, not over file assignments — a
    name in a docstring is the thing being forbidden, so a check that only
    looked at which files exist would pass on the failure it exists for.

    Scoped to `WEBREF_PKG`, which is the tree these pins are actually about.
    K2 and K3 both range wider than the package (`.claude/tools/` and
    `.claude/skills/`); that half is `rederive couplings`', so a violation
    planted outside the package turns the harness red and leaves this suite
    green — verified by planting one.
    """

    # Assembled from fragments on purpose: written whole, the needles would
    # match this file and the pins could never be green.
    _B_ARTIFACT = re.compile("cite" + ".?" + "audit")
    _B_FALLTHROUGH = re.compile(re.escape("_cat" + "alog"))
    _UPSTREAM_SOURCE = re.compile(re.escape("webref" + "_data"))
    # An elidex file path is `.claude/skills/` or `.claude/tools/` followed
    # by TWO further segments. The tool's own one-segment invocation path is
    # excluded deliberately: an install path is not a path into elidex's tree.
    _ELIDEX_PATH = re.compile(
        r"\.claude/(skills|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+"
    )

    def test_no_slice_b_artifact_is_named(self):
        """S7: the detector and its fall-through are not named here yet."""
        self.assertEqual(_scan(WEBREF_PKG, self._B_ARTIFACT), [])
        self.assertEqual(_scan(WEBREF_PKG, self._B_FALLTHROUGH), [])

    def test_the_shared_map_does_not_reach_upstream(self):
        """S7, third clause: the pinned map imports no upstream source."""
        body = Path(spec_labels.__file__).read_text(encoding="utf-8")
        self.assertIsNone(self._UPSTREAM_SOURCE.search(body))

    def test_no_elidex_file_path_in_this_package(self):
        """S8, package half: an absolute over the package, not a delta."""
        self.assertEqual(_scan(WEBREF_PKG, self._ELIDEX_PATH), [])


class TestNoNetworkOrCliSubprocess(unittest.TestCase):
    def test_import_and_lookup_reach_neither(self):
        """T-net: THE IMPORT PATH is inert — a tuple and three dicts.

        Scoped to the import path, not to the suite: this one test is where
        the escapes are poisoned, because the module load is the thing under
        test and re-executing it once under the poison exercises it. The
        load-time cost is paid on every plan-review gate run — the gate
        subprocesses the CLI once per citation it verifies — so what has to
        be inert is the import, not each subsequent call.
        """
        with patch("subprocess.run",
                   side_effect=AssertionError("subprocess.run on the import path")), \
             patch("urllib.request.urlopen",
                   side_effect=AssertionError("urlopen on the import path")):
            importlib.reload(spec_labels)
            importlib.reload(coverage_map)
            self.assertEqual(spec_labels.label_for("html"), "WHATWG HTML")
            self.assertEqual(spec_labels.shortname_for("WHATWG Fetch"), "fetch")
            self.assertEqual(coverage_map._spec_label("fetch"), "WHATWG Fetch")


if __name__ == "__main__":
    unittest.main()
