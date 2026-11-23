# Plan — P1a cut-over: the bypass-guard patch (global)

## §0 Status

**Where this belongs (PM 916)**: this file is §4.8 of the P1a memo,
`docs/plans/2026-11-agent-neutral-git-guards-p1a.md`, moved here verbatim so that memo stays under 1000 lines. It
is in neither PR's diff: the PM applies it at the P1a cut-over, and the P1a memo's cut-over (its §7.1 step A2, with
E24 of its §7.5) owns applying it, by reference to this file. Every section and id named below without a file
(§3.2, §7.1, I1, I4, …) is the P1a memo's; this file defines none of its own.
Since the move: PM 922 added the `git_segments()` iterator ("One derivation" below). [history] PM 930, PM 936 and
PM 938 revised a pass-through decision layer this patch carried for the `:24` / `:30` hooks; PM 950 (user, option D)
superseded them and removed that layer, so the patch is the deny rules and the iterator only. PM 986 (Codex #677
round 2) added the reading of abbreviated long options ("Long options as git reads them" below): `hook-v7-3.diff`.

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

**Long options as git reads them (Codex #677 round 2, PM 986).** git's parse-options takes any unique prefix of a
long option for the option (`git config --rename parking core` is `--rename-section`, measured: the hooks path moves
to `/alt`), and `--no-<opt>` for a negatable one; a prefix of several is refused as ambiguous, a name of none as
unknown. The patch reads long options the same way, through one function, `resolve_long(token, names)`: the exact
name, else the one name the token is a prefix of, else "ambiguous" / "unknown". Its two callers:

- `git config`: `names` is git's own list for the mode — the subcommand, which git reads only as the first argument
  (measured: `git config --worktree set a.b c` is the legacy mode, "key does not contain a section: set"), or the
  legacy mode. The lists (`CONFIG_LONG`) are taken from git 2.56.1: `git config <sub> --git-completion-helper-all`
  for each of `set` / `unset` / `rename-section` / `remove-section` / `get` / `list` / `edit`, and the `-h` of the
  legacy mode, `git config --get -h` (`--[no-]x` read as `x` and `no-x`). The canonical names then feed the same
  classification as before. **Fail-safe**: a long option that resolves to no name (ambiguous or unknown) denies the
  call when any argument (or a `--opt=` value) is section `core` or a key in it (`core` / `core.…`, any case); without
  such an argument the call is left alone (other keys are out of scope). git refuses such a call itself while the
  lists match git; the rule is for the lists drifting from git (a new option, a new git), and the lists are re-derived
  on a git upgrade.
- the hooked subcommands' `--no-verify`: matched against `no-verify` alone. Every prefix of `--no-verify` is, to
  git 2.56.1, either `--no-verify` or ambiguous with another option of the subcommand (`--no-verbose`,
  `--no-verify-signatures`, the negations behind `--n` / `--no` / `--no-`; measured for commit, push, merge, pull,
  rebase and am), so the reduced list gives the same verdict as the full one under the fail-safe: deny.

Not a general argv parser: short options (git does not abbreviate them) and the git global options before the
subcommand (`-c`, `--config-env`; git.c compares them exactly, measured: `git --config-e=…` is "unknown option") are
read as before. `commit_value_skips` still compares the value-taking commit options exactly: an abbreviated one
(`--mess x`) is not skipped, so its value is read as an option — the deny side only.

```diff
--- a/git-hook-bypass-guard.py
+++ b/git-hook-bypass-guard.py
@@ -4,15 +4,40 @@
 deny する形 (git の呼び出し 1 つごとに判定):
   - `-c core.hooksPath=...` / `-c core.hooksPath ...` / `-ccore.hooksPath=...` /
     `--config-env=core.hooksPath=...` (キー名は大文字小文字を区別しない。`git -C dir` 付きも)
-  - commit / push / merge / pull / rebase / cherry-pick / am / revert の `--no-verify` (--no-ve... の省略形も)、
+  - commit / push / merge / pull / rebase / cherry-pick / am / revert の `--no-verify` (git が受ける省略形も)、
     `git commit -n` (-an / -nm のような束ね形も)
+  - `git config` で core.hooksPath を書く形すべて: 旧形式 (`git config [--local|--worktree|--global|--system|
+    -f <file>|--file[=]<file>] core.hooksPath <dir>`、option は位置引数の後ろでもよい)、`--add` / `--replace-all` /
+    `--unset` / `--unset-all`、subcommand `set` / `unset`、section ごとの `--remove-section` / `--rename-section` /
+    `remove-section` / `rename-section` (section `core` を消す・`core` から/へ rename する)、
+    editor で開く `-e` / `--edit` / `edit` (どの key も書ける)。
+    読む形 (`--get` 系、`-l` / `--list`、`get` / `list`、key だけの旧形式 `git config core.hooksPath`) は通す。
+    `--file` の先が elidex の中なら、cwd が外でも deny。
+    読めない long option (未知・曖昧な省略形) があり、どれかの引数が section `core` か `core.` で始まる key なら deny
+  - long option の読み: git の parse-options と同じく、正確な名前か、ただ 1 つの名前の prefix (`--rename` =
+    `--rename-section`、`--no-<opt>` の否定も) を正準名に直してから判定する (resolve_long)。`git config` は mode ごとの
+    git 自身の option 一覧 (CONFIG_LONG) と照合し、`--no-verify` は `no-verify` の prefix なら deny (git はそれを
+    `--no-verify` と読むか、`--no-verbose` 等と曖昧として拒む)。git の global option (`--config-env` 等) は git が
+    省略形を受けないので正確な名前だけ
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
@@ -35,6 +60,33 @@
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
+# long options of `git config` per mode (None = the legacy mode), as git 2.56.1 lists them -- re-derive on a git
+# upgrade: `git config <sub> --git-completion-helper-all` for each subcommand, `git config --get -h` for the legacy
+# mode (`--[no-]x` = x and no-x). resolve_long() matches a token against the mode's list.
+_LOC = "global system local worktree file blob no-global no-system no-local no-worktree no-file no-blob "
+_TYPE = "type bool int bool-or-int bool-or-str path expiry-date no-type "
+_SHOW = ("null name-only show-origin show-scope show-names includes "
+         "no-null no-name-only no-show-origin no-show-scope no-show-names no-includes ")
+CONFIG_LONG = {k: tuple(v.split()) for k, v in {
+    None: _LOC + _TYPE + _SHOW + "get get-all get-regexp get-urlmatch replace-all add unset unset-all rename-section "
+          "remove-section list edit get-color get-colorbool default comment fixed-value no-default no-comment "
+          "no-fixed-value",
+    "set": _LOC + _TYPE + "all value fixed-value comment append no-all no-value no-fixed-value no-comment no-append",
+    "unset": _LOC + "all value fixed-value no-all no-value no-fixed-value",
+    "rename-section": _LOC, "remove-section": _LOC, "edit": _LOC,
+    "get": _LOC + _TYPE + _SHOW + "all regexp value fixed-value url default no-all no-regexp no-value "
+           "no-fixed-value no-url no-default",
+    "list": _LOC + _TYPE + _SHOW,
+}.items()}
+# those that take a value (`--opt=value` is one token, else the next token); `-f` is handled with the short clusters
+CONFIG_VALUE_OPTS = {"file", "blob", "type", "default", "comment", "value", "url"}
 
 
 def state_dir():
@@ -95,19 +147,44 @@
 
 
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
     return s.split("=", 1)[0].strip().lower() == "core.hookspath"
 
 
+def resolve_long(tok, names):
+    """`--name[=value]` -> the long option git's parse-options takes it for: the exact name, else the one name it is a
+    prefix of; "?" if it is a prefix of several (git: ambiguous), "" if of none (git: unknown)."""
+    n = tok[2:].split("=", 1)[0]
+    if n in names:
+        return n
+    c = [x for x in names if x.startswith(n)]
+    return c[0] if len(c) == 1 else ("?" if c else "")
+
+
 def long_is_no_verify(tok):
-    return tok.startswith("--no-ve") and "no-verify".startswith(tok[2:])
+    # matched against `no-verify` alone: each prefix of it is, to git, either --no-verify or ambiguous with another
+    # option (--no-verbose, --no-verify-signatures; git 2.56.1 refuses it) -- denied either way (fail-safe)
+    return tok.startswith("--") and len(tok) > 2 and resolve_long(tok, ("no-verify",)) == "no-verify"
 
 
+def core_name(a):
+    """an argument (or a `--opt=` value) naming section `core` or a key in it, case-insensitive"""
+    a = (a.split("=", 1)[1] if a.startswith("--") and "=" in a else a).lower()
+    return a == "core" or a.startswith("core.")
+
+
 def commit_value_skips(args):
     """commit の引数で -m 等の「値」にあたる位置の index 集合。値は option でも message でもない。"""
     skip = set()
@@ -138,6 +215,65 @@
             if ch in COMMIT_VALUE_SHORT:
                 break
     return False
+
+
+def config_hookspath_write(args, cwd):
+    """`git config` の後ろの token -> (violation or None, `--file` の先 or None)。
+    subcommand は最初の引数だけ (git もそう読む)。位置引数は option とその値を除いた全部 (git は option を位置引数の
+    後ろにも置ける)。long option は resolve_long で mode の一覧 (CONFIG_LONG) の正準名に直す。直せない (未知・曖昧)
+    ものがあれば、どれかの引数が core の section / key の時 deny (fail-safe: 一覧が git とずれても core は通さない)。"""
+    sub = args[0] if args and args[0] in CONFIG_SUBCMDS else None
+    pos, acts, target, unknown, i = [], set(), None, False, 1 if sub else 0
+    while i < len(args):
+        a = args[i]
+        if a == "--":
+            pos.extend(args[i + 1:])
+            break
+        o = resolve_long(a, CONFIG_LONG[sub]) if a.startswith("--") else None
+        if o in CONFIG_VALUE_OPTS:
+            v = a.split("=", 1)[1] if "=" in a else (args[i + 1] if i + 1 < len(args) else None)
+            if o == "file":
+                target = v
+            i += 1 if "=" in a else 2
+            continue
+        if o in ("", "?"):
+            unknown = True
+        elif o is not None:
+            acts.add("--" + o)
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
+    names = pos
+    if unknown and any(map(core_name, args)):
+        return "git config (読めない option と core)", target
+    if sub == "edit" or acts & {"-e", "--edit"}:
+        return "git config --edit", target
+    if sub in ("rename-section", "remove-section") or acts & {"--rename-section", "--remove-section"}:
+        if names and names[0].lower() == "core":
+            return "git config remove/rename-section core", target
+        if (sub == "rename-section" or "--rename-section" in acts) and len(names) > 1 and names[1].lower() == "core":
+            return "git config rename-section into core", target
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
@@ -173,6 +309,12 @@
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
@@ -186,14 +328,16 @@
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
@@ -204,13 +348,10 @@
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
@@ -218,9 +359,14 @@
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
the one home of the counts and probe results** (the P1a memo points here): run on 2026-11-23 (PM 950; re-run on
`hook-v7.diff`, `hook-v7-1.diff` (PM 967) and `hook-v7-2.diff` (the rename into `core`, Codex #677 P2); last run on
`hook-v7-3.diff`, PM 986) against a scratch copy of the live file patched with `hook-v7-3.diff` (the block above) and
`test-v7-3.diff`, both in
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/ci-tools-lane/guardpatch/` (`guardpatch/` below): 20/20 tests OK; the same
tests against the unpatched file: 53 failures and 5 errors (subtests counted); against `hook-v7-2.diff`: 12 failures,
all of them cases new in `test-v7-3.diff` (11 `DENY` entries and the from-outside method, which stops at its first new
`--fil` line); `test-v7-2.diff`'s tests against the patched file: 20/20 OK; the live file's own 12 tests against the
patched file: 12/12 OK. The two diffs, applied with `patch -p1` to a fresh copy of the live files (`9902fe77…0438`),
reproduce the patched files byte for byte. The option lists: `guardpatch/config-opts-v7-3.py <scratch repo> --probe
<patched hook>` asserts that `CONFIG_LONG` equals git's lists (the commands of "Long options as git reads them") and
runs `git config [<sub>] --<p>` and `--<p>=x` for every prefix `p` of every listed name, plus nine unknown names, in
the eight modes: 1285 prefixes, git's verdict (the name it reports, ambiguous, or unknown) equal to `resolve_long`'s
for 1285/1285. (git compares `--n` / `--no` / `--no-` against the negations only without an `=`: `--no=x` is
"unknown" where `--no` is ambiguous; both are unresolved, so the probe takes the verdict from the bare form.) The
blind-spot probe of `scan()` (the "`scan()`'s deny is not widened" paragraph above), run on the `hook-v7-1.diff` copy
and not re-run since (`hook-v7-2.diff` changes only `config_hookspath_write`, which none of these commands reaches;
`hook-v7-3.diff` changes, besides it, only which prefixes `long_is_no_verify` takes, and every probe command carries
the full `--no-verify`): 21 commands, one per listed shape, each of which would run
`git commit --no-verify -m x` in an elidex worktree; 20 are not denied, and the one denied is
`pushd <outside>; popd; git commit --no-verify -m x` with the hook's cwd in elidex (`pushd` / `popd` are both ignored,
so the hook's cwd stands, which is where `popd` returns); the two plain controls (the commit with the cwd in elidex,
and `cd <elidex> && …` from outside) are denied. Each positive config form was measured to reroute, or to write, the key
(§3.2; the rename into `core` on 2026-11-23, git 2.56.1, `HOME` / `GIT_CONFIG_GLOBAL=/dev/null` /
`GIT_CONFIG_NOSYSTEM=1` isolated: with `parking.hooksPath /alt` set, `--rename-section parking core`,
`config rename-section parking Core` and `--file .git/config --rename-section parking core` move
`--git-path hooks` to `/alt`, as does `--rename-section parking.sub core` from `parking.sub.hooksPath`;
`--rename-section parking other` and `parking core.sub` — which makes `[core "sub"]` — do not; the abbreviated forms
on 2026-11-23 in the same isolation, each in a fresh repository with the key it acts on set, table below):

