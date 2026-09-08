#!/usr/bin/env python3
"""ONE destination -> sibling resolver -- for `plan-memo-umbrella-check.py`.

`sibling_path` answers ONE question, and it is the only place this checker
decides that a link names a memo it must go and scan: does this link
destination name a memo on disk beside this one, under one
platform-independent reading?  Its stages (a)-(e) are its own docstring, in
spec order, and the path machinery below is what they read -- the WHATWG URL
scheme class (`_SCHEME`), the C0 / DEL class (`_CONTROL`), the DOS device
names and the characters Windows does not read as letters of a name
(`_DEVICE_NAMES` / `_RESERVED_CHARS` / `_is_reserved_component`), and the ONE
guarded `resolve()` (`_resolve`, which the population reads too).  `directory`
is the LINKING memo's directory: "beside this memo" in the stages below is
that directory, and nothing here knows what a memo is.

That is a different subject from `plan_memo_memo.py`'s `Memo`, which is one
document's block structure and lexing; it imports this module (`linked_files`
/ `unresolved_references` are its consumers of the resolver) and this one
never imports it.  The controls are `plan_memo_selftest_cases_sibling.py`'s,
carved on the same subject.
"""

import pathlib
import re
from urllib.parse import unquote

from plan_memo_lexer import FILE_SUFFIX


