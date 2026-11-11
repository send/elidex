#!/usr/bin/env bash
# Trip-wire: a module's documentation must live in ONE place, not as an outer `///`
# on the `mod x;` line PLUS an inner `//!` at the top of the module file.
#
# THE PROPERTY (defined by shape, not by any symptom vocabulary): a `mod X;`
# declaration is preceded — stepping upward over attribute lines (`#[...]`) — by an
# outer `///` doc comment, AND the module's file starts (first non-blank line) with
# `//!`.
#
# WHY it is a defect: rustdoc joins the two doc blocks into one. An intra-doc link
# written in the `//!` block therefore resolves in the OUTER scope (the declaring
# module), not the module's own scope, so a link that reads correctly in the file
# silently breaks (PR #622 precedent; the branch that added this wire also repaired
# 2 latent ones with `super::`). The fix is to fold the outer doc into the `//!`.
# `///` on a `mod X;` whose file has no `//!` is fine, and so is `//!` with no outer doc.
#
# MODULE FILE RESOLUTION, as rustc does it:
#   * `#[path = "..."]`: relative to the declaring file's directory;
#   * else, for a declaring `mod.rs` / `lib.rs` / `main.rs`: `<dir>/X.rs` or
#     `<dir>/X/mod.rs`;
#   * else: `<dir>/<stem>/X.rs` or `<dir>/<stem>/X/mod.rs`.
# A module file that cannot be found is not a hit (nothing to start with `//!`).
#
# POPULATION: every tracked `*.rs` (`git ls-files`).
#
# ALLOWLIST: `outer-inner-module-doc-allowlist.tsv` beside this script
# (`path<TAB>mod<TAB>owner<TAB>reason`). The wire fails on a violation NOT listed
# (added) and on a listed site that no longer violates (stale) — the same
# live-vs-committed diff in both directions as layout-box-reader-trip-wire.sh.
#
# CONTROLS (run on every invocation, fixture repos in a temp dir): the
# `vm/host/mod.rs` shape, with an attribute between `///` and `mod`, with `#[path]`,
# in a non-mod.rs declaring file and in `X/mod.rs` form must each go red; `///` on a
# module with no `//!`, `//!` with no outer doc, and a blank line between `///` and
# `mod` must stay green; an allowlisted violation is green, a stale allowlist entry red.
#
# Surface: bash 3.2 + git + grep + awk only (the `trip-wires` CI job declares no
# interpreter). Run from anywhere. Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ALLOWLIST="$ROOT/.claude/tools/outer-inner-module-doc-allowlist.tsv"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }

