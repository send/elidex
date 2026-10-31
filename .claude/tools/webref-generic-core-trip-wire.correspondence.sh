#!/usr/bin/env bash
# THE ALWAYS-ON HALF of the mutation set for `webref-generic-core-trip-wire.sh` —
# sourced by that wire's mutation set (`…trip-wire.mutations.sh`), never run on
# its own.
#
# WHY IT IS A SEPARATE FILE. The mutation set draws its own seam: the opt-in
# half applies each record and runs the wire (minutes per entry, gated), and the
# always-on half checks the records against the labels the run used, on every
# run. That is two subjects, and the second was a third of the mutation set. The
# split is taken at the boundary its comments already drew, before the
# fixture-git rebuild's census and anchor rules take the mutation set past 1000
# lines (CLAUDE.md touch-time split). The block below began as the always-on
# half moved verbatim; the mutation set keeps the format's readers (`_mut_parse`,
# `_mut_src_for`, `_mut_target`, `_mut_apply`), the trial and the opt-in run,
# and its header states the format and the three checks this file implements.
# WHAT IT CONSUMES: `$_VFY` and the run registry in it (`$_VFY/.registry`),
# `$_cen_lbl`, `$_af_lbl`, `$_CONTROLS`, `$_lbd_files`, `$_lbd_names` and `$_MUT_PARTS` (from the controls
# file), `_mut_part_path` (from the controls file), `_cap_rows` (from the run
# primitives), `_mutants` (the record table), `_mut_parse` and `_mut_src_for`,
# `$_MUT_UNRECORDED_MAX`, `$_MUT_RECORDS_MIN` (from the mutation set, which
# refuses to run without this file), `_mut_gen_floor` (from the generated
# half), and `$SELF` (from the wire). WHAT IT DEFINES: `_mut_cap_member` (the census's membership
# callback), `_mut_fresh` (the anchor rule) and `_mut_fresh_gate` (its red), `_mut_correspondence` and `_mut_census_end` (the controls
# file's last statement).

