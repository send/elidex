#!/usr/bin/env python3
"""Offline tests: multi-line IDL extended attributes (`webref idl`) and the
ReSpec ED render/TR-fallback path of `webref body`. No network: fetches are
injected / patched, HTML and IDL are fixtures."""
from __future__ import annotations

import argparse
import contextlib
import datetime
import io
import re
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref.cache import NotFound  # noqa: E402
from _webref.commands import body as body_mod  # noqa: E402
from _webref.commands.idl import _IDL_DECL_RE, extract_idl_blocks  # noqa: E402
from _webref import respec as respec_mod  # noqa: E402

IDL = """\
[Exposed=Window,
 LegacyOverrideBuiltIns]
interface DOMStringMap {
  getter DOMString (DOMString name);
};

[Exposed=Window]
interface Plain {
  attribute long x;
};

[Exposed=Window,
 SecureContext]
interface Other {
  [CEReactions] attribute long y;
};

interface Target {
  attribute long z;
};

[Exposed=Window] interface XRMesh {
  readonly attribute long w;
};

[SecureContext] partial interface Navigator {
  readonly attribute long n;
};
"""


def depth_blind_extract(text: str, iface: str) -> list[str]:
    """The pre-fix buffering logic (only lines starting with `[` buffer)."""
    iface_re = re.compile(rf"\b{re.escape(iface)}\b")
    out: list[str] = []
    buf: list[str] = []
    in_block = False
    for line in text.splitlines():
        if not in_block:
            if line.lstrip().startswith("["):
                buf.append(line)
                continue
            if _IDL_DECL_RE.match(line):
                if iface_re.search(line):
                    out.extend(buf)
                    out.append(line)
                    in_block = True
                buf = []
                continue
            buf = []
            continue
        out.append(line)
        if line.strip() == "};":
            in_block = False
            out.append("")
    return out


class IdlExtAttrTests(unittest.TestCase):
    def test_multiline_attrs_kept(self):
        out = extract_idl_blocks(IDL, "DOMStringMap")
        self.assertEqual(out[:3], ["[Exposed=Window,", " LegacyOverrideBuiltIns]",
                                   "interface DOMStringMap {"])

    def test_single_line_attrs_unchanged(self):
        out = extract_idl_blocks(IDL, "Plain")
        self.assertEqual(out[0], "[Exposed=Window]")
        self.assertEqual(out[1], "interface Plain {")

    def test_attrs_of_nonmatching_block_do_not_leak(self):
        out = extract_idl_blocks(IDL, "Target")
        self.assertEqual(out[0], "interface Target {")
        self.assertNotIn(" SecureContext]", out)

    def test_same_line_attrs_and_decl_found_by_name(self):
        out = extract_idl_blocks(IDL, "XRMesh")
        self.assertEqual(out[0], "[Exposed=Window] interface XRMesh {")
        self.assertIn("  readonly attribute long w;", out)
        nav = extract_idl_blocks(IDL, "Navigator")
        self.assertEqual(nav[0], "[SecureContext] partial interface Navigator {")

    def test_mutant_depth_blind_misses_same_line_block(self):
        self.assertEqual(depth_blind_extract(IDL, "XRMesh"), [])
        self.assertNotEqual(depth_blind_extract(IDL, "XRMesh"),
                            extract_idl_blocks(IDL, "XRMesh"))

    def test_nested_bracket_in_attribute_argument(self):
        # Synthetic: the measured corpus (356 IDL extracts) has no nested `[`,
        # but depth counting must still close the list at the matching `]`.
        text = '[Foo=(a,\n  [b]),\n Bar]\ninterface N {\n};\n'
        self.assertEqual(extract_idl_blocks(text, "N")[:3],
                         ["[Foo=(a,", "  [b]),", " Bar]"])

    def test_mutant_depth_blind_loses_attrs(self):
        # Mutant check: the old line-prefix buffering must fail the
        # multi-line control, proving the control has teeth.
        out = depth_blind_extract(IDL, "DOMStringMap")
        self.assertEqual(out[0], "interface DOMStringMap {")
        self.assertNotIn("[Exposed=Window,", out)
        self.assertNotEqual(out, extract_idl_blocks(IDL, "DOMStringMap"))


NIGHTLY = "https://w3c.github.io/x-spec/"
RELEASE = "https://www.w3.org/TR/x-spec/"
UNRENDERED = b"<html><body><section><h2>Source</h2><p>respec source, no ids</p></section></body></html>"
RENDERED = (b'<html><body><section id="sec"><h2 id="dom-x-foo">foo()</h2>'
            b"<p>Rendered steps.</p></section></body></html>")
