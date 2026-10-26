"""Isolated execution for the allen-science service.

run_math(expression) first runs the pinned admission gate math-admit (seeded Omega,
math-admit.om) on the expression; only an expression it admits (exit 0, empty stdout) reaches the
pinned aien-math engine (`aien-math eval`, code on stdin). Both run in the same bubblewrap sandbox:
every namespace unshared (no network at all, not even loopback), nested user namespaces disabled,
cleared environment, the one binary as the only read-only bind, a small private tmpfs at /tmp, no
/dev, no /proc, and CPU, address-space, file-size and descriptor limits. stdout is read
incrementally and the sandbox is killed past the cap, so a large output never sits in this
process's memory. No shell is involved anywhere: argv is a fixed list.

run_omega(source) is the gateway's Omega compile path (allen_chat.py: sandboxed(), signatures(),
the two-stage omc then omc-ir compile and irwasm), copied verbatim so the same pinned tools run
under the same bwrap argv; test_science_sandbox.py checks the copy against the gateway.

selftest() verifies the four pins, runs one fixture through each path and checks that the gate
refuses Run[x] and URLRead[x]; the service calls it at startup and refuses to start when it returns
problems.
"""

import base64
import hashlib
import os
import re
import resource
import selectors
import signal
import subprocess
import threading
import time

ROOT = "/opt/allen-science"
BWRAP = "/usr/bin/bwrap"
AIEN_MATH = ROOT + "/aien-math"
OMC = ROOT + "/omc"
IRWASM = ROOT + "/irwasm"
MATH_ADMIT = ROOT + "/math-admit"

# aien-math main 43c629db362b48aeec3590a8f0cd30e927e5ec16 (clean tree, reproducible) built with omc
# e623d440... (omega-mv 3d000060); frozen copy /tmp/allen-science-math-release/build/aien-math.
AIEN_MATH_PIN = "a3c538e9e2c8b255ba3daf4cf5a486b379e2722e86a177c173ff2d097e9ddd59"
# math-admit.om compiled with the gateway's omc (OMC_PIN); exit 0 admit, 2 refuse, 3 IO.
MATH_ADMIT_PIN = "c11c1accbaee1ca184104ab50d12649e85d22047ad37458b05a59244a96a97af"
ADMIT_WALL_S = 2
# The gateway's pins (allen_chat.py OMC_PIN, IRWASM_PIN).
OMC_PIN = "734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61"
IRWASM_PIN = "2d235a70c39ac1461a09c41e10e058d962b767ae83893cdcfc693ef6c47182e6"

MATH_ARGV = ("eval",)          # code on stdin: no argv quoting, nothing visible in the host's ps
MAX_EXPR = 4096                # bytes of UTF-8 accepted from the service
MATH_WALL_S = 8                # wall clock; the CPU limit below catches spinning first
MATH_OUT_CAP = 64 << 10        # stdout bytes; past this the sandbox is killed
MATH_ERR_CAP = 4096            # stderr bytes kept (the rest is drained and dropped)
MATH_TMPFS = 8 << 20
# The engine's static RW segment is about 978 MiB of zero pages (readelf: 0x3d220ee0), so the
# address-space limit must sit above it; its pools are fixed, exhaustion exits 3.
MATH_AS = 1152 << 20
MATH_SLOTS = threading.BoundedSemaphore(2)
OMEGA_SLOTS = threading.BoundedSemaphore(2)

OUT_MARK = "Out[1]= "
MESSAGE = re.compile(r"^[A-Za-z$][A-Za-z0-9$`]*::[A-Za-z0-9$]+:")
HEAD = re.compile(r"^\s*([A-Za-z$][A-Za-z0-9$`]*)\s*\[")


