import os, socket, subprocess, sys
def t(name, f):
    try:
        r = f(); print("ALLOWED", name, r)
    except Exception as e:
        print("DENIED", name, type(e).__name__, str(e)[:80])
home = "/home/drakestapleton"
def conn(h, p):
    s = socket.create_connection((h, p), timeout=4); s.close(); return "connected"
t("read ~/.ssh", lambda: os.listdir(home + "/.ssh"))
t("read ~/.config/gh", lambda: os.listdir(home + "/.config/gh"))
t("read ~/.omp", lambda: os.listdir(home + "/.omp"))
t("read gb10 lock", lambda: open("/tmp/aien-gb10.lock").read(1))
t("write /opt/allen-chat", lambda: open("/opt/allen-chat/x", "w").write("x"))
t("read contact store /var/lib/allen-contacts", lambda: os.listdir("/var/lib/allen-contacts"))
t("list spool-in (write-only dir)", lambda: os.listdir("/var/lib/allen-chat/spool-in"))
t("create file in spool-in (needed)", lambda: (open("/var/lib/allen-chat/spool-in/.probe", "w").write("x"), os.unlink("/var/lib/allen-chat/spool-in/.probe")))
t("create file in contacts-drop (needed)", lambda: (open("/var/lib/allen-chat/contacts-drop/.probe", "w").write("x"), os.unlink("/var/lib/allen-chat/contacts-drop/.probe")))
t("write spool-status (pusher only)", lambda: open("/var/lib/allen-chat/spool-status/x.json", "w").write("x"))
t("run gh", lambda: subprocess.run(["gh", "auth", "status"], capture_output=True, timeout=10).returncode == 0 or (_ for _ in ()).throw(RuntimeError("gh failed")))
t("connect api.github.com:443 by name", lambda: conn("api.github.com", 443))
t("write /var/tmp", lambda: open("/var/tmp/allenchat-probe", "w").write("x"))
t("sudo -n true", lambda: subprocess.run(["sudo", "-n", "true"], capture_output=True, timeout=10).returncode == 0 or (_ for _ in ()).throw(RuntimeError("rc!=0")))
t("connect github.com:443", lambda: conn("140.82.112.3", 443))
t("connect 1.1.1.1:443", lambda: conn("1.1.1.1", 443))
t("connect hub LAN 192.168.1.192:22", lambda: conn("192.168.1.192", 22))
t("connect 127.0.0.1:18091 (control, needed)", lambda: conn("127.0.0.1", 18091))
t("connect 100.64.0.3:443 (hub tailnet, allowed)", lambda: conn("100.64.0.3", 443))
t("AF_NETLINK socket", lambda: socket.socket(socket.AF_NETLINK, socket.SOCK_RAW, 0))
def via_proc():
    import glob
    for d in glob.glob("/proc/[0-9]*"):
        try:
            return os.listdir(d + "/root/home/drakestapleton/.ssh")
        except Exception:
            continue
    raise PermissionError("no /proc/<pid>/root path reaches the real home")
t("read ~/.ssh via /proc/<pid>/root", via_proc)
t("count visible pids", lambda: len([d for d in os.listdir("/proc") if d.isdigit()]))
t("read /opt/allen-chat/allen_chat.py (needed)", lambda: len(open("/opt/allen-chat/allen_chat.py").read()))
