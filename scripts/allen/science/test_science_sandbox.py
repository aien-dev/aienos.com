#!/usr/bin/env python3
"""Boundary probes for allen-science: the per-request engine sandbox (science_sandbox.run_math),
the Omega compile path (run_omega, checked against the gateway it was copied from), and the
service sandbox built by launch.py. Every negative check has a positive control showing the probe
itself works (the same probe succeeds outside the sandbox, or a permitted action succeeds inside).

Host-only; needs no root and touches no live service. Usage:
  python3 test_science_sandbox.py [--engine PATH] [--omc PATH] [--irwasm PATH] [--gateway DIR]
                                  [--busybox PATH] [--service-port PORT]
--service-port runs the real allen_science.py next to this file through launch.py on that port
(it only accepts 127.0.0.1:18113) when the port is free.
"""

import argparse
import inspect
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import science_sandbox as ss  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("--engine", default="/tmp/allen-science-math-release/build/aien-math")
ap.add_argument("--omc", default="/opt/allen-chat/omc")
ap.add_argument("--irwasm", default="/opt/allen-chat/irwasm")
ap.add_argument("--math-admit", default=os.path.join(HERE, "math-admit"))
ap.add_argument("--gateway", default="/tmp/allenchat/gw")
ap.add_argument("--busybox", default="/usr/bin/busybox")
ap.add_argument("--service-port", type=int, default=0)
A = ap.parse_args()
ss.AIEN_MATH, ss.OMC, ss.IRWASM, ss.MATH_ADMIT = A.engine, A.omc, A.irwasm, A.math_admit
ss.ENGINE_MANIFEST = os.path.join(HERE, "engine.json")

results = []


def check(name, ok, detail=""):
    results.append((name, bool(ok)))
    print("%s %s: %s" % ("PASS" if ok else "FAIL", name, detail))


def bare(expr, env=None):
    """The engine with no sandbox: the positive control for every file/env probe."""
    p = subprocess.run([A.engine, "eval"], input=expr.encode(), capture_output=True, timeout=20,
                       env=env, cwd="/tmp")
    return p.stdout.decode()


def listener(host):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind((host, 0))
    s.listen(8)

    def serve():
        while True:
            try:
                c, _ = s.accept()
            except OSError:
                return
            c.sendall(b"HOSTSIDE\n")
            c.close()
    threading.Thread(target=serve, daemon=True).start()
    return s, s.getsockname()[1]


def tailnet_ip():
    out = subprocess.run(["ip", "-4", "-o", "addr", "show", "scope", "global"], capture_output=True,
                         text=True).stdout
    for line in out.split("\n"):
        w = line.split()
        if "inet" in w:
            return w[w.index("inet") + 1].split("/")[0]
    return None


def aien_math_procs():
    return subprocess.run(["pgrep", "-f", "^/aien-math eval"], capture_output=True).stdout.split()


def E(expr):
    """The raw engine in its sandbox: no gate, no wrapper. math-admit refuses most probes below;
    the sandbox must hold even for an expression the gate wrongly admitted."""
    text, failed = ss.engine_text(expr.encode())
    if failed:
        return failed
    at = text.rfind(ss.OUT_MARK)
    value = text[at + len(ss.OUT_MARK):].rstrip("\n") if at >= 0 else ""
    messages = [ln for ln in text.splitlines() if "::" in ln]
    ok = at >= 0 and not messages and "$Failed" not in value
    return {"ok": ok, "status": "ok" if ok else "unsupported", "output": value, "messages": messages}


# ---------------------------------------------------------------- 1. engine results
r = ss.run_math(ss.SELFTEST_MATH[0])
check("symbolic fixture admitted and evaluated (control)", r["ok"] and r["output"] == ss.SELFTEST_MATH[1], r)
r = E("FooBar[x]")
check("raw engine: undefined symbol carries a message", not r["ok"]
      and any(m.startswith("Symbol::undefined") for m in r["messages"]), r)
r = ss.run_math("Sin[x]")
check("unevaluated Sin[x] is unsupported", not r["ok"] and "unevaluated" in r.get("error", ""), r)
r = E("f[x_] := x^2; f[3]")
check("raw engine evaluates a compound statement (control for the probes below)",
      r["ok"] and r["output"] == "9", r)
r = E("1+")
check("raw engine: syntax error is not ok", not r["ok"], r)

# ---------------------------------------------------------------- 1b. admission gate (math-admit)
fixtures = json.load(open(os.path.join(HERE, "admission-tests.json")))
check("admission fixtures name the pinned gate", fixtures["binary_sha256"] == ss.MATH_ADMIT_PIN
      and fixtures["compiler_sha256"] == ss.OMC_PIN, fixtures["binary_sha256"])
wrong = []
for case in fixtures["negative"]:
    # run_math refuses (the NUL case already at the byte bound); the gate alone must refuse too.
    r = ss.run_math(case["expression"])
    g = ss.admit(case["expression"].encode())
    if r["status"] != "refused" or not g or g.get("error") != ss.ADMIT_REFUSAL:
        wrong.append((case["expression"], r["status"], g))
