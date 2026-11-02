#!/usr/bin/env python3
"""Production checks for "Reaching Drake" (relay, console, live takeover), each with a control.

Run on the Spark as drakestapleton (reads ALLEN_CONSOLE_TOKEN from atlas-vault; never prints it).
Sends ONE real relay email to drake@aienos.com marked TEST and checks it arrived via
`spark-mail gandi-list --account drake` (IMAP, mail.gandi.net). Turns Drake's availability on
for the takeover test and restores the previous availability afterwards.

Usage: test_reaching_drake_prod.py [--skip-email]
"""

import argparse
import http.client
import json
import re
import secrets
import ssl
import subprocess
import sys
import threading
import time
import urllib.parse

SITE = "www.aienos.com"
CONSOLE = ("100.64.0.2", 18112)
VAULT = "/home/drakestapleton/.local/bin/atlas-vault"
SPARK_MAIL = "/home/drakestapleton/.local/lib/aien/releases/review-0df7c321c150/spark-mail"
results = []


def check(name, ok, detail):
    results.append((name, ok))
    print("%s %s: %s" % ("PASS" if ok else "FAIL", name, detail), flush=True)


def public(method, path, body=None, headers=None, timeout=60):
    c = http.client.HTTPSConnection(SITE, 443, timeout=timeout, context=ssl.create_default_context())
    h = dict(headers or {})
    if body is not None:
        h["Content-Type"] = "application/json"
        body = json.dumps(body)
    c.request(method, path, body, h)
    return c, c.getresponse()


def sse(resp):
    ev = None
    for line in resp:
        line = line.rstrip(b"\r\n")
        if line.startswith(b"event: "):
            ev = line[7:].decode()
        elif line.startswith(b"data: "):
            yield ev, json.loads(line[6:])
            ev = None


