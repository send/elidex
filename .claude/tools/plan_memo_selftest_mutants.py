#!/usr/bin/env python3
"""Re-executable mutation proof for `plan-memo-umbrella-check.py --self-test --mutants`.

A control that has never gone red proves nothing.  `MUTANTS` names, for each
lexing clause and each gating stage, ONE edit to the source that defeats it,
and the control that must turn red when the edit is applied.  Most rows
REMOVE a clause; a few (F7, the shape rule of `is_empty`) RE-INJECT the
defect the clause was written against, because the clause is a default (an
empty census is clean; a cell with no alphanumeric is empty) and there is no
line to delete -- the defect has to be put back to show the control sees it.
The runner patches the source TEXT, exec's a fresh module set from it (plan
§2 I-F), and re-runs the named control(s) against the patched checker:

  * the substring must occur EXACTLY ONCE in its file -- a substring that no
    longer applies (the code moved) is a FAIL, never "survived";
  * every named control must fail under the mutant; a mutant all of whose
    controls stay green has SURVIVED, and that is a FAIL too;
  * a control that CRASHES under the mutant proves nothing about the clause
    (an exception is not the control going red) and is a FAIL.

Row shape: (name, file, find, replace, [control names]).  A row whose file is
the RUNNER itself (`plan_memo_umbrella_selftest.py`) patches the runner, not
the checker set, and takes its controls from the patched runner's registry.
"""

LEXER, BLOCKS, TABLES, MEMO, ROLES, CHECK, SELFTEST = (
    "plan_memo_lexer.py", "plan_memo_blocks.py", "plan_memo_tables.py", "plan_memo_memo.py",
    "plan_memo_roles.py", "plan-memo-umbrella-check.py", "plan_memo_umbrella_selftest.py")

# The spec-example conformance control (`plan_memo_selftest_conformance.py`):
# the one control a spec-table transcription error turns red.
SPEC_EXAMPLES = "CommonMark 0.31.2 spec examples (Tabs, §4.1-§4.9): Phase 1's block sequence aligns with the html"
# The block-sequence control over the §4.4 / §5.1 shapes the vendored examples
# do not reach (each expected sequence read off commonmark.js 0.31.2).
SEQUENCE = "Phase 1's block sequence over the §4.4 chunk and §5.1 container shapes matches commonmark.js"

