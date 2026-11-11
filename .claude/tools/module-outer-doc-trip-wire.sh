#!/usr/bin/env bash
# Trip-wire: a file-module declaration `mod X;` has no outer doc -- module docs live only
# inside the module file (`//!`), never on the declaration.
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
#   * A file module is `mod X;` (optional `pub` / `pub(...)`, raw ident `r#x` allowed)
#     on a line of its own. Inline `mod x { ... }` is not one: an inline module's outer
#     doc is its only home outside the block.
#   * An outer doc, attached as rustdoc attaches it: `///` (not `////`), `/** ... */`
#     (one or several lines), `#[doc = ...]`, `#[doc(...)]`, or a `#[cfg_attr(..., doc ...)]`.
#     Doc *text* is the concern, so `#[doc(hidden)]` and `#[doc(alias ...)]` do not count.
#   * Walking upward from `mod X;` the wire skips blank lines, plain `//` comments, plain
#     `/* */` blocks that start their line, and attribute lines (multi-line attributes
#     tracked by bracket depth, `"..."` strings with escapes blanked first, trailing
#     `// comment` dropped). Anything else -- an item, a code line ending in a block
#     comment, `//!` -- stops the walk.
#
# CONTROLS run on every invocation: ONE fixture repo, one directory per case, scanned
# once and checked per path -- red mutants must be reported, green negatives must not.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

# Hermetic git: a global core.excludesFile / quotepath must not change which files exist.
hgit() { GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 git -c core.quotepath=false "$@"; }

