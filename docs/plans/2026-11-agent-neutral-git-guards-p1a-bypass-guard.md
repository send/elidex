# Plan — P1a cut-over: the bypass-guard patch (global)

## §0 Status

**Where this belongs (PM 916)**: this file is §4.8 of the P1a memo,
`docs/plans/2026-11-agent-neutral-git-guards-p1a.md`, moved here verbatim so that memo stays under 1000 lines. It
is in neither PR's diff: the PM applies it at the P1a cut-over, and the P1a memo's cut-over (its §7.1 step A2, with
E24 of its §7.5) owns applying it, by reference to this file. Every section and id named below without a file
(§3.2, §7.1, I1, I4, …) is the P1a memo's; this file defines none of its own.
Since the move it has two revisions: PM 922 added the `git_segments()` iterator ("One derivation" below), and PM 930
its marks and `pass_through_defers()` ("Defer only on derivable shapes" below).

### §4.8 The bypass-guard patch (global; the PM applies it at §7.1 A2) [P1a cut-over]

PM 900: the user approved extending `~/.claude/hooks/git-hook-bypass-guard.py`; the PM's A2-scope decision sets the
scope to the whole write grammar of `core.hooksPath` — the legacy options and the `config set` / `unset`
subcommands, `--add` / `--replace-all` / `--file`, a case-insensitive key, and `GIT_CONFIG_PARAMETERS`. The patch is
written here, against the file as read on 2026-11-21 (`shasum -a 256` =
`9902fe77ac1ebbf9912315265b4f93e24b70a7603d91d5945bcca7469e1a0438`; if the file differs when the PM applies it, the
patch is re-derived, not forced). It follows the file's own structure: one more case in `check_git` (by
subcommand), the env rule widened inside `env_hooks_assign`, the same `in_scope` (no copy of its roots), the same
fail-open on an internal error, and tests in the existing `git-hook-bypass-guard.test.py` lists.

**One derivation of each git segment (PM 922).** The patch also moves the walk now inside `scan()` — the shell
segments, the `cd` tracking, `git -C` (through `check_git`), the env assignments and the `bash -c` / `sh -c` /
`eval` recursion — into one iterator, `git_segments(cmd, cwd)`, which yields `(git_args, effective_cwd, violation)`
for every git segment, nested ones included. `scan()` becomes the first in-scope violation of that iterator
(behaviour unchanged: every existing test passes on it), and the P1a memo §7.1 pass-through iterates the same
function, so the cwd it judges is the one `scan()` judges, by construction. The third field is the violation
`check_git` (or an env assignment) found; the pass-through ignores it.

**Defer only on derivable shapes (PM 930).** Deferring when every *yielded* segment is installed skipped the Claude
hooks for git calls the iterator never yields or whose cwd it misreads: from an installed elidex worktree,
`git status && env -C <another repository> git push origin HEAD:main` yields only `status`, and 13 of the 22 forms of
the PM 930 probe deferred for a push that runs outside elidex. So `git_segments` takes an optional `marks` list and
adds a reason to it at every shape whose cwd it does not derive positively, and the predicate is one function in this
file, `pass_through_defers(cmd, cwd, installed)`: true only when no mark is set, at least one git segment is yielded,
and every yielded cwd is in scope and `installed` (the caller passes `installed`: the §4.7 form of the P1a memo
§7.1). The rule: **defer only on shapes the walker derives positively; an unknown shape does not defer.** A command
not deferred runs the Claude hook as today, and the double guard is harmless. The marks, one line of
`git_segments` each:
- an operator holding `(` or `)` (a subshell, whose `cd` does not leak; an unquoted `$( … )`; a function; `case`);
- a token holding `` ` `` or `$(` (command substitution inside quotes);
- an assignment or a wrapper (`env`, `command`, `sudo`, `exec`, `time`, `nohup`, `nice`, `builtin`) before the
  command (`GIT_DIR=…`, `GIT_WORK_TREE=…`, `env -C …`, `command -p …`);
- a command that is a reserved word (`if`, `then`, `{`, `}`, `while`, …) or a builtin that changes the shell's state
  (`pushd`, `popd`, `export`, `unset`, `source`, `eval`, …);
- a `git` token (by basename) in an argument position (`xargs … git`, `find … -exec git`);
- a `cd` that is not `cd <dir>` with one literal target (no `$`, no `` ` ``, no unresolved `~`) run whatever the
  previous command did and reaching the next one: `cd -`, a bare `cd`, `cd -P …`, a `cd` after `&&` / `||` / `|` /
  `&`, or before `||` / `|` / `&`;
