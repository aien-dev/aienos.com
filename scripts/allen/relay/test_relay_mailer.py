#!/usr/bin/env python3
"""Offline tests for allen_relay_mailer.py against a loopback SMTP stub (never Gandi).
Each guard has a control. Usage: test_relay_mailer.py"""

import email
import email.policy
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
# allen_spool.py sits one level up in the repo (scripts/allen/) and in ../gw in the work tree.
GW = next(d for d in (os.path.dirname(HERE), os.path.join(os.path.dirname(HERE), "gw"))
          if os.path.exists(os.path.join(d, "allen_spool.py")))
sys.path.insert(0, HERE)
sys.path.insert(0, GW)
import allen_relay_mailer as M  # noqa: E402

results = []


def check(name, ok, detail):
    results.append((name, ok))
    print("%s %s: %s" % ("PASS" if ok else "FAIL", name, detail), flush=True)


class SMTPStub:
    """Plain SMTP on loopback; records envelopes. reject=True answers DATA with 451."""

    def __init__(self):
        self.s = socket.socket()
        self.s.bind(("127.0.0.1", 0))
        self.s.listen(8)
        self.port = self.s.getsockname()[1]
        self.mail, self.reject = [], False
        threading.Thread(target=self.serve, daemon=True).start()

    def serve(self):
        while True:
            try:
                c, _ = self.s.accept()
            except OSError:
                return
            threading.Thread(target=self.one, args=(c,), daemon=True).start()

    def one(self, c):
        f = c.makefile("rb")
        c.sendall(b"220 stub\r\n")
        rcpt, frm, data = [], None, None
        while True:
            line = f.readline()
            if not line:
                return
            cmd = line.decode().strip()
            up = cmd.upper()
            if data is not None:
                if cmd == ".":
                    if self.reject:
                        c.sendall(b"451 try later\r\n")
                    else:
                        self.mail.append({"from": frm, "to": rcpt, "data": b"".join(data).decode()})
                        c.sendall(b"250 queued\r\n")
                    data = None
                else:
                    data.append(line)
                continue
            if up.startswith("EHLO") or up.startswith("HELO"):
                c.sendall(b"250-stub\r\n250 8BITMIME\r\n")
            elif up.startswith("MAIL FROM:"):
                frm, rcpt = cmd[10:].strip(), []
                c.sendall(b"250 ok\r\n")
            elif up.startswith("RCPT TO:"):
                rcpt.append(cmd[8:].strip().split()[0])
                c.sendall(b"250 ok\r\n")
            elif up == "DATA":
                data = []
                c.sendall(b"354 go\r\n")
            elif up == "QUIT":
                c.sendall(b"221 bye\r\n")
                c.close()
                return
            else:
                c.sendall(b"250 ok\r\n")


def record(rid, **kw):
    rec = {"v": 1, "type": "relay", "id": "r_" + rid, "received": "2026-10-26T12:00:00Z",
           "conv": "0123456789abcdef", "message": "Hello Drake.\nI am building a kelp boat in Omega and want to talk.",
           "name": "Test Visitor", "contact": "test@example.org",
           "transcript": [{"role": "user", "content": "hi"}, {"role": "assistant", "content": "hello"}]}
    rec.update(kw)
    return rec


def put(d, rid, rec):
    with open(os.path.join(d, rid + ".json"), "w") as f:
        json.dump(rec, f)


def run_once(d, state, port, *extra):
    p = subprocess.run([sys.executable, os.path.join(HERE, "allen_relay_mailer.py"), "--drop", d, "--state", state,
                        "--smtp-host", "127.0.0.1", "--smtp-port", str(port), "--smtp-plain", "--once"] + list(extra),
                       stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30,
                       env=dict(os.environ, PYTHONPATH=GW))
    return p.returncode, p.stdout + p.stderr


