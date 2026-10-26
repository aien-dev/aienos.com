#!/usr/bin/python3.12 -I
"""Launcher for allen-science: run as the allen-science user (never root) by the supervisor.

It binds the one listening socket, 127.0.0.1:18113, in the host network namespace, then execs
bubblewrap so the service runs in an empty network namespace holding only that inherited socket:
the gateway can connect in over loopback, the service cannot open any connection out (not to
loopback, the tailnet or the internet). Inside: a read-only root built from the Python interpreter,
its standard library, the shared libraries they, bwrap and prlimit load (resolved with ldd at
launch), bwrap and prlimit themselves (for the per-request sandbox) and /opt/allen-science; private
tmpfs /tmp and HOME; a cleared environment; no capabilities; its own pid, ipc, uts and cgroup
namespaces. Nothing from /etc, /home, /root, /var or /run is mounted. Every inherited descriptor
other than stdin, stdout and stderr is closed first, so the listener on fd 3 is the only one the
service receives. The launcher writes nothing; output goes to the supervisor.
"""

import argparse
import glob
import os
import socket
import subprocess
import sys

PYTHON = "/usr/bin/python3.12"
BWRAP = "/usr/bin/bwrap"
PRLIMIT = "/usr/bin/prlimit"
LDD = "/usr/bin/ldd"
STDLIB = "/usr/lib/python3.12"
ROOT = "/opt/allen-science"
HOME = "/home/allen-science"
LISTEN_FD = 3


def libraries(binaries):
    """{path inside the sandbox: real host file} for every shared object the binaries load."""
    found = {}
    for b in binaries:
        out = subprocess.run([LDD, b], capture_output=True, text=True, env={"PATH": "/usr/bin:/bin"},
                             timeout=20).stdout
        for line in out.splitlines():
            parts = line.split()
            path = parts[2] if "=>" in parts and len(parts) > 2 else parts[0] if parts else ""
            if not path.startswith("/"):
                continue
            # /lib is a symlink to usr/lib on this host; the sandbox recreates that symlink.
            dest = "/usr" + path if path.startswith("/lib/") else path
            found[dest] = os.path.realpath(path)
    return found


def sandbox_argv(root, service_args):
    dynload = sorted(glob.glob(STDLIB + "/lib-dynload/*.so"))
    argv = [
        BWRAP, "--unshare-all", "--die-with-parent", "--new-session", "--clearenv",
        "--cap-drop", "ALL",
        "--ro-bind", PYTHON, PYTHON, "--ro-bind", BWRAP, BWRAP, "--ro-bind", PRLIMIT, PRLIMIT,
        "--ro-bind", STDLIB, STDLIB,
    ]
    for dest, src in sorted(libraries([PYTHON, BWRAP, PRLIMIT] + dynload).items()):
        argv += ["--ro-bind", src, dest]
    argv += [
        "--symlink", "usr/lib", "/lib", "--symlink", "usr/bin", "/bin",
        "--ro-bind", root, ROOT,
        # The per-request bwrap writes its uid map under /proc: a fresh proc for this pid namespace.
        "--proc", "/proc", "--dev", "/dev",
        "--size", str(32 << 20), "--tmpfs", "/tmp",
        "--size", str(4 << 20), "--tmpfs", "/home", "--dir", HOME,
        "--remount-ro", "/", "--chdir", HOME,
        "--setenv", "HOME", HOME, "--setenv", "PATH", "/usr/bin:/bin", "--setenv", "LANG", "C.UTF-8",
        "--", PYTHON, "-I", ROOT + "/allen_science.py", "--listen-fd", str(LISTEN_FD),
    ] + service_args
    return argv


def close_fds_except(keep):
    """Close every open descriptor above 2 that is not in keep. Enumerates /proc/self/fd, so a
    descriptor numbered above a lowered RLIMIT_NOFILE is closed too; refuses to start without it."""
    try:
        fds = [int(n) for n in os.listdir("/proc/self/fd")]
    except OSError as e:
        sys.exit("allen-science launch: cannot list /proc/self/fd (%s); refusing to start" % e)
    for fd in fds:
        if fd > 2 and fd not in keep:
            try:
                os.close(fd)
            except OSError:
                pass  # the directory descriptor listdir used is already closed


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--listen", default="127.0.0.1:18113", help="loopback HOST:PORT (127.0.0.1 only)")
    ap.add_argument("--root", default=ROOT, help="directory bound read-only at " + ROOT)
    ap.add_argument("service_args", nargs="*", help="passed to allen_science.py after --")
    a = ap.parse_args()
    if os.geteuid() == 0 or os.getuid() == 0:
        sys.exit("allen-science launch: refusing to run as root; the supervisor drops to allen-science")
    host, _, port = a.listen.rpartition(":")
    if host != "127.0.0.1" or not port.isdigit():
        sys.exit("allen-science launch: --listen must be 127.0.0.1:PORT")
    if not os.path.isfile(os.path.join(a.root, "allen_science.py")):
        sys.exit("allen-science launch: %s/allen_science.py missing" % a.root)
    # Only stdin, stdout, stderr and the listener may cross into the service.
    close_fds_except(())
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind((host, int(port)))
    s.listen(64)
    if s.fileno() != LISTEN_FD:
        os.dup2(s.fileno(), LISTEN_FD)
        s.close()
    os.set_inheritable(LISTEN_FD, True)
    argv = sandbox_argv(os.path.realpath(a.root), a.service_args)
    close_fds_except((LISTEN_FD,))
    os.execv(BWRAP, argv)


if __name__ == "__main__":
    main()
