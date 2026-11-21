#!/usr/bin/env python3
"""Offline tests: multi-line IDL extended attributes (`webref idl`). No
network: IDL is a fixture."""
from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from _webref._test_support import mutate  # noqa: E402
from _webref.commands import idl as idl_mod  # noqa: E402
from _webref.commands.idl import _IDL_DECL_RE, extract_idl_blocks  # noqa: E402

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
        text = 'enum E {\n  "a\n}; b",\n  "c"\n};\ninterface After {\n};\n'
        self.assertEqual(extract_idl_blocks(text, "E"),
                         ["enum E {", '  "a', '}; b",', '  "c"', "};", ""])

    def test_mutant_string_mode_reset_at_line_end(self):
        m = mutate(idl_mod, "end = line.find('\"', i)\n            if end < 0:\n                return depth, mode, False",
                   "end = line.find('\"', i)\n            if end < 0:\n                return depth, 'code', False")
        text = 'enum E {\n  "a\n}; b",\n  "c"\n};\ninterface After {\n};\n'
        self.assertEqual(m.extract_idl_blocks(text, "E"), ["enum E {", '  "a', '}; b",', ""])
        self.assertEqual(len(extract_idl_blocks(text, "E")), 6)

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


if __name__ == "__main__":
    unittest.main()
