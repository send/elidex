#!/usr/bin/env bash
# Trip-wire: a file-module declaration `mod X;` has no outer doc -- module docs live only
# inside the module file (`//!`), never on the declaration.
#
# POLICY (fail-closed): the wire walks upward from each `mod X;` and must classify EVERY
# line it crosses with certainty as exactly one of: blank, plain `//` comment, a complete
# single-line block comment (`/* .. */` opening and closing on that line, nothing else on
# it), a complete attribute (brackets balanced, no raw string, no string literal spanning
# lines, nothing after the closing `]` but an optional `//` comment), an outer-doc form,
# or a terminator (any other code line: the walk stops, "no doc"). A line that is none of
# these -- inside a multi-line `/* */`, a raw-string or multi-line-string attribute, several
# syntax elements on one line (`#[a] /* c */`, `/** d */ #[a]`, `#[a] #[b]`), or
# `#[a] mod x;` with the attribute on the declaration line -- is UNDECIDABLE and the wire
# goes red at `path:line`. The wire does not grow a lexer to decide such lines.
#
# Surface: runs in the `trip-wires` CI job, which has no setup step; it needs only what
# that job already has. Run from anywhere. Exits non-zero on any violation.
#
# WHY: rustdoc joins an outer doc on `mod X;` with the `//!` doc in X's file into one
# block, so an intra-doc link written in the file resolves in the DECLARING module's
# scope and silently breaks (PR #622 precedent). The rule needs no module-file
# resolution: with no outer doc on the declaration there is nothing to join.
#
# SCOPE OF THE SHAPE:
#   * A file module is `mod X;` (optional `pub` / `pub(...)`, raw ident `r#x` allowed,
#     `pub(crate)mod` too) on a line of its own (an optional trailing `//` comment is ok).
#     Inline `mod x { ... }` is not one: an inline module's outer doc is its only home
#     outside the block. Anything else on the declaration line before `mod` (attribute,
#     block comment) is UNDECIDABLE.
#   * An outer doc is a form that carries doc TEXT: `///` (not `////`), a single-line
#     `/** ... */`, `#[doc = ...]`, `#[cfg_attr(..., doc = ...)]`. The list forms
#     `#[doc(hidden|alias|cfg|inline|no_inline ...)]` / `cfg_attr(..., doc(...))` are
#     attributes, not doc text. A multi-line `/** */` is UNDECIDABLE (see POLICY).
#   * `//!` above the declaration is the parent file's inner doc: it stops the walk.
#   * Unreadable tracked file: red `unreadable: <path>`; a file is never skipped silently.
#     The file list is `git ls-files -z`; NULs become newlines for awk (a path containing
#     a newline splits into unreadable names, i.e. fails closed).
#
# CONTROLS run on every invocation: ONE fixture repo, one directory per case, scanned
# once and checked per path -- red mutants must be reported, green negatives must not.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

# Hermetic git: a global core.excludesFile / quotepath must not change which files exist.
hgit() { GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 git -c core.quotepath=false "$@"; }

# The one place the UNDECIDABLE guidance sentence lives.
GUIDE="undecidable shape: move the doc above \`mod X;\` into the module file's \`//!\`, or put each syntax element on its own line"

