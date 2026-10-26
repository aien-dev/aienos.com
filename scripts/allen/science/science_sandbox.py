"""Isolated execution for the allen-science service.

run_math(expression) first checks that the expression is exactly one bracket-balanced expression
and runs the pinned admission gate math-admit (seeded Omega, math-admit.om) on it; only an
expression it admits (exit 0, empty stdout) reaches the pinned aien-math engine (`aien-math eval`,
code on stdin), spliced into the internal completeness wrapper WRAP and evaluated once. Both
programs run in the same bubblewrap sandbox:
every namespace unshared (no network at all, not even loopback), nested user namespaces disabled,
cleared environment, the one binary as the only read-only bind, a small private tmpfs at /tmp, no
/dev, no /proc. Resource limits are set by the fixed /usr/bin/prlimit argv in front of bwrap (no
Python code runs between fork and exec, so a request thread cannot hang in the child before the
deadline starts). stdin is fed and stdout/stderr are collected incrementally under one deadline;
the sandbox is killed past the stdout cap and only a bounded stderr prefix is kept. No shell is
involved anywhere: argv is a fixed list.

The engine's provenance manifest (engine.json: commit and binary sha256) is required and must name
exactly the pinned engine; a missing or stale manifest refuses every math request and selftest().

run_omega(source) is the gateway's Omega compile path (allen_chat.py): the same bwrap argv, the
same per-process limits, the same two-stage omc then omc-ir compile and irwasm, and the same
response dicts; only the collection differs (bounded and incremental here, where the gateway uses
communicate()). test_science_sandbox.py checks the argv and limits against the gateway.

selftest() verifies the four pins and the manifest, runs one fixture through each path and checks
that the gate refuses Run[x] and URLRead[x]; the service calls it at startup and refuses to start
when it returns problems.
"""

import base64
import hashlib
import json
import os
import re
import selectors
import subprocess
import threading
import time

ROOT = "/opt/allen-science"
BWRAP = "/usr/bin/bwrap"
PRLIMIT = "/usr/bin/prlimit"
AIEN_MATH = ROOT + "/aien-math"
OMC = ROOT + "/omc"
IRWASM = ROOT + "/irwasm"
MATH_ADMIT = ROOT + "/math-admit"
ENGINE_MANIFEST = ROOT + "/engine.json"

# aien-math main 43c629db (clean tree, reproducible) built with omc e623d440 (omega-mv 3d000060);
# frozen copy /tmp/allen-science-math-release/build/aien-math. The commit and the sha256 are one
# pin: engine.json must state both exactly.
ENGINE_COMMIT = "43c629db362b48aeec3590a8f0cd30e927e5ec16"
AIEN_MATH_PIN = "a3c538e9e2c8b255ba3daf4cf5a486b379e2722e86a177c173ff2d097e9ddd59"
ENGINE_OMC_PIN = "e623d440956b1473ca32da85a019cbb3c953d4bdd6e384952d9989e8039425e3"
# math-admit.om (sha256 d064507f...5931) compiled with the gateway's omc (OMC_PIN); exit 0 admit,
# 2 refuse, 3 IO. This build refuses ++/--, a top-level ',' and the wrapper's names (WRAP).
MATH_ADMIT_PIN = "04c58b6861d15f21d038aff560aa1ae6eeadd76387154f3ada208b73d43632c9"
# The gateway's pins (allen_chat.py OMC_PIN, IRWASM_PIN).
OMC_PIN = "734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61"
IRWASM_PIN = "2d235a70c39ac1461a09c41e10e058d962b767ae83893cdcfc693ef6c47182e6"

MATH_ARGV = ("eval",)          # code on stdin: no argv quoting, nothing visible in the host's ps
MAX_EXPR = 4096                # bytes of UTF-8 accepted from the service
ADMIT_WALL_S = 2
MATH_WALL_S = 8                # wall clock; spinning code uses about 60% CPU, so this fires first
RUN_WALL_S = ADMIT_WALL_S + MATH_WALL_S  # one deadline for a whole run_math
MATH_OUT_CAP = 64 << 10        # stdout bytes; past this the sandbox is killed
ERR_CAP = 4096                 # stderr bytes kept (the rest is drained and dropped)
MATH_TMPFS = 8 << 20
# The engine's static RW segment is about 978 MiB of zero pages (readelf: 0x3d220ee0), so the
# address-space limit must sit above it; its pools are fixed, exhaustion exits 3.
MATH_AS = 1152 << 20
MATH_LIMITS = ("--cpu=5:6", "--as=%d" % MATH_AS, "--fsize=%d" % (1 << 20), "--nofile=32", "--core=0")
# The gateway's sandbox_limits(): CPU 4/5 s, 512 MiB, 4 MiB files, 64 descriptors, no core.
OMEGA_LIMITS = ("--cpu=4:5", "--as=%d" % (512 << 20), "--fsize=%d" % (4 << 20), "--nofile=64",
                "--core=0")
