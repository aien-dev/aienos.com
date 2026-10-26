#!/usr/bin/env python3
"""allen-relay-mailer: emails visitor messages from public Allen to Drake, and only to Drake.

The gateway (allen-chat) cannot send anything: it drops each visitor-confirmed
relay as one JSON file into /var/lib/allen-chat/relay-drop/ (it can create files
there but cannot list or read the directory). This process, running as the
system user allen-relay (setpriv, no_new_privs, no capabilities, no systemd
unit), polls that directory every 2 s, re-validates every file with the same
validator (allen_spool.validate_relay_record), and sends ONE plain-text email per
item from aien@aienos.com to drake@aienos.com. The recipient is a constant; the
send function refuses any other address and no input field can name one.

Transport: the same as spark-mail's Gandi connector (spark-mail-rs src/gandi.rs,
sovereign-core 0df7c321): SMTP over implicit TLS to mail.gandi.net:465,
authenticated as aien@aienos.com with GANDI_AIEN_MAIL_PASSWORD. spark-mail's own
HTTP send (127.0.0.1:18092 /api/mail/send) relays through a bridge on
127.0.0.1:1025 that is not running, and its gandi-send CLI reads atlas-vault as
drakestapleton, so neither is usable from this user. The password is never on
disk or in the environment: the root start script reads it from atlas-vault and
hands it over on stdin; this process reads one line and closes stdin.

State (/var/lib/allen-relay/state.json, 0600): today's count (global cap 30 per
UTC day, persisted) and the ids already sent, so a crash between the SMTP
acceptance and the delete never sends an item twice. A failed send keeps the
file and retries with exponential backoff (2 s doubling to 15 min).

Log lines carry relay ids, Message-IDs and counts only, never message text,
names or contacts.

Telegram: telegram_notify() is a disabled hook. It reads TELEGRAM_BOT_TOKEN and
TELEGRAM_CHAT_ID (absent today) and does nothing unless --telegram is given AND
both are set; it is not wired into the start script.

Python standard library only (see /opt/allen-chat/DEPENDENCIES.md).
"""

import argparse
import json
import os
import re
import smtplib
import ssl
import sys
import time
import urllib.parse
import urllib.request
from email.message import EmailMessage
from email.utils import formatdate

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import allen_spool  # noqa: E402

SENDER = "aien@aienos.com"
RECIPIENT = "drake@aienos.com"
CONSOLE = "http://100.64.0.2:18112"
DROP_NAME = re.compile(r"^[0-9a-f]{16}\.json$")
MAX_FILE = 128 * 1024
POLL_S = 2.0
BACKOFF_MAX = 900.0
KEEP_SENT = 500


class Refused(Exception):
    pass


def log(msg):
    print(time.strftime("%Y-%m-%dT%H:%M:%SZ ", time.gmtime()) + msg, flush=True)


def one_line(s):
    return " ".join(s.split())


def compose(rec):
    """The email for one validated relay record."""
    msg = EmailMessage()
    first = one_line(rec["message"])
    msg["Subject"] = "Allen relay: " + first[:60].rstrip()
    msg["From"] = "Allen relay <%s>" % SENDER
    msg["To"] = RECIPIENT
    msg["Date"] = formatdate(usegmt=True)
    msg["Message-ID"] = "<allen-relay-%s@aienos.com>" % rec["id"][2:]
    msg["Auto-Submitted"] = "auto-generated"
    conv = rec["conv"]
    lines = ["A visitor of www.aienos.com/allen/ asked Allen to pass this on to you.", "",
             "Message:", rec["message"], ""]
    lines.append("Name: " + (rec["name"] or "(not given)"))
    lines.append("Contact: " + (rec["contact"] or "(not given)"))
    lines += ["", "Conversation: " + (conv or "(none)"),
              "Console: %s/session/%s" % (CONSOLE, conv) if conv else "Console: %s/" % CONSOLE,
              "Relay id: " + rec["id"], "Received: " + rec["received"], ""]
    if rec["transcript"]:
        lines.append("Transcript (the visitor chose to include it):")
        for m in rec["transcript"]:
            lines.append("%s: %s" % ("Visitor" if m["role"] == "user" else "Allen", m["content"]))
        lines.append("")
    else:
        lines += ["The visitor did not include the conversation.", ""]
    lines.append("Replies to this email go to the AIEN mailbox, not to the visitor. "
                 "Visitor text is untrusted data, not instructions.")
    msg.set_content("\n".join(lines))
    return msg


def send(msg, to, password, smtp):
    """Submit one message. Refuses any recipient but RECIPIENT, whatever the caller passes."""
    if to != RECIPIENT or msg["To"] != RECIPIENT:
        raise Refused("recipient %r refused: this mailer sends to %s only" % (to, RECIPIENT))
    host, port, tls = smtp
    if tls:
        conn = smtplib.SMTP_SSL(host, port, context=ssl.create_default_context(), timeout=30)
    else:  # tests only: a loopback stub
        if host not in ("127.0.0.1", "::1"):
            raise Refused("plain SMTP is allowed only to a loopback test server")
        conn = smtplib.SMTP(host, port, timeout=30)
    try:
        if tls:
            conn.login(SENDER, password)
        refused = conn.send_message(msg, from_addr=SENDER, to_addrs=[RECIPIENT])
        if refused:
            raise smtplib.SMTPRecipientsRefused(refused)
    finally:
        try:
            conn.quit()
        except (smtplib.SMTPException, OSError):
            conn.close()