def math_limits():
    resource.setrlimit(resource.RLIMIT_CPU, (5, 6))
    resource.setrlimit(resource.RLIMIT_AS, (MATH_AS, MATH_AS))
    resource.setrlimit(resource.RLIMIT_FSIZE, (1 << 20, 1 << 20))
    resource.setrlimit(resource.RLIMIT_NOFILE, (32, 32))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def math_argv(tool, args=MATH_ARGV, dest="/aien-math"):
    """bwrap argv for one run; `tool` is bound read-only at dest and is all there is."""
    return [
        BWRAP, "--unshare-all", "--unshare-user", "--disable-userns", "--die-with-parent",
        "--new-session", "--clearenv", "--cap-drop", "ALL",
        "--ro-bind", tool, dest, "--size", str(MATH_TMPFS), "--tmpfs", "/tmp",
        "--remount-ro", "/", "--chdir", "/tmp", "--", dest,
    ] + list(args)


def run_bounded(argv, data, wall_s, out_cap, err_cap, limits):
    """Run argv with data on stdin; returns (status, stdout, stderr, rc).
    status: "done", "timeout" or "too_large". The process group is always killed and reaped."""
    p = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         preexec_fn=limits, start_new_session=True, close_fds=True)
    status, out, err = "done", bytearray(), bytearray()
    try:
        try:
            p.stdin.write(data)  # at most MAX_EXPR bytes: fits the pipe buffer, never blocks
        except BrokenPipeError:
            pass
        p.stdin.close()
        deadline = time.monotonic() + wall_s
        sel = selectors.DefaultSelector()
        sel.register(p.stdout, selectors.EVENT_READ, out)
        sel.register(p.stderr, selectors.EVENT_READ, err)
        while sel.get_map():
            left = deadline - time.monotonic()
            if left <= 0:
                status = "timeout"
                break
            for key, _ in sel.select(left):
                chunk = os.read(key.fd, 65536)
                if not chunk:
                    sel.unregister(key.fileobj)
                elif key.data is out:
                    out += chunk
                    if len(out) > out_cap:
                        status = "too_large"
                        break
                elif len(err) < err_cap:
                    err += chunk[:err_cap - len(err)]
            if status != "done":
                break
        sel.close()
        if status == "done":
            try:
                p.wait(max(0.0, deadline - time.monotonic()))
            except subprocess.TimeoutExpired:
                status = "timeout"
    finally:
        if p.poll() is None:
            # bwrap leads its own session; --die-with-parent and the pid namespace take the
            # engine down with it.
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            p.wait()
        p.stdout.close()
        p.stderr.close()
    return status, bytes(out), bytes(err), p.returncode


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 16), b""):
            h.update(block)
    return h.hexdigest()


_pinned = {}
_pin_lock = threading.Lock()


def pinned(path, pin):
    """True when path hashes to pin. Cached per (path, pin): the install directory is root-owned
    and read-only to the service, and the service runs it read-only bound."""
    key = (path, pin)
    with _pin_lock:
        if key not in _pinned:
            try:
                _pinned[key] = sha256_file(path) == pin
            except OSError:
                _pinned[key] = False
        return _pinned[key]


def last_statement(expr):
    """Text after the last top-level ';' (outside brackets, braces, parens and strings)."""
    depth, quoted, cut, i = 0, False, 0, 0
    while i < len(expr):
        c = expr[i]
        if quoted:
            if c == "\\":
                i += 1
            elif c == '"':
                quoted = False
        elif c == '"':
            quoted = True
        elif c in "[{(":
            depth += 1
        elif c in "]})":
            depth -= 1
        elif c == ";" and depth == 0:
            cut = i + 1
        i += 1
    return expr[cut:]


def parse_engine(expr, text):
    """Split engine stdout into (status, value, messages, error). Success is only a value with no
    engine message, no $Failed and not an unevaluated echo of the input's head."""
    if text.startswith(OUT_MARK):
        at = 0
    else:
        at = text.find("\n" + OUT_MARK)
        at = at + 1 if at >= 0 else -1
    head_text = text[:at] if at >= 0 else text
    messages = [ln for ln in head_text.splitlines() if MESSAGE.match(ln)]
    if at < 0:
        return "unsupported", "", messages, "the engine printed no result"
    value = text[at + len(OUT_MARK):].rstrip("\n")
    if messages:
        return "unsupported", value, messages, "the engine reported: " + messages[0][:200]
    if "$Failed" in value:
        return "unsupported", value, messages, "the engine returned $Failed"
    m = HEAD.match(last_statement(expr))
    if m and value.startswith(m.group(1) + "["):
        return "unsupported", value, messages, "%s stayed unevaluated" % m.group(1)
    return "ok", value, messages, None


