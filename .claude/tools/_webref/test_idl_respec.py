#!/usr/bin/env python3
"""Offline tests: multi-line IDL extended attributes (`webref idl`) and the
ReSpec ED render/TR-fallback path of `webref body`. No network: fetches are
injected / patched, HTML and IDL are fixtures."""
from __future__ import annotations

import argparse
import contextlib
import datetime
import inspect
import io
import re
import sys
import types
import unittest
import urllib.error
import warnings
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref import cache as cache_mod  # noqa: E402
from _webref import cli as cli_mod  # noqa: E402
from _webref import resolver as resolver_mod  # noqa: E402
from _webref.cache import FetchError, NotFound  # noqa: E402
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


NIGHTLY = "https://w3c.github.io/x-spec/"
RELEASE = "https://www.w3.org/TR/x-spec/"
GEN = respec_mod.generator_url(NIGHTLY)
UNRENDERED = b"<html><body><section><h2>Source</h2><p>respec source, no ids</p></section></body></html>"
RENDERED = (b'<html><head><meta name="generator" content="ReSpec 35.0"></head>'
            b'<body><section id="sec"><h2 id="dom-x-foo">foo()</h2>'
            b"<p>Rendered steps.</p></section></body></html>")
# Rendered ReSpec ED that lacks the anchor asked for.
RENDERED_NO_ANCHOR = (b'<html><head><meta name="generator" content="ReSpec 35.0"></head>'
                      b'<body><section id="other"><h2 id="dom-x-bar">bar()</h2>'
                      b"<p>x</p></section></body></html>")
TR = (b'<html><body><time class="dt-published" datetime="2026-06-11">11 June 2026</time>'
      b'<section id="sec"><h2 id="dom-x-foo">foo()</h2><p>TR steps.</p></section></body></html>')
TODAY = datetime.date.today()

TC39_LOC = "https://tc39.es/x262/"
TC39_MULTI = TC39_LOC + "multipage/chap.html"
BIBLIO = {"location": TC39_LOC, "entries": [
    {"type": "clause", "id": "sec-chap", "number": "1", "title": "Chap"},
    {"type": "clause", "id": "sec-foo", "number": "1.2", "title": "Foo"},
]}
TC39_HTML = (b'<html><body><emu-clause id="sec-foo"><h1>Foo</h1>'
             b"<p>tc39 steps.</p></emu-clause></body></html>")


def entry(generator: str, release: bool = True):
    e = {"generator": generator, "nightly": {"url": NIGHTLY}}
    if release:
        e["release"] = {"url": RELEASE}
    return e


class CandidateTests(unittest.TestCase):
    """`resolve_body_candidates`: the single place that orders body sources."""

    def cands(self, generator="respec", href=NIGHTLY, release=True):
        with mock.patch.object(respec_mod, "spec_entry",
                               lambda s: entry(generator, release)), \
             mock.patch.object(resolver_mod, "webref_resolve_body_url",
                               lambda s, a: (href, a)):
            return resolver_mod.resolve_body_candidates("x-spec", "dom-x-foo")

    def test_respec_order_nightly_generator_release(self):
        anchor, c = self.cands()
        self.assertEqual(anchor, "dom-x-foo")
        self.assertEqual([x.url for x in c], [NIGHTLY, GEN, RELEASE])
        self.assertEqual([x.label for x in c], ["nightly", "generator", "tr"])
        # the rendering is cited as the nightly it renders
        self.assertEqual(c[1].cite, NIGHTLY)
        self.assertIn("type=respec&url=https%3A%2F%2Fw3c.github.io%2Fx-spec%2F", GEN)

    def test_respec_without_release_has_no_tr(self):
        _, c = self.cands(release=False)
        self.assertEqual([x.label for x in c], ["nightly", "generator"])

    def test_bikeshed_is_one_unlabelled_candidate(self):
        _, c = self.cands(generator="bikeshed")
        self.assertEqual(c, [resolver_mod.Candidate(NIGHTLY)])

    def test_respec_subpage_href_is_one_unlabelled_candidate(self):
        # href is not the nightly: rendering "the nightly" would be another page
        _, c = self.cands(href=NIGHTLY + "sub.html")
        self.assertEqual(c, [resolver_mod.Candidate(NIGHTLY + "sub.html")])

    def test_tc39_multipage_then_single_page(self):
        with mock.patch.object(resolver_mod, "tc39_biblio", lambda s: BIBLIO):
            anchor, c = resolver_mod.resolve_body_candidates("ecma262", "sec-foo")
        self.assertEqual(anchor, "sec-foo")
        self.assertEqual([x.url for x in c], [TC39_MULTI, TC39_LOC])
        self.assertTrue(all(x.label is None for x in c))