| form (cwd in scope) | git 2.56.1 | `hook-v7-2.diff` | `hook-v7-3.diff` |
|---|---|---|---|
| `config --rename parking core`, `--rename-s parking core`, `--ren parking Core` | accepted: hooks path → `/alt` | allow | deny |
| `config --rem core`; `--unset-a core.hooksPath`; `--fil=.git/config --unset-a core.hooksPath` | accepted: `/alt` → default | allow | deny |
| `config --ed` (an editor that appends `[core] hooksPath = /alt`) | accepted: → `/alt` | allow | deny |
| `config rename-section --fil .git/config parking core` | accepted: → `/alt` | allow | deny |
| `config --loc core.hooksPath /alt`; `--fil .git/config core.hooksPath /alt`; `set --loc …`; `unset --a core.hooksPath`; `--no-glo --loc core.hooksPath /alt` | accepted: the key written | deny | deny |
| `config --uns core.hooksPath`; `--re core zz`; `--fi .git/config core.hooksPath /alt` | ambiguous (exit 129) | allow; allow; deny | deny (fail-safe) |
| `config --bogus core.hooksPath` | unknown (exit 129) | allow | deny (fail-safe) |
| `config --e`; `--bogus user.name a` | ambiguous; unknown | allow | allow (no `core` argument) |
| `config --no-glo --get core.hooksPath`; `--get-a core.hooksPath`; `--ren parking other` | accepted: a read; a read; no `core` | allow | allow |
| `config --worktree set core.hooksPath /alt` | legacy mode: "key does not contain a section: set" | deny | deny |
| `am --no-v` (a failing `pre-applypatch`) | accepted: the hook skipped | allow | deny |
| `commit` / `push` / `rebase` `--no-veri`, `--no-verif` | accepted (commit, push with a failing hook: the hook skipped) | deny | deny |
| `commit` / `push` / `merge` / `pull` / `rebase` `--no-v`, `--no-ve`, `--no-ver`; `merge` / `pull` `--no-veri`, `--no-verif`; `--n`, `--no`, `--no-` of those five and `am` | ambiguous (exit 129) | `--no-ve…` deny, the shorter allow | deny (fail-safe) |
| `git --config-e=core.hooksPath=/alt …` | unknown option (git.c compares exactly) | allow | allow |
| `cherry-pick` / `revert` `--no-verify` | not an option (git prints the usage) | deny | deny (unchanged) |