def math_result(status, output="", error=None, messages=()):
    r = {"ok": status == "ok", "status": status, "output": output, "messages": list(messages)}
    if error:
        r["error"] = error
    return r


def checked_input(expression):
    """(data, None) for an expression within the byte bounds, else (None, refusal)."""
    if not isinstance(expression, str):
        return None, math_result("refused", error="expression must be a string")
    if "\0" in expression:
        return None, math_result("refused", error="expression contains a NUL byte")
    try:
        data = expression.encode("utf-8")
    except UnicodeEncodeError:
        return None, math_result("refused", error="expression is not valid Unicode")
    if not expression.strip():
        return None, math_result("refused", error="empty expression")
    if len(data) > MAX_EXPR:
        return None, math_result("refused", error="expression longer than %d bytes" % MAX_EXPR)
    return data, None


ADMIT_REFUSAL = "outside the admitted mathematics (math-admit refused it)"


def admit(data):
    """None when math-admit admits data (exit 0, nothing on stdout), else the refusal or error."""
    status, out, err, rc = run_bounded(math_argv(MATH_ADMIT, (), "/math-admit"), data, ADMIT_WALL_S,
                                       4096, MATH_ERR_CAP, math_limits)
    if status != "done":
        return math_result("error", error="the admission gate failed: " + status)
    if rc == 2 and not out:
        return math_result("refused", error=ADMIT_REFUSAL)
    if rc != 0 or out:
        return math_result("error", error="the admission gate failed: exit %d" % rc)
    return None


def evaluate(expression, data):
    """The engine run alone, without the gate; run_math is the only production caller."""
    status, out, err, rc = run_bounded(math_argv(AIEN_MATH), data, MATH_WALL_S, MATH_OUT_CAP,
                                       MATH_ERR_CAP, math_limits)
    if status == "timeout":
        return math_result("timeout", error="the engine ran past %d s" % MATH_WALL_S)
    if status == "too_large":
        return math_result("too_large", error="the engine printed more than %d bytes" % MATH_OUT_CAP)
    if rc != 0:
        why = ("memory pool exhausted" if rc == 3 else "trap %d" % (rc - 100) if 100 <= rc < 200
               else "CPU limit" if rc in (-signal.SIGXCPU, -signal.SIGKILL) else "exit %d" % rc)
        return math_result("error", error="the engine stopped: " + why)
    try:
        text = out.decode("utf-8")
    except UnicodeDecodeError:
        return math_result("error", error="the engine printed invalid UTF-8")
    st, value, messages, error = parse_engine(expression, text)
    return math_result(st, value, error, messages)


def run_math(expression):
    """Admit, then evaluate one expression with the pinned aien-math. Never raises for gate or
    engine behaviour."""
    data, refusal = checked_input(expression)
    if refusal:
        return refusal
    if not os.access(BWRAP, os.X_OK):
        return math_result("error", error="sandbox missing")
    if not pinned(MATH_ADMIT, MATH_ADMIT_PIN):
        return math_result("error", error="admission gate does not match its pin")
    if not pinned(AIEN_MATH, AIEN_MATH_PIN):
        return math_result("error", error="engine does not match its pin")
    if not MATH_SLOTS.acquire(blocking=False):
        return math_result("busy", error="the engine is busy")
    try:
        return admit(data) or evaluate(expression, data)
    finally:
        MATH_SLOTS.release()


# ---- Omega compile path: verbatim from allen_chat.py (sandbox_limits, sandboxed, signatures,
# OMC_HINTS, OMC_ERR, TYPES, NAME, MAX_SOURCE and the omega() handler body). ----