def main():
    smtp = SMTPStub()
    d = tempfile.mkdtemp(prefix="relay-test-drop")
    sd = tempfile.mkdtemp(prefix="relay-test-state")
    state = os.path.join(sd, "state.json")
    try:
        a, b = "a" * 16, "b" * 16
        put(d, a, record(a))
        put(d, b, record(b, to="someone@example.org"))
        put(d, "c" * 16, record("c" * 16, message="token ghp_" + "A1b2C3d4" * 5))
        rc, out = run_once(d, state, smtp.port)
        m = smtp.mail[0] if smtp.mail else {"data": "", "to": []}
        body = m["data"]
        check("one valid item sends exactly one mail to drake@aienos.com", rc == 0 and len(smtp.mail) == 1
              and m["to"] == ["<drake@aienos.com>"] and m["from"] == "<aien@aienos.com>",
              "rc %d, %d mails, rcpt %s" % (rc, len(smtp.mail), m["to"]))
        subj = str(email.message_from_string(body, policy=email.policy.default)["Subject"]).strip()
        check("subject is 'Allen relay: <first 60 chars>'",
              subj == "Allen relay: Hello Drake. I am building a kelp boat in Omega and want to", subj)
        check("body carries message, name, contact, transcript, conversation and console link",
              all(x in body for x in ("I am building a kelp boat", "Name: Test Visitor", "Contact: test@example.org",
                                      "Visitor: hi", "Allen: hello", "Conversation: 0123456789abcdef",
                                      "http://100.64.0.2:18112/session/0123456789abcdef", "Relay id: r_" + a)),
              "%d bytes" % len(body))
        check("items with a recipient field or a secret are rejected, not sent", not os.listdir(d)
              and "rejected %s.json" % b in out and "rejected %s.json" % ("c" * 16) in out, out.strip()[-200:])
        st = json.load(open(state))
        check("sent id and daily count persisted", st["count"] == 1 and st["sent"] == ["r_" + a], "%s" % st)

        # Already-sent id (crash between SMTP accept and delete): removed, not mailed again.
        put(d, a, record(a))
        rc, out = run_once(d, state, smtp.port)
        check("an already-sent id is never mailed twice", len(smtp.mail) == 1 and not os.listdir(d),
              "%d mails" % len(smtp.mail))

        # Failure keeps the file; the next pass (control) sends it.
        e = "e" * 16
        put(d, e, record(e))
        smtp.reject = True
        rc, out = run_once(d, state, smtp.port)
        kept = os.listdir(d)
        smtp.reject = False
        rc2, out2 = run_once(d, state, smtp.port)
        check("failed send keeps the drop and logs a backoff", kept == [e + ".json"] and "retry in" in out,
              out.strip()[-120:])
        check("retry after a failure sends it (control)", len(smtp.mail) == 2 and not os.listdir(d), out2.strip()[-120:])

        # Daily cap persisted across runs.
        f1, f2 = "f" * 16, "1" * 16
        put(d, f1, record(f1))
        put(d, f2, record(f2))
        rc, out = run_once(d, state, smtp.port, "--per-day", "3")
        check("global daily cap holds the rest", len(smtp.mail) == 3 and len(os.listdir(d)) == 1, "%d mails, %d kept"
              % (len(smtp.mail), len(os.listdir(d))))
        rc, out = run_once(d, state, smtp.port, "--per-day", "3")
        check("cap is persisted across restarts", len(smtp.mail) == 3 and len(os.listdir(d)) == 1,
              "%d mails after restart" % len(smtp.mail))

        # Recipient is hardcoded: send() refuses anything else; control sends to Drake.
        msg = M.compose(M.allen_spool.validate_relay_record(record("2" * 16)))
        try:
            M.send(msg, "someone@example.org", "", ("127.0.0.1", smtp.port, False))
            refused = False
        except M.Refused:
            refused = True
        n = len(smtp.mail)
        M.send(msg, M.RECIPIENT, "", ("127.0.0.1", smtp.port, False))
        check("send() refuses any recipient but drake@aienos.com", refused, "refused=%s" % refused)
        check("send() to drake@aienos.com works (control)", len(smtp.mail) == n + 1, "%d mails" % len(smtp.mail))
        try:
            M.send(msg, M.RECIPIENT, "", ("192.0.2.1", 25, False))
            plain = False
        except M.Refused:
            plain = True
        check("plain SMTP refused off loopback", plain, "refused=%s" % plain)
        os.environ.pop("TELEGRAM_BOT_TOKEN", None)
        os.environ.pop("TELEGRAM_CHAT_ID", None)
        check("telegram hook disabled by default and without env", M.telegram_notify({}, False) == "disabled"
              and M.telegram_notify({}, True).startswith("no TELEGRAM"), "ok")
        p = subprocess.run([sys.executable, os.path.join(HERE, "allen_relay_mailer.py"), "--drop", d, "--state", state,
                            "--once"], stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30,
                       env=dict(os.environ, PYTHONPATH=GW))
        check("refuses to start without a password on stdin", p.returncode != 0 and "no mail password" in p.stderr,
              p.stderr.strip())
    finally:
        shutil.rmtree(d, ignore_errors=True)
        shutil.rmtree(sd, ignore_errors=True)
    failed = [n for n, ok in results if not ok]
    print("%d/%d passed" % (len(results) - len(failed), len(results)))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