| kind | commands (cwd in scope unless stated) | expected |
|---|---|---|
| legacy write | `git config core.hooksPath /alt`; with `--local`, `--worktree`, `--global`, `--system`; `git config core.hooksPath /alt --worktree`; `--worktree core.HOOKSPATH`, `--worktree CORE.hookspath`; `--add --worktree`; `--replace-all`; `--type=path --worktree`, `--type path --worktree`; `--comment x`; `--worktree -- core.hooksPath /alt` | deny |
| legacy unset / section | `--unset`, `--unset-all core.hooksPath`; `--remove-section core`; `--rename-section core zz`; `--rename-section parking core` | deny |
| subcommands | `config set --worktree`, `set --all`, `set --append`; `config unset`, `unset --all`; `remove-section core`; `rename-section Core zz`; `rename-section parking Core` | deny |
| file target | `--file .git/config.worktree`, `-f …`, `-f.git/…`, `--file=…`; from outside: `--file <elidex>/.git/config core.hooksPath /alt`, `-f <elidex>/.git/config --unset core.hooksPath`, `--file <elidex>/.git/config --rename-section parking core` | deny |
| editor | `git config --edit --worktree`, `-e`, `config edit` | deny (an editor can write any key) |
| reach | `git -C <elidex> config --worktree …`; `cd <elidex> && git config --worktree …` from outside; `bash -c '…'`; `git status && git config …` | deny |
| env | `GIT_CONFIG_PARAMETERS="'core.hooksPath'='/alt'" git commit -m x`; `env GIT_CONFIG_PARAMETERS="'core.hookspath=/alt'" git push origin HEAD`; `export GIT_CONFIG_PARAMETERS="'CORE.HOOKSPATH'='/alt'"; git commit -m x` | deny |
| abbreviated (a unique prefix) | `git am --no-v x.patch`; `config --rename parking core`, `--ren parking Core`; `--rem core`; `--unset-a core.hooksPath`; `--ed`; `rename-section --fil .git/config parking core`; from outside: `--fil <elidex>/.git/config core.hooksPath /alt`, `--fil=<elidex>/.git/config --unset-a core.hooksPath` | deny |
| unresolved with a `core` argument (fail-safe) | `config --uns core.hooksPath`, `--re core zz` (ambiguous); `--bogus core.hooksPath` (unknown); `git push --no-v origin HEAD` (an ambiguous prefix of `--no-verify`) | deny |
| abbreviated or unresolved, no `core` write (negative) | `config --get-a core.hooksPath`; `--no-glo --get core.hooksPath`; `--ren parking other`; `--e` (ambiguous); `--bogus user.name a` (unknown); `git merge --no-verify-signatures topic` | allow |
| reads (negative) | `git config core.hooksPath`, `--get`, `--get-all`, `--show-origin --get`, `--worktree core.hooksPath`, `--get-regexp core.hooks`, `--list`, `-l --show-scope`, `config get`, `config get --all`, `config list` | allow |
| other keys (negative) | `--worktree user.name a`; `config set core.editor vim`; `--unset user.email`; `--remove-section alias`; `--rename-section parking other`; `--rename-section parking core.sub`; `--worktree core.hooks /alt`; `--worktree core.hooksPathX /alt`; `GIT_CONFIG_PARAMETERS="'user.name'='a'" git commit -m x` | allow |
| text, not a command (negative) | `git commit -m 'git config core.hooksPath /alt'`; `git log --grep 'git config --worktree core.hooksPath'` | allow |
| out of scope (negative) | every `DENY` entry from `/private/tmp` (the existing loop); `git config --file /private/tmp/x core.hooksPath /alt` from `/private/tmp` | allow |
| `git_segments` cwd | `cd /s/wt && git push origin HEAD` from `/s/repo`; `git -C sub push origin HEAD` from `/r`; `bash -c 'cd /x && git status'`; `sh -c 'git -C y status'`; `eval "cd /x && git status"`; the compound `cd /s/repo && git push origin HEAD:x; cd /s/wt && git push origin HEAD:y` from `/s/repo` | effective cwds `/s/wt`; `/r/sub`; `/x`; `/r/y`; `/x`; `/s/repo` then `/s/wt` |
| `scan()` on the iterator | `git_segments` replaced in the loaded module by one that yields a sentinel violation in scope | `scan("true", <out of scope>)` returns the sentinel |