MATH_SLOTS = threading.BoundedSemaphore(2)
OMEGA_SLOTS = threading.BoundedSemaphore(2)

OUT_MARK = "Out[1]= "
# The pinned printer (src/print/pr.om pr_trunc at 43c629db) cuts any result over 10005 code points
# to its first 4998 and last 4997 around " ... ": exactly 10000. Anything this long may be cut.
TRUNCATED_AT = 10000
# Admitted heads that ask the engine to do something. One left anywhere in the result means that
# part was not done; mathematical functions (Sin, Exp, Gamma, ...) are legitimate in results.
EVAL_HEADS = frozenset("""
N D Integrate NIntegrate Limit Series Normal Sum Product Solve Reduce Simplify FullSimplify Expand
Factor Together Apart Cancel Numerator Denominator Coefficient CoefficientList Collect Det Inverse
Transpose Eigenvalues Eigenvectors EigenSystem Eigensystem MatrixRank RowReduce LinearSolve Dot Tr
DSolve DSolveValue NDSolve Mean Median Variance StandardDeviation Covariance Correlation PDF CDF
Quantile Probability Expectation Length Total Range Table First Chop SingularValueList MatrixExp
IdentityMatrix DiagonalMatrix UnitConvert UnitSimplify QuantityMagnitude QuantityUnit
""".split())
EVAL_HEAD = re.compile(r"(?<![A-Za-z0-9$`])(%s)\[" % "|".join(sorted(EVAL_HEADS, key=len, reverse=True)))


def limited(limits, argv):
    """argv run under fixed rlimits: prlimit sets them on itself, then execs argv."""
    return [PRLIMIT] + list(limits) + ["--"] + argv


def math_argv(tool, args=MATH_ARGV, dest="/aien-math"):
    """bwrap argv for one run; `tool` is bound read-only at dest and is all there is."""
    return [
        BWRAP, "--unshare-all", "--unshare-user", "--disable-userns", "--die-with-parent",
        "--new-session", "--clearenv", "--cap-drop", "ALL",
        "--ro-bind", tool, dest, "--size", str(MATH_TMPFS), "--tmpfs", "/tmp",
        "--remount-ro", "/", "--chdir", "/tmp", "--", dest,
    ] + list(args)


def run_bounded(argv, data, wall_s, out_cap, err_cap):
    """Run argv with data on stdin; returns (status, stdout, stderr, rc). status is "done",
    "timeout" or "too_large". stdin is written non-blocking alongside the reads, so any input size
    is fed under the same deadline. The process group is always killed and reaped."""
    deadline = time.monotonic() + wall_s
    p = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         start_new_session=True, close_fds=True)
    status, out, err = "done", bytearray(), bytearray()
    view, pos = memoryview(data), 0
    sel = selectors.DefaultSelector()
    try:
        if data:
            os.set_blocking(p.stdin.fileno(), False)
            sel.register(p.stdin, selectors.EVENT_WRITE)
        else:
            p.stdin.close()
        sel.register(p.stdout, selectors.EVENT_READ, out)
        sel.register(p.stderr, selectors.EVENT_READ, err)
        while sel.get_map():
            left = deadline - time.monotonic()
            if left <= 0:
                status = "timeout"
                break
            for key, _ in sel.select(left):
                if key.fileobj is p.stdin:
                    try:
                        pos += os.write(key.fd, view[pos:pos + 65536])
                    except BlockingIOError:
                        continue
                    except BrokenPipeError:
                        pos = len(view)
                    if pos >= len(view):
                        sel.unregister(p.stdin)
                        p.stdin.close()
                    continue
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
        if status == "done":
            try:
                p.wait(max(0.0, deadline - time.monotonic()))
            except subprocess.TimeoutExpired:
                status = "timeout"
    finally:
        sel.close()
        if p.poll() is None:
            # prlimit execs bwrap in place, so p.pid leads the sandbox's session; --die-with-parent
            # and the pid namespace take everything inside down with it.
            try:
                os.killpg(p.pid, 9)
            except ProcessLookupError:
                pass
            p.wait()
        for f in (p.stdin, p.stdout, p.stderr):
            try:
                f.close()
            except OSError:
                pass
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