class Console:
    def __init__(self):
        self.cookie = self.csrf = None

    def req(self, method, path, form=None):
        c = http.client.HTTPConnection(*CONSOLE, timeout=10)
        h = {"Cookie": self.cookie} if self.cookie else {}
        body = None
        if form is not None:
            body = urllib.parse.urlencode(form)
            h["Content-Type"] = "application/x-www-form-urlencoded"
        c.request(method, path, body, h)
        r = c.getresponse()
        return r.status, r.getheader("Set-Cookie") or "", r.read().decode("utf-8", "replace")

    def login(self, token):
        st, setc, _ = self.req("POST", "/login", {"token": token})
        if st == 303:
            self.cookie = setc.split(";")[0]
            _, _, page = self.req("GET", "/")
            m = re.search(r"name=csrf value='([^']+)'", page)
            self.csrf = m.group(1) if m else None
        return st

    def post(self, path, **form):
        return self.req("POST", path, dict(form, csrf=self.csrf))[0]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--skip-email", action="store_true")
    a = ap.parse_args()
    token = subprocess.run([VAULT, "get", "ALLEN_CONSOLE_TOKEN"], capture_output=True, text=True).stdout.strip()

    # Console reachability.
    hub = subprocess.run(["ssh", "-o", "BatchMode=yes", "hub", "curl -s -o /dev/null -w '%{http_code}' -m5 "
                          "http://100.64.0.2:18112/; echo \" rc=$?\""], capture_output=True, text=True, timeout=30)
    hubctl = subprocess.run(["ssh", "-o", "BatchMode=yes", "hub", "curl -s -o /dev/null -w '%{http_code}' -m5 "
                             "-H 'X-Real-IP: 192.0.2.1' http://100.64.0.2:18110/allen/api/presence"],
                            capture_output=True, text=True, timeout=30)
    check("console unreachable from the hub", "rc=0" not in hub.stdout, hub.stdout.strip())
    check("hub still reaches the gateway port (control)", hubctl.stdout.strip() == "200", hubctl.stdout.strip())
    for path in ("/allen/api/console", "/allen/api/console/login", "/allen/console"):
        c, r = public("GET", path)
        r.read()
        check("no public route to the console: %s" % path, r.status == 404, "%d" % r.status)
    cons = Console()
    st, _, _ = cons.req("GET", "/")
    check("console from the Spark without token: 401", st == 401, "%d" % st)
    st = cons.login(token)
    st2, _, page = cons.req("GET", "/")
    check("console from the Spark with the token (control)", st == 303 and st2 == 200 and "Availability" in page,
          "login %d, dashboard %d" % (st, st2))
    c, r = public("GET", "/allen/api/presence")
    pres = json.loads(r.read())
    check("public presence route", r.status == 200 and set(pres) == {"online", "hours", "handoff", "transcribe"}
          and isinstance(pres["transcribe"], bool), "%s" % pres)
    prev_online, prev_hours = pres["online"], pres["hours"]

    # Real relay email.
    if not a.skip_email:
        nonce = secrets.token_hex(4)
        msg = ("TEST from AllenRelay %s: production relay check, please ignore. This verifies that a visitor "
               "message relayed by public Allen reaches drake@aienos.com." % nonce)
        conv = secrets.token_hex(16)
        c, r = public("POST", "/allen/api/relay", {"message": msg, "name": "AllenRelay test", "contact": "",
                                                   "transcript": [{"role": "user", "content": "test"},
                                                                  {"role": "assistant", "content": "test reply"}]},
                      {"X-Allen-Conversation": conv, "X-Allen-Anonymous": "1"})
        j = json.loads(r.read())
        check("production relay accepted", r.status == 200 and j.get("sent"), "%d %s" % (r.status, j))
        rid = j.get("id", "")
        sent_line = ""
        for _ in range(30):
            log = subprocess.run(["sudo", "-n", "grep", "sent " + rid, "/var/log/allen-relay.log"],
                                 capture_output=True, text=True).stdout.strip()
            if log:
                sent_line = log
                break
            time.sleep(2)
        check("mailer sent it (Gandi SMTP accepted)", bool(sent_line), sent_line or "no 'sent' line in 60 s")
        found = None
        for _ in range(12):
            out = subprocess.run([SPARK_MAIL, "gandi-list", "--account", "drake", "--limit", "15"],
                                 capture_output=True, text=True, timeout=60)
            try:
                items = json.loads(out.stdout.splitlines()[0]) if out.stdout else []
            except (ValueError, IndexError):
                items = []
            found = next((m for m in items if nonce in json.dumps(m)), None)
            if found:
                break
            time.sleep(10)
        check("relay email arrived in drake@aienos.com (IMAP)", found is not None,
              json.dumps({k: found.get(k) for k in found if k in ("uid", "subject", "from", "date", "message_id")})
              if found else "not found in the last 15 messages after 2 min")

    # Live takeover end to end.
    conv = secrets.token_hex(16)
    H = {"X-Allen-Conversation": conv, "X-Allen-Anonymous": "1"}
    try:
        st = cons.post("/presence", online="on", hours=prev_hours or "")
        c, r = public("GET", "/allen/api/presence")
        p = json.loads(r.read())
        check("console turns availability on", st == 303 and p["online"] and p["handoff"], "%s" % p)
        c, r = public("POST", "/allen/api/handoff", {"transcript": [{"role": "user", "content": "TEST from "
                                                                     "AllenRelay: takeover check"}]}, H)
        j = json.loads(r.read())
        check("handoff accepted while online", r.status == 200 and j.get("waiting"), "%d %s" % (r.status, j))
        evs = []
        sc, sr = public("GET", "/allen/api/session", headers=H, timeout=120)

        def listen():
            try:
                for e in sse(sr):
                    evs.append(e)
            except (OSError, ValueError, AttributeError):  # AttributeError: closed under the reader
                pass
        threading.Thread(target=listen, daemon=True).start()
        time.sleep(2)
        c, r = public("POST", "/allen/api/say", {"text": "Drake: hi (visitor text, must not become human)"}, H)
        r.read()
        st = cons.post("/session/%s/join" % conv)
        time.sleep(2)
        c, r = public("POST", "/allen/api/chat", {"messages": [{"role": "user", "content": "Are you there?"}]}, H)
        chat_ev = list(sse(r)) if r.status == 200 else [("http", r.status)]
        check("while joined, chat answers error human", chat_ev == [("error", chat_ev[0][1])]
              and chat_ev[0][1].get("code") == "human", "%s" % chat_ev)
        st2 = cons.post("/session/%s/reply" % conv, text="TEST: this is Drake's console reply.")
        time.sleep(3)
        humans = [d for e, d in evs if e == "human"]
        check("console reply arrives on the visitor's session as event human", st == 303 and st2 == 303
              and len(humans) == 1 and humans[0]["from"] == "Drake"
              and humans[0]["text"] == "TEST: this is Drake's console reply.", "%s" % evs)
        st = cons.post("/session/%s/handback" % conv)
        time.sleep(2)
        states = [d["drake"] for e, d in evs if e == "state"]
        check("state events waiting, joined, left", st == 303 and states == ["waiting", "joined", "left"],
              "%s" % states)
        t0 = time.monotonic()
        c, r = public("POST", "/allen/api/chat", {"messages": [{"role": "user", "content":
                                                                "Reply with the single word ok."}]}, H, timeout=200)
        chat_ev = list(sse(r)) if r.status == 200 else [("http", r.status)]
        text = "".join(d for e, d in chat_ev if e is None)
        check("after hand back chat works again", chat_ev and chat_ev[-1][0] == "done" and text.strip() != "",
              "%r in %.1fs" % (text[:80], time.monotonic() - t0))
        sc.close()
    finally:
        cons.post("/presence", **({"online": "on"} if prev_online else {}), hours=prev_hours)
        c, r = public("GET", "/allen/api/presence")
        p = json.loads(r.read())
        check("availability restored", p["online"] == prev_online and p["hours"] == prev_hours, "%s" % p)
    failed = [n for n, ok in results if not ok]
    print("%d/%d passed" % (len(results) - len(failed), len(results)))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
