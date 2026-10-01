#!/usr/bin/env bash
# THE GENERATED MUTATION SET for `webref-generic-core-trip-wire.sh` — sourced by
# that wire's mutation set (`…trip-wire.mutations.sh`), never run on its own.
#
# WHY IT IS A SEPARATE FILE. The mutation set holds two populations, and its
# header draws the boundary between them: the hand-written records, whose unit
# is a LABEL, and this one, GENERATED from `$K2RE` and `$K2RE_PATH`, whose unit
# is a RULE of those two regexes. That boundary is the seam. The split is taken
# before the fixture-git rebuild adds its infrastructure and records to the
# mutation set, which would otherwise take it past 1000 lines
# (docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-reviews.md §8.1).
# WHAT IT CONSUMES: `$SELF`, `$_VFY`, `$K2RE` and `$K2RE_PATH` (from the
# wire), `$_mg_lbl` (from the controls file), and `_mut_trial`, `_mut_target`
# (whose wire pair `_mut_splice` writes through) and `_mut_restore_copies`
# (from the mutation set, which refuses to run without this file). WHAT IT
# DEFINES: `_mut_equivalent`, `_mut_assign_value`, `_mut_regex_mutants`,
# `_mut_splice`, `_mut_gen_floor` (which sets `_mut_gen_floor_n`), `_mut_gen_run`.

# ---- THE GENERATED SET: THE TWO REGEXES' OWN STRUCTURE ----------------------
# WHY IT IS GENERATED AND THE HAND SET IS NOT. Every boundary defect this wire
# has had was found the same way: somebody enumerated the population BY HAND
# and missed part of it. Two attestations in a row declared that population
# complete; the first missed the leading `/`, the class's `A-Z` and a
# one-character final segment, and the second — on the head that fixed those —
# found six more. A hand list is exactly the instrument that cannot see what it
# forgot, so the population here is DERIVED from the assignments and the
# derivation is the criterion:
#
#   for every member of every bracket expression in `$K2RE` and `$K2RE_PATH`,
#   the value with that member DROPPED (a widening: over-match, false
#   positive); for every quantifier, the value TIGHTENED (`*`->`+`,
#   `+`->`{2,}`: under-match, false negative); for every alternation, the
#   value with one BRANCH DROPPED, once per branch (a narrowing); and for
#   every escape, the value with the BACKSLASH REMOVED (a widening). Each must
#   red the control set, or appear in `_mut_equivalent` with the argument why
#   it cannot change a verdict. A mutant that is neither is a failure.
#   ⚠ AND THAT IS NOT EVERY DIRECTION. Widening a branch list
#   (`(skills|tools)` -> `(skills|tools|hooks)`) needs a payload this scanner
#   would have to invent, so it stays with the mutation set's hand records.
#
# ⚠ THE CLASSES ARE NOT RE-SPELLED HERE. The values come from the wire's own
# assignment lines, and a mutated value is spliced back over that same line; if
# either line is missing, duplicated, or cannot be read back as the value the
# running wire holds, the run SAYS SO AND REDS rather than testing a regex
# nobody wrote.
# ⚠ WHAT IT DOES NOT CLAIM, and the hand records do: WHICH control kills.
# The generator knows the rule, not the fixture, so it requires only that a
# CONTROL caught the mutant — a red raised by the real tree instead is reported
# as a kill by the wrong subject. Naming the control stays the hand records'
# job, which is why the two sets are not the same list and neither replaces the
# other.
# ⚠ AND IT IS OPT-IN WITH THE HAND SET, under the same `WEBREF_WIRE_MUTANTS`,
# for the same reason: one control pass per mutant.

