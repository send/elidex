# Plan — P1a cut-over: the bypass-guard patch (global)

## §0 Status

**Where this belongs (PM 916)**: this file is §4.8 of the P1a memo,
`docs/plans/2026-11-agent-neutral-git-guards-p1a.md`, moved here verbatim so that memo stays under 1000 lines. It
is in neither PR's diff: the PM applies it at the P1a cut-over, and the P1a memo's cut-over (its §7.1 step A2, with
E24 of its §7.5) owns applying it, by reference to this file. Every section and id named below without a file
(§3.2, §7.1, I1, I4, …) is the P1a memo's; this file defines none of its own.
Since the move it has four revisions: PM 922 added the `git_segments()` iterator ("One derivation" below), PM 930
a list of marks for the pass-through, PM 936 replaced that list with a positive grammar, and PM 938 narrowed the
grammar's git segments to `push` and `commit` with option allowlists ("Defer only on a positive minimal grammar" below).

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

**Defer only on a positive minimal grammar (PM 936, PM 938).** PM 930 deferred unless `git_segments` set a *mark* at a shape
it does not derive. That is an enumerated exemption, and the next shape no mark named fell through as derivable: the
PM 936 attestation found zsh's builtins and precommands (zsh, `/bin/zsh`, is the shell the Bash tool runs) — `chdir
<dir>`, `noglob cd <dir>`, `nocorrect cd <dir>`, `repeat 1 cd <dir>`, `- cd <dir>`; 87 of zsh's 127 builtins and
reserved words were on no list — a `#` comment and a heredoc body, both read by the walker as commands; from an
elidex worktree `chdir <lexime>; git push …` and `noglob cd <lexime>; git push …` deferred (driver-confirmed). So the
criterion is turned around: **only the commands of one small grammar can defer; everything else does not.** The
grammar is one function in this file, `pass_through_grammar(cmd)` (its git segments in `pass_through_git(args)`),
and it is the grammar's one home (the P1a memo §7.1 points here). `pass_through_defers(cmd, cwd, installed)` is true
only when the grammar accepts `cmd`, `git_segments` yields at least one git segment, and every yielded cwd is in scope and `installed` (the caller passes
`installed`: the §4.7 form of the P1a memo §7.1). The grammar:
- a command is one or more non-empty segments joined only by `;` or `&&`;
- a segment is `cd <path>` with exactly one argument, an absolute path, or `git`, then zero or more `-C <path>`, then
  `push` or `commit` and only the words that subcommand's allowlist names (`pass_through_git(args)`). The program word
  is an unquoted `git` or `cd`;
- `push` (what the push-branch hook `:24` decides): each word is `-u`, `--set-upstream`, `-n`, `--dry-run`, `--tags`
  or a positional; the first positional (the remote) is a name of letters, digits, `_`, `.` and `-` only (no `/` or
  `:`, so no URL, path or `<transport>::` helper), the others (refspecs) use those and `/ @ : +` only; no positional
  starts with `-`;