check("run_math and the gate alone refuse every unsafe fixture (%d)" % len(fixtures["negative"]),
      not wrong and fixtures["negative"], wrong)
for unsafe in ("Pause[30]", 'ReadString["/etc/hostname"]', 'Export["/tmp/x", 1]', 'Environment["HOME"]',
               "FooBar[x]", "f[x_] := x^2; f[3]", "While[True, 1]", 'ToExpression["Run"]', "x++", "--x"):
    r = ss.run_math(unsafe)
    check("refused before the engine: %s" % unsafe, r["status"] == "refused", r)
for safe in ("Integrate[x^2, {x, 0, 1}]", "D[x^3, x]", "Factor[x^2 - 1]"):
    r = ss.run_math(safe)
    check("gate admits %s (control)" % safe, r["status"] not in ("refused", "error"), r)
for safe in ("x+1", "x-1"):
    g = ss.admit(safe.encode())
    r = ss.run_math(safe)
    check("gate alone admits %s (control for ++/--)" % safe, g is None and r["ok"], (g, r))
for unsafe in ("x++", "--x"):
    g = ss.admit(unsafe.encode())
    check("gate alone refuses %s" % unsafe, g is not None and g.get("error") == ss.ADMIT_REFUSAL, g)

# ---------------------------------------------------------------- 1c. result classifier
def wrapped_out(value):
    return "Out[1]= %s%s%s\n" % (ss.COMPUTED[0], value, ss.COMPUTED[1])


cases = [
    ("Out[1]= aienUnsupported\n", "unsupported"),
    (wrapped_out("{Sin[x], Integrate[E^x^3, x]}"), "unsupported"),
    (wrapped_out("{{x -> 1}, Solve[q, x]}"), "unsupported"),
    (wrapped_out("$Failed"), "unsupported"),
    ("Power::infy: Infinite expression 1/0 encountered.\n\n" + wrapped_out("ComplexInfinity"), "unsupported"),
    ("Out[1]= {Sin[x]}\n", "unsupported"),
    ("Out[1]= aienComputed[x\n", "unsupported"),
    ("Out[1]= aienComputed[" + "a" * 4986 + " ... " + "b" * 4996 + "]\n", "unsupported"),
    (wrapped_out("Sin[x]"), "ok"),
    (wrapped_out("2*Cos[x]*Sin[x]"), "ok"),
    (wrapped_out("c" * 9985), "ok"),
]
got = [ss.parse_engine(t)[0] for t, _ in cases]
check("wrapper classifier: only a complete aienComputed[...] without messages, $Failed, residual "
      "operations or printer cut is ok", got == [w for _, w in cases], got)
r = ss.run_math("Range[3000]")
check("real engine truncation (Range[3000]) is unsupported, not exact", r["status"] == "unsupported"
      and r["output"] == "" and "cut" in r.get("error", ""), r)
for echo in ("{Sin[x]}", "List[Sin[x]]", "{ Sin[ x ] , 2 }", "List[Sin[x], 2]"):
    r = ss.run_math(echo)
    check("real structural echo %s is unsupported" % echo, r["status"] == "unsupported"
          and "unevaluated" in r.get("error", ""), r)
for computed in (("D[Sin[x]^2, x]", "Sin[x]"), ("Plus[x, x]", "2*x"), ("List[Sin[x], 1+1]", "{Sin[x], 2}"),
                 ("x+1", "x"), ("Plus[x, 0]", "x"), ("x+0", "x"), ("Times[x, 1]", "x")):
    r = ss.run_math(computed[0])
    check("real computed result %s is ok (control)" % computed[0], r["ok"] and computed[1] in r["output"], r)
for bad in ("x, y", "Sin[x]]", "{x", "Sin[x); y", "aienR", 'f["]'):
    check("not one expression refused before any process: %r" % bad, not ss.single_expression(bad), bad)
check("one expression with commas inside brackets and strings passes (control)",
      ss.single_expression('{Sin[x], 2, ","}') and ss.single_expression("Plus[x, 0]"), "")
check("wrapper text is internal: the computed output is the bare value", "aien" not in
      ss.run_math("Plus[x, x]")["output"], "")

# ---------------------------------------------------------------- 2. input bounds never spawn
real_popen = subprocess.Popen
spawned = []
subprocess.Popen = lambda *a, **k: spawned.append(a) or (_ for _ in ()).throw(AssertionError("spawned"))
refusals = [ss.run_math(x) for x in (None, b"1+1", "", "   ", "1+\0" + "1", "x" * (ss.MAX_EXPR + 1),
                                     "\ud800")]
subprocess.Popen = real_popen
check("bad inputs refused without a subprocess",
      all(x["status"] == "refused" and not x["ok"] for x in refusals) and not spawned,
      [x["error"] for x in refusals])
r = ss.run_math("x" * (ss.MAX_EXPR - 10) + " + 0")
check("input at the cap passes the byte bound (control)", "longer than" not in r.get("error", ""), r)

