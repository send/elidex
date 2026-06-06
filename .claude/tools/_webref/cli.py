"""webref CLI — argparse wiring + main entry point.

See package docstring at .claude/tools/webref for user-facing help text.
"""
from __future__ import annotations

import argparse
import json
import sys

from .cache import NotFound
from .commands.aoid import cmd_aoid
from .commands.body import cmd_body
from .commands.coverage_map import cmd_coverage_map
from .commands.css import cmd_css
from .commands.dfn import cmd_dfn
from .commands.element import cmd_element
from .commands.heading import cmd_heading
from .commands.idl import cmd_idl
from .commands.specs import cmd_specs

COMMON_SHORTNAMES = """\
Common shortnames:
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
  ecma402      ECMAScript Internationalization API (tc39, biblio.json)

Examples:
  .claude/tools/webref heading html 4.13                     # all §4.13.x sections
  .claude/tools/webref heading ecma262 25.5.2                # §25.5.2 JSON.parse
  .claude/tools/webref aoid ecma262 ToNumber                 # AO → §7.1.4 + anchor
  .claude/tools/webref dfn html 'reaction queue'             # term → §heading + anchor
  .claude/tools/webref idl html CustomElementRegistry
  .claude/tools/webref element html canvas                   # → HTMLCanvasElement
  .claude/tools/webref css css-overflow-3 overflow-x         # property metadata
  .claude/tools/webref heading webcrypto 31                  # §31 HMAC (number recovered from title)
  .claude/tools/webref dfn webcrypto 'normalize an algorithm'
  .claude/tools/webref body webcrypto hmac                   # §31 HMAC algorithm prose
  .claude/tools/webref body ecma262 sec-iteratorclose        # §7.4.11 IteratorClose prose (multipage)
  .claude/tools/webref body ecma262 IteratorClose            # AO name resolves to anchor first
  .claude/tools/webref body html the-iframe-element          # §4.8.5 prose (multipage chapter)
  .claude/tools/webref specs canvas                          # shortname by keyword
  .claude/tools/webref coverage-map ecma262 15.7.14 html 4.13.4 ecma262 ClassDefinitionEvaluation
                                                             # plan-memo §3 skeleton

Cache: HTTP responses are cached in ~/.cache/elidex-webref/ (or
       $XDG_CACHE_HOME/elidex-webref/ when XDG_CACHE_HOME is set) with
       ETag / Last-Modified conditional GET. Set ELIDEX_WEBREF_NO_CACHE=1
       to bypass.
"""


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=".claude/tools/webref",
        description=(
            "webref lookup helper — spec citation verification for elidex. "
            "Backs Axis 4 (Spec citation discipline) recipe in "
            ".claude/skills/elidex-review/axes.md."
        ),
        epilog=COMMON_SHORTNAMES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = p.add_subparsers(dest="command", required=True, metavar="<command>")

    h = sub.add_parser("heading", help="§number → title + anchor")
    h.add_argument("shortname", help="e.g. html, dom, css-overflow-3")
    h.add_argument("prefix", help="section number prefix, e.g. 4.13 or 4.12.5.3")
    h.add_argument(
        "--exact", action="store_true",
        help="match the section number exactly (no prefix tree); use for drift-verify",
    )
    h.set_defaults(func=cmd_heading)

    i = sub.add_parser("idl", help="interface IDL fragment")
    i.add_argument("shortname")
    i.add_argument("iface", help="interface / dictionary / enum name")
    i.set_defaults(func=cmd_idl)

    d = sub.add_parser("dfn", help="concept dfn → §heading + anchor")
    d.add_argument("shortname")
    d.add_argument("term", help="dfn term (substring match if exact misses)")
    d.set_defaults(func=cmd_dfn)

    e = sub.add_parser("element", help="HTML/SVG element → interface name + href")
    e.add_argument("shortname")
    e.add_argument("name", help="element tag name, e.g. canvas")
    e.set_defaults(func=cmd_element)

    c = sub.add_parser("css", help="CSS property / @rule / selector / value metadata")
    c.add_argument("shortname")
    c.add_argument("name", help="property / @rule / selector / value name")
    c.set_defaults(func=cmd_css)

    a = sub.add_parser("aoid", help="(tc39) abstract operation aoid → §number + anchor")
    a.add_argument("shortname", help="ecma262 / ecma402")
    a.add_argument("name", help="abstract operation name, e.g. ToNumber")
    a.set_defaults(func=cmd_aoid)

    b = sub.add_parser(
        "body",
        help="fetch + extract section prose by anchor (or AO name for tc39)",
        description=(
            "Fetch the spec chapter HTML for `<shortname>` (multipage URL "
            "via webref href or tc39 biblio derivation), extract the section "
            "rooted at the given anchor, and print it as plain text. For "
            "tc39 specs (ecma262/ecma402), an abstract operation name "
            "(e.g. IteratorClose) is auto-resolved to its sec-* anchor. "
            "Output preserves <ol>/<ul> numbering and headings; cross-ref "
            "tags are kept as text only. Cached chapter HTML lives in the "
            "shared ~/.cache/elidex-webref/ HTTP cache (conditional GET)."
        ),
    )
    b.add_argument("shortname", help="e.g. html, dom, ecma262")
    b.add_argument(
        "anchor",
        help="section anchor (e.g. sec-iteratorclose, the-iframe-element) "
        "or tc39 abstract operation name (e.g. IteratorClose)",
    )
    b.set_defaults(func=cmd_body)

    s = sub.add_parser("specs", help="shortname lookup (by spec title)")
    s.add_argument("keyword")
    s.set_defaults(func=cmd_specs)

    cm = sub.add_parser(
        "coverage-map",
        help="plan-memo §3 Spec coverage map skeleton + breadth verdict",
        description=(
            "Generate plan-memo §3 Spec coverage map starter rows from a list of "
            "(spec, ref) pairs. `ref` is a §number (e.g. 15.7.14) for any spec, "
            "or an abstract-operation name (e.g. ClassDefinitionEvaluation) for "
            "tc39 specs. Each pair is verified via webref / tc39-biblio. Prints "
            "the verified table + anchors + breadth count + split-decision "
            "verdict per feedback_plan-scope-re-evaluation.md."
        ),
    )
    cm.add_argument(
        "pairs", nargs="+",
        metavar="<spec> <ref>",
        help="pairs of spec shortname + §number-or-AO-name (even count)",
    )
    cm.set_defaults(func=cmd_coverage_map)

    return p


def main() -> None:
    args = build_parser().parse_args()
    try:
        args.func(args)
    except NotFound as e:
        # Safety net for the few raw `fetch_json` paths that let NotFound
        # bubble (the `specs` keyword search + the lazy catalog load in
        # `_data_index`, both hitting `index.json`). Per-spec data-kind
        # fetches go through `fetch_data`, which exits with its own
        # diagnostic. Convert here to a clean CLI exit instead of a traceback.
        sys.exit(f"webref: fetch failed: {e}")
    except json.JSONDecodeError as e:
        sys.exit(f"webref: invalid JSON from upstream: {e}")
    except KeyboardInterrupt:
        sys.exit(130)
    except BrokenPipeError:
        # Common when piping to `head` — exit quietly.
        sys.exit(0)
