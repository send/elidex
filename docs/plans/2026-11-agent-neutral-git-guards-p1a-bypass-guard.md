# Plan — P1a cut-over: the bypass-guard patch (global)

## §0 Status

**Where this belongs (PM 916)**: this file is §4.8 of the P1a memo,
`docs/plans/2026-11-agent-neutral-git-guards-p1a.md`, moved here verbatim so that memo stays under 1000 lines. It
is in neither PR's diff: the PM applies it at the P1a cut-over, and the P1a memo's cut-over (its §7.1 step A2, with
E24 of its §7.5) owns applying it, by reference to this file. Every section and id named below without a file
(§3.2, §7.1, I1, I4, …) is the P1a memo's; this file defines none of its own.
Since the move: PM 922 added the `git_segments()` iterator ("One derivation" below). [history] PM 930, PM 936 and
PM 938 revised a pass-through decision layer this patch carried for the `:24` / `:30` hooks; PM 950 (user, option D)
superseded them and removed that layer, so the patch is the deny rules and the iterator only (`hook-v7-1.diff`).

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
`eval` recursion — into one iterator, `git_segments(cmd, cwd)`, which yields `(effective_cwd, violation)` for every
git segment, nested ones included; the violation is the one `check_git` (or an env assignment) found. `scan()`
becomes the first in-scope violation of that iterator (behaviour unchanged: every existing test passes on it), and
it is the iterator's one caller, so the derivation exists once.

**`scan()`'s deny is not widened; its blind spots (PM 930, PM 936).** The shapes whose cwd it misreads or whose git
call it does not see are not denied, the deny is not widened to them, and its docstring lists them, zsh's forms
included (PM 936): a `cd` inside `( … )` read as leaking, `cd -`, `pushd` / `popd`, a `cd` target with `$`, a
command starting with a reserved word, `bash -lc`, git after a wrapper's options, git in an argument position,
`GIT_DIR=` / `GIT_WORK_TREE=` and `--git-dir` / `--work-tree`, zsh's `chdir` and the precommands (`noglob cd`,
`nocorrect cd`, `repeat N cd`, `- cd`), a `#` comment and a heredoc body (both read as commands). The probe of these
shapes: the Tests paragraph below. A bypass through one of them (`--no-verify`, or a reroute of the hooks path) is
not stopped locally: this guard misses it and git does not run the hooks. What stands behind it, and the two classes
nothing stands behind, are P1a I4's "Residual accepted gap". The reroute of the hooks path itself, which no git hook sees, stays
with this guard's text reading and `--check` (I4).

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
@@ -6,13 +6,31 @@
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
+heredoc の本文 (command として読む)。これらの形の `--no-verify` や hooksPath の迂回は deny されず、git hook も
+走らないので local では止まらない。後ろに何があるか (と、何も無い 2 つ) は P1 plan memo の P1a I4
+(Residual accepted gap) を見よ。
 
 Fails OPEN: どんな内部エラーでも no output, exit 0。
 Env overrides (tests): GIT_BYPASS_GUARD_STATE_DIR, GIT_BYPASS_GUARD_EXTRA_SCOPE (os.pathsep 区切り)。
@@ -35,6 +53,15 @@
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
@@ -95,9 +122,16 @@
 
 
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
@@ -138,6 +172,58 @@
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
@@ -173,6 +259,12 @@
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
@@ -186,14 +278,16 @@
     return (hits[0] if hits else None), eff
 
 
-def scan(cmd, cwd, depth=0):
-    """-> (violation, effective_cwd_of_the_violating_git) or (None, None)。cd を追跡する。"""
+def git_segments(cmd, cwd, depth=0):
+    """cmd の git の呼び出しごとに (実効 cwd, 違反 or "") を yield する。
+    実効 cwd の導出 (hook JSON の cwd、先行する `cd`、`git -C <dir>`) と `bash -c` / `sh -c` / `eval` の再帰は
+    ここにだけある。scan() はこの上に作る。"""
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
@@ -204,13 +298,10 @@
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
+            yield eff, (v or seg_env or exported)
         elif prog in SHELLS or prog == "eval":
             inner = None
             if prog == "eval":