# ---------------------------------------------------------------- 3. engine sandbox: filesystem
probe = 'ReadString["/etc/hostname"]'
outside = bare(probe)
r = E(probe)
check("host file unreadable inside (outside control reads it)",
      not r["ok"] and "Out[1]= \"" in outside, "inside=%r outside=%r" % (r["output"], outside[-40:]))
for path in ("/etc/passwd", "/home", os.path.expanduser("~"), "/root", "/proc/self", "/dev/null",
             "/opt/allen-science", "/usr"):
    e = 'FileExistsQ["%s"]' % path
    out_host = bare(e)
    r = E(e)
    check("%s invisible inside" % path, r["output"] == "False",
          "inside=%s outside=%s" % (r["output"], out_host.strip().rsplit(" ", 1)[-1]))
r = E('Export["/tmp/ok.txt", "hi"]; ReadString["/tmp/ok.txt"]')
check("private tmpfs writable and readable (control)", r["ok"] and r["output"] == '"hi"', r)
r = E('Export["/tmp/ok.txt", "hi"]; FileExistsQ["/tmp/ok.txt"]')
r2 = E('FileExistsQ["/tmp/ok.txt"]')
check("tmpfs does not persist between requests", r["output"] == "True" and r2["output"] == "False",
      "%s then %s" % (r["output"], r2["output"]))
for target in ("/evil.txt", "/aien-math"):
    r = E('Export["%s", "x"]; FileByteCount["%s"]' % (target, target))
    check("write to %s refused" % target, r["output"] != "1", r["output"][:60])
size = E('Export["/tmp/big.txt", StringRepeat["a", 3000000]]; FileByteCount["/tmp/big.txt"]')
check("file size capped at 1 MiB", not size["ok"] or int(size["output"] or 0) <= 1 << 20, size)

# ---------------------------------------------------------------- 4. environment
env = dict(os.environ, SCIENCE_CANARY="should-not-leak")
outside = bare('Environment["SCIENCE_CANARY"]', env)
os.environ["SCIENCE_CANARY"] = "should-not-leak"
r = E('Environment["SCIENCE_CANARY"]')
r2 = E('Environment["HOME"]')
del os.environ["SCIENCE_CANARY"]
check("environment cleared (outside control sees the canary)",
      "should-not-leak" in outside and "should-not-leak" not in json.dumps(r) and r2["output"] == "$Failed",
      "inside=%r home=%r" % (r["output"], r2["output"]))

# ---------------------------------------------------------------- 5. time, CPU, output, memory
seen = []
w = threading.Thread(target=lambda: (time.sleep(2), seen.append(aien_math_procs())))
w.start()
t0 = time.monotonic()
r = E("Pause[30]; 1")
dt = time.monotonic() - t0
w.join()
time.sleep(0.5)
check("sleep killed at the wall clock", r["status"] == "timeout" and dt < ss.MATH_WALL_S + 2, "%.1fs" % dt)
check("engine visible while running (control)", bool(seen and seen[0]), seen)
check("no engine process survives the kill", not aien_math_procs(), aien_math_procs())
t0 = time.monotonic()
r = E("While[True, 1]")
dt = time.monotonic() - t0
time.sleep(0.5)
check("CPU spin stopped within the wall clock and nothing survives", not r["ok"] and dt < ss.MATH_WALL_S + 2
      and not aien_math_procs(), "%s %.1fs %s" % (r["status"], dt, r.get("error")))
# The engine truncates a printed value at about 10000 characters itself; Print is not truncated.
r = E('Do[Print[StringRepeat["a", 5000]], {100}]; 1')
check("output past 64 KiB is cut off", r["status"] == "too_large", r["status"])
r = E('Do[Print[StringRepeat["a", 50]], {10}]; StringLength[StringRepeat["ab", 100000]]')
check("same work with small output passes (control)", r["ok"] and r["output"] == "200000", r)
t0 = time.monotonic()
r = E("Length[Range[10^9]]")
check("huge allocation fails bounded", not r["ok"] and time.monotonic() - t0 < ss.MATH_WALL_S + 2,
      "%s %s" % (r["status"], r.get("error")))
r = E("Length[Range[10^5]]")
check("moderate allocation works (control)", r["ok"] and r["output"] == "100000", r)

# ---------------------------------------------------------------- 6. network, via busybox in the
# exact engine sandbox argv (the engine has no network builtins to probe with).
PRE = 'b() { (exec -a busybox /aien-math "$@"); }; '


def in_engine_sandbox(script, wall=6):
    argv = ss.math_argv(A.busybox, ["sh", "-c", PRE + script])
    i = argv.index("--")
    argv[i:i] = ["--argv0", "busybox"]
    st, out, err, rc = ss.run_bounded(ss.limited(ss.MATH_LIMITS, argv), b"", wall, 1 << 16, 4096)
    return out.decode("utf-8", "replace")