class BodyLoopTests(unittest.TestCase):
    """`cmd_body`'s one candidate loop, with fetches faked as FetchError."""

    def run_cmd_body(self, candidates, pages, module=body_mod, anchor="dom-x-foo"):
        calls: list[str] = []

        def fetch(url: str) -> bytes:
            calls.append(url)
            v = pages.get(url, NotFound(url))
            if isinstance(v, BaseException):
                raise v
            return v

        buf = io.StringIO()
        exit_msg = None
        with mock.patch.object(module, "cached_fetch_url", fetch), \
             mock.patch.object(module, "resolve_body_candidates",
                               lambda s, a: (a, candidates)), \
             contextlib.redirect_stdout(buf):
            try:
                module.cmd_body(argparse.Namespace(shortname="x-spec", anchor=anchor))
            except SystemExit as e:
                exit_msg = str(e)
        return buf.getvalue(), calls, exit_msg

    RESPEC = [resolver_mod.Candidate(NIGHTLY, "nightly"),
              resolver_mod.Candidate(GEN, "generator", cite=NIGHTLY),
              resolver_mod.Candidate(RELEASE, "tr")]

    def test_first_hit_wins(self):
        out, calls, _ = self.run_cmd_body(self.RESPEC, {NIGHTLY: RENDERED, GEN: TR})
        self.assertEqual(calls, [NIGHTLY])
        self.assertEqual(out.splitlines()[0], f"# source: ED nightly as served ({TODAY})")
        self.assertIn("Rendered steps.", out)

    def test_unrendered_nightly_picks_spec_generator(self):
        out, calls, _ = self.run_cmd_body(self.RESPEC, {NIGHTLY: UNRENDERED, GEN: RENDERED})
        self.assertEqual(calls, [NIGHTLY, GEN])
        lines = out.splitlines()
        self.assertEqual(lines[0], f"# source: ED rendered via spec-generator ({TODAY})")
        self.assertEqual(lines[1], f"# {NIGHTLY}#dom-x-foo")
        self.assertIn("Rendered steps.", out)

    def test_fetch_error_moves_to_next_candidate(self):
        for failure in (NotFound(GEN), FetchError("webref: HTTP 500 fetching " + GEN)):
            out, calls, _ = self.run_cmd_body(
                self.RESPEC, {NIGHTLY: UNRENDERED, GEN: failure, RELEASE: TR})
            self.assertEqual(calls, [NIGHTLY, GEN, RELEASE])
            lines = out.splitlines()
            self.assertTrue(lines[0].startswith("# source: TR 2026-06-11"), lines[0])
            self.assertEqual(lines[1], f"# {RELEASE}#dom-x-foo")
            self.assertIn("TR steps.", out)

    def test_mutant_stop_on_first_fetch_error_fails_the_control(self):
        # The same scenario run against a loop that gives up on the first
        # FetchError must NOT reach the TR: the control above has teeth.
        src = inspect.getsource(body_mod)
        needle = "first_error = first_error or e\n            continue"
        self.assertIn(needle, src)
        mutant = types.ModuleType("_webref.commands.body_mutant")
        mutant.__package__ = "_webref.commands"
        exec(compile(src.replace(needle, needle.replace("continue", "break")),
                     "body_mutant", "exec"), mutant.__dict__)
        pages = {NIGHTLY: UNRENDERED, GEN: FetchError("boom"), RELEASE: TR}
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", DeprecationWarning)
            out, calls, _ = self.run_cmd_body(self.RESPEC, pages, module=mutant)
        self.assertEqual(calls, [NIGHTLY, GEN])
        self.assertNotIn("TR steps.", out)
        out, calls, _ = self.run_cmd_body(self.RESPEC, pages)
        self.assertEqual(calls, [NIGHTLY, GEN, RELEASE])
        self.assertIn("TR steps.", out)

    def test_generator_lacking_anchor_falls_back_to_tr(self):
        out, calls, _ = self.run_cmd_body(
            self.RESPEC, {NIGHTLY: UNRENDERED, GEN: UNRENDERED, RELEASE: TR})
        self.assertEqual(calls, [NIGHTLY, GEN, RELEASE])
        self.assertIn("TR steps.", out)

    def test_all_sources_miss(self):
        out, calls, msg = self.run_cmd_body(
            self.RESPEC, {NIGHTLY: UNRENDERED, GEN: UNRENDERED, RELEASE: UNRENDERED})
        self.assertTrue(out.startswith("# source: none"))
        self.assertIn(f"not encountered in {RELEASE}", msg)

    def test_miss_on_unfetchable_tail_names_last_fetched(self):
        out, calls, msg = self.run_cmd_body(self.RESPEC, {NIGHTLY: UNRENDERED})
        self.assertEqual(calls, [NIGHTLY, GEN, RELEASE])
        self.assertIn(f"not encountered in {NIGHTLY}", msg)

    def test_every_fetch_failing_surfaces_the_first_error(self):
        err = FetchError("webref: HTTP 500 fetching " + NIGHTLY)
        with self.assertRaises(FetchError) as cm:
            self.run_cmd_body(self.RESPEC, {NIGHTLY: err})
        self.assertIs(cm.exception, err)

    def test_rendered_nightly_missing_anchor_skips_fallbacks(self):
        out, calls, msg = self.run_cmd_body(
            self.RESPEC, {NIGHTLY: RENDERED_NO_ANCHOR, GEN: RENDERED, RELEASE: TR})
        self.assertEqual(calls, [NIGHTLY])
        self.assertTrue(out.startswith("# source: none"))
        self.assertIn(f"not encountered in {NIGHTLY}", msg)

    def test_mutant_without_skip_would_fetch_the_fallbacks(self):
        pages = {NIGHTLY: RENDERED_NO_ANCHOR, GEN: RENDERED, RELEASE: TR}
        with mock.patch.object(body_mod, "miss_is_final", lambda label, data: False):
            out, calls, _ = self.run_cmd_body(self.RESPEC, pages)
        self.assertEqual(calls, [NIGHTLY, GEN])
        self.assertIn("Rendered steps.", out)

    def test_unlabelled_single_candidate_prints_no_source_line(self):
        out, _, _ = self.run_cmd_body([resolver_mod.Candidate(NIGHTLY)], {NIGHTLY: RENDERED})
        self.assertEqual(out.splitlines()[0], f"# {NIGHTLY}#dom-x-foo")
        self.assertNotIn("# source:", out)

    def test_unlabelled_miss_prints_no_source_none(self):
        out, _, msg = self.run_cmd_body([resolver_mod.Candidate(NIGHTLY)], {NIGHTLY: UNRENDERED})
        self.assertEqual(out, "")
        self.assertIn("not encountered", msg)

    def test_tc39_multipage_404_falls_back_to_single_page(self):
        cands = [resolver_mod.Candidate(TC39_MULTI), resolver_mod.Candidate(TC39_LOC)]
        out, calls, _ = self.run_cmd_body(cands, {TC39_LOC: TC39_HTML}, anchor="sec-foo")
        self.assertEqual(calls, [TC39_MULTI, TC39_LOC])
        self.assertEqual(out.splitlines()[0], f"# {TC39_LOC}#sec-foo")
        self.assertIn("tc39 steps.", out)

    def test_tc39_multipage_hit_never_fetches_single_page(self):
        cands = [resolver_mod.Candidate(TC39_MULTI), resolver_mod.Candidate(TC39_LOC)]
        out, calls, _ = self.run_cmd_body(cands, {TC39_MULTI: TC39_HTML}, anchor="sec-foo")
        self.assertEqual(calls, [TC39_MULTI])
        self.assertEqual(out.splitlines()[0], f"# {TC39_MULTI}#sec-foo")


