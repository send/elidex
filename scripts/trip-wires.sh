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
cd "$(dirname "$0")/.."

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

ran_names=""
for w in .claude/tools/*-trip-wire.sh; do
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
    [ "$w" = '.claude/tools/*-trip-wire.sh' ] || echo "WARN: skipping unreadable $w" >&2
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