o = in_engine_sandbox("echo ALIVE")
check("busybox probe runs in the engine sandbox (control)", o.strip() == "ALIVE", repr(o))
lo, lo_port = listener("127.0.0.1")
tip = tailnet_ip()
tn, tn_port = listener(tip) if tip else (None, 0)
for host, port in (("127.0.0.1", lo_port), (tip, tn_port)):
    if not host:
        continue
    o = in_engine_sandbox("b nc -w 2 %s %d </dev/null; echo rc=$?" % (host, port))
    c = subprocess.run([A.busybox, "nc", "-w", "2", host, str(port)], capture_output=True,
                       stdin=subprocess.DEVNULL, timeout=10).stdout.decode()
    check("engine sandbox cannot reach %s:%d (outside control reaches it)" % (host, port),
          "HOSTSIDE" not in o and "HOSTSIDE" in c, "inside=%r outside=%r" % (o.strip()[-40:], c.strip()))
o = in_engine_sandbox("b ls / /proc /dev 2>&1; b env; echo END")
check("engine sandbox root holds only the engine and tmp, no /proc, /dev",
      sorted(o.split("END")[0].split("\n")[0:2]) == ["aien-math", "tmp"] or
      ("aien-math\ntmp" in o and "/proc" in o and "No such file" in o), repr(o[:200]))
o = in_engine_sandbox("b unshare -U b echo NESTED 2>&1; echo END")
check("nested user namespaces disabled", "NESTED" not in o.replace("b echo NESTED", ""), repr(o[:120]))
o = in_engine_sandbox("ulimit -t; ulimit -n; echo END")
check("prlimit applied the CPU and descriptor limits in the leaf", o.split()[:2] == ["5", "32"], repr(o))
check("no Python pre-exec callback anywhere in the helper", "preexec" not in inspect.getsource(ss), "")

# ---------------------------------------------------------------- 6b. bounded collection (compiler path)
BB_PRE = 'b() { (exec -a busybox /tool "$@"); }; '


def busybox_procs():
    return subprocess.run(["pgrep", "-f", "^busybox (yes|sleep 5|sh -c b)"], capture_output=True).stdout.split()


t0 = time.monotonic()
out, err, rc = ss.sandboxed(A.busybox, b"", 5, 1 << 20, args=("yes",), argv0="busybox")
check("compiler-path stdout flood stopped at the cap", out is None and err == b"output too large"
      and time.monotonic() - t0 < 4, "%r %r %.1fs" % (err, rc, time.monotonic() - t0))
t0 = time.monotonic()
st, out, err, rc = ss.run_bounded(ss.limited(ss.OMEGA_LIMITS, ss.sandbox_argv(
    A.busybox, ("sh", "-c", BB_PRE + "b yes >&2"), "busybox")), b"", 2, 1 << 20, 4096)
check("stderr flood: kept to 4096 bytes and killed at the deadline", st == "timeout" and len(err) == 4096
      and not out and time.monotonic() - t0 < 4, "%s err=%d %.1fs" % (st, len(err), time.monotonic() - t0))
big = b"x" * (8 << 20)
out, err, rc = ss.sandboxed(A.busybox, big, 10, 1 << 20, args=("wc", "-c"), argv0="busybox")
check("8 MiB stdin fed without deadlock (control)", out is not None and out.strip() == b"8388608", (out, err, rc))
t0 = time.monotonic()
out, err, rc = ss.sandboxed(A.busybox, big, 2, 1 << 20, args=("sleep", "5"), argv0="busybox")
check("unread 8 MiB stdin does not block past the deadline", out is None and err == b"timeout"
      and time.monotonic() - t0 < 4, "%r %.1fs" % (err, time.monotonic() - t0))
time.sleep(0.5)
check("no flood or sleep process survives", not busybox_procs(), busybox_procs())

# ---------------------------------------------------------------- 7. Omega compile path parity
sys.path.insert(0, A.gateway)
import allen_chat as gw  # noqa: E402
same = {k: getattr(gw, k) == getattr(ss, k) for k in
        ("BWRAP", "OMC_PIN", "IRWASM_PIN", "MAX_SOURCE", "OMC_HINTS", "TYPES")}
same["OMC_ERR"] = gw.OMC_ERR.pattern == ss.OMC_ERR.pattern
same["NAME"] = gw.NAME.pattern == ss.NAME.pattern
same["signatures"] = inspect.getsource(gw.signatures) == inspect.getsource(ss.signatures)


class Spawned(Exception):
    pass


def record(argv, **kw):
    seen_argv.append(argv)
    raise Spawned()


seen_argv = []


def spawned_argv(mod):
    seen_argv.clear()
    real_popen = mod.subprocess.Popen
    mod.subprocess.Popen = record
    try:
        mod.sandboxed("/x/tool", b"", 1, 1, args=("a", "b"), argv0="busybox")
    except Spawned:
        pass
    finally:
        mod.subprocess.Popen = real_popen
    return list(seen_argv)


gw_argv, ss_argv = spawned_argv(gw), spawned_argv(ss)
same["argv"] = len(gw_argv) == 1 and gw_argv == ss_argv
check("omega path spawns exactly the gateway's argv (prlimit limits + bwrap) with its pins and hints",
      all(same.values()), [k for k, v in same.items() if not v] or " ".join(ss_argv[0][:7]))