- `commit` (what the fmt hook `:30` decides): each word is `-a`, or `-m` / `-F` and its value, the next word whatever
  it is (git's option parser takes it so too: measured, `git commit -m -n` records the message `-n`). No pathspec. A combined cluster (`-am`), an attached value (`-mx`) and a
  long form (`--message=x`) are not on the list, so such a command does not defer (`:30` decides it, as today);
- anything else in a git segment — another subcommand (an alias too: git ignores an alias that hides a git command, deprecated
  ones excepted, git-config(1) `alias.*`; measured on 2.56.1, `-c alias.push=…` is ignored; so a `push` / `commit` word
  is the builtin), another option before or after the subcommand (`-c …`, `--exec-path`,
  `--no-pager`, `--receive-pack` / `--exec`, `--template`, `--no-verify`, `--`) — and the whole command does not defer;
- a path has no `.` or `..` component, so the textual join `git_segments` and `in_scope` make is the directory the
  shell enters; a `cd` target is absolute, so `CDPATH` and zsh's `cdablevars` play no part;
- every character is a letter, a digit, a space, or one of `_ . / @ % + , : = - ' " ; &` — so no `#`, `<`, `>`,
  `|`, `$`, `` ` ``, `~`, `\`, `(`, `)`, `{`, `}`, `[`, `]`, `*`, `?`, `!`, `^`, tab or newline: no comment,
  heredoc, redirection, pipe, expansion, subshell, brace, glob or second line; no `;` or `&` inside quotes (so the
  grammar's segments are `git_segments`'s); no unquoted word starts with `=` (zsh's `=cmd` expansion).

**Why only `push` / `commit` (PM 938).** PM 936's grammar took any subcommand with any arguments, so a git subcommand
that runs a command deferred — `git rebase --exec "git -C <lexime> push …"`, `rebase -x`, `git bisect run git -C
<lexime> push …`, `git submodule foreach '…'`, `git difftool -x '…'` (the attestation of bd367914, driver-confirmed):
the git inside is an argument, which neither `git_segments` nor the grammar reads as a git call. The skipped hooks
decide only `push` (`:24`) and `commit` (`:30`), so only those can defer, each with the options it needs. `-C <path>`
stays before the subcommand: it is the form a driver uses from another cwd (`git -C <wt> push …`), and `git_segments`
derives its cwd as it does a `cd`'s. PM 936's other pre-subcommand options (`--no-pager`, `-p`, `-P`, `--paginate`,
`--no-optional-locks`, `--no-advice`) are dropped: neither hook needs them.

So a builtin, precommand, reserved word, assignment or wrapper in front, a nested shell (`bash -c …`), a git
subcommand or option that runs a command (`rebase --exec` / `-x`, `bisect run`, `submodule foreach`, `difftool -x`,
`filter-branch`, `push --receive-pack` / `--exec`, `fetch --upload-pack`, `-c core.editor=…`, an alias), any other git
subcommand (`git status && git push …`), a pipe into another program (`git log | tail`) and a compound with another
program (`cargo fmt --all && git commit …`) do not defer: the Claude hook runs on them as today, and the double guard
is harmless. A `cd <dir>; git …` whose `cd` fails leaves git in the previous cwd; `installed(<dir>)` then fails too (its `git -C <dir>` cannot enter the
directory either), so the command does not defer. The grammar assumes that the session shell defines no alias or
function named `git` or `cd` (`type cd git` in `/bin/zsh` on 2026-11-22: a shell builtin, `/opt/homebrew/bin/git`).
`git_segments` takes no marks: PM 930's `marks` argument and its operator tracking are gone, so the iterator and
`scan()` are PM 922's, unchanged in behaviour (every existing test passes on them).
**`scan()`'s deny is not widened (PM 930, PM 936).** The shapes whose cwd it misreads or whose git call it does not
see are its blind spots, and its docstring lists them: a `cd` inside `( … )` read as leaking, `cd -`, `pushd` /
`popd`, a `cd` target with `$`, a command starting with a reserved word, `bash -lc`, git after a wrapper's options,
git in an argument position, `GIT_DIR=` / `GIT_WORK_TREE=` and `--git-dir` / `--work-tree`, zsh's `chdir` and the
precommands (`noglob cd`, `nocorrect cd`, `repeat N cd`, `- cd`), a `#` comment and a heredoc body (both read as
commands). For what the hooks decide (push and commit), the long-run canonical guard is P1a's git hooks: git runs
them whatever the shell spelling. The reroute of the hooks path itself, which no git hook sees, stays with this
guard's text reading and `--check` (I4).

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
@@ -6,13 +6,33 @@
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
+引数の位置にある git、`GIT_DIR=` / `GIT_WORK_TREE=` と `--git-dir` / `--work-tree`、zsh の `chdir` と precommand
+(`noglob cd` / `nocorrect cd` / `repeat N cd` / `- cd`)、`#` から行末までの comment (その中も command として読む)、
+heredoc の本文 (command として読む)。長期の正はここではなく elidex の git hook (P1 plan memo の P1a、pre-push /
+pre-commit): shell の綴りによらず git 自身が走らせる。
+pass_through_defers() (elidex の push-branch / fmt hook の pass-through) は逆向きに倒す: pass_through_grammar() の
+正の最小文法 (`cd <絶対 path>` と、許可リストの option だけの `git push` / `git commit`) に当てはまる command だけを
+defer の候補にし、それ以外 (上の形も、command を走らせる git の subcommand / option も、未知の形も) は defer しない。
 
 Fails OPEN: どんな内部エラーでも no output, exit 0。
 Env overrides (tests): GIT_BYPASS_GUARD_STATE_DIR, GIT_BYPASS_GUARD_EXTRA_SCOPE (os.pathsep 区切り)。