TR = (b'<html><body><time class="dt-published" datetime="2026-06-11">11 June 2026</time>'
      b'<section id="sec"><h2 id="dom-x-foo">foo()</h2><p>TR steps.</p></section></body></html>')
TODAY = datetime.date(2026, 11, 21)


def index(generator: str):
    return {"x-spec": {"generator": generator, "nightly": {"url": NIGHTLY},
                       "release": {"url": RELEASE}}}


class RespecTests(unittest.TestCase):
    def run_resolve(self, fetches: dict, generator="respec", nightly=UNRENDERED):
        calls: list[str] = []

        def fetch(url: str) -> bytes:
            calls.append(url)
            v = fetches.get(url, NotFound(url))
            if isinstance(v, BaseException):
                raise v
            return v

        with mock.patch.object(respec_mod, "_data_index", lambda: index(generator)):
            r = respec_mod.resolve_respec_body("x-spec", NIGHTLY, "dom-x-foo",
                                               nightly, fetch=fetch, today=TODAY)
        return r, calls

    def test_unrendered_nightly_picks_spec_generator(self):
        gen = respec_mod.generator_url(NIGHTLY)
        self.assertIn("type=respec&url=https%3A%2F%2Fw3c.github.io%2Fx-spec%2F", gen)
        r, calls = self.run_resolve({gen: RENDERED})
        self.assertTrue(r[0])
        self.assertIn("Rendered steps.", r[1])
        self.assertEqual(r[3], "# source: ED rendered via spec-generator (2026-11-21)")
        self.assertEqual(calls, [gen])

    def test_rendered_nightly_needs_no_fetch(self):
        r, calls = self.run_resolve({}, nightly=RENDERED)
        self.assertTrue(r[0])
        self.assertEqual(calls, [])
        self.assertTrue(r[3].startswith("# source: ED nightly"))

    def test_generator_failure_falls_back_to_tr(self):
        gen = respec_mod.generator_url(NIGHTLY)
        for failure in (NotFound(gen), SystemExit("HTTP 500")):
            r, calls = self.run_resolve({gen: failure, RELEASE: TR})
            self.assertTrue(r[0])
            self.assertEqual(r[2], RELEASE)
            self.assertIn("TR steps.", r[1])
            self.assertTrue(r[3].startswith("# source: TR 2026-06-11"), r[3])
            self.assertEqual(calls, [gen, RELEASE])

    def test_generator_lacking_anchor_falls_back_to_tr(self):
        gen = respec_mod.generator_url(NIGHTLY)
        r, _ = self.run_resolve({gen: UNRENDERED, RELEASE: TR})
        self.assertEqual(r[2], RELEASE)
        self.assertTrue(r[0])

    def test_all_sources_miss(self):
        r, _ = self.run_resolve({})
        self.assertFalse(r[0])
        self.assertTrue(r[3].startswith("# source: none"))

    def test_bikeshed_spec_takes_old_path(self):
        r, calls = self.run_resolve({}, generator="bikeshed")
        self.assertIsNone(r)
        self.assertEqual(calls, [])

    def test_cmd_body_prints_source_line_first(self):
        gen = respec_mod.generator_url(NIGHTLY)
        pages = {NIGHTLY: UNRENDERED, gen: RENDERED}
        out = self.run_cmd_body("respec", pages)
        lines = out.splitlines()
        self.assertEqual(lines[0], "# source: ED rendered via spec-generator (%s)"
                         % datetime.date.today())
        self.assertEqual(lines[1], f"# {NIGHTLY}#dom-x-foo")
        self.assertIn("Rendered steps.", out)

    def test_cmd_body_bikeshed_control_unchanged(self):
        out = self.run_cmd_body("bikeshed", {NIGHTLY: RENDERED})
        self.assertEqual(out.splitlines()[0], f"# {NIGHTLY}#dom-x-foo")
        self.assertNotIn("# source:", out)

    def run_cmd_body(self, generator: str, pages: dict) -> str:
        def fetch(url: str) -> bytes:
            if url in pages:
                return pages[url]
            raise NotFound(url)

        args = argparse.Namespace(shortname="x-spec", anchor="dom-x-foo")
        buf = io.StringIO()
        with mock.patch.object(respec_mod, "_data_index", lambda: index(generator)), \
             mock.patch.object(respec_mod, "cached_fetch_url", fetch), \
             mock.patch.object(body_mod, "cached_fetch_url", fetch), \
             mock.patch.object(body_mod, "resolve_body_source",
                               lambda s, a: (NIGHTLY, a)), \
             contextlib.redirect_stdout(buf):
            body_mod.cmd_body(args)
        return buf.getvalue()


if __name__ == "__main__":
    unittest.main()