handler = inspect.getsource(gw.Handler.omega) if hasattr(gw, "Handler") else ""
check("gateway handler still uses sandboxed(app.omc/irwasm) as copied",
      "sandboxed(app.omc, data, 5, 4 << 20)" in handler and "sandboxed(app.irwasm, ir, 5, 1 << 20)" in handler,
      "handler found=%s" % bool(handler))
good = "fn gcd a ulong b ulong to ulong\nwhile ne b 0\nt = rem a b\na = b\nb = t\nend\nret a\nend\n"
o = ss.run_omega(good)
check("omega gcd compiles to wasm (control)", o.get("ok") and o["functions"][0]["name"] == "gcd"
      and __import__("base64").b64decode(o["wasm"])[:4] == b"\0asm", {k: o.get(k) for k in ("ok", "functions")})
o = ss.run_omega("fn f a ulong to ulong\nx = add a 1 7\nret x\nend\n")
check("native pre-check refuses IR-accepted extra words", not o["ok"] and o["code"] == 5, o)
o = ss.run_omega("x" * (ss.MAX_SOURCE + 1))
check("oversized Omega source refused", not o["ok"] and o["stage"] == "input", o["message"])

# ---------------------------------------------------------------- 8. selftest, and pin failure
check("selftest passes with pinned tools (control)", ss.selftest() == [], "")
bad = tempfile.NamedTemporaryFile(delete=False)
bad.write(b"not the engine")
bad.close()
keep, ss.AIEN_MATH = ss.AIEN_MATH, bad.name
r = ss.run_math("1+1")
problems = ss.selftest()
ss.AIEN_MATH = keep
check("unpinned engine is refused (fail closed)", r["status"] == "error" and "engine" in r["error"]
      and "pin" in r["error"] and problems, problems)
keep, ss.MATH_ADMIT = ss.MATH_ADMIT, bad.name
r = ss.run_math("1+1")
problems = ss.selftest()
ss.MATH_ADMIT = keep
os.unlink(bad.name)
check("unpinned admission gate is refused (fail closed)", r["status"] == "error" and "gate" in r["error"]
      and problems, problems)

# ---------------------------------------------------------------- 8b. engine provenance manifest
check("shipped engine.json names the pinned engine (control)", ss.manifest_problem() is None,
      ss.manifest_problem())
good_manifest = json.load(open(ss.ENGINE_MANIFEST))
bad_manifests = {"missing": None, "not JSON": "{", "stale commit": dict(good_manifest, commit="0" * 40),
                 "other binary": dict(good_manifest, binary_sha256="0" * 64), "array": []}
tmpdir = tempfile.mkdtemp(prefix="science-manifest-")
keep_manifest = ss.ENGINE_MANIFEST
for label, content in bad_manifests.items():
    path = os.path.join(tmpdir, label.replace(" ", "-") + ".json")
    if content is not None:
        with open(path, "w") as fh:
            fh.write(content if isinstance(content, str) else json.dumps(content))
    ss.ENGINE_MANIFEST = path
    ss._manifest.clear()
    problems = ss.selftest()
    r = ss.run_math(ss.SELFTEST_MATH[0])
    check("%s manifest refuses startup and every math request" % label,
          any("manifest" in p for p in problems) and r["status"] == "error" and "manifest" in r["error"],
          (problems, r.get("error")))
ss.ENGINE_MANIFEST = keep_manifest
ss._manifest.clear()
shutil.rmtree(tmpdir)

# ---------------------------------------------------------------- 8c. installer (DESTDIR, no root)
def install(src_dir, destdir, engine=None):
    env = {"PATH": "/usr/bin:/bin"}
    if destdir is not None:
        env["DESTDIR"] = destdir
    return subprocess.run(["sh", os.path.join(src_dir, "install.sh"), engine or A.engine, A.omc, A.irwasm,
                           A.math_admit], capture_output=True, env=env, timeout=60)


def tree(d):
    return sorted(os.path.relpath(os.path.join(p, n), d) for p, _, ns in os.walk(d) for n in ns)


inst = tempfile.mkdtemp(prefix="science-install-")
p = install(HERE, inst)
opt, gwdir = os.path.join(inst, "opt/allen-science"), os.path.join(inst, "opt/allen-chat")
same_bytes = lambda a, b: open(a, "rb").read() == open(b, "rb").read()
ok_pins = p.returncode == 0 and all(ss.sha256_file(os.path.join(opt, n)) == pin for n, pin in (
    ("aien-math", ss.AIEN_MATH_PIN), ("math-admit", ss.MATH_ADMIT_PIN), ("omc", ss.OMC_PIN), ("irwasm", ss.IRWASM_PIN)))
check("install places pinned tools and a valid manifest (control)",
      ok_pins and ss.manifest_problem(os.path.join(opt, "engine.json")) is None, p.stderr.decode()[-200:])