# THE EQUIVALENCE TABLE — the only escape from "must be killed", and each entry
# carries the argument, not a name. One record per line, TAB-separated:
#   <the generated rule, exactly as the generator names it>  <why it cannot
#   change a verdict>
# ⚠ AN ENTRY THAT NAMES NO GENERATED MUTANT IS A FAILURE, not a comment: the
# names embed the class text, so any edit to a class retires every argument
# made about it and the next reader has to make them again.
# ⚠ IT IS EMPTY, AND THAT IS A MEASUREMENT. Every mutant the generator returns
# is pinned by a fixture; none has been shown unable to change a verdict.
# ⚠ ONE ARGUMENT WAS OFFERED FOR THIS TABLE AND IT IS FALSE — recorded so it
# is not offered again. It said that widening the FIRST SEGMENT's class by
# dropping `/` cannot change a verdict, "because any mutant match implies a
# baseline match at the same start". It does not: the first segment may then
# END at a later `/`, which lets a match START where `[^/[:space:]]+` could
# not — measured, `.claude/tools/foo//x` reads GREEN at baseline and RED with
# that one member dropped, and it is the `midclass` fixture's `/` line. An
# equivalence argument is a claim about every input, so what it needs is the
# counter-example searched for, not the regex reread.
_mut_equivalent() { cat <<'EQUIV'
EQUIV
}

# $1 = variable name. Puts the value the WIRE'S OWN assignment line holds into
# `$_mut_av_out`. Fails, setting `$_mut_gen_fail`, unless there is exactly one
# such line and it can be read back.
# ⚠ IT SETS A VARIABLE RATHER THAN PRINTING. Called as `$(…)` it would run in a
# subshell, and every diagnostic it writes — the whole point of failing closed
# here — would be discarded with it.
_mut_assign_value() {
  # $2 = the file to read (default: the wire). The generated half reads its own
  # spliced copy back through this same parser, so "what the mutant says" and
  # "what the mutant is" cannot drift apart.
  _av_f="${2:-$SELF}"
  _av_n="$(grep -c "^$1='" "$_av_f")" || _av_n=0
  if [ "$_av_n" -ne 1 ]; then
    _mut_gen_fail="\$$1 has $_av_n assignment line(s) matching \`^$1='\` in $_av_f, not exactly one"
    return 1
  fi
  _mut_av_out="$( ( unset "$1"; eval "$(grep "^$1='" "$_av_f")" 2>/dev/null || exit 1
                    eval "printf '%s' \"\${$1-}\"" ) )" || {
      _mut_gen_fail="\$$1's assignment line could not be read back as a value"
      return 1; }
}