def sibling_path(directory, dest):
    """The ONE destination -> sibling mapping: the memo on disk a link
    destination names, or None when it names none.  POLICY (CommonMark
    §6.3 / GFM say nothing about siblings on disk): a sibling is a
    RELATIVE `.md` path beside this memo.  Stages, in spec order:
      (a) the scheme test on the RAW path component -- WHATWG URL §4.4
          "URL parsing", the basic URL parser's *scheme start state*
          (https://url.spec.whatwg.org/#scheme-start-state, step 1: "If
          c is an ASCII alpha, append c, lowercased, to buffer, and set
          state to scheme state") and *scheme state*
          (https://url.spec.whatwg.org/#scheme-state, step 1: "If c is
          an ASCII alphanumeric, U+002B (+), U+002D (-), or U+002E (.),
          append c, lowercased, to buffer"; step 2: "Otherwise, if c is
          U+003A (:)" -- the scheme is set) read the input's code points
          AS WRITTEN; `%` is in neither class, so at `%` the parser
          leaves for the *no scheme state* and `notes%3Achild.md` has no
          scheme (`_SCHEME` is that class and that terminator).
          Percent-decoding is no step of the parser at all: the *path
          state* (https://url.spec.whatwg.org/#path-state)
          percent-ENCODES and keeps `%xx` as written.
          ⚠ SINCE PR #510 R25-1 this stage decides nothing (c) would not
          also decide: every scheme ends in a `:`, decoding never removes
          one, and a `:` anywhere in a decoded COMPONENT is refused at
          (c) -- so `https://guide.md` is no sibling by BOTH readings,
          and no destination separates them.  MEASURED, and the PROBE is
          stated rather than a stored number: run `sibling_path` under
          the three readings -- as written, `if False` here, and
          `_SCHEME.match(unquote(raw))` here -- over `<scheme>:<body>`
          for the schemes `http` / `https` / `file` / `mailto` /
          `a+b-c.d` / `x1` / `HTTPS` / `c` / `n` and the bodies
          `//host/x.md`, `///x.md`, `/x.md`, `x.md`, `sub/x.md`,
          `sub\\x.md`, the empty one and `x.md?q#f`, each in five
          encodings (raw, `quote`d, the colon percent-encoded, the scheme
          percent-encoded, both) plus the relative controls, from ONE
          fixture directory so a temporary path is not the difference; at
          R25-1 that was 367 destinations and the three readings agreed
          on all of them.  The stage is KEPT because it is the URL
          standard's own question, asked before the path question and
          answering it for the right reason, not because a case
          exercises it; the two mutants that used to witness it (the
          stage dropped, and the scheme read after decoding) are DELETED
          as equivalent in `plan_memo_selftest_mutants.py` /
          `_mutants_pr510.py`, each with that reasoning where the row
          stood.  If you find a destination the scheme test refuses and
          stage (c) admits, it belongs beside this stage as a control and
          those rows come back with it;
      (b) percent-decode (`slice%20sib.md` is `slice sib.md`, as
          `<slice sib.md>` is) -- WHATWG URL §1.3 "Percent-encoded
          bytes", *percent-decode* on a string
          (https://url.spec.whatwg.org/#string-percent-decode: "Let bytes
          be the UTF-8 encoding of input. Return the percent-decoding of
          bytes"), the operation a consumer applies to a parsed path;
          `urllib.parse.unquote` is that operation;
      (c) the DECODED name must be RELATIVE on every platform, and hold
          no C0 control / DEL (`child%00.md` would make `resolve()`
          raise).  ONE platform-independent reading of the name:
          Windows path syntax (`pathlib.PureWindowsPath`), the superset
          -- `/` and `\\` both separate, and a drive letter (`C:`), a UNC
          prefix (`\\\\server\\share`) or a root (`/`, `\\`) ANCHORS.  It
          is the URL standard's own reading of a special-scheme path
          (`file` is a special scheme): *path state*
          (https://url.spec.whatwg.org/#path-state) step 1 ends a
          segment at "U+002F (/)" or, "url is special and c is U+005C
          (\\)", at a backslash (with an invalid-reverse-solidus
          validation error), and step 1.4.1's Windows drive letter rule
          is, in the spec's words, "a (platform-independent) Windows
          drive letter quirk".  So `PureWindowsPath(name).anchor` must
          be empty: `/x`, `//host/x` (a site URL joined to the memo's
          directory would probe the host's filesystem root), `\\x`,
          `C:\\temp\\x`, `\\\\server\\share\\x` and the drive-relative
          `C:x` (raw `C:x.md` is already a URL of scheme `c` at (a);
          percent-encoded `C%3Ax.md` decodes to a drive anchor here --
          so does the one-letter `n%3Ax.md`, where the multi-letter
          `notes%3Ax.md` of (a) is a file name) are all rejected.  ⚠
          Until PR #510 R19 this stage rejected a leading `/` only, so
          `C%3A%5Ctemp%5Cchild.md` (`C:\\temp\\child.md`) and
          `%5Cchild.md` (`\\child.md`) passed, and on Windows `parent /
          name` discarded the memo's directory.  An empty anchor is not
          enough: a RELATIVE name whose component is a DOS device, holds
          a character Windows does not read as a letter of a name, or
          ends in a dot or a space is no file beside the memo either
          (`_is_reserved_component`, per PART, so `dir/NUL.md` and
          `NUL/child.md` go too).  ⚠ Until PR #510 R22 the device and
          trailing-run halves passed, and until R25-1 the CHARACTER half
          did; all three fail SILENTLY where an anchor does not: on
          Windows reading `NUL.md` SUCCEEDS and yields an empty
          stream, so the population counted an empty linked memo and
          could exit 0 having omitted the sibling the author linked;
          `notes:child.md` names an NTFS alternate data stream, and
          where one exists reading it succeeds and yields ANOTHER file's
          content, so the population would scan text no memo holds;
          and `dir /child.md` reads a different directory there than
          here.  The cost, named: a POSIX memo genuinely called
          `NUL.md` or `notes:child.md` is now not a sibling either --
          dropped without a report, exactly as `/abs/x.md` and
          `C:\\x.md` already are, which is this stage's standing
          polarity;
      (d) the `.md` suffix -- the lexer's `FILE_SUFFIX`, the ONE
          spelling of "is a file name" (the lexer's bare file token
          reads the same constant over prose); the stem is unconstrained
          on both sides, so `.md` alone is a sibling file (PR #510 R20);
      (e) the name's PARTS joined beside the memo -- the same Windows
          syntax, so `sub%5Cchild.md` is the sibling `sub/child.md` on
          POSIX as on Windows (never the POSIX file named `sub\\child.md`:
          a backslash is a separator everywhere, never a name character)
          -- then `resolve()` (`_resolve`); an `OSError` or -- on Python
          3.9-3.12, for a symlink loop -- a `RuntimeError` there makes
          the sibling UNAVAILABLE: the joined, unresolved path is
          returned and the population's one I/O chokepoint reports it as
          an unavailable linked memo (exit 2), never a crash, never a
          silent drop.
    """
    raw = re.split(r"[#?]", dest, 1)[0]
    if _SCHEME.match(raw):                                       # (a)
        return None
    name = unquote(raw)                                          # (b)
    p = pathlib.PureWindowsPath(name)
    if (_CONTROL.search(name) or p.anchor                        # (c)
            or any(_is_reserved_component(s) for s in p.parts)):
        return None
    if not name.endswith(FILE_SUFFIX):                           # (d)
        return None
    return _resolve(directory.joinpath(*p.parts))                # (e)


_SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def _resolve(path):
    """`path.resolve()`, or `path` itself when resolving raises -- `OSError`
    (an over-long name) or, on Python 3.9-3.12, `RuntimeError` for a symlink
    loop (3.13 made that an `OSError`).  The ONE site that guards it; the
    unresolved path then reaches the population's I/O chokepoint as an
    unavailable memo."""
    try:
        return path.resolve()
    except (OSError, RuntimeError):
        return path
_CONTROL = re.compile(r"[\x00-\x1f\x7f]")

