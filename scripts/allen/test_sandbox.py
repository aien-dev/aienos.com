#!/usr/bin/env python3
"""Escape tests for the allen-chat compile sandbox.

Runs a static busybox (a verification tool, never shipped) through the exact
sandboxed() call the gateway uses for omc and irwasm, and checks each guard
with a negative control: the same probe outside the sandbox, or a permitted
action inside it, must succeed, so a pass is not just a missing applet.
Usage: test_sandbox.py [busybox path]. In probe scripts "@" stands for the
busybox binary, because the sandbox has no PATH and no /bin.
"""

import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from allen_chat import sandboxed  # noqa: E402

BB = sys.argv[1] if len(sys.argv) > 1 else "/usr/bin/busybox"
results = []
PRE = 'b() { (exec -a busybox /tool "$@"); }; '


def inside(script, wall=5):
    t0 = time.monotonic()
    out, err, rc = sandboxed(BB, b"", wall, 1 << 20, ["sh", "-c", PRE + script.replace("@", "b ")],
                             argv0="busybox")
    return out or b"", err or b"", rc, time.monotonic() - t0


def outside(script):
    p = subprocess.run([BB, "sh", "-c", script.replace("@", BB + " ")], capture_output=True, timeout=20)
    return p.stdout, p.stderr, p.returncode


def check(name, ok, detail):
    results.append((name, ok))
    print("%s %s: %s" % ("PASS" if ok else "FAIL", name, detail))


# 0. Control: the sandbox runs busybox at all.
o, e, rc, _ = inside("@echo ALIVE")
check("sandbox runs probes (control)", o.strip() == b"ALIVE", "rc=%s out=%r err=%r" % (rc, o, e[:80]))

# 1. Network: the model server on host loopback and the hub over the tailnet.
for host, port in (("127.0.0.1", "18091"), ("100.64.0.3", "443")):
    probe = "@echo | @nc -w 2 %s %s; echo rc=$?" % (host, port)
    o, e, rc, _ = inside(probe)
    co, ce, crc = outside(probe)
    check("network denied to %s:%s" % (host, port), b"rc=0" not in o and b"rc=" in o,
          "inside %r %r; control outside %r" % (o.strip(), e.strip()[:60], co.strip()))
    check("network control %s:%s reachable outside" % (host, port), b"rc=0" in co, repr(co.strip()))

# 2. Host files are invisible.
probe = "@ls -d /home /etc/passwd /tmp/aien-gb10.lock /usr 2>&1; echo END"
o, e, rc, _ = inside(probe)
co, _, _ = outside(probe)
seen = [p for p in (b"/home", b"/etc/passwd", b"/usr") if (p + b"\n") in o]
check("host files invisible", not seen and b"END" in o,
      "inside saw %r; control outside listed /home=%s" % (seen, b"/home\n" in co))

# 3. Writes outside the tmpfs are refused; writes inside it work (control).
o, e, rc, _ = inside("echo x > /evil 2>/dev/null && echo ROOTW; echo x > /tool 2>/dev/null && echo TOOLW; "
                     "echo x > /tmp/ok && echo TMPW")
check("root and tool read-only", b"ROOTW" not in o and b"TOOLW" not in o, "out=%r" % o)
check("tmpfs writable (control)", b"TMPW" in o, "out=%r" % o)

# 4. File size caps: a 16 MiB write stops at the 4 MiB RLIMIT_FSIZE / 8 MiB tmpfs.
o, e, rc, _ = inside("@dd if=/dev/zero of=/tmp/big bs=1M count=16 2>/dev/null; @wc -c < /tmp/big")
size = int(o.split()[-1]) if o.split() else -1
check("file size capped", 0 < size <= 8 << 20, "wrote %d bytes of 16 MiB" % size)

# 5. CPU spin is killed by the CPU limit / wall clock.
o, e, rc, dt = inside("while :; do :; done", wall=8)
check("cpu spin killed", rc != 0 and 3 < dt < 7, "rc=%s after %.1fs (RLIMIT_CPU 4 s)" % (rc, dt))

# 6. Sleeping past the wall clock is killed too (the CPU limit does not catch it).
o, e, rc, dt = inside("@sleep 30; echo SLEPT", wall=3)
check("wall clock kill", rc == -9 and dt < 5 and b"SLEPT" not in o, "rc=%s after %.1fs" % (rc, dt))