# THE SCANNER. $1 = variable name (for the diagnostics), $2 = the ERE. Prints
# one mutant per line, TAB-separated: <the rule it dropped or tightened>
# <the whole mutated value>. Fails, setting `$_mut_gen_fail`, on anything it
# cannot account for — an unterminated bracket expression, an unterminated
# `[: :]`, a drop that would leave a class matching nothing, a quantifier
# outside the two it knows how to tighten, a `|` outside any group, an
# unmatched `)`, or a group left open at the end.
# ⚠ THE MEMBERS ARE POSIX BRACKET MEMBERS, NOT BYTES: a leading `]` is literal,
# `[:space:]` is one member, and `a-z` is one member. Byte-wise dropping would
# make `A-Z` three edits and `[:space:]` nine, and none of those nine is a rule
# anybody wrote.
_mut_regex_mutants() {
  _rm_v="$1"; _rm_re="$2"; _rm_len=${#_rm_re}; _rm_i=0; _rm_bi=0; _rm_qi=0
  _rm_ei=0; _rm_ai=0; _rm_gd=0
  while [ "$_rm_i" -lt "$_rm_len" ]; do
    _rm_ch="${_rm_re:$_rm_i:1}"
    if [ "$_rm_ch" = '\' ]; then
      # UNESCAPE. `\.` means the character; `.` means any. The edit is
      # structural — the backslash is removed and nothing is invented.
      _rm_ei=$((_rm_ei + 1))
      printf '%s\t%s\n' "$_rm_v escape#$_rm_ei \\${_rm_re:$((_rm_i + 1)):1} unescaped" \
        "${_rm_re:0:$_rm_i}${_rm_re:$((_rm_i + 1))}"
      _rm_i=$((_rm_i + 2)); continue
    fi
    if [ "$_rm_ch" = '*' ] || [ "$_rm_ch" = '+' ]; then
      _rm_qi=$((_rm_qi + 1))
      if [ "$_rm_ch" = '*' ]; then _rm_rep='+'; else _rm_rep='{2,}'; fi
      printf '%s\t%s\n' "$_rm_v quant#$_rm_qi $_rm_ch tightened to $_rm_rep" \
        "${_rm_re:0:$_rm_i}$_rm_rep${_rm_re:$((_rm_i + 1))}"
      _rm_i=$((_rm_i + 1)); continue
    fi
    # ⚠ AND THE OTHER QUANTIFIERS FAIL CLOSED, rather than being walked past as
    # ordinary characters. POSIX ERE's quantifier set is CLOSED — `*`, `+`, `?`
    # and `{…}` — so this is the COMPLEMENT of the two handled above, not a
    # list of things somebody thought of. Passing them over silently is the one
    # failure direction this generator must not have: a `?` or a `{n,m}` added
    # to either regex would be a rule nothing tightens, and the run would go on
    # printing that every quantifier of those two regexes is pinned.
    # Neither regex holds one today, which is exactly when the arm is cheap.
    case "$_rm_ch" in
      '?'|'{')
        _mut_gen_fail="$_rm_v: quantifier \`$_rm_ch\` at offset $_rm_i is one this scanner does not tighten"
        return 1 ;;
    esac
    # ALTERNATION. `(` and `|` used to be walked past as ordinary characters,
    # so every branch of both regexes was outside the generated population. A
    # fail-closed arm is not available: both regexes hold them. The edit is
    # DROP-A-BRANCH — structural, inventing no vocabulary. The opposite
    # direction, widening a branch list, needs a payload this scanner would
    # have to make up, and stays with the hand-written records.
    # ⚠ A `|` OUTSIDE A GROUP FAILS CLOSED, as the unhandled quantifiers do:
    # its branches are the whole expression and dropping one is not this edit.
    # Neither regex holds one today.
    if [ "$_rm_ch" = '(' ]; then
      _rm_gs[$_rm_gd]=$_rm_i; _rm_gb[$_rm_gd]=''; _rm_gd=$((_rm_gd + 1))
      _rm_i=$((_rm_i + 1)); continue
    fi
    if [ "$_rm_ch" = '|' ]; then
      if [ "$_rm_gd" -eq 0 ]; then
        _mut_gen_fail="$_rm_v: alternation \`|\` at offset $_rm_i is outside any group"
        return 1
      fi
      _rm_gb[$((_rm_gd - 1))]="${_rm_gb[$((_rm_gd - 1))]} $_rm_i"
      _rm_i=$((_rm_i + 1)); continue
    fi
    if [ "$_rm_ch" = ')' ]; then
      if [ "$_rm_gd" -eq 0 ]; then
        _mut_gen_fail="$_rm_v: unmatched \`)\` at offset $_rm_i"; return 1
      fi
      _rm_gd=$((_rm_gd - 1)); _rm_os=${_rm_gs[$_rm_gd]}; _rm_bl=${_rm_gb[$_rm_gd]}
      if [ -n "$_rm_bl" ]; then
        _rm_ai=$((_rm_ai + 1)); _rm_prev=$_rm_os; _rm_k=0
        for _rm_sep in $_rm_bl "$_rm_i"; do
          if [ "$_rm_k" -eq 0 ]; then _rm_a=$((_rm_prev + 1)); _rm_b=$_rm_sep
          else _rm_a=$_rm_prev; _rm_b=$((_rm_sep - 1)); fi
          printf '%s\t%s\n' \
            "$_rm_v alt#$_rm_ai without branch \`${_rm_re:$((_rm_prev + 1)):$(( _rm_sep - _rm_prev - 1 ))}\`" \
            "${_rm_re:0:$_rm_a}${_rm_re:$((_rm_b + 1))}"
          _rm_prev=$_rm_sep; _rm_k=$((_rm_k + 1))
        done
      fi
      _rm_i=$((_rm_i + 1)); continue
    fi
    if [ "$_rm_ch" != '[' ]; then _rm_i=$((_rm_i + 1)); continue; fi
    _rm_bs=$_rm_i; _rm_bi=$((_rm_bi + 1)); _rm_j=$((_rm_i + 1))
    [ "${_rm_re:$_rm_j:1}" != '^' ] || _rm_j=$((_rm_j + 1))
    _rm_first=1; _rm_ms=(); _rm_ml=()
    while :; do
      if [ "$_rm_j" -ge "$_rm_len" ]; then
        _mut_gen_fail="$_rm_v: unterminated bracket expression at offset $_rm_bs"; return 1
      fi
      _rm_mc="${_rm_re:$_rm_j:1}"
      if [ "$_rm_mc" = ']' ] && [ "$_rm_first" -eq 0 ]; then break; fi
      _rm_first=0
      if [ "${_rm_re:$_rm_j:2}" = '[:' ]; then
        _rm_k=$((_rm_j + 2))
        while [ "$_rm_k" -lt "$_rm_len" ] && [ "${_rm_re:$_rm_k:2}" != ':]' ]; do _rm_k=$((_rm_k + 1)); done
        if [ "$_rm_k" -ge "$_rm_len" ]; then
          _mut_gen_fail="$_rm_v: unterminated [: :] class at offset $_rm_j"; return 1
        fi
        _rm_mlen=$(( _rm_k + 2 - _rm_j ))
      elif [ "${_rm_re:$((_rm_j + 1)):1}" = '-' ] && [ -n "${_rm_re:$((_rm_j + 2)):1}" ] \
           && [ "${_rm_re:$((_rm_j + 2)):1}" != ']' ]; then
        _rm_mlen=3
      else
        _rm_mlen=1
      fi
      _rm_ms[${#_rm_ms[@]}]=$_rm_j; _rm_ml[${#_rm_ml[@]}]=$_rm_mlen
      _rm_j=$(( _rm_j + _rm_mlen ))
    done
    _rm_be=$_rm_j
    _rm_bt="${_rm_re:$_rm_bs:$(( _rm_be - _rm_bs + 1 ))}"
    _rm_n=0
    while [ "$_rm_n" -lt "${#_rm_ms[@]}" ]; do
      _rm_s=${_rm_ms[$_rm_n]}; _rm_l=${_rm_ml[$_rm_n]}
      _rm_nb="${_rm_re:$_rm_bs:$(( _rm_s - _rm_bs ))}${_rm_re:$(( _rm_s + _rm_l )):$(( _rm_be - _rm_s - _rm_l + 1 ))}"
      # ⚠ A NEGATED CLASS WITH ONE MEMBER BECOMES `.`, NOT `[^]`. `[^]` is not a
      # class at all (POSIX reads the `]` as a literal member and keeps
      # looking), so spelling the widening that way would test a regex nobody
      # means. "Not `/`" widened by dropping `/` IS "any character".
      case "$_rm_nb" in
        '[^]') _rm_nb='.' ;;
        '[]')  _mut_gen_fail="$_rm_v: dropping ${_rm_re:$_rm_s:$_rm_l} from $_rm_bt leaves a class matching nothing"; return 1 ;;
      esac
      printf '%s\t%s\n' "$_rm_v class#$_rm_bi $_rm_bt without ${_rm_re:$_rm_s:$_rm_l}" \
        "${_rm_re:0:$_rm_bs}$_rm_nb${_rm_re:$(( _rm_be + 1 ))}"
      _rm_n=$((_rm_n + 1))
    done
    _rm_i=$((_rm_be + 1))
  done
  if [ "$_rm_gd" -ne 0 ]; then
    _mut_gen_fail="$_rm_v: $_rm_gd group(s) left open at the end of the expression"; return 1
  fi
}

# Splice a mutated value back over the wire's own assignment line, from
# `$_mut_src` into `$_mut_tgt` — the wire's pair, which `_mut_gen_run` resolves
# through `_mut_target`. $1 = variable name, $2 = the value.
# ⚠ NOT `sed`: the value holds `/`, `&`, `\` and both quotes, so every
# replacement would need escaping in a second dialect — one more spelling of
# the thing this generator exists to stop spelling twice. The whole line is
# rewritten instead, and the value reaches `awk` through the environment, which
# interprets nothing.
_mut_splice() {
  # ⚠ THE QUOTING IS BUILT AS DATA, NOT WRITTEN AS SOURCE. Spelled inline as
  # backslash escapes, the replacement text is read differently by bash 3.2 and
  # 5.x: 3.2 keeps the backslashes, so the spliced line carried `\'` where it
  # meant `'"'"'`, the mutant's predicate was NOT the one the entry names, and
  # every generated mutant "SURVIVED" under 3.2 while none did under 5.3
  # (measured, PR519 — the wire is run under both on purpose). Both strings come
  # from `printf` so the shell never re-reads them.
  _sp_q="$(printf "'")"; _sp_r="$(printf "'\"'\"'")"
  _sp_v="${2//$_sp_q/$_sp_r}"
  _MUT_GEN_LINE="$1='$_sp_v'" _MUT_GEN_VAR="$1" awk '
    BEGIN { v = ENVIRON["_MUT_GEN_VAR"] "="; l = ENVIRON["_MUT_GEN_LINE"] }
    index($0, v) == 1 { print l; next }
    { print }' "$_mut_src" > "$_mut_tgt" || return 1
  # ⚠ AND IT IS READ BACK. A copy that holds a DIFFERENT value than the entry
  # names tests a different question, and when that value happens to change no
  # verdict the trial reports "survived" — sending a reader to add a control
  # that is not missing. Same class as the shipped set's "MATCHED NOTHING".
  _mut_assign_value "$1" "$_mut_tgt" || return 1
  [ "$_mut_av_out" = "$2" ] || {
    _mut_gen_fail="$1: the spliced copy reads back as a different value than this entry names"
    return 1; }
}