MUTANTS = [
    # -- CommonMark §4.5 fenced code blocks
    ("fence: opener needs >=3 fence characters", BLOCKS,
     '(`{3,}|~{3,})', '(`{4,}|~{4,})',
     # the tilde control: three literal backtick lines would pair as a code
     # span and mask the site anyway, so only the tilde form can go red
     ["(fence) a tilde fence masks too"]),
    ("fence: opener indent <=3 columns (re-inject any indent)", BLOCKS,
     '    rest = unindented(line)\n    m = _FENCE_OPEN.match(rest)',
     '    rest = line.lstrip(" \\t")\n    m = _FENCE_OPEN.match(rest)',
     ["(fence) four spaces of indent is not a fence"]),
    ("fence: backtick info string may not hold a backtick", BLOCKS,
     '(m.group(1)[0] == "`" and "`" in m.group(2))', 'False',
     ["(fence) a backtick fence whose info string holds a backtick is not a fence"]),
    ("fence: closer uses the opener's character", BLOCKS,
     're.escape(ch)', '"[`~]"',
     ["(fence) a closer of the OTHER character does not close"]),
    ("fence: closer at least as long as the opener", BLOCKS,
     '"{%d,}" % k', '"{3,}"',
     ["(fence) a shorter closer does not close"]),
    ("fence: closer followed only by spaces/tabs", BLOCKS,
     'r"[ \\t]*$"', 'r".*$"',
     ["(fence) a closer followed by text does not close"]),
    ("fence: fenced lines are not paragraph lines (A x B)", BLOCKS,
     '    closer = fence_opener(line)\n    if closer is not None:\n        return "fence", closer',
     '    closer = None\n    if closer is not None:\n        return "fence", closer',
     ["(fence) a link inside a fence is not a link (A x B)"]),
    # -- CommonMark §6.1 code spans
    ("span: opener and closer are backtick strings of EQUAL length", LEXER,
     '        if rb - ra == k:\n            return rb', '        if True:\n            return rb',
     ["(span) backtick strings pair by EQUAL length"]),
    ("span: an unmatched backtick string is literal", LEXER,
     '            if close is None:\n                i = a1                  # an unmatched backtick string is literal',
     '            if close is None:\n                code.append((i, n))\n                i = n',
     ["(span) an unmatched backtick string is literal, not a mask to end of line"]),
    ("span: lexed over the paragraph, not the line", MEMO,
     '            if kind:\n                flush(kind)', '            flush()',
     ["(span) a code span may cross a line ending"]),
    ("span: a list item starts a block", BLOCKS,
     'one_line_block(line) is not None or list_item_line(line) or ', 'one_line_block(line) is not None or ',
     ["(span) a paragraph ends at a list item: a backtick open in one item and closed in the next is literal"]),
    ("span: a `>` line starts a block", BLOCKS,
     '    return one_line_block(line) is not None or list_item_line(line) or quote_content(line) is not None',
     '    return one_line_block(line) is not None or list_item_line(line)',
     ["(span) a paragraph ends at a `>` line"]),
    ("span: an ATX heading is a block", BLOCKS,
     '    m = _ATX.match(rest)\n    if m:', '    m = None\n    if m:',
     ["(span) a paragraph ends at an ATX heading"]),
    ("A x E: kind markers are read from the MASKED declaring field", MEMO,
     'row.field = stream(row.cells[row.schema.decl].lexed)',
     'row.field = row.cells[row.schema.decl].text',
     ["(span) a quoted kind marker is not a declaration (A x E)"]),
    # -- GFM §4.10 rows and tables
    ("row: an unescaped `|` splits even inside backticks (Example 200)", BLOCKS,
     '        if c != "|":\n            continue',
     '        if c != "|" or line[start:i].count("`") % 2:\n            continue',
     ["(row) an unescaped `|` inside backticks SPLITS the row: each half is prose with a literal "
      "backtick, so the id on each side is a mention"]),
    ("row: `\\|` becomes `|` (the backslash is consumed)", BLOCKS,
     '            breaks[-1].append(i - 1)', '            pass',
     ["(row) `\\|` is `|` in the cell, so `9z \\| 7z` is an id-only run"]),
    ("row: the leading pipe is optional", BLOCKS,
     'if stripped.startswith("|") and bounds:', 'if bounds:',
     ["(row) a table without leading and trailing pipes declares its rows"]),
    ("row: the trailing pipe is optional", BLOCKS,
     'if stripped.endswith("|") and bounds and not _escaped(stripped, len(stripped) - 1):', 'if bounds:',
     ["a table with and without edge pipes reads the same"]),
    ("row: offsets map through each cell's raw segments", BLOCKS,
     '        return raw_start + (i - off)', '        return i',
     ["a site after an escaped pipe is reported at its raw column"]),
    ("table: delimiter cell = >=1 hyphen with optional colons", BLOCKS,
     '_DELIM_CELL = re.compile(r":?-+:?")', '_DELIM_CELL = re.compile(r"[:\\- ]*")',
     ["(table) a delimiter cell is >=1 hyphen with optional colons; `:` alone is not"]),
    ("table: header and delimiter must have equal width", BLOCKS,
     '    return width is not None and len(split_row(lines[i])) == width',
     '    return width is not None',
     ["(rc) a slice header over a one-cell delimiter row is not a table, so its wide body row "
      "is not a width miss"]),
    ("table: a schema body row of the wrong width is exit 2 (the width miss gates)", MEMO,
     '                for lineno, msg in t.misses:\n'
     '                    self.misses.append((memo.path.name, lineno, msg))',
     '                for lineno, msg in t.misses:\n'
     '                    pass',
     ["(rc) a schema body row whose width differs from its header is rc 2"]),
    # -- CommonMark §6.3 links / §4.7 definitions
    ("link: bare destination balances parentheses", LEXER,
     '        if c == "(":\n            depth += 1', '        if c == "(":\n            break',
     ["(link) bare destination with a balanced parenthesis pair"]),
    ("link: backslash escapes ASCII punctuation (destination, `<dest>`, title)", LEXER,
     'return s[j] == "\\\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT', 'return False',
     ["(link) bare destination with an escaped parenthesis",
      "(link) `<dest>` may contain an escaped `>`",
      '(link) a `"…"` title may contain an escaped `"`']),
    ("link: only ASCII space ends a bare destination, not Unicode whitespace", LEXER,
     'if c == " " or ord(c) <= 31 or ord(c) == 127:', 'if c.isspace() or ord(c) <= 31:',
     ["(link) a non-ASCII space is destination content (only ASCII space and controls end it)"]),
    ("link: 0x7F is an ASCII control character", LEXER,
     'if c == " " or ord(c) <= 31 or ord(c) == 127:', 'if c == " " or ord(c) <= 31:',
     ["(link) an ASCII control character (0x7F) ends a bare destination"]),
    ("link: `<dest>` holds no unescaped `<`", LEXER,
     'elif s[j] in "<>\\n":', 'elif s[j] in ">\\n":',
     ["(link) `<dest>` may not contain an unescaped `<`"]),
    ("link: a `(…)` title holds no unescaped `(`", LEXER,
     '        elif opener == "(" and s[j] == "(":\n            return None',
     '        elif False:\n            return None',
     ["(link) a `(…)` title may not contain an unescaped `(`"]),
    ("link: label matching collapses internal whitespace", LEXER,
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).casefold()',
     'return label.strip(" \\t\\r\\n").casefold()',
     ["(link) label matching collapses internal whitespace"]),
    ("link: label matching is a case FOLD", LEXER,
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).casefold()',
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).lower()',
     ["(link) label matching is a Unicode case FOLD, not lower()"]),
    ("link: label matching folds spaces, tabs, line endings -- not Unicode whitespace", LEXER,
     'return _LABEL_WS.sub(" ", label.strip(" \\t\\r\\n")).casefold()',
     'return " ".join(label.split()).casefold()',
     ["(link) label matching collapses spaces, tabs and line endings only: a non-breaking "
      "space is not a space, so the reference is unanswered"]),
    ("link: label content = a character that is not a space, tab, or line ending", LEXER,
     'return bool(raw.strip(" \\t\\r\\n"))', 'return bool(raw.strip())',
     ["(link) a label holding only a non-breaking space is a label (§6.3: at least one "
      "character that is not a space, tab, or line ending)"]),
    ("link: `[text]` followed by a link label is not a shortcut", LEXER,
     '                return end, defs.get(normalize_label(raw)), "full", raw',
     '                if defs.get(normalize_label(raw)) is not None:\n'
     '                    return end, defs.get(normalize_label(raw)), "full", raw',
     ["(link) `[label][undefined]` is neither a full reference nor a shortcut (§6.3 Example 571: "
      "a shortcut is not followed by a link label) -- the unanswered label is a schema miss, not "
      "a link to the sibling"]),
    ("link: collapsed reference resolves the text as label", LEXER,
     '    return end, defs.get(normalize_label(raw)), form, raw',
     '    return end, None if form == "collapsed" else defs.get(normalize_label(raw)), form, raw',
     ["(link) collapsed reference `[label][]`"]),
    ("link: bracket text nests (full reference with inner brackets): an inactive inner opener "
     "is popped and its `]` is literal, not a failure of the outer", LEXER,
     '                i += 1                  # literal `]`; the opener is gone; the tail is NOT consumed\n                continue',
     '                stack.clear()\n                i += 1\n                continue',
     ["(link) full reference whose text holds nested brackets; the label is the link's tail, not "
      "prose, and so is the definition"]),
    ("def: the first definition of a label wins", MEMO,
     '                self.defs.setdefault(normalize_label(raw), dest)',
     '                self.defs[normalize_label(raw)] = dest',
     ["(def) the FIRST definition of a label wins"]),
    ("def: up to one line ending before the destination", BLOCKS,
     '        k = _skip_ws(s, k + 1)\n        dest, k = link_destination(s, k)',
     '        k = _skip_ws(s, k + 1, newlines=0)\n        dest, k = link_destination(s, k)',
     ["(def) one line ending is allowed before the destination"]),
    ("def: nothing but whitespace after the destination/title", BLOCKS,
     '    if s[k] == "\\n":\n        return k + 1\n    return None',
     '    if s[k] == "\\n":\n        return k + 1\n    return k',
     ["(def) text after the destination is not a definition, so the reference is unanswered: "
      "a schema miss"]),
    ("def: a definition cannot interrupt a paragraph (Phase 1: only at a block start)", MEMO,
     '            if d is not None and not cur:', '            if d is not None:',
     ["(def) a definition cannot interrupt a paragraph: the reference is unanswered, and "
      "reported ONCE (`[text][label]` re-scans `[label]`)"]),
    # -- I-F one population, one pipeline
    ("population: every memo's rows are declared (census)", MEMO,
     '        for memo in self.memos:\n            self._declare(memo)', '        self._declare(self.main)',
     ["(population) an umbrella declared in a linked memo is in the census"]),
    ("population: every memo's ids are in the keep-set", MEMO,
     '        return set(self.ids)',
     '        return {rid for rid, r in self.ids.items() if r.memo is self.main}',
     ["(population) a terminal id declared in a linked memo is in the keep-set, so `Tq / 9z` is "
      "an id-only run, not code"]),
    ("population: every memo's rows are asserted", MEMO,
     'return [r for memo in self.memos for r in memo.schema_rows(name)]',
     'return list(self.main.schema_rows(name))',
     ["(b) a sibling umbrella's Deps edge is asserted"]),
    ("population: the link walk is transitive", MEMO,
     'queue.extend(memo.linked_files())',
     'queue.extend(memo.linked_files() if len(self.memos) == 1 else [])',
     ["(population) the population is transitive: a memo linked from a linked memo is scanned"]),
    ("gate: an absent linked memo is a schema miss", MEMO,
     '            except (OSError, RuntimeError, UnicodeDecodeError) as e:\n                self.misses.append(',
     '            except (OSError, RuntimeError, UnicodeDecodeError) as e:\n                [].append(',
     ["(rc) a linked memo that is not on disk is rc 2, never clean"]),
    ("gate: an unmatched schema is a schema miss", MEMO,
     '                if s.name not in matched:', '                if False:',
     ["(rc) a schema with no matching table is rc 2"]),
    ("gate: a duplicate declaration is a schema miss", MEMO,
     '                if rid in self.ids:', '                if False:',
     ["(rc) the same id declared in two memos is rc 2"]),
    ("gate: KIND-SPELLING is a mechanical finding", CHECK,
     '        if len(pop.spellings) > 1:', '        if False:',
     ["(rc) the undetermined kind written two ways is KIND-SPELLING, rc 1"]),
    ("gate: a mechanical finding is exit 1", CHECK,
     'rc = 1 if mechanical else 0', 'rc = 0',
     ["(rc) the undetermined kind written two ways is KIND-SPELLING, rc 1"]),
    # -- /code-review high: one mutant per fix
    ("F1 id cell: the id is followed by a non-id character, not the cell end", TABLES,
     '+ r"(?![0-9A-Za-z-])")', '+ r"$")',
     ["(id) an id cell with trailing prose declares the id at its start",
      "(id) a backticked slug with trailing prose declares the slug"]),
    ("F1 id cell: a cell not starting with an id declares nothing (no fallback to the cell)", TABLES,
     'return g.group("id") if g else None', 'return g.group("id") if g else cell_text.strip()',
     ["(id) a cell that does not start with an id declares nothing: the row is unkeyed (its "
      "Deps edge would go unasserted), so the run is a schema miss"]),
    ("#2 gate: an unkeyed schema row is a schema miss (not a note, not a silent drop)", MEMO,
     '                    if not is_blank_id_cell(row.id_cell()):\n'
     '                        self.misses.append(',
     '                    if False:\n'
     '                        self.misses.append(',
     ["(id) a cell that does not start with an id declares nothing: the row is unkeyed (its "
      "Deps edge would go unasserted), so the run is a schema miss"]),
    ("F2 population: links in CELLS join the population", MEMO,
     '        for lx in self.lexed():\n            for _, _, dest in lx.links:',
     '        for lx in (p.lexed for p in self.paragraphs):\n            for _, _, dest in lx.links:',
     ["(rc) a link to an absent memo inside a table CELL is rc 2",
      "(population) a violation in a sibling linked ONLY from a cell is reported"]),
    ("F3 population: a destination with a scheme or `//` is not a sibling", MEMO,
     '        if _SCHEME.match(raw):                                       # (a)',
     '        if False:                                                    # (a)',
     ["(rc) an absolute URL ending in `.md` is not a sibling on disk: rc 0"]),
    ("F4 attribution: the FIRST marker occurrence decides", TABLES,
     '    m = re.search(re.escape(MARKER), field)',
     '    m = list(re.finditer(re.escape(MARKER), field))[-1]',
     ["(a) a self-declaring field that later says a sibling 'is not it' stays self-declaring"]),
    ("F5 kind: the undetermined spelling is collected beside the marker", MEMO,
     '        if m:\n            self.spellings.add(m.group(0))',
     '        if m and MARKER not in row.field:\n            self.spellings.add(m.group(0))',
     ["(rc) a row carrying the marker AND one undetermined spelling, beside another row's other "
      "spelling, is KIND-SPELLING rc 1"]),
    ("F6 (c): the Deps cell's ids are the population's mentions, not a raw tokenisation", ROLES,
     'cell_ids = in_deps.get(_row_key(row), set())',
     'cell_ids = set(re.findall(SHORT_ID, deps))',
     ["(c-seed) a Deps cell naming a FILE whose name holds the id does not carry the id",
      "(c-seed) a Deps cell `xxxxC` does not carry the id `C`"]),
    ("F7 check: an empty no-owner census is clean, not a schema miss", CHECK,
     '    notes.append("[CENSUS] %d no-owner rows (umbrella + kind-undetermined)" % len(umb))',
     '    if not umb:\n        return _result(findings, notes, 2, [], pop)\n'
     '    notes.append("[CENSUS] %d no-owner rows (umbrella + kind-undetermined)" % len(umb))',
     ["(rc) a memo whose every row is terminal is rc 0, not a schema miss",
      "(rc) an all-terminal memo reports a zero census"]),
    ("F8 / #4 empty cell: `n/a` is a lexical exception (case-insensitive)", TABLES,
     'EMPTY_WORDS = frozenset({"n/a", "none"})', 'EMPTY_WORDS = frozenset({"none"})',
     ["(c-seed) a Deps cell `n/a` is empty, so ordering prose is reported",
      "(c-seed) a Deps cell `N/A` is empty (the lexical exception is case-insensitive)"]),
    ("#4 empty cell: emptiness is decided by SHAPE (no alphanumeric), re-injecting the old list", TABLES,
     'return not any(ch.isalnum() for ch in bare) or bare.casefold() in EMPTY_WORDS',
     'return bare in {"", "\\u2014", "-"} or bare.casefold() in EMPTY_WORDS',
     ["(c-seed) a Deps cell `–` (en dash) is empty by shape: no alphanumeric",
      "(c-seed) a Deps cell `--` is empty by shape"]),
    ("4.5 id cell: blanks are LITERAL, not the shape rule (re-inject `is_empty`)", MEMO,
     '                    if not is_blank_id_cell(row.id_cell()):',
     '                    if not __import__("plan_memo_tables").is_empty(row.id_cell()):',
     ["(id) an id cell `?` is not a blank: unkeyed, rc 2",
      "(id) an id cell `…` is not a blank: unkeyed, rc 2 (the shape rule would skip it)",
      "(id) an id cell `**?**` is not a blank: decoration does not blank it, rc 2"]),
    ("4.5 id cell: `—` is a literal blank", TABLES,
     'ID_CELL_BLANKS = frozenset({"", "\\u2014", "-", "\\u2013"})',
     'ID_CELL_BLANKS = frozenset({"", "-", "\\u2013"})',
     ["(id) an id cell `—` is a literal blank: a deliberate non-row, rc 0"]),
    ("4.5 link: a citation-grammar label is exempt in every reference form", MEMO,
     'exempt = _CITE_LABEL.fullmatch(key) is not None or form == "shortcut"',
     'exempt = form == "shortcut"',
     ["(link) adjacent citations `[C19][C20]` are not a full reference: rc 0",
      "(link) a collapsed-shaped citation `[C19][]` is not a reference: rc 0"]),
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
    ("R1-2 runner: the emptiness guard fires on zero controls / zero mutants", SELFTEST,
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
     '        if is_img:\n            images.append((i, end))',
     '        if is_img:\n            images.append((i, end))\n            i += 1\n            continue',
     ["(image) `![alt][img]` with a definition is consumed whole: `[img]` is not re-read as a "
      "shortcut, and the image destination is not a memo"]),
    ("R3-1 link: an escaped `[` is not an opener", LEXER,
     '        if _is_escape(s, i):\n            i += 2                      # §2.4: `\\[` / `\\]` / `\\`` are literal',
     '        if _is_escape(s, i) and s[i + 1] != "[":\n            i += 2',
     ["(link) an escaped `\\[` opens nothing: `\\[x](absent-file.md)` is not a link, rc 0"]),
    ("R3-2 / R5-2 population: a `/`-leading path -- raw `/x.md`, `//host/x.md`, or DECODED "
     "`%2Ftmp%2Fx.md` -- is not a sibling", MEMO,
     '        if name.startswith("/") or _CONTROL.search(name):            # (c)',
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
     ["Phase-1 orphan detection is linear: <= 4 link_label calls per line, t(4N)/t(N) < 8"]),
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
     '= inline_pass(blank_spans(self.text, [(m.start(), m.end()) '
     'for m in re.finditer(r"`[^`]*`", self.text)]), defs)',
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
     '    defs, _ = reference_definitions(__import__("plan_memo_lexer").blank_spans(block, __import__("plan_memo_lexer").inline_pass(block, {})[0]), limit=1, start=off)',
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
     ["unresolved_references scales linearly: t(4N)/t(N) < 8"]),
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
     '        if name.startswith("/") or _CONTROL.search(name):            # (c)',
     '        if name.startswith("/"):                                     # (c)',
     ["a decoded destination with a C0 control character is rejected, never resolved"]),
    # -- PR #510 Codex R8
    ("R8-1 sibling: the scheme is read on the RAW path, before decoding (re-inject scheme-after-decode)", MEMO,
     '        if _SCHEME.match(raw):                                       # (a)',
     '        if _SCHEME.match(unquote(raw)):                              # (a)',
     ["(link) `notes%3Achild.md` has no scheme (WHATWG URL: a scheme is read BEFORE decoding): it is "
      "the local file `notes:child.md`, and it is scanned"]),
    ("R8-2 sibling: an OSError from resolve() is the unavailable-sibling miss (unguard it)", MEMO,
     '    try:\n        return path.resolve()\n    except (OSError, RuntimeError):\n        return path',
     '    return path.resolve()',
     ["an OSError from resolve() is the unavailable-sibling schema miss, never an exception"]),
    ("R8-5 sibling: the dedup is a set (re-inject the list membership test)", MEMO,
     '                if f is not None and f not in seen:\n                    seen.add(f)',
     '                if f is not None and f not in out:\n                    pass',
     ["linked_files scales linearly: t(4N)/t(N) < 8 (set dedup)"]),
    ("R8-3 bare id: the far side of a `.` is the ASCII id class, not `str.isalnum`", CHECK,
     'bool(_ID_CONTINUES.match(text[j]))', 'text[j].isalnum()',
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
    ("R9 F1 setext: the underline closes the paragraph (re-inject the join)", MEMO,
     '                if cur and is_setext_underline(line) and not container_text(cur[0][1]):',
     '                if False:',
     ["(setext) `Heading\\n===` is a heading; the `===` underline ends the paragraph, so a code "
      "span opened in the heading does not reach the next paragraph's site"]),
    ("R9 F1 setext: not after a list item or `>` line (Examples 92-94)", MEMO,
     '                if cur and is_setext_underline(line) and not container_text(cur[0][1]):',
     '                if cur and is_setext_underline(line):',
     ["(setext) `==` after a list item is NOT an underline (§4.3 Examples 92-94): the item's "
      "paragraph continues and a code span crosses it"]),
    ("R9 F1 / R13 seed: every raw HTML-block line is recorded for the seed", MEMO,
     '                        self.raw_html.extend((linenos[k], lines[k]) for k in range(i, end))',
     '                        pass',
     ["(lex-seed) an HTML-block opener holding a declared id is a seed",
      "(lex-seed) `<pre>\\nSlice 9z owns it\\n</pre>`: the inner line holding the id is seeded (type 1 "
      "ends at `</pre>`)"]),
    ("R9 F1 seed: only a line holding a `|` or a declared id is reported", CHECK,
     '            if "|" in line or ids:', '            if True:',
     ["(lex-seed) an HTML-block line with neither a `|` nor a declared id is no seed"]),
    ("R9 F2 I/O: a decode error is the unavailable-memo miss (unguard it)", MEMO,
     '            except (OSError, RuntimeError, UnicodeDecodeError) as e:', '            except (OSError, RuntimeError) as e:',
     ["an undecodable sibling is the unavailable-linked-memo schema miss, never an exception"]),
    ("R9 F3 ascii: the row-noun anchor is an ASCII class (re-inject `\\b`)", ROLES,
     'MENTION_PROSE = re.compile(r"(?<![0-9A-Za-z])" + ROW_NOUN_ID', 'MENTION_PROSE = re.compile(r"\\b" + ROW_NOUN_ID',
     ["(ascii) `次のSlice Cが所有する` reaches the naming worklist: the row-noun anchor is not `\\b` "
      "(no Unicode word boundary before `Slice`)"]),
    ("R9 F3 ascii: the slug anchor is an ASCII class (re-inject `\\w`)", ROLES,
     'MENTION_SLOT = re.compile(r"(?<![0-9A-Za-z_-])"', 'MENTION_SLOT = re.compile(r"(?<![\\w-])"',
     ["(ascii) `次は#11-zz-alphaが所有する` reaches the naming worklist: the slug anchor is an ASCII "
      "class, not `\\w`"]),
    ("R9 F3 ascii: list markers are ASCII digits (re-inject `\\d`)", BLOCKS,
     '_LIST_ITEM = re.compile(r"^(?:[-+*]|[0-9]{1,9}[.)])(?:[ \\t]|$)")',
     '_LIST_ITEM = re.compile(r"^(?:[-+*]|\\d{1,9}[.)])(?:[ \\t]|$)")',
     ["(ascii) `١.` (an Arabic-Indic digit) is not a list marker (§5.2: ASCII digits)"]),
    ("R9 #3 row: breaks are partitioned in the one scan (re-inject the per-cell filter)", BLOCKS,
     '        out.append(_cell(line, a, b, cell_breaks))',
     '        out.append(_cell(line, a, b, [x for bs in breaks for x in bs if a <= x < b]))',
     ["split_row scales linearly: t(4N)/t(N) < 8 (breaks partitioned in the scan)"]),
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
    ("RG2 IMP-2: an orphan is a VALID definition off a block start (re-inject the label-colon shape)", MEMO,
     '            if d is not None:\n                # a valid definition that cannot take effect: the orphan\n'
     '                self.orphans.setdefault(normalize_label(d[0]), set()).add(linenos[i])',
     '            if d is not None or (line.lstrip(" ").startswith("[") and "]:" in line):\n'
     '                self.orphans.setdefault(normalize_label(d[0] if d else line.split("]:")[0].lstrip(" [")), set()).add(linenos[i])',
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
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))+\\.md(?![0-9A-Za-z]))',
     '|(?P<file>[\\w./-]+\\.md(?![0-9A-Za-z]))',
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
    ("#4 empty cell: a word outside the lexical exceptions is NOT empty", TABLES,
     'EMPTY_WORDS = frozenset({"n/a", "none"})', 'EMPTY_WORDS = frozenset({"n/a", "none", "nil"})',
     ["(b) a Deps cell `nil` -- a word outside the lexical exceptions -- is NOT empty: the "
      "stated polarity is a reported edge (false rc 1), never a silent skip"]),
    ("F9 span: an escaped backtick opens no span", LEXER,
     '        if _is_escape(s, i):\n            i += 2                      # §2.4: `\\[` / `\\]` / `\\`` are literal',
     '        if _is_escape(s, i) and s[i + 1] != "`":\n            i += 2',
     ["(span) a backtick behind a backslash is literal and opens no span"]),
    ("F12 link: the link tail masks a slug", TABLES,
     '    out += [(a, b, "link") for a, b, _ in lx.links]', '    pass',
     ["(link) a `#11-` slug in a link DESTINATION is not a naming site"]),
    ("F13 link: an unanswered full reference is reported", LEXER,
     '                if form is not None and not (form == "shortcut" and pos == relabel):',
     '                if form is not None and form != "full" and not (form == "shortcut" and pos == relabel):',
     ["(link) a full reference no definition answers is a schema miss"]),
    ("F13 link: a shortcut with an orphan definition is reported", LEXER,
     '                if form is not None and not (form == "shortcut" and pos == relabel):',
     '                if form is not None and form != "shortcut":',
     ["(link) a shortcut whose only definition sits mid-paragraph is a schema miss"]),
    ("#1 gate: an unresolved reference is a schema miss (rc 2), not a note", MEMO,
     '            for lineno, label in memo.unresolved_references():\n'
     '                self.misses.append(',
     '            for lineno, label in ():\n'
     '                self.misses.append(',
     ["(rc) a full reference no definition answers is rc 2, never clean",
      "(link) a full reference no definition answers is a schema miss"]),
    ("C7 kind: the population's pointer kind excludes the row from the acceptance seed", ROLES,
     '        if row.kind != "terminal":\n            continue',
     '        if row.kind in ("umbrella", "undetermined"):\n            continue',
     ["(accept-vocab seed) a POINTER row is excluded from the population",
      "(accept-vocab seed) a row whose marker is ATTRIBUTED to another row is a pointer and owes "
      "no acceptance condition"]),
    ("C8 disposition: a kept slug inside a code span is visible", TABLES,
     '            if m.group(0) in keep:', '            if False:',
     ["(span) a kept slug inside a command-line code span is a naming site"]),
    # -- /elidex-review Stage 6
    ("#3 bare id: bounded by the complement of the id-continuation class (re-inject a list)", CHECK,
     '_ID_CONTINUES = re.compile(r"[0-9A-Za-z]")', '_ID_CONTINUES = re.compile(r"[^\\s,;/()\\[\\]*`.:]")',
     ["(bare) an id before `?` is bounded", "(bare) an id before `!` is bounded",
      "(bare) an id inside ASCII double quotes is bounded",
      "(bare) an id inside curly double quotes is bounded"]),
    ("#3 bare id: a hyphen bounds a short id (re-inject it into the class)", CHECK,
     '_ID_CONTINUES = re.compile(r"[0-9A-Za-z]")', '_ID_CONTINUES = re.compile(r"[0-9A-Za-z-]")',
     ["(bare) a hyphen bounds a short id: `after 9z-7z` names 9z"]),
    ("#3 file token: a bare `.md` file name is masked before the bare scan", LEXER,
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))+\\.md(?![0-9A-Za-z]))', '',
     ["(bare) a bare `.md` file name holding an id is a file token, not a site"]),
    ("#3 bare id: a dotted number is one token", CHECK,
     '    return text[i] == "." and 0 <= j < len(text) and bool(_ID_CONTINUES.match(text[j]))',
     '    return False',
     ["(bare) a dotted number is one token: `§6.9z` names no row"]),
    ("#3 bare id: a decorated side is bounded by its decoration", CHECK,
     '        if not tok.group("r") and _glued(text, e, +1):', '        if _glued(text, e, +1):',
     ["(bare) the decoration closes the token even against an id character: `**9z**7z`"]),
    ("#8 stream: the (c) seed reads the Slice cell's disposed stream", ROLES,
     '        body = _stream(row, "Slice")\n        empty = is_empty(deps)',
     '        body = row.col("Slice").text\n        empty = is_empty(deps)',
     ["(c-seed) ordering vocabulary inside a code span is code, not prose"]),
    ("#8 stream: the acceptance seed and RETIRED read the disposed stream", ROLES,
     '        body = _stream(row, "Slice")\n        # the population decided',
     '        body = row.col("Slice").text\n        # the population decided',
     ["(accept-vocab seed) a retirement word inside a code span does not retire the row"]),
    ("#8 stream: the (d) seed reads the disposed stream", ROLES,
     'for m in OWNS_TWO.finditer(_stream(row, "Slice")):', 'for m in OWNS_TWO.finditer(row.col("Slice").text):',
     ["(d) an ownership clause inside a code span is code, not a two-owner claim"]),
    # ⚠ The former row "#8 stream: the licensing rule reads the disposed stream"
    # (`Mention.text` -> `block.text`) is DELETED as an EQUIVALENT mutant, not
    # kept as a survivor: since R12-C the scanners match on the stream, so a
    # mention's `start` / `end` never absorb a masked span's delimiter, and
    # both licensing grammars are anchored at the mention (`LICENSE_BEFORE`
    # `…$`, `LICENSE_AFTER` `^…`) while every masked span is delimited by a
    # character they reject (`` ` `` / `[` / `]` / `(` / `)` / `.md`) -- the
    # raw text and the stream read the same at that site for every fixture.
    # Its control stays (the property holds); the clause it tested is now
    # enforced one step upstream, by R12-C's mutant.
    ("#8 stream: every span of the mask is blanked, not only code", TABLES,
     'return blank_spans(lx.text, [(a, b) for a, b, _ in lx.mask])',
     'return blank_spans(lx.text, [(a, b) for a, b, k in lx.mask if k == "code"])',
     ["(stream) ordering vocabulary in a link TITLE is the link's tail, not prose"]),
    ("#11 identity: per-memo maps are keyed on the resolved path, not the basename", MEMO,
     '        return str(self.path)', '        return self.path.name',
     ["(c-seed) a sibling of the SAME basename in another directory, whose Deps cell at the "
      "same line names the party, does not discharge the main memo's row"]),
    # -- PR #510 Codex R12
    ("R12-A setext: an underline is a boundary only where a paragraph is open (re-inject the "
     "unconditional arm)", BLOCKS,
     'or (para_open and not _is_lazy(lazy, i) and is_setext_underline(lines[i]))',
     'or is_setext_underline(lines[i])',
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
     "'any number of spaces')", BLOCKS,
     '    rest = unindented(line)\n    return rest is not None and pat.match(rest) is not None',
     '    rest = line.lstrip(" ")\n    return pat.match(rest) is not None',
     [SPEC_EXAMPLES]),
    ("R12-C anchors: the anchored pass reads the disposed stream (re-inject the raw text)", CHECK,
     '        for mt in pat.finditer(b.stream):', '        for mt in pat.finditer(b.text):',
     ["(anchor) `` `Slice `C owns it ``: the row noun is inside a code span, so on the disposed "
      "stream there is no `Slice C` to anchor on -- 0 sites (the bare `C` is a declared miss)"]),
    ("R12-D witness: Phase 1's `link_label` calls are counted where Phase 1 makes them (re-bind the "
     "counter to the lexer's binding, which sees only Phase 2)", SELFTEST,
     'with _count_calls(plan_memo_blocks, "link_label", limit=4 * n) as c:',
     'with _count_calls(__import__("plan_memo_lexer"), "link_label", limit=4 * n) as c:',
     ["Phase-1 orphan detection is linear: <= 4 link_label calls per line, t(4N)/t(N) < 8"]),
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
     '                if quote_content(line) is not None:\n                    flush()',
     '                if False:\n                    flush()',
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
     'or (para_open and not _is_lazy(lazy, i) and is_setext_underline(lines[i]))',
     'or (para_open and is_setext_underline(lines[i]))',
     ["(quote) `> Heading `open\\n===\\nSlice 9z owns it` here`: a lazy `===` is the quote paragraph's "
      "text, not an underline (§5.1, Example 93) -- one paragraph, the span masks the site",
      SPEC_EXAMPLES]),
    ("R13 §5.1: a lazy candidate where no paragraph is open ends the quote (drop the stop: it is "
     "parsed inside)", MEMO,
     '                if lazy is not None and lazy[i]:\n                    break',
     '                if False:\n                    break',
     [SEQUENCE]),
    ("R13 §5.1: a lazy candidate is a boundary wherever no paragraph is open (drop the arm: a table's "
     "or a raw extent's next line)", BLOCKS,
     '    if _is_lazy(lazy, i) and not para_open:\n        return True',
     '    if False:\n        return True',
     [SEQUENCE]),
    ("R13 §5.1: a lazy candidate ends a fence inside the quote (re-inject 'the fence runs on')", BLOCKS,
     '        while j < n and not _is_lazy(lazy, j):\n            if fence_closes(arg, lines[j]):',
     '        while j < n:\n            if fence_closes(arg, lines[j]):',
     [SEQUENCE]),
    ("R13 §5.1: neither table row may be lazy (re-inject a lazy header / delimiter row)", BLOCKS,
     '            or _is_lazy(lazy, i) or _is_lazy(lazy, i + 1)):', '            or False):',
     [SEQUENCE]),
    ("R13 §5.1: a quote's lazy candidates are gathered once, up to the first boundary (drop the "
     "bound: every quote re-scans the rest of the document, quadratic -- the result is the same, the "
     "cost is not)", MEMO,
     '                if block_end(content, len(content) - 1, True, inner_lazy):',
     '                if False:',
     ["block quotes are linear: N quotes cost <= 4N quote_content calls"]),
    ("R13 §5.1: lazy continuation (drop it: a marker-less line never joins the quote)", MEMO,
     '                if block_end(content, len(content) - 1, True, inner_lazy):',
     '                if True:',
     ["(quote) `> open `here\\nSlice 9z owns it` there`: the marker-less line is lazy continuation text "
      "of the quote's paragraph (§5.1), so the span crosses it: no site",
      SEQUENCE, SPEC_EXAMPLES]),
    ("R13 §5.2 seed: indented code after a list item's paragraph is recorded for the seed", MEMO,
     '                        self.item_code.extend((linenos[k], lines[k]) for k in range(i, end))',
     '                        pass',
     ["(lex-seed) `- item\\n\\n    Slice 9z owns it`: an indented line after a list item's paragraph is "
      "the item's content under CommonMark (Example 108) and indented code under LEXED-FLAT -- seeded"]),
    ("R13 §5.2 seed: only after a LIST ITEM's paragraph (re-inject 'after any paragraph')", MEMO,
     '                after_item = kind == "p" and list_item_line(cur[0][1])',
     '                after_item = kind == "p"',
     ["(lex-seed) `para\\n\\n    Slice 9z owns it`: indented code after a plain paragraph is a code block "
      "under CommonMark too -- raw, no seed"]),
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
]


