#!/usr/bin/env python3
"""Offline tests: multi-line IDL extended attributes (`webref idl`) and the
ReSpec ED render/TR-fallback path of `webref body`. No network: fetches are
injected / patched, HTML and IDL are fixtures."""
from __future__ import annotations

import argparse
import contextlib
import datetime
import functools
import inspect
import io
import json
import re
import sys
import tempfile
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
from _webref.cache import Fetched, FetchError, NotFound  # noqa: E402
from _webref.commands import body as body_mod  # noqa: E402
from _webref.commands import idl as idl_mod  # noqa: E402
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


def mutate(mod, needle: str, replacement: str, *more: str):
    """Re-exec `mod`'s source with one edit: a scratch copy carrying the
    mutant, never written to disk. The needle must be present."""
    src = inspect.getsource(mod)
    pairs = [(needle, replacement), *zip(more[::2], more[1::2])]
    for n, _ in pairs:
        assert n in src, n
    m = types.ModuleType(mod.__name__ + "_mutant")
    m.__package__ = mod.__package__
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        exec(compile(functools.reduce(lambda a, p: a.replace(*p), pairs, src),
                     "mutant", "exec"), m.__dict__)
    return m


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

    def test_one_line_blocks_close_on_the_declaration_line(self):
        text = ('enum E { "a" };\n[Exposed=Window] interface X {};\n'
                'interface After {\n  attribute long a;\n};\n')
        self.assertEqual(extract_idl_blocks(text, "E"), ['enum E { "a" };', ""])
        self.assertEqual(extract_idl_blocks(text, "X"),
                         ["[Exposed=Window] interface X {};", ""])
        self.assertEqual(extract_idl_blocks(text, "After")[0], "interface After {")

    def test_mutant_one_line_block_never_closes(self):
        text = 'enum E { "a" };\ninterface After {\n};\n'
        m = mutate(idl_mod, 'bdepth, mode, ended = _scan_definition(decl_text, 0, "code")',
                   'bdepth, mode, ended = 0, "code", False')
        self.assertIn("interface After {", m.extract_idl_blocks(text, "E"))
        self.assertNotIn("interface After {", extract_idl_blocks(text, "E"))

    CB = ("callback PositionCallback = undefined (GeolocationPosition position);\n\n"
          "callback LongCb = undefined (\n  long a,\n  long b\n);\n\n"
          "interface After {\n  attribute long a;\n};\n")

    def test_multiline_callback_ends_at_its_semicolon(self):
        self.assertEqual(extract_idl_blocks(self.CB, "PositionCallback"),
                         ["callback PositionCallback = undefined (GeolocationPosition position);", ""])
        self.assertEqual(extract_idl_blocks(self.CB, "LongCb"),
                         ["callback LongCb = undefined (", "  long a,", "  long b", ");", ""])

    def test_trailing_comment_after_close_ends_the_block(self):
        text = ("interface A {\n  attribute long a;\n}; // WEBGL_x\n\n"
                "interface B {\n  attribute long b;\n};\n")
        self.assertEqual(extract_idl_blocks(text, "A"),
                         ["interface A {", "  attribute long a;", "}; // WEBGL_x", ""])

    def test_braces_and_semicolons_in_strings_and_comments_are_ignored(self):
        text = ('enum E {\n  "a;}",\n  "b{" // };\n};\n'
                'interface X {\n  /* }; */\n  attribute long a;\n};\n'
                'interface After {\n};\n')
        self.assertEqual(extract_idl_blocks(text, "E")[-2:], ["};", ""])
        self.assertEqual(len(extract_idl_blocks(text, "E")), 5)
        self.assertEqual(extract_idl_blocks(text, "X")[-2:], ["};", ""])
        self.assertEqual(len(extract_idl_blocks(text, "X")), 5)

    def test_backslash_is_not_an_escape_in_idl_strings(self):
        # Web IDL: string = /"[^"]*"/ — `"\\"` is a complete string.
        text = 'enum E { "\\" };\ninterface After {\n};\n'
        self.assertEqual(extract_idl_blocks(text, "E"), ['enum E { "\\" };', ""])

    def test_mutant_backslash_escape_rule_restored(self):
        m = mutate(idl_mod, "end = line.find('\"', i)\n            if end < 0:\n                return depth, mode, False\n            i, mode = end + 1",
                   "end = next((j for j in range(i, n) if line[j] == '\"' and line[max(j - 1, 0)] != chr(92)), -1)\n            if end < 0:\n                return depth, mode, False\n            i, mode = end + 1")
        text = 'enum E { "\\" };\ninterface After {\n};\n'
        self.assertNotEqual(m.extract_idl_blocks(text, "E"), extract_idl_blocks(text, "E"))

    def test_string_spanning_lines_hides_its_semicolon(self):
        text = 'enum E {\n  "a\n;b",\n  "c"\n};\ninterface After {\n};\n'
        self.assertEqual(extract_idl_blocks(text, "E"),
                         ["enum E {", '  "a', ';b",', '  "c"', "};", ""])

    def test_mutant_line_shape_end_swallows_following_definitions(self):
        m = mutate(idl_mod, "bdepth, mode, ended = _scan_definition(line, bdepth, mode)",
                   'ended = line.strip() == "};"')
        text = ("callback LongCb = undefined (\n  long a\n);\n\n"
                "interface After {\n  attribute long a;\n};\n")
        self.assertEqual(len(extract_idl_blocks(text, "LongCb")), 4)
        self.assertGreater(len(m.extract_idl_blocks(text, "LongCb")), 4)
        text2 = "interface A {\n}; // c\ninterface B {\n};\n"
        self.assertEqual(extract_idl_blocks(text2, "A"), ["interface A {", "}; // c", ""])
        self.assertGreater(len(m.extract_idl_blocks(text2, "A")), 3)

    def test_name_match_is_the_declared_name_only(self):
        text = ("interface Document : Node {\n  attribute long d;\n};\n\n"
                "interface Node {\n  attribute long n;\n};\n")
        out = extract_idl_blocks(text, "Node")
        self.assertEqual(out[0], "interface Node {")
        self.assertNotIn("interface Document : Node {", out)
        self.assertEqual(extract_idl_blocks(text, "Document")[0],
                         "interface Document : Node {")

    def test_mutant_name_matched_anywhere_pulls_inheritor(self):
        text = "interface Document : Node {\n};\n"
        m2 = mutate(idl_mod, "iface_re.match(decl_text)", "iface_re.search(decl_text)",
                    'iface_re = re.compile(_IDL_DECL_RE.pattern + re.escape(iface) + r"\\b")',
                    'iface_re = re.compile(rf"\\b{re.escape(iface)}\\b")')
        self.assertNotEqual(m2.extract_idl_blocks(text, "Node"), [])
        self.assertEqual(extract_idl_blocks(text, "Node"), [])

    def test_two_lists_on_one_line_keep_both(self):
        text = "[A] [B,\n C]\ninterface X {\n};\n"
        self.assertEqual(extract_idl_blocks(text, "X")[:3],
                         ["[A] [B,", " C]", "interface X {"])
        self.assertEqual(extract_idl_blocks("[A] [B] interface X {\n};\n", "X")[0],
                         "[A] [B] interface X {")

    def test_mutant_scan_stops_at_first_close(self):
        text = "[A] [B,\n C]\ninterface X {\n};\n"
        m = mutate(idl_mod, "                    last_close = i\n",
                   "                    return 0, line[i + 1:]\n")
        self.assertNotEqual(m.extract_idl_blocks(text, "X")[:2], ["[A] [B,", " C]"])
        self.assertEqual(extract_idl_blocks(text, "X")[:2], ["[A] [B,", " C]"])


