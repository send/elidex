#!/usr/bin/env bash
# Controls for claim-provenance-trip-wire.sh.
#
# Every clause the wire implements gets a control that can red on its
# regression, and where a clause has a passing direction, one that can red on
# that. A control which cannot fire is indistinguishable from a clean tree --
# so each `fail` probe also names a substring the diagnostic must contain,
# because two different defects of one fixture produce the same exit code.
#
# Fixture model:
#   own        its own mktemp -d and its own wire invocation, torn down after.
#              Two `own` rows cannot mask each other, whatever their $3.
#   escapes    one fixture, two arms, ONE invocation -- so their diagnostics
#              must discriminate. Both arms are malformed, which is why the
#              fixture exists at all only under the fail-without-ending rule.
#   escapes10  the same shape at ten arms, one per rejected escape.
#   clone      a `git clone --local` of this repository, built once and reused,
#              one wire invocation per row, annotation planted and removed.

set -euo pipefail

WIRE_DIR=$(cd "$(dirname "$0")" && pwd)
WIRE="$WIRE_DIR/claim-provenance-trip-wire.sh"
PASS=0; FAIL=0

pass_() { PASS=$((PASS+1)); printf '  ok    %s\n' "$1"; }
fail_() { FAIL=$((FAIL+1)); printf '  FAIL  %s\n       %s\n' "$1" "$2"; }

# Run the wire in $1 with the recursion guard set; echo status and output.
run_wire() { # $1 = dir
  local out rc=0
  out=$(cd "$1" && CLAIM_WIRE_SELFTEST=1 bash "$WIRE" 2>&1) || rc=$?
  printf '%s\n' "$rc"
  printf '%s\n' "$out"
}

st_probe() { # $1 pass|fail  $2 label  $3 dir  $4 required diagnostic substring
  local res rc out
  res=$(run_wire "$3"); rc=${res%%$'\n'*}; out=${res#*$'\n'}
  if [ "$1" = pass ]; then
    [ "$rc" -eq 0 ] && { pass_ "$2"; return; }
    fail_ "$2" "should have passed, exited $rc: $(printf '%s' "$out" | head -2 | tr '\n' ' ')"
    return
  fi
  if [ "$rc" -eq 0 ]; then
    fail_ "$2" "was NOT caught — this control cannot fire"; return
  fi
  case "$out" in
    *"$4"*) pass_ "$2" ;;
    *) fail_ "$2" "failed for the wrong reason; wanted [$4], got: $(printf '%s' "$out" | head -2 | tr '\n' ' ')" ;;
  esac
}

# A fresh `own` fixture: a git repo carrying the wire and this helper, so the
# wire finds its helper beside itself and the guard stops the recursion.
mkfixture() {
  local d; d=$(mktemp -d)
  mkdir -p "$d/.claude/tools"
  cp "$WIRE" "$d/.claude/tools/"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$d/.claude/tools/claim-provenance-selftest.sh"
  ( cd "$d" && git init -q . )
  printf '%s' "$d"
}
add() { ( cd "$1" && git add -A >/dev/null 2>&1 ); }

# --------------------------------------------------------------------------
# C15 / C14 -- count semantics, and count=0 must not abort
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'sole sole\n' > "$d/a.md"
printf 'x <!-- claim count=1 pattern="sole" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C14 positive  count = matching lines, not matches" "$d" ""
printf 'x <!-- claim count=2 pattern="sole" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C14 negative  a line with two matches counts once" "$d" "asserted=2 actual=1"
rm -rf "$d"

d=$(mkfixture); printf 'nothing here\n' > "$d/a.md"
printf 'x <!-- claim count=0 pattern="zzz" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C15 positive  count=0 is legal and must not abort" "$d" ""
printf 'x <!-- claim count=1 pattern="zzz" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C15 negative  a false zero-claim reds" "$d" "asserted=1 actual=0"
rm -rf "$d"

# --------------------------------------------------------------------------
# C40 -- `files` counts FILES, not matching lines. Round 8 found the verb had no
# control at all: mutating `-lE` to `-nE` (so `files=` silently counted lines)
# reddened nothing, while §5 ships a live `files=` annotation that runs on every
# push. The fixture discriminates the two: three files, five matching lines.
# --------------------------------------------------------------------------
# The data lives in a subdirectory the annotation's own file is not in: with
# `in="*.md"` the annotation matches its own `pattern=` literal and counts
# itself, which is a fixture defect that reds both arms and hides the verb.
d=$(mkfixture); mkdir "$d/data"
printf 'hit\nhit\n' > "$d/data/one.md"; printf 'hit\nhit\n' > "$d/data/two.md"; printf 'hit\n' > "$d/data/three.md"
printf 'x <!-- claim files=3 pattern="hit" in="data/*.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C40 positive  files= counts files (3), not matching lines (5)" "$d" ""
printf 'x <!-- claim files=5 pattern="hit" in="data/*.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C40 negative  5 is the line count, and it is wrong" "$d" "asserted=5 actual=3"
rm -rf "$d"