check("installed service and gateway science_protocol.py are byte-identical to the source (copies, not links)",
      p.returncode == 0 and same_bytes(os.path.join(opt, "science_protocol.py"), os.path.join(HERE, "science_protocol.py"))
      and same_bytes(os.path.join(gwdir, "science_protocol.py"), os.path.join(HERE, "science_protocol.py"))
      and not os.path.islink(os.path.join(gwdir, "science_protocol.py")), tree(inst))
stale_src = tempfile.mkdtemp(prefix="science-stale-")
for n in os.listdir(HERE):
    if os.path.isfile(os.path.join(HERE, n)):
        shutil.copy2(os.path.join(HERE, n), os.path.join(stale_src, n))
with open(os.path.join(stale_src, "engine.json"), "w") as fh:
    json.dump(dict(good_manifest, commit="0" * 40), fh)
dest2 = tempfile.mkdtemp(prefix="science-install-")
p = install(stale_src, dest2)
check("install refuses a stale manifest and writes nothing", p.returncode != 0 and b"manifest" in p.stderr
      and tree(dest2) == [], p.stderr.decode().strip()[-160:])
os.unlink(os.path.join(stale_src, "engine.json"))
p = install(stale_src, dest2)
check("install refuses a missing manifest and writes nothing", p.returncode != 0 and b"manifest" in p.stderr
      and tree(dest2) == [], p.stderr.decode().strip()[-160:])
p = install(HERE, dest2, engine=A.math_admit)
check("install refuses an engine that is not the pinned one", p.returncode != 0 and b"pinned" in p.stderr
      and tree(dest2) == [], p.stderr.decode().strip()[-160:])
p = install(HERE, None)
check("install without DESTDIR requires root", (os.getuid() == 0) or (p.returncode != 0 and b"root" in p.stderr),
      p.stderr.decode().strip())
for d in (inst, stale_src, dest2):
    shutil.rmtree(d)

# ---------------------------------------------------------------- 9. service sandbox (launch.py)
PROBE_SERVICE = r'''
import os
def is_open(n):
    try:
        os.fstat(n)
        return True
    except OSError:
        return False
open_fds = [n for n in range(1024) if is_open(n)]
import json, socket, sys
sys.path.insert(0, "/opt/allen-science")
import science_sandbox as ss
fd = int(sys.argv[sys.argv.index("--listen-fd") + 1])
lo_port, tn_host, tn_port = int(sys.argv[-3]), sys.argv[-2], int(sys.argv[-1])
f = {"fds": open_fds}
def attempt(fn):
    try:
        return fn()
    except Exception as e:
        return "ERR %s" % type(e).__name__
def connect(h, p):
    s = socket.socket(); s.settimeout(2); s.connect((h, p)); d = s.recv(64); s.close(); return d.decode()
def write(p):
    with open(p, "w") as fh:
        fh.write("x")
    return "WROTE"
f["uid"] = os.getuid()
f["env"] = dict(os.environ)
f["root"] = sorted(os.listdir("/"))
f["home"] = sorted(os.listdir("/home"))
for p in ("/etc/passwd", "/etc/shadow", "/root", "/var", "/run", "/home/drakestapleton", "/usr/bin/curl",
          "/usr/bin/sudo", "/usr/lib/python3/dist-packages", "/tmp/aien-gb10.lock"):
    f["exists " + p] = os.path.exists(p)
f["write /"] = attempt(lambda: write("/evil"))
f["write opt"] = attempt(lambda: write("/opt/allen-science/evil"))
f["write usr"] = attempt(lambda: write("/usr/lib/python3.12/evil"))
f["write tmp"] = attempt(lambda: write("/tmp/ok"))
f["write home"] = attempt(lambda: write(os.environ["HOME"] + "/ok"))
f["loopback out"] = attempt(lambda: connect("127.0.0.1", lo_port))
f["tailnet out"] = attempt(lambda: connect(tn_host, tn_port)) if tn_port else "skipped"
f["internet out"] = attempt(lambda: connect("1.1.1.1", 443))
f["pids"] = sorted(int(p) for p in os.listdir("/proc") if p.isdigit())
f["capeff"] = [l.split()[1] for l in open("/proc/self/status") if l.startswith("CapEff")][0]
f["nnp"] = [l.split()[1] for l in open("/proc/self/status") if l.startswith("NoNewPrivs")][0]
f["math"] = ss.run_math(ss.SELFTEST_MATH[0])
f["math file"] = ss.evaluate('FileExistsQ["/opt/allen-science/aien-math"]', b'FileExistsQ["/opt/allen-science/aien-math"]')
f["math run"] = ss.run_math("Run[x]")
f["omega"] = {k: v for k, v in ss.run_omega(ss.SELFTEST_OMEGA).items() if k != "wasm"}
f["selftest"] = ss.selftest()
srv = socket.socket(fileno=fd)
f["listen"] = list(srv.getsockname())
body = json.dumps(f).encode()
while True:
    c, _ = srv.accept()
    c.sendall(body)
    c.close()
'''


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