NIGHTLY = "https://w3c.github.io/x-spec/"
RELEASE = "https://www.w3.org/TR/x-spec/"
GEN = respec_mod.generator_url(NIGHTLY)
UNRENDERED = (b"<html><head><script>var respecConfig = {};</script></head><body><section>"
              b"<h2>Source</h2><p>respec source, no ids</p></section></body></html>")
RENDERED = (b'<html><head><meta name="generator" content="ReSpec 35.0"></head>'
            b'<body><section id="sec"><h2 id="dom-x-foo">foo()</h2>'
            b"<p>Rendered steps.</p></section></body></html>")
# Rendered ReSpec ED that lacks the anchor asked for.
RENDERED_NO_ANCHOR = (b'<html><head><meta name="generator" content="ReSpec 35.0"></head>'
                      b'<body><section id="other"><h2 id="dom-x-bar">bar()</h2>'
                      b"<p>x</p></section></body></html>")
RENDERED_PLAIN = (b'<html><body><h2 id="dom-x-foo">foo</h2><p>Plain steps.</p></body></html>')
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
            anchor, c = resolver_mod.resolve_body_candidates("x-spec", "dom-x-foo")
            return anchor, list(c)  # the tail consults the catalog: stay patched

    def test_respec_order_nightly_generator_release(self):
        anchor, c = self.cands()
        self.assertEqual(anchor, "dom-x-foo")
        self.assertEqual([x.url for x in c], [NIGHTLY, GEN, RELEASE])
        self.assertEqual([x.label for x in c], [None, "generator", "tr"])
        # the rendering is cited as the nightly it renders
        self.assertEqual(c[1].cite, NIGHTLY)
        self.assertIn("type=respec&url=https%3A%2F%2Fw3c.github.io%2Fx-spec%2F", GEN)

    def test_respec_without_release_has_no_tr(self):
        _, c = self.cands(release=False)
        self.assertEqual([x.label for x in list(c)], [None, "generator"])

    def test_bikeshed_is_one_unlabelled_candidate(self):
        _, c = self.cands(generator="bikeshed")
        self.assertEqual(list(c), [resolver_mod.Candidate(NIGHTLY)])

    def test_respec_subpage_href_is_one_unlabelled_candidate(self):
        # href is not the nightly: rendering "the nightly" would be another page
        _, c = self.cands(href=NIGHTLY + "sub.html")
        self.assertEqual(list(c), [resolver_mod.Candidate(NIGHTLY + "sub.html")])

    def test_tc39_multipage_then_single_page(self):
        with mock.patch.object(resolver_mod, "tc39_biblio", lambda s: BIBLIO):
            anchor, c = resolver_mod.resolve_body_candidates("ecma262", "sec-foo")
        self.assertEqual(anchor, "sec-foo")
        self.assertEqual([x.url for x in c], [TC39_MULTI, TC39_LOC])
        self.assertTrue(all(x.label is None for x in c))