# awk: for one file, print "<line>\t<name>\t<path attr or empty>" for every `mod X;`
# carrying an outer `///` (walking up over `#[...]` lines; `////` is not a doc).
SCAN_AWK='
{ L[NR] = $0 }
END {
  for (i = 1; i <= NR; i++) {
    if (!match(L[i], /^[ \t]*(pub(\([^)]*\))?[ \t]+)?mod[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*;/)) continue
    name = L[i]
    sub(/^[ \t]*(pub(\([^)]*\))?[ \t]+)?mod[ \t]+/, "", name)
    sub(/[^A-Za-z0-9_].*$/, "", name)
    outer = 0; custom = ""
    for (j = i - 1; j >= 1; j--) {
      s = L[j]; gsub(/^[ \t\r]+|[ \t\r]+$/, "", s)
      if (substr(s, 1, 3) == "///" && substr(s, 1, 4) != "////") { outer = 1; continue }
      if (substr(s, 1, 2) == "#[") {
        if (match(L[j], /^[ \t]*#\[path[ \t]*=[ \t]*"[^"]+"\]/)) {
          c = L[j]; sub(/^[^"]*"/, "", c); sub(/".*$/, "", c); custom = c
        }
        continue
      }
      break
    }
    if (outer) printf "%d\t%s\t%s\n", i, name, custom
  }
}'

# first non-blank line of a file starts with `//!`?
starts_inner() {
  # `exit` inside a rule still runs END, and a bare `exit` there would keep the rule's
  # status only if END does not set one — so carry the verdict in a variable.
  awk 'v == "" { s = $0; gsub(/^[ \t\r]+|[ \t\r]+$/, "", s); if (s == "") next;
         v = (substr(s, 1, 3) == "//!") ? 1 : 0 }
       END { exit (v == 1 ? 0 : 1) }' "$1"
}

# live_sites <root>: "<path>\t<line>\t<mod>" for every violating site.
live_sites() {
  local root="$1" f cand d base sub line name custom tgt
  git -C "$root" ls-files '*.rs' | while IFS= read -r f; do
    [ -f "$root/$f" ] || continue
    # `|| true`: a file with no `mod x;` line is the normal case, not an error.
    grep -qE '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?mod[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*;' "$root/$f" || continue
    awk "$SCAN_AWK" "$root/$f" | while IFS=$'\t' read -r line name custom; do
      d="$(dirname "$f")"; base="$(basename "$f")"
      if [ -n "$custom" ]; then
        case "$custom" in /*) cand="$custom" ;; *) cand="$d/$custom" ;; esac
        tgt=""; [ -f "$root/$cand" ] && tgt="$root/$cand"
      else
        case "$base" in
          mod.rs|lib.rs|main.rs) sub="$d" ;;
          *) sub="$d/${base%.rs}" ;;
        esac
        tgt=""
        if [ -f "$root/$sub/$name.rs" ]; then tgt="$root/$sub/$name.rs"
        elif [ -f "$root/$sub/$name/mod.rs" ]; then tgt="$root/$sub/$name/mod.rs"; fi
      fi
      [ -n "$tgt" ] || continue
      if starts_inner "$tgt"; then printf '%s\t%s\t%s\n' "$f" "$line" "$name"; fi
    done
  done
}

# check <root> <allowlist>: prints diagnostics, returns 1 on any violation.
check() {
  local root="$1" allow="$2" live live_keys allow_rows allow_keys added stale bad rc=0
  live="$(live_sites "$root")"
  live_keys="$(printf '%s\n' "$live" | awk -F'\t' 'NF { print $1 "\t" $3 }' | sort -u)"
  allow_rows="$( { grep -vE '^[[:space:]]*(#|$)' "$allow" 2>/dev/null || true; } )"
  bad="$(printf '%s\n' "$allow_rows" | awk -F'\t' 'NF && (NF != 4 || $1 == "" || $2 == "" || $3 == "" || $4 == "") { print }')"
  if [ -n "$bad" ]; then
    red "FAIL: malformed allowlist row(s) (need path<TAB>mod<TAB>owner<TAB>reason, all non-empty):"
    printf '%s\n' "$bad" | sed 's/^/  ? /'; rc=1
  fi
  allow_keys="$(printf '%s\n' "$allow_rows" | awk -F'\t' 'NF { print $1 "\t" $2 }' | sort)"
  if [ -n "$allow_keys" ] && [ "$(printf '%s\n' "$allow_keys" | uniq -d)" != "" ]; then
    red "FAIL: duplicate allowlist key(s):"
    printf '%s\n' "$allow_keys" | uniq -d | sed 's/^/  = /'; rc=1
  fi
  allow_keys="$(printf '%s\n' "$allow_keys" | sort -u)"
  added="$(comm -23 <(printf '%s\n' "$live_keys") <(printf '%s\n' "$allow_keys") | grep -v '^$' || true)"
  stale="$(comm -13 <(printf '%s\n' "$live_keys") <(printf '%s\n' "$allow_keys") | grep -v '^$' || true)"
  if [ -n "$added" ]; then
    red "FAIL: outer /// doc on a 'mod X;' whose module file starts with //! (rustdoc joins them; an intra-doc link in the //! resolves in the OUTER scope). Fold the outer doc into the //!:"
    printf '%s\n' "$added" | while IFS=$'\t' read -r p n; do
      printf '  + %s  mod %s;  (line(s): %s)\n' "$p" "$n" \
        "$(printf '%s\n' "$live" | awk -F'\t' -v p="$p" -v n="$n" '$1 == p && $3 == n { printf "%s ", $2 }')"
    done
    rc=1
  fi
  if [ -n "$stale" ]; then
    red "FAIL: STALE allowlist entr(y/ies) — the site no longer violates; remove the row from $(basename "$ALLOWLIST") in the same change:"
    printf '%s\n' "$stale" | sed 's/^/  - /'
    rc=1
  fi
  return "$rc"
}

# `--list`: print every live site (path<TAB>line<TAB>mod), ignoring the allowlist, and
# stop — for comparing against an independent implementation of the same property.
if [ "${1:-}" = "--list" ]; then live_sites "$ROOT"; exit 0; fi

# ---- controls: prove the check fires before trusting its verdict -----------------
if ! st="$(mktemp -d)" || [ -z "$st" ] || [ ! -d "$st" ]; then
  red "FAIL: outer-inner-module-doc self-test could not create a scratch dir (TMPDIR/disk?)"
  exit 1
fi
trap 'rm -rf "$st"' EXIT
ctl_ok=0
mk() { # $1 = case name; fixture repo at $st/$1
  mkdir -p "$st/$1"; git -C "$st/$1" init -q
}
fin() { git -C "$st/$1" add -A; }
w() { mkdir -p "$(dirname "$1")"; printf '%b' "$2" > "$1"; } # w <file> <content with \n escapes>
probe() { # $1 = red|green, $2 = case, $3 = allowlist file, $4 = required diagnostic substring (red)
  local out rc=0
  out="$(check "$st/$2" "$3" 2>&1)" || rc=$?
  if [ "$1" = green ] && [ "$rc" -ne 0 ]; then
    red "FAIL: self-test '$2' should be green but was red:"; printf '%s\n' "$out" >&2; ctl_ok=1
  fi
  if [ "$1" = red ]; then
    if [ "$rc" -eq 0 ]; then
      red "FAIL: self-test '$2' was NOT caught — this wire cannot fire."; ctl_ok=1
    else
      case "$out" in *"$4"*) ;; *) red "FAIL: self-test '$2' went red for the WRONG reason (no '$4')."; ctl_ok=1 ;; esac
    fi
  fi
}
EMPTY="$st/empty.tsv"; : > "$EMPTY"

mk m1; w "$st/m1/src/mod.rs" '/// outer doc\nmod foo;\n'; w "$st/m1/src/foo.rs" '//! inner\n'; fin m1
probe red m1 "$EMPTY" "+ src/mod.rs  mod foo;"
mk m2; w "$st/m2/src/mod.rs" '/// outer doc\n#[cfg(test)]\nmod foo;\n'; w "$st/m2/src/foo.rs" '\n//! inner\n'; fin m2
probe red m2 "$EMPTY" "+ src/mod.rs  mod foo;"
mk m3; w "$st/m3/src/lib.rs" '/// outer doc\n#[path = "other/bar.rs"]\nmod foo;\n'; w "$st/m3/src/other/bar.rs" '//! inner\n'; fin m3
probe red m3 "$EMPTY" "+ src/lib.rs  mod foo;"
mk m4; w "$st/m4/src/a.rs" '/// outer doc\npub(crate) mod b;\n'; w "$st/m4/src/a/b.rs" '//! inner\n'; fin m4
probe red m4 "$EMPTY" "+ src/a.rs  mod b;"
mk m5; w "$st/m5/src/lib.rs" '/// outer doc\nmod b;\n'; w "$st/m5/src/b/mod.rs" '//! inner\n'; fin m5
probe red m5 "$EMPTY" "+ src/lib.rs  mod b;"
# negative controls
mk n1; w "$st/n1/src/mod.rs" '/// outer doc\nmod foo;\n'; w "$st/n1/src/foo.rs" 'use x;\n//! not first\n'; fin n1
probe green n1 "$EMPTY"
mk n2; w "$st/n2/src/mod.rs" 'mod foo;\n'; w "$st/n2/src/foo.rs" '//! inner\n'; fin n2
probe green n2 "$EMPTY"
mk n3; w "$st/n3/src/mod.rs" '/// outer doc\n\nmod foo;\n'; w "$st/n3/src/foo.rs" '//! inner\n'; fin n3
probe green n3 "$EMPTY"
# allowlist: a listed violation is green; a listed non-violation is stale (red)
AL="$st/allow.tsv"; printf 'src/mod.rs\tfoo\tVM P4\tpending\n' > "$AL"
probe green m1 "$AL"
probe red n1 "$AL" "STALE allowlist"
printf 'src/mod.rs\tfoo\n' > "$st/badrow.tsv"
probe red m1 "$st/badrow.tsv" "malformed allowlist"

[ "$ctl_ok" -eq 0 ] || exit 1

echo "outer-inner-module-doc: no outer /// + inner //! pair on a module declaration"
if check "$ROOT" "$ALLOWLIST"; then
  green "OK"
else
  exit 1
fi
