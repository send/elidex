#!/usr/bin/env bash
#
# `mise run trip-wires` and the `trip-wires` job in `.github/workflows/ci.yml`:
# run every layering trip-wire. Discovery is the glob below — dropping a
# `.claude/tools/<name>-trip-wire.sh` in enrols it in both the local gate and CI
# with no list to update. Retention is `REQUIRED_WIRES` below; see the comment
# there for why the two directions are deliberately asymmetric. Each wire's own
# header documents the surface it covers.
#
# One script rather than the same loop inlined at both call sites: the two would
# be a hand-kept assertion that they are identical, which is exactly what having
# one file makes structurally true instead. Same reason `mise.toml [tasks.doc]`
# delegates to `scripts/doc-changed.sh` — and, as there, both call sites invoke it
# as `bash scripts/trip-wires.sh`, so a Windows contributor without bash gets a
# clear "bash: command not found" rather than a confusing partial execute from
# whatever shell mise picks per platform. (The shebang is for direct execution;
# it is not consulted when the file is passed as an argument to `bash`.)
#
# The CI job running this is deliberately UNGATED by the paths-filter. That
# rationale lives with the job it explains, in `.github/workflows/ci.yml`.

set -euo pipefail

# The glob is repo-root-relative, so don't inherit the caller's cwd. (No arrays or
# `shopt` here: /bin/bash on stock macOS is 3.2, and the wires already record that
# hazard class — see layout-box-reader-trip-wire.sh on `declare -A`.)
#
# Resolve symlinks first: a bare `dirname "$0"` points at the LINK, so this script
# symlinked onto someone's PATH would `cd` outside the repo, match no wires, and
# report it as "required trip-wire(s) did not run" — telling the reader that four
# wires were deleted and inviting them to edit REQUIRED_WIRES, which is the one
# edit that genuinely disables the gate. A gate whose diagnostic misdirects toward
# switching it off is worse than one that simply refuses to run. `readlink -f` is
# not portable to stock macOS, so walk the chain by hand.
src="$0"
while [ -L "$src" ]; do
  link_dir="$(cd -P "$(dirname "$src")" && pwd)"
  src="$(readlink "$src")"
  case "$src" in
    /*) ;;
    *) src="$link_dir/$src" ;;
  esac
done
root="$(cd -P "$(dirname "$src")/.." && pwd)"
cd "$root"

# …and verify the result rather than assuming it, so "the root is wrong" can never
# be reported as "the wires are gone".
if [ ! -d .claude/tools ]; then
  echo "FAIL: resolved repo root '$root' has no .claude/tools/ directory, so this" >&2
  echo "      run gated nothing. This is a cwd/invocation problem, NOT a missing" >&2
  echo "      wire — do not 'fix' it by editing REQUIRED_WIRES below." >&2
  exit 1
fi

# Wires that MUST still be present. Membership is asserted in ONE direction only,
# and the asymmetry is the point:
#   * ADDING a wire needs no edit here — the glob enrols it. More gating is the
#     safe direction, and a list you must remember to extend is the maintenance
#     burden this driver exists to avoid.
#   * REMOVING one must be deliberate. A wire that is deleted, renamed off the
#     `*-trip-wire.sh` convention, or moved into a subdirectory would otherwise
#     leave a SMALLER set running, exit 0, and report green — the gate quietly
#     covering less than the docs claim. That is the same "hole in the shape of
#     the gate's own tamper path" the ungated CI job (see
#     `.github/workflows/ci.yml`) exists to close, one level up; a glob alone
#     re-opens it here. It is also the shape the wires themselves already use:
#     `layout-box-reader-trip-wire.sh` does not trust the live tree either, it
#     diffs it against a committed allowlist.
REQUIRED_WIRES="
layout-box-reader-trip-wire.sh
native-ctor-guard-trip-wire.sh
wasm-runtime-trip-wire.sh
wasm-vm-trip-wire.sh
"

# Guard the guard: an emptied REQUIRED_WIRES would make every check below vacuous.
if [ -z "$(printf '%s' "$REQUIRED_WIRES" | tr -d '[:space:]')" ]; then
  echo "FAIL: REQUIRED_WIRES is empty — this driver would assert nothing at all." >&2
  exit 1
fi

# Written once and expanded unquoted below so it globs. Spelling the pattern a second
# time as a literal would put the very duplication this driver exists to remove back
# into the driver: the unmatched-glob comparison has to be the SAME pattern, and a
# hand-kept second copy silently stops matching the moment the convention changes.
WIRE_GLOB='.claude/tools/*-trip-wire.sh'

# Positive + negative control, the same discipline `layout-box-reader-trip-wire.sh`
# applies to its own ban patterns: "a gate that cannot fire is indistinguishable from a
# clean tree, so trusting its verdict requires first proving it fires." This driver
# decides whether that wire runs AT ALL, so its two assertions — root verification and
# wire retention — are in the same load-bearing class and get the same treatment. That
# file also records a wire #5 that was WITHDRAWN for claiming a bound it did not have,
# which is the precedent for not accepting hand-verification as a control: a check
# nobody re-runs is a transcript, not a gate.
#
# `TRIP_WIRES_SELFTEST` stops the fixture invocations below from recursing.
if [ -z "${TRIP_WIRES_SELFTEST:-}" ]; then
  # Distinguish an environment failure from a dead assertion — with the dir empty the
  # probes would exercise nothing and silently "pass".
  if ! st_dir="$(mktemp -d)" || [ -z "$st_dir" ] || [ ! -d "$st_dir" ]; then
    echo "FAIL: could not create a scratch dir for the driver self-test (TMPDIR/disk?)," >&2
    echo "      so this run's assertions were never proved able to fire." >&2
    exit 1
  fi

  # A fixture repo: this script, plus stub wires named exactly as REQUIRED_WIRES.
  mkdir -p "$st_dir/scripts" "$st_dir/.claude/tools"
  cp "$src" "$st_dir/scripts/trip-wires.sh"
  for req in $REQUIRED_WIRES; do
    printf '#!/usr/bin/env bash\nexit 0\n' > "$st_dir/.claude/tools/$req"
  done

  # $3 (a substring the diagnostic MUST contain) is not decoration: the two failing
  # assertions produce the SAME exit code, and the retention check fires on a wrong root
  # too (no root -> no wires -> all four missing). Keying on the status alone therefore
  # cannot tell "root verification works" from "retention masked its absence" — verified:
  # deleting the root check left an exit-code-only probe green. Asserting the message is
  # what pins the property that matters, namely that a cwd problem is never reported as
  # deleted wires.
  st_probe() { # $1 = expected pass|fail, $2 = label, $3 = required diagnostic substring
    local rc=0 out
    out="$(TRIP_WIRES_SELFTEST=1 bash "$st_dir/scripts/trip-wires.sh" 2>&1)" || rc=$?
    if [ "$1" = pass ] && [ "$rc" -ne 0 ]; then
      echo "FAIL: driver self-test — '$2' should have passed but exited $rc. The driver" >&2
      echo "      reds a healthy tree, which teaches people to ignore it." >&2
      return 1
    fi
    if [ "$1" = fail ]; then
      if [ "$rc" -eq 0 ]; then
        echo "FAIL: driver self-test — '$2' was NOT caught. That assertion cannot fire, so" >&2
        echo "      this run's OK verdict would be meaningless." >&2
        return 1
      fi
      case "$out" in
        *"$3"*) ;;
        *)
          echo "FAIL: driver self-test — '$2' failed for the WRONG reason: the diagnostic" >&2
          echo "      does not contain '$3', so another check masked the one under test." >&2
          return 1
          ;;
      esac
    fi
    return 0
  }

  # ⚠ Every st_probe call MUST be an operand of `||`: `set -e` is suspended only there,
  # so a bare call would abort at the first failure instead of printing the diagnostic.
  st_ok=0
  st_probe pass 'complete wire set' || st_ok=1
  mv "$st_dir/.claude/tools/layout-box-reader-trip-wire.sh" \
     "$st_dir/.claude/tools/layout-box-reader.disabled" || st_ok=1
  st_probe fail 'a required wire renamed off the convention' \
    'required trip-wire(s) did not run' || st_ok=1
  mv "$st_dir/.claude/tools/layout-box-reader.disabled" \
     "$st_dir/.claude/tools/layout-box-reader-trip-wire.sh" || st_ok=1
  st_probe pass 'wire restored' || st_ok=1
  rm -rf "$st_dir/.claude"
  st_probe fail 'a resolved root with no .claude/tools' \
    'has no .claude/tools/ directory' || st_ok=1

  rm -rf "$st_dir"
  [ "$st_ok" -eq 0 ] || exit 1
fi

ran_names=""
# shellcheck disable=SC2086  # unquoted on purpose — this is the glob expansion
for w in $WIRE_GLOB; do
  # nullglob is off, so an unmatched glob arrives as the literal pattern. `continue`,
  # not `break`: a `break` here would abandon every remaining wire on any single
  # unreadable entry (a dangling symlink, say) and — with earlier wires already run —
  # still exit 0. Silent truncation is the failure mode REQUIRED_WIRES exists to catch,
  # so don't hand it a second way in.
  if [ ! -e "$w" ]; then
    # The unmatched-glob literal is the expected case and needs no noise — the
    # missing-check below turns it into a FAIL. Anything else reaching here is a
    # real directory entry we could not stat (a dangling symlink, say); say so,
    # or this `continue` becomes the one path that drops a wire in silence.
    [ "$w" = "$WIRE_GLOB" ] || echo "WARN: skipping unreadable $w" >&2
    continue
  fi
  echo "===== $w"
  bash "$w"
  ran_names="$ran_names $(basename "$w")"
done

missing=""
for req in $REQUIRED_WIRES; do
  case " $ran_names " in
    *" $req "*) ;;
    *) missing="$missing $req" ;;
  esac
done

if [ -n "$missing" ]; then
  echo "FAIL: required trip-wire(s) did not run:$missing" >&2
  echo "      The gate silently covers less than the docs claim when a wire leaves the" >&2
  echo "      set. If the removal is deliberate, drop it from REQUIRED_WIRES in this" >&2
  echo "      script in the SAME commit, so shrinking the gate is a visible edit." >&2
  exit 1
fi
