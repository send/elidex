#!/usr/bin/env python3
"""PR #510 Codex R1-R16 mutants for `plan-memo-umbrella-check.py --self-test --mutants`.

The second of three mutant modules, split from `plan_memo_selftest_mutants.py`
at the review-round seam the control registry is split at
(`plan_memo_selftest_cases.py` / `_cases_pr510.py` / `_cases_inline.py`): that
module holds the row shape, the runner `run` and every PRE-converge mutant (the
lexing clauses, the gating stages, the `/code-review high` and `/elidex-review`
Stage-6 fixes); this one holds the mutants of PR #510 review rounds R1-R16 and
the design re-gates over R4-R9 -- the LEXICAL SUBSTRATE and the BLOCK grammar --
and `plan_memo_selftest_mutants_inline.py` those of R17 on, the rounds that
closed the Phase-2 INLINE construct family.  The seam is the cases modules'
exactly: `_cases_pr510.py` here, `_cases_inline.py` there.  All three append to
the SAME `MUTANTS` list -- one registry, read at one import site (the runner).
The rules are the first module's: the substring must occur EXACTLY ONCE in its
file, every named control must go red, a crash is a FAIL.  A mutant's control
lives in `plan_memo_selftest_cases_pr510.py` under the same round label.
"""

from plan_memo_selftest_mutants import (
    BLOCKS, CHECK, CONTROLS, IDS, LEXER, MEMO, MUTANTS, ROLES, SEQUENCE, SPEC_EXAMPLES,
    TABLES,
)

# The pre-mask two mutants below re-inject.  It is spelled HERE, in the mutant
# registry, because the checker has no such function any more: since design
# re-gate 4 `stream` RENDERS a block (dropping, blanking or substituting each
# span as §3.0b's `Renders` column says) instead of blanking one, and a
# blank-everything helper left in the lexer for these two rows alone would be
# production dead code -- exactly what the plan forbids.  A mutant may spell
# the defect it re-injects; the subject may not keep it around.
BLANK = ('lambda t, sp: "".join(" " if any(a <= k < b for a, b in sp) and c != "\\n" else c '
         'for k, c in enumerate(t))')