def manifest_problem(path=None):
    """None when the engine manifest names exactly the pinned engine, else why not."""
    path = path or ENGINE_MANIFEST
    try:
        with open(path, "rb") as f:
            m = json.loads(f.read(65536))
    except (OSError, ValueError) as e:
        return "engine manifest %s unreadable: %s" % (path, type(e).__name__)
    # engine.json is shared with ScienceMath's corpus receipt; these four fields are the identity.
    want = {"engine": "aien-dev/aien-math", "commit": ENGINE_COMMIT, "binary_sha256": AIEN_MATH_PIN,
            "omc_sha256": ENGINE_OMC_PIN}
    if not isinstance(m, dict):
        return "engine manifest %s is not an object" % path
    wrong = sorted(k for k, v in want.items() if m.get(k) != v)
    if wrong:
        return "engine manifest %s does not match the pinned engine: %s" % (path, ", ".join(wrong))
    return None


_manifest = {}


def manifest_ok():
    with _pin_lock:
        if ENGINE_MANIFEST not in _manifest:
            _manifest[ENGINE_MANIFEST] = manifest_problem() is None
        return _manifest[ENGINE_MANIFEST]


# ScienceMath's completeness wrapper for aien-math 43c629db (no engine change): the expression is
# evaluated once inside With; HoldComplete keeps the input's structure (List[x] and {x} alike, no
# Evaluate or Sequence splicing) for an exact SameQ test, and the FreeQ guard marks results that
# still hold an operation, or the derivative pattern behind the known wrong Limit answers.
WRAP = ("With[{aienR = ", "}, If[HoldComplete[", "] =!= HoldComplete[aienR] && FreeQ[HoldComplete[aienR], "
        "Integrate|D|DSolve|NDSolve|Limit|Sum|Product|Solve|NSolve|Series|Simplify|FullSimplify|"
        "Expectation|Probability|Eigensystem|SingularValueDecomposition|MatrixExp|UnitConvert|"
        "Derivative[_][Abs|Sign|Floor|Ceiling|Round|UnitStep][_]], aienComputed[aienR], aienUnsupported]]")
COMPUTED = ("aienComputed[", "]")
UNSUPPORTED = "aienUnsupported"


def single_expression(expr):
    """True when expr can be spliced into WRAP as exactly one expression: brackets balanced and
    nested correctly outside strings, no top-level ',' or ';', no ';' anywhere, no wrapper names.
    math-admit enforces the same; this is the helper's own boundary before splicing."""
    if ";" in expr or "aien" in expr:
        return False
    stack, quoted, i = [], False, 0
    pairs = {"]": "[", "}": "{", ")": "("}
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
            stack.append(c)
        elif c in pairs:
            if not stack or stack.pop() != pairs[c]:
                return False
        elif c == "," and not stack:
            return False
        i += 1
    return not stack and not quoted


def wrapped(data):
    return (WRAP[0] + "%s" + WRAP[1] + "%s" + WRAP[2]).encode() % (data, data)


def parse_engine(text):
    """Split the wrapper's engine stdout into (status, value, messages, error). Success is only
    exactly "Out[1]= aienComputed[VALUE]" with no line containing "::", a value short enough that
    the printer did not cut it, no $Failed/$Aborted and no evaluation request (Integrate[, ...)
    left anywhere in it."""
    messages = [ln for ln in text.splitlines() if "::" in ln]
    if messages:
        return "unsupported", "", messages, "the engine reported: " + messages[0][:200]
    body = text.rstrip("\n")
    if not body.startswith(OUT_MARK):
        return "unsupported", "", messages, "the engine printed no result"
    value = body[len(OUT_MARK):]
    if value == UNSUPPORTED:
        return "unsupported", "", messages, "the engine left the expression or an operation unevaluated"
    if len(value) >= TRUNCATED_AT:
        return "unsupported", "", messages, "the result is too long to print whole (the engine cut it)"
    if not (value.startswith(COMPUTED[0]) and value.endswith(COMPUTED[1])):
        return "unsupported", "", messages, "the engine printed an unexpected result"
    value = value[len(COMPUTED[0]):-len(COMPUTED[1])]
    if "$Failed" in value or "$Aborted" in value:
        return "unsupported", value, messages, "the engine returned $Failed"
    m = EVAL_HEAD.search(value)
    if m:
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
    if not single_expression(expression):
        return None, math_result("refused", error="not exactly one bracket-balanced expression")
    return data, None


ADMIT_REFUSAL = "outside the admitted mathematics (math-admit refused it)"


def left_until(deadline, cap):
    return min(cap, deadline - time.monotonic())


def admit(data, deadline=None):
    """None when math-admit admits data (exit 0, nothing on stdout), else the refusal or error."""
    deadline = deadline or time.monotonic() + RUN_WALL_S
    status, out, err, rc = run_bounded(limited(MATH_LIMITS, math_argv(MATH_ADMIT, (), "/math-admit")),
                                       data, left_until(deadline, ADMIT_WALL_S), 4096, ERR_CAP)
    if status != "done":
        return math_result("error", error="the admission gate failed: " + status)
    if rc == 2 and not out:
        return math_result("refused", error=ADMIT_REFUSAL)
    if rc != 0 or out:
        return math_result("error", error="the admission gate failed: exit %d" % rc)
    return None