class BodyLoopTests(unittest.TestCase):
    """`cmd_body`'s one candidate loop, with fetches faked as FetchError."""

    def run_cmd_body(self, candidates, pages, module=body_mod, anchor="dom-x-foo", date=None):
        calls: list[str] = []

        def fetch(url: str) -> bytes:
            calls.append(url)
            v = pages.get(url, NotFound(url))
            if isinstance(v, BaseException):
                raise v
            return v

        buf = io.StringIO()
        exit_msg = None
        with mock.patch.object(module, "cached_fetch",
                           lambda u: Fetched(fetch(u), date or TODAY)), \
             mock.patch.object(module, "resolve_body_candidates",
                               lambda s, a: (a, candidates)), \
             contextlib.redirect_stdout(buf):
            try:
                module.cmd_body(argparse.Namespace(shortname="x-spec", anchor=anchor))
            except SystemExit as e:
                exit_msg = str(e)
        return buf.getvalue(), calls, exit_msg

    RESPEC = [resolver_mod.Candidate(NIGHTLY),
              resolver_mod.Candidate(GEN, "generator", cite=NIGHTLY),
              resolver_mod.Candidate(RELEASE, "tr")]

    def test_first_hit_wins(self):
        out, calls, _ = self.run_cmd_body(self.RESPEC, {NIGHTLY: RENDERED, GEN: TR})
        self.assertEqual(calls, [NIGHTLY])
        lines = out.splitlines()
        self.assertEqual(lines[0], f"# source: rendered ReSpec page as served ({TODAY})")
        self.assertEqual(lines[1], f"# {NIGHTLY}#dom-x-foo")
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
        needle = 'record.append(Outcome(cand.label, "unreachable", str(e)))\n            continue'
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

    def test_miss_on_unfetchable_tail_reports_the_outage(self):
        out, calls, msg = self.run_cmd_body(self.RESPEC, {NIGHTLY: UNRENDERED})
        self.assertEqual(calls, [NIGHTLY, GEN, RELEASE])
        self.assertIn(f"{GEN}", msg)  # the first failed candidate, by URL
        self.assertIn("unreachable", msg)
        self.assertNotIn("not encountered", msg)
        self.assertNotIn("# source: none", out)

    def test_mutant_outage_reported_as_not_found(self):
        mutant = mutate(body_mod, "if not found and first_error is not None:",
                        "if False:")
        _, _, msg = self.run_cmd_body(self.RESPEC, {NIGHTLY: UNRENDERED}, module=mutant)
        self.assertIn("not encountered", msg)

    def test_unrendered_nightly_hit_is_not_accepted(self):
        raw = (b'<html><head><script>var respecConfig = {};</script></head><body>'
               b'<section id="dom-x-foo"><p>RAW respec markup.</p></section></body></html>')
        out, calls, _ = self.run_cmd_body(self.RESPEC, {NIGHTLY: raw, GEN: RENDERED})
        self.assertEqual(calls, [NIGHTLY, GEN])
        self.assertNotIn("RAW respec", out)
        self.assertIn("Rendered steps.", out)

    def test_mutant_unrendered_nightly_hit_accepted(self):
        raw = (b'<html><head><script>var respecConfig = {};</script></head><body>'
               b'<section id="dom-x-foo"><p>RAW respec markup.</p></section></body></html>')
        mutant = mutate(body_mod, 'if kind == "unrendered":',
                        "if False:")
        out, calls, _ = self.run_cmd_body(self.RESPEC, {NIGHTLY: raw, GEN: RENDERED},
                                          module=mutant)
        self.assertEqual(calls, [NIGHTLY])
        self.assertIn("RAW respec", out)

    def test_rendered_nightly_hit_still_accepted(self):
        out, calls, _ = self.run_cmd_body(self.RESPEC, {NIGHTLY: RENDERED})
        self.assertEqual(calls, [NIGHTLY])

    def test_every_fetch_failing_surfaces_the_first_error(self):
        err = FetchError("webref: HTTP 500 fetching " + NIGHTLY)
        with self.assertRaises(FetchError) as cm:
            self.run_cmd_body(self.RESPEC, {NIGHTLY: err})
        self.assertIs(cm.exception, err)

    def test_rendered_nightly_missing_anchor_skips_fallbacks(self):
        out, calls, msg = self.run_cmd_body(
            self.RESPEC, {NIGHTLY: RENDERED_NO_ANCHOR, GEN: RENDERED, RELEASE: TR})
        self.assertEqual(calls, [NIGHTLY])
        self.assertEqual(out, "")  # no fallback reached: no provenance line
        self.assertIn(f"not encountered in {NIGHTLY}", msg)

    def test_mutant_without_skip_would_fetch_the_fallbacks(self):
        pages = {NIGHTLY: RENDERED_NO_ANCHOR, GEN: RENDERED, RELEASE: TR}
        with mock.patch.object(body_mod, "miss_is_final", lambda label, data: False):
            out, calls, _ = self.run_cmd_body(self.RESPEC, pages)
        self.assertEqual(calls, [NIGHTLY, GEN])
        self.assertIn("Rendered steps.", out)

    def test_non_respec_single_candidate_prints_no_source_line(self):
        out, _, _ = self.run_cmd_body([resolver_mod.Candidate(NIGHTLY)], {NIGHTLY: RENDERED_PLAIN})
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

    def test_socket_errors_become_fetch_error(self):
        import http.client
        for exc in (TimeoutError("timed out"), ConnectionResetError("reset"),
                    http.client.RemoteDisconnected("closed")):
            for env in ({"ELIDEX_WEBREF_NO_CACHE": "1"}, {}):
                with self.subTest(exc=type(exc).__name__, env=bool(env)), \
                     mock.patch.dict("os.environ", env), \
                     tempfile.TemporaryDirectory() as td, \
                     mock.patch.dict("os.environ", {"XDG_CACHE_HOME": td}), \
                     mock.patch.object(cache_mod.urllib.request, "urlopen",
                                       mock.Mock(side_effect=exc)):
                    with self.assertRaises(FetchError) as cm:
                        cache_mod.cached_fetch_url(self.URL)
                    self.assertIn(f"network error fetching {self.URL}", str(cm.exception))

    def test_timeout_moves_body_to_next_candidate(self):
        cands = [resolver_mod.Candidate(NIGHTLY),
                 resolver_mod.Candidate(GEN, "generator", cite=NIGHTLY)]

        def urlopen(req, timeout=None):
            if req.full_url == NIGHTLY:
                raise TimeoutError("timed out")
            return io.BytesIO(RENDERED)

        class Resp(io.BytesIO):
            headers = {}
            def __enter__(self): return self
            def __exit__(self, *a): return False

        def urlopen2(req, timeout=None):
            if req.full_url == NIGHTLY:
                raise TimeoutError("timed out")
            return Resp(RENDERED)

        buf = io.StringIO()
        with mock.patch.dict("os.environ", {"ELIDEX_WEBREF_NO_CACHE": "1"}), \
             mock.patch.object(cache_mod.urllib.request, "urlopen", urlopen2), \
             mock.patch.object(body_mod, "resolve_body_candidates",
                               lambda s, a: (a, cands)), \
             contextlib.redirect_stdout(buf):
            body_mod.cmd_body(argparse.Namespace(shortname="x-spec", anchor="dom-x-foo"))
        self.assertIn("Rendered steps.", buf.getvalue())

    def test_mutant_socket_errors_escape(self):
        m = mutate(cache_mod, "except (TimeoutError, OSError, http.client.HTTPException) as e:",
                   "except ZeroDivisionError as e:")
        with mock.patch.dict("os.environ", {"ELIDEX_WEBREF_NO_CACHE": "1"}), \
             mock.patch.object(m.urllib.request, "urlopen",
                               mock.Mock(side_effect=TimeoutError("t"))):
            with self.assertRaises(TimeoutError):
                m.cached_fetch_url(self.URL)