# ALWAYS ON (called by `_mut_correspondence`): the generated set over the
# running wire's `$K2RE` and `$K2RE_PATH` is not empty. An empty set tests no
# rule, and the opt-in run would print `0 mutant(s)` beside `0 neither killed
# nor argued equivalent`, a pass — the blind this guards against.
_mut_gen_floor() {
  _mgf_n=0; _mut_gen_fail=""
  for _mgf_v in K2RE K2RE_PATH; do
    eval "_mgf_val=\"\${$_mgf_v-}\""
    _mut_regex_mutants "$_mgf_v" "$_mgf_val" > "$_VFY/.genfloor" || {
      echo "!! CONTROL FAILED ($_mg_lbl): generating \$$_mgf_v's mutants failed${_mut_gen_fail:+: $_mut_gen_fail}" >&2
      command rm -f "$_VFY/.genfloor"; return 1; }
    _mgf_n=$((_mgf_n + $(awk 'END{print NR}' "$_VFY/.genfloor")))
  done
  command rm -f "$_VFY/.genfloor"
  _mut_gen_floor_n="$_mgf_n"   # the run-time layer in `_mut_run` compares its own count with this
  [ "$_mgf_n" -gt 0 ] || {
    echo "!! CONTROL FAILED ($_mg_lbl): no mutant from \$K2RE or \$K2RE_PATH" >&2
    return 1; }
}