def wait_body(port, proc, seconds=30):
    end = time.monotonic() + seconds
    while time.monotonic() < end and proc.poll() is None:
        try:
            c = socket.create_connection(("127.0.0.1", port), timeout=1)
            c.settimeout(20)
            data = b""
            while True:
                d = c.recv(65536)
                if not d:
                    break
                data += d
            c.close()
            return data
        except OSError:
            time.sleep(0.2)
    return None


def stop(proc):
    if proc.poll() is None:
        proc.kill()
    proc.wait()


root = tempfile.mkdtemp(prefix="science-root-")
for src, name in ((os.path.join(HERE, "science_sandbox.py"), "science_sandbox.py"), (A.engine, "aien-math"),
                  (A.omc, "omc"), (A.irwasm, "irwasm"), (A.math_admit, "math-admit"),
                  (os.path.join(HERE, "engine.json"), "engine.json")):
    shutil.copy2(src, os.path.join(root, name))
with open(os.path.join(root, "allen_science.py"), "w") as fh:
    fh.write(PROBE_SERVICE)
os.chmod(root, 0o755)
port = free_port()
canary_env = dict(os.environ, SCIENCE_CANARY="should-not-leak", ALLEN_UPSTREAM_TOKEN="should-not-leak")
canary_r, canary_w = os.pipe()
canary_sock = socket.socket()
HIGH_FD = 900
os.dup2(canary_r, HIGH_FD)
# The launcher starts with soft RLIMIT_NOFILE 64 and a descriptor numbered 900 above it: a
# closerange up to the soft limit would leave it open.
launch_argv = ["/usr/bin/prlimit", "--nofile=64:4096", "--", sys.executable, "-I",
               os.path.join(HERE, "launch.py"), "--listen", "127.0.0.1:%d" % port,
               "--root", root, "--", str(lo_port), tip or "", str(tn_port)]
ctl = subprocess.run(["/usr/bin/prlimit", "--nofile=64:4096", "--", sys.executable, "-I", "-c",
                      "import os, resource; print(os.path.exists('/proc/self/fd/%d'), "
                      "resource.getrlimit(resource.RLIMIT_NOFILE)[0])" % HIGH_FD],
                     capture_output=True, text=True, pass_fds=(HIGH_FD,)).stdout.split()
check("high fd control: fd %d is open in a child whose soft limit is lower" % HIGH_FD,
      ctl == ["True", "64"], ctl)
proc = subprocess.Popen(launch_argv, env=canary_env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                        pass_fds=(canary_r, canary_w, canary_sock.fileno(), HIGH_FD))
passed_fds = sorted((canary_r, canary_w, canary_sock.fileno(), HIGH_FD))
os.close(HIGH_FD)
data = wait_body(port, proc)
stop(proc)
log = proc.stdout.read().decode("utf-8", "replace")
f = json.loads(data) if data else {}
check("service reachable inbound over loopback via the inherited socket (control)", bool(f),
      "launcher output: %r" % log[-400:])
if f:
    check("service runs unprivileged, same uid, no capabilities, no_new_privs",
          f["uid"] == os.getuid() and f["capeff"] == "0000000000000000" and f["nnp"] == "1",
          "uid=%s capeff=%s nnp=%s" % (f["uid"], f["capeff"], f["nnp"]))
    check("only stdin, stdout, stderr and the listener reach the service (canary fds %s passed in)" % passed_fds,
          f["fds"] == [0, 1, 2, 3] and all(n > 3 for n in passed_fds), f["fds"])
    # bwrap sets PWD for --chdir; it holds the private home, nothing from the host.
    check("environment is only HOME, LANG, PATH, PWD (launcher canaries absent)",
          f["env"] == {"HOME": "/home/allen-science", "LANG": "C.UTF-8", "PATH": "/usr/bin:/bin",
                       "PWD": "/home/allen-science"}, f["env"])
    check("root holds only the bound tree", f["root"] == ["bin", "dev", "home", "lib", "opt", "proc", "tmp", "usr"],
          f["root"])
    check("/home holds only the private home", f["home"] == ["allen-science"], f["home"])
    hidden = {k: v for k, v in f.items() if k.startswith("exists ")}
    host = {k: os.path.exists(k[7:]) for k in hidden}
    check("host credentials and files invisible (they exist outside)",
          not any(hidden.values()) and host["exists /etc/passwd"] and host["exists /home/drakestapleton"],
          "inside=%s outside=%s" % ([k for k, v in hidden.items() if v], [k for k, v in host.items() if v]))
    check("read-only root, install dir and stdlib",
          all(f[k].startswith("ERR") for k in ("write /", "write opt", "write usr")),
          [f[k] for k in ("write /", "write opt", "write usr")])
    check("private /tmp and HOME writable (control)", f["write tmp"] == "WROTE" and f["write home"] == "WROTE",
          [f["write tmp"], f["write home"]])
    lo_out = socket.create_connection(("127.0.0.1", lo_port), timeout=2).recv(64).decode()
    check("no outbound loopback from the service (host control reaches it)",
          f["loopback out"].startswith("ERR") and "HOSTSIDE" in lo_out, [f["loopback out"], lo_out.strip()])
    if tip:
        tn_out = socket.create_connection((tip, tn_port), timeout=2).recv(64).decode()
        check("no outbound tailnet from the service (host control reaches it)",
              f["tailnet out"].startswith("ERR") and "HOSTSIDE" in tn_out, [f["tailnet out"], tn_out.strip()])
    check("no outbound internet from the service", f["internet out"].startswith("ERR"), f["internet out"])
    check("own pid namespace", len(f["pids"]) <= 4 and max(f["pids"]) < 50, f["pids"])
    check("nested engine sandbox works inside the service sandbox", f["math"]["ok"]
          and f["math"]["output"] == ss.SELFTEST_MATH[1], f["math"])
    check("engine cannot see the service's install dir", f["math file"]["output"] == "False", f["math file"])
    check("nested admission gate refuses Run[x] inside the service sandbox",
          f["math run"]["status"] == "refused" and f["math run"].get("error") == ss.ADMIT_REFUSAL, f["math run"])
    check("nested Omega compile works inside the service sandbox", f["omega"].get("ok"), f["omega"])
    check("selftest passes inside the service sandbox", f["selftest"] == [], f["selftest"])
