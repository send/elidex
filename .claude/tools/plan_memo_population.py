#!/usr/bin/env python3
"""The memo SET reachable from one memo -- for `plan-memo-umbrella-check.py`.

`Population` is the transitive closure over the memos a memo links (a visited
set, so a cycle is not an error) and the ONE map `ids` every scan and
assertion reads.  ONE memo -- its block structure, its lexing and its sibling
resolution -- is `plan_memo_memo.py`'s `Memo`, imported here and never the
reverse; the row grammar, the table schemas and the mask disposition are
`plan_memo_tables.py`'s.

Two rules decided here, once:
  * a memo that cannot be opened, read or decoded is an UNAVAILABLE linked
    memo -- the documented exit-2 miss at the population's one I/O
    chokepoint, never an exception out of the population; an exception
    from PARSING it is a crash out of the population (crash = FAIL), never
    that miss -- the chokepoint catches `OSError` and `UnicodeDecodeError`
    and nothing else (⚠ until PR #510 R16 it caught `RuntimeError` too,
    for the py<=3.12 symlink-loop case that `plan_memo_memo._resolve` alone
    guards, and a `RecursionError` -- a `RuntimeError` -- out of ~500 nested
    `>` markers was reported as "linked memo unavailable", rc 2, no census);
  * the population is transitive over the memos a memo links; the same id
    declared twice, and a reference no definition answers (the memo it meant
    to link is outside the population), are schema misses.
"""

import pathlib

from plan_memo_memo import Memo, _resolve
from plan_memo_tables import (
    KIND_PHRASES, SCHEMAS, attributed_to_other, dispose, is_blank_id_cell,
    kind_disagreements, stream,
)