class LazyCatalogTests(unittest.TestCase):
    """The catalog (`index.json`) is consulted only when the first candidate
    missed, failed, or was an unrendered ReSpec source."""

    # A real catalog case: a ReSpec spec whose nightly root ends in `.html`.
    HTML_NIGHTLY = "https://w3c.github.io/encrypted-media/eme-initdata-cenc.html"
    HTML_RELEASE = "https://www.w3.org/TR/eme-initdata-cenc/"

    def boom(self, shortname):
        raise FetchError("webref: HTTP 503 fetching index.json")

    def index_fetch(self, pages, calls, index=None, index_error=None):
        def fetch(url):
            calls.append(url)
            if url.endswith("/index.json"):
                if index_error:
                    raise index_error
                return json.dumps({"results": index}).encode()
            v = pages.get(url, NotFound(url))
            if isinstance(v, BaseException):
                raise v
            return v
        return fetch

    def run_body(self, href, pages, index=None, index_error=None, name="x-spec"):
        calls: list[str] = []
        fetch = self.index_fetch(pages, calls, index, index_error)
        buf = io.StringIO()
        msg = None
        webref_data = sys.modules["_webref.sources.webref_data"]
        with mock.patch.object(body_mod, "cached_fetch", lambda u: Fetched(fetch(u), TODAY)), \
             mock.patch.object(webref_data, "cached_fetch_url", fetch), \
             mock.patch.object(webref_data, "_INDEX", None), \
             mock.patch.object(resolver_mod, "webref_resolve_body_url",
                               lambda s, a: (href, a)), \
             contextlib.redirect_stdout(buf):
            try:
                body_mod.cmd_body(argparse.Namespace(shortname=name, anchor="dom-x-foo"))
            except SystemExit as e:
                msg = str(e)
        return buf.getvalue(), calls, msg

    def respec_index(self, name="eme-initdata-cenc"):
        return [{"shortname": name, "generator": "respec",
                 "nightly": {"url": self.HTML_NIGHTLY},
                 "release": {"url": self.HTML_RELEASE}}]

    def gen(self):
        return respec_mod.generator_url(self.HTML_NIGHTLY)

    def catalog_calls(self, calls):
        return [c for c in calls if c.endswith("/index.json")]

    def test_first_hit_never_fetches_the_catalog(self):
        # (a) a plain `body html ...` hit: headings-style page, no ReSpec
        out, calls, _ = self.run_body("https://html.spec.whatwg.org/",
                                      {"https://html.spec.whatwg.org/": RENDERED_PLAIN},
                                      index=[], name="html")
        self.assertEqual(calls, ["https://html.spec.whatwg.org/"])
        self.assertIn("Plain steps.", out)

    def test_rendered_respec_first_hit_no_catalog_names_source(self):
        out, calls, _ = self.run_body(NIGHTLY, {NIGHTLY: RENDERED}, index=[])
        self.assertEqual(self.catalog_calls(calls), [])
        self.assertEqual(out.splitlines()[0],
                         f"# source: rendered ReSpec page as served ({TODAY})")
        self.assertNotIn("nightly", out.splitlines()[0])

    def test_rendered_respec_first_miss_no_catalog(self):
        out, calls, _ = self.run_body(NIGHTLY, {NIGHTLY: RENDERED_NO_ANCHOR}, index=[])
        self.assertEqual(calls, [NIGHTLY])

    def test_html_nightly_unrendered_hit_tries_generator(self):
        # (b) the nightly root ends in `.html`: the chain must still apply
        out, calls, _ = self.run_body(
            self.HTML_NIGHTLY, {self.HTML_NIGHTLY: UNRENDERED, self.gen(): RENDERED},
            index=self.respec_index(), name="eme-initdata-cenc")
        self.assertEqual(calls[0], self.HTML_NIGHTLY)
        self.assertIn(self.gen(), calls)
        self.assertEqual(out.splitlines()[0],
                         f"# source: ED rendered via spec-generator ({TODAY})")
        self.assertIn("Rendered steps.", out)

    def test_html_nightly_miss_falls_back_to_tr(self):
        out, calls, _ = self.run_body(
            self.HTML_NIGHTLY, {self.HTML_RELEASE: TR}, index=self.respec_index(),
            name="eme-initdata-cenc")
        self.assertEqual([c for c in calls if not c.endswith("/index.json")],
                         [self.HTML_NIGHTLY, self.gen(), self.HTML_RELEASE])
        self.assertIn("TR steps.", out)

    def test_mutant_html_guard_restored_loses_the_chain(self):
        m = mutate(resolver_mod, "    ent = respec_entry(shortname)\n",
                   '    if url.split("?", 1)[0].endswith(".html"):\n        return []\n'
                   "    ent = respec_entry(shortname)\n")
        with mock.patch.object(resolver_mod, "_respec_fallbacks", m._respec_fallbacks):
            _, calls, _ = self.run_body(
                self.HTML_NIGHTLY,
                {self.HTML_NIGHTLY: UNRENDERED, self.gen(): RENDERED},
                index=self.respec_index(), name="eme-initdata-cenc")
        self.assertNotIn(self.gen(), calls)

    def test_mutant_eager_catalog_fetches_it_on_a_hit(self):
        m = mutate(resolver_mod,
                   "    return anchor, CandidateChain(Candidate(url), lambda: _respec_fallbacks(shortname, url))",
                   "    _pre = _respec_fallbacks(shortname, url)\n"
                   "    return anchor, CandidateChain(Candidate(url), lambda: _pre)")
        calls: list[str] = []
        fetch = self.index_fetch({NIGHTLY: RENDERED}, calls, index=[])
        webref_data = sys.modules["_webref.sources.webref_data"]
        buf = io.StringIO()
        with mock.patch.object(body_mod, "cached_fetch", lambda u: Fetched(fetch(u), TODAY)), \
             mock.patch.object(webref_data, "cached_fetch_url", fetch), \
             mock.patch.object(webref_data, "_INDEX", None), \
             mock.patch.object(body_mod, "resolve_body_candidates", m.resolve_body_candidates), \
             mock.patch.object(m, "webref_resolve_body_url", lambda s, a: (NIGHTLY, a)), \
             contextlib.redirect_stdout(buf):
            body_mod.cmd_body(argparse.Namespace(shortname="x-spec", anchor="dom-x-foo"))
        self.assertEqual(len(self.catalog_calls(calls)), 1)

    def test_catalog_outage_on_a_miss_is_unverified_absence(self):
        # (c) catalog unreachable: an unreachable source, never "not encountered"
        out, calls, msg = self.run_body(
            NIGHTLY, {NIGHTLY: UNRENDERED},
            index_error=FetchError("webref: HTTP 503 fetching index.json"))
        self.assertEqual(len(self.catalog_calls(calls)), 1)
        self.assertIn("index.json", msg)
        self.assertIn("absence is unverified", msg)
        self.assertIn("unreachable: catalog", msg)
        self.assertNotIn("not encountered", msg)
        self.assertEqual(out, "")

    def test_catalog_outage_with_unreachable_first_surfaces_first_error(self):
        err = FetchError("webref: HTTP 500 fetching " + NIGHTLY)
        with self.assertRaises(FetchError) as cm:
            self.run_body(NIGHTLY, {NIGHTLY: err},
                          index_error=FetchError("webref: HTTP 503 fetching index.json"))
        self.assertIs(cm.exception, err)

    def test_catalog_notfound_is_unreachable_too(self):
        _, _, msg = self.run_body(NIGHTLY, {NIGHTLY: UNRENDERED},
                                  index_error=NotFound("index.json"))
        self.assertIn("absence is unverified", msg)

    def test_mutant_swallowed_catalog_outage_reads_as_a_miss(self):
        m = mutate(resolver_mod, "    ent = respec_entry(shortname)\n",
                   "    try:\n        ent = respec_entry(shortname)\n"
                   "    except FetchError:\n        return []\n",
                   "from .cache import NotFound", "from .cache import FetchError, NotFound")
        with mock.patch.object(resolver_mod, "_respec_fallbacks", m._respec_fallbacks):
            out, calls, msg = self.run_body(
                NIGHTLY, {NIGHTLY: UNRENDERED},
                index_error=FetchError("webref: HTTP 503 fetching index.json"))
        self.assertIn("not encountered", msg)  # the defect R1a closes