MAX_SOURCE = 16384
OMC_HINTS = {
    3: "An unknown name or a word IR mode refuses: floats, host words, continue, sizeof, or a call to "
       "a function that is not defined earlier (no recursion, no forward calls).",
    4: "The line ended before the statement was complete.",
    5: "Extra words at the end of the line: every op takes a fixed number of values.",
    6: "A number is malformed: digits only, at most 20, with an optional leading minus.",
    7: "Types do not match or a value is used the wrong way. There are no implicit conversions: "
       "convert with the type word, for example ulong x.",
    8: "A number is outside the range of its type.",
    9: "Not allowed here: a call cycle or recursion (call only earlier functions), struct, global "
       "or text in IR mode, more than 6 parameters, or break outside a loop.",
    10: "A name is reused: no shadowing, and a name cannot repeat a function name.",
    11: "Ambiguous number: two bare numbers in one op, or a bare number with no type around it. "
        "Write the type, for example ulong 1.",
    12: "Too many live values for the 16 registers. Split the function into smaller earlier helpers.",
    13: "A name is too long (at most 8 characters) or a function is unknown.",
    14: "A capacity was exceeded.",
    15: "A block is missing its end.",
    16: "A word was not understood where a type or value was expected.",
    17: "A line follows ret, break or trap in the same block and can never run.",
    18: "The function can reach its end without ret.",
    22: "A text literal has a bad escape.",
}
OMC_ERR = re.compile(rb"omc: line (\d+): error (\d+)")
TYPES = {"bool", "uchar", "ushort", "uint", "ulong", "byte", "short", "int", "long"}
NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_]{0,62}$")


def sandbox_limits():
    os.setsid()
    resource.setrlimit(resource.RLIMIT_CPU, (4, 5))
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (4 << 20, 4 << 20))
    resource.setrlimit(resource.RLIMIT_NOFILE, (64, 64))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def sandboxed(tool, data, wall_s, out_cap, args=(), argv0=None):
    """Run one pinned tool under bwrap: no network, no host files, read-only root,
    a small empty tmpfs, its own pid namespace, no /proc, CPU/memory/file-size limits."""
    argv = [
        BWRAP, "--unshare-all", "--die-with-parent", "--new-session", "--clearenv",
        "--cap-drop", "ALL", "--ro-bind", tool, "/tool", "--size", str(8 << 20), "--tmpfs", "/tmp",
        "--dev", "/dev", "--remount-ro", "/", "--chdir", "/tmp",
    ] + (["--argv0", argv0] if argv0 else []) + ["--", "/tool"] + list(args)
    p = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE, preexec_fn=sandbox_limits, close_fds=True)
    try:
        out, err = p.communicate(data, timeout=wall_s)
    except subprocess.TimeoutExpired:
        os.killpg(p.pid, signal.SIGKILL)
        p.communicate()
        return None, b"timeout", -9
    if len(out) > out_cap:
        return None, b"output too large", -1
    return out, err[:4096], p.returncode


def signatures(source):
    fns = []
    for line in source.splitlines():
        w = line.split()
        if len(w) < 3 or w[0] != "fn" or "to" not in w:
            continue
        cut = w.index("to")
        name, params, ret = w[1], w[2:cut], w[cut + 1:]
        if not NAME.match(name) or len(params) % 2 or len(ret) != 1:
            return None
        ps = []
        for i in range(0, len(params), 2):
            if params[i + 1] not in TYPES:
                return None
            ps.append({"name": params[i], "type": params[i + 1]})
        if ret[0] not in TYPES and ret[0] != "void":
            return None
        fns.append({"name": name, "params": ps, "ret": ret[0]})
    return fns


def omega_refusal(stage, message, hint="", code=None):
    return {"ok": False, "stage": stage, "code": code, "line": None, "message": message, "hint": hint}