**Mutants** (each a replacement in a temp copy of the patched file, asserted to occur exactly once, run through the
file's own `GIT_BYPASS_GUARD_SCRIPT` override, one at a time; on 2026-11-23 (PM 950, re-run on `hook-v7.diff` and
`hook-v7-2.diff`: 13/13; last run on `hook-v7-3.diff` / `test-v7-3.diff`, PM 986, by `guardpatch/mutants-v7-3.py
<patched hook> <patched test>`) all 22 were killed by the tests above (22/22). The `scan()` row replaces a block, the
others one line; the last nine are new in `hook-v7-3.diff`):

| replaced | → | killed by |
|---|---|---|
| `cwd = os.path.join(cwd, os.path.expanduser(d))` (in `git_segments`) | `pass` (the iterator ignores `cd`) | the `cd` / nested / `eval` / compound cwd cases; `cd <elidex> && …` from outside (the existing `test_cd_into_elidex` and the from-outside config method) |
| `yield from git_segments(inner, cwd, depth + 1)` | `pass` (recursion dropped) | the nested and `eval` cwd cases, the `bash -c` `DENY` entries |
| the body of `scan()` | a loop over a full copy of `git_segments` (a divergent copy; the runner's form: a second name bound to the original `git_segments` at load, which the test's replacement does not reach) | only the `scan()`-on-the-iterator case: the copy behaves identically, so no behavioural case can see it; the test replaces `git_segments` and observes that `scan()` follows it |
| `if not any(n.lower() == "core.hookspath" for n in names):` | exact-case compare | `core.HOOKSPATH` forms |
| `if k.upper() == "GIT_CONFIG_PARAMETERS" and` | `if False and` | the env rows |
| `if target and not in_scope(eff) and in_scope(target):` | `if False:` | the from-outside `--file` rows |
| `if sub is None and len(names) == 1 and not acts & CONFIG_WRITE_ACTIONS:` | drop `len(names) == 1` | every legacy write |
| `CONFIG_READ_SUBCMDS = {"list", "get"}` | add `"set"` | the `set` rows |
| `if sub == "config":` | another name | every config row |
| `if names and names[0].lower() == "core":` | exact-case compare | `rename-section Core zz` |
| `if (sub == "rename-section" or "--rename-section" in acts) and len(names) > 1 and names[1].lower() == "core":` | `if False:` (the destination is not checked) | `--rename-section parking core`, `rename-section parking Core`, the from-outside `--file … --rename-section parking core` |
| `if sub == "edit" or acts & {"-e", "--edit"}:` | `if sub == "edit":` | `--edit`, `-e` |
| `pos.extend(args[i + 1:])` | `pass` | `--worktree -- core.hooksPath /alt` |
| `o = resolve_long(a, CONFIG_LONG[sub]) if a.startswith("--") else None` | the token's own name (no normalization) | the abbreviated and the unresolved-with-`core` `config` rows (`--rename`, `--ren`, `--rem`, `--unset-a`, `--ed`, `rename-section --fil …`, `--uns`, `--re core zz`, `--bogus core.hooksPath`), the from-outside `--fil` |
| `return c[0] if len(c) == 1 else ("?" if c else "")` | `return "?" if c else ""` (exact names only) | `--ed`, `am --no-v`, `push --no-v`, `commit --no-ver`, the from-outside `--fil`; the allowed `--get-a core.hooksPath` and `--no-glo --get core.hooksPath` |
| `o = resolve_long(a, CONFIG_LONG[sub])` | `CONFIG_LONG[None]` (the legacy list in every mode) | `config get --all core.hooksPath` (allowed: `--all` is unknown to the legacy mode) |
| `_LOC`'s string | without its `no-` names | `--no-glo --get core.hooksPath` (allowed) |
| `if unknown and any(map(core_name, args)):` | `if False:` (no fail-safe) | `--uns core.hooksPath`, `--re core zz`, `--bogus core.hooksPath` |
| `return a == "core" or a.startswith("core.")` (in `core_name`) | `return a == "core"` | `--uns core.hooksPath`, `--bogus core.hooksPath` |
| `target = v` (a value-taking option that resolves to `file`) | `pass` | the from-outside method (`--fil <elidex>/.git/config …`) |
| `return tok.startswith("--") and len(tok) > 2 and resolve_long(tok, ("no-verify",)) == "no-verify"` | `hook-v7-2.diff`'s `tok.startswith("--no-ve") and "no-verify".startswith(tok[2:])` | `am --no-v`, `push --no-v` |
| the same line | without `tok.startswith("--") and len(tok) > 2 and` | the allowed `commit` / `push` rows (each plain argument, `x`, would read as a prefix of `no-verify`) |

**The PM's application and E2E line** (global: it acts on every session on this machine at once, other
repositories' sessions included, `~/.claude` CLAUDE.md "変更の blast radius"): back up both files
(`cp -p ~/.claude/hooks/git-hook-bypass-guard{,.test}.py <backup dir>/`, the path into the PM's log); show the user
the whole diff (`guardpatch/hook-v7-3.diff`, the block above, and `test-v7-3.diff`) — the `git_segments()` refactor of
`scan()` (PM 922) included — and apply it only after; apply the diff and the test additions, then `python3 ~/.claude/hooks/git-hook-bypass-guard.test.py` → OK, then the mutants
above → each killed. E2E, from a Claude session whose cwd is an elidex worktree: the Bash call
`git config --worktree core.hooksPath /tmp/x` → denied with "git config core.hooksPath" in the reason, and
`git rev-parse --git-path hooks` unchanged afterwards; `git config --get core.hooksPath` → runs. If the hook files
are dotfiles-managed, the edit goes through the dotfiles session.