# One awk process for the whole tree. stdin = tracked `*.rs` paths; -v root = tree to read.
# Prints "<path>\t<line>\t<mod>" for every `mod X;` carrying an outer doc.
SCAN_AWK='
function trim(s) { gsub(/^[ \t\r]+|[ \t\r]+$/, "", s); return s }
# blank "..." string contents (escapes honoured) and drop a trailing // comment
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
  return out
}
function nopen(s,  t) { t = s; return gsub(/\[/, "", t) }
function nclose(s,  t) { t = s; return gsub(/\]/, "", t) }
# attribute text (whitespace removed) that is an outer doc
function attr_is_doc(t) {
  gsub(/[ \t\r]+/, "", t)
  if (t ~ /^#\[doc=/) return 1
  if (t ~ /^#\[doc\(/) return !(t ~ /^#\[doc\((hidden|alias)/)
  if (t ~ /^#\[cfg_attr\(/) {
    if (t ~ /,doc=/) return 1
    if (t ~ /,doc\(/ && !(t ~ /,doc\((hidden|alias)/)) return 1
  }
  return 0
}
# walk upward from line i (a `mod X;`); returns 1 if an outer doc is found
function has_outer(i,  j, k, s, t, d, txt, found) {
  found = 0; j = i - 1
  while (j >= 1) {
    s = trim(L[j])
    if (s == "") { j--; continue }
    if (substr(s, 1, 3) == "///" && substr(s, 1, 4) != "////") { found = 1; j--; continue }
    if (substr(s, 1, 3) == "//!") return found
    if (substr(s, 1, 2) == "//") { j--; continue }
    if (length(s) >= 2 && substr(s, length(s) - 1) == "*/") {
      k = j
      while (k > 1 && index(L[k], "/*") == 0) k--
      t = trim(L[k])
      if (substr(t, 1, 2) != "/*" || substr(t, 1, 3) == "/*!") return found
      if (substr(t, 1, 3) == "/**" && substr(t, 1, 4) != "/***" && t != "/**/") found = 1
      j = k - 1; continue
    }
    t = trim(strip(L[j]))
    if (substr(t, length(t)) == "]") {
      d = nclose(t) - nopen(t); k = j; txt = t
      while (d > 0 && k > 1) { k--; t = trim(strip(L[k])); d += nclose(t) - nopen(t); txt = t " " txt }
      if (substr(trim(strip(L[k])), 1, 2) != "#[") return found
      if (attr_is_doc(txt)) found = 1
      j = k - 1; continue
    }
    return found
  }
  return found
}
{
  f = $0; n = 0
  while ((getline line < (root "/" f)) > 0) L[++n] = line
  close(root "/" f)
  for (i = 1; i <= n; i++) {
    if (!match(L[i], /^[ \t]*(pub(\([^)]*\))?[ \t]+)?mod[ \t]+(r#)?[A-Za-z_][A-Za-z0-9_]*[ \t]*;/)) continue
    m = substr(L[i], RSTART, RLENGTH); sub(/^.*mod[ \t]+/, "", m); sub(/[ \t]*;$/, "", m)
    if (has_outer(i)) printf "%s\t%d\t%s\n", f, i, m
  }
}'

live_sites() { hgit -C "$1" ls-files '*.rs' | awk -v root="$1" "$SCAN_AWK"; }

# ---- controls ----------------------------------------------------------------------
if ! st="$(mktemp -d)" || [ -z "$st" ] || [ ! -d "$st" ]; then
  red "FAIL: module-outer-doc self-test could not create a scratch dir (TMPDIR/disk?)"
  exit 1
fi
trap 'rm -rf "$st"' EXIT
mkdir "$st/repo"; hgit -C "$st/repo" init -q
EXPECT="$st/expect"; : > "$EXPECT"
# ctl <red|green> <case> <content (printf %b)>
ctl() {
  mkdir -p "$st/repo/$2"; printf '%b' "$3" > "$st/repo/$2/a.rs"
  printf '%s %s\n' "$1" "$2" >> "$EXPECT"
}
D='/// outer doc\n'
ctl red  doc_line      "${D}mod foo;\n"
ctl red  block_single  '/** x */\nmod foo;\n'
ctl red  block_multi   '/**\n * x\n */\nmod foo;\n'
ctl red  attr_doc      '#[doc = "x"]\nmod foo;\n'
ctl red  cfg_attr_doc  '#[cfg_attr(doc, doc = "x")]\nmod foo;\n'
ctl red  sep_blank     "${D}\nmod foo;\n"
ctl red  sep_comment   "${D}// plain\nmod foo;\n"
ctl red  sep_multiattr "${D}#[cfg(any(\n    test,\n    debug_assertions,\n))]\nmod foo;\n"
ctl red  sep_attr_cmt  "${D}#[cfg(test)] // c\nmod foo;\n"
ctl red  sep_attr_str  "${D}#[cfg_attr(x, path = \"a]b\")]\nmod foo;\n"
ctl red  raw_ident     "${D}mod r#async;\n"
ctl red  pub_crate     "${D}pub(crate) mod x;\n"
ctl green none         'mod foo;\n'
ctl green four_slash   '//// not a doc\nmod foo;\n'
ctl green doc_hidden   '#[doc(hidden)]\nmod foo;\n'
ctl green item_between "${D}fn g() {}\nmod foo;\n"
ctl green item_blockcm "${D}fn g() {} /* c */\nmod foo;\n"
ctl green inline_mod   "${D}mod x { }\n"
hgit -C "$st/repo" add -A
SCAN="$(live_sites "$st/repo")"
ctl_rc=0
while read -r kind c; do
  hit=0; printf '%s\n' "$SCAN" | awk -F'\t' -v p="$c/a.rs" '$1 == p { f = 1 } END { exit !f }' && hit=1
  if [ "$kind" = red ] && [ "$hit" -eq 0 ]; then red "FAIL: control '$c' (red) was NOT caught -- this wire cannot fire."; ctl_rc=1; fi
  if [ "$kind" = green ] && [ "$hit" -eq 1 ]; then red "FAIL: control '$c' (green) was reported."; ctl_rc=1; fi
done < "$EXPECT"
[ "$ctl_rc" -eq 0 ] || exit 1

echo "module-outer-doc: a 'mod X;' declaration has no outer doc ($(wc -l < "$EXPECT" | tr -d ' ') controls ok)"
live="$(live_sites "$ROOT")"
if [ -n "$live" ]; then
  red "FAIL: outer doc on a file-module declaration (rustdoc joins it with the module file's inner doc; links there resolve in the OUTER scope). Move it into the module file's //!:"
  printf '%s\n' "$live" | awk -F'\t' '{ printf "  + %s:%s  mod %s;\n", $1, $2, $3 }'
  exit 1
fi
green "OK"