# 7. Background children die with the sandbox (pid namespace, die-with-parent).
import threading
marker = "%d.25" % (100000 + os.getpid() % 100000)
seen_running = []
def watch():
    time.sleep(1.0)
    seen_running.append(subprocess.run(["pgrep", "-f", "sleep " + marker], capture_output=True).stdout.strip())
w = threading.Thread(target=watch)
w.start()
inside("for i in 1 2 3 4 5; do @sleep %s & done; @sleep 30" % marker, wall=2)
w.join()
time.sleep(1)
left = subprocess.run(["pgrep", "-f", "sleep " + marker], capture_output=True).stdout.strip()
check("orphan control: children visible while the sandbox runs", bool(seen_running and seen_running[0]),
      "pgrep saw %d processes" % len((seen_running or [b""])[0].split()))
check("no orphans survive", not left, "pgrep found %r" % left)

# 8. Memory cap: a 1 GiB allocation fails under RLIMIT_AS 512 MiB; 64 MiB works (control).
o, e, rc, _ = inside("v=$(@head -c 1073741824 /dev/zero | @tr '\\0' a); echo ALLOC ${#v}", wall=8)
check("memory capped", b"ALLOC 1073741824" not in o and rc is not None, "rc=%s out=%r err=%r" % (rc, o[:40], e[:60]))
o, e, rc, _ = inside("v=$(@head -c 67108864 /dev/zero | @tr '\\0' a); echo ALLOC ${#v}", wall=8)
check("64 MiB allocation works (control)", b"ALLOC 67108864" in o, "rc=%s out=%r" % (rc, o[:40]))

# 9. Environment is cleared (no inherited secrets).
os.environ["ALLENCHAT_CANARY"] = "should-not-leak"
o, e, rc, _ = inside("@env; echo END")
check("environment cleared", b"ALLENCHAT_CANARY" not in o and b"END" in o, "env=%r" % o[:120])

# 10. No /proc: host processes and kernel state are not visible.
o, e, rc, _ = inside("@ls /proc 2>&1; echo END")
check("no /proc in the sandbox", b"END" in o and b"self" not in o, "out=%r" % o[:80])

# 11. Output floods are cut while they are read, not after: stdout past the cap ends the run.
t0 = time.monotonic()
o, e, rc = sandboxed(BB, b"", 5, 1 << 20, ["sh", "-c", PRE + "b yes"], argv0="busybox")
dt = time.monotonic() - t0
check("stdout flood cut at the cap", o is None and e == b"output too large" and rc == -1 and dt < 3,
      "rc=%s err=%r after %.1fs" % (rc, e, dt))
# 12. A stderr flood keeps only 4096 bytes and still ends at the wall clock.
t0 = time.monotonic()
o, e, rc = sandboxed(BB, b"", 2, 1 << 20, ["sh", "-c", PRE + "b yes >&2"], argv0="busybox")
dt = time.monotonic() - t0
check("stderr flood bounded and timed out", rc == -9 and e == b"timeout" and dt < 4, "rc=%s after %.1fs" % (rc, dt))
o, e, rc = sandboxed(BB, b"", 5, 1 << 20, ["sh", "-c", PRE + "b head -c 100000 /dev/zero | b tr '\\0' e >&2"],
                     argv0="busybox")
check("stderr kept to its first 4096 bytes (control: the run completes)", rc == 0 and len(e) == 4096,
      "rc=%s stderr %d bytes" % (rc, len(e)))
# 13. 8 MiB of stdin is fed in full (control for the non-blocking writer).
big = b"z" * (8 << 20)
o, e, rc = sandboxed(BB, big, 10, 1 << 20, ["sh", "-c", PRE + "b wc -c"], argv0="busybox")
check("8 MiB stdin delivered", rc == 0 and o and o.strip() == b"8388608", "rc=%s out=%r" % (rc, (o or b"")[:20]))
# 14. A tool that never reads its 8 MiB stdin is still killed at the wall clock.
t0 = time.monotonic()
o, e, rc = sandboxed(BB, big, 2, 1 << 20, ["sh", "-c", PRE + "b sleep 30"], argv0="busybox")
dt = time.monotonic() - t0
check("unread stdin cannot outlast the wall clock", rc == -9 and dt < 4, "rc=%s after %.1fs" % (rc, dt))

failed = [n for n, ok in results if not ok]
print("%d/%d passed" % (len(results) - len(failed), len(results)))
sys.exit(1 if failed else 0)