# The DOS DEVICE names -- Microsoft, "Naming Files, Paths, and Namespaces"
# (https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file):
# "Do not use the following reserved names for the name of a file", and the
# rule holds "regardless of the file extension" and in any directory.  The
# set is CPython `ntpath._reserved_names`' reading of that paragraph, spelled
# here as data rather than called: `PureWindowsPath.is_reserved()` is
# deprecated in 3.13 and removed in 3.15 (it raises a DeprecationWarning on
# the 3.14 this runs on), and `os.path.isreserved` exists only on 3.13+ and
# only in `ntpath` -- on POSIX `os.path` IS `posixpath` and has no such
# function, so neither is available to a checker that must decide this the
# same way on every platform.
_DEVICE_NAMES = frozenset(
    {"AUX", "CON", "CONIN$", "CONOUT$", "NUL", "PRN"}
    | {p + n for p in ("COM", "LPT") for n in "123456789¹²³"})


# The characters Windows does not read as letters of a name -- Microsoft,
# "Naming Files, Paths, and Namespaces" (the source `_DEVICE_NAMES` above
# cites), "Use any character in the current code page for a name, except:"
# `< > : " / \ | ? *` -- MINUS what the stages above already own: the ASCII
# controls are stage (c)'s `_CONTROL` (wider: it holds DEL too), and `/` and
# `\` are SEPARATORS to the `PureWindowsPath` parse that produced these parts,
# so neither can stand inside a component.  CPython spells the same list as
# `ntpath._reserved_chars`.  What each one DOES there is a different question,
# and only one of them is settled from here: `:` opens an NTFS ALTERNATE DATA
# STREAM (`notes:child.md` is the stream `child.md` of the file `notes`), which
# is why R25 found it; for the other six, see `_is_reserved_component`.
_RESERVED_CHARS = frozenset('*?"<>:|')


def _is_reserved_component(part):
    """Whether one already-parsed `PureWindowsPath` component is a name
    Windows does NOT resolve to a file of that name beside its parent --
    pure string logic, so it is decided identically on POSIX and testable
    there.  The WHOLE of CPython `ntpath._isreservedname`'s reading, minus
    the two halves other stages own (`_RESERVED_CHARS` above), in its order:

      * a component ending in `.` or ` ` (the components `.` and `..`
        excepted): Windows strips the trailing run, so `dir /child.md`
        reads `dir\\child.md` there and a different directory here;
      * a component holding a `_RESERVED_CHARS` character;
      * otherwise the stem before the first `.`, its trailing spaces
        stripped, upper-cased, is a DOS device (`_DEVICE_NAMES`): `NUL.md`,
        `dir/NUL.md`, `nul.md` and `prn .md` all resolve to a device.

    THE PREDICATE IS DRAWN ON STAGE (c)'s POLICY, NOT ON A FAILURE MODE (PR
    #510 R25-1).  Stage (c) asks for ONE platform-independent reading of the
    name: a sibling is a relative `.md` file beside this memo, decided
    identically on every platform.  A component holding one of these seven is
    not that, and the two reasons are different -- `:` is Windows path SYNTAX
    (`PureWindowsPath` models its drive meaning and not its stream meaning, so
    `notes:child.md` comes back with an empty anchor and one part), while the
    other six are characters that platform's API will not put in a name at all
    -- but the POLICY answers both at once: whether the name denotes a file
    beside the memo must not depend on where the checker runs.

    ⚠ THIS REVERSES the reading of `notes%3Achild.md` as the local file
    `notes:child.md` (PR #510 R8), and the cost is the standing polarity of
    this stage: a POSIX memo genuinely named `notes:child.md` -- or holding
    any of the other six -- is no longer a sibling, dropped without a report,
    exactly as `/abs/x.md`, `C:\\x.md`, `sub\\child.md` and `NUL.md` already
    are.  What R8 decided about stage (a) STANDS: the scheme test still reads
    the RAW component, `%` is in neither of the URL parser's classes, and
    `notes%3Achild.md` still has no scheme.  It is stage (c) that now refuses
    it, and for the path reason rather than the URL one.

    ⚠ AND THE ARGUMENT R25 FALSIFIED, named so it is not written again: until
    R25 this docstring left `*?"<>:|` to stage (e) "because on Windows a name
    holding one raises `OSError` there -- reported as an unavailable linked
    memo, exit 2".  That is FALSE for `:`: on Windows `notes:child.md` names
    an alternate data stream, and where that stream exists opening it
    SUCCEEDS, so the population would scan the contents of an unrelated file's
    stream and could exit 0 -- the same "clean exit for content that could not
    be scanned" §1 forbids, which is the class the device names were rejected
    for.  For the other six the honest position is that THIS TREE CANNOT
    DETERMINE IT: the checker runs on POSIX, no Windows is reachable from
    here, and the claim that they raise is the same unverified claim about a
    platform that has just been falsified once.  So they are decided by the
    policy above and not by their failure mode -- and as a class, because an
    enumerated exemption leaves its next member authoritative by default."""
    if part[-1:] in (".", " "):
        return part not in (".", "..")
    if _RESERVED_CHARS.intersection(part):
        return True
    return part.partition(".")[0].rstrip(" ").upper() in _DEVICE_NAMES