- a git option before the subcommand other than `-C <literal dir>`, `--no-pager`, `-p`, `-P`, `--paginate`,
  `--no-optional-locks`, `--no-advice` (so `--git-dir`, `--work-tree`, `-c`, `--config-env`), or no subcommand;
- a shell that is not `<sh> -c '…'` (`bash -lc`, `bash -x -c`, `bash script.sh`);
- nesting deeper than 3.

A relative `cd <dir>` is derived as `<cwd>/<dir>`, which assumes an empty `CDPATH` (it is empty in the session shell,
2026-11-22). The marks are a second channel: the yields and the cwd tracking are unchanged, so `scan()` is too (a
differential run of the PM 922 iterator and this one over 568 (command, cwd) pairs — the test lists, the 22 probe
forms, the attestation's five `git config` forms and three `cd` / `eval` forms, from four cwds — found 0
differences in `scan()` and in the yielded cwds).
**`scan()`'s deny is not widened (PM 930).** Several of the marked shapes are its blind spots — it misreads the cwd
or does not see the git call — and its docstring lists them: a `cd` inside `( … )` read as leaking, `cd -`,
`pushd` / `popd`, a `cd` target with `$`, a command starting with a reserved word, `bash -lc`, git after a wrapper's
options, git in an argument position, `GIT_DIR=` / `GIT_WORK_TREE=` and `--git-dir` / `--work-tree`.

**No silent overlap with the existing rules.** `-c core.hooksPath=…` (and `-c core.hooksPath …`,
`-ccore.hooksPath=…`), `--config-env=core.hooksPath=…` and a `GIT_CONFIG_KEY_<n>=core.hooksPath` assignment are
already denied — they are git's global options and environment, read before the subcommand. The new rule reads only
what follows the `config` subcommand, plus one more variable name in the env rule; a command carrying both kinds is
denied once, with the first hit's name.

**Scope of the new rule.** A write is denied when the git segment's effective cwd is in scope (as every rule of the
file), or when `--file` names a path in scope (an elidex `.git/config` written from outside). `--global` /
`--system` from outside the scope stay allowed: the shared repository's local `core.hooksPath` (I1) outranks them.
The docstring names what the rule cannot see (indirect `include.path`, a swapped `GIT_CONFIG_GLOBAL` / `HOME`, a
direct file write, `--git-dir` from outside); the persistent ones are detected by `--check` (I4).