@@ -35,6 +55,15 @@
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
@@ -95,9 +124,16 @@
 
 
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
@@ -138,6 +174,58 @@
             if ch in COMMIT_VALUE_SHORT:
                 break
     return False
+
+
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
 
 
 def check_git(args, cwd):
@@ -173,6 +261,12 @@
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
@@ -186,14 +280,16 @@
     return (hits[0] if hits else None), eff
 
 
-def scan(cmd, cwd, depth=0):
-    """-> (violation, effective_cwd_of_the_violating_git) or (None, None)。cd を追跡する。"""
+def git_segments(cmd, cwd, depth=0):
+    """cmd の git の呼び出しごとに (git の後ろの token, 実効 cwd, 違反 or "") を yield する。
+    実効 cwd の導出 (hook JSON の cwd、先行する `cd`、`git -C <dir>`) と `bash -c` / `sh -c` / `eval` の再帰は
+    ここにだけある。scan() と、elidex の push-branch / fmt hook の pass-through の両方がこれを使う。"""
     if depth > 3:
-        return None, None
-    exported = False  # `export GIT_CONFIG_KEY_n=core.hooksPath` seen earlier in this command
+        return
+    exported = ""  # `export GIT_CONFIG_KEY_n=core.hooksPath` / `GIT_CONFIG_PARAMETERS=...` seen earlier in this command
     for seg in segments(cmd):
         j = 0
-        seg_env = any(env_hooks_assign(t) for t in seg[:next((n for n, t in enumerate(seg) if not (is_assign(t) or os.path.basename(t) in WRAPPERS)), len(seg))])
+        seg_env = next(filter(None, map(env_hooks_assign, seg[:next((n for n, t in enumerate(seg) if not (is_assign(t) or os.path.basename(t) in WRAPPERS)), len(seg))])), "")
         while j < len(seg) and (is_assign(seg[j]) or os.path.basename(seg[j]) in WRAPPERS):
             j += 1
         if j >= len(seg):
@@ -204,13 +300,10 @@
             if d:
                 cwd = os.path.join(cwd, os.path.expanduser(d))
         elif prog == "export":
-            exported = exported or any(env_hooks_assign(t) for t in rest)
+            exported = exported or next(filter(None, map(env_hooks_assign, rest)), "")
         elif prog == "git":
             v, eff = check_git(rest, cwd)
-            if not v and (seg_env or exported):
-                v = "GIT_CONFIG_KEY_n=core.hooksPath"
-            if v and in_scope(eff):
-                return v, eff
+            yield rest, eff, (v or seg_env or exported)
         elif prog in SHELLS or prog == "eval":
             inner = None
             if prog == "eval":
@@ -218,12 +311,141 @@
             elif "-c" in rest and rest.index("-c") + 1 < len(rest):
                 inner = rest[rest.index("-c") + 1]
             if inner:
-                v, eff = scan(inner, cwd, depth + 1)
-                if v:
-                    return v, eff
+                yield from git_segments(inner, cwd, depth + 1)
+
+
+def scan(cmd, cwd):
+    """-> (violation, effective_cwd_of_the_violating_git) or (None, None)。"""
+    for _args, eff, v in git_segments(cmd, cwd):
+        if v and in_scope(eff):
+            return v, eff
     return None, None
 
 