@@ -218,9 +309,14 @@
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
+    for eff, v in git_segments(cmd, cwd):
+        if v and in_scope(eff):
+            return v, eff
     return None, None
 
 
```

**Tests** (added to the `DENY` / `ALLOW` lists, as two methods of `T`, and as one class, `Segments`, that loads the
script under test by path, in `git-hook-bypass-guard.test.py`). **This paragraph and the Mutants paragraph below are
the one home of the counts and probe results** (the P1a memo points here): run on 2026-11-23 (PM 950, re-run on the
fix round's `hook-v7.diff` and on `hook-v7-1.diff`, PM 967: 20/20 and the live 12/12) against a scratch copy of the
live file patched with `hook-v7-1.diff` (the block above)
and `test-v6.diff`, both in
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/ci-tools-lane/guardpatch/` (`guardpatch/` below): 20 tests OK; the same
tests against the unpatched file: 40 failures and 5 errors (subtests counted); the live file's own 12 tests against
the patched file: OK. The two diffs, applied with `patch -p1` to a fresh copy of the live files (`9902fe77…0438`),
reproduce the patched files byte for byte. The blind-spot probe of `scan()` (the "`scan()`'s deny is not widened"
paragraph above) on the same patched copy: 21 commands, one per listed shape, each of which would run
`git commit --no-verify -m x` in an elidex worktree; 20 are not denied, and the one denied is
`pushd <outside>; popd; git commit --no-verify -m x` with the hook's cwd in elidex (`pushd` / `popd` are both ignored,
so the hook's cwd stands, which is where `popd` returns); the two plain controls (the commit with the cwd in elidex,
and `cd <elidex> && …` from outside) are denied. Each positive config form was measured to reroute, or to write, the key
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
| `git_segments` cwd | `cd /s/wt && git push origin HEAD` from `/s/repo`; `git -C sub push origin HEAD` from `/r`; `bash -c 'cd /x && git status'`; `sh -c 'git -C y status'`; `eval "cd /x && git status"`; the compound `cd /s/repo && git push origin HEAD:x; cd /s/wt && git push origin HEAD:y` from `/s/repo` | effective cwds `/s/wt`; `/r/sub`; `/x`; `/r/y`; `/x`; `/s/repo` then `/s/wt` |
| `scan()` on the iterator | `git_segments` replaced in the loaded module by one that yields a sentinel violation in scope | `scan("true", <out of scope>)` returns the sentinel |

**Mutants** (each a replacement in a temp copy of the patched file, asserted to occur exactly once, run through the
file's own `GIT_BYPASS_GUARD_SCRIPT` override; on 2026-11-23 (PM 950, re-run on `hook-v7.diff`) all 12 were killed by the tests above. The
`scan()` row replaces a block, the others one line):

| replaced | → | killed by |
|---|---|---|
| `cwd = os.path.join(cwd, os.path.expanduser(d))` (in `git_segments`) | `pass` (the iterator ignores `cd`) | the `cd` / nested / `eval` / compound cwd cases; `cd <elidex> && …` from outside (the existing `test_cd_into_elidex` and the from-outside config method) |
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
the whole diff (`guardpatch/hook-v7-1.diff`, the block above, and `test-v6.diff`) — the `git_segments()` refactor of
`scan()` (PM 922) included — and apply it only after; apply the diff and the test additions, then `python3 ~/.claude/hooks/git-hook-bypass-guard.test.py` → OK, then the mutants
above → each killed. E2E, from a Claude session whose cwd is an elidex worktree: the Bash call
`git config --worktree core.hooksPath /tmp/x` → denied with "git config core.hooksPath" in the reason, and
`git rev-parse --git-path hooks` unchanged afterwards; `git config --get core.hooksPath` → runs. If the hook files
are dotfiles-managed, the edit goes through the dotfiles session.
