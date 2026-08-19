#!/usr/bin/env bash
# claim provenance trip-wire.
#
# Re-executes every `claim` annotation in the tracked tree and fails the push if
# any asserted figure disagrees with what the command actually reports. A number
# in a document is a claim about the tree at the moment it was written; nothing
# re-derives it, so it goes stale silently and in the direction that reads as
# still-true. This wire is the re-derivation.
#
# Grammar (one line, anywhere on the line, followed by anything):
#
#   <!-- claim count=N [icase] pattern="ERE" in="PATHSPEC" -->
#   <!-- claim files=N [icase] pattern="ERE" in="PATHSPEC" -->
#   <!-- claim lines=N                       in="PATHSPEC" -->
#
# An annotation preceded by an odd number of backticks on its line is a MENTION
# and is ignored -- that is how this header can spell one without executing it.
#
# Cost, as a shape rather than a clock: ONE tree-wide `git grep` for the scan,
# then a CONSTANT number of commands per annotation -- never a term per tracked
# file and never one per resolved path. C27 asserts exactly that, by counting
# `git` invocations in a small tree and in a large one and requiring the same
# number, so the bound is decidable on every push without a stopwatch or a
# threshold. (A wall-clock trigger was tried and withdrawn: nothing here has a
# sub-second timer -- stock macOS is bash 3.2, `SECONDS` and `date +%s` are
# whole seconds -- so the comparison it named could not be evaluated by the
# program that was asked to evaluate it.)
#
# When a clause needs work the shape cannot hold, RE-PLAN: reduce the
# annotation set, or change the execution model. Do NOT batch by `in=`:
# no-eval forbids the shape, and `git grep` cannot attribute a batched result
# to the annotation that asked for it.
#
# Scope note: this wire measures the repository it is POINTED AT (cwd), while
# the driver anchors to where the wires are ($0). Two roots, two rules, each for
# its own job -- see resolve_roots below.

set -euo pipefail

# --------------------------------------------------------------------------
# Roots. There are two, and conflating them is the defect this block exists to
# prevent: the tree under test comes from the invoking cwd, while the wire's own
# helper sits beside the wire wherever that has been copied to. A control that
# copies the wire into a fixture directory has cwd = fixture and $0 = fixture,
# so both resolve there; a control that runs the installed wire against a clone
# has cwd = clone and $0 = this repository.
# --------------------------------------------------------------------------
SCAN_ROOT=$(git rev-parse --show-toplevel)
WIRE_DIR=$(cd "$(dirname "$0")" && pwd)
SELFTEST="$WIRE_DIR/claim-provenance-selftest.sh"

FAILED=0
ERRFILE=$(mktemp "${TMPDIR:-/tmp}/claim-wire.XXXXXX")
trap 'rm -f "$ERRFILE"' EXIT

note() { printf '%s\n' "$*"; }
fail() { FAILED=1; printf '%s\n' "$*"; }

# --------------------------------------------------------------------------
# The helper is required BY NAME, unconditionally, and only its INVOCATION sits
# under the recursion guard. The order matters and it is not a preference: the
# guard is an inherited environment variable, so folding the existence check
# inside it means anything that sets the variable -- including a fork PR's own
# script, or a stale export in a developer shell -- silently turns the whole
# self-test into a no-op on the ungated per-push path, and the wire still
# exits 0. A gate that can be disabled by an env var is not a gate.
# --------------------------------------------------------------------------
if [ ! -r "$SELFTEST" ]; then
  fail "FAIL  self-test helper not readable at $SELFTEST"
  exit 1
fi
if [ -z "${CLAIM_WIRE_SELFTEST:-}" ]; then
  CLAIM_WIRE_SELFTEST=1 bash "$SELFTEST" || exit 1
fi

# --------------------------------------------------------------------------
# Scan. One tree-wide command. Its exit status is a datum -- 1 means "no
# annotation anywhere", which is a legal tree, not a failure -- so it is
# captured and branched on rather than left bare.
# --------------------------------------------------------------------------
cd "$SCAN_ROOT"
LAX='<!--[[:space:]]*claim[^A-Za-z]'
if candidates=$(git grep -nE "$LAX" -- \
      '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**'); then
  :
else
  rc=$?
  [ "$rc" -eq 1 ] || { fail "FAIL  the scan command failed: git exited $rc"; exit 1; }
  candidates=''
fi

[ -n "$candidates" ] || { note "OK    no claim annotation in the tracked tree"; exit 0; }