# --------------------------------------------------------------------------
# C4 / C5 / C24 / C36 -- the backtick arm, on each surface, both directions.
# Wrong-count-inert / right-shape-live, so an inertness regression reds the
# POSITIVE rather than leaving it vacuously green.
# --------------------------------------------------------------------------
for surface in md sh toml rs; do
  case $surface in
    md) comment='%s\n' ;; sh|toml) comment='# %s\n' ;; rs) comment='// %s\n' ;;
  esac
  case $surface in md) id=C4 ;; sh) id=C5 ;; toml) id=C24 ;; rs) id=C36 ;; esac
  d=$(mkfixture); printf 'target\n' > "$d/a.md"
  # backticked, deliberately wrong count -> must be ignored
  printf "$comment" 'x `<!-- claim count=9 pattern="target" in="a.md" -->`' > "$d/n.$surface"; add "$d"
  st_probe pass "$id positive  backticked annotation in .$surface is inert" "$d" ""
  # same line unbackticked -> must red
  printf "$comment" 'x <!-- claim count=9 pattern="target" in="a.md" -->' > "$d/n.$surface"; add "$d"
  st_probe fail "$id negative  the same line unbackticked is live" "$d" "asserted=9 actual=1"
  rm -rf "$d"
done

# --------------------------------------------------------------------------
# C35 -- a fenced annotation is STRICT, not inert (there is no fence arm)
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'target\n' > "$d/a.md"
printf '```\n<!-- claim count=1 pattern="target" in="a.md" -->\n```\n' > "$d/n.md"; add "$d"
st_probe pass "C35 positive  a fenced annotation executes (true count)" "$d" ""
printf '```\n<!-- claim count=9 pattern="target" in="a.md" -->\n```\n' > "$d/n.md"; add "$d"
st_probe fail "C35 negative  a fenced annotation is not inert" "$d" "asserted=9"
rm -rf "$d"

# --------------------------------------------------------------------------
# C1 -- the production is not anchored: an annotation in a table cell is strict
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'target\n' > "$d/a.md"
printf '| a | <!-- claim count=1 pattern="target" in="a.md" --> | b |\n' > "$d/n.md"; add "$d"
st_probe pass "C1  positive  an annotation inside a table cell executes" "$d" ""
printf '| a | <!-- claim count=9 pattern="target" in="a.md" --> | b |\n' > "$d/n.md"; add "$d"
st_probe fail "C1  negative  the same cell with a wrong count reds" "$d" "asserted=9"
rm -rf "$d"

# --------------------------------------------------------------------------
# C17 -- self-matching is permitted; the count includes the annotation's line
# --------------------------------------------------------------------------
d=$(mkfixture)
printf 'x <!-- claim count=1 pattern="claim count" in="n.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C17 positive  an annotation may match its own line" "$d" ""
printf 'x <!-- claim count=0 pattern="claim count" in="n.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C17 negative  excluding its own line reds" "$d" "asserted=0 actual=1"
rm -rf "$d"

# --------------------------------------------------------------------------
# C6 / C7 -- malformed, and its token. No positive: malformed has no passing
# direction (§2 I1×I6 licenses exactly this).
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'x <!-- claim cont=3 pattern="x" in="a.md" -->\n' > "$d/n.md"
printf 'a\n' > "$d/a.md"; add "$d"
st_probe fail "C6  negative  an unknown verb is MALFORMED, with the token" "$d" "MALFORMED"
rm -rf "$d"

d=$(mkfixture); printf 'x <!-- claim lines=5 pattern="fn" in="a.md" -->\n' > "$d/n.md"
printf 'a\n' > "$d/a.md"; add "$d"
st_probe fail "C7  negative  lines= takes no pattern" "$d" "lines= takes no pattern"
rm -rf "$d"

# --------------------------------------------------------------------------
# C8 / C9 -- the escapes fixture: TWO arms, ONE invocation. Both malformed, so
# the row exists only under the fail-without-ending rule, and the diagnostics
# must discriminate within the invocation.
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'a1 b2\n' > "$d/a.md"
{ printf 'x <!-- claim count=1 pattern="a\\d" in="a.md" -->\n'
  printf 'y <!-- claim count=1 pattern="b\\b" in="a.md" -->\n'; } > "$d/n.md"; add "$d"