def run(reg):
    """Apply each mutant to a fresh module set and re-run its controls.
    Returns the list of FAIL strings (empty = every mutant was killed)."""
    import plan_memo_umbrella_selftest as st

    fails = []
    print()
    print("mutants (each must turn its control red):")
    for name, file, find, replace, controls in MUTANTS:
        src = (st.HERE / file).read_text()
        n = src.count(find)
        if n != 1:
            fails.append("MUTANT %r: substring occurs %d times in %s (must be exactly 1) -- the "
                         "mutant no longer applies and proves nothing" % (name, n, file))
            print("  FAIL [MUTANT] %s (substring x%d)" % (name, n))
            continue
        unknown = [c for c in controls if c not in reg]
        if unknown:
            fails.append("MUTANT %r names unknown control(s) %s" % (name, unknown))
            print("  FAIL [MUTANT] %s (unknown control)" % name)
            continue
        patched = src.replace(find, replace)
        if file == SELFTEST:
            # a mutant against the runner: the checker set is unpatched, the
            # controls come from the PATCHED runner's registry
            M, table = st.load(), st.patched_runner(patched).registry()
        else:
            M, table = st.load({file: patched}), reg
        survived, crash = [], None
        try:
            for c in controls:
                try:
                    ok = table[c][1](M)[0]
                except Exception as e:
                    # an exception is not the control going red: the clause
                    # under test was never exercised, so this proves nothing
                    crash = "%s: %s" % (type(e).__name__, str(e)[:60])
                    break
                if ok:
                    survived.append(c)
        finally:
            st.unload()
        if crash:
            fails.append("MUTANT %r crash: %s" % (name, crash))
        elif survived:
            fails.append("MUTANT %r SURVIVED: control(s) stayed green: %s" % (name, survived))
        print("  %-4s [MUTANT] %s%s" % ("FAIL" if (survived or crash) else "ok", name,
                                        " (crash: %s)" % crash if crash else ""))
    print("%d mutant(s), %d survived, %d crashed."
          % (len(MUTANTS), sum(1 for f in fails if "SURVIVED" in f),
             sum(1 for f in fails if " crash: " in f)))
    return fails