class Population:
    """The memo set reachable from one memo through its links (visited set, so
    a cycle is not an error), with ONE map `ids`: id -> its declaring `Row`
    (`row.kind` = "umbrella" / "undetermined" / "pointer" / "terminal").  `misses` holds
    every schema miss (absent memo, unresolved reference, unmatched schema, row
    width, duplicate declaration, unkeyed schema row); a non-empty `misses`
    is exit 2 -- never a clean run.
    """

    def __init__(self, main_path):
        self.memos = []
        self.misses = []            # [(file, lineno, message)]
        self.spellings = set()
        self.attributed = []        # [(file, table, lineno, row name (`Row.name`), other)]
        self.ids = {}
        queue, seen = [_resolve(pathlib.Path(main_path))], set()
        self.root = queue[0].parent     # the root memo's directory: what `display` names relative to
        while queue:
            p = queue.pop(0)
            if p in seen:
                continue
            seen.add(p)
            # the ONE I/O chokepoint: a memo that cannot be opened, read or
            # decoded (absent, a directory, over-long, invalid UTF-8) is an
            # UNAVAILABLE linked memo -- the documented exit-2 miss, never an
            # exception out of the population.  I/O ONLY: `Memo(p)` also
            # PARSES, and a parser exception must surface as a crash (crash =
            # FAIL), never as this miss -- so no `RuntimeError` here (its one
            # legitimate source, the py<=3.12 symlink-loop `resolve()`, is
            # guarded at `_resolve`; a `RecursionError` IS a `RuntimeError`,
            # and PR #510 R16 found ~500 nested `>` reported as "unavailable")
            try:
                memo = Memo(p)
            except (OSError, UnicodeDecodeError) as e:
                self.misses.append((self.display(p), 0, "linked memo unavailable (%s) -- its population "
                                    "is unscanned" % type(e).__name__))
                continue
            self.memos.append(memo)
            queue.extend(memo.linked_files())
            # a reference no definition answers is prose under §6.3, and the
            # memo it meant to link is NOT in the population: never a clean run
            for lineno, label in memo.unresolved_references():
                self.misses.append((self.display(memo.path), lineno,
                                    "unresolved reference %r -- no definition answers it, so a memo "
                                    "it meant to link is NOT in the population" % label))
        self.main = self.memos[0] if self.memos else None
        if self.main is not None:
            matched = {t.schema.name for t in self.main.tables if t.schema is not None}
            for s in SCHEMAS:
                if s.name not in matched:
                    self.misses.append((self.display(self.main.path), 0,
                                        "no table matched schema %r -- its whole population is unscanned" % s.name))
        for memo in self.memos:
            for t in memo.tables:
                for lineno, msg in t.misses:
                    self.misses.append((self.display(memo.path), lineno, msg))
        # ids first (the keep-set the disposition needs), then masks, then kinds
        for memo in self.memos:
            self._declare(memo)
        keep = self.keep()
        for memo in self.memos:
            for lx in memo.lexed():
                dispose(lx, keep)
        for row in self.declaring_rows():
            row.field = stream(row.cells[row.schema.decl].lexed)
        for row in self.ids.values():
            row.kind = self._kind(row)
            self._kind_residue(row)

    def display(self, path):
        """The ONE display name of a memo, for every printer -- a finding's
        file column, the worklist, the population summary: the memo's
        resolved `path` RELATIVE to the root memo's directory (`root`), or
        the resolved absolute path when it is not under that directory.
        Identity is `Memo.key`; a basename is neither -- two memos in
        different directories may share one, and `a/child.md:1` and
        `b/child.md:1` printed as `child.md:1` named two sites as one (PR
        #510 R19)."""
        try:
            return str(path.relative_to(self.root))
        except ValueError:
            return str(path)

    # -- declarations ------------------------------------------------------

    def _declare(self, memo):
        for s in SCHEMAS:
            if s.idc is None:
                continue
            for row in memo.schema_rows(s.name):
                rid = row.self_id
                if rid is None:
                    # a LITERAL blank id cell is a deliberate non-row; anything
                    # else that is not an id THIS TABLE KEYS (`Schema.kinds`,
                    # applied in `bare_id`) is an UNKEYED row: it would be
                    # dropped from `ids`, so assertion (b) would never see its
                    # Deps edge -- the I-C silent-skip class, and a schema
                    # miss.  ONE message for both shapes, since they are one
                    # question: a citation-shaped id in a slice table keys no
                    # slice row, and both mention passes ignore it, so its
                    # ownership text could never be checked (PR #510 R21)
                    if not is_blank_id_cell(row.id_cell()):
                        self.misses.append((self.display(memo.path), row.lineno,
                                            "the %r row's id cell does not start with an id of a kind this "
                                            "table keys (%s): %r; the row declares nothing and is unkeyed, "
                                            "so its cells would go unasserted"
                                            % (s.name, " / ".join(s.kinds), row.id_cell()[:60])))
                    continue
                if rid in self.ids:
                    r2 = self.ids[rid]
                    self.misses.append((self.display(memo.path), row.lineno,
                                        "row %s is declared twice (also %s:%d in %r); a population "
                                        "with two declarations of one id cannot be scanned"
                                        % (row.name(), self.display(r2.memo.path), r2.lineno, r2.schema.name)))
                    continue
                self.ids[rid] = row

    def _kind(self, row):
        """The kind the row's masked declaring field declares.  The
        undetermined SPELLING is collected independently of the marker (a row
        can carry both; its kind stays umbrella, its spelling still joins
        `spellings`).  A row whose marker is attributed to another row is a
        POINTER (§5: a pointer slot carries no marker of its own -- assertion
        (a) reports it), as is a row that says so in words.

        Every phrase read here comes from `KIND_PHRASES` and none is matched
        directly, so a phrase that decides a kind is necessarily a member --
        and so is necessarily gated by `_kind_residue`, which iterates the
        same tuple.  The ORDER between the members is this function's (the
        marker outranks the undetermined spelling, which outranks the pointer
        phrase); their MEMBERSHIP is not."""
        if row.field is None:
            return "terminal"
        hit = {name: rx.search(row.field) for name, rx in KIND_PHRASES}
        if hit["undetermined"]:
            self.spellings.add(hit["undetermined"].group(0))
        if hit["marker"]:
            other = attributed_to_other(row.field, row.self_id)
            if other:
                self.attributed.append((self.display(row.memo.path), row.schema.name, row.lineno, row.name(), other))
                return "pointer"
            return "umbrella"
        if hit["undetermined"]:
            return "undetermined"
        if hit["pointer"]:
            return "pointer"
        return "terminal"

    def _kind_residue(self, row):
        """The one place the residue GATES (`plan_memo_tables.split_units`),
        for EVERY member of `KIND_PHRASES` and not for the one member that
        was in front of me when I wrote it.  A declaring field that spells a
        kind phrase ACROSS a span the checker does not read as prose --
        `**UMBRELLA, not a `terminal` unit.**`, `KIND UNDETER`MINED`` -- is
        read here as no such declaration, so the row would leave the umbrella
        census as an active terminal, silently, at rc 0: §1's "never a clean
        exit for could not scan" over the census this program exists to take.
        The row is a schema miss instead, and a reader decides whether the
        code span is a quotation or a typo (a kind phrase QUOTED WHOLE stays
        what I-A says it is: not a declaration, not a straddle, no miss).

        The other direction is the same miss and is gated by the same call:
        `KIND `x` UNDETERMINED` is the undetermined kind to the STREAM only
        because a blank stands as spaces, and no kind at all to a reader --
        the kind this row was just assigned is then the one nobody reads.
        `kind_disagreements` asks both directions per phrase, so neither the
        phrase nor the direction is enumerated here."""
        if row.field is None:
            return
        for name in kind_disagreements(row.cells[row.schema.decl].lexed):
            self.misses.append((self.display(row.memo.path), row.lineno,
                                "row %s spells the %s kind phrase in its declaring field ACROSS a "
                                "span this checker does not read as prose (a code span, an autolink, "
                                "a citation id or a file name): the two readings of the field "
                                "disagree about the phrase, so the row's kind cannot be decided"
                                % (row.name(), name)))

    # -- inventories -------------------------------------------------------

    def ids_of_kind(self, kind):
        return {rid: r for rid, r in self.ids.items() if r.kind == kind}

    def no_owner_ids(self):
        """Every row that carries no owner and no ordering -- the property §5's
        naming rule is stated over (umbrella + kind-undetermined)."""
        return {rid: r for rid, r in self.ids.items() if r.kind in ("umbrella", "undetermined")}

    def data_rows(self, name):
        """[Row] over every memo, for schema `name`."""
        return [r for memo in self.memos for r in memo.schema_rows(name)]

    def declaring_rows(self):
        """Every row of a schema with a declaring field and an id column, over
        every memo -- the set `field` is written for and read over (assertion
        (a)).  It is NOT `ids.values()`: a row whose id cell is EMPTY (`**—**`)
        is a deliberate non-row, unkeyed and outside `ids`, but its declaring
        field is still read (the #506 memo has one such row, the
        `Function`/`eval` row); a non-empty non-id cell is a schema miss, so
        after `misses` those two sets differ by exactly the empty-id rows."""
        return [r for s in SCHEMAS if s.decl is not None and s.idc is not None
                for r in self.data_rows(s.name)]

    def keep(self):
        """The code-span keep-set: every declared id, from every memo."""
        return set(self.ids)