+# pass-through の受理文法 (pass_through_grammar) に使ってよい字。`#` `<` `>` `|` `$` `` ` `` `~` `\` `(` `)` `{` `}`
+# `[` `]` `*` `?` `!` `^` 改行 tab は入っていない (comment、heredoc / redirect、pipe、展開、subshell、brace、glob)
+PT_CHARS = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _./@%+,:=-'\";&")
+# defer してよい git の呼び出し: skip される hook が判定する subcommand (push = push-branch hook、commit = fmt hook) と、
+# それぞれの option の許可リスト。ほかの subcommand・option は (command を走らせるもの: rebase --exec、bisect run、
+# submodule foreach、difftool -x、push --receive-pack / --exec、-c core.editor=...、alias を含めて) defer しない
+PT_PUSH_FLAGS = {"-u", "--set-upstream", "-n", "--dry-run", "--tags"}
+PT_COMMIT_FLAGS = {"-a"}
+PT_COMMIT_VALUED = {"-m", "-F"}  # 値は次の語 (git の parse-options と同じく、`-` で始まっても値)
+PT_NAME_CHARS = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.-")
+PT_REFSPEC_CHARS = PT_NAME_CHARS | set("/@:+")
+
+
+def plain_path(d, absolute):
+    """`.` / `..` の component を持たない path の字面 (absolute なら `/` で始まる)。展開の字は PT_CHARS が除く。"""
+    return bool(d) and not d.startswith(("-", "=")) and (d.startswith("/") or not absolute) \
+        and all(c not in (".", "..") for c in d.split("/"))
+
+
+def pass_through_git(args):
+    """`git` の後ろの語 -> pass-through の文法に当てはまれば True。`-C <path>` (0 回以上) の後に `push` か `commit`。
+    push: 各語は PT_PUSH_FLAGS か位置引数。位置引数の 1 つ目 (remote) は PT_NAME_CHARS だけの名前 (`/` と `:` が無い
+    ので URL・path・`<transport>::` は外れる)、2 つ目以降 (refspec) は PT_REFSPEC_CHARS だけ。どちらも `-` で始まらない。
+    commit: 各語は `-a`、または `-m` / `-F` とその値 (次の語) だけ。位置引数 (pathspec) は無い。束ねた短い option
+    (`-am`) と値を付けた形 (`-mx`、`--message=x`) は許可リストに無いので外れる。"""
+    k = 0
+    while k + 1 < len(args) and args[k] == "-C" and plain_path(args[k + 1], absolute=False):
+        k += 2
+    sub, rest = (args[k], args[k + 1:]) if k < len(args) else (None, [])
+    if sub == "push":
+        pos = []
+        for a in rest:
+            if a in PT_PUSH_FLAGS:
+                continue
+            if a.startswith("-"):
+                return False
+            pos.append(a)
+        return all(p and set(p) <= (PT_NAME_CHARS if n == 0 else PT_REFSPEC_CHARS) for n, p in enumerate(pos))
+    if sub == "commit":
+        k = 0
+        while k < len(rest):
+            if rest[k] in PT_COMMIT_FLAGS:
+                k += 1
+            elif rest[k] in PT_COMMIT_VALUED and k + 1 < len(rest):
+                k += 2
+            else:
+                return False
+        return True
+    return False
+
+
+def pass_through_grammar(cmd):
+    """pass-through が defer を考えてよい形 (正の最小文法) なら True。command = segment を `;` か `&&` だけで
+    つないだもの。segment は pass_through_git() の `git push` / `git commit` か `cd <絶対 path>` (引数 1 つ)。
+    program の語は quote 無しの `git` / `cd` そのもの、path は `.` / `..` の無い字面、字は PT_CHARS だけ、quote の中に
+    `;` / `&` は無く、quote 無しで `=` から始まる語 (zsh の =cmd 展開) も無い。これ以外 (builtin / precommand / 予約語 /
+    代入 / wrapper が前に付く形、`#`、heredoc、展開、redirect、pipe、subshell、brace、入れ子の shell、push / commit
+    以外の subcommand、許可リストに無い option) はすべて False。"""
+    if not cmd or not set(cmd) <= PT_CHARS:
+        return False
+    segs, words, word, quoted, q, i = [], [], None, False, None, 0
+    while i < len(cmd):
+        c = cmd[i]
+        if q:
+            if c in ";&":
+                return False
+            if c == q:
+                q = None
+            else:
+                word += c
+        elif c in "'\"":
+            q, quoted, word = c, True, (word or "")
+        elif c in " ;&":
+            if word is not None:
+                words.append((word, quoted))
+                word, quoted = None, False
+            if c != " ":
+                if c == "&":
+                    if cmd[i:i + 2] != "&&":
+                        return False
+                    i += 1
+                if not words:
+                    return False
+                segs.append(words)
+                words = []
+        else:
+            word = (word or "") + c
+        i += 1
+    if q:
+        return False
+    if word is not None:
+        words.append((word, quoted))
+    if not words:
+        return False
+    segs.append(words)
+    for seg in segs:
+        if any(w.startswith("=") and not qd for w, qd in seg) or seg[0][1]:
+            return False
+        prog, args = seg[0][0], [w for w, _qd in seg[1:]]
+        if prog == "cd":
+            if not (len(args) == 1 and plain_path(args[0], absolute=True)):
+                return False
+        elif prog == "git":
+            if not pass_through_git(args):
+                return False
+        else:
+            return False
+    return True
+
+
+def pass_through_defers(cmd, cwd, installed):
+    """elidex の push-branch / fmt hook の pass-through の判定 (P1 plan memo §7.1)。installed(cwd) -> bool は呼ぶ側が
+    渡す (その worktree の installed(wt))。True (defer してよい) は、cmd が pass_through_grammar() に当てはまり、
+    git_segments() (scan() と同じ導出) が git の呼び出しを 1 つ以上 yield し、そのすべての実効 cwd が in_scope で
+    installed の時だけ。それ以外と内部エラーは defer しない。"""
+    try:
+        if not pass_through_grammar(cmd):
+            return False
+        effs = [eff for _args, eff, _v in git_segments(cmd, cwd)]
+        return bool(effs) and all(in_scope(e) for e in effs) and all(installed(e) for e in effs)
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
Mutants paragraph below are the one home of the counts and probe results** (the P1a memo points here): run on
2026-11-22 (PM 938) against a scratch copy of the live file patched with `hook-v5.diff` (the block above) and
`test-v5.diff`, both in `/Users/kazuaki/repos/send.sh/elidex-pm-trial/ci-tools-lane/guardpatch/` (`guardpatch/`
below): 25 tests OK; against the unpatched file: 40 failures and 389 errors (subtests counted). The two diffs, applied
with `patch -p1` to a fresh copy of the live files (`9902fe77…0438`), reproduce the patched files byte for byte. The
**corpus** is 322 distinct forms whose git runs outside elidex, each given an elidex JSON cwd and every cwd reported
installed: the 72 forms of `REPORTED` whose push runs elsewhere (21 of the PM 930 probe's 22 — the 22nd,
`cd a; cd b && git push`, runs inside elidex —, the PM 936 attestation's 20, 8 forms of the classes PM 936
named, and PM 938's 23: the attestation's five and 18 siblings), and two forms per word of zsh 5.9's builtins and reserved words (`print -l ${(k)builtins}` and
`${(k)reswords}`: 127 distinct words, pinned in the test; zshmisc(1)'s precommand modifiers are among them),
generated from the pinned list, not picked (`ZSH_FORMS`). On the patched file none defers; on the PM 936 file
(`hook-v4.diff`) 21 of the 322 defer (the PM 938 forms but the two `-c` ones); on the PM 930 file (`hook-v3.diff`,
marks) 203. Each positive config form was measured to reroute, or to write, the key
(§3.2):

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
| pass-through: corpus | the 322 forms above: `chdir <lexime>; git push …`, `noglob cd <lexime>; git push …`, `- cd …`, `repeat 1 cd …`, `cd <lexime>; true # ; cd <elidex>` and a newline before `git push …`, `cd <lexime>; cat <<EOF` with `cd <elidex>` as its body before `git push …`, `bash -lc '…'`, `env -C <lexime> git push …`, `GIT_DIR=…`, `( cd …; git push )`, `{ cd …; git push; }`, `CDPATH=…`, `cd <lexime>/.. && cd lexime && …`, and `<w> cd <lexime>; git push …` / `<w> <lexime>; git push …` for each of the 127 zsh words; PM 938: `git rebase --exec "git -C <lexime> push"`, `rebase -x`, `bisect run git -C <lexime> push`, `submodule foreach` (also `--recursive`), `difftool -x` / `--extcmd`, `filter-branch --tree-filter`, `push --receive-pack=…` (before and after the refspec) / `--exec=…`, `fetch` / `ls-remote --upload-pack=…`, `archive --exec=…`, `grep --open-files-in-pager=…`, `-c core.editor=… commit`, `-c core.sshCommand=… push`, `for-each-repo --config=x push`, an alias (`git pu`), `merge -s x`, and each after a plain `git commit` / `git push` or behind `-C <elidex>` | none defers |
| pass-through: plain | `git push`; `git push origin HEAD`, `origin wtb`; `-u origin HEAD:refs/heads/wtb`; `--set-upstream --dry-run origin +wtb :old`; `-n --tags origin`; `cd <elidex> && git push origin HEAD`, `cd <elidex>; git push origin HEAD` and `git -C <elidex> push origin HEAD` from another repository; `git commit -m x`; `-m 'fix: one line' -m 'body, two'`; `-a -m x`; `-F msg.txt`; `git commit`; `git commit -m x && git push origin wtb` | the grammar accepts; defers |
| pass-through: one rule each | 46 otherwise plain commands from an elidex cwd, each breaking one rule: `#` and `<<<` (as a `-m` value), `>`, a newline, `&` (trailing, and between two commands), `\|\|`, `$`, `` ` ``, `*`, `( )`, `{ }`, `noglob git`, `..` and `.` in a `cd` target, a relative `cd`, `cd` with two arguments, a bare `cd`, `..` in `-C`, a word `=x`, `'git'`, `;` and `&&` inside quotes, `-c x=y`, no subcommand (`git --no-pager`, `git`), `bash -c '…'`, `GIT_DIR=`, a trailing `;`, `;;`, `&&&`, the empty command; PM 938: `git status && git push …`, `--no-pager push`, an unlisted push option (a force flag, `--receive-pack=x`, `--`), a path remote, an `ext::` remote, a refspec with `,`, `commit --template=x`, `--no-verify`, `-am x`, `-mx`, a pathspec, `-m` without a value | the grammar rejects; does not defer |
| pass-through: edges | `cd <elidex>` alone from another repository (no git call); `git push origin HEAD` from `/private/tmp`; an `installed` that raises | does not defer |
| `scan()` on the iterator | `git_segments` replaced in the loaded module by one that yields a sentinel violation in scope | `scan("true", <out of scope>)` returns the sentinel |

**Mutants** (each a replacement in a temp copy of the patched file, asserted to occur exactly once, run through the
file's own `GIT_BYPASS_GUARD_SCRIPT` override; on 2026-11-22 (PM 938) all 38 were killed by the tests above.
(i)–(v) are the ones PM 936 named, (a)–(d3) the ones PM 938 named; the `scan()` row replaces a block, the others one
or two lines. (ii) and (iii) are killed by the one-rule rows only: without a newline, which the grammar excludes too,
a `#` or a `<<` cannot make git run where the walker does not see it, so every corpus form carrying one also breaks a
second rule; the rows pin the rule itself, in the one position the grammar leaves open to any word, a `-m` value):

