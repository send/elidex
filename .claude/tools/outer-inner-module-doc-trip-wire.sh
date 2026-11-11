#!/usr/bin/env bash
# Trip-wire: a module's documentation must live in ONE place, not as an outer doc
# attached to the `mod x;` line PLUS an inner doc at the top of the module file.
#
# THE PROPERTY (defined by shape, not by any symptom vocabulary), with "attaches" meant
# as rustdoc means it -- a doc comment attaches to the next item no matter what lies
# between them:
#   * OUTER doc on `mod X;`: walking upward from the declaration, skip blank lines,
#     plain `//` comments (also `////`, plain `/* */`) and attribute lines (incl.
#     multi-line `#[cfg(any(\n...))]`, tracked by bracket depth); any of `///`,
#     `#[doc = ...]`, `/** ... */` found on the way is an outer doc. Any other line stops
#     the walk.
#   * INNER doc in the module file: after leading blank lines, `//` comments, plain
#     `/* */` blocks and `#![...]` inner attributes other than `#![doc = ...]`, the file
#     has `//!`, `/*! ... */` or `#![doc = ...]`.
# A hit needs both.
#
# WHY it is a defect: rustdoc joins the two doc blocks into one. An intra-doc link
# written in the inner block therefore resolves in the OUTER scope (the declaring
# module), not the module's own scope, so a link that reads correctly in the file
# silently breaks (PR #622 precedent; the branch that added this wire also repaired
# 2 latent ones with `super::`). The fix is to fold the outer doc into the inner one.
# An outer doc on a `mod X;` whose file has no inner doc is fine, and so is an inner
# doc with no outer doc.
#
# MODULE FILE RESOLUTION, as rustc does it:
#   * `#[path = "..."]`: relative to the declaring file's directory;
#   * else, for a declaring `mod.rs` / `lib.rs` / `main.rs`: `<dir>/X.rs` or
#     `<dir>/X/mod.rs`;
#   * else: `<dir>/<stem>/X.rs` or `<dir>/<stem>/X/mod.rs`.
# A module file that is not a tracked `*.rs` is not a hit (nothing to start with a doc).
#
# POPULATION: every tracked `*.rs` (`git ls-files`), scanned in ONE awk process.
#
# ALLOWLIST: `outer-inner-module-doc-allowlist.tsv` beside this script
# (`path<TAB>mod<TAB>owner<TAB>reason`). The wire fails on a violation NOT listed
# (added) and on a listed site that no longer violates (stale) -- the same
# live-vs-committed diff in both directions as layout-box-reader-trip-wire.sh.
#
# CONTROLS (run on every invocation, fixture repos in a temp dir): the shapes above
# each go red (plain `mod.rs`, attribute / multi-line attribute / `//` comment / blank
# line between doc and `mod`, `#[doc]` and `/** */` as the outer doc, `#[path]`,
# non-mod.rs declaring file, `X/mod.rs` form, module file led by `#![allow]` or an
# SPDX comment, `#![doc]` / `/*! */` as the inner doc); no inner doc, an inner doc
# that is not first, `////`, and no outer doc stay green; an allowlisted violation is
# green, a stale allowlist entry red.
#
# Surface: bash 3.2 + git + awk only (the `trip-wires` CI job declares no
# interpreter). Run from anywhere. Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ALLOWLIST="$ROOT/.claude/tools/outer-inner-module-doc-allowlist.tsv"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

# Hermetic git for fixtures and the scan: a global core.excludesFile / quotepath must not
# change which files exist (#501 R92 precedent).
hgit() { GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 git -c core.quotepath=false "$@"; }

