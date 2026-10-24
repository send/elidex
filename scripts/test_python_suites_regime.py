"""Offline-regime pins (T-net, §6.1 G1-G14) for scripts/python-suites.sh.

The fixture member sources (G1-G14 classes) are string constants here: each is
written into a scratch repository and run by the copied driver. Shared helpers:
`python_suites_fixtures.py`.
"""

from __future__ import annotations

import ast
import os
import socket
import sys
import threading
import unittest

from python_suites_fixtures import (
    PIN_FILES,
    REGIME_KEYS,
    REPO,
    UNGUARDED_LAUNCHES,
    DriverCase,
    PASSING,
    _launch,
    base_env,
    regime_changes,
)

# ---------------------------------------------------------------------------
# T-net: the offline regime (§6.1 G1-G14).


class Listener:
    """A loopback listener in this process (bind/listen/accept are not guarded):
    reads what the client sends, answers a fixed HTTP/1.0 response, closes."""

    def __init__(self, case, family=socket.AF_INET, host="127.0.0.1"):
        self.sock = socket.socket(family, socket.SOCK_STREAM)
        self.sock.bind((host, 0))
        self.sock.listen(32)
        self.sock.settimeout(0.2)  # only bounds how long close() can wait
        self.port = self.sock.getsockname()[1]
        self.stop = False
        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()
        case.addCleanup(self.close)

    def _serve(self):
        while not self.stop:
            try:
                conn, _ = self.sock.accept()
            except OSError:
                continue
            try:
                conn.settimeout(5)
                conn.recv(65536)  # the request (or a TLS hello, or nothing): answer at once
                conn.sendall(b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            except OSError:
                pass
            finally:
                conn.close()

    def close(self):
        self.stop = True
        self.sock.close()  # first, so a blocked accept() fails now
        self.thread.join(5)


G1_MEMBER = """
import http.client
import os
import ssl
import unittest
import urllib.error
import urllib.request

PORT = int(os.environ["PYSUITES_PIN_PORT"])
DIRECT = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class G1(unittest.TestCase):
    def test_http_client(self):
        c = http.client.HTTPConnection("127.0.0.1", PORT, timeout=10)
        c.connect()
        c.close()

    def test_urlopen_http(self):
        with DIRECT.open("http://127.0.0.1:%d/" % PORT, timeout=10) as r:
            self.assertEqual(r.read(), b"ok")

    def test_urlopen_https(self):
        # TCP connects, then TLS fails against the plain-HTTP listener: proof
        # the connect happened. Under the regime it fails at the socket layer,
        # before TLS.
        with self.assertRaises(urllib.error.URLError) as cm:
            DIRECT.open("https://127.0.0.1:%d/" % PORT, timeout=10)
        self.assertIsInstance(cm.exception.reason, (ssl.SSLError, ConnectionError))
"""

# Exit 0: connected. 42: refused by the offline regime. 3: any other failure.
CONNECT_CHILD = (
    "import socket, sys\n"
    "try:\n"
    "    s = socket.socket(); s.connect(('127.0.0.1', int(sys.argv[1]))); s.close()\n"
    "except OSError as e:\n"
    "    sys.exit(42 if 'python-suites offline regime' in str(e) else 3)\n"
)

RAW_CONNECT_CHILD = (
    "import os, socket; s = socket.socket(); "
    "s.connect(('127.0.0.1', int(os.environ['PYSUITES_PIN_PORT']))); s.close()"
)

G2_MEMBER = """
import os
import subprocess
import sys
import tempfile
import unittest

CHILD = %r


class G2(unittest.TestCase):
    def test_python_child(self):
        r = subprocess.run([sys.executable, "-c", CHILD])
        self.assertEqual(r.returncode, 0)

    def test_shebang_child(self):
        d = tempfile.mkdtemp()
        p = os.path.join(d, "tool")
        with open(p, "w") as f:
            f.write("#!/usr/bin/env python3\\n" + CHILD + "\\n")
        os.chmod(p, 0o755)
        r = subprocess.run([p])
        self.assertEqual(r.returncode, 0)
""" % RAW_CONNECT_CHILD

G3_MEMBER = """
import os
import socket
import unittest


class G3(unittest.TestCase):
    def test_connect_ex(self):
        s = socket.socket()
        try:
            self.assertEqual(s.connect_ex(("127.0.0.1", int(os.environ["PYSUITES_PIN_PORT"]))), 0)
        finally:
            s.close()
"""

G4_MEMBER = """
import os
import socket
import unittest


class G4(unittest.TestCase):
    def test_connect_inet6(self):
        s = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
        try:
            s.connect(("::1", int(os.environ["PYSUITES_PIN_PORT6"])))
        finally:
            s.close()
"""

G5_MEMBER = """
import os
import socket
import unittest

PORT = int(os.environ["PYSUITES_PIN_PORT"])


class G5(unittest.TestCase):
    def test_getaddrinfo(self):
        self.assertTrue(socket.getaddrinfo("localhost", PORT))

    def test_gethostbyname(self):
        self.assertTrue(socket.gethostbyname("localhost"))

    def test_gethostbyname_ex(self):
        self.assertTrue(socket.gethostbyname_ex("localhost"))

    def test_gethostbyaddr(self):
        self.assertTrue(socket.gethostbyaddr("127.0.0.1"))

    def test_getnameinfo(self):
        flags = socket.NI_NUMERICHOST | socket.NI_NUMERICSERV
        self.assertTrue(socket.getnameinfo(("127.0.0.1", PORT), flags))
"""

G5_TOPLEVEL_MEMBER = """
import socket
import unittest

ADDRS = socket.getaddrinfo("localhost", 80)


class G5Top(unittest.TestCase):
    def test_imported(self):
        self.assertTrue(ADDRS)
"""

G6_INHERITED = {
    "NO_PROXY": "*",
    "no_proxy": "*",
    "http_proxy": "http://proxy.invalid:3128",
    "https_proxy": "http://proxy.invalid:3128",
    "XDG_CACHE_HOME": "/nonexistent/python-suites-pin-xdg",
}

G6_MEMBER = """
import subprocess
import unittest

INHERITED = %r


def child_env():
    out = subprocess.run(["/bin/sh", "-c", "env"], stdout=subprocess.PIPE).stdout.decode()
    return dict(l.split("=", 1) for l in out.splitlines() if "=" in l)


class G6(unittest.TestCase):
    def check(self, key):
        self.assertEqual(child_env().get(key), INHERITED[key])

    def test_NO_PROXY(self):
        self.check("NO_PROXY")

    def test_no_proxy(self):
        self.check("no_proxy")

    def test_http_proxy(self):
        self.check("http_proxy")

    def test_https_proxy(self):
        self.check("https_proxy")

    def test_XDG_CACHE_HOME(self):
        self.check("XDG_CACHE_HOME")
""" % G6_INHERITED

G7_COUNT_MEMBER = """
import socket
import subprocess
import sys
import unittest

MARK = "_python_suites_guard_installs"


class G7(unittest.TestCase):
    def test_count_in_loader(self):
        self.assertEqual(getattr(socket, MARK, 0), 1)

    def test_count_in_child(self):
        out = subprocess.run(
            [sys.executable, "-c", "import socket; print(getattr(socket, %r, 0))" % MARK],
            stdout=subprocess.PIPE,
        ).stdout
        self.assertEqual(out.strip(), b"1")
"""

RAW_CONNECT_MEMBER = """
import os
import socket
import unittest


class Raw(unittest.TestCase):
    def test_raw_connect(self):
        s = socket.socket()
        try:
            s.connect(("127.0.0.1", int(os.environ["PYSUITES_PIN_PORT"])))
        finally:
            s.close()
"""

G7_NESTED_MEMBER = """
import os
import subprocess
import unittest


class Nested(unittest.TestCase):
    def test_inner_driver_run(self):
        inner = os.environ["PYSUITES_PIN_INNER"]
        p = subprocess.run(
            ["bash", os.path.join(inner, "scripts", "python-suites.sh")],
            cwd=inner,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        out = p.stdout.decode()
        self.assertNotEqual(p.returncode, 0, out)
        fails = [l for l in out.splitlines() if l.startswith("python-suites: FAIL")]
        self.assertTrue(any("r/test_raw.py::Raw.test_raw_connect" in l for l in fails), out)
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_loader", out)
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_child", out)
"""

G8_MARK_MEMBER = """
import sys
import unittest


class G8(unittest.TestCase):
    def test_chained_sitecustomize_ran(self):
        self.assertEqual(getattr(sys, "_python_suites_pin_site_marker", None), "set")
"""

G10_MEMBER = """
import os
import socket
import subprocess
import sys
import tempfile
import unittest

CONNECT = @CONNECT@


class G10(unittest.TestCase):
    def setUp(self):
        self.srv = socket.socket()
        self.srv.bind(("127.0.0.1", 0))
        self.srv.listen(8)
        self.addCleanup(self.srv.close)
        self.port = str(self.srv.getsockname()[1])

    def child(self, argv, env=None):
        return subprocess.run(argv, env=env).returncode

    # In the class (the stated limit is real): these CAN reach the network.
    def test_pythonpath_override_is_in_the_class(self):
        d = tempfile.mkdtemp()
        env = {**os.environ, "PYTHONPATH": d}
        self.assertEqual(self.child([sys.executable, "-c", CONNECT, self.port], env), 0)

    def test_parent_environ_mutation_is_in_the_class(self):
        old = os.environ.get("NO_PROXY")
        os.environ["NO_PROXY"] = "*"
        try:
            out = subprocess.run(
                ["/bin/sh", "-c", 'printf %s "$NO_PROXY"'], stdout=subprocess.PIPE
            ).stdout
            self.assertEqual(out, b"*")
        finally:
            if old is None:
                del os.environ["NO_PROXY"]
            else:
                os.environ["NO_PROXY"] = old

    # Outside the class: these CANNOT.
    def test_execve_with_environ_is_outside_the_class(self):
        hop = "import os, sys; os.execve(sys.executable, [sys.executable, '-c', sys.argv[1], sys.argv[2]], os.environ)"
        self.assertEqual(self.child([sys.executable, "-c", hop, CONNECT, self.port]), 42)

    def test_non_regime_key_change_is_outside_the_class(self):
        env = {**os.environ, "X": "1"}
        self.assertEqual(self.child([sys.executable, "-c", CONNECT, self.port], env), 42)

    def test_control_non_network_child_passes(self):
        self.assertEqual(self.child([sys.executable, "-c", "pass"]), 0)
""".replace("@CONNECT@", repr(CONNECT_CHILD))

G11_MUTATOR = """
import os
import unittest


class A(unittest.TestCase):
    def test_mutates_regime_keys(self):
        os.environ["NO_PROXY"] = "*"
        os.environ["PYTHONPATH"] = "/nonexistent"
        os.putenv("https_proxy", "http://proxy.invalid:3128")
"""

G11_CHECKER = """
import os
import socket
import subprocess
import sys
import unittest

CONNECT = @CONNECT@


class B(unittest.TestCase):
    def test_regime_restored_for_the_next_member(self):
        out = subprocess.run(["/bin/sh", "-c", "env"], stdout=subprocess.PIPE).stdout.decode()
        env = dict(l.split("=", 1) for l in out.splitlines() if "=" in l)
        self.assertNotIn("NO_PROXY", env)
        self.assertTrue(env["https_proxy"].startswith("http://127.0.0.1:"), env["https_proxy"])

    def test_child_still_guarded(self):
        srv = socket.socket()
        srv.bind(("127.0.0.1", 0))
        srv.listen(8)
        try:
            r = subprocess.run([sys.executable, "-c", CONNECT, str(srv.getsockname()[1])])
            self.assertEqual(r.returncode, 42)
        finally:
            srv.close()
""".replace("@CONNECT@", repr(CONNECT_CHILD))

G11_GUARD_UNDO = """
import socket
import unittest


class A(unittest.TestCase):
    def test_undoes_the_guard(self):
        socket.socket.connect = socket._socket.socket.connect
"""

G11_GUARD_CONNECT = """
import os
import socket
import unittest


class B(unittest.TestCase):
    def test_connect_after_a_member_undid_the_guard(self):
        s = socket.socket()
        try:
            s.connect(("127.0.0.1", int(os.environ["PYSUITES_PIN_PORT"])))
        finally:
            s.close()
"""


class TestOfflineRegimePins(DriverCase, unittest.TestCase):
    def plant(self, files, extra_env, names):
        """Each plant: green without the regime (G9 baseline), red with it."""
        s = self.scratch(files)
        for member in files:
            self.assertBaselineGreen(s, member, extra_env)
        return self.assertRed(s.run(extra_env), *names)

    def test_G1_in_process_connection_is_red(self):
        lst = Listener(self)
        self.plant(
            {"g1/test_g1.py": G1_MEMBER},
            {"PYSUITES_PIN_PORT": str(lst.port)},
            [
                "ERROR g1/test_g1.py::G1.test_http_client",
                "ERROR g1/test_g1.py::G1.test_urlopen_http",
                "g1/test_g1.py::G1.test_urlopen_https",
            ],
        )

    def test_G2_python_child_and_shebang_child_are_red(self):
        lst = Listener(self)
        self.plant(
            {"g2/test_g2.py": G2_MEMBER},
            {"PYSUITES_PIN_PORT": str(lst.port)},
            ["FAILED g2/test_g2.py::G2.test_python_child", "FAILED g2/test_g2.py::G2.test_shebang_child"],
        )

    def test_G3_connect_ex_is_red(self):
        lst = Listener(self)
        self.plant(
            {"g3/test_g3.py": G3_MEMBER},
            {"PYSUITES_PIN_PORT": str(lst.port)},
            ["ERROR g3/test_g3.py::G3.test_connect_ex"],
        )

    def test_G4_inet6_connect_is_red(self):
        lst = Listener(self, socket.AF_INET6, "::1")
        self.plant(
            {"g4/test_g4.py": G4_MEMBER},
            {"PYSUITES_PIN_PORT6": str(lst.port)},
            ["ERROR g4/test_g4.py::G4.test_connect_inet6"],
        )

    def test_G5_name_resolution_is_red_including_at_import_time(self):
        self.plant(
            {"g5/test_g5.py": G5_MEMBER, "g5top/test_g5_top.py": G5_TOPLEVEL_MEMBER},
            {"PYSUITES_PIN_PORT": "80"},
            [
                "ERROR g5/test_g5.py::G5.test_getaddrinfo",
                "ERROR g5/test_g5.py::G5.test_gethostbyname",
                "ERROR g5/test_g5.py::G5.test_gethostbyname_ex",
                "ERROR g5/test_g5.py::G5.test_gethostbyaddr",
                "ERROR g5/test_g5.py::G5.test_getnameinfo",
                "g5top/test_g5_top.py: import failed",
            ],
        )

    def test_G6_environment_half_once_per_regime_key(self):
        s = self.scratch({"g6/test_g6.py": G6_MEMBER})
        self.assertBaselineGreen(s, "g6/test_g6.py", G6_INHERITED)
        out = self.assertRed(
            s.run(G6_INHERITED, purpose="g6-inherited-driver"),
            *["FAILED g6/test_g6.py::G6.test_%s" % k for k in G6_INHERITED]
        )
        self.assertEqual(len([l for l in out.splitlines() if l.startswith("python-suites: test FAILED")]), 5, out)

    def test_G7_guard_installed_exactly_once_and_nested_run_idempotent(self):
        lst = Listener(self)
        port = {"PYSUITES_PIN_PORT": str(lst.port)}
        inner = self.scratch({"c/test_count.py": G7_COUNT_MEMBER, "r/test_raw.py": RAW_CONNECT_MEMBER})
        self.assertBaselineGreen(inner, "r/test_raw.py", port)
        # Direct run: count 1 in the loader and a child; the plant red.
        out = self.assertRed(inner.run(port), "r/test_raw.py::Raw.test_raw_connect")
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_loader", out)
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_child", out)
        # The same run nested inside a guarded driver run gives the same result.
        outer = self.scratch({"n/test_nested.py": G7_NESTED_MEMBER})
        self.assertGreen(outer.run(dict(port, PYSUITES_PIN_INNER=str(inner.dir))))

    def test_G7_guard_install_failure_fails_closed(self):
        shadow = self.tmpdir("python-suites-shadow-")
        (shadow / "socket.py").write_text('raise ImportError("planted socket shadow")\n', encoding="utf-8")
        s = self.scratch({"a/test_ok.py": PASSING})
        code, out = s.run({"PYTHONPATH": self.pythonpath_after(shadow)})
        self.assertNotEqual(code, 0, out)
        self.assertIn("the offline guard did not install (ImportError: planted socket shadow)", out)
        self.assertIn("loader exited 97", out)
        self.assertNotIn("test ok", out)

    def test_G8_existing_sitecustomize_still_runs(self):
        lst = Listener(self)
        site = self.tmpdir("python-suites-site-")
        (site / "sitecustomize.py").write_text(
            'import sys\nsys._python_suites_pin_site_marker = "set"\n', encoding="utf-8"
        )
        pp = self.pythonpath_after(site)
        s = self.scratch({"m/test_mark.py": G8_MARK_MEMBER, "r/test_raw.py": RAW_CONNECT_MEMBER})
        self.assertBaselineGreen(s, "r/test_raw.py", {"PYSUITES_PIN_PORT": str(lst.port)})
        out = self.assertRed(
            s.run({"PYTHONPATH": pp, "PYSUITES_PIN_PORT": str(lst.port)}),
            "r/test_raw.py::Raw.test_raw_connect",
        )
        self.assertIn("python-suites: test ok m/test_mark.py::G8.test_chained_sitecustomize_ran", out)

    def test_G9_unnamed_regime_changing_launch_is_refused(self):
        env = dict(base_env(), NO_PROXY="*")
        with self.assertRaises(AssertionError) as cm:
            _launch([sys.executable, "-c", "pass"], env=env, purpose="driver")
        self.assertIn("not a named member of the unguarded-child class", str(cm.exception))
        # Appending after the reference PYTHONPATH is not a change.
        ref = {"PYTHONPATH": "/guard"}
        self.assertEqual(regime_changes({"PYTHONPATH": "/guard" + os.pathsep + "/x"}, ref), [])
        self.assertEqual(regime_changes({"PYTHONPATH": "/x" + os.pathsep + "/guard"}, ref), ["PYTHONPATH"])
        # A named member is admitted.
        stripped = {k: v for k, v in base_env().items() if k not in REGIME_KEYS}
        self.assertEqual(_launch([sys.executable, "-c", "pass"], env=stripped, purpose="g9-baseline")[0], 0)

    def test_G9_every_launch_goes_through_the_accounting(self):
        spawners = {"run", "Popen", "call", "check_call", "check_output", "system", "popen", "fork", "forkpty"}
        spawners |= {n for n in dir(os) if n.startswith(("exec", "spawn", "posix_spawn"))}
        stray = []
        used = set()
        for rel in PIN_FILES:
            tree = ast.parse((REPO / rel).read_text(encoding="utf-8"))
            inside = set()
            for fn in ast.walk(tree):
                if isinstance(fn, ast.FunctionDef) and fn.name == "_launch":
                    inside |= {id(n) for n in ast.walk(fn)}
            stray += [
                "%s:%d" % (rel, n.lineno)
                for n in ast.walk(tree)
                if isinstance(n, ast.Call)
                and isinstance(n.func, ast.Attribute)
                and isinstance(n.func.value, ast.Name)
                and n.func.value.id in ("subprocess", "os")
                and n.func.attr in spawners
                and id(n) not in inside
            ]
            # The named set equals the purposes actually used for regime changes.
            for n in ast.walk(tree):
                if isinstance(n, ast.Call):
                    for kw in n.keywords:
                        if kw.arg == "purpose" and isinstance(kw.value, ast.Constant):
                            used.add(kw.value.value)
        self.assertEqual(stray, [], "process launches outside _launch at %s" % stray)
        self.assertEqual(set(UNGUARDED_LAUNCHES) - used, set(), "named but never launched")

    def test_G10_class_is_defined_by_regime_key_difference(self):
        s = self.scratch({"g10/test_g10.py": G10_MEMBER})
        self.assertGreen(s.run())

    def test_G11_regime_keys_restored_before_each_member(self):
        s = self.scratch({"e/test_a_mutator.py": G11_MUTATOR, "e/test_b_checker.py": G11_CHECKER})
        self.assertGreen(s.run())

    def test_G11_socket_guard_restored_before_each_member(self):
        # Member A puts the original connect back; member B, later in the same
        # root, connects to a loopback listener. Green without the regime;
        # red with it only if the loader re-installed the guard for B.
        lst = Listener(self)
        self.plant(
            {"g11g/test_a_undo.py": G11_GUARD_UNDO, "g11g/test_b_connect.py": G11_GUARD_CONNECT},
            {"PYSUITES_PIN_PORT": str(lst.port)},
            ["ERROR g11g/test_b_connect.py::B.test_connect_after_a_member_undid_the_guard"],
        )

    def test_G12_git_variables_are_dropped_for_the_loader_and_members(self):
        decoy = self.tmpdir("python-suites-decoy-")
        member = """
            import os
            import subprocess
            import tempfile
            import unittest


            class G12(unittest.TestCase):
                def test_no_git_variable(self):
                    self.assertEqual([k for k in os.environ if k.startswith("GIT_")], [])

                def test_git_init_in_a_tempdir(self):
                    d = tempfile.mkdtemp()
                    subprocess.run(["git", "init", "-q", d], check=True)
                    open(os.path.join(d, "f"), "w").close()
                    subprocess.run(["git", "-C", d, "add", "f"], check=True)
                    self.assertTrue(os.path.exists(os.path.join(d, ".git", "index")))
        """
        s = self.scratch({"g12/test_g12.py": member})
        self.assertGreen(
            s.run({"GIT_DIR": str(decoy / "decoy.git"), "GIT_INDEX_FILE": str(decoy / "decoy-index")})
        )
        self.assertEqual(sorted(p.name for p in decoy.iterdir()), [])

    def test_G13_af_unix_goes_through_and_af_inet_stays_denied(self):
        member = """
            import multiprocessing
            import os
            import socket
            import sys
            import tempfile
            import unittest


            class G13(unittest.TestCase):
                def test_af_unix_pair_and_connect(self):
                    a, b = socket.socketpair()
                    a.close()
                    b.close()
                    d = tempfile.mkdtemp()
                    path = os.path.join(d, "s")
                    srv = socket.socket(socket.AF_UNIX)
                    srv.bind(path)
                    srv.listen(2)
                    cli = socket.socket(socket.AF_UNIX)
                    self.assertEqual(cli.connect_ex(path), 0)
                    cli2 = socket.socket(socket.AF_UNIX)
                    cli2.connect(path)
                    for s in (cli, cli2, srv):
                        s.close()

                def test_forkserver_where_it_exists(self):
                    if not sys.platform.startswith("linux"):
                        return  # forkserver is Linux's 3.14+ default; elsewhere only this part is skipped
                    p = multiprocessing.get_context("forkserver").Process(target=len, args=([],))
                    p.start()
                    p.join(60)
                    self.assertEqual(p.exitcode, 0)

                def test_af_inet_connect_is_denied(self):
                    srv = socket.socket()
                    srv.bind(("127.0.0.1", 0))
                    srv.listen(1)
                    cli = socket.socket()
                    try:
                        with self.assertRaises(OSError) as cm:
                            cli.connect(srv.getsockname())
                        self.assertIn("python-suites offline regime", str(cm.exception))
                        with self.assertRaises(OSError):
                            cli.connect_ex(srv.getsockname())
                    finally:
                        cli.close()
                        srv.close()
        """
        s = self.scratch({"g13/test_g13.py": member})
        self.assertGreen(s.run())

    def test_G14_marker_changes_are_not_hidden(self):
        # A chained sitecustomize that bumps the install marker: the loader refuses (98).
        bump = self.tmpdir("python-suites-bump-")
        (bump / "sitecustomize.py").write_text(
            "import socket\nsocket._python_suites_guard_installs += 1\n", encoding="utf-8"
        )
        s = self.scratch({"a/test_ok.py": PASSING})
        code, out = s.run({"PYTHONPATH": self.pythonpath_after(bump)})
        self.assertNotEqual(code, 0, out)
        self.assertIn("loader exited 98", out)
        self.assertIn("the offline guard is not installed exactly once", out)
        self.assertNotIn("test ok", out)
        # A member that bumps it is seen by the next member's count test, not re-stamped to 1.
        bumper = """
            import socket
            import unittest


            class A(unittest.TestCase):
                def test_bumps(self):
                    socket._python_suites_guard_installs += 1
        """
        s = self.scratch({"c/test_a_bump.py": bumper, "c/test_b_count.py": G7_COUNT_MEMBER})
        self.assertRed(s.run(), "c/test_b_count.py::G7.test_count_in_loader")