# --------------------------------------------------------------------------
# Per candidate: classify, then measure. No arm exits; every diagnostic
# accumulates and the wire exits once, at the end.
# --------------------------------------------------------------------------
while IFS= read -r hit; do
  [ -n "$hit" ] || continue
  file=${hit%%:*}; rest=${hit#*:}
  line=${rest%%:*}; text=${rest#*:}

  before=${text%%<!--*}
  ticks=$(printf '%s' "$before" | tr -cd '`' | wc -c | tr -d ' ')
  if [ $(( ticks % 2 )) -eq 1 ]; then
    note "MENTION  $file:$line"
    continue
  fi

  body=${text#*<!--}; body=${body%%-->*}
  # Word-split the body into fields, with pathname expansion OFF. The splitting
  # is wanted; the globbing is the defect it used to carry -- an annotation body
  # containing `*` expanded against the working directory, so the MALFORMED
  # diagnostic named a file the author never wrote, and which file it named
  # depended on where the wire was run. That is the "no unquoted expansion" half
  # of argv fidelity, one field earlier than the executor.
  set -f; set -- $body; set +f
  [ "${1:-}" = claim ] || { fail "MALFORMED  $file:$line  first field: ${1:-<empty>}"; continue; }
  shift

  verb=''; expected=''; icase=''; pattern=''; spec=''
  case "${1:-}" in
    count=*|files=*|lines=*) verb=${1%%=*}; expected=${1#*=}; shift ;;
    *) fail "MALFORMED  $file:$line  first field: ${1:-<empty>}"; continue ;;
  esac
  case "$expected" in ''|*[!0-9]*) fail "MALFORMED  $file:$line  count is not an integer: $expected"; continue ;; esac
  if [ "${1:-}" = icase ]; then icase=-i; shift; fi

  rawline=$body
  if [ "$verb" != lines ]; then
    pattern=$(printf '%s' "$rawline" | sed -n 's/.*pattern="\([^"]*\)".*/\1/p')
    [ -n "$pattern" ] || { fail "MALFORMED  $file:$line  no pattern= field"; continue; }
    case "$pattern" in
      *'\b'*|*'\B'*|*'\d'*|*'\D'*|*'\s'*|*'\S'*|*'\w'*|*'\W'*|*'\<'*|*'\>'*)
        esc=$(printf '%s' "$pattern" | grep -oE '\\[bBdDsSwW<>]' | head -1)
        fail "MALFORMED  $file:$line  pattern contains $esc, which POSIX ERE reads as a literal"
        continue ;;
    esac
  else
    case "$rawline" in *pattern=*) fail "MALFORMED  $file:$line  lines= takes no pattern"; continue ;; esac
  fi
  spec=$(printf '%s' "$rawline" | sed -n 's/.*in="\([^"]*\)".*/\1/p')
  [ -n "$spec" ] || { fail "MALFORMED  $file:$line  no in= field"; continue; }

  # Liveness (D4), against the tree the verbs read. Existential: stop at the
  # first present path. The exhausting walk happens only on the branch that
  # ends this annotation.
  if tracked=$(git ls-files -- "$spec"); then :; else
    rc=$?; [ "$rc" -eq 1 ] || { fail "FAIL  $file:$line  git ls-files exited $rc"; continue; }
    tracked=''
  fi
  [ -n "$tracked" ] || { fail "FAIL  $file:$line  in= matches no tracked file: $spec"; continue; }

  present=''
  while IFS= read -r p; do
    [ -e "$p" ] && { present=$p; break; }
  done <<EOF
$tracked
EOF
  [ -n "$present" ] || {
    fail "FAIL  $file:$line  in= resolves to tracked paths, none present in the working tree: $spec"
    continue; }

  if [ "$verb" = lines ]; then
    n_paths=$(printf '%s\n' "$tracked" | grep -c .)
    [ "$n_paths" -eq 1 ] || { fail "FAIL  $file:$line  lines= resolves to more than one tracked file ($n_paths): $spec"; continue; }
    [ -f "$present" ] || { fail "FAIL  $file:$line  lines= resolves to a path that is not a regular file: $present"; continue; }
    : > "$ERRFILE"
    if actual=$(awk 'END{print NR}' "$present" 2>"$ERRFILE"); then :; else
      rc=$?
      fail "FAIL  $file:$line  the measurement command failed: awk exited $rc: $(cat "$ERRFILE")"
      continue
    fi
  else
    case "$verb" in count) flag=-nE ;; files) flag=-lE ;; esac
    : > "$ERRFILE"
    if out=$(git grep $flag $icase -e "$pattern" -- "$spec" 2>"$ERRFILE"); then
      actual=$(printf '%s' "$out" | grep -c .)
    else
      rc=$?
      if [ "$rc" -ne 1 ]; then
        fail "FAIL  $file:$line  the measurement command failed: git exited $rc: $(cat "$ERRFILE")"
        continue
      fi
      actual=0
    fi
  fi

  if [ "$actual" = "$expected" ]; then
    note "OK    $verb asserted=$expected actual=$actual  in=$spec"
  else
    fail "FAIL  $verb asserted=$expected actual=$actual  in=$spec   ($file:$line)"
  fi
done <<EOF
$candidates
EOF

exit "$FAILED"