# One awk process for the whole tree. stdin = tracked `*.rs` paths (repo-relative);
# -v root = tree to read. Prints "<path>\t<line>\t<mod>" for every violating site.
SCAN_AWK='
function trim(s) { gsub(/^[ \t\r]+|[ \t\r]+$/, "", s); return s }
function nopen(s,  t) { t = s; return gsub(/\[/, "", t) }
function nclose(s,  t) { t = s; return gsub(/\]/, "", t) }
# `/** x */`-style outer block doc (not `/***`, not `/**/`)
function is_block_doc(t) { return substr(t, 1, 3) == "/**" && substr(t, 1, 4) != "/***" && t != "/**/" }
function norm(p,  n, a, i, k, out) {
  n = split(p, a, "/"); k = 0
  for (i = 1; i <= n; i++) {
    if (a[i] == "" || a[i] == ".") continue
    if (a[i] == ".." && k > 0) { k--; continue }
    ST[++k] = a[i]
  }
  out = ST[1]
  for (i = 2; i <= k; i++) out = out "/" ST[i]
  return out
}
# does the file (L[1..n]) start with an inner doc?
function starts_inner(n,  i, s, d) {
  i = 1
  while (i <= n) {
    s = trim(L[i])
    if (s == "") { i++; continue }
    if (substr(s, 1, 3) == "///" && substr(s, 1, 4) != "////") return 0
    if (substr(s, 1, 3) == "//!") return 1
    if (substr(s, 1, 2) == "//") { i++; continue }
    if (substr(s, 1, 3) == "/*!") return 1
    if (is_block_doc(s)) return 0
    if (substr(s, 1, 2) == "/*") {
      while (i <= n && index(L[i], "*/") == 0) i++
      i++; continue
    }
    if (s ~ /^#!\[doc[ \t]*=/) return 1
    if (substr(s, 1, 3) == "#![") {
      d = nopen(s) - nclose(s); i++
      while (d > 0 && i <= n) { d += nopen(L[i]) - nclose(L[i]); i++ }
      continue
    }
    return 0
  }
  return 0
}
# walk upward from line i (a `mod X;`); sets OUTER and CUSTOM
function walk_up(i,  j, k, s, t, d, c) {
  OUTER = 0; CUSTOM = ""
  j = i - 1
  while (j >= 1) {
    s = trim(L[j])
    if (s == "") { j--; continue }
    if (substr(s, 1, 3) == "///" && substr(s, 1, 4) != "////") { OUTER = 1; j--; continue }
    if (substr(s, 1, 3) == "//!") return
    if (substr(s, 1, 2) == "//") { j--; continue }
    if (length(s) >= 2 && substr(s, length(s) - 1) == "*/") {
      k = j
      while (k > 1 && index(L[k], "/*") == 0) k--
      if (is_block_doc(trim(L[k]))) OUTER = 1
      j = k - 1; continue
    }
    if (substr(s, length(s)) == "]") {
      d = nclose(s) - nopen(s); k = j
      while (d > 0 && k > 1) { k--; d += nclose(L[k]) - nopen(L[k]) }
      t = trim(L[k])
      if (substr(t, 1, 2) != "#[") return
      if (t ~ /^#\[doc[ \t]*=/) OUTER = 1
      if (t ~ /^#\[path[ \t]*=[ \t]*"[^"]+"\]/) { c = t; sub(/^[^"]*"/, "", c); sub(/".*$/, "", c); CUSTOM = c }
      j = k - 1; continue
    }
    return
  }
}
{
  f = $0; n = 0
  while ((getline line < (root "/" f)) > 0) L[++n] = line
  close(root "/" f)
  KNOWN[f] = 1
  if (starts_inner(n)) INNER[f] = 1
  d = ""; if (match(f, /\/[^\/]*$/)) d = substr(f, 1, RSTART - 1)
  base = substr(f, length(d) + (d == "" ? 1 : 2))
  for (i = 1; i <= n; i++) {
    if (!match(L[i], /^[ \t]*(pub(\([^)]*\))?[ \t]+)?mod[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*;/)) continue
    m = substr(L[i], RSTART, RLENGTH); sub(/^.*mod[ \t]+/, "", m); sub(/[ \t]*;$/, "", m)
    walk_up(i)
    if (!OUTER) continue
    if (CUSTOM != "") { tgt1 = norm((d == "" ? "" : d "/") CUSTOM); tgt2 = "" }
    else {
      if (base == "mod.rs" || base == "lib.rs" || base == "main.rs") sub_ = d
      else { sub_ = base; sub(/\.rs$/, "", sub_); sub_ = (d == "" ? "" : d "/") sub_ }
      pre = (sub_ == "" ? "" : sub_ "/")
      tgt1 = pre m ".rs"; tgt2 = pre m "/mod.rs"
    }
    NS++; SF[NS] = f; SL[NS] = i; SN[NS] = m; ST1[NS] = tgt1; ST2[NS] = tgt2
  }
}
END {
  for (s = 1; s <= NS; s++) {
    tgt = ""
    if (ST1[s] in KNOWN) tgt = ST1[s]; else if (ST2[s] != "" && (ST2[s] in KNOWN)) tgt = ST2[s]
    if (tgt != "" && (tgt in INNER)) printf "%s\t%d\t%s\n", SF[s], SL[s], SN[s]
  }
}'

# live_sites <root>: "<path>\t<line>\t<mod>" for every violating site.
live_sites() {
  hgit -C "$1" ls-files '*.rs' | awk -v root="$1" "$SCAN_AWK"
}

# diff_live <live> <allowlist>: compare live sites with the allowlist; diagnostics on
# stdout, returns 1 on any violation. (The live scan is done by the caller, once.)
diff_live() {
  local live="$1" allow="$2" live_keys allow_rows allow_keys dups added stale bad rc=0
  live_keys="$(printf '%s\n' "$live" | awk -F'\t' 'NF { print $1 "\t" $3 }' | sort -u)"
  allow_rows="$( { grep -vE '^[[:space:]]*(#|$)' "$allow" 2>/dev/null || true; } )"
  bad="$(printf '%s\n' "$allow_rows" | awk -F'\t' 'NF && (NF != 4 || $1 == "" || $2 == "" || $3 == "" || $4 == "") { print }')"
  if [ -n "$bad" ]; then
    red "FAIL: malformed allowlist row(s) (need path<TAB>mod<TAB>owner<TAB>reason, all non-empty):"
    printf '%s\n' "$bad" | sed 's/^/  ? /'; rc=1
  fi
  allow_keys="$(printf '%s\n' "$allow_rows" | awk -F'\t' 'NF { print $1 "\t" $2 }' | sort)"
  dups="$(printf '%s\n' "$allow_keys" | uniq -d)"
  if [ -n "$dups" ]; then
    red "FAIL: duplicate allowlist key(s):"
    printf '%s\n' "$dups" | sed 's/^/  = /'; rc=1
  fi
  added="$(comm -23 <(printf '%s\n' "$live_keys") <(printf '%s\n' "$allow_keys") | grep -v '^$' || true)"
  stale="$(comm -13 <(printf '%s\n' "$live_keys") <(printf '%s\n' "$allow_keys") | grep -v '^$' || true)"
  if [ -n "$added" ]; then
    red "FAIL: outer doc on a 'mod X;' whose module file starts with an inner doc (rustdoc joins them; an intra-doc link in the inner doc resolves in the OUTER scope). Fold the outer doc into the inner one:"
    printf '%s\n' "$added" | while IFS=$'\t' read -r p n; do
      printf '  + %s  mod %s;  (line(s): %s)\n' "$p" "$n" \
        "$(printf '%s\n' "$live" | awk -F'\t' -v p="$p" -v n="$n" '$1 == p && $3 == n { printf "%s ", $2 }')"
    done
    rc=1
  fi
  if [ -n "$stale" ]; then
    red "FAIL: STALE allowlist entr(y/ies) -- the site no longer violates; remove the row from $(basename "$allow") in the same change:"
    printf '%s\n' "$stale" | sed 's/^/  - /'
    rc=1
  fi
  return "$rc"
}

# ---- controls: prove the check fires before trusting its verdict -----------------
if ! st="$(mktemp -d)" || [ -z "$st" ] || [ ! -d "$st" ]; then
  red "FAIL: outer-inner-module-doc self-test could not create a scratch dir (TMPDIR/disk?)"
  exit 1
fi
trap 'rm -rf "$st"' EXIT
ctl_ok=0
# fx <case> <declaring file> <its content> <module file> <its content>
# Builds the fixture repo $st/<case> (contents are printf %b strings) and scans it ONCE
# into $LIVE_<case>; probes then vary only the allowlist.
fx() {
  local c="$1" f
  mkdir -p "$st/$c"; hgit -C "$st/$c" init -q
  for f in "$2" "$4"; do mkdir -p "$(dirname "$st/$c/$f")"; done
  printf '%b' "$3" > "$st/$c/$2"; printf '%b' "$5" > "$st/$c/$4"
  hgit -C "$st/$c" add -A
  printf -v "LIVE_$c" '%s' "$(live_sites "$st/$c")"
}
probe() { # $1 = red|green, $2 = case, $3 = allowlist file, $4 = required diagnostic substring (red)
  local out rc=0 var="LIVE_$2"
  out="$(diff_live "${!var}" "$3" 2>&1)" || rc=$?
  if [ "$1" = green ] && [ "$rc" -ne 0 ]; then
    red "FAIL: self-test '$2' should be green but was red:"; printf '%s\n' "$out" >&2; ctl_ok=1
  fi
  if [ "$1" = red ]; then
    if [ "$rc" -eq 0 ]; then
      red "FAIL: self-test '$2' was NOT caught -- this wire cannot fire."; ctl_ok=1
    else
      case "$out" in *"$4"*) ;; *) red "FAIL: self-test '$2' went red for the WRONG reason (no '$4')."; ctl_ok=1 ;; esac
    fi
  fi
}
EMPTY="$st/empty.tsv"; : > "$EMPTY"
OUT='/// outer doc\n'

# red: each shape that attaches an outer doc to `mod foo;` over an inner-doc file
fx m1 src/mod.rs "${OUT}mod foo;\n" src/foo.rs '//! inner\n'
probe red m1 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m2 src/mod.rs "${OUT}#[cfg(test)]\nmod foo;\n" src/foo.rs '\n//! inner\n'
probe red m2 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m3 src/lib.rs "${OUT}#[path = \"other/bar.rs\"]\nmod foo;\n" src/other/bar.rs '//! inner\n'
probe red m3 "$EMPTY" "+ src/lib.rs  mod foo;"
fx m4 src/a.rs "${OUT}pub(crate) mod b;\n" src/a/b.rs '//! inner\n'
probe red m4 "$EMPTY" "+ src/a.rs  mod b;"
fx m5 src/lib.rs "${OUT}mod b;\n" src/b/mod.rs '//! inner\n'
probe red m5 "$EMPTY" "+ src/lib.rs  mod b;"
fx m6 src/mod.rs "${OUT}\nmod foo;\n" src/foo.rs '//! inner\n'                       # blank line between
probe red m6 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m7 src/mod.rs "${OUT}// plain\nmod foo;\n" src/foo.rs '//! inner\n'              # `//` comment between
probe red m7 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m8 src/mod.rs "${OUT}#[cfg(any(\n    test,\n    debug_assertions,\n))]\nmod foo;\n" src/foo.rs '//! inner\n'  # multi-line attribute
probe red m8 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m9 src/mod.rs '#[doc = "x"]\nmod foo;\n' src/foo.rs '//! inner\n'                 # #[doc = ..] outer
probe red m9 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m10 src/mod.rs '/** x */\nmod foo;\n' src/foo.rs '//! inner\n'                    # block outer
probe red m10 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m11 src/mod.rs '/**\n * x\n */\nmod foo;\n' src/foo.rs '//! inner\n'              # multi-line block outer
probe red m11 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m12 src/mod.rs "${OUT}mod foo;\n" src/foo.rs '#![allow(x)]\n//! inner\n'          # inner attr first
probe red m12 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m13 src/mod.rs "${OUT}mod foo;\n" src/foo.rs '// SPDX-License-Identifier: MIT\n//! inner\n'  # comment first
probe red m13 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m14 src/mod.rs "${OUT}mod foo;\n" src/foo.rs '#![doc = "x"]\n'                    # #![doc = ..] inner
probe red m14 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m15 src/mod.rs "${OUT}mod foo;\n" src/foo.rs '/*! inner */\n'                     # block inner
probe red m15 "$EMPTY" "+ src/mod.rs  mod foo;"
fx m16 src/mod.rs "${OUT}mod foo;\n" src/foo.rs '#![allow(\n    x,\n)]\n//! inner\n' # multi-line inner attr first
probe red m16 "$EMPTY" "+ src/mod.rs  mod foo;"
# green: not a violation
fx n1 src/mod.rs "${OUT}mod foo;\n" src/foo.rs 'use x;\n//! not first\n'              # inner doc not first
probe green n1 "$EMPTY"
fx n2 src/mod.rs 'mod foo;\n' src/foo.rs '//! inner\n'                               # no outer doc
probe green n2 "$EMPTY"
fx n3 src/mod.rs '//// not a doc\nmod foo;\n' src/foo.rs '//! inner\n'               # `////` is not a doc
probe green n3 "$EMPTY"
fx n4 src/mod.rs "${OUT}mod foo;\n" src/foo.rs 'fn f() {}\n'                         # no inner doc
probe green n4 "$EMPTY"
fx n5 src/mod.rs "${OUT}fn g() {}\nmod foo;\n" src/foo.rs '//! inner\n'              # an item between: doc is not on `mod`
probe green n5 "$EMPTY"
fx n6 src/mod.rs '#[doc(hidden)]\nmod foo;\n' src/foo.rs '//! inner\n'               # #[doc(hidden)] is not doc text
probe green n6 "$EMPTY"
# allowlist: a listed violation is green; a listed non-violation is stale (red)
AL="$st/allow.tsv"; printf 'src/mod.rs\tfoo\tVM P4\tpending\n' > "$AL"
probe green m1 "$AL"
probe red n1 "$AL" "STALE allowlist"
printf 'src/mod.rs\tfoo\n' > "$st/badrow.tsv"
probe red m1 "$st/badrow.tsv" "malformed allowlist"
printf 'src/mod.rs\tfoo\tVM P4\tpending\nsrc/mod.rs\tfoo\tVM P4\tagain\n' > "$st/dup.tsv"
probe red m1 "$st/dup.tsv" "duplicate allowlist"

[ "$ctl_ok" -eq 0 ] || exit 1

echo "outer-inner-module-doc: no outer doc + inner doc pair on a module declaration"
if diff_live "$(live_sites "$ROOT")" "$ALLOWLIST"; then
  green "OK"
else
  exit 1
fi
