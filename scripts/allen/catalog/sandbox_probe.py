import os, socket, urllib.request
def t(name, f):
    try:
        r = f(); print(name, "OK", r)
    except Exception as e:
        print(name, "DENIED", type(e).__name__)
t("list-/home", lambda: os.listdir("/home"))
t("read-gh-hosts", lambda: len(open("/home/drakestapleton/.config/gh/hosts.yml").read()))
t("write-/opt/allen-catalog", lambda: open("/opt/allen-catalog/x", "w"))
t("write-/var/lib/allen-catalog", lambda: (open("/var/lib/allen-catalog/.probe", "w").close(), os.unlink("/var/lib/allen-catalog/.probe"))[1])
t("gb10-lock", lambda: os.stat("/tmp/aien-gb10.lock"))
for host, port in (("127.0.0.1", 18091), ("192.168.1.192", 22), ("100.64.0.3", 443)):
    t("connect-%s:%d" % (host, port), lambda: socket.create_connection((host, port), timeout=4).close())
t("github-api", lambda: urllib.request.urlopen("https://api.github.com/zen", timeout=10).status)
t("ids+caps", lambda: (os.getuid(), [l.strip() for l in open("/proc/self/status") if l.startswith(("CapBnd", "NoNewPrivs"))]))