# The generated half of the run. Sets `_mut_gen_n` / `_mut_gen_bad`.
_mut_gen_run() {
  # This half only ever edits the wire, named through the one resolver — so it
  # cannot inherit whatever the last hand record left behind.
  _mut_restore_copies || { _mut_gen_bad=$((_mut_gen_bad + 1)); return 0; }
  _mut_target wire
  _mut_gen_fail=""
  : > "$_VFY/.genmutants"
  for _gr_v in K2RE K2RE_PATH; do
    _mut_assign_value "$_gr_v" || { _mut_gen_bad=1; break; }
    _gr_val="$_mut_av_out"
    # …and the line must hold what the RUNNING wire holds, or the generator is
    # mutating a spelling that is no longer live.
    eval "_gr_live=\"\${$_gr_v-}\""
    if [ "$_gr_val" != "$_gr_live" ]; then
      _mut_gen_fail="\$$_gr_v's assignment line reads back as a different value than the running wire holds"
      _mut_gen_bad=1; break
    fi
    _mut_regex_mutants "$_gr_v" "$_gr_val" >> "$_VFY/.genmutants" || { _mut_gen_bad=1; break; }
  done
  if [ -n "$_mut_gen_fail" ]; then
    echo "!! the boundary-mutant generator could not read the wire's regexes:" >&2
    echo "   $_mut_gen_fail" >&2
    echo "   No rule of \$K2RE or \$K2RE_PATH was tested, so this run decided nothing" >&2
    echo "   about either predicate's structure." >&2
    return 0
  fi
  _mut_equivalent > "$_VFY/.genequiv"
  : > "$_VFY/.genseen"
  while IFS="$(printf '\t')" read -r _gr_name _gr_re; do
    [ -n "${_gr_re:-}" ] || continue
    _mut_gen_n=$((_mut_gen_n + 1))
    printf '%s\n' "$_gr_name" >> "$_VFY/.genseen"
    if ! _mut_splice "${_gr_name%% *}" "$_gr_re"; then
      echo "!! MUTANT $_gr_name: the mutated assignment could not be written, so" >&2
      echo "   this rule was not tested. Unknown fails closed here as everywhere." >&2
      _mut_gen_bad=$((_mut_gen_bad + 1)); continue
    fi
    _gr_rc=0; _mut_trial "$_gr_name" '!kill' || _gr_rc=$?
    [ "$_gr_rc" -ne 2 ] || { _mut_gen_bad=$((_mut_gen_bad + 1)); continue; }
    [ "$_gr_rc" -eq 1 ] || continue
    # SURVIVED. The only way that is not a gap is an argued equivalence.
    _gr_why="$(awk -F'\t' -v n="$_gr_name" '$1==n{print $2; exit}' "$_VFY/.genequiv")"
    if [ -z "$_gr_why" ]; then
      echo "!! GENERATED MUTANT SURVIVED: $_gr_name" >&2
      echo "   The wire still exited 0 with that rule widened or tightened, so no" >&2
      echo "   control poses the question it answers. Either add the control, or —" >&2
      echo "   if the mutant cannot change ANY verdict — say why in \`_mut_equivalent\`." >&2
      echo "   The mutated predicate was: $_gr_re" >&2
      _mut_gen_bad=$((_mut_gen_bad + 1))
    fi
  done < "$_VFY/.genmutants"
  # …and an argument nobody is making any more is not documentation.
  while IFS="$(printf '\t')" read -r _gr_name _; do
    [ -n "${_gr_name:-}" ] || continue
    grep -qxF -- "$_gr_name" "$_VFY/.genseen" || {
      echo "!! \`_mut_equivalent\` claims \"$_gr_name\", which this run's generator does" >&2
      echo "   not produce. The class or quantifier it argued about was edited, so the" >&2
      echo "   argument has to be made again against what is there now." >&2
      _mut_gen_bad=$((_mut_gen_bad + 1)); }
  done < "$_VFY/.genequiv"
  command rm -f "$_VFY/.genmutants" "$_VFY/.genequiv" "$_VFY/.genseen"
}