class OutcomeRecordTests(unittest.TestCase):
    """R1/R3: the source line is built from the per-candidate outcome record."""

    RESPEC = BodyLoopTests.RESPEC
    body = BodyLoopTests.run_cmd_body

    def test_tr_after_unreachable_ed_says_so(self):
        err = FetchError("webref: HTTP 500 fetching " + NIGHTLY)
        out, _, _ = self.body(self.RESPEC, {NIGHTLY: err, GEN: NotFound(GEN), RELEASE: TR})
        first = out.splitlines()[0]
        self.assertTrue(first.startswith("# source: TR 2026-06-11 ("), first)
        self.assertIn("ED unreachable: webref: HTTP 500", first)
        self.assertIn("spec-generator unreachable", first)
        self.assertNotIn("not renderable or lacks", first)

    def test_tr_after_unrendered_and_miss_says_why(self):
        out, _, _ = self.body(self.RESPEC, {NIGHTLY: UNRENDERED, GEN: UNRENDERED, RELEASE: TR})
        self.assertIn("(ED unrendered; spec-generator unrendered)", out.splitlines()[0])

    def test_generator_after_unreachable_ed_says_so(self):
        err = FetchError("webref: HTTP 500 fetching " + NIGHTLY)
        out, _, _ = self.body(self.RESPEC, {NIGHTLY: err, GEN: RENDERED})
        self.assertIn(f"({TODAY}; ED unreachable: webref: HTTP 500", out.splitlines()[0])

    def test_mutant_unreachable_not_recorded_in_source_line(self):
        m = mutate(respec_mod, 'if o.state == "unreachable":\n        return f"{name} unreachable',
                   'if False:\n        return f"{name} unreachable')
        line = m.source_line("tr", TR, TODAY,
                             [m.Outcome(None, "unreachable", "boom")])
        self.assertNotIn("unreachable", line)
        self.assertIn("ED unreachable: boom",
                      respec_mod.source_line("tr", TR, TODAY,
                                             [respec_mod.Outcome(None, "unreachable", "boom")]))

    def test_source_date_is_the_fetched_copys_date_not_today(self):
        d = datetime.date(2025, 1, 2)
        for pages, first in (({NIGHTLY: RENDERED}, "rendered ReSpec page as served"),
                             ({NIGHTLY: UNRENDERED, GEN: RENDERED}, "ED rendered via spec-generator")):
            out, _, _ = self.body(self.RESPEC, pages, date=d)
            self.assertEqual(out.splitlines()[0], f"# source: {first} (2025-01-02)")

    def test_mutant_date_is_today(self):
        m = mutate(body_mod, "record[-1].date", "__import__('datetime').date.today()")
        out, _, _ = self.body(self.RESPEC, {NIGHTLY: RENDERED}, module=m,
                              date=datetime.date(2025, 1, 2))
        self.assertNotIn("2025-01-02", out.splitlines()[0])

    def test_text_empty_hit_still_names_source_first(self):
        empty = (b'<html><head><meta name="generator" content="ReSpec 35.0"></head>'
                 b'<body><section id="dom-x-foo"></section></body></html>')
        out, _, _ = self.body([resolver_mod.Candidate(NIGHTLY)], {NIGHTLY: empty})
        self.assertTrue(out.startswith("# source: rendered ReSpec page as served"))