MUTANTS += [
    # -- PR #510 Codex R1
    ("R1-1 row: only an ODD backslash run escapes `|` (even run = literal backslash + pipe)", BLOCKS,
     '        if _escaped(line, i):\n            breaks[-1].append(i - 1)',
     '        if i > 0 and line[i - 1] == "\\\\":\n            breaks[-1].append(i - 1)',
     ["(row) `a\\\\|b` holds an UNESCAPED pipe (§2.4: `\\\\` is a literal backslash): 5 cells "
      "under a 4-cell header is a width miss, rc 2"]),
    ("R1-1 row: the trailing-pipe check uses the same parity", BLOCKS,
     'if stripped.endswith("|") and bounds and not _escaped(stripped, len(stripped) - 1):',
     'if stripped.endswith("|") and bounds and not stripped.endswith("\\\\|"):',
     ["(row) a trailing `\\\\|` is a literal backslash then the trailing pipe: rc 0"]),
    ("R1-2 runner: the emptiness guard fires on zero controls / zero mutants", CONTROLS,
     '    if n_controls == 0:\n        out.append(', '    if False:\n        out.append(',
     ["an empty control or mutant registry is a FAIL, never green"]),
    ("R1-3 link: a link deactivates every `[` opener before it (links may not contain links)", LEXER,
     '                if not opener[1]:\n                    opener[2] = False',
     '                if False:\n                    opener[2] = False',
     ["(link) nested inline links: the INNER link is the link, the outer tail is text -- "
      "`child.md` joins the population, absent `parent.md` is not linked",
      "(link) a reference link nested in inline brackets: the inner reference is the link, the "
      "outer tail is text"]),
    ("R1-4 def: an orphan candidate is parsed with its continuation line, not per line", MEMO,
     '            text, off = "\\n".join(lines[i:j]), 0',
     '            text, off = "\\n".join(lines[i:i + 1]), 0',
     ["(def) a would-be MULTILINE definition that interrupts a paragraph is an orphan: the "
      "shortcut naming it is a schema miss, not an exempt citation-style shortcut"]),
    # -- PR #510 Codex R2
    ("R2-1 def: the next-line title is tried before the destination-only ending", BLOCKS,
     '        t = link_title(s, k2) if k2 > k else None', '        t = None',
     ["(def) a next-line title is part of the definition, not prose: an id in it is no site",
      "(def) a next-line title holding `[x](missing.md)` is a title, not a link: rc 0"]),
    ("R2-2 link: `![` opens an image, which is not a link", LEXER,
     '    return i > 0 and s[i - 1] == "!" and not _escaped(s, i - 1)', '    return False',
     ["(link) a link wrapping an IMAGE `[![alt](img.png)](sib.md)` links the sibling; `img.png` is "
      "never a memo"]),
    ("R2-3 link: the reference form is the lexer's (re-inject a raw bracket walk)", MEMO,
     'or form == "shortcut"',
     'or "][" not in lx.text[off:lx.text.find("]", off) + 2]',
     ["(link) `[foo\\]][missing]` is a FULL reference (the `]` is escaped): a schema miss, not an "
      "exempt shortcut"]),
    # -- PR #510 Codex R3: "Appendix: A parsing strategy" bracket stack
    ("R3-1 link: an IMAGE does not deactivate the openers before it (deactivate on image)", LEXER,
     '            for opener in stack:        # links may not contain links\n                if not opener[1]:',
     '        if True:\n            for opener in stack:\n                if not opener[1]:',
     ["(link) a link wrapping a REFERENCE image `[![alt][img]](child.md)`: the image does not "
      "deactivate the outer opener, so `child.md` is scanned",
      "(link) a link wrapping an IMAGE `[![alt](img.png)](sib.md)` links the sibling; `img.png` is "
      "never a memo"]),
    ("R3-1 link: one pass, no recursive inner re-parse (re-inject one: exponential)", LEXER,
     '        if not active:\n            i += 1                      # literal `]`; the opener is gone',
     '        if not active or inline_pass(s[pos + 1:i], defs)[3] is None:\n            i += 1',
     ["links() is linear: 30 nested brackets are one inline_pass call"]),
    ("R3-1 link: a consumed image tail is masked and not re-read", LEXER,
     '            images.append((i, end, "image"))',
     '            images.append((i, end, "image"))\n            i += 1\n            continue',
     ["(image) `![alt][img]` with a definition is consumed whole: `[img]` is not re-read as a "
      "shortcut, and the image destination is not a memo"]),
    ("R3-1 link: an escaped `[` is not an opener", LEXER,
     '        if _is_escape(s, i):\n            subst.append((i, i + 2, s[i + 1]))  # §2.4: `\\[` renders the character alone',
     '        if _is_escape(s, i) and s[i + 1] != "[":\n            subst.append((i, i + 2, s[i + 1]))',
     ["(link) an escaped `\\[` opens nothing: `\\[x](absent-file.md)` is not a link, rc 0"]),
    ("R3-2 / R5-2 population: a `/`-leading path -- raw `/x.md`, `//host/x.md`, or DECODED "
     "`%2Ftmp%2Fx.md` -- is not a sibling (drop the anchor test)", MEMO,
     '        if _CONTROL.search(name) or p.anchor:                        # (c)',
     '        if _CONTROL.search(name):                                    # (c)',
     ["(rc) a root-relative `/guide.md` is not a sibling on disk (nothing probed): rc 0",
      "(rc) a protocol-relative `//host/x.md` is not a sibling on disk: rc 0",
      "(rc) a percent-encoded ABSOLUTE destination `%2Ftmp%2Fchild.md` is rejected after decoding "
      "(never probes `/tmp/child.md`): rc 0"]),
    # -- design re-gate
    ("RG-1 def: orphan detection joins each run ONCE (re-inject a per-line join of the rest: "
     "quadratic)", MEMO,
     '                defs_at[i] = definition_block(run_text[i], run_off[i])',
     '                defs_at[i] = (__import__("plan_memo_blocks").reference_definitions('
     '"\\n".join(lines[i:]))[0] or [None])[0]',
     ["Phase-1 orphan detection is linear: <= 4 link_label calls per line"]),
    ("RG-3 link: one label grammar -- a collapsed / shortcut text is a label iff `link_label` "
     "reads it from the opener", LEXER,
     '    raw, _ = link_label(s, opener)\n    if raw is None:', '    raw = s[opener + 1:close]\n    if False:',
     ["(link) bracket text holding unescaped brackets is not a label (§6.3), so `[the [x] walk][]` "
      "is no collapsed reference: rc 0"]),
    # -- PR #510 Codex R4
    ("R4-1 gate: an unresolved IMAGE reference is not a memo miss", MEMO,
     '                if is_image:\n                    continue', '                if False:\n                    continue',
     ["(image) an undefined reference image `![diagram][missing-image]` is literal syntax, not an "
      "unresolved memo reference: rc 0"]),
    ("R4-2 population: the destination path is percent-decoded", MEMO,
     '        name = unquote(raw)                                          # (b)',
     '        name = raw                                                   # (b)',
     ["(link) a percent-encoded destination `slice%20sib.md` links the file `slice sib.md`, as "
      "`<slice sib.md>` does"]),
    ("R4-3 pass: one inline pass over the RAW text (re-introduce the code pre-mask)", LEXER,
     '= inline_pass(self.text, defs)',
     '= inline_pass((%s)(self.text, [(m.start(), m.end()) '
     'for m in re.finditer(r"`[^`]*`", self.text)]), defs)' % BLANK,
     ["(span) a backtick inside a link DESTINATION is consumed by the link, not a code span: "
      "`[sib](slice`x`.md)` links the sibling"]),
    ("R4-3 pass: a code span swallows a `]` (brackets inside it are not delimiters)", LEXER,
     '                code.append((i, close))\n                i = close',
     '                code.append((i, close))\n                i += 1',
     ["(span) a backtick BEFORE the `]` opens a code span that swallows it: `[not a "
      "`link](absent.md)`` is code, no link, rc 0",
      "(link) a link inside a code span is not a link (A x B)",
      "(rc) a code-quoted link to an absent file is not a link: rc 0"]),
    # -- PR #510 Codex R5: Phase 1 / Phase 2
    ("R5-1 phase 1: definitions are read from RAW lines (re-inject the inline pre-mask)", BLOCKS,
     '    defs, _ = reference_definitions(block, limit=1, start=off)',
     '    defs, _ = reference_definitions((%s)(block, __import__("plan_memo_lexer").inline_pass(block, {})[0]), limit=1, start=off)' % BLANK,
     ["(def) a definition is read from RAW lines at a block start: `[sib]: slice`x`.md` keeps its "
      "backticks in the destination and the sibling is scanned"]),
    ("R5-3 disposition: a slug is atomic in an id-only run (re-inject the hyphen split)", TABLES,
     '|-]+)" % (SLUG_ID, CITE_ID, SHORT_ID),', '|-]+)" % (SHORT_ID, CITE_ID, SHORT_ID),',
     ["(span) a `#11-` slug is ATOMIC in an id-only run: `` `#11-zz-alpha / 9z` `` is the document "
      "spelling two ids, both reported"]),
    # -- PR #510 Codex R6
    ("R6-2 locate: a bisect over the line offsets, not a linear scan per site", MEMO,
     '        k = bisect.bisect_right(self.offsets, i) - 1',
     '        k = 0\n        while k + 1 < len(self.offsets) and self.offsets[k + 1] <= i:\n            k += 1',
     ["unresolved_references is linear: <= N*(log2 N + 2) reads of the line-offset table (a bisect per site, not a scan)"]),
    # -- PR #510 Codex R7
    ("R7-1 def: the definition is parsed over the rest of the block (re-inject a 3-line window)", MEMO,
     '                text, off = "\\n".join(lines[i:j]), 0',
     '                text, off = "\\n".join(lines[i:i + 3]), 0',
     ["(def) a label spanning FIVE lines is a definition (§4.7 / §6.3: a label may span lines); "
      "the later shortcut resolves and the sibling is scanned"]),
    ("R7-3 def: a title may not cross a blank line (re-inject the blank into the run)", BLOCKS,
     '    while j < len(lines) and not block_end(lines, j, True, lazy):\n        j += 1',
     '    while j < len(lines) and (is_blank(lines[j]) or not block_end(lines, j, True, lazy)):\n        j += 1',
     ["(def) `[sib]: child.md \"title` whose title crosses a BLANK line is not a definition "
      "(commonmark.js: a paragraph): `[sib]` later is an exempt shortcut, rc 0, and the sibling is "
      "NOT walked -- its violation is not reported"]),
    ("R7-2 population: a C0 control character in a decoded destination is rejected", MEMO,
     '        if _CONTROL.search(name) or p.anchor:                        # (c)',
     '        if p.anchor:                                                 # (c)',
     ["a decoded destination with a C0 control character is rejected, never resolved"]),
    # -- PR #510 Codex R8
    ("R8-1 sibling: the scheme is read on the RAW path, before decoding (re-inject scheme-after-decode)", MEMO,
     '        if _SCHEME.match(raw):                                       # (a)',
     '        if _SCHEME.match(unquote(raw)):                              # (a)',
     ["(link) `notes%3Achild.md` has no scheme (WHATWG URL §4.4 #scheme-start-state / #scheme-state read the "
      "input as written and `%` is in neither class; #string-percent-decode is a later, separate operation): "
      "it is the local file `notes:child.md`, and it is scanned"]),
    ("R8-2 sibling: an OSError from resolve() is the unavailable-sibling miss (unguard it)", MEMO,
     '    try:\n        return path.resolve()\n    except (OSError, RuntimeError):\n        return path',
     '    return path.resolve()',
     ["an OSError from resolve() is the unavailable-sibling schema miss, never an exception"]),
    ("R8-5 sibling: the dedup is a set (re-inject the list membership test)", MEMO,
     '                if f is not None and f not in seen:\n                    seen.add(f)',
     '                if f is not None and f not in out:\n                    pass',
     ["linked_files is linear: <= N Path.__eq__ calls over N distinct siblings (a set dedup hashes, a list compares)"]),
    ("R8-3 bare id: the far side of a `.` is the ASCII id class, not `str.isalnum`", IDS,
     'bool(cont.match(text[j]))', 'text[j].isalnum()',
     ["(bare) `9z.次の工程` bounds the id: the far side of the `.` is not an ASCII id character, so "
      "the site is reported",
      "(bare) `9z.é` bounds the id (a dotted number is ASCII on both sides)"]),
    ("R8-4 row: edge pipes are detected with the space/tab class", BLOCKS,
     '    stripped = line.strip(" \\t")    # the same space/tab class as cell trimming',
     '    stripped = line.strip()',
     ["(table) a row opening with an NBSP before its `|` is not edge-piped: the NBSP is a cell, the "
      "header is 7 wide over a 6-cell delimiter, no table"]),
    ("R8 sweep: a blank line is spaces or tabs only (§2.1)", BLOCKS,
     '    return not line.strip(" \\t")', '    return not line.strip()',
     ["(span) an NBSP-only line is NOT blank (§4.9: spaces or tabs only), so it does not end the "
      "paragraph and the code span crosses it"]),
    # -- PR #510 Codex R9
    ("R9 F1 setext: the underline closes the paragraph (re-inject the join: drop the `block_end` arm, so "
     "the run reads on through `===`)", BLOCKS,
     'or (para_open and not _is_lazy(lazy, i) and setext_underline(lines[i]) is not None)',
     'or False',
     ["(setext) `Heading\\n===` is a heading; the `===` underline ends the paragraph, so a code "
      "span opened in the heading does not reach the next paragraph's site"]),
    # ⚠ The former row "R9 F1 setext: not after a list item or `>` line (Examples 92-94)" (drop
    # `container_text`) is DELETED with that predicate: since R15 a list item is a container, so
    # `- a `x\n==` is the item's paragraph and its lazy candidate `==`, and the underline is
    # refused by `block_end`'s lazy arm -- the R13 §5.1 lazy-setext mutant below names the control.
    ("R9 F1 / R13 seed: every raw HTML-block line is recorded for the seed (re-inject 'indented only')", MEMO,
     '                    if opener[0] != "fence":', '                    if opener[0] == "indented":',
     ["(lex-seed) an HTML-block opener holding a declared id is a seed",
      "(lex-seed) `<pre>\\nSlice 9z owns it\\n</pre>`: the inner line holding the id is seeded (type 1 "
      "ends at `</pre>`)"]),
    ("R9 F1 seed: only a line holding a `|` or a declared id is reported", CHECK,
     '            if "|" in line or ids:', '            if True:',
     ["(lex-seed) an HTML-block line with neither a `|` nor a declared id is no seed"]),
    ("R9 F2 I/O: a decode error is the unavailable-memo miss (unguard it)", MEMO,
     '            except (OSError, UnicodeDecodeError) as e:', '            except OSError as e:',
     ["an undecodable sibling is the unavailable-linked-memo schema miss, never an exception"]),
    ("R9 F3 ascii: the row-noun anchor is an ASCII class (re-inject `\\b`)", ROLES,
     'NOUN_ANCHOR = re.compile(r"(?<!%s)%s" % (ALNUM, ROW_NOUN_SEP))',
     'NOUN_ANCHOR = re.compile(r"\\b%s" % ROW_NOUN_SEP)',
     ["(ascii) `次のSlice Cが所有する` reaches the naming worklist: the row-noun anchor is not `\\b` "
      "(no Unicode word boundary before `Slice`)"]),
    ("R9 F3 ascii: the slug boundary is an ASCII class (re-inject `\\w`)", IDS,
     '    "slug": re.compile("[%s_-]" % ALNUM_CHARS),', '    "slug": re.compile(r"[\\w-]"),',
     ["(ascii) `次は#11-zz-alphaが所有する` reaches the naming worklist: the slug anchor is an ASCII "
      "class, not `\\w`"]),
    ("R9 F3 ascii: list markers are ASCII digits (re-inject `\\d`)", BLOCKS,
     '(?P<num>[0-9]{1,9})', '(?P<num>\\d{1,9})',
     ["(ascii) `١.` (an Arabic-Indic digit) is not a list marker (§5.2: ASCII digits)"]),
    ("R9 #3 row: breaks are partitioned in the one scan (re-inject the per-cell filter)", BLOCKS,
     '        out.append(_cell(line, a, b, cell_breaks))',
     '        out.append(_cell(line, a, b, [x for bs in breaks for x in bs if a <= x < b]))',
     ["split_row is linear: <= 64 source lines per character and per cell (breaks partitioned in the one scan)"]),
    # -- design re-gate R4-R9
    ("RG2 IMP-1: run_end reads the ONE predicate (re-inject raw/blank-only run ends)", BLOCKS,
     '    while j < len(lines) and not block_end(lines, j, True, lazy):\n        j += 1',
     '    while j < len(lines) and not (raw_opener(lines[j], True) is not None or is_blank(lines[j])):\n        j += 1',
     ["(block) `[Slice 9z owns it]:\\n---` is a setext heading (`<h2>…</h2>`), not a definition: the "
      "label line is paragraph text and its site is reported",
      "(block) `[Slice 9z owns it]:\\n#` is a paragraph and an ATX heading, not a definition",
      "(block) `[Slice 9z owns it]:\\n>` is a paragraph and a block quote, not a definition",
      "(block) `[Slice 9z owns it]:\\n***` is a paragraph and a thematic break, not a definition"]),
    ("RG2 IMP-1: a table header is a block end (local policy; re-inject the pure-CommonMark reading)", BLOCKS,
     '            or table_header_at(lines, i, lazy))',
     '            or False)',
     ["(block) `[Slice 9z owns it]:\\n|a|b|\\n|--|--|` -- the table header ends the run (local policy "
      "over pure CommonMark, which has no tables): a paragraph and a table, not a definition with the "
      "header row as its destination"]),
    ("RG2 IMP-1: admit_table reads the ONE predicate (re-inject a third boundary)", TABLES,
     '    while j < n and not block_end(lines, j, False, lazy):\n        body = split_row(lines[j])',
     '    while j < n and not __import__("plan_memo_blocks").is_blank(lines[j]):\n        body = split_row(lines[j])',
     ["(table) a list item right after a schema table ends it (a block start), so it is not a 1-cell "
      "body row: rc 0"]),
    ("RG2 IMP-1 / R13 §4.4: indented code cannot interrupt a paragraph (re-inject it as an opener "
     "with one open)", BLOCKS,
     '    if not para_open and is_indented(line):\n        return "indented", None',
     '    if is_indented(line):\n        return "indented", None',
     ["(block) `[Slice 9z owns it]:\\n    code` IS a definition (§4.4: indented code cannot interrupt "
      "a paragraph; commonmark.js: destination `code`): a block of its own, not scanned",
      "(indented) `text\\n    Slice 9z owns it`: an indented line cannot interrupt a paragraph (§4.4; "
      "commonmark.js: one paragraph) -- paragraph text, the site is reported",
      SPEC_EXAMPLES]),
    ("RG2 IMP-2: an orphan is a VALID definition off a block start (re-inject the label-colon shape, "
     "which blocked the shortcut by shape alone -- as if it named a sibling)", MEMO,
     '            if d is not None:\n                # a valid definition that cannot take effect: the orphan, keyed\n'
     '                # on its label\'s `[` (§4.7: after <=3 columns of indentation),\n'
     '                # its destination kept\n'
     '                self.orphans.setdefault(normalize_label(d[0]), []).append((linenos[i], indentation(line)[1], d[1]))',
     '            if d is not None or (line.lstrip(" ").startswith("[") and "]:" in line):\n'
     '                self.orphans.setdefault(normalize_label(d[0] if d else line.split("]:")[0].lstrip(" [")), []).append((linenos[i], indentation(line)[1], d[1] if d else "orphan.md"))',
     ["(cite) `[C1]: ECMA-262 §1 says so, and the table cites it.` at a block start is prose "
      "(commonmark.js: a paragraph), and the citation shortcut stays exempt: rc 0",
      "(def) a label-and-colon line that is NOT a valid definition (junk after the destination) at a "
      "block start is prose, not an orphan: `[sib]` later is exempt, rc 0"]),
    ("RG2 IMP-3: RuntimeError from resolve() is guarded with OSError", MEMO,
     '    except (OSError, RuntimeError):\n        return path', '    except OSError:\n        return path',
     ["an OSError from resolve() is the unavailable-sibling schema miss, never an exception"]),
    ("RG2 MIN-1: every line of an HTML block is seeded to its end condition", BLOCKS,
     '    if html_block_ends(arg, lines[i]):      # the opener may meet the end condition itself\n        return i + 1',
     '    if True:\n        return i + 1',
     ["(lex-seed) `<pre>\\nSlice 9z owns it\\n</pre>`: the inner line holding the id is seeded (type 1 "
      "ends at `</pre>`)",
      "(lex-seed) `<!-- c\\n|9z|\\n-->`: a `|` line inside a comment block (type 2 ends at `-->`) is "
      "seeded"]),
    ("RG2 MIN-1: a type-6 block ends at a blank line", BLOCKS,
     '        if arg in ("t6", "t7") and is_blank(lines[j]):',
     '        if False:',
     ["(lex-seed) a type-6 block ends at a blank line: the paragraph after it is not seeded"]),
    # -- PR #510 Codex R10
    ("R10-1 raw: HTML-block lines are raw extents (re-inject them into the paragraph)", BLOCKS,
     '    return "html", t', '    return None',
     ["(html) `<pre>\\n`\\n</pre>\\nSlice 9z` owns it`: the backtick inside the raw HTML block does "
      "not pair with the prose one -- the site is reported",
      "(lex-seed) an HTML-block opener holding a declared id is a seed"]),
    ("R10-1 raw: a type-7 opener counts only where no paragraph is open (make it interrupt)", BLOCKS,
     '    if t is not None and not (t == "t7" and para_open):', '    if t is not None:',
     ["(html) `text\\n<span>\\n9z owns it` -- a type-7 opener cannot interrupt a paragraph "
      "(commonmark.js): the lines stay paragraph text and the site is reported"]),
    ("R10-1 raw: the end condition on the opener line closes the block there", BLOCKS,
     '    if html_block_ends(arg, lines[i]):      # the opener may meet the end condition itself\n        return i + 1',
     '    if False:\n        return i + 1',
     ["(html) `<pre></pre>\\n9z owns it` -- the opener meets the end condition itself, so the block is "
      "that one line and the next line is prose"]),
    ("R10-2 blocks: the id cell is scanned with its own id suppressed (re-inject the skip)", CHECK,
     '                for col, cell in enumerate(row.cells):\n                    src = (',
     '                for col, cell in enumerate(row.cells):\n                    if row.schema is not None and col == row.schema.idc:\n                        continue\n                    src = (',
     ["(id) the id cell's trailing prose is scanned: `**7z** — Slice 9z lands first` reports `9z` "
      "(the row's own `7z` is suppressed)"]),
    ("R10-3 file: a bare `.md` name is a path-syntax run (re-inject the narrow class)", LEXER,
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))*%s(?!%s))',
     '|(?P<file>[\\w./-]*%s(?!%s))',
     ["(file) `9z+notes.md` is one file name: no site", "(file) `9z@notes.md` is one file name: no site",
      "(file) `(9z).md` is one file name (a balanced parenthesis pair): no site"]),
    # -- PR #510 Codex R11
    ("R11-1 driver: block start from the block STATE (re-inject the look-back at the previous raw line: "
     "blank / one-line block before it)", MEMO,
     '                opener = raw_opener(line, False)',
     '                opener = raw_opener(line, not (i == 0 or is_blank(lines[i - 1]) '
     'or one_line_block(lines[i - 1])))',
     ["(table) the prose inside the type-7 block after a schema table is raw, not a site",
      "(html) `Heading\\n===\\n<span>\\n9z owns it`: after a setext heading no paragraph is open "
      "(commonmark.js: a heading and a raw block) -- the look-back at the previous line missed this: no site"]),
    ("R11-1 table: no paragraph is open inside a table (re-inject the paragraph's interruption rule)", TABLES,
     '    while j < n and not block_end(lines, j, False, lazy):', '    while j < n and not block_end(lines, j, True, lazy):',
     ["(table) a type-7 HTML opener right after a schema table ENDS it (a table is not a paragraph): "
      "`<span>` + prose are raw lines, not one-cell rows -- no width miss, rc 0"]),
    ("R11-1 raw: a type-7 opener opens where no paragraph is open (re-inject 'never at a block start')", BLOCKS,
     '    if t is not None and not (t == "t7" and para_open):', '    if t is not None and t != "t7":',
     ["(html) `# h\\n<span>\\n9z owns it`: after a one-line block no paragraph is open, so the type-7 "
      "opener is a block start (commonmark.js: a heading and a raw block): no site",
      "(table) a type-7 HTML opener right after a schema table ENDS it (a table is not a paragraph): "
      "`<span>` + prose are raw lines, not one-cell rows -- no width miss, rc 0"]),
    ("R11-2 link: the re-scanned label of a failed full reference is the same site (re-inject the second record)", LEXER,
     '                if form is not None and not (form == "shortcut" and pos == relabel):',
     '                if form is not None:',
     ["(image) the reviewer's input `![alt][missing]` + an orphan `[missing]: image.md` in the same "
      "paragraph: the re-scanned `[missing]` is the same failed site, not a shortcut for the orphan rule "
      "-- rc 0, `image.md` never walked",
      "(def) a definition cannot interrupt a paragraph: the reference is unanswered, and reported ONCE "
      "(`[text][label]` re-scans `[label]`)"]),
    ("R11-2 link: the tail of a failed reference is NOT consumed (re-inject the advance to `end`)", LEXER,
     '                i += 1                  # literal `]`; the opener is gone; the tail is NOT consumed',
     '                i = end',
     ["(image) `![alt][missing][Slice 9z]` with `[Slice 9z]` defined: the failed image's label is "
      "re-scanned and `[missing][Slice 9z]` is a link (§6.3 Example 571; commonmark.js) -- its tail is "
      "masked, the sibling is walked: 1 site, not 2"]),
    # -- PR #510 Codex R12
    ("R12-A setext: an underline is a boundary only where a paragraph is open (re-inject the "
     "unconditional arm)", BLOCKS,
     'or (para_open and not _is_lazy(lazy, i) and setext_underline(lines[i]) is not None)',
     'or setext_underline(lines[i]) is not None',
     ["(table) `===` right after a schema table is a one-cell body row (§4.3: no paragraph to "
      "underline; GFM §4.10 Example 202: a pipe-less line is a row): width miss, rc 2"]),
    # ⚠ The former row "R12-A setext: a run headed by an indented-code line is not a paragraph an
    # underline closes (drop §4.4 from `container_text`)" is DELETED with its arm: since R13 an
    # indented line at a block start is a RAW extent (`raw_opener`), so no run is headed by one and
    # `container_text` has nothing to say about it; Example 100 (`    foo\n---`) is now guarded by
    # the §4.4 opener mutant below, which reds the conformance control at 100 among others.
    ("R12-B tabs: indentation is measured in columns, a tab to the next multiple of 4 (re-inject "
     "the four-space literal)", BLOCKS,
     '    return indentation(line)[0] >= 4 and not is_blank(line)',
     '    return line.startswith("    ") and not is_blank(line)',
     ["(indented) `\\tSlice 9z owns it`: a tab is four columns (§2.2), so the line is indented code at "
      "a block start -- a raw extent, no site",
      "(indented) ` \\tSlice 9z owns it`: a space then a tab is four columns (§2.2) -- raw, no site",
      SPEC_EXAMPLES]),
    ("R12-B tabs: a block start allows at most three columns through the one measure (re-inject "
     "'any number of spaces' in `unindented`, which every block start reads)", BLOCKS,
     '    return line[j:] if col < 4 else None', '    return line[j:]',
     [SPEC_EXAMPLES]),
    ("R12-C anchors: the anchored pass reads the disposed stream (re-inject the raw text, for the anchor "
     "and the token map alike)", CHECK,
     '    at = {t.start: t for t in b.tokens}\n    for nm in NOUN_ANCHOR.finditer(b.stream):',
     '    at = {t.start: t for t in tokens(b.text)}\n    for nm in NOUN_ANCHOR.finditer(b.text):',
     ["(anchor) `` `Slice `C owns it ``: the row noun is inside a code span, so on the disposed "
      "stream there is no `Slice C` to anchor on -- 0 sites (the bare `C` is a declared miss)"]),
    ("R12-D witness: Phase 1's `link_label` calls are counted where Phase 1 makes them (re-bind the "
     "counter to the lexer's binding, which sees only Phase 2)", CONTROLS,
     'with _count_calls(plan_memo_blocks, "link_label", limit=4 * n) as c:',
     'with _count_calls(__import__("plan_memo_lexer"), "link_label", limit=4 * n) as c:',
     ["Phase-1 orphan detection is linear: <= 4 link_label calls per line"]),
    ("R12-E conformance: the type-6 tag list is the spec's (drop `div`; `search`, the brief's "
     "example, has no spec example to exercise it)", BLOCKS,
     'details|dialog|dir|div|dl|', 'details|dialog|dir|dl|',
     [SPEC_EXAMPLES]),
    # -- PR #510 Codex R13: §4.4 indented code is a RAW extent, §5.1 block quotes are containers
    ("R13 §4.4: an indented line at a block start opens a raw extent (drop the opener arm)", BLOCKS,
     '    if not para_open and is_indented(line):\n        return "indented", None',
     '    if False:\n        return "indented", None',
     ["(indented) `    Slice 9z owns it` at a block start is a raw extent (§4.4), like a fence: no site",
      "(table) an indented line right after a schema table opens an indented code block (a block-level "
      "structure; no paragraph is open), not a one-cell row: rc 0",
      SPEC_EXAMPLES]),
    ("R13 §4.4: a blank line inside the chunk stays in it when an indented line follows (re-inject "
     "'a blank line ends it')", BLOCKS,
     '            elif not is_blank(lines[j]):\n                break',
     '            else:\n                break',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R13 §4.4: the trailing blank lines are not part of the block (re-inject them)", BLOCKS,
     '        return end\n    if kind == "fence":', '        return j\n    if kind == "fence":',
     [SEQUENCE]),
    ("R13 §5.1: a `>` line opens the container (drop the branch: the marker line heads a paragraph)", MEMO,
     '                if quote_content(line) is not None:\n                    open_block()',
     '                if False:\n                    open_block()',
     ["(quote) `> [sib]: slice-9z-sib.md`: a definition inside a block quote registers (§5.1 container, "
      "Example 218) -- the sibling is walked and its violation reported",
      "(quote) a slot table inside a block quote is a table: its row declares its id",
      SPEC_EXAMPLES]),
    ("R13 §5.1: the marker's tab is one column of marker space and the rest content indentation "
     "(re-inject one column per tab in the content's leading whitespace)", BLOCKS,
     '        col += 1 if line[j] == " " else 4 - col % 4', '        col += 1',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R13 §5.1: a setext underline is never a lazy continuation line -- but a lazy `===` is paragraph "
     "text, Example 93 (re-inject the underline on a lazy line)", BLOCKS,
     'or (para_open and not _is_lazy(lazy, i) and setext_underline(lines[i]) is not None)',
     'or (para_open and setext_underline(lines[i]) is not None)',
     ["(quote) `> Heading `open\\n===\\nSlice 9z owns it` here`: a lazy `===` is the quote paragraph's "
      "text, not an underline (§5.1, Example 93) -- one paragraph, the span masks the site",
      "(setext) `==` after a list item is NOT an underline (§4.3 Examples 92-94): the item's "
      "paragraph continues and a code span crosses it",
      SPEC_EXAMPLES]),
    ("R13 §5.1: a lazy candidate where no paragraph is open ends the quote (drop the stop: it is "
     "parsed inside; Example 237 is the spec's instance)", MEMO,
     '                if lazy is not None and lazy[i] and not ((cur or i == def_end) and table_header_at(lines, i, lazy)):\n'
     '                    break',
     '                if False:\n                    break',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R13 §5.1: a lazy candidate is a boundary wherever no paragraph is open (drop the arm: a table's "
     "or a raw extent's next line)", BLOCKS,
     '    if _is_lazy(lazy, i) and not para_open:\n        return True',
     '    if False:\n        return True',
     [SEQUENCE]),
    ("R13 §5.1: a lazy candidate ends a fence inside the quote (re-inject 'the fence runs on')", BLOCKS,
     '        while j < n and not _is_lazy(lazy, j):\n            if fence_closes(arg, lines[j]):',
     '        while j < n:\n            if fence_closes(arg, lines[j]):',
     [SEQUENCE]),
    ("R13 §5.1 / RG3 MIN-9: a lazy DELIMITER row opens no table (re-inject it)", BLOCKS,
     '            or _is_lazy(lazy, i + 1)):', '            or False):',
     [SEQUENCE,
      "(quote) a slot table inside a block quote whose DELIMITER row is lazy is one paragraph (cmark-gfm: "
      "the delimiter arrives in an unmatched container): no id declared"]),
    ("RG3 MIN-9: a lazy HEADER row is the table's header where a paragraph is open (re-inject the header "
     "arm: 'neither row may be lazy')", BLOCKS,
     '            or _is_lazy(lazy, i + 1)):', '            or _is_lazy(lazy, i) or _is_lazy(lazy, i + 1)):',
     [SEQUENCE,
      "(quote) a slot table inside a block quote whose HEADER row is lazy is a table (cmark-gfm: the header "
      "is read out of the quote paragraph's last line): its row declares its id"]),
    ("RG3 MIN-9: the driver hands a lazy header to the table instead of ending the quote (re-inject the "
     "unconditional stop)", MEMO,
     '                if lazy is not None and lazy[i] and not ((cur or i == def_end) and table_header_at(lines, i, lazy)):\n'
     '                    break',
     '                if lazy is not None and lazy[i]:\n                    break',
     [SEQUENCE,
      "(quote) a slot table inside a block quote whose HEADER row is lazy is a table (cmark-gfm: the header "
      "is read out of the quote paragraph's last line): its row declares its id"]),
    ("RG3 MIN-5: the tab after the marker gives ONE column to the marker's space (re-inject none: Example 6 "
     "would hold seven spaces)", BLOCKS,
     "        c0 = col + 1            # one column of the tab is the marker's space",
     "        c0 = col",
     [SEQUENCE]),
    # ⚠ the gather line is the same in `_quote` and `_item` (one laziness mechanism), so each
    # mutant of it carries the line before it at its own indentation: 16 columns is the quote's
    ("R13 §5.1: a quote's lazy candidates are gathered once, up to the first boundary (drop the "
     "bound: every quote re-scans the rest of the document, quadratic -- the result is the same, the "
     "cost is not)", MEMO,
     '                inner_lazy.append(True)\n                if block_end(content, len(content) - 1, True, inner_lazy):',
     '                inner_lazy.append(True)\n                if False:',
     ["block quotes are linear: N quotes cost <= 4N quote_content calls"]),
    ("R13 §5.1: lazy continuation (drop it: a marker-less line never joins the quote)", MEMO,
     '                inner_lazy.append(True)\n                if block_end(content, len(content) - 1, True, inner_lazy):',
     '                inner_lazy.append(True)\n                if True:',
     ["(quote) `> open `here\\nSlice 9z owns it` there`: the marker-less line is lazy continuation text "
      "of the quote's paragraph (§5.1), so the span crosses it: no site",
      SEQUENCE, SPEC_EXAMPLES]),
    ("RG3 IMP-2: every indented-code line is recorded for the seed (re-inject 'HTML lines only')", MEMO,
     '                    if opener[0] != "fence":', '                    if opener[0] == "html":',
     ["(lex-seed) `para\\n\\n    Slice 9z owns it`: indented code after a plain paragraph is raw under "
      "CommonMark too, but holds a declared id -- seeded by the one raw-line rule",
      "(lex-seed) a 4-space-indented slot row after a table's rows is raw (cmark-gfm: `<pre><code>`) "
      "and is SEEDED -- it holds a `|` -- never a silent skip (I-C)",
      "(lex-seed) a tab-indented slot row after a table's rows is raw (§2.2: four columns) and is SEEDED",
      "(item) … and that raw line inside the item is seeded (the one raw-line rule reaches into a "
      "container)"]),
    # ⚠ The five "RG3 IMP-1" rows (the item-open bit: its reading, its SET rule, its one-block
    # memory, the quote inside the item, the CLEAR rule) are DELETED with the bit: since R15 a list
    # item is a container, and the shapes they guarded are the R15 controls below.
    # -- PR #510 Codex R13: §4.6 case per condition, GFM §4.10 excess cells
    ("R13 §4.6: condition 5 `<![CDATA[` is exact (re-inject case folding)", BLOCKS,
     r'(?P<t5>!\[CDATA\[)', r'(?P<t5>(?i:!\[CDATA\[))',
     ["(html) `<![cdata[` is no CDATA opener (§4.6 condition 5 is exact; commonmark.js: a paragraph): "
      "the next line is prose and the site is reported"]),
    # ⚠ both probes interrupt a paragraph: at a block start `<PRE>` / `<DIV>` are type-7 openers
    # too, so a block-start probe never reaches the fold (the first R13 probes did, and survived);
    # no vendored example separates type 6 from type 7 by case either (Example 151's `<DIV CLASS>`
    # is a valid open tag at a block start)
    ("R13 §4.6: condition 1's tags are case-insensitive (drop the fold: `<PRE>` is type 7, which cannot "
     "interrupt)", BLOCKS,
     '(?P<t1>(?i:pre|script|style|textarea)', '(?P<t1>(?:pre|script|style|textarea)',
     ["(html) `text\\n<PRE>\\n9z owns it\\n</pre>`: `<PRE>` opens a type-1 block (§4.6 condition 1 is "
      "case-insensitive), which interrupts the paragraph -- raw to `</pre>`, no site"]),
    ("R13 §4.6: condition 6's tag list is case-insensitive (drop the fold: `<DIV>` is type 7, which "
     "cannot interrupt)", BLOCKS,
     '(?P<t6>/?(?i:', '(?P<t6>/?(?:',
     ["(html) `text\\n<DIV>\\n9z owns it`: `<DIV>` opens a type-6 block (§4.6 condition 6 is "
      "case-insensitive), which interrupts the paragraph -- raw to the blank line, no site"]),
    ("R13 §4.6: condition 4 is `<!` + an ASCII letter of EITHER case (re-inject upper only)", BLOCKS,
     '(?P<t4>![A-Za-z])', '(?P<t4>![A-Z])',
     ["(html) `<!doctype` opens a type-4 block (§4.6 condition 4: `<!` + an ASCII letter, either case) "
      "to the `>` line: raw, no site"]),
    ("R13 GFM §4.10: the excess cells of a NON-schema row are ignored before lexing (re-inject them)", TABLES,
     'body[:width], schema))', 'body, schema))',
     ["(table) an excess body cell of a non-schema table is ignored (GFM §4.10): its `9z owns it` is "
      "never lexed -- no site",
      "(rc) an excess body cell of a non-schema table holding `[x](absent-file.md)` is ignored (GFM "
      "§4.10): no link, nothing walked, rc 0"]),
    # -- PR #510 Codex R14: ONE id-token grammar for every reader
    ("R14-1 seed: the raw-line seed reads the grammar (re-inject the seed's own boundary regex -- the "
     "former `_BARE_TOKEN`, a second spelling that rejected a hyphen on either side)", CHECK,
     'ids = sorted({t.id for t in tokens(line) if t.kind != "cite" and t.id in keep})',
     'ids = sorted({t for t in __import__("re").findall(r"(?<![0-9A-Za-z-])(?:#11-[a-z0-9-]+|[0-9A-Za-z]{1,4})'
     '(?![0-9A-Za-z-])", line) if t in keep})',
     ["(lex-seed) a raw HTML line `9z-owner`: a hyphen bounds the short id on the raw line as in "
      "prose, so the line is seeded holding `9z`",
      "PROPERTY: the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)"]),
    ("R14-2 slug: bounded on its RIGHT side too (re-inject the left-only boundary)", IDS,
     '        if not t.r and _glued(text, t.end, +1, t.kind, pos, hi):',
     '        if not t.r and t.kind != "slug" and _glued(text, t.end, +1, t.kind, pos, hi):',
     ["(slug) `#11-zz-alphaZZ` in prose is not the id `#11-zz-alpha`: the slug is bounded on its right "
      "as on its left -- 0 sites",
      "(slug) `` `tool #11-zz-alpha_extra` ``: the kept-slug exception inside a code span reads the "
      "grammar's boundary, so nothing is excepted and the span stays code -- 0 sites"]),
    ("R14-3 def: the orphan exemption is by the orphan's exact bracket (re-inject the line-number test)", MEMO,
     'if exempt and (key not in orphans or site in orphans[key]):',
     'if exempt and (key not in orphans or site[0] in {s[0] for s in orphans[key]}):',
     ["an orphan definition exempts its OWN bracket only: `[sib]: child.md \"[sib]\"` is the documented "
      "miss, rc 2, child.md not walked"]),
    ("R14-4 cite: the citation label is either case, ONE predicate for the mask and the exemption "
     "(re-inject upper-case only)", IDS,
     'CITE_LABEL = r"[A-Za-z][0-9]+"', 'CITE_LABEL = r"[A-Z][0-9]+"',
     ["(cite) `[c1]` beside a declared no-owner id `c1`: a lowercase citation is masked by the same "
      "predicate the reference walk exempts it by -- 0 sites",
      "(cite) adjacent lowercase citations `[c1][c2]` are not a full reference (the exemption is the "
      "grammar's either-case predicate): rc 0"]),
    ("R14 property: a second spelling of an id class outside the grammar module is red (re-inject one "
     "in the row-noun anchor, behaviour unchanged)", ROLES,
     'NOUN_ANCHOR = re.compile(r"(?<!%s)%s" % (ALNUM, ROW_NOUN_SEP))',
     'NOUN_ANCHOR = re.compile(r"(?<![0-9A-Za-z])" + ROW_NOUN_SEP)',
     ["PROPERTY: the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)"]),
    # -- PR #510 Codex R15: §5.2 list items are containers; orphans keep their destination; the
    # row noun folds case
    ("R15 §5.2: a list item is a container (re-inject the flat reading: the marker line heads a "
     "paragraph, its content indentation tracked by nothing)", MEMO,
     '                if item_marker(line) is not None:\n                    open_block()',
     '                if False:\n                    open_block()',
     ["(item) the reviewer's input `- item\\n\\n    [child](child.md)`: the indented line is the item's "
      "SECOND paragraph (§5.2, Example 108: the content indentation is 2), not indented code -- the link "
      "is found, `child.md` is walked and its violation reported",
      "(item) a slot table inside a list item is a table (cmark-gfm, measured): its row declares its id",
      "(item) `> - [sib]: slice-9z-sib.md`: a definition inside an item inside a block quote registers -- "
      "the sibling is walked and its violation reported",
      SEQUENCE, SPEC_EXAMPLES]),
    ("R15 §5.2: the content indentation is stripped from every continuation line (drop the strip: the "
     "item's second paragraph is indented code again)", MEMO,
     '                    content.append(strip_columns(line, min(offset, indentation(line)[0])))',
     '                    content.append(line)',
     ["(item) the reviewer's input `- item\\n\\n    [child](child.md)`: the indented line is the item's "
      "SECOND paragraph (§5.2, Example 108: the content indentation is 2), not indented code -- the link "
      "is found, `child.md` is walked and its violation reported",
      "(item) `1. item\\n\\n     Slice 9z owns it`: an ordered item's content indentation is 3 (`1. `), so "
      "five columns are two inside it -- the item's second paragraph, the site is reported",
      SPEC_EXAMPLES]),
    ("R15 §5.2: the content indentation is W + N in LINE columns -- a tab past it leaves its remaining "
     "columns (re-inject 'a tab is one column' in the strip)", BLOCKS,
     '        col += 4 - col % 4 if line[j] == "\\t" else 1',
     '        col += 1',
     [SEQUENCE]),
    ("R15 §5.2: lazy continuation (drop it: a line short of the content indentation never joins the "
     "item; 20 columns is the item gather's line)", MEMO,
     '                    inner_lazy.append(True)\n                    if block_end(content, len(content) - 1, True, inner_lazy):',
     '                    inner_lazy.append(True)\n                    if True:',
     ["(item) `- open `x\\nSlice 9z owns it` end`: the line short of the content indentation is the item "
      "paragraph's lazy continuation text (§5.2 rule 5), so the span crosses it: no site",
      SPEC_EXAMPLES]),
    ("R15 §5.2: the enclosing container's lazy candidate stays lazy and whole inside the item (re-inject "
     "'strip it when indented enough')", MEMO,
     '                if _is_lazy(lazy, j) or (not is_blank(line) and indentation(line)[0] < offset):',
     '                if not is_blank(line) and indentation(line)[0] < offset:',
     [SEQUENCE]),
    ("R15 §5.2: the interruption rule -- an EMPTY item cannot interrupt a paragraph, nor an ordered item "
     "not starting at 1 (re-inject 'any marker line interrupts', the pre-R15 local policy)", BLOCKS,
     '    return not para_open or (not is_blank(content) and (marker in "-+*" or int(marker[:-1]) == 1))',
     '    return True',
     ["(item) `open `x\\n2. 9z owns it` end`: an ordered item not starting at 1 cannot interrupt a "
      "paragraph (§5.2) -- one paragraph, the span masks the site",
      "(item) `open `x\\n*\\n9z owns it` end`: an EMPTY item cannot interrupt a paragraph (§5.2; `*`, not "
      "`-`, which would be a setext underline) -- one paragraph, the span masks the site",
      SPEC_EXAMPLES]),
    ("R15 §5.2: an ordered item interrupts only when it starts at 1 (re-inject 'any number')", BLOCKS,
     'int(marker[:-1]) == 1))', 'True))',
     ["(item) `open `x\\n2. 9z owns it` end`: an ordered item not starting at 1 cannot interrupt a "
      "paragraph (§5.2) -- one paragraph, the span masks the site",
      SPEC_EXAMPLES]),
    ("R15 §4.1: a thematic break takes precedence over a list marker (drop it: `* * *` is an item)", BLOCKS,
     '    if m is None or _THEMATIC.match(line[j:]):', '    if m is None:',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R15 §5.2: an item can begin with at most one blank line (drop the rule: Example 280's `foo` joins "
     "the empty item)", MEMO,
     '        if not (is_blank(first) and j < n and is_blank(lines[j])):', '        if True:',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R15 §5.3: sibling items form one list only when of the same type (re-inject 'any marker continues "
     "the list')", BLOCKS,
     '    return a == b if a in "-+*" else b not in "-+*" and a[-1] == b[-1]', '    return True',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R15 §5.3: a list is loose when a non-final item ends with a blank line (drop the arm)", MEMO,
     '            loose = loose or ends_blank or k > j', '            loose = loose or k > j',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R15 §5.3: a list is loose when an item opens a block across a gap (drop the arm)", MEMO,
     '            loose = loose or gap', '            loose = loose or False',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R15 §5.3: a gap opens only after a block at its level (re-inject 'any blank line': an item's blank "
     "first line would loosen its list)", MEMO,
     '            gap = gap or len(sequence) > seq0', '            gap = True',
     [SEQUENCE, SPEC_EXAMPLES]),
    ("R15 #2 def: an orphan's destination decides the miss (re-inject the drop: every orphan blocks the "
     "shortcut)", MEMO,
     '            if any(self.sibling_path(dest) is not None for _, _, dest in entries):',
     '            if True:',
     ["(def) `paragraph\\n[x]: #section\\n[x]`: the orphan names a section, never a memo -- the shortcut "
      "is prose, rc 0",
      "(def) `paragraph\\n[x]: https://example.com/a\\n[x]`: the orphan names an external URL, never a "
      "memo -- rc 0"]),
    ("R15 #3 noun: the row noun folds ASCII case in ONE place (re-inject the Title/lower enumeration)", TABLES,
     'ROW_NOUN = r"(?ai:slices?|rows?|umbrellas?)"',
     'ROW_NOUN = r"(?:Slices?|slices?|Rows?|rows?|Umbrellas?|umbrellas?)"',
     ["(noun) `SLICE C owns it` names the row: the row noun folds case (a bare `C` is the declared "
      "single-letter miss, so only the anchor can reach it)",
      "(noun) `ROW 9 lands first` names the row (a bare `9` is the declared numeric miss)",
      "(noun) `UMBRELLA C owns it` names the row"]),
]

MUTANTS += [
    # -- PR #510 Codex R16
    ("R16 #2 §2.5: character references in a destination are decoded (re-inject backslash-only unescaping: "
     "no reference is ever matched)", LEXER,
     '        m = _CHAR_REF.match(s, i)\n        ch = _reference(m) if m else None',
     '        m = None\n        ch = _reference(m) if m else None',
     ["(link) `[child](child&#46;md)`: a decimal character reference in the destination is decoded (§2.5 / "
      "§6.3) -- `child.md` is walked and its violation reported",
      "(link) `[child](<child&#46;md>)`: the angle-bracket destination decodes by the same rule",
      "(link) `[child](child&period;md)`: a named entity reference (HTML5 `&period;`) decodes",
      "(link) `[child](child&#x2E;md)`: a hexadecimal character reference decodes",
      "(def) `[sib]: child&#46;md`: a reference definition's destination is decoded by the SAME normalisation "
      "-- the sibling is walked",
      "(link) `[x](slice&#37;20sib.md)`: §2.5 decodes `&#37;` to `%`, then `sibling_path` percent-decodes "
      "`%20` (stage b) -- the two decoders run in spec order, `slice sib.md` is walked"]),
    ("R16 #2 §2.5: the decoder is ONE pass of the spec's grammar, not `html.unescape` (re-inject it as a "
     "second pass: HTML's legacy semicolon-less `&copy`, and a decoded `&` / an escaped `&` re-read as a "
     "reference)", LEXER,
     '        out.append(s[i])\n        i += 1\n    return "".join(out)',
     '        out.append(s[i])\n        i += 1\n    return __import__("html").unescape("".join(out))',
     ["(link) `[x](child&copy.md)`: HTML's legacy semicolon-less `&copy` is NOT a reference in CommonMark "
      "(§2.5 Example 29) -- the destination is the literal `child&copy.md`, and THAT file is walked "
      "(`html.unescape` would have named `child©.md`)",
      "(link) `[x](child\\&#46;md)`: a backslash-escaped `&` opens no reference (ONE pass: §2.4 and §2.5 "
      "meet at the character once) -- literal `child&#46;md`, no memo, 0 sites",
      "(link) `[x](child&#x26;#46;md)`: a decoded `&` is a character, never the start of a second reference "
      "(one pass, no re-scan) -- literal `child&#46;md`, 0 sites"]),
    ("R16 #2 §2.5: U+0000 is replaced by U+FFFD (drop the rule: `&#0;` yields a C0 control the resolver "
     "rejects)", LEXER,
     '    if n == 0 or n > 0x10FFFF or 0xD800 <= n <= 0xDFFF:', '    if n > 0x10FFFF or 0xD800 <= n <= 0xDFFF:',
     ["(link) `[x](child&#0;.md)`: U+0000 is replaced by U+FFFD (§2.5, \"for security reasons\") -- the memo "
      "named `child\\ufffd.md` is walked (a raw U+0000 would be rejected as a C0 control)"]),
    ("R16 #2 §2.5: a reference needs its `;` (re-inject an optional semicolon)", LEXER,
     '[A-Za-z][A-Za-z0-9]{1,31});")', '[A-Za-z][A-Za-z0-9]{1,31});?")',
     ["(link) `[x](child&#46md)`: `&#46` without `;` is no reference -- the destination is the literal "
      "`child&#46md`, which names no memo: 0 sites, `child.md` not walked"]),
    ("R16 #2 §6.3: label matching is on the RAW label (re-inject character-reference decoding in "
     "`normalize_label`)", LEXER,
     '    return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).casefold()',
     '    return _LABEL_WS.sub(" ", normalize_destination(label).strip(" \\t\\r\\n")).casefold()',
     ["(label) `[foo&auml;]: child.md` then `[fooä]`: §6.3 label matching is on the RAW label (case fold, "
      "strip, collapse -- no character-reference decoding; commonmark.js: no link) -- the shortcut is prose, "
      "0 sites, `child.md` not walked"]),
    # #3 (IMP): container nesting off the call stack, and the I/O chokepoint narrowed to I/O
    ("R16 #3 nesting: the frame stack has no depth cap (re-inject one of 200 -- a `RecursionError` at the "
     "201st nested container, as the call stack gave at ~500)", MEMO,
     '                frames.append(child)\n                value = None',
     '                if len(frames) >= 200:\n                    raise RecursionError("nesting deeper than 200")\n'
     '                frames.append(child)\n                value = None',
     ["container nesting is off the call stack: 1,000 nested quotes / items parse as commonmark.js nests them"]),
    ("R16 #3 chokepoint: only I/O is the unavailable-memo miss (re-inject `RuntimeError` in the chokepoint's "
     "except: a parser exception becomes rc 2)", MEMO,
     '            except (OSError, UnicodeDecodeError) as e:',
     '            except (OSError, RuntimeError, UnicodeDecodeError) as e:',
     ["a RuntimeError raised while PARSING a memo is a crash out of check(), never the unavailable-memo miss"]),
]