| replaced | → | killed by |
|---|---|---|
| (i) the `else: return False` for a program other than `git` / `cd` in `pass_through_grammar` | `pass` (any program word accepted: the old behaviour) | the corpus, one-rule |
| (ii) `PT_CHARS` | `#` added | one-rule (`-m #c`) |
| (iii) `PT_CHARS` | `<` added (`<<` allowed) | one-rule (`-m <<<x`) |
| (iv) `prog, args = seg[0][0], …` | a word in front of `cd` skipped (a builtin before `cd` accepted) | the corpus (`noglob cd <lexime>; git push …` and the other generated `<w> cd` forms) |
| (v) `if prog == "cd":` | `if prog == "cd" and False:` (plain `cd <wt> && git push` rejected) | plain, the E21 compound case |
| `if c in ";&":` (inside quotes) | `if False:` | one-rule (`'a;b'`, `'a&&b'`) |
| the `.` / `..` component test of `plain_path` | dropped | one-rule (`/x/..`, `/.`, `-C ../main`) |
| `(d.startswith("/") or not absolute)` | `True` (a relative `cd` accepted) | one-rule (`cd x`) |
| the unquoted-`=` word test | dropped | one-rule (`=x`) |
| the quoted-program test (`seg[0][1]`) | dropped | one-rule (`'git'`) |
| `if cmd[i:i + 2] != "&&":` | `if False:` (a lone `&` taken as a separator) | one-rule (`git fetch & git push …`) |
| the inner `if not words: return False` | `pass` (empty segments accepted) | one-rule (`;;`, trailing `;`) |
| `len(args) == 1 and` for `cd` | dropped | one-rule (`cd <a> <b>`) |
| (a) the final `return False` of `pass_through_git` | `return True` (any subcommand, any arguments: PM 936's grammar) | the corpus (the PM 938 forms), one-rule |
| (b) `if a in PT_PUSH_FLAGS:` | `or a.startswith(("--exec", "--receive-pack"))` added (`-x` is rebase's / difftool's, which (a) covers) | the corpus (`push --exec=…`, `--receive-pack=…`), one-rule |
| (c) `return False` for an unlisted push option | `continue` (any push option accepted) | the corpus, one-rule (a force flag, `--`) |
| (d1) `return all(p and set(p) …` | `bool(pos) and` added (plain `git push` rejected) | plain |
| (d2) `PT_REFSPEC_CHARS` for the refspecs | `set()` (`git push origin <branch>` rejected) | plain, the E21 compound case |
| (d3) `PT_COMMIT_VALUED = {"-m", "-F"}` | `{"-F"}` (`git commit -m x` rejected) | plain |
| `args[k] == "-C"` in the pre-subcommand loop | `args[k].startswith("-")` (any option with a value accepted) | one-rule (`-c x=y`), the corpus (`-c core.editor=…`) |
| `PT_NAME_CHARS` for the remote | `PT_REFSPEC_CHARS` | one-rule (path remote, `ext::x`) |
| the commit loop's `else: return False` | `k += 1` (any commit word accepted) | one-rule (`--template`, `-am`, `-mx`, a pathspec) |
| `and k + 1 < len(rest)` for `-m` / `-F` | dropped | one-rule (`-m` without a value) |
| `if not pass_through_grammar(cmd):` (in `pass_through_defers`) | `if False:` | the corpus, one-rule |
| `bool(effs) and all(in_scope(e) for e in effs) and all(installed` | the `in_scope` conjunct dropped | the corpus, edges (`/private/tmp`) |
| ` and all(installed(e) for e in effs)` | dropped | the E21 compound case, the raising `installed` |
| `cwd = os.path.join(cwd, os.path.expanduser(d))` (in `git_segments`) | `pass` (the iterator ignores `cd`) | the `cd` / nested / `eval` cwd cases, the E21 compound case, `cd <elidex> && …` from outside, plain, the corpus |
| `yield from git_segments(inner, cwd, depth + 1)` | `pass` (recursion dropped) | the nested and `eval` cwd cases, the `bash -c` `DENY` entries |
| the body of `scan()` | a loop over a full copy of `git_segments` (a divergent copy) | only the `scan()`-on-the-iterator case: the copy behaves identically, so no behavioural case can see it; the test replaces `git_segments` and observes that `scan()` follows it |
| `if not any(n.lower() == "core.hookspath" for n in names):` | exact-case compare | `core.HOOKSPATH` forms |
| `if k.upper() == "GIT_CONFIG_PARAMETERS" and` | `if False and` | the env rows |
| `if target and not in_scope(eff) and in_scope(target):` | `if False:` | the from-outside `--file` rows |
| `if sub is None and len(names) == 1 and not acts & CONFIG_WRITE_ACTIONS:` | drop `len(names) == 1` | every legacy write |
| `CONFIG_READ_SUBCMDS = {"list", "get"}` | add `"set"` | the `set` rows |
| `if sub == "config":` | another name | every config row |
| `if names and names[0].lower() == "core":` | exact-case compare | `rename-section Core zz` |
| `if sub == "edit" or acts & {"-e", "--edit"}:` | `if sub == "edit":` | `--edit`, `-e` |
| `pos.extend(args[i + 1:])` | `pass` | `--worktree -- core.hooksPath /alt` |

**The PM's application and E2E line** (global: it acts on every session on this machine at once, other
repositories' sessions included, `~/.claude` CLAUDE.md "変更の blast radius"): back up both files
(`cp -p ~/.claude/hooks/git-hook-bypass-guard{,.test}.py <backup dir>/`, the path into the PM's log); show the user
the whole diff (`guardpatch/hook-v5.diff`, the block above, and `test-v5.diff`) — the `git_segments()` refactor of
`scan()` (PM 922) and `pass_through_grammar()` / `pass_through_git()` / `pass_through_defers()` (PM 936, PM 938)
included — and apply it only after; apply the
diff and the test additions, then `python3 ~/.claude/hooks/git-hook-bypass-guard.test.py` → OK, then the mutants
above → each killed. E2E, from a Claude session whose cwd is an elidex worktree: the Bash call
`git config --worktree core.hooksPath /tmp/x` → denied with "git config core.hooksPath" in the reason, and
`git rev-parse --git-path hooks` unchanged afterwards; `git config --get core.hooksPath` → runs. If the hook files
are dotfiles-managed, the edit goes through the dotfiles session.