st_probe fail "C8  negative  \\d is rejected, naming that escape" "$d" 'contains \d'
st_probe fail "C9  negative  \\b is rejected, naming that escape" "$d" 'contains \b'
rm -rf "$d"

# --------------------------------------------------------------------------
# C25 -- all ten of bBdDsSwW<>, one fixture, ten arms, each naming its own
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'sample text\n' > "$d/a.md"
: > "$d/n.md"
for e in b B d D s S w W '<' '>'; do
  printf 'x <!-- claim count=1 pattern="q\\%s" in="a.md" -->\n' "$e" >> "$d/n.md"
done
add "$d"
for e in b B d D s S w W '<' '>'; do
  st_probe fail "C25 negative  \\$e is rejected, naming \\$e" "$d" "contains \\$e"
done
rm -rf "$d"

# --------------------------------------------------------------------------
# C37 -- the reject set is EXACTLY those ten: a backslash outside it is
# accepted and measured. Without this, a wire rejecting every backslash passes
# C8, C9 and all ten of C25 vacuously.
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'fn (x)\nfn y\n' > "$d/a.md"
printf 'x <!-- claim count=1 pattern="fn \\(" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C37 positive  a backslash outside the reject set is measured" "$d" ""
printf 'x <!-- claim count=2 pattern="fn \\(" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C37 negative  and its count is checked" "$d" "asserted=2 actual=1"
rm -rf "$d"

# --------------------------------------------------------------------------
# C23 -- the optional icase keyword is READ, not ignored
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'SOLE\nsole\n' > "$d/a.md"
printf 'x <!-- claim count=2 icase pattern="sole" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C23 positive  icase makes the match case-insensitive" "$d" ""
printf 'x <!-- claim count=2 pattern="sole" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C23 negative  without icase the same count is wrong" "$d" "asserted=2 actual=1"
rm -rf "$d"

# --------------------------------------------------------------------------
# C13 -- `-e` is not optional: a pattern beginning with `-` is read as an
# option and git exits 129, which rule 4 turns into a measurement failure.
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'fn a() -> u32 {}\n' > "$d/a.md"
printf 'x <!-- claim count=1 pattern="-> u32" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C13 positive  a leading-dash pattern measures (so -e is present)" "$d" ""
rm -rf "$d"

# --------------------------------------------------------------------------
# C29 / C32 -- rule 4: a measurement the program refused to run is its own
# outcome, and never reports actual=0.
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'a1\n' > "$d/a.md"
printf 'x <!-- claim count=1 pattern="[0-9]" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C29 positive  a valid ERE is measured" "$d" ""
printf 'x <!-- claim count=0 pattern="foo(" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C29 negative  git refusing the pattern is not actual=0" "$d" "the measurement command failed"
rm -rf "$d"

d=$(mkfixture); printf 'a\nb\n' > "$d/a.md"
printf 'x <!-- claim lines=2 in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C32 positive  lines= measures a readable file" "$d" ""
chmod 000 "$d/a.md"
st_probe fail "C32 negative  an unreadable file is a measurement failure, not 0" "$d" "the measurement command failed"
chmod 644 "$d/a.md"; rm -rf "$d"

# --------------------------------------------------------------------------
# C31 -- lines counts LINES, not newlines: the wc -l value reds the positive
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'a\nb' > "$d/a.md"        # no trailing newline
printf 'x <!-- claim lines=2 in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C31 positive  a file with no trailing newline has 2 lines" "$d" ""
printf 'x <!-- claim lines=1 in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C31 negative  1 is the wc -l value, and it is wrong" "$d" "asserted=1 actual=2"
rm -rf "$d"

# --------------------------------------------------------------------------
# C28 / C33 -- liveness against the WORKING TREE, and the file-type test.
# git ls-files reports the index; all three verbs read the worktree.
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'a\nb\n' > "$d/a.md"
printf 'x <!-- claim lines=2 in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C28 positive  a present tracked file measures" "$d" ""
rm -f "$d/a.md"
st_probe fail "C28 negative  indexed-but-absent is refused, and says which" "$d" "present in the working tree"
rm -rf "$d"

d=$(mkfixture); mkdir "$d/realdir"; printf 'x\n' > "$d/realdir/x.md"
( cd "$d" && ln -s realdir link ); printf 'a\nb\n' > "$d/ok.md"
printf 'x <!-- claim lines=2 in="ok.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C33 positive  a regular file measures" "$d" ""
printf 'x <!-- claim lines=2 in="link" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C33 negative  a tracked symlink-to-directory is refused on type" "$d" "not a regular file"
rm -rf "$d"