def engine_text(data, wall_s=MATH_WALL_S):
    """(stdout text, None) from one engine run, or (None, failure result)."""
    status, out, err, rc = run_bounded(limited(MATH_LIMITS, math_argv(AIEN_MATH)), data, wall_s,
                                       MATH_OUT_CAP, ERR_CAP)
    if status == "timeout":
        return None, math_result("timeout", error="the engine ran past %d s" % MATH_WALL_S)
    if status == "too_large":
        return None, math_result("too_large", error="the engine printed more than %d bytes" % MATH_OUT_CAP)
    if rc != 0:
        why = ("memory pool exhausted" if rc == 3 else "trap %d" % (rc - 100) if 100 <= rc < 200
               else "CPU limit" if rc in (-24, -9) else "exit %d" % rc)
        return None, math_result("error", error="the engine stopped: " + why)
    try:
        return out.decode("utf-8"), None
    except UnicodeDecodeError:
        return None, math_result("error", error="the engine printed invalid UTF-8")


def evaluate(expression, data, deadline=None):
    """One engine run of the expression inside WRAP (run_math is the only production caller; the
    caller has checked single_expression). The wrapper is internal: the expression alone is what
    the service records as provenance."""
    deadline = deadline or time.monotonic() + RUN_WALL_S
    wall = left_until(deadline, MATH_WALL_S)
    if wall <= 0:
        return math_result("timeout", error="the request ran past its time budget")
    text, failed = engine_text(wrapped(data), wall)
    if failed:
        return failed
    st, value, messages, error = parse_engine(text)
    return math_result(st, value, error, messages)


def run_math(expression):
    """Admit, then evaluate one expression with the pinned aien-math. Never raises for gate or
    engine behaviour."""
    data, refusal = checked_input(expression)
    if refusal:
        return refusal
    if not (os.access(BWRAP, os.X_OK) and os.access(PRLIMIT, os.X_OK)):
        return math_result("error", error="sandbox missing")
    if not pinned(MATH_ADMIT, MATH_ADMIT_PIN):
        return math_result("error", error="admission gate does not match its pin")
    if not pinned(AIEN_MATH, AIEN_MATH_PIN):
        return math_result("error", error="engine does not match its pin")
    if not manifest_ok():
        return math_result("error", error="engine manifest missing or stale")
    if not MATH_SLOTS.acquire(blocking=False):
        return math_result("busy", error="the engine is busy")
    try:
        deadline = time.monotonic() + RUN_WALL_S
        return admit(data, deadline) or evaluate(expression, data, deadline)
    finally:
        MATH_SLOTS.release()


# ---- Omega compile path: the gateway's (allen_chat.py: sandboxed, signatures, OMC_HINTS, OMC_ERR,
# TYPES, NAME, MAX_SOURCE and the omega() handler body), with bounded collection. ----

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


def sandbox_argv(tool, args=(), argv0=None):
    """The gateway's bwrap argv for omc and irwasm, unchanged."""
    return [
        BWRAP, "--unshare-all", "--die-with-parent", "--new-session", "--clearenv",
        "--cap-drop", "ALL", "--ro-bind", tool, "/tool", "--size", str(8 << 20), "--tmpfs", "/tmp",
        "--dev", "/dev", "--remount-ro", "/", "--chdir", "/tmp",
    ] + (["--argv0", argv0] if argv0 else []) + ["--", "/tool"] + list(args)


def sandboxed(tool, data, wall_s, out_cap, args=(), argv0=None):
    """The gateway's sandboxed(): same argv, limits and return values (out, err, rc), with stdout
    capped while it is read and stderr kept to its first 4096 bytes."""
    status, out, err, rc = run_bounded(limited(OMEGA_LIMITS, sandbox_argv(tool, args, argv0)), data,
                                       wall_s, out_cap, ERR_CAP)
    if status == "timeout":
        return None, b"timeout", -9
    if status == "too_large":
        return None, b"output too large", -1
    return out, err, rc


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
    if not (os.access(BWRAP, os.X_OK) and os.access(PRLIMIT, os.X_OK)):
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
    for tool in (BWRAP, PRLIMIT):
        if not os.access(tool, os.X_OK):
            return ["%s missing" % tool]
    problems = []
    for path, pin in ((MATH_ADMIT, MATH_ADMIT_PIN), (AIEN_MATH, AIEN_MATH_PIN), (OMC, OMC_PIN),
                      (IRWASM, IRWASM_PIN)):
        if not pinned(path, pin):
            problems.append("%s does not hash to %s" % (path, pin))
    m = manifest_problem()
    if m:
        problems.append(m)
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