```diff
--- a/git-hook-bypass-guard.py
+++ b/git-hook-bypass-guard.py
@@ -6,13 +6,29 @@
     `--config-env=core.hooksPath=...` (キー名は大文字小文字を区別しない。`git -C dir` 付きも)
   - commit / push / merge / pull / rebase / cherry-pick / am / revert の `--no-verify` (--no-ve... の省略形も)、
     `git commit -n` (-an / -nm のような束ね形も)
+  - `git config` で core.hooksPath を書く形すべて: 旧形式 (`git config [--local|--worktree|--global|--system|
+    -f <file>|--file[=]<file>] core.hooksPath <dir>`、option は位置引数の後ろでもよい)、`--add` / `--replace-all` /
+    `--unset` / `--unset-all`、subcommand `set` / `unset`、section ごとの `--remove-section` / `--rename-section` /
+    `remove-section` / `rename-section` (section `core`)、editor で開く `-e` / `--edit` / `edit` (どの key も書ける)。
+    読む形 (`--get` 系、`-l` / `--list`、`get` / `list`、key だけの旧形式 `git config core.hooksPath`) は通す。
+    `--file` の先が elidex の中なら、cwd が外でも deny
 対象は実効 cwd (hook JSON の cwd、先行する `cd`、`git -C <dir>`) が elidex の checkout / worktree の中にある時だけ。
 範囲の考え方は ecs-mission-guard.py の in_scope と同じ (SCOPE_ROOTS / SCOPE_PARENT)。外では何も出さず exit 0。
 
 shell の字句は shlex で quote を考慮して分け、`&& ; | ( )` と改行で区切った各 command を見る。
 `bash -c '...'` / `sh -c` / `eval` の引数は再帰して見る。引数の文字列の中身 (grep の pattern、commit message) は見ない。
-`GIT_CONFIG_KEY_<n>=core.hooksPath` の env 代入も deny (command 先頭の `VAR=x`、`env VAR=x`、同じ command 内で先行する `export VAR=x`)。
-見えない所 (blind spot): `$(...)` / バッククォート内、変数展開で組み立てた git、git alias。
+`GIT_CONFIG_KEY_<n>=core.hooksPath` と、値に core.hooksPath を含む `GIT_CONFIG_PARAMETERS=...` の env 代入も deny
+(command 先頭の `VAR=x`、`env VAR=x`、同じ command 内で先行する `export VAR=x`)。
+見えない所 (blind spot): `$(...)` / バッククォート内、変数展開で組み立てた git、git alias。core.hooksPath を間接に
+変える形 (`include.path` / `includeIf.*.path` の書き込み、`-c include.path=...`、`GIT_CONFIG_GLOBAL` / `HOME` を
+差し替えた file)、`.git/config` / `config.worktree` への直接の file 書き込み、cwd の外から `--git-dir` で elidex を
+指す形。永続する迂回は `mise run install-githooks -- --check` (elidex の P1 plan memo の installed(wt)) が見つける。
+実効 cwd の導出にも見えない所がある (deny はここを広げない。git を見落とすか cwd を読み違える): `( ... )` の中の
+`cd` を外へ漏れると読む、`cd -` / `pushd` / `popd` / `$` を含む `cd` の先、`if` / `{` など予約語で始まる command、`bash -lc`
+(`-c` だけを認める)、`env -C` / `command -p` / `sudo ...` など wrapper の option の後ろの git、`xargs git` のように
+引数の位置にある git、`GIT_DIR=` / `GIT_WORK_TREE=` と `--git-dir` / `--work-tree`。
+pass_through_defers() (elidex の push-branch / fmt hook の pass-through) は逆向きに倒す: cwd を導出できると認めた
+形だけで defer し、それ以外 (上の形、認めない形すべて) は defer しない。
 
 Fails OPEN: どんな内部エラーでも no output, exit 0。
 Env overrides (tests): GIT_BYPASS_GUARD_STATE_DIR, GIT_BYPASS_GUARD_EXTRA_SCOPE (os.pathsep 区切り)。
@@ -29,12 +45,28 @@
 HOOKED_SUBCMDS = {"commit", "push", "merge", "pull", "rebase", "cherry-pick", "am", "revert"}
 WRAPPERS = {"env", "command", "exec", "time", "nohup", "nice", "sudo", "builtin"}
 SHELLS = {"bash", "sh", "zsh", "dash", "ksh"}
+# pass-through: cwd を導出できると認めない command (予約語、shell の状態を変える builtin)
+RESERVED = {"if", "then", "else", "elif", "fi", "case", "esac", "for", "select", "while", "until", "do", "done",
+            "in", "function", "time", "coproc", "{", "}", "!", "[[", "]]"}
+SHELL_STATE = {"pushd", "popd", "export", "unset", "source", ".", "alias", "set", "shopt", "declare", "typeset",
+               "local", "readonly", "trap", "hash", "enable", "eval"}
+# pass-through: subcommand より前に置いてよい git の option (どの repository で走るかを変えないもの)
+GIT_PLAIN_OPTS = {"--no-pager", "-P", "-p", "--paginate", "--no-optional-locks", "--no-advice"}
 # git global options that take a separate value (besides -C / -c / --config-env handled below)
 GLOBAL_VALUE_OPTS = {"--git-dir", "--work-tree", "--namespace", "--exec-path", "--super-prefix", "--attr-source"}
 # commit short options that consume a value (rest of the cluster / next token is the value)
 COMMIT_VALUE_SHORT = set("mFCctSu")
 COMMIT_VALUE_LONG = {"--message", "--file", "--reuse-message", "--reedit-message", "--author", "--date",
                      "--template", "--fixup", "--squash", "--cleanup", "--trailer", "--gpg-sign"}
+# git config (git-config(1) COMMANDS / DEPRECATED MODES / OPTIONS, git 2.56.1)
+CONFIG_SUBCMDS = {"list", "get", "set", "unset", "rename-section", "remove-section", "edit"}
+CONFIG_READ_SUBCMDS = {"list", "get"}
+CONFIG_READ_ACTIONS = {"--get", "--get-all", "--get-regexp", "--get-urlmatch", "--get-color", "--get-colorbool",
+                       "-l", "--list"}
+CONFIG_WRITE_ACTIONS = {"--add", "--replace-all", "--unset", "--unset-all", "--rename-section", "--remove-section",
+                        "-e", "--edit"}
+# long options that take a separate value (`--opt=value` is one token); `-f` is handled with the short clusters
+CONFIG_VALUE_LONG = {"--file", "--blob", "--type", "--default", "--comment", "--value", "--url"}
 
 
 def state_dir():
@@ -70,34 +102,70 @@
 SEPS = {";", "&", "&&", "|", "||", "|&", ";;", "(", ")", "\n"}
 
 
-def segments(cmd):
-    """shlex で分け、制御演算子と改行で区切った token 列のリストを返す。"""
+def split(cmd):
+    """shlex で分け、制御演算子と改行で区切る -> ([(前の演算子, token 列, 後ろの演算子)], 演算子すべて)。端は ""。"""
     lex = shlex.shlex(cmd, posix=True, punctuation_chars=";&|()\n")
     lex.whitespace = " \t\r"
     lex.whitespace_split = False
     lex.commenters = ""
-    segs, cur = [], []
+    parts, ops, cur, before = [], [], [], ""
     for tok in lex:
         if tok and all(c in ";&|()\n" for c in tok):  # operator run, quoted ones are never all-punct unquoted
+            ops.append(tok)
             if cur:
-                segs.append(cur)
-            cur = []
+                parts.append((before, cur, tok))
+            cur, before = [], tok
         else:
             cur.append(tok)
     if cur:
-        segs.append(cur)
-    return segs
+        parts.append((before, cur, ""))
+    return parts, ops
 
 
+def segments(cmd):
+    """制御演算子と改行で区切った token 列のリスト。"""
+    return [seg for _b, seg, _a in split(cmd)[0]]
+
+
+def seq_op(op):
+    """端、または `;` と改行だけの演算子 (前の command の成否によらず次が走る区切り)。"""
+    return op == "" or (set(op) <= {";", "\n"} and ";;" not in op)
+
+
+def opaque_path(d):
+    """cd / -C の先を字面から導出できない (展開が要る) なら True。"""
+    return "$" in d or "`" in d or os.path.expanduser(d).startswith("~")
+
+
+def git_opts_derivable(args):
+    """git の後ろの token: subcommand の前が `-C <dir>` と GIT_PLAIN_OPTS だけで、subcommand があれば True。"""
+    i = 0
+    while i < len(args) and args[i].startswith("-"):
+        if args[i] == "-C" and i + 1 < len(args) and not opaque_path(args[i + 1]):
+            i += 2
+        elif args[i] in GIT_PLAIN_OPTS:
+            i += 1
+        else:
+            return False
+    return i < len(args)
+
+
 def is_assign(tok):
     k, eq, _ = tok.partition("=")
     return bool(eq) and k.replace("_", "a").isalnum() and not k[0].isdigit()
 
 
 def env_hooks_assign(tok):
+    """env 代入 1 つ -> 拾った形の名前、または ""。"""
     k, eq, v = tok.partition("=")
-    return bool(eq) and k.upper().startswith("GIT_CONFIG_KEY_") and k[len("GIT_CONFIG_KEY_"):].isdigit() \
-        and v.strip().lower() == "core.hookspath"
+    if not eq:
+        return ""
+    if k.upper().startswith("GIT_CONFIG_KEY_") and k[len("GIT_CONFIG_KEY_"):].isdigit() \
+            and v.strip().lower() == "core.hookspath":
+        return "GIT_CONFIG_KEY_n=core.hooksPath"
+    if k.upper() == "GIT_CONFIG_PARAMETERS" and "core.hookspath" in v.lower():
+        return "GIT_CONFIG_PARAMETERS=core.hooksPath"
+    return ""
 
 
 def hookspath_key(s):
@@ -140,6 +208,58 @@
     return False
 
 
+def config_hookspath_write(args, cwd):
+    """`git config` の後ろの token -> (violation or None, `--file` の先 or None)。
+    位置引数は option とその値を除いた全部 (git は option を位置引数の後ろにも置ける)。未知の option が値を取って
+    位置がずれても、core.hooksPath が位置引数のどこかにあり読む形でなければ書く形として拾う (未知は deny 側)。"""
+    pos, acts, target, i = [], set(), None, 0
+    while i < len(args):
+        a = args[i]
+        if a == "--":
+            pos.extend(args[i + 1:])
+            break
+        if a in CONFIG_VALUE_LONG and i + 1 < len(args):
+            if a == "--file":
+                target = args[i + 1]
+            i += 2
+            continue
+        if a.startswith("--file="):
+            target = a[len("--file="):]
+        elif a.startswith("--"):
+            acts.add(a.split("=", 1)[0])
+        elif a.startswith("-") and len(a) > 1:
+            for k, ch in enumerate(a[1:], 1):
+                if ch == "f":
+                    if k < len(a) - 1:
+                        target = a[k + 1:]
+                    elif i + 1 < len(args):
+                        target = args[i + 1]
+                        i += 1
+                    break
+                acts.add("-" + ch)
+        else:
+            pos.append(a)
+        i += 1
+    if target is not None:
+        target = os.path.join(cwd, os.path.expanduser(target))
+    sub = pos[0] if pos and pos[0] in CONFIG_SUBCMDS else None
+    names = pos[1:] if sub else pos
+    if sub == "edit" or acts & {"-e", "--edit"}:
+        return "git config --edit", target
+    if sub in ("rename-section", "remove-section") or acts & {"--rename-section", "--remove-section"}:
+        if names and names[0].lower() == "core":
+            return "git config remove/rename-section core", target
+        return None, target
+    if not any(n.lower() == "core.hookspath" for n in names):
+        return None, target
+    reads = sub in CONFIG_READ_SUBCMDS or bool(acts & CONFIG_READ_ACTIONS)
+    if reads and not acts & CONFIG_WRITE_ACTIONS:
+        return None, target
+    if sub is None and len(names) == 1 and not acts & CONFIG_WRITE_ACTIONS:
+        return None, target  # `git config core.hooksPath` = 旧形式の読み
+    return "git config core.hooksPath", target
+
+
 def check_git(args, cwd):
     """args: git の後ろの token。-> (violation or None, effective_cwd)"""
     i, hits = 0, []
@@ -173,6 +293,12 @@
             break
     sub = args[i] if i < len(args) else None
     rest = args[i + 1:] if sub else []
+    if sub == "config":
+        v, target = config_hookspath_write(rest, eff)
+        if v:
+            hits.append(v)
+            if target and not in_scope(eff) and in_scope(target):
+                eff = target
     if sub in HOOKED_SUBCMDS:
         skip = commit_value_skips(rest) if sub == "commit" else set()
         for k, r in enumerate(rest):
@@ -186,44 +312,83 @@
     return (hits[0] if hits else None), eff
 
 
-def scan(cmd, cwd, depth=0):
-    """-> (violation, effective_cwd_of_the_violating_git) or (None, None)。cd を追跡する。"""
+def git_segments(cmd, cwd, depth=0, marks=None):
+    """cmd の git の呼び出しごとに (git の後ろの token, 実効 cwd, 違反 or "") を yield する。
+    実効 cwd の導出 (hook JSON の cwd、先行する `cd`、`git -C <dir>`) と `bash -c` / `sh -c` / `eval` の再帰は
+    ここにだけある。scan() と、elidex の push-branch / fmt hook の pass-through の両方がこれを使う。
+    marks (list) を渡すと、実効 cwd を導出できると認めない形を見るたびに理由を 1 つ足す。yield と cwd の追跡は
+    marks によらず同じ (scan() の判定は変わらない)。"""
+    mark = marks.append if marks is not None else (lambda _why: None)
     if depth > 3:
-        return None, None
-    exported = False  # `export GIT_CONFIG_KEY_n=core.hooksPath` seen earlier in this command
-    for seg in segments(cmd):
+        mark("nested deeper than 3")
+        return
+    parts, ops = split(cmd)
+    if any(c in op for op in ops for c in "()"):
+        mark("( ): subshell, $( ), function or case")
+    exported = ""  # `export GIT_CONFIG_KEY_n=core.hooksPath` / `GIT_CONFIG_PARAMETERS=...` seen earlier in this command
+    for before, seg, after in parts:
+        if any("`" in t or "$(" in t for t in seg):
+            mark("command substitution")
         j = 0
-        seg_env = any(env_hooks_assign(t) for t in seg[:next((n for n, t in enumerate(seg) if not (is_assign(t) or os.path.basename(t) in WRAPPERS)), len(seg))])
+        seg_env = next(filter(None, map(env_hooks_assign, seg[:next((n for n, t in enumerate(seg) if not (is_assign(t) or os.path.basename(t) in WRAPPERS)), len(seg))])), "")
         while j < len(seg) and (is_assign(seg[j]) or os.path.basename(seg[j]) in WRAPPERS):
             j += 1
+        if j:
+            mark("assignment or wrapper before the command")
         if j >= len(seg):
             continue
         prog, rest = os.path.basename(seg[j]), seg[j + 1:]
+        if prog in RESERVED or prog in SHELL_STATE:
+            mark("reserved word or shell builtin: " + prog)
+        if prog != "git" and any(os.path.basename(t) == "git" for t in rest):
+            mark("git in an argument position")
+        if prog == "cd" and not (len(rest) == 1 and not rest[0].startswith("-") and not opaque_path(rest[0])
+                                 and seq_op(before) and (seq_op(after) or after == "&&")):
+            mark("cd whose target or reach is not derivable")
         if prog == "cd" and rest:
             d = rest[0] if not rest[0].startswith("-") else (rest[1] if len(rest) > 1 else "")
             if d:
                 cwd = os.path.join(cwd, os.path.expanduser(d))
         elif prog == "export":
-            exported = exported or any(env_hooks_assign(t) for t in rest)
+            exported = exported or next(filter(None, map(env_hooks_assign, rest)), "")
         elif prog == "git":
+            if not git_opts_derivable(rest):
+                mark("git option other than -C before the subcommand")
             v, eff = check_git(rest, cwd)
-            if not v and (seg_env or exported):
-                v = "GIT_CONFIG_KEY_n=core.hooksPath"
-            if v and in_scope(eff):
-                return v, eff
+            yield rest, eff, (v or seg_env or exported)
         elif prog in SHELLS or prog == "eval":
             inner = None
             if prog == "eval":
                 inner = " ".join(rest)
             elif "-c" in rest and rest.index("-c") + 1 < len(rest):
                 inner = rest[rest.index("-c") + 1]
+            if prog in SHELLS and not (rest[:1] == ["-c"] and len(rest) > 1):
+                mark("shell other than `<sh> -c '...'`")
             if inner:
-                v, eff = scan(inner, cwd, depth + 1)
-                if v:
-                    return v, eff
+                yield from git_segments(inner, cwd, depth + 1, marks)
+
+
+def scan(cmd, cwd):
+    """-> (violation, effective_cwd_of_the_violating_git) or (None, None)。"""
+    for _args, eff, v in git_segments(cmd, cwd):
+        if v and in_scope(eff):
+            return v, eff
     return None, None
 
 
+def pass_through_defers(cmd, cwd, installed):
+    """elidex の push-branch / fmt hook の pass-through の判定 (P1 plan memo §7.1)。installed(cwd) -> bool は呼ぶ側が
+    渡す (その worktree の installed(wt))。True (defer してよい) は、cwd を導出できないと marks された形が 1 つも無く、
+    git の呼び出しが 1 つ以上あり、そのすべての実効 cwd が in_scope で installed の時だけ。未知の形は defer しない。
+    相対の `cd <dir>` は CDPATH が空だと仮定して導出する。内部エラーも defer しない。"""
+    try:
+        marks = []
+        effs = [eff for _args, eff, _v in git_segments(cmd, cwd, marks=marks)]
+        return not marks and bool(effs) and all(in_scope(e) for e in effs) and all(installed(e) for e in effs)
+    except Exception:
+        return False
+
+
 def reason(v):
     return ("git の hook を飛ばす指定 (%s) を止めました。elidex の hook (fmt / ratchet 等) は飛ばさない規則です。\n"
             "hook が失敗した原因を直して通してください。どうしても飛ばす必要があるなら PM に上げて指示を受けてください。" % v)
```

**Tests** (added to the `DENY` / `ALLOW` lists, as two methods of `T`, and as two classes, `Segments` and
`PassThrough`, that load the script under test by path, in `git-hook-bypass-guard.test.py`). **This paragraph and the
Mutants paragraph below are the one home of the counts** (the P1a memo points here): run on 2026-11-22 (PM 930)
against a scratch copy of the live file patched with `hook-v3.diff` (the block above) and `test-v3.diff`, both in
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/ci-tools-lane/guardpatch/` (`guardpatch/` below): 26 tests OK;
against the unpatched file: 40 failures and 68 errors. Each positive form was measured to reroute, or to write, the
key (§3.2):

| kind | commands (cwd in scope unless stated) | expected |
|---|---|---|
| legacy write | `git config core.hooksPath /alt`; with `--local`, `--worktree`, `--global`, `--system`; `git config core.hooksPath /alt --worktree`; `--worktree core.HOOKSPATH`, `--worktree CORE.hookspath`; `--add --worktree`; `--replace-all`; `--type=path --worktree`, `--type path --worktree`; `--comment x`; `--worktree -- core.hooksPath /alt` | deny |
| legacy unset / section | `--unset`, `--unset-all core.hooksPath`; `--remove-section core`; `--rename-section core zz` | deny |
| subcommands | `config set --worktree`, `set --all`, `set --append`; `config unset`, `unset --all`; `remove-section core`; `rename-section Core zz` | deny |
| file target | `--file .git/config.worktree`, `-f …`, `-f.git/…`, `--file=…`; from outside: `--file <elidex>/.git/config core.hooksPath /alt`, `-f <elidex>/.git/config --unset core.hooksPath` | deny |
| editor | `git config --edit --worktree`, `-e`, `config edit` | deny (an editor can write any key) |
| reach | `git -C <elidex> config --worktree …`; `cd <elidex> && git config --worktree …` from outside; `bash -c '…'`; `git status && git config …` | deny |
| env | `GIT_CONFIG_PARAMETERS="'core.hooksPath'='/alt'" git commit -m x`; `env GIT_CONFIG_PARAMETERS="'core.hookspath=/alt'" git push origin HEAD`; `export GIT_CONFIG_PARAMETERS="'CORE.HOOKSPATH'='/alt'"; git commit -m x` | deny |
| reads (negative) | `git config core.hooksPath`, `--get`, `--get-all`, `--show-origin --get`, `--worktree core.hooksPath`, `--get-regexp core.hooks`, `--list`, `-l --show-scope`, `config get`, `config get --all`, `config list` | allow |
| other keys (negative) | `--worktree user.name a`; `config set core.editor vim`; `--unset user.email`; `--remove-section alias`; `--worktree core.hooks /alt`; `--worktree core.hooksPathX /alt`; `GIT_CONFIG_PARAMETERS="'user.name'='a'" git commit -m x` | allow |
| text, not a command (negative) | `git commit -m 'git config core.hooksPath /alt'`; `git log --grep 'git config --worktree core.hooksPath'` | allow |
| out of scope (negative) | every `DENY` entry from `/private/tmp` (the existing loop); `git config --file /private/tmp/x core.hooksPath /alt` from `/private/tmp` | allow |
| `git_segments` cwd | `cd /s/wt && git push origin HEAD` from `/s/repo`; `git -C sub push origin HEAD` from `/r`; `bash -c 'cd /x && git status'`; `sh -c 'git -C y status'`; `eval "cd /x && git status"` | effective cwds `/s/wt`; `/r/sub`; `/x`; `/r/y`; `/x` |
| E21 compound | `cd <e>/repo && git push origin HEAD:x; cd <e>/wt && git push origin HEAD:y` from `<e>/repo` (`<e>` in scope), with only `<e>/repo` installed | cwds `<e>/repo`, `<e>/wt`; `pass_through_defers` false; true when both are |
| pass-through: the PM 930 probe | the 22 forms, each with its JSON cwd, every cwd reported installed | `cd a; cd b && git push` from an elidex cwd defers; the 21 others do not, the 13 that deferred before among them |
| pass-through: derivable | `git push origin HEAD`; `git status && git push origin HEAD`; `cd <elidex> && git push origin HEAD` and `git -C <elidex> push origin HEAD` from another repository; `bash -c 'git push origin HEAD'`; `git push origin HEAD 2>&1 \| tail -5`; `git --no-pager log -1 && git push origin HEAD`; `cargo fmt --all && git add -A && git commit -m x` | defers |
| pass-through: one kind of mark each | `(true; cd <elidex>; true); git push …` from another repository; `xargs … git`; `echo "$(cd <other> && git push …)"; git status`; `GIT_DIR=…`; `{ cd <other>; git push …; }`; `cd <elidex> && cd - && git push …`; `false && cd <elidex>; git push …`; `--git-dir=…`; `bash -lc '…'` — every yielded cwd in scope | exactly one kind of mark; does not defer |
| pass-through: edges | five nested `bash -c` under `git status &&`; `ls -la`; an unterminated quote; an `installed` that raises | does not defer |
| marks beside `scan()` | the 22 forms, `git_segments` with and without `marks` | the same yields |
| `scan()` on the iterator | `git_segments` replaced in the loaded module by one that yields a sentinel violation in scope | `scan("true", <out of scope>)` returns the sentinel |

**Mutants** (each a replacement in a temp copy of the patched file, asserted to occur exactly once, run through the
file's own `GIT_BYPASS_GUARD_SCRIPT` override; on 2026-11-22 (PM 930) all twenty-six were killed by the tests above.
Twenty-five are one-line replacements; the `scan()` row replaces a block):

| replaced line | → | killed by |
|---|---|---|
| `if not any(n.lower() == "core.hookspath" for n in names):` | exact-case compare | `core.HOOKSPATH` forms |
| `if k.upper() == "GIT_CONFIG_PARAMETERS" and` | `if False and` | the env rows |
| `if target and not in_scope(eff) and in_scope(target):` | `if False:` | the from-outside `--file` rows |
| `if sub is None and len(names) == 1 and not acts & CONFIG_WRITE_ACTIONS:` | drop `len(names) == 1` | every legacy write |
| `CONFIG_READ_SUBCMDS = {"list", "get"}` | add `"set"` | the `set` rows |
| `    if sub == "config":` | another name | every config row |
| `        if names and names[0].lower() == "core":` | exact-case compare | `rename-section Core zz` |
| `    if sub == "edit" or acts & {"-e", "--edit"}:` | `if sub == "edit":` | `--edit`, `-e` |
| `            pos.extend(args[i + 1:])` | `pass` | `--worktree -- core.hooksPath /alt` |
| `                cwd = os.path.join(cwd, os.path.expanduser(d))` (in `git_segments`) | `pass` (the iterator ignores `cd`) | the E21 compound case, the `cd` / nested / `eval` cwd cases, `cd <elidex> && …` from outside, the pass-through probe, derivable and one-mark cases |
| `                yield from git_segments(inner, cwd, depth + 1, marks)` | `pass` (recursion dropped) | the nested and `eval` cwd cases, the `bash -c` `DENY` entries, the pass-through probe, derivable and nesting cases |
| the body of `scan()` | a full copy of the pre-refactor walk (a divergent copy) | only the `scan()`-on-the-iterator case: the copy behaves identically, so no behavioural case can see it; the test replaces `git_segments` and observes that `scan()` follows it |
| `        return not marks and bool(effs) and …` (in `pass_through_defers`) | drop `not marks and` (marks ignored) | the probe and one-mark cases, nesting |
| `            mark("git in an argument position")` | `pass` (an un-yielded `git` token not marked) | the `xargs` one-mark case, the probe |
| `        mark("( ): subshell, $( ), function or case")` | `pass` (a subshell's `cd` read as leaking) | the `(true; cd <elidex>; true); git push …` one-mark case (in the plain `(cd …)` forms the `cd` mark also fires) |
| `            mark("command substitution")` | `pass` | the `"$( … )"` one-mark case |
| `            mark("assignment or wrapper before the command")` | `pass` | the `GIT_DIR=` one-mark case, the probe |
| `            mark("reserved word or shell builtin: " + prog)` | `pass` | the `{ …; }` one-mark case, the probe |
| `            mark("cd whose target or reach is not derivable")` | `pass` | the `cd -` and `false && cd` one-mark cases, the probe |
| `                mark("git option other than -C before the subcommand")` | `pass` | the `--git-dir=` one-mark case, the probe |
| `` mark("shell other than `<sh> -c '...'`") `` | `pass` | the `bash -lc` one-mark case, the probe |
| `        mark("nested deeper than 3")` | `pass` | the nesting case |
| `bool(effs) and all(in_scope(e) for e in effs) and all(installed` | drop the `in_scope` conjunct | the probe (`cd ~/repos/send.sh/lexime && git push` and the other out-of-scope forms) |
| ` and all(installed(e) for e in effs)` | dropped | the E21 compound case, the raising `installed` |
| `            cur, before = [], tok` (in `split`) | `cur = []` (the operator before a segment is lost) | the `false && cd` one-mark case |
| `and (seq_op(after) or after == "&&")):` | `and seq_op(after)):` (a `cd` before `&&` marked) | the derivable cases, the E21 compound case, the probe |

**The PM's application and E2E line** (global: it acts on every session on this machine at once, other
repositories' sessions included, `~/.claude` CLAUDE.md "変更の blast radius"): back up both files
(`cp -p ~/.claude/hooks/git-hook-bypass-guard{,.test}.py <backup dir>/`, the path into the PM's log); show the user
the whole diff (`guardpatch/hook-v3.diff`, the block above, and `test-v3.diff`) — the `git_segments()` refactor of
`scan()` (PM 922) and the marks and `pass_through_defers()` (PM 930) included — and apply it only after; apply the
diff and the test additions, then `python3 ~/.claude/hooks/git-hook-bypass-guard.test.py` → OK, then the mutants
above → each killed. E2E, from a Claude session whose cwd is an elidex worktree: the Bash call
`git config --worktree core.hooksPath /tmp/x` → denied with "git config core.hooksPath" in the reason, and
`git rev-parse --git-path hooks` unchanged afterwards; `git config --get core.hooksPath` → runs. If the hook files
are dotfiles-managed, the edit goes through the dotfiles session.