# The census's membership callback (`_cap_rows member`): the member label is in `$_mc_ctl`.
_mut_cap_member() {
  case "$_mc_ctl" in *"$_mc_nl${_cr_f[2]}$_mc_nl"*) ;; *)
    echo "!! CONTROL FAILED ($_cen_lbl): declared capability member \"${_cr_f[2]}\" is no \`_control\` this run made" >&2
    _mut_corr_bad=1 ;;
  esac
}
# WHAT A FRESH ANCHOR IS: `_mut_fresh <expression> <target>` sets `_mf_v` to the first clause the expression
# fails, in this order, or `fresh`: `stale` / `count:<n>` (1, ONE LINE: `diff` counts exactly one changed line),
# `twice` (2, POSITION-UNIQUE: applied twice it changes nothing more than once, else a second occurrence on
# the line is where a respelling of the first would move the edit), `carry` (3, FAITHFUL: each character of
# the changed line outside the changed span — the common prefix and suffix of the line before and after —,
# replaced by \001 and, a letter or digit, by another of its class, either stops the match or comes out at its
# mapped place in the changed line, for every probe; anything else is read through a wildcard and written back
# as something else, a restated text a respelling would see reverted). A deletion or a pure insertion has no
# changed line: clauses 1 and 2 only. Clauses 2 and 3 only add reds; their false reds are cured by anchoring
# more precisely. Status 1 = a tool failed, `_mf_v` the reason (decided nothing).
# THE VARIANTS: one awk pass writes each variant with its expected image, and the target with the changed line
# replaced IN PLACE by the variants; one `sed` runs the expression over that; one awk pass compares (values
# through ENVIRON). That is exact only for a line-local expression: per command one `s`, `y` or `d` under at
# most one `/regex/` address. ANY OTHER FORM — a range (`,` after an address), a numeric or `$` address, any
# other command (`n N D G H x P`, `h g b t`, …) — can read line position or neighbouring lines, which K
# consecutive variants do not preserve: it gets one whole-target copy and one `sed` per variant, compared whole.
# ⚠ BLIND (the opt-in run's kill is the backstop): only the changed line is perturbed, only outside the changed
# span — the lines a range's addresses or a hold-space copy read (BS-8), a variable defined elsewhere, a wildcard
# INSIDE the span (`s/^k=a.c$/k=aXc/` is fresh and reverts `k=aYc`), a class whose characters no probe leaves
# (punctuation, space: `s/^k=v[[:punct:]]$/k=w,/` on `k=v,` is fresh) or a partial range the probe falls outside
# (`[a-c]`), a quantifier whose match length a substitution probe cannot change (`x*`, `x\{1,\}`), the `I` flag,
# and a pure insertion (no clause 3) can drift with the record fresh.
_mut_fresh() {
  local _mf_d=0 _mf_a=0 _mf_n _mf_h="" _mf_l _mf_rc=0 _mf_bad _mf_m _mf_k _mf_i=0
  sed -e "$1" "$2" > "$_VFY/.anchor" 2>/dev/null || { _mf_v="not a valid sed expression"; return 1; }
  diff "$2" "$_VFY/.anchor" > "$_VFY/.anchordiff" || _mf_rc=$?
  [ "$_mf_rc" -le 1 ] || { _mf_v="diff of ${2##*/} failed (exit $_mf_rc)"; return 1; }
  # Counted in the shell: the larger of the `<` and `>` lines; a one-line change's own header (`NcM`) kept.
  while IFS= read -r _mf_l; do
    case "$_mf_l" in '<'*) _mf_d=$((_mf_d + 1)) ;; '>'*) _mf_a=$((_mf_a + 1)) ;; [0-9]*c[0-9]*) _mf_h="$_mf_l" ;; esac
  done < "$_VFY/.anchordiff"
  _mf_n=$(( _mf_d > _mf_a ? _mf_d : _mf_a ))
  if [ "$_mf_n" -eq 0 ]; then _mf_v=stale; return 0; fi
  if [ "$_mf_n" -ne 1 ]; then _mf_v="count:$_mf_n"; return 0; fi
  sed -e "$1" -e "$1" "$2" > "$_VFY/.anchor2" 2>/dev/null || { _mf_v="applied twice, sed failed"; return 1; }
  _mf_rc=0; cmp -s "$_VFY/.anchor" "$_VFY/.anchor2" || _mf_rc=$?
  [ "$_mf_rc" -le 1 ] || { _mf_v="cmp of the output applied twice failed (exit $_mf_rc)"; return 1; }
  if [ "$_mf_rc" -eq 1 ]; then _mf_v=twice; return 0; fi
  _mf_v=fresh; [ "$_mf_d" -eq 1 ] && [ "$_mf_a" -eq 1 ] || return 0
  case "$_mf_h" in *[!0-9c]*|"") _mf_v="the one changed line has no \`NcM\` diff header ($_mf_h)"; return 1 ;; esac
  # The form: x = not line-local, when a command position (outside the delimited `/re/`, `s` and `y` parts, read
  # with `\` escaping as sed does) holds anything but `s` `y` `d` `/re/` `!` `{` `}` `;` — a range `,` included;
  # a regex's bracket expression is read to its `]` (POSIX), and one holding the delimiter (seds differ) is x too.
  K2_E="$1" K2_N="${_mf_h%c*}" K2_M="${_mf_h#*c}" K2_X="$_VFY/.fvexp" K2_D="$_VFY/.fv." awk '
    function sk(i, d, b,  c, j) { for (i++; i <= n; i++) { c = substr(e, i, 1); if (c == "\\") i++; else if (c == d) return i + 1
        else if (b && c == "[") { j = i + 1; if (substr(e, j, 1) == "^") j++; if (substr(e, j, 1) == "]") j++
          for (; j <= n && substr(e, j, 1) != "]"; j++) if (substr(e, j, 1) == "[" && index(":=.", substr(e, j + 1, 1))) {
            c = index(substr(e, j + 2), substr(e, j + 1, 1) "]"); j = c ? j + c + 2 : n }
          if (j > n || index(substr(e, i, j - i + 1), d)) { x = 1; return n + 1 }; i = j } }; x = 1; return i }
    function pr(c) { if (index("abcdefghijklmnopqrstuvwxyz", c)) return c == "q" ? "j" : "q"
      if (index("ABCDEFGHIJKLMNOPQRSTUVWXYZ", c)) return c == "Q" ? "J" : "Q"
      return index("0123456789", c) ? (c == "7" ? "3" : "7") : "" }
    BEGIN { e = ENVIRON["K2_E"]; n = length(e); for (i = 1; !x && i <= n; ) { c = substr(e, i, 1)
        if (index(" \t\n;{}!", c)) i++; else if (c == "/") i = sk(i, c, 1); else if (c == "d") i++
        else if (c == "s" || c == "y") { d = substr(e, i + 1, 1); i = sk(sk(i + 1, d, c == "s") - 1, d, 0)
          if (c == "s") while (i <= n && index("gpI0123456789", substr(e, i, 1))) i++ }
        else x = 1 } }
    FILENAME == ARGV[1] { if (FNR == ENVIRON["K2_M"] + 0) l2 = $0; next }
    { L[++nl] = $0 }
    FNR == ENVIRON["K2_N"] + 0 { a = length($0); b = length(l2); p = 0; s = 0
      while (p < a && p < b && substr($0, p + 1, 1) == substr(l2, p + 1, 1)) p++
      while (s < a - p && s < b - p && substr($0, a - s, 1) == substr(l2, b - s, 1)) s++
      for (i = 1; i <= a; i++) if (i <= p || i > a - s) { j = (i <= p) ? i : i - a + b; q = "\001" pr(substr($0, i, 1))
        for (t = 1; t <= length(q); t++) { r = substr(q, t, 1)
          V[++k] = substr($0, 1, i - 1) r substr($0, i + 1); E[k] = substr(l2, 1, j - 1) r substr(l2, j + 1) } } }
    END { printf "%s %d\n", (x ? "exact" : "inplace"), k > ENVIRON["K2_X"]
      for (t = 1; t <= k; t++) { print V[t] > ENVIRON["K2_X"]; print E[t] > ENVIRON["K2_X"] }
      if (x) { for (t = 1; t <= k; t++) { f = ENVIRON["K2_D"] t; for (r = 1; r <= nl; r++) print (r == ENVIRON["K2_N"] + 0 ? V[t] : L[r]) > f; close(f) } }
      else for (r = 1; r <= nl; r++) { if (r == ENVIRON["K2_N"] + 0) { for (t = 1; t <= k; t++) print V[t] } else print L[r] } }' \
    "$_VFY/.anchor" "$2" > "$_VFY/.fvar" || { _mf_v="the variant pass failed"; return 1; }
  read -r _mf_m _mf_k < "$_VFY/.fvexp" || :
  case "$_mf_m:$_mf_k" in inplace:[0-9]*|exact:[0-9]*) ;; *) _mf_v="the variant pass wrote \"$_mf_m $_mf_k\""; return 1 ;; esac
  if [ "$_mf_m" = exact ]; then
    while [ "$_mf_i" -lt "$_mf_k" ]; do _mf_i=$((_mf_i + 1))
      sed -e "$1" "$_VFY/.fv.$_mf_i" > "$_VFY/.fo.$_mf_i" 2>/dev/null || { _mf_v="the variants' sed failed"; return 1; }
    done
    # Each output, whole, is the target with its variant unchanged (read) or `.anchor` with the image (carried).
    _mf_bad="$(K2_N="${_mf_h%c*}" K2_M="${_mf_h#*c}" K2_O="$2" K2_A="$_VFY/.anchor" K2_X="$_VFY/.fvexp" K2_D="$_VFY/.fo." awk 'BEGIN {
      while ((getline l < ENVIRON["K2_O"]) > 0) o[++no] = l; while ((getline l < ENVIRON["K2_A"]) > 0) an[++na] = l
      getline l < ENVIRON["K2_X"]; while ((getline v < ENVIRON["K2_X"]) > 0 && (getline w < ENVIRON["K2_X"]) > 0) {
        f = ENVIRON["K2_D"] (++k); u = 1; c = 1; m = 0
        while ((getline l < f) > 0) { m++; if (l != (m == ENVIRON["K2_N"] + 0 ? v : o[m])) u = 0; if (l != (m == ENVIRON["K2_M"] + 0 ? w : an[m])) c = 0 }
        close(f); if (m != no) u = 0; if (m != na) c = 0; if (!u && !c) bad++ }
      print bad + 0 }')" || { _mf_v="the variant comparison failed"; return 1; }
  else
    sed -e "$1" "$_VFY/.fvar" > "$_VFY/.fvout" 2>/dev/null || { _mf_v="the variants' sed failed"; return 1; }
    # Each output line from N on is its variant unchanged (read) or its expected image (carried); a line missing is bad.
    _mf_bad="$(K2_N="${_mf_h%c*}" awk 'FILENAME == ARGV[1] { if (FNR == 1) next; if (FNR % 2) e[nv] = $0; else v[++nv] = $0; next }
      FNR >= ENVIRON["K2_N"] + 0 && m < nv { m++; if ($0 != v[m] && $0 != e[m]) bad++ }
      END { print bad + nv - m }' "$_VFY/.fvexp" "$_VFY/.fvout")" || { _mf_v="the variant comparison failed"; return 1; }
  fi
  case "$_mf_bad" in 0) ;; [1-9]*) _mf_v=carry ;; *) _mf_v="the variant comparison printed \"$_mf_bad\""; return 1 ;; esac
}
# THE GATE, `_mut_fresh_gate <expression> <target> <what>`: judges AND reds — `_mut_fresh`'s verdict, its message,
# and the red (`_mut_corr_bad=1`, status 1) on anything but `fresh`. The self-test drives its rows through this
# same function and asserts the red, so a mutant that removes the red step here is killed there (R7e).
_mut_fresh_gate() {
  if ! _mut_fresh "$1" "$2"; then echo "!! $3 \"$1\": $_mf_v, so nothing was decided about it." >&2
  else case "$_mf_v" in
    fresh) return 0 ;;
    stale)
      echo "!! $3 \"$1\" no longer matches ${2##*/}: its anchor is stale," >&2
      echo "   so the record would test a copy identical to the shipped file." >&2 ;;
    count:*)
      echo "!! $3 \"$1\" changes ${_mf_v#count:} lines of ${2##*/}, not exactly one:" >&2
      echo "   it is anchored on more than the line it is about (a comment, a second site), so" >&2
      echo "   it stays \"fresh\" after the intended line is respelled. Anchor it on one line;" >&2
      echo "   a record that needs several edits is several TAB-separated expressions." >&2 ;;
    twice)
      echo "!! $3 \"$1\" changes ${2##*/} again when applied twice:" >&2
      echo "   its anchor occurs more than once on the line (or matches its own output), so a" >&2
      echo "   respelling of the first occurrence moves the edit to the next. Anchor it on text that occurs once." >&2 ;;
    *)
      echo "!! $3 \"$1\" rewrites text of ${2##*/} it does not read:" >&2
      echo "   a character outside its edit, perturbed, is matched by a wildcard and written back as" >&2
      echo "   something else, so a respelling there would be silently reverted. Spell what it keeps," >&2
      echo "   \`\\(…\\)\` and \`\\1\`, and escape a literal \`.\`." >&2 ;;
  esac; fi
  _mut_corr_bad=1; return 1
}
# The always-on half: the records against the labels the run used.
_mut_correspondence() {
  _mut_corr_bad=0
  # ---- THE SIBLING GUARD, ALWAYS ON --------------------------------------------
  # The shipped files beside the wire must be exactly `$_MUT_PARTS`: a part left
  # off the list is not copied beside a mutant, which then exits 2 for a reason
  # that is not its mutation. The test is on the PART NAME (what follows the
  # wire's own stem), never the full path, so a checkout whose path holds
  # `.mutant.` is not mistaken for a copy; a concurrent run's copies
  # (`mutant.<pid>.<part>`) are skipped by that same name. Inside a mutant the
  # stem is `….mutant.<pid>`, and its copies are exactly the parts.
  _mp_have=""
  _mut_part_path '*'
  for _mp_f in "$_mut_pp_pre"*"$_mut_pp_suf"; do
    [ -e "$_mp_f" ] || continue
    _mp_n="${_mp_f#"$_mut_pp_pre"}"; _mp_n="${_mp_n%"$_mut_pp_suf"}"
    case "$_mp_n" in mutant.*) continue ;; esac
    _mp_have="$_mp_have $_mp_n"
  done
  if [ "$(printf '%s\n' $_mp_have | sort | tr '\n' ' ')" != "$(printf '%s\n' $_MUT_PARTS | sort | tr '\n' ' ')" ]; then
    echo "!! the parts beside the wire are [$_mp_have ], but \`_MUT_PARTS\` is [ $_MUT_PARTS ]," >&2
    echo "   so a mutant would run without one of them, or beside one nobody copies." >&2
    _mut_corr_bad=1
  fi
  # ---- THE CORRESPONDENCE, CHECKED ON EVERY RUN ---------------------------------
  # ⚠ THESE TWO USED TO LIVE INSIDE THE MUTATION BLOCK, which is opt-in and costs
  # minutes — so a PR that deleted a record or renamed a control stayed green
  # until somebody happened to run the harness by hand. They are static
  # properties of the shipped files and run no mutant, so they belong where
  # every run sees them. (The mutation RUN stays opt-in; only its bookkeeping
  # moved.)
  _mutants > "$_VFY/.mutants"
  _mut_orphan=0
  # THE NEEDLE, MATERIALISED ONCE: `_mut_parse` stays the one parser of the
  # record format, and this loop writes the `_pr_needle` it sets for every
  # record to `.needles`, one per line.
  # Every other reader (the `!survive` count, the record count, the label
  # ratchet) tests EXACT equality against that file, never a substring of a raw
  # `.mutants` line — a label that is a prefix of a longer needle is not recorded.
  : > "$_VFY/.needles"
  while IFS= read -r _mline; do
    [ -n "$_mline" ] || continue
    _mut_parse "$_mline"
    # ⚠ A LINE THAT IS NOT A RECORD IS AN ERROR HERE, NOT SOMETHING TO SKIP, and
    # the asymmetry it replaces is why: this loop dropped every line with no
    # needle, so a line the here-document turns into DATA passed this check in
    # silence while `_mut_run` applied it as a sed script and reported MATCHED
    # NOTHING. Four such lines were added and shipped that way; what caught them
    # was the opt-in run, minutes long, and what should have is this, which costs
    # nothing. `_mutants` now drops `#` comments at its own materialiser, so what
    # reaches here is a line that is neither — prose with no `#`, or a record
    # whose TAB an editor ate.
    if [ "$_pr_n" -lt 1 ] || [ -z "$_pr_needle" ]; then
      echo "!! mutation set line \"$_mline\" has no TAB-separated needle, so it is not a" >&2
      echo "   record. Write a note as a \`#\` comment; anything else in that" >&2
      echo "   here-document is data, and the mutation run applies it as a sed expression." >&2
      _mut_orphan=1; continue
    fi
    printf '%s\n' "$_pr_needle" >> "$_VFY/.needles"
  done < "$_VFY/.mutants"
  # ---- THE CENSUS: THE LABELS THE RUN USED --------------------------------------
  # Two sources: the run registry (`$_VFY/.registry` — what bash RAN: the run
  # primitives' `_ctl_reg` appends `<file>:<line>	<kind>	<label>` under each
  # `_control` / `_ctl_run` / `_ctl_gate` / `_fgit_window_verdict_exit` call
  # made under this `_VFY`, kind `control` when `_control` made it, `direct`
  # otherwise), and the values of the `_*_lbl` names the TEXT EXPANDS (rule 1,
  # below: the ORPHAN CHECK's population, `.vlabels`; the ratchet and the literal rule read the names
  # the files SPELL instead, `.mvalues`). `.labels` is the registry's labels and the expansions' union;
  # `.ctllabels` (the `!kill` allowlist) is the `control`-kind labels.
  # THE REGISTRY IS BIJECTED WITH THE TEXT (rule 2): the call lines of `$_CONTROLS`
  # (a non-comment line naming a run primitive as a word) are exactly the registered
  # sites, kinds included, so an unexecuted call (an `if false &&`, a branch this
  # machine does not take), a call registered at another line and a call the text hides
  # (`eval`, a split word, a helper defined on another line) are each red, and the
  # registry's sites are the same on every machine (its rows and labels per site too:
  # every call passes a fixed label, measured). One parse reads it (rule 3): a row
  # holds exactly three TAB fields. A `_control` LABEL IS LITERAL: one of that kind equal
  # to a variable's value is red — the variables' producers are `echo` lines, not
  # calls, so a second `_control` under a recorded variable label would otherwise pass
  # under that record. The rule is `_control`'s only: a direct `_ctl_run` / `_ctl_gate`
  # / verdict call may pass a variable label (a residual, named with the same-line one
  # below). The registry's length is noted (`.census`) before it is read: a call
  # after this point would count nowhere, so the controls file's last statement reds
  # if the file grew (`_mut_census_end`, below; a file, so a `( … )` or `$(…)` call
  # grows it too). A control a machine cannot pose is registered all the same (the
  # table's decline, in the run primitives), so an orphan needle is a rename, a
  # deletion or an unreached call on every machine. A run that ends before this point
  # is red without it (the window's exit 2, `_lbd`, a missing part).
  # ONE LABEL, ONE SITE: a label registered from two lines is red — a second
  # producer reusing a recorded label would otherwise pass under the first's
  # record. The line is the finest site bash reports: under rule 2 only calls on ONE
  # physical line are one site; a fold across separate call lines (a `trap` body, a
  # multi-line `eval`) is red, the unreported line having no site (records.sh:382). And a direct block (an `if ! _ctl_run`
  # passing a variable label) under a label whose own producer is an `echo` line is one
  # site, as the literal rule above does not reach it.
  wc -l < "$_VFY/.registry" | tr -d '[:space:]' > "$_VFY/.census"
  # RULE 1: a `_*_lbl` name is used where bash EXPANDS it (`$name`, or `${name` and a non-identifier character), in the
  # wire or a part but the record table (its here-document is quoted: a `$name` there is data). Each line, in this order:
  # (1) a COMMENT, from a `#` at the line's start or after whitespace or one of `;&|()<>` (where bash begins a word), is
  # cut FIRST (`$#`, `${x#y}`, `\#`, `a#b` are none; a quote stripped before the cut would pair across a `#`, a comment's
  # text turned code: a false green); (2) a quoted here-document's body (`<<'W'`, `<<"W"`, `<<\W`) is skipped to its
  # terminator; (3) each `\`+char is dropped, then each single-quoted span (`'$x_lbl'`, `\$x_lbl`, `$'…'` expand nothing).
  # ITS ERRORS GO BOTH WAYS: an UNDER-count (a `#` or `'` inside double quotes, an unquoted here-document body's `#`, a bare arithmetic name
  # `$((x_lbl))`) orphans a record, a false red; an OVER-count (a ` #` in an earlier double-quoted string on a quoted here-document's line, an apostrophe in
  # one before single-quoted data, a multi-line single-quoted string's line — this class not measured by `oracle2.sh`) leaves a record non-orphan, its opt-in trial not killed as named, red (`oracle2.sh`, 395 cases,
  # bash 5.3 and 3.2, the figures stated here, not the logs: 315 agree, 58 under, 18 over, 4 syntax-excluded). So `.vlabels` feeds the ORPHAN CHECK only: the RATCHET (`.bare`) and the literal
  # rule read `.mvalues`, the values of every name `_lbd` asserts defined (`$_lbd_names`, the `_*_lbl` tokens of these files and the record table): spelled ⊇ expanded, so their error is red.
  _cen_files=(); _mut_part_path records
  for _cen_f in "${_lbd_files[@]}"; do case "$_cen_f" in "$_mut_pp") ;; *) _cen_files+=("$_cen_f") ;; esac; done
  _cen_used="$(awk 'FNR == 1 { hd = "" } hd != "" { l = $0; if (hdt) sub(/^\t+/, "", l); if (l == hd) hd = ""; next }
    { s = $0; if (match(s, /(^|[[:space:];&|()<>])#/)) s = substr(s, 1, RSTART - 1)
      if (match(s, /(^|[^<])<<-?[ \t]*(\047|"|\\)[A-Za-z0-9_]+/)) { hd = substr(s, RSTART, RLENGTH); hdt = (hd ~ /<<-/); sub(/^.*[\047"\\]/, "", hd) }
      gsub(/\\./, " ", s); gsub(/\047[^\047]*\047/, " ", s); s = s " "
      while (match(s, /[$][{]?_[A-Za-z0-9_]*_lbl[^A-Za-z0-9_]/)) {
        t = substr(s, RSTART, RLENGTH - 1); sub(/^[$][{]?/, "", t); print t; s = substr(s, RSTART + 1) } }' "${_cen_files[@]}" | LC_ALL=C sort -u)" || {
    echo "!! the label-expansion scan failed: nothing was decided." >&2; _mut_corr_bad=1; }
  for _cen_v in $_cen_used; do printf '%s\n' "${!_cen_v}"; done | LC_ALL=C sort -u > "$_VFY/.vlabels"
  for _cen_v in $_lbd_names; do printf '%s\n' "${!_cen_v}"; done | LC_ALL=C sort -u > "$_VFY/.mvalues"
  # RULES 2 AND 3, ONE PASS over the registry and the controls file: a registry row holds exactly three
  # TAB fields (site, kind, label); a call line of `$_CONTROLS` (a non-comment line naming `_control`,
  # `_ctl_run`, `_ctl_gate` or `_fgit_window_verdict_exit` as a word; kind `control` when it names `_control`,
  # else `direct`) is a registered site, and every registered site is a call line. The same pass writes
  # `.labels` (the `.vlabels` and the registry's labels) and `.ctllabels` (the `control`-kind ones).
  _cen_rc=0
  K2_CEN="$_cen_lbl" K2_CF="${_CONTROLS##*/}" K2_CTL="$_VFY/.ctllabels" K2_LAB="$_VFY/.labels" awk -F'\t' '
    function named(s, w) { return (s ~ ("(^|[^A-Za-z0-9_])" w "([^A-Za-z0-9_]|$)")) }
    BEGIN { cen = ENVIRON["K2_CEN"]; cf = ENVIRON["K2_CF"]; err = "/dev/stderr"
      printf "" > ENVIRON["K2_CTL"]; printf "" > ENVIRON["K2_LAB"]; close(ENVIRON["K2_CTL"]); close(ENVIRON["K2_LAB"])
      lc = "LC_ALL=C sort -u > \"$K2_LAB\""; cc = "LC_ALL=C sort -u > \"$K2_CTL\"" }
    FILENAME == ARGV[1] { print $0 | lc; next }
    FILENAME == ARGV[2] { isv[$0] = 1; next }
    FILENAME == ARGV[3] {
      if (NF != 3) { print "!! CONTROL FAILED (" cen "): the registry row \"" $0 "\" holds " NF " fields, not 3 (site, kind, label)." > err; bad = 1; next }
      l = $3; k = $1 "\t" $2
      if (!((l, $1) in seen)) { seen[l, $1] = 1; sites[l] = sites[l] " " $1; n[l]++ }
      if (!(k in rs)) { rs[k] = 1; ro[++nr] = k }
      print l | lc
      if ($2 == "control") { print l | cc
        if ((l in isv) && !(l in lit)) { lit[l] = 1
          print "!! CONTROL FAILED (" cen "): \"" l "\" is a `_*_lbl` value passed to `_control` (" $1 "):" > err
          print "   a `_control` label is literal — a second producer under a variable label would pass under its record." > err
          bad = 1 } }
      next }
    { if ($0 ~ /^[[:space:]]*#/) next
      c = named($0, "_control")
      if (!(c || named($0, "_ctl_run") || named($0, "_ctl_gate") || named($0, "_fgit_window_verdict_exit"))) next
      kd = c ? "control" : "direct"; k = cf ":" FNR "\t" kd; cl[k] = 1
      if (!(k in rs)) xc = xc " " cf ":" FNR " (" kd ")" }
    END { for (l in n) if (n[l] > 1) {
        print "!! CONTROL FAILED (" cen "): \"" l "\" was registered from more than one call site:" sites[l] > err
        print "   — one label, one call site: a second producer under a recorded label would pass under that record." > err
        bad = 1 }
      for (i = 1; i <= nr; i++) if (!(ro[i] in cl)) { split(ro[i], a, "\t"); xr = xr " " a[1] " (" a[2] ")" }
      if (xc != "") { print "!! CONTROL FAILED (" cen "): call lines of the controls file with no registered site:" xc > err
        print "   — the call never ran (a branch not taken, an `if false &&`), or it registered at another line." > err; bad = 1 }
      if (xr != "") { print "!! CONTROL FAILED (" cen "): registered sites that are no call line of the controls file:" xr > err
        print "   — a call the text hides (`eval`, a split word, a helper defined elsewhere), or a line the file no longer has." > err; bad = 1 }
      close(lc); close(cc); exit bad }' "$_VFY/.vlabels" "$_VFY/.mvalues" "$_VFY/.registry" "$_CONTROLS" || _cen_rc=$?
  [ "$_cen_rc" -le 1 ] || echo "!! the registry scan failed (awk exited $_cen_rc): nothing was decided." >&2
  [ "$_cen_rc" -eq 0 ] || _mut_corr_bad=1
  # READER 4 OF THE CAPABILITY TABLE: every declared member is a `control`-kind label of THIS run's
  # registry (the list just written), else the table names a control that was renamed or never made.
  _mc_nl=$'\n'; _mc_ctl="$_mc_nl$(cat "$_VFY/.ctllabels")$_mc_nl"
  _cap_rows member _mut_cap_member
  # `.bare`: the registry's labels and `.mvalues` (the spelled names) less the needles (the orphan check reads `.labels`).
  _mut_arc=0
  K2_CEN="$_cen_lbl" K2_BARE="$_VFY/.bare" awk 'BEGIN { printf "" > ENVIRON["K2_BARE"] }
    FILENAME == ARGV[1] { nd[++nn] = $0; isn[$0] = 1; next }
    { if (FILENAME == ARGV[3]) isl[$0] = 1; if (!($0 in isb)) { isb[$0] = 1; lb[++nl] = $0 } }
    END {
      for (i = 1; i <= nn; i++) if (nd[i] != "!survive" && !(nd[i] in isl)) {
        print "!! CONTROL FAILED (" ENVIRON["K2_CEN"] "): mutation record needle \"" nd[i] "\" is no label this run used (none registered" > "/dev/stderr"
        print "   by a call, no `_*_lbl` value): its control was renamed or deleted, or the needle is some other string." > "/dev/stderr"
        bad = 1
      }
      for (i = 1; i <= nl; i++) if (!(lb[i] in isn)) print lb[i] > ENVIRON["K2_BARE"]
      exit bad
    }' "$_VFY/.needles" "$_VFY/.mvalues" "$_VFY/.labels" || _mut_arc=$?
  [ "$_mut_arc" -le 1 ] || echo "!! the needle/label scan failed (awk exited $_mut_arc): nothing was decided." >&2
  [ "$_mut_arc" -eq 0 ] || _mut_orphan=1
  [ "$_mut_orphan" -eq 0 ] || _mut_corr_bad=1
  # ---- THE GENERATED SET IS NOT EMPTY, ALWAYS ON --------------------------------
  # A static property of `_mut_regex_mutants`' output over the running wire's
  # `$K2RE` and `$K2RE_PATH`, so it is checked where every run (and every
  # record's trial) sees it -- never in the opt-in run alone, which an empty
  # generated set would otherwise pass silently (`0 mutant(s)`, `0 neither
  # killed nor argued equivalent`).
  _mut_gen_floor || _mut_corr_bad=1
  # ---- EVERY RECORD STILL APPLIES, ALWAYS ON ------------------------------------
  # Each record's expression must be FRESH on the file it targets (`_mut_fresh`:
  # one line, position-unique, faithful). Records anchor on harness and fixtures
  # text that an equivalent edit can change (spelling an option long, adding an
  # allowlist entry), and a record that matches nothing, or matches a second
  # occurrence, or rewrites text it does not read, tests a copy that is not the
  # one it was written for. The opt-in run would catch it, minutes later and only
  # when someone runs it; this costs seven processes per line-local expression.
  # THE RULE'S OWN SELF-TEST FIRST, on a fixed fixture whose verdicts are known,
  # through the gate the records use: a wrong verdict, or a red (`_mut_corr_bad`,
  # status, message) present on `fresh` or missing on another row, is red under
  # `$_af_lbl`, whose records (one per verdict arm, the gate's red step, the exact path and the two probes) pin
  # each, and a variant pass this machine's awk or sed made vacuous (every record
  # read fresh) reds the carry row. ⚠ ABOVE THE SKIP BELOW, so a record that
  # disables an arm is exercised inside its own mutant.
  printf '%s\n' 'a=1' 'x ab ab' 'k=vx' > "$_VFY/.affix"
  # The last three rows pin the variant pass's mechanisms (R7f, R7g, R7h): the range ends at the first variant laid in
  # place, so only the exact path reds it; the `[a-z]` read is caught only by the same-class probe, the `.` one only by \001.
  for _af_r in 's/^a=1$/a=2/ fresh' 's/^zz$/y/ stale' 's/=/:/ count:2' 's/ab/XY/ twice' 's/^k=.*/k=wx/ carry' \
    '/^x/,/=/s/^k=.*/k=wx/ carry' 's/^k=v[a-z]$/k=wx/ carry' 's/^x.ab ab$/x ab XY/ carry'; do
    _af_keep="$_mut_corr_bad"; _mut_corr_bad=0; _af_rc=0; _af_g=0; [ "${_af_r##* }" = fresh ] || _af_g=1
    _mut_fresh_gate "${_af_r% *}" "$_VFY/.affix" "anchor self-test row" 2> "$_VFY/.afmsg" || _af_rc=1
    _af_m=0; if [ -s "$_VFY/.afmsg" ]; then _af_m=1; fi; _af_red="$_mut_corr_bad:$_af_rc:$_af_m"; _mut_corr_bad="$_af_keep"
    [ "$_mf_v" = "${_af_r##* }" ] && [ "$_af_red" = "$_af_g:$_af_g:$_af_g" ] && continue
    echo "!! CONTROL FAILED ($_af_lbl): the anchor gate reads \"${_af_r% *}\" on its fixture as \"$_mf_v\" (red:status:message $_af_red), not \"${_af_r##* }\" ($_af_g:$_af_g:$_af_g)." >&2
    _mut_corr_bad=1
  done
  # ⚠ THE RECORDS NOT INSIDE A MUTANT: there one target is edited ON PURPOSE, so
  # the records anchored on the edited text no longer apply, by design.
  case "${SELF##*/}" in
    *.mutant.*) : ;;
    *) while IFS= read -r _ma_rec; do
         [ -n "$_ma_rec" ] || continue
         _mut_parse "$_ma_rec"
         _mut_src_for "$_pr_which"
         _ma_i=0
         while [ "$_ma_i" -lt "$_pr_n" ]; do
           _ma_x="${_pr_expr[$_ma_i]}"; _ma_i=$((_ma_i + 1))
           # An empty word (`:a`, a sed label) and one letter (`s:a:b:`) are not prefixes; the red below is LATENT (see "UNLISTED" in `…trip-wire.mutations.sh`).
           case "$_ma_x" in *:*) _ma_w="${_ma_x%%:*}"; case "$_ma_w" in ""|?|*[!A-Za-z]*) ;; *)
             echo "!! mutation record expression \"$_ma_x\": \"$_ma_w:\" is no prefix (one is read once, from a record's FIRST expression); sed would read it as a command (\`w\` writes, \`e\` runs)." >&2
             _mut_corr_bad=1; continue ;; esac ;; esac
           # ⚠ EACH EXPRESSION ALONE, AND IT MUST BE FRESH on its target (one
           # changed line, applied twice = once, faithful outside the changed span).
           # Whole-record, whole-file was the old granularity: an expression that
           # also matched a COMMENT passed on the comment after the code line was
           # respelled, and the second half of a `range-s;s` pair kept a record
           # "fresh" when the range half had gone stale. `diff` counts a changed
           # line once whether `s` rewrote it or `d` removed it.
           _mut_fresh_gate "$_ma_x" "$_mut_src" "mutation record expression" || :
         done
       done < "$_VFY/.mutants" ;;
  esac
  command rm -f "$_VFY/.anchor" "$_VFY/.anchordiff" "$_VFY/.anchor2" "$_VFY/.fvar" "$_VFY/.fvexp" "$_VFY/.fvout" "$_VFY/.affix" "$_VFY/.afmsg" "$_VFY"/.fv.* "$_VFY"/.fo.*
  # …the standing negative control must BE there, and the set may not shrink. ⚠ EACH TEST BELOW IS `! [ … ]`: a
  # non-integer operand (an empty count, a mistyped floor or ratchet) makes `[` exit 2; errexit is off in this body (it runs
  # as `_mut_correspondence || ctl_ok=1`; records.sh says so), so a bare `[` is read as false and the test silently passes;
  # `! [ … ]` makes that 2 a red (as at `_control`'s exit check). ⚠ `awk`, NOT `grep -c`: it exits 1 on no match (the `wc -l` trap below).
  _mut_surv="$(awk '$0=="!survive"{n++} END{print n+0}' "$_VFY/.needles")"
  if ! [ "$_mut_surv" -eq 1 ]; then
    echo "!! the standing negative control (\`!survive\`) is not in the mutation set" >&2
    echo "   exactly once (found $_mut_surv), so a broken harness cannot be told" >&2
    echo "   from a real kill." >&2
    _mut_corr_bad=1
  fi
  _mut_recs="$(awk 'END{print NR+0}' "$_VFY/.needles")"
  if ! [ "$_mut_recs" -ge "$_MUT_RECORDS_MIN" ]; then
    echo "!! the mutation set holds $_mut_recs records, against a floor of $_MUT_RECORDS_MIN." >&2
    echo "   A record was deleted. If that is deliberate, say why and LOWER the floor" >&2
    echo "   in the same edit — so a smaller set is a visible decision." >&2
    _mut_corr_bad=1
  fi
  # …and the direction that actually finds things: labels with NO record (`.bare`, above).
  # ⚠ `wc -l`, NOT `grep -c .`. `grep` exits 1 when no line is selected, so on an
  # EMPTY `.bare` — the state this ratchet exists to let you reach — the
  # assignment failed and `set -e` aborted the required gate before the
  # comparison ran. The success case was the one that broke it.
  _mut_bare="$(wc -l < "$_VFY/.bare" | tr -d '[:space:]')"
  if ! [ "$_mut_bare" -le "$_MUT_UNRECORDED_MAX" ]; then
    echo "!! CONTROL FAILED ($_cen_lbl): $_mut_bare labels have no mutation record, against a ratchet of $_MUT_UNRECORDED_MAX." >&2
    echo "   Either the new label needs a record, or a record was deleted. The bare ones:" >&2
    sed 's/^/     /' "$_VFY/.bare" >&2
    echo "   If a label genuinely cannot have one, say why and RAISE the ratchet in the" >&2
    echo "   same edit — so widening the gap is a visible decision rather than a silence." >&2
    _mut_corr_bad=1
  elif ! [ "$_mut_bare" -ge "$_MUT_UNRECORDED_MAX" ]; then
    echo "!! CONTROL FAILED ($_cen_lbl): only $_mut_bare labels have no mutation record, below the ratchet of $_MUT_UNRECORDED_MAX:" >&2
    echo "   a label gained a record, or went away. LOWER the ratchet to $_mut_bare in the" >&2
    echo "   same edit — a slack ratchet lets the next label go bare in silence." >&2
    _mut_corr_bad=1
  fi
  command rm -f "$_VFY/.bare" "$_VFY/.labels"
  return "$_mut_corr_bad"

}
# THE REGISTRY MAY NOT GROW AFTER THE CENSUS. Asked by the controls file as its
# last statement (the wire makes no label-taking call after that file returns):
# the registry's length now against the note the census left. A late call
# counted nowhere, whichever shell made it (a `( … )`, a `$(…)`: the file is
# shared), so it is red under the census's label, listing the late lines; no
# note, or no count, is red too.
_mut_census_end() {
  local _cend_n _cend_now
  _cend_n="$(cat "$_VFY/.census" 2>/dev/null)" || _cend_n=""
  _cend_now="$(wc -l < "$_VFY/.registry" 2>/dev/null | tr -d '[:space:]')" || _cend_now=""
  if [ -n "$_cend_n" ] && [ -n "$_cend_now" ] && [ "$_cend_now" -eq "$_cend_n" ]; then return 0; fi
  echo "!! CONTROL FAILED ($_cen_lbl): the run registry grew after the label census (${_cend_n:-no note} → ${_cend_now:-no count}); the late calls:" >&2
  tail -n "+$((${_cend_n:-0} + 1))" "$_VFY/.registry" 2>/dev/null | sed 's/^/     /' >&2
  return 1
}
