"""Run inside the pusher unit's sandbox (same properties via systemd-run) and report what it can reach."""
import ctypes
import mmap
import os
import socket


def t(label, f):
    try:
        print(label, "->", f())
    except Exception as e:
        print(label, "-> DENIED", type(e).__name__, getattr(e, "errno", ""))


def wtest(d):
    p = os.path.join(d, ".probe-%d" % os.getpid())
    open(p, "w").close()
    os.unlink(p)
    return "ok"


print("uid/gid/groups:", os.getuid(), os.getgid(), os.getgroups())
t("list /home", lambda: os.listdir("/home"))
t("read /home/drakestapleton/.config/gh/hosts.yml", lambda: len(open("/home/drakestapleton/.config/gh/hosts.yml").read()))
t("list /root", lambda: os.listdir("/root"))
t("list spool-in", lambda: os.listdir("/var/lib/allen-chat/spool-in"))
t("write spool-status", lambda: wtest("/var/lib/allen-chat/spool-status"))
t("write state dir", lambda: wtest("/var/lib/omega-pusher"))
t("list /var/lib/allen-contacts", lambda: os.listdir("/var/lib/allen-contacts"))
t("list contacts-drop", lambda: os.listdir("/var/lib/allen-chat/contacts-drop"))
t("write /opt/omega-projects-pusher", lambda: wtest("/opt/omega-projects-pusher"))
t("write /etc", lambda: wtest("/etc"))
t("write /var/lib", lambda: wtest("/var/lib"))
t("/tmp entries", lambda: len(os.listdir("/tmp")))
t("gb10 lock visible", lambda: os.path.exists("/tmp/aien-gb10.lock"))
t("read /etc/credstore.encrypted", lambda: os.listdir("/etc/credstore.encrypted"))
t("credential dir", lambda: (os.environ.get("CREDENTIALS_DIRECTORY"),
                             sorted(os.listdir(os.environ["CREDENTIALS_DIRECTORY"]))
                             if os.environ.get("CREDENTIALS_DIRECTORY") else None))
t("credential length", lambda: len(open(os.path.join(os.environ["CREDENTIALS_DIRECTORY"], "omega-projects-probe")).read()))
t("AF_NETLINK socket", lambda: socket.socket(socket.AF_NETLINK, socket.SOCK_RAW, 0))
t("AF_PACKET socket", lambda: socket.socket(socket.AF_PACKET, socket.SOCK_RAW, 0))
t("connect api.github.com:443", lambda: socket.create_connection(("api.github.com", 443), timeout=10).close() or "ok")
libc = ctypes.CDLL(None, use_errno=True)
t("personality(PER_LINUX32) rc,errno", lambda: (libc.personality(0x0008), ctypes.get_errno()))
t("mount rc,errno", lambda: (libc.mount(b"none", b"/tmp", b"tmpfs", 0, None), ctypes.get_errno()))
t("unshare(CLONE_NEWUSER) rc,errno", lambda: (libc.unshare(0x10000000), ctypes.get_errno()))
t("mmap W+X", lambda: mmap.mmap(-1, 4096, prot=7))
t("ps sees pids", lambda: len([p for p in os.listdir("/proc") if p.isdigit()]))
for k in ("CapBnd", "CapEff", "NoNewPrivs", "Seccomp"):
    print([ln.strip() for ln in open("/proc/self/status") if ln.startswith(k + ":")])
t("/dev/tpmrm0 visible", lambda: os.path.exists("/dev/tpmrm0"))
t("read a spool-in item (group allen-spool)", lambda: [len(open(os.path.join("/var/lib/allen-chat/spool-in", n)).read())
                                                       for n in os.listdir("/var/lib/allen-chat/spool-in")])