p = subprocess.run([sys.executable, "-I", os.path.join(HERE, "launch.py"), "--listen", "0.0.0.0:%d" % free_port(),
                    "--root", root], capture_output=True, timeout=20)
check("launcher refuses a non-loopback listen address", p.returncode != 0 and b"127.0.0.1" in p.stderr,
      p.stderr.decode().strip())

# ---------------------------------------------------------------- 10. the real service, optional
if A.service_port:
    taken = subprocess.run(["ss", "-ltnH", "sport = :%d" % A.service_port], capture_output=True).stdout.strip()
    if taken:
        print("NOT_RUN real service: port %d in use" % A.service_port)
    else:
        real = tempfile.mkdtemp(prefix="science-real-")
        for name in os.listdir(root):
            if name != "allen_science.py":
                shutil.copy2(os.path.join(root, name), os.path.join(real, name))
        for name in ("allen_science.py", "science_protocol.py", "engine.json"):
            if os.path.exists(os.path.join(HERE, name)):
                shutil.copy2(os.path.join(HERE, name), os.path.join(real, name))
        os.chmod(real, 0o755)
        proc = subprocess.Popen([sys.executable, "-I", os.path.join(HERE, "launch.py"), "--listen",
                                 "127.0.0.1:%d" % A.service_port, "--root", real],
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        up = False
        end = time.monotonic() + 30
        while time.monotonic() < end and proc.poll() is None and not up:
            try:
                socket.create_connection(("127.0.0.1", A.service_port), timeout=1).close()
                up = True
            except OSError:
                time.sleep(0.2)
        time.sleep(2)
        alive = proc.poll() is None

        def compute(tool, arguments):
            body = json.dumps({"tool": tool, "arguments": arguments}).encode()
            c = socket.create_connection(("127.0.0.1", A.service_port), timeout=30)
            c.sendall(b"POST /compute HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\n"
                      b"Connection: close\r\nContent-Length: %d\r\n\r\n" % len(body) + body)
            data = b""
            while True:
                d = c.recv(65536)
                if not d:
                    break
                data += d
            c.close()
            head, _, payload = data.partition(b"\r\n\r\n")
            return head.split(b"\r\n")[0].decode(), payload.decode("utf-8", "replace")

        computed = unsupported = refused = compiled = ("", "")
        if up and alive:
            computed = compute("math_compute", {"title": "fixture", "expression": ss.SELFTEST_MATH[0]})
            unsupported = compute("math_compute", {"title": "nested echo", "expression": "{Sin[x]}"})
            refused = compute("math_compute", {"title": "unsafe", "expression": "Run[x]"})
            compiled = compute("omega_compile", {"title": "f", "source": ss.SELFTEST_OMEGA})
        stop(proc)
        out = proc.stdout.read().decode("utf-8", "replace")
        check("real allen_science.py starts in the launcher sandbox (selftest passed, accepting)",
              up and alive, "output=%r" % out[-400:])
        check("real /compute math: computed with the pinned commit", " 200 " in computed[0]
              and '"computed"' in computed[1] and ss.SELFTEST_MATH[1] in computed[1]
              and ss.ENGINE_COMMIT in computed[1], computed[0] + " " + computed[1][:300])
        check("real /compute nested echo {Sin[x]} is not computed", '"computed"' not in unsupported[1]
              and '"unsupported"' in unsupported[1], unsupported[0] + " " + unsupported[1][:200])
        check("real /compute Run[x] is refused, never computed", '"computed"' not in refused[1]
              and refused[0] != "", refused[0] + " " + refused[1][:200])
        check("real /compute omega_compile returns wasm", " 200 " in compiled[0] and '"wasm"' in compiled[1],
              compiled[0] + " " + compiled[1][:200])
        shutil.rmtree(real)
shutil.rmtree(root)
lo.close()
if tn:
    tn.close()

failed = [n for n, ok in results if not ok]
print("%d/%d passed" % (len(results) - len(failed), len(results)))
sys.exit(1 if failed else 0)
