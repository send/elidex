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
a SELF-TEST module (`SELFTEST` below -- the CONTROLS module or the WORK
module) patches the self-test, not the checker set: the module is exec'd from
the patched text (`plan_memo_selftest_harness.patched_module`) and the row's
controls are taken from the PATCHED module's registry, MERGED over the
unpatched rest.  The merge is what makes the set extensible: a mutant against
a control that lives in the work module takes that control from the patched
text while every other control stays as loaded, and `SELFTEST` -- not a
comparison against one file name -- is the predicate.

The registry is split at the review-round seam, like the controls: this
module holds the row shape, `run`, and every PRE-converge mutant (the lexing
clauses, the gating stages, the `/code-review high` and `/elidex-review`
Stage-6 fixes); `plan_memo_selftest_mutants_pr510.py` holds PR #510's review
rounds R1-R16 and the design re-gates, and `plan_memo_selftest_mutants_inline.py`
every round from R17 on (the Phase-2 inline construct family) -- the cases
modules' own seams.  All three append to this same `MUTANTS` -- one list, filled
by three modules, read at one import site (the runner).
"""

IDS, EMPHASIS, LEXER, BLOCKS, TABLES, MEMO, POPULATION, ROLES, CHECK, CONTROLS, WORK = (
    "plan_memo_ids.py", "plan_memo_emphasis.py", "plan_memo_lexer.py", "plan_memo_blocks.py",
    "plan_memo_tables.py", "plan_memo_memo.py", "plan_memo_population.py", "plan_memo_roles.py",
    "plan-memo-umbrella-check.py", "plan_memo_selftest_controls.py", "plan_memo_selftest_work.py")

# The SELF-TEST modules: a mutant row naming one of these patches the proof,
# not the checker set.  A SET, not a comparison against `CONTROLS`, so the
# next self-test module a touch-time split carves out arrives here rather
# than silently falling into the checker branch (where `load()` would refuse
# a file that is not in the module set).
SELFTEST = frozenset((CONTROLS, WORK))

# The spec-example conformance control (`plan_memo_selftest_conformance.py`):
# the one control a spec-table transcription error turns red.
SPEC_EXAMPLES = "CommonMark 0.31.2 spec examples (Tabs, §4.1-§4.9, §5.1-§5.3): Phase 1's block sequence aligns with the html"
# The block-sequence control over the §4.4 / §5.1 / §5.2 shapes the vendored
# examples do not reach (each expected sequence read off commonmark.js 0.31.2).
SEQUENCE = "Phase 1's block sequence over the §4.4 chunk and the §5.1 / §5.2 container shapes matches commonmark.js"
# The inline half of the conformance control: the spec's §2.4 / §2.5 / §6.1 /
# §6.3 / §6.4 / §6.5 / §6.6
# examples against the spans Phase 2 masks (PR #510 R17).
INLINE_EXAMPLES = ("CommonMark 0.31.2 spec examples (§2.4, §2.5, §6.1-§6.6): Phase 2's inline claim aligns "
                   "with the html")

# `Memo.sibling_path` stage (c) AS IT STANDS: three clauses in one `if` (a C0
# control, a Windows anchor, a reserved component -- PR #510 R22).  Mutants in
# BOTH derived registries drop one clause each and keep the others, so the
# source text is spelled ONCE, here, beside the file names they already share.
# ⚠ It stood as two byte-identical copies, one per derived registry, from the
# R17 registry split until R22 -- and R22 changed this very line, so both had
# to move in lockstep with nothing enforcing it, and a missed one does not fail
# loudly: the runner reports `(unknown control)` and continues.  An anchor two
# rounds share is shared state and lives with the other shared state.
STAGE_C = ('        if (_CONTROL.search(name) or p.anchor                        # (c)\n'
           '                or any(_is_reserved_component(s) for s in p.parts)):')

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
    ("span: a list item starts a block (drop the arm: a marker line never ends a run, nor a sibling item's "
     "lazy gather)", BLOCKS,
     '    m = item_marker(line)\n    if m is None:\n        return False',
     '    m = item_marker(line)\n    if True:\n        return False',
     ["(span) a paragraph ends at a list item: a backtick open in one item and closed in the next is literal",
      SPEC_EXAMPLES]),
    ("span: a `>` line starts a block", BLOCKS,
     '    if one_line_block(line) is not None or quote_content(line) is not None:\n        return True',
     '    if one_line_block(line) is not None:\n        return True',
     ["(span) a paragraph ends at a `>` line"]),
    ("span: an ATX heading is a block", BLOCKS,
     '    m = _ATX.match(rest)\n    if m:', '    m = None\n    if m:',
     ["(span) a paragraph ends at an ATX heading"]),
    ("A x E: kind markers are read from the MASKED declaring field", POPULATION,
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
    ("table: a schema body row of the wrong width is exit 2 (the width miss gates)", POPULATION,
     '                for lineno, msg in t.misses:\n'
     '                    self.misses.append((self.display(memo.path), lineno, msg))',
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
    ("population: every memo's rows are declared (census)", POPULATION,
     '        for memo in self.memos:\n            self._declare(memo)', '        self._declare(self.main)',
     ["(population) an umbrella declared in a linked memo is in the census"]),
    ("population: every memo's ids are in the keep-set", POPULATION,
     '        return set(self.ids)',
     '        return {rid for rid, r in self.ids.items() if r.memo is self.main}',
     ["(population) a terminal id declared in a linked memo is in the keep-set, so `Tq / 9z` is "
      "an id-only run, not code"]),
    ("population: every memo's rows are asserted", POPULATION,
     'return [r for memo in self.memos for r in memo.schema_rows(name)]',
     'return list(self.main.schema_rows(name))',
     ["(b) a sibling umbrella's Deps edge is asserted"]),
    ("population: the link walk is transitive", POPULATION,
     'queue.extend(memo.linked_files())',
     'queue.extend(memo.linked_files() if len(self.memos) == 1 else [])',
     ["(population) the population is transitive: a memo linked from a linked memo is scanned"]),
    ("gate: an absent linked memo is a schema miss", POPULATION,
     '            except (OSError, UnicodeDecodeError) as e:\n                self.misses.append(',
     '            except (OSError, UnicodeDecodeError) as e:\n                [].append(',
     ["(rc) a linked memo that is not on disk is rc 2, never clean"]),
    ("gate: an unmatched schema is a schema miss", POPULATION,
     '                if s.name not in matched:', '                if False:',
     ["(rc) a schema with no matching table is rc 2"]),
    ("gate: a duplicate declaration is a schema miss", POPULATION,
     '                if rid in self.ids:', '                if False:',
     ["(rc) the same id declared in two memos is rc 2"]),
    ("gate: KIND-SPELLING is a mechanical finding", CHECK,
     '    if len(pop.spellings) > 1:', '    if False:',
     ["(rc) the undetermined kind written two ways is KIND-SPELLING, rc 1"]),
    ("gate: a mechanical finding is exit 1", CHECK,
     'rc = 1 if mechanical else 0', 'rc = 0',
     ["(rc) the undetermined kind written two ways is KIND-SPELLING, rc 1"]),
    # -- /code-review high: one mutant per fix
    ("F1 id cell: the id is followed by a non-id character, not the cell end", TABLES,
     'return t.id if t is not None and t.start == 0 and t.kind in kinds else None',
     'return t.id if t is not None and t.start == 0 and t.kind in kinds and t.end == len(cell_text.strip(" \\t")) else None',
     ["(id) an id cell with trailing prose declares the id at its start",
      "(id) a backticked slug with trailing prose declares the slug"]),
    ("F1 id cell: a cell not starting with an id declares nothing (no fallback to the cell)", TABLES,
     'return t.id if t is not None and t.start == 0 and t.kind in kinds else None',
     'return t.id if t is not None and t.start == 0 and t.kind in kinds else cell_text.strip()',
     ["(id) a cell that does not start with an id declares nothing: the row is unkeyed (its "
      "Deps edge would go unasserted), so the run is a schema miss"]),
    ("#2 gate: an unkeyed schema row is a schema miss (not a note, not a silent drop)", POPULATION,
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
     # the match goes through `MARKER_RE` (bounded) since PR #510 R22; the
     # MUTATION is untouched -- the LAST occurrence decides instead of the first
     '    m = MARKER_RE.search(field)',
     '    m = list(MARKER_RE.finditer(field))[-1]',
     ["(a) a self-declaring field that later says a sibling 'is not it' stays self-declaring"]),
    ("F5 kind: the undetermined spelling is collected beside the marker", POPULATION,
     '        if hit["undetermined"]:\n            self.spellings.add(hit["undetermined"].group(0))',
     # both sides read out of `hit` since PR #510 R23, when the three kind phrases
     # became one tuple: a mutant may name only what the mutated file has (spelling
     # this as `MARKER.search` crashed with a NameError at R22 -- a crash is a
     # FAIL); the MUTATION is untouched -- the spelling collected only where the
     # marker is absent
     '        if hit["undetermined"] and not hit["marker"]:\n'
     '            self.spellings.add(hit["undetermined"].group(0))',
     ["(rc) a row carrying the marker AND one undetermined spelling, beside another row's other "
      "spelling, is KIND-SPELLING rc 1"]),
    ("F6 (c): the Deps cell's ids are the population's mentions, not a raw tokenisation", ROLES,
     'cell_ids = in_deps.get(_row_key(row), set())',
     'cell_ids = set(re.findall(__import__("plan_memo_ids").SHORT_ID, deps))',
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
    ("4.5 id cell: blanks are LITERAL, not the shape rule (re-inject `is_empty`)", POPULATION,
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
     'exempt = is_cite_label(key) or form == "shortcut"',
     'exempt = form == "shortcut"',
     ["(link) adjacent citations `[C19][C20]` are not a full reference: rc 0",
      "(link) a collapsed-shaped citation `[C19][]` is not a reference: rc 0"]),
    ("#4 empty cell: a word outside the lexical exceptions is NOT empty", TABLES,
     'EMPTY_WORDS = frozenset({"n/a", "none"})', 'EMPTY_WORDS = frozenset({"n/a", "none", "nil"})',
     ["(b) a Deps cell `nil` -- a word outside the lexical exceptions -- is NOT empty: the "
      "stated polarity is a reported edge (false rc 1), never a silent skip"]),
    ("F9 span: an escaped backtick opens no span", LEXER,
     '        if _is_escape(s, i):\n            subst.append((i, i + 2, s[i + 1]))  # §2.4: `\\[` renders the character alone',
     '        if _is_escape(s, i) and s[i + 1] != "`":\n            subst.append((i, i + 2, s[i + 1]))',
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
    ("#1 gate: an unresolved reference is a schema miss (rc 2), not a note", POPULATION,
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
     '            if t.kind == "slug" and t.id in keep:', '            if False:',
     ["(span) a kept slug inside a command-line code span is a naming site"]),
    # -- /elidex-review Stage 6
    ("#3 bare id: bounded by the complement of the id-continuation class (re-inject a list)", IDS,
     '    "short": re.compile(ALNUM),', '    "short": re.compile(r"[^\\s,;/()\\[\\]*`.:]"),',
     ["(bare) an id before `?` is bounded", "(bare) an id before `!` is bounded",
      "(bare) an id inside ASCII double quotes is bounded",
      "(bare) an id inside curly double quotes is bounded"]),
    ("#3 / R14-1 bare id: a hyphen bounds a short id, in prose and on a raw line (re-inject it into the "
     "class)", IDS,
     '    "short": re.compile(ALNUM),', '    "short": re.compile("[%s-]" % ALNUM_CHARS),',
     ["(bare) a hyphen bounds a short id: `after 9z-7z` names 9z",
      "(lex-seed) a raw HTML line `9z-owner`: a hyphen bounds the short id on the raw line as in "
      "prose, so the line is seeded holding `9z`",
      "(lex-seed) a raw HTML line `owner-9z`: the hyphen bounds on the left too -- seeded holding `9z`"]),
    ("#3 file token: a bare `.md` file name is masked before the bare scan", LEXER,
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))*%s(?!%s))', '|(?P<file>(?!)%s%s)',
     ["(bare) a bare `.md` file name holding an id is a file token, not a site"]),
    ("#3 bare id: a dotted number is one token", IDS,
     '    return kind == "short" and text[i] == "." and lo <= j < hi and bool(cont.match(text[j]))',
     '    return False',
     ["(bare) a dotted number is one token: `§6.9z` names no row"]),
    ("#3 bare id: a decorated side is bounded by its decoration", IDS,
     '        if not t.r and _glued(text, t.end, +1, t.kind, pos, hi):',
     '        if _glued(text, t.end, +1, t.kind, pos, hi):',
     ["(bare) the decoration closes the token even against an id character: `**9z**7z`"]),
    ("#8 stream: the (c) seed reads the Slice cell's disposed stream", ROLES,
     '        body = _stream(row, "Slice")\n        empty = is_empty(_stream(row, "Deps"))',
     '        body = row.col("Slice").text\n        empty = is_empty(_stream(row, "Deps"))',
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
     '        v = 1 if RENDERS_TEXT[kind] else 2',
     '        v = (1 if RENDERS_TEXT[kind] else 2) if kind == "code" else 0',
     ["(stream) ordering vocabulary in a link TITLE is the link's tail, not prose"]),
    ("#11 identity: per-memo maps are keyed on the resolved path, not the basename", MEMO,
     '        return str(self.path)', '        return self.path.name',
     ["(c-seed) a sibling of the SAME basename in another directory, whose Deps cell at the "
      "same line names the party, does not discharge the main memo's row"]),
]


def run(reg):
    """Apply each mutant to a fresh module set and re-run its controls.
    Returns the list of FAIL strings (empty = every mutant was killed)."""
    import plan_memo_selftest_harness as h

    fails = []
    print()
    print("mutants (each must turn its control red):")
    for name, file, find, replace, controls in MUTANTS:
        src = (h.HERE / file).read_text()
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
        if file in SELFTEST:
            # a mutant against the self-test: the checker set is unpatched, and
            # the PATCHED module's registry fragment is merged over the loaded
            # one, so the row's controls come from the patched text whichever
            # self-test module they live in
            M, table = h.load(), dict(reg)
            table.update(h.patched_module(file, patched).registry())
        else:
            M, table = h.load({file: patched}), reg
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
            h.unload()
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