# --------------------------------------------------------------------------
# C10 / C11 -- liveness, dead pathspec and the lines= arity rule
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'a\n' > "$d/a.md"
printf 'x <!-- claim count=0 pattern="zzz" in="nosuch/**/*.rss" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C10 negative  a pathspec matching no tracked file is refused" "$d" "matches no tracked file"
rm -rf "$d"

d=$(mkfixture); printf 'a\n' > "$d/one.md"; printf 'b\n' > "$d/two.md"
printf 'x <!-- claim lines=1 in="*.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C11 negative  lines= must resolve to exactly one file" "$d" "more than one tracked file"
rm -rf "$d"

# --------------------------------------------------------------------------
# C16 -- D2 self-exclusion, BOTH directions. The positive needs a tree whose
# only annotation is excluded, i.e. a tree the scan finds nothing in.
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'a\n' > "$d/a.md"
printf 'x <!-- claim count=9 pattern="a" in="a.md" -->\n' > "$d/.claude/tools/n.md"; add "$d"
st_probe pass "C16 positive  a wrong annotation under .claude/tools is not scanned" "$d" ""
cp "$d/.claude/tools/n.md" "$d/n.md"; add "$d"
st_probe fail "C16 negative  the byte-identical annotation one level up reds" "$d" "asserted=9"
rm -rf "$d"

# --------------------------------------------------------------------------
# C18 -- the wire requires its helper BY NAME, and invokes it. Arm (b) needs
# the guard cleared, and a stub rather than the real helper, so the recursion
# has nowhere to go.
# --------------------------------------------------------------------------
d=$(mktemp -d); mkdir -p "$d/.claude/tools"; cp "$WIRE" "$d/.claude/tools/"
( cd "$d" && git init -q . && git add -A >/dev/null 2>&1 )
# The guard-SET path must fail too. Round 8 found this arm inverted: it passed
# when the guard suppressed the check and called that "(documented)", while
# §4.1.2 documents the opposite. An arm rewritten to assert the defect is worse
# than a missing arm, because it reports coverage.
out=$( cd "$d" && CLAIM_WIRE_SELFTEST=1 bash .claude/tools/claim-provenance-trip-wire.sh 2>&1 ) || true
case "$out" in
  *"self-test helper"*) pass_ "C18 (a)  guard set: a missing helper still fails" ;;
  *) fail_ "C18 (a)  guard set: a missing helper still fails" \
       "the guard suppressed the check; an inherited env var must not disable the gate" ;;
esac
# The guard is INHERITED, so these two arms must clear it explicitly. Run
# standalone the self-test has no guard set and they passed either way;
# invoked by the wire (which sets it) they exercised a wire that skips the
# very step C18 exists to prove. Found by running through the driver.
rc=0; out=$( cd "$d" && env -u CLAIM_WIRE_SELFTEST bash .claude/tools/claim-provenance-trip-wire.sh 2>&1 ) || rc=$?
case "$out" in
  *"self-test helper"*) pass_ "C18 (a)  guard cleared: the missing helper is named" ;;
  *) fail_ "C18 (a)  guard cleared: the missing helper is named" "got: $(printf '%s' "$out" | head -1)" ;;
esac
printf '#!/usr/bin/env bash\necho STUB-SELFTEST-RAN\nexit 3\n' > "$d/.claude/tools/claim-provenance-selftest.sh"
rc=0; out=$( cd "$d" && env -u CLAIM_WIRE_SELFTEST bash .claude/tools/claim-provenance-trip-wire.sh 2>&1 ) || rc=$?
if [ "$rc" -ne 0 ] && case "$out" in *STUB-SELFTEST-RAN*) true ;; *) false ;; esac; then
  pass_ "C18 (b)  the stub ran and its status propagated"
else
  fail_ "C18 (b)  the stub ran and its status propagated" "rc=$rc out=$(printf '%s' "$out" | head -1)"
fi
rm -rf "$d"

# --------------------------------------------------------------------------
# C12 -- argv fidelity (I5), in all three places a value crosses into a
# command. Round 8 shipped this clause with no control at all, and the wire was
# violating it: `set -- $body` glob-expanded the field split, so an annotation
# body containing `*` reported a filename from the working directory as its
# first field. Which filename depended on where the wire ran.
# --------------------------------------------------------------------------
d=$(mkfixture); printf 'zzz\n' > "$d/aaa.md"; printf 'q\n' > "$d/bbb.md"
printf 'x <!-- claim * count=1 pattern="zzz" in="aaa.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C12 (a)  a glob in the body is a field, not a pathname expansion" "$d" "first field: *"
rm -rf "$d"