class ClassifyRespecTests(unittest.TestCase):
    """R2: one classifier, fixed precedence: generator meta -> rendered; ReSpec
    loader script or respecConfig -> unrendered; `id="respecDocument"` ->
    rendered; else ordinary."""

    EXT = (b'<html><head><script src="https://www.w3.org/Tools/respec/respec-w3c" '
           b'class="remove" async></script></head><body><section id="dom-x-foo">'
           b'<p>{{Foo}}</p></section></body></html>')
    # Shaped like the cached dx-prof-1.0 / dx-prof-conneg nightlies: hand-written
    # ReSpec source whose <body> carries id="respecDocument", a loader script,
    # and no generator meta.
    DX = (b'<!DOCTYPE html><html lang="en"><head><meta charset="utf-8">'
          b'<script class="remove" src="https://www.w3.org/Tools/respec/respec-w3c" '
          b'async></script></head><body id="respecDocument" class="h-entry">'
          b'<section id="dom-x-foo"><p>{{Foo}}</p></section></body></html>')
    classify = staticmethod(lambda h: respec_mod.classify_respec(h))

    def test_loader_script_is_unrendered(self):
        self.assertEqual(self.classify(self.EXT), "unrendered")
        for src in (b"js/respec-w3c.js", b"//www.w3.org/Tools/respec/respec-w3c-common",
                    b"https://www.w3.org/Tools/respec/respec-w3c-common.js",
                    b"respec-w3c.js?v=3"):
            with self.subTest(src=src):
                self.assertEqual(self.classify(b"<script src='" + src + b"'></script>"),
                                 "unrendered")
        self.assertEqual(self.classify(b"<script src=respec-w3c.js class=remove></script>"),
                         "unrendered")

    def test_only_a_src_attribute_naming_a_respec_build_is_a_loader(self):
        for h in (b"<script src='js/irrespective.js'></script>",
                  b'<script data-src="respec-w3c.js"></script>',
                  b"<script src='respec-config.js'></script>",
                  b"<script src='https://cdn.jsdelivr.net/gh/w3c/respec-vc@3.5.1/dist/main.js'></script>",
                  b"<script src='/respec/index.html'></script>"):
            with self.subTest(h=h):
                self.assertEqual(self.classify(h), "ordinary")

    def test_inline_config_is_unrendered(self):
        self.assertEqual(self.classify(UNRENDERED), "unrendered")

    def test_hand_written_respec_source_with_respecDocument_is_unrendered(self):
        self.assertEqual(self.classify(self.DX), "unrendered")

    def test_respecDocument_alone_is_rendered(self):
        self.assertEqual(self.classify(b'<div id="respecDocument"><h2>x</h2></div>'), "rendered")

    def test_generator_meta_outranks_loader_and_config(self):
        both = (b'<meta name="generator" content="ReSpec 35.0">'
                b'<script src="respec-w3c.js"></script><script>respecConfig</script>')
        self.assertEqual(self.classify(both), "rendered")

    def test_ordinary_pages(self):
        for h in (RENDERED_PLAIN, b"<html><body><script src='app.js'></script></body></html>"):
            self.assertEqual(self.classify(h), "ordinary")

    def test_mutant_respecDocument_before_loader(self):
        m = mutate(respec_mod,
                   '    if _has_respec_loader(html) or b"respecConfig" in html:\n        return "unrendered"\n'
                   '    if b\'id="respecDocument"\' in html:\n        return "rendered"\n',
                   '    if b\'id="respecDocument"\' in html:\n        return "rendered"\n'
                   '    if _has_respec_loader(html) or b"respecConfig" in html:\n        return "unrendered"\n')
        self.assertEqual(m.classify_respec(self.DX), "rendered")
        self.assertEqual(self.classify(self.DX), "unrendered")

    def test_mutant_config_literal_only(self):
        m = mutate(respec_mod, 'if _has_respec_loader(html) or b"respecConfig" in html:',
                   'if b"respecConfig" in html:')
        self.assertEqual(m.classify_respec(self.EXT), "ordinary")
        self.assertEqual(self.classify(self.EXT), "unrendered")

    def test_mutant_data_src_and_substring_accepted(self):
        m = mutate(respec_mod, "(?<![-\\w])src", "src",
                   "_LOADER_NAME_RE.fullmatch(name)", "'respec' in name")
        self.assertEqual(m.classify_respec(b'<script data-src="respec-w3c.js"></script>'),
                         "unrendered")
        self.assertEqual(m.classify_respec(b"<script src='irrespective.js'></script>"),
                         "unrendered")

    def test_miss_is_final_uses_the_classifier(self):
        self.assertTrue(respec_mod.miss_is_final(None, RENDERED))
        self.assertFalse(respec_mod.miss_is_final(None, self.DX))
        self.assertFalse(respec_mod.miss_is_final("generator", RENDERED))

    def test_hand_written_source_hit_is_not_accepted_in_body(self):
        out, calls, _ = BodyLoopTests.run_cmd_body(
            self, BodyLoopTests.RESPEC, {NIGHTLY: self.DX, GEN: RENDERED})
        self.assertEqual(calls, [NIGHTLY, GEN])
        self.assertNotIn("{{Foo}}", out)

    def test_external_config_nightly_hit_is_not_accepted_in_body(self):
        out, calls, _ = BodyLoopTests.run_cmd_body(
            self, BodyLoopTests.RESPEC, {NIGHTLY: self.EXT, GEN: RENDERED})
        self.assertEqual(calls, [NIGHTLY, GEN])
        self.assertNotIn("{{Foo}}", out)