def run_omega(source):
    """Compile Omega source to WebAssembly exactly as the gateway's /omega route does. Returns the
    gateway's JSON body as a dict; input and busy refusals use stage "input" and "busy"."""
    if not isinstance(source, str) or not source.strip():
        return omega_refusal("input", "Send the Omega source as a string.")
    if len(source.encode("utf-8", "replace")) > MAX_SOURCE:
        return omega_refusal("input", "Omega source is limited to 16 KiB here.")
    if not os.access(BWRAP, os.X_OK):
        return omega_refusal("sandbox", "The compile sandbox is missing.")
    if not (pinned(OMC, OMC_PIN) and pinned(IRWASM, IRWASM_PIN)):
        return omega_refusal("sandbox", "The compiler does not match its pin.")
    if not OMEGA_SLOTS.acquire(blocking=False):
        return omega_refusal("busy", "The compiler is busy. Try again in a moment.")
    try:
        body = source.replace("\r\n", "\n")
        if body.split("\n", 1)[0].strip() == "ir":
            body = body.split("\n", 1)[1] if "\n" in body else ""
        t0 = time.monotonic()
        # The pinned omc's IR mode skips checks its native mode makes, so the same unit must first
        # pass the native function-mode compile (its machine words are discarded), then IR.
        for stage, data, shift in (("omc", body.encode("utf-8"), 0),
                                   ("omc-ir", b"ir\n" + body.encode("utf-8"), 1)):
            out, err, rc = sandboxed(OMC, data, 5, 4 << 20)
            if rc == 0 and out:
                ir = out
                continue
            m = OMC_ERR.search(err or b"")
            code = int(m.group(2)) if m else None
            line = int(m.group(1)) - shift if m else None
            msg = ("omc: line %d: error %d" % (line, code)) if m else (
                "the compiler stopped (%s)" % (err.decode("utf-8", "replace").strip() or "exit %d" % rc))
            if stage == "omc-ir" and m:
                msg += " (IR mode)"
            return {"ok": False, "stage": "omc", "code": code, "line": line, "message": msg,
                    "hint": OMC_HINTS.get(code, "See the Omegese guide.")}
        wasm, err, rc = sandboxed(IRWASM, ir, 5, 1 << 20)
        if rc != 0 or not wasm or wasm[:4] != b"\0asm":
            return {"ok": False, "stage": "irwasm", "code": rc, "line": None,
                    "message": "irwasm: " + (err.decode("utf-8", "replace").strip() or "exit %d" % rc),
                    "hint": "The IR translator refused this unit; keep to scalar "
                            "functions that call only earlier functions."}
        fns = signatures(source)
        if not fns:
            return {"ok": False, "stage": "limit", "code": None, "line": None,
                    "message": "No callable function signature found.",
                    "hint": "Write functions as: fn name a ulong b ulong to ulong"}
        return {"ok": True, "wasm": base64.b64encode(wasm).decode(), "functions": fns,
                "compile_ms": int((time.monotonic() - t0) * 1000),
                "omc_sha256": OMC_PIN, "irwasm_sha256": IRWASM_PIN}
    finally:
        OMEGA_SLOTS.release()


SELFTEST_MATH = ("Expand[(a+b)^2]", "a^2 + 2*a*b + b^2")
SELFTEST_OMEGA = "fn f a uint to uint\nret a\nend\n"
SELFTEST_REFUSED = ("Run[x]", "URLRead[x]")


def selftest():
    """Problems that must stop the service from starting; empty when both paths work."""
    problems = []
    if not os.access(BWRAP, os.X_OK):
        return ["%s missing" % BWRAP]
    for path, pin in ((MATH_ADMIT, MATH_ADMIT_PIN), (AIEN_MATH, AIEN_MATH_PIN), (OMC, OMC_PIN),
                      (IRWASM, IRWASM_PIN)):
        if not pinned(path, pin):
            problems.append("%s does not hash to %s" % (path, pin))
    if problems:
        return problems
    r = run_math(SELFTEST_MATH[0])
    if not r["ok"] or r["output"] != SELFTEST_MATH[1]:
        problems.append("math sandbox fixture failed: %r" % (r,))
    for unsafe in SELFTEST_REFUSED:
        r = run_math(unsafe)
        if r["status"] != "refused" or r.get("error") != ADMIT_REFUSAL:
            problems.append("admission gate did not refuse %s: %r" % (unsafe, r))
    o = run_omega(SELFTEST_OMEGA)
    if not o.get("ok"):
        problems.append("omega sandbox fixture failed: %r" % (o,))
    return problems