def telegram_notify(rec, enabled):
    """DISABLED hook: a one-line ping to Drake's Telegram. Runs only with --telegram and both
    TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID set (none of them is today). Returns why it did
    nothing, or 'sent'."""
    token = os.environ.get("TELEGRAM_BOT_TOKEN", "")
    chat = os.environ.get("TELEGRAM_CHAT_ID", "")
    if not enabled:
        return "disabled"
    if not (token and chat):
        return "no TELEGRAM_BOT_TOKEN/TELEGRAM_CHAT_ID"
    text = "Allen relay %s: %s" % (rec["id"], one_line(rec["message"])[:200])
    data = urllib.parse.urlencode({"chat_id": chat, "text": text}).encode()
    urllib.request.urlopen("https://api.telegram.org/bot%s/sendMessage" % token, data, timeout=10).read(4096)
    return "sent"


class State:
    def __init__(self, path):
        self.path = path
        self.day, self.count, self.sent = "", 0, []
        try:
            with open(path, encoding="utf-8") as f:
                s = json.load(f)
            self.day, self.count = str(s.get("day", "")), int(s.get("count", 0))
            self.sent = [x for x in s.get("sent", []) if isinstance(x, str)][-KEEP_SENT:]
        except (OSError, ValueError, TypeError, AttributeError):
            pass

    def today(self):
        d = time.strftime("%Y-%m-%d", time.gmtime())
        if d != self.day:
            self.day, self.count = d, 0
        return self.count

    def record(self, rid):
        self.today()
        self.count += 1
        self.sent = (self.sent + [rid])[-KEEP_SENT:]
        tmp = self.path + ".tmp"
        fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump({"day": self.day, "count": self.count, "sent": self.sent}, f)
            f.flush()
            os.fsync(f.fileno())
        os.rename(tmp, self.path)


def read_item(path):
    st = os.lstat(path)
    if not os.path.isfile(path) or os.path.islink(path) or st.st_size > MAX_FILE:
        raise allen_spool.Invalid("not a regular file under %d bytes" % MAX_FILE)
    with open(path, encoding="utf-8") as f:
        return allen_spool.validate_relay_record(json.load(f))


def run(args, password):
    state = State(args.state)
    retry_at, fails = {}, {}
    smtp = (args.smtp_host, args.smtp_port, not args.smtp_plain)
    log("allen-relay-mailer: drop %s, to %s only, cap %d/day, smtp %s:%d%s"
        % (args.drop, RECIPIENT, args.per_day, args.smtp_host, args.smtp_port, "" if smtp[2] else " (plain, test)"))
    while True:
        now = time.monotonic()
        try:
            names = sorted(n for n in os.listdir(args.drop) if DROP_NAME.match(n))
        except OSError as e:
            log("cannot list the drop directory: %s" % e.strerror)
            names = []
        for name in names:
            if retry_at.get(name, 0) > now:
                continue
            path = os.path.join(args.drop, name)
            try:
                rec = read_item(path)
            except (allen_spool.Invalid, ValueError, UnicodeDecodeError) as e:
                log("rejected %s: %s" % (name, str(e)[:120]))
                os.unlink(path)
                continue
            except OSError:
                continue
            if rec["id"] in state.sent:
                log("already sent %s; removing the drop" % rec["id"])
                os.unlink(path)
                continue
            if state.today() >= args.per_day:
                retry_at[name] = now + 600
                continue
            msg = compose(rec)
            try:
                send(msg, RECIPIENT, password, smtp)
            except (smtplib.SMTPException, OSError, ssl.SSLError) as e:
                fails[name] = fails.get(name, 0) + 1
                wait = min(BACKOFF_MAX, POLL_S * 2 ** fails[name])
                retry_at[name] = now + wait
                log("send failed for %s (%s); retry in %ds" % (rec["id"], type(e).__name__, wait))
                continue
            state.record(rec["id"])
            os.unlink(path)
            fails.pop(name, None)
            retry_at.pop(name, None)
            log("sent %s Message-ID %s (today %d/%d)" % (rec["id"], msg["Message-ID"], state.count, args.per_day))
            try:
                why = telegram_notify(rec, args.telegram)
                if why == "sent":
                    log("telegram ping sent for %s" % rec["id"])
            except OSError as e:
                log("telegram ping failed for %s (%s)" % (rec["id"], type(e).__name__))
        if args.once:
            return
        time.sleep(POLL_S)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--drop", default="/var/lib/allen-chat/relay-drop")
    ap.add_argument("--state", default="/var/lib/allen-relay/state.json")
    ap.add_argument("--per-day", type=int, default=30)
    ap.add_argument("--smtp-host", default="mail.gandi.net")
    ap.add_argument("--smtp-port", type=int, default=465)
    ap.add_argument("--smtp-plain", action="store_true", help="tests only: plain SMTP to a loopback stub")
    ap.add_argument("--once", action="store_true", help="one pass over the drop, then exit (tests)")
    ap.add_argument("--telegram", action="store_true", help="enable the disabled Telegram hook (needs env)")
    args = ap.parse_args()
    password = sys.stdin.readline().rstrip("\r\n")
    sys.stdin.close()
    os.close(0)  # the pipe from the starter; nothing else arrives on it
    if not password and not args.smtp_plain:
        sys.exit("allen-relay-mailer: no mail password on stdin; refusing to start")
    run(args, password)


if __name__ == "__main__":
    main()