class FetchedDateTests(unittest.TestCase):
    """The date is a property of the fetched copy (`cache.Fetched.date`)."""
    URL = "https://example.invalid/copy-date/"
    LM = "Tue, 03 Mar 2026 10:00:00 GMT"

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        env = mock.patch.dict("os.environ", {"XDG_CACHE_HOME": self.tmp.name})
        env.start()
        self.addCleanup(env.stop)
        import os
        os.environ.pop("ELIDEX_WEBREF_NO_CACHE", None)
        self.dir = Path(self.tmp.name) / "elidex-webref"
        self.dir.mkdir()
        import hashlib
        self.key = hashlib.sha1(self.URL.encode()).hexdigest()

    class Resp(io.BytesIO):
        def __init__(self, data, headers):
            super().__init__(data)
            self.headers = headers
        def __enter__(self): return self
        def __exit__(self, *a): return False

    def fetch(self, mod=cache_mod, *, resp=None, error=None):
        def urlopen(req, timeout=None):
            if error is not None:
                raise error
            return resp
        with mock.patch.object(mod.urllib.request, "urlopen", urlopen):
            return mod.cached_fetch(self.URL)

    def store(self, meta, age_date=None):
        import os
        (self.dir / self.key).write_bytes(b"old")
        (self.dir / f"{self.key}.meta").write_text(json.dumps(meta))
        if age_date:
            ts = datetime.datetime.combine(age_date, datetime.time(12)).timestamp()
            os.utime(self.dir / self.key, (ts, ts))

    def not_modified(self):
        return urllib.error.HTTPError(self.URL, 304, "nm", {}, None)

    def test_200_with_last_modified_is_that_date(self):
        f = self.fetch(resp=self.Resp(b"new", {"Last-Modified": self.LM}))
        self.assertEqual((f.data, f.date), (b"new", datetime.date(2026, 3, 3)))

    def test_200_without_last_modified_uses_the_response_date_then_today(self):
        f = self.fetch(resp=self.Resp(b"n", {"Date": "Mon, 02 Feb 2026 10:00:00 GMT"}))
        self.assertEqual(f.date, datetime.date(2026, 2, 2))
        f = self.fetch(resp=self.Resp(b"n", {}))
        self.assertEqual(f.date, datetime.date.today())

    def test_304_etag_only_is_revalidated_today_not_first_store_date(self):
        self.store({"etag": '"x"'}, age_date=datetime.date(2025, 7, 8))
        f = self.fetch(error=self.not_modified())
        self.assertEqual((f.data, f.date), (b"old", datetime.date.today()))

    def test_304_keeps_the_stored_last_modified(self):
        self.store({"etag": '"x"', "last_modified": self.LM}, age_date=datetime.date(2025, 7, 8))
        self.assertEqual(self.fetch(error=self.not_modified()).date, datetime.date(2026, 3, 3))

    def test_cache_write_failure_dates_the_fetched_response_not_the_old_meta(self):
        old = "Sat, 01 Feb 2025 10:00:00 GMT"
        self.store({"last_modified": old})
        # No conditional headers are sent when the stored meta is unusable, but
        # here the meta is valid: the server answers 200 with a new copy.
        with mock.patch.object(Path, "write_bytes", side_effect=OSError("disk full")):
            f = self.fetch(resp=self.Resp(b"new", {"Last-Modified": self.LM}))
        self.assertEqual((f.data, f.date), (b"new", datetime.date(2026, 3, 3)))

    def test_mutant_date_reread_from_meta_after_the_fetch(self):
        old = "Sat, 01 Feb 2025 10:00:00 GMT"
        self.store({"last_modified": old})
        m = mutate(cache_mod, "            return Fetched(data, date)\n",
                   "            return Fetched(data, _http_date(json.loads(meta_path.read_text('utf-8')).get('last_modified')) or date)\n")
        with mock.patch.object(Path, "write_bytes", side_effect=OSError("disk full")):
            f = self.fetch(m, resp=self.Resp(b"new", {"Last-Modified": self.LM}))
        self.assertEqual(f.date, datetime.date(2025, 2, 1))  # the mutant is wrong

    def test_no_cache_env_dates_the_response(self):
        with mock.patch.dict("os.environ", {"ELIDEX_WEBREF_NO_CACHE": "1"}):
            f = self.fetch(resp=self.Resp(b"n", {"Last-Modified": self.LM}))
        self.assertEqual(f.date, datetime.date(2026, 3, 3))

    def test_cached_fetch_url_still_returns_bytes(self):
        with mock.patch.object(cache_mod.urllib.request, "urlopen",
                               lambda r, timeout=None: self.Resp(b"abc", {})):
            self.assertEqual(cache_mod.cached_fetch_url(self.URL), b"abc")


if __name__ == "__main__":
    unittest.main()