class FetchErrorEdgeTests(unittest.TestCase):
    """cache raises typed errors; `cli.main` is the one place that exits."""

    URL = "https://example.invalid/spec/"

    def run_main(self, exc: BaseException) -> str:
        argv = ["webref", "body", "x-spec", "a"]
        with mock.patch.object(sys, "argv", argv), \
             mock.patch.dict("os.environ", {"ELIDEX_WEBREF_NO_CACHE": "1"}), \
             mock.patch.object(cache_mod.urllib.request, "urlopen",
                               mock.Mock(side_effect=exc)), \
             mock.patch.object(body_mod, "resolve_body_candidates",
                               lambda s, a: (a, [resolver_mod.Candidate(self.URL)])):
            with self.assertRaises(SystemExit) as cm:
                cli_mod.main()
        return str(cm.exception)

    def http(self, code):
        return urllib.error.HTTPError(self.URL, code, "x", {}, None)

    def test_http_error_exit_message_unchanged(self):
        self.assertEqual(self.run_main(self.http(500)),
                         f"webref: HTTP 500 fetching {self.URL}")

    def test_network_error_exit_message_unchanged(self):
        self.assertEqual(self.run_main(urllib.error.URLError("dns down")),
                         f"webref: network error fetching {self.URL}: dns down")

    def test_not_found_exit_message_unchanged(self):
        self.assertEqual(self.run_main(self.http(404)),
                         f"webref: fetch failed: {self.URL}")

    def test_cache_layer_raises_instead_of_exiting(self):
        with mock.patch.dict("os.environ", {"ELIDEX_WEBREF_NO_CACHE": "1"}), \
             mock.patch.object(cache_mod.urllib.request, "urlopen",
                               mock.Mock(side_effect=self.http(503))):
            with self.assertRaises(FetchError):
                cache_mod.cached_fetch_url(self.URL)
        self.assertTrue(issubclass(NotFound, FetchError))


if __name__ == "__main__":
    unittest.main()