# One awk process for the whole tree. stdin = tracked `*.rs` paths (one per line); -v root =
# tree to read. Prints "<path>\t<message>" where message is one of
#   "<path>:<line>  mod <name>"                    outer doc on a file-module declaration
#   "<path>:<line>  UNDECIDABLE: <guide>"          a line the walk cannot classify
#   "unreadable: <path>"                           tracked file that could not be read
SCAN_AWK='
function trim(s) { gsub(/^[ \t\r]+|[ \t\r]+$/, "", s); return s }
# blank "..." string contents (escapes honoured) and drop a trailing // comment;
# STROPEN = 1 when a string is still open at end of line (multi-line string)
function strip(s,  i, c, out, q) {
  out = ""; q = 0
  for (i = 1; i <= length(s); i++) {
    c = substr(s, i, 1)
    if (q) {
      if (c == "\\") { i++; continue }
      if (c == "\"") { q = 0; out = out c }
      continue
    }
    if (c == "\"") { q = 1; out = out c; continue }
    if (c == "/" && substr(s, i + 1, 1) == "/") break
    out = out c
  }
  STROPEN = q
  return out
}
function rawstr(s) { return match(s, /(^|[^A-Za-z0-9_])b?r#*"/) > 0 }
function nopen(s,  t) { t = s; return gsub(/\[/, "", t) }
function nclose(s,  t) { t = s; return gsub(/\]/, "", t) }
# position of the "]" matching the leading "#[" of txt, or 0
function attr_end(txt,  i, c, dep) {
  dep = 0
  for (i = 2; i <= length(txt); i++) {
    c = substr(txt, i, 1)
    if (c == "[") dep++
    else if (c == "]") { dep--; if (dep == 0) return i }
  }
  return 0
}
# attribute text that carries doc text: #[doc = ..] or #[cfg_attr(.., doc = ..)]
function attr_is_doc(t) {
  gsub(/[ \t\r]+/, "", t)
  if (t ~ /^#\[doc=/) return 1
  if (t ~ /^#\[cfg_attr\(/ && t ~ /,doc=/) return 1
  return 0
}
# walk upward from line i (a `mod X;`); returns 1 if an outer doc is found; sets UND to the
# line number of the first unclassifiable line (0 = none)
function walk(i,  j, k, s, t, d, u, txt, found, sub1, p, q, pre, bad0, bad, isattr, e, rest) {
  found = 0; UND = 0; j = i - 1
  while (j >= 1) {
    s = trim(L[j])
    if (s == "") { j--; continue }
    if (substr(s, 1, 3) == "///" && substr(s, 1, 4) != "////") { found = 1; j--; continue }
    if (substr(s, 1, 3) == "//!") return found
    if (substr(s, 1, 2) == "//") { j--; continue }
    if (substr(s, 1, 2) == "/*" || index(s, "*/") > 0) {
      if (substr(s, 1, 2) == "/*") {
        sub1 = substr(s, 3); p = index(sub1, "*/")
        if (p > 0 && p == length(sub1) - 1 && index(sub1, "/*") == 0) {
          if (substr(s, 1, 3) == "/*!") return found
          if (substr(s, 1, 3) == "/**" && substr(s, 1, 4) != "/***" && s != "/**/") found = 1
          j--; continue
        }
        UND = j; return found
      }
      # code, then one self-contained block comment closing the line = certain terminator
      p = index(s, "/*")
      if (p > 1) {
        sub1 = substr(s, p + 2); q = index(sub1, "*/")
        pre = trim(substr(s, 1, p - 1))
        if (q > 0 && q == length(sub1) - 1 && index(sub1, "/*") == 0 && pre !~ /^[#*\/]/) return found
      }
      UND = j; return found
    }
    t = trim(strip(s)); bad0 = (STROPEN || rawstr(s))
    isattr = (substr(s, 1, 2) == "#[") || (substr(t, length(t)) == "]") || (substr(s, length(s)) == "]")
    if (!isattr) return found
    k = j; d = 0; txt = ""; bad = bad0
    while (1) {
      u = trim(strip(L[k]))
      if (STROPEN || rawstr(L[k]) || index(L[k], "/*") > 0) bad = 1
      d += nclose(u) - nopen(u); txt = (txt == "" ? u : u " " txt)
      if (d <= 0 || k <= 1) break
      k--
    }
    txt = trim(txt)
    if (substr(txt, 1, 2) != "#[") { if (bad0) { UND = j; return found } return found }
    e = attr_end(txt)
    if (bad || e == 0) { UND = j; return found }
    rest = trim(substr(txt, e + 1))
    if (rest != "") { if (rest ~ /^[#\/]/) { UND = j; return found } return found }
    if (attr_is_doc(txt)) found = 1
    j = k - 1
  }
  return found
}
{
  f = $0; n = 0
  while ((r = (getline line < (root "/" f))) > 0) L[++n] = line
  close(root "/" f)
  if (r < 0) { printf "%s\tunreadable: %s\n", f, f; next }
  delete seen
  for (i = 1; i <= n; i++) {
    if (match(L[i], /^[ \t]*(pub[ \t]+|pub\([^)]*\)[ \t]*)?mod[ \t]+(r#)?[A-Za-z_][A-Za-z0-9_]*[ \t]*;/)) {
      m = substr(L[i], RSTART, RLENGTH); rest = trim(substr(L[i], RSTART + RLENGTH))
      sub(/^.*mod[ \t]+/, "", m); sub(/[ \t]*;$/, "", m)
      if (rest != "" && substr(rest, 1, 2) != "//") { if (!(i in seen)) printf "%s\t%s:%d  UNDECIDABLE: %s\n", f, f, i, guide; seen[i] = 1; continue }
    } else if (match(L[i], /(^|[^A-Za-z0-9_#])mod[ \t]+(r#)?[A-Za-z_][A-Za-z0-9_]*[ \t]*;/)) {
      pre = substr(L[i], 1, RSTART - 1)
      if (pre !~ /\/\// && pre ~ /(#\[|\*\/)/) { if (!(i in seen)) printf "%s\t%s:%d  UNDECIDABLE: %s\n", f, f, i, guide; seen[i] = 1 }
      continue
    } else continue
    w = walk(i)
    if (UND) { if (!(UND in seen)) printf "%s\t%s:%d  UNDECIDABLE: %s\n", f, f, UND, guide; seen[UND] = 1; continue }
    if (w) printf "%s\t%s:%d  mod %s\n", f, f, i, m
  }
}'

live_scan() { hgit -C "$1" ls-files -z '*.rs' | tr '\0' '\n' | awk -v root="$1" -v guide="$GUIDE" "$SCAN_AWK"; }

# ---- controls ----------------------------------------------------------------------
# CONTROLS run on every invocation: ONE fixture repo, one directory per case, scanned
# once; each control asserts the EXACT reported text for its path (empty = green).
# MODULE_OUTER_DOC_VERBOSE=1 lists every control and its result.
if ! st="$(mktemp -d)" || [ -z "$st" ] || [ ! -d "$st" ]; then
  red "FAIL: module-outer-doc self-test could not create a scratch dir (TMPDIR/disk?)"
  exit 1
fi
trap 'rm -rf "$st"' EXIT
mkdir "$st/repo"; hgit -C "$st/repo" init -q
CASES="$st/cases"; : > "$CASES"
# ctl <case> <content (printf %b)> [<suffix after "<case>/a.rs:" | @unreadable>]
ctl() {
  mkdir -p "$st/repo/$1"; printf '%b' "$2" > "$st/repo/$1/a.rs"
  local exp=""
  case "${3-}" in "") ;; @unreadable) exp="unreadable: $1/a.rs" ;; *) exp="$1/a.rs:$3" ;; esac
  printf '%s\t%s\n' "$1" "$exp" >> "$CASES"
}
D='/// outer doc\n'
# red: outer doc on the declaration
ctl doc_line      "${D}mod foo;\n"                                   '2  mod foo'
ctl block_single  '/** x */\nmod foo;\n'                              '2  mod foo'
ctl attr_doc      '#[doc = "x"]\nmod foo;\n'                          '2  mod foo'
ctl cfg_attr_doc  '#[cfg_attr(doc, doc = "x")]\nmod foo;\n'           '2  mod foo'
ctl cfg_attr_docsrs '#[cfg_attr(docsrs, doc = "x")]\nmod foo;\n'      '2  mod foo'
ctl sep_blank     "${D}\nmod foo;\n"                                  '3  mod foo'
ctl sep_comment   "${D}// plain\nmod foo;\n"                         '3  mod foo'
ctl sep_multiattr "${D}#[cfg(any(\n    test,\n    debug_assertions,\n))]\nmod foo;\n" '6  mod foo'
ctl sep_attr_cmt  "${D}#[cfg(test)] // c\nmod foo;\n"                '3  mod foo'
ctl sep_attr_str  "${D}#[cfg_attr(x, path = \"a]b\")]\nmod foo;\n"    '3  mod foo'
ctl sep_block1    "${D}/* c */\nmod foo;\n"                           '3  mod foo'
ctl raw_ident     "${D}mod r#async;\n"                                '2  mod r#async'
ctl pub_crate     "${D}pub(crate) mod x;\n"                           '2  mod x'
ctl pub_crate_nsp "${D}pub(crate)mod x;\n"                            '2  mod x'
ctl trailing_cmt  "${D}mod foo; // c\n"                               '2  mod foo'
# green: no outer doc on the declaration
ctl none          'mod foo;\n'
ctl four_slash    '//// not a doc\nmod foo;\n'
ctl doc_hidden    '#[doc(hidden)]\nmod foo;\n'
ctl doc_cfg       '#[doc(cfg(x))]\nmod foo;\n'
ctl doc_inline    '#[doc(inline)]\nmod foo;\n'
ctl doc_noinline  '#[doc(no_inline)]\nmod foo;\n'
ctl doc_alias     '#[doc(alias = "a")]\nmod foo;\n'
ctl cfg_attr_cfg  '#[cfg_attr(docsrs, doc(cfg(feature = "x")))]\nmod foo;\n'
ctl inner_stops   '/// a\n//! inner\nmod foo;\n'
ctl item_between  "${D}fn g() {}\nmod foo;\n"
ctl item_blockcm  "${D}fn g() {} /* c */\nmod foo;\n"
ctl attr_item     "${D}#[test] fn g() {}\nmod foo;\n"
ctl inline_mod    "${D}mod x { }\n"
# red: UNDECIDABLE (fail-closed), reported at the unclassifiable line
U="UNDECIDABLE: $GUIDE"
ctl und_block_multi '/**\n * x\n */\nmod foo;\n'                       "3  $U"
ctl und_block_glob  '/**\n * src/*.rs\n */\nmod foo;\n'               "3  $U"
ctl und_raw_attr    '#[doc = r"x"]\nmod foo;\n'                        "1  $U"
ctl und_rawhash     '#[doc = r#"x"#]\nmod foo;\n'                      "1  $U"
ctl und_multistr    '#[doc = "a\nb"]\nmod foo;\n'                      "2  $U"
ctl und_attr_blockc '#[cfg(test)] /* c */\nmod foo;\n'                 "1  $U"
ctl und_blockc_attr '/** d */ #[cfg(test)]\nmod foo;\n'                "1  $U"
ctl und_two_attrs   '#[a] #[b]\nmod foo;\n'                            "1  $U"
ctl und_sameline    '#[cfg(test)] mod foo;\n'                          "1  $U"
ctl und_sameline_cm '/* c */ mod foo;\n'                               "1  $U"
# red: tracked but unreadable (deleted from the working tree after staging)
ctl gone            'mod foo;\n'                                       @unreadable
hgit -C "$st/repo" add -A
rm "$st/repo/gone/a.rs"
SCANF="$st/scan"; live_scan "$st/repo" > "$SCANF"
ctl_rc=0
while IFS=$'\t' read -r c exp; do
  got="$(awk -F'\t' -v p="$c/a.rs" '$1 == p { print $2 }' "$SCANF")"
  if [ "$got" = "$exp" ]; then
    [ -z "${MODULE_OUTER_DOC_VERBOSE-}" ] || printf '  ok  %-5s %s\n' "$([ -n "$exp" ] && echo red || echo green)" "$c"
  else
    red "FAIL: control '$c': expected [$exp] got [$got]"; ctl_rc=1
  fi
done < "$CASES"
# no report may fall outside a known control
stray="$(awk -F'\t' 'NR == FNR { k[$1 "/a.rs"] = 1; next } !($1 in k)' "$CASES" "$SCANF")"
if [ -n "$stray" ]; then red "FAIL: scan reported lines outside any control: $stray"; ctl_rc=1; fi
[ "$ctl_rc" -eq 0 ] || exit 1

echo "module-outer-doc: a 'mod X;' declaration has no outer doc ($(wc -l < "$CASES" | tr -d ' ') controls ok)"
live="$(live_scan "$ROOT")"
if [ -n "$live" ]; then
  red "FAIL: outer doc on a file-module declaration (rustdoc joins it with the module file's inner doc; links there resolve in the OUTER scope), or an unclassifiable line / unreadable file. Move the doc into the module file's //!:"
  printf '%s\n' "$live" | awk -F'\t' '{ printf "  + %s\n", $2 }'
  exit 1
fi
green "OK"