# A pathspec with a space is one argv element or it is two pathspecs.
d=$(mkfixture); mkdir "$d/two words"; printf 'hit\nhit\n' > "$d/two words/a.md"
printf 'x <!-- claim count=2 pattern="hit" in="two words/a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C12 (b)  a pathspec containing a space reaches git as one element" "$d" ""
printf 'x <!-- claim count=1 pattern="hit" in="two words/a.md" -->\n' > "$d/n.md"; add "$d"
st_probe fail "C12 (b)  and it is the file that was measured, not another" "$d" "asserted=1 actual=2"
rm -rf "$d"

# No eval: a command substitution in the pattern is an ERE, not a command. The
# arm is worth more than its verdict -- it asserts the side effect did NOT
# happen, which no exit status can show.
d=$(mkfixture); printf 'harmless\n' > "$d/a.md"
printf 'x <!-- claim count=0 pattern="a`touch OWNED`b" in="a.md" -->\n' > "$d/n.md"; add "$d"
st_probe pass "C12 (c)  a backticked pattern body is matched, not executed" "$d" ""
if [ -e "$d/OWNED" ]; then
  fail_ "C12 (c)  and it left no side effect" "the annotation body was executed as a command"
else
  pass_ "C12 (c)  and it left no side effect"
fi
rm -rf "$d"

# --------------------------------------------------------------------------
# C27 -- I7's structural half: the work is one tree-wide command plus a
# CONSTANT number per annotation, never a term per tracked file. Counted, not
# timed: a `git` shim on PATH tallies invocations, and the same annotation set
# must cost the same in a 2-file tree and a 60-file one. Round 8 left I7 with
# no control, while §2 described this one as though it existed.
# --------------------------------------------------------------------------
count_git() { # $1 = dir -> echoes the number of `git` invocations the wire made
  local shim tally
  shim=$(mktemp -d); tally="$shim/tally"; : > "$tally"
  { printf '#!/usr/bin/env bash\n'
    printf 'printf x >> %s\n' "$tally"
    printf 'exec %s "$@"\n' "$(command -v git)"; } > "$shim/git"
  chmod +x "$shim/git"
  ( cd "$1" && PATH="$shim:$PATH" CLAIM_WIRE_SELFTEST=1 bash "$WIRE" ) >/dev/null 2>&1 || true
  awk 'END{print length($0)}' "$tally" 2>/dev/null || printf '0'
  rm -rf "$shim"
}

mk_i7() { # $1 = number of filler files -> a fixture with 2 count annotations
  local d i; d=$(mkfixture)
  printf 'hit\n' > "$d/a.md"
  i=0; while [ "$i" -lt "$1" ]; do printf 'filler\n' > "$d/f$i.md"; i=$((i+1)); done
  # `in="*.md"` deliberately: it resolves to 2 paths in the small tree and 61 in
  # the large one, so a term per RESOLVED path diverges here too, not only a
  # term per tracked file. The count is 3 either way -- a.md's line, plus the
  # two annotation lines, each of which contains `hit` inside its own pattern=.
  { printf 'x <!-- claim count=3 pattern="hit" in="*.md" -->\n'
    printf 'y <!-- claim count=3 pattern="hit" in="*.md" -->\n'; } > "$d/n.md"
  add "$d"; printf '%s' "$d"
}
small=$(mk_i7 1); large=$(mk_i7 60)
n_small=$(count_git "$small"); n_large=$(count_git "$large")
# rev-parse + scan, then ls-files + grep for each of the two annotations.
if [ "$n_small" = "$n_large" ] && [ "$n_small" = 6 ]; then
  pass_ "C27  git invocations are 2 + 2 per annotation, and do not move with the tree"
else
  fail_ "C27  git invocations are 2 + 2 per annotation, and do not move with the tree" \
    "2-file tree: $n_small, 61-file tree: $n_large, expected 6 in both"
fi
rm -rf "$small" "$large"

# --------------------------------------------------------------------------
# C30 -- §1's size bound on the two shipped files
# --------------------------------------------------------------------------
over=0
for f in "$WIRE" "$WIRE_DIR/claim-provenance-selftest.sh"; do
  n=$(awk 'END{print NR}' "$f")
  [ "$n" -le 800 ] || { over=1; fail_ "C30  $f exceeds 800 lines ($n)" "re-derive the decomposition"; }
done
[ "$over" -eq 0 ] && pass_ "C30  both shipped files are under the 800-line band"

printf 'claim-provenance self-test: %d/%d controls ok\n' "$PASS" "$((PASS+FAIL))"
[ "$FAIL" -eq 0 ]
