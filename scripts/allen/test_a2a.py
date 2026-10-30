#!/usr/bin/env python3
"""A2A SendMessage against the introduce door. No sandbox, no model, no network listen.

Drives Handler.do_POST on a throwaway drop directory. A consented send stores one
introduce record. Missing or false consent, malformed JSON-RPC, and an unsupported
method store nothing. The JSON-RPC envelope is not written even when the
introduction is.
"""

import io
import json
import os
import re
import sys
import tempfile
import threading
from email.message import Message

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "contacts"))

import a2a  # noqa: E402
import allen_chat  # noqa: E402
import allen_contacts_ingest  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
SENTINEL = "SENTINEL-a2a-message-not-stored-9f3c2a"
results = []


def check(name, ok, detail):
    results.append((name, ok))
    print("%s %s: %s" % ("PASS" if ok else "FAIL", name, detail), flush=True)


def intro(**over):
    body = {
        "consent": True, "kind": "agent", "name": "Test Agent", "org": "",
        "contact": "test@example.org", "github": "", "origin": "",
        "purpose": "testing the door", "homepage": "", "framework": "",
        "model": "", "human": "Drake (test)",
    }
    body.update(over)
    return body


class App:
    """The pieces accept_introduction actually uses. Not a full gateway."""

    def __init__(self, drop):
        self.args = type("A", (), {"contacts_drop": drop, "submit_per_day": 40})()
        self.stats = allen_chat.Stats()
        self.intro_rate = allen_chat.Buckets(3, 1200)
        self.salt = b"a2a-test-salt!!"
        self.trusted = {"127.0.0.1"}
        self.visitor_secret = None
        self.day = (None, 0)
        self.day_lock = threading.Lock()

    def drop(self, rec, d=None, name=None):
        allen_chat.App.drop(self, rec, d, name)

    def submit_day(self):
        return allen_chat.App.submit_day(self)

    def client_key(self, handler):
        return allen_chat.App.client_key(self, handler)


def files(drop):
    return sorted(f for f in os.listdir(drop) if not f.startswith("."))


def tree_text(drop):
    out = []
    for name in files(drop):
        with open(os.path.join(drop, name), "rb") as f:
            out.append(f.read())
    return b"\n".join(out)


class Call:
    def __init__(self, app, raw, path, extra=None):
        self.app = app
        self.headers = Message()
        self.headers["Content-Type"] = "application/json"
        self.headers["Content-Length"] = str(len(raw))
        self.headers["X-Real-IP"] = "203.0.113.50"
        for k, v in (extra or {}).items():
            self.headers[k] = v
        self.rfile = io.BytesIO(raw)
        self.wfile = io.BytesIO()
        self.client_address = ("127.0.0.1", 9)
        self.close_connection = False
        self.command = "POST"
        self.path = path
        self.requestline = "POST %s HTTP/1.1" % path
        self.request_version = "HTTP/1.1"
        self.body_read = False


def post(app, path, obj=None, raw=None, headers=None, version="1.0", content_type="application/json"):
    if raw is None:
        raw = json.dumps(obj).encode()
    call = allen_chat.Handler.__new__(allen_chat.Handler)
    src = Call(app, raw, path, headers)
    if "Content-Type" in src.headers:
        src.headers.replace_header("Content-Type", content_type)
    else:
        src.headers["Content-Type"] = content_type
    route = path.split("?", 1)[0]
    if version is not False and route == "/allen/api/a2a" and "A2A-Version" not in src.headers:
        src.headers["A2A-Version"] = version
    call.__dict__.update(src.__dict__)
    call.server = type("S", (), {"app": app})()
    call.do_POST()
    data = call.wfile.getvalue()
    head, _, body = data.partition(b"\r\n\r\n")
    line = head.split(b"\r\n", 1)[0]
    status = int(line.split()[1])
    hdrs = {}
    for row in head.split(b"\r\n")[1:]:
        if b":" in row:
            k, v = row.split(b":", 1)
            hdrs[k.decode().lower()] = v.decode().strip()
    payload = json.loads(body.decode()) if body else None
    return status, hdrs, payload


def rpc(method, message, rid="t1", params_extra=None):
    params = {"message": message}
    if params_extra:
        params.update(params_extra)
    return {"jsonrpc": "2.0", "id": rid, "method": method, "params": params}


def message(fields, text=None, message_id="msg-1", context_id=None, part_meta=None):
    parts = []
    if text is not None:
        part = {"text": text, "mediaType": "text/plain"}
        if part_meta is not None:
            part["metadata"] = part_meta
        parts.append(part)
    parts.append({"data": fields, "mediaType": "application/json"})
    msg = {"role": "ROLE_USER", "messageId": message_id, "parts": parts}
    if context_id is not None:
        msg["contextId"] = context_id
    return msg


def no_store(hdrs):
    return hdrs.get("cache-control") == "no-store"


def test_valid_send():
    drop = tempfile.mkdtemp(prefix="a2a-valid-")
    app = App(drop)
    try:
        st, hdrs, body = post(app, "/allen/api/introduce", intro())
        drops = files(drop)
        rec = json.load(open(os.path.join(drop, drops[0]))) if drops else {}
        check("introduce door still stores one consented record",
              st == 200 and body.get("stored") is True and len(drops) == 1
              and rec.get("name") == "Test Agent" and "consent" not in rec
              and no_store(hdrs),
              "status %s drops %d" % (st, len(drops)))
        os.remove(os.path.join(drop, drops[0]))
        st, hdrs, body = post(app, "/allen/api/a2a", rpc("SendMessage", message(intro())))
        drops = files(drop)
        raw = open(os.path.join(drop, drops[0]), "rb").read() if drops else b""
        rec = json.loads(raw) if raw else {}
        parsed = allen_contacts_ingest.parse_drop(raw) if raw else None
        result = (body or {}).get("result", {}).get("message", {})
        data = next((p.get("data") for p in result.get("parts", []) if "data" in p), {})
        check("SendMessage with consent stores one introduce record",
              st == 200 and no_store(hdrs) and body.get("jsonrpc") == "2.0"
              and result.get("role") == "ROLE_AGENT"
              and data.get("stored") is True and data.get("id") == rec.get("id")
              and len(drops) == 1 and parsed is not None
              and rec.get("purpose") == "testing the door" and "consent" not in rec
              and "jsonrpc" not in raw.decode() and "SendMessage" not in raw.decode(),
              "status %s drops %d role %s" % (st, len(drops), result.get("role")))
        st, hdrs, body = post(app, "/allen/api/a2a",
                              rpc("message/send", message(intro(name="Legacy Agent"), message_id="msg-2"), rid=2))
        drops = files(drop)
        check("legacy message/send is the same call",
              st == 200 and no_store(hdrs) and len(drops) == 2
              and body.get("result", {}).get("message", {}).get("role") == "ROLE_AGENT",
              "status %s drops %d" % (st, len(drops)))
    finally:
        for name in files(drop):
            os.remove(os.path.join(drop, name))
        os.rmdir(drop)


def test_consent():
    drop = tempfile.mkdtemp(prefix="a2a-consent-")
    app = App(drop)
    try:
        missing = intro()
        missing.pop("consent")
        cases = (
            ("missing consent", missing),
            ("consent false", intro(consent=False)),
            ("consent as string", intro(consent="true")),
            ("consent number 1", intro(consent=1)),
            ("consent yes", intro(consent="yes")),
        )
        for name, fields in cases:
            st, hdrs, body = post(app, "/allen/api/a2a", rpc("SendMessage", message(fields)))
            err = (body or {}).get("error", {})
            text = json.dumps(err)
            check("refuses %s and stores nothing" % name,
                  st == 200 and no_store(hdrs) and err.get("code") == -32602
                  and "Nothing was stored" in text and not files(drop),
                  "status %s code %s drops %d" % (st, err.get("code"), len(files(drop))))
        st, hdrs, body = post(app, "/allen/api/a2a", rpc("SendMessage", message(intro())),
                              headers={"X-Allen-Anonymous": "1"})
        err = (body or {}).get("error", {})
        check("anonymous mode stores nothing",
              st == 200 and no_store(hdrs) and err.get("code") == -32602
              and "nothing was stored" in json.dumps(err).lower() and not files(drop),
              "status %s code %s" % (st, err.get("code")))
    finally:
        os.rmdir(drop)


def test_malformed():
    drop = tempfile.mkdtemp(prefix="a2a-bad-")
    app = App(drop)
    try:
        cases = (
            ("not json", b"{", -32700),
            ("empty body", b"", -32700),
            ("json array", b"[1,2]", -32600),
            ("missing id", b'{"jsonrpc":"2.0","method":"SendMessage"}', -32600),
            ("wrong version", b'{"jsonrpc":"1.0","id":1,"method":"SendMessage"}', -32600),
        )
        for name, raw, code in cases:
            st, hdrs, body = post(app, "/allen/api/a2a", raw=raw)
            err = (body or {}).get("error", {})
            check("malformed %s" % name,
                  st == 200 and no_store(hdrs) and err.get("code") == code and not files(drop),
                  "status %s code %s body %s" % (st, err.get("code"), body))
    finally:
        os.rmdir(drop)


def test_unsupported():
    drop = tempfile.mkdtemp(prefix="a2a-method-")
    app = App(drop)
    try:
        for method, code in (("GetTask", -32004), ("SendStreamingMessage", -32004),
                             ("CreateTaskPushNotificationConfig", -32003),
                             ("GetExtendedAgentCard", -32004), ("nope", -32601)):
            st, hdrs, body = post(app, "/allen/api/a2a",
                                  {"jsonrpc": "2.0", "id": 7, "method": method, "params": {}})
            err = (body or {}).get("error", {})
            check("unsupported %s" % method,
                  st == 200 and no_store(hdrs) and err.get("code") == code
                  and body.get("id") == 7 and not files(drop),
                  "status %s code %s" % (st, err.get("code")))
    finally:
        os.rmdir(drop)


def test_nothing_stored():
    """The envelope, message id, and unused text are not on disk. Failures write no file."""
    drop = tempfile.mkdtemp(prefix="a2a-store-")
    app = App(drop)
    before = set(os.listdir(drop))
    try:
        post(app, "/allen/api/a2a", raw=b"not-json")
        post(app, "/allen/api/a2a", {"jsonrpc": "2.0", "id": 1, "method": "GetTask", "params": {}})
        bare = intro()
        bare.pop("consent")
        post(app, "/allen/api/a2a", rpc("SendMessage", message(bare)))
        post(app, "/allen/api/a2a", rpc("SendMessage", message(intro(consent=False))))
        check("refusals create no file", files(drop) == [] and set(os.listdir(drop)) == before,
              "entries %s" % os.listdir(drop))
        fields = intro(purpose="testing the door")
        msg = message(fields, text=SENTINEL, message_id=SENTINEL, context_id=SENTINEL,
                      part_meta={"note": SENTINEL})
        st, hdrs, body = post(app, "/allen/api/a2a", rpc("SendMessage", msg, rid="keep"))
        blob = tree_text(drop)
        check("consented send does not store the message envelope",
              st == 200 and no_store(hdrs) and len(files(drop)) == 1
              and SENTINEL.encode() not in blob
              and b"jsonrpc" not in blob and b"SendMessage" not in blob
              and b"testing the door" in blob and b'"consent"' not in blob,
              "status %s files %s" % (st, files(drop)))
        # The reply may echo the context id. That echo is the HTTP body, not a file.
        echoed = (body or {}).get("result", {}).get("message", {}).get("contextId")
        check("context id is returned and not stored",
              echoed == SENTINEL and SENTINEL.encode() not in blob, "context %s" % echoed)
    finally:
        for name in files(drop):
            os.remove(os.path.join(drop, name))
        os.rmdir(drop)


def test_version():
    drop = tempfile.mkdtemp(prefix="a2a-version-")
    app = App(drop)
    try:
        for name, kwargs in (
            ("missing header", {"version": False}),
            ("empty header", {"version": ""}),
            ("header 0.3", {"version": "0.3"}),
            ("blank header", {"version": "  "}),
        ):
            st, hdrs, body = post(app, "/allen/api/a2a", rpc("SendMessage", message(intro())), **kwargs)
            err = (body or {}).get("error", {})
            check("A2A-Version %s is 0.3 and stores nothing" % name,
                  st == 200 and no_store(hdrs) and err.get("code") == -32009 and not files(drop),
                  "status %s code %s body %s" % (st, err.get("code"), body))
        st, hdrs, body = post(app, "/allen/api/a2a?A2A-Version=1.0",
                              rpc("SendMessage", message(intro(name="Query Version"))), version=False)
        check("query A2A-Version 1.0 is accepted when the header is absent",
              st == 200 and no_store(hdrs) and (body or {}).get("result", {}).get("message", {}).get("role") == "ROLE_AGENT"
              and len(files(drop)) == 1,
              "status %s code %s" % (st, (body or {}).get("error")))
    finally:
        for name in files(drop):
            os.remove(os.path.join(drop, name))
        os.rmdir(drop)


def nginx_patterns(conf, variable):
    """Regexes that the named map assigns 0, after nginx double-quote escapes."""
    chunk = conf.split("map $content_type $%s {" % variable, 1)[1].split("}", 1)[0]
    patterns = []
    for quoted in re.findall(r'"~\*(.*?)"\s+0;', chunk):
        out = []
        i = 0
        while i < len(quoted):
            if quoted[i] == "\\" and i + 1 < len(quoted):
                nxt = quoted[i + 1]
                out.append({"\\": "\\", "t": "\t", "n": "\n", "r": "\r"}.get(nxt, "\\" + nxt))
                i += 2
            else:
                out.append(quoted[i])
                i += 1
        patterns.append("".join(out))
    return patterns


def ctype_allowed(patterns, ctype):
    return any(re.search(pat, ctype, re.IGNORECASE) for pat in patterns)


def test_a2a_content_type():
    drop = tempfile.mkdtemp(prefix="a2a-ctype-")
    app = App(drop)
    zones = open(os.path.join(HERE, "nginx-allen-zones.conf"), encoding="utf-8").read()
    loc = open(os.path.join(HERE, "nginx-allen-location.conf"), encoding="utf-8").read()
    block = loc.split("location = /allen/api/a2a {", 1)[1].split("\n    }", 1)[0]
    a2a_pats = nginx_patterns(zones, "allen_a2a_notjson")
    json_pats = nginx_patterns(zones, "allen_notjson")
    try:
        check("a2a location does not use $allen_notjson",
              "$allen_notjson" not in block and "if ($allen_a2a_notjson)" in block,
              block.strip().splitlines()[0] if block.strip() else "missing")
        check("application/a2a+json is allowed only on the a2a map",
              ctype_allowed(a2a_pats, "application/a2a+json")
              and ctype_allowed(a2a_pats, "application/a2a+json; charset=utf-8")
              and ctype_allowed(a2a_pats, "application/json")
              and not ctype_allowed(a2a_pats, "text/plain")
              and not ctype_allowed(json_pats, "application/a2a+json")
              and ctype_allowed(json_pats, "application/json"),
              "a2a %s json %s" % (a2a_pats, json_pats))
        st, hdrs, body = post(app, "/allen/api/a2a", rpc("SendMessage", message(intro(name="A2A Type"))),
                              content_type="application/a2a+json")
        err = (body or {}).get("error")
        check("application/a2a+json is not a 415 and stores the introduction",
              st == 200 and st != 415 and no_store(hdrs) and err is None and len(files(drop)) == 1,
              "status %s err %s drops %d" % (st, err, len(files(drop))))
        st, hdrs, body = post(app, "/allen/api/a2a", rpc("GetTask", message(intro())),
                              content_type="application/a2a+json; charset=utf-8")
        check("application/a2a+json with a charset reaches the method",
              st == 200 and (body or {}).get("error", {}).get("code") == -32004 and len(files(drop)) == 1,
              "status %s body %s" % (st, body))
        st, _, body = post(app, "/allen/api/a2a", rpc("SendMessage", message(intro())),
                           content_type="text/plain")
        check("text/plain is still 415 and stores nothing more",
              st == 415 and len(files(drop)) == 1, "status %s drops %d body %s" % (st, len(files(drop)), body))
    finally:
        for name in files(drop):
            os.remove(os.path.join(drop, name))
        os.rmdir(drop)


def test_docs_and_card():
    doc = a2a.__doc__
    required = (
        "SendMessage", "message/send", "SendStreamingMessage", "GetTask", "ListTasks",
        "CancelTask", "SubscribeToTask", "CreateTaskPushNotificationConfig",
        "GetTaskPushNotificationConfig", "ListTaskPushNotificationConfigs",
        "DeleteTaskPushNotificationConfig", "GetExtendedAgentCard",
    )
    missing = [name for name in required if name not in doc]
    check("code docs name every unsupported method", not missing, "missing %s" % missing)
    card_path = os.path.join(ROOT, "om", "a2a", "agent-card.template.json")
    card = json.load(open(card_path, encoding="utf-8"))
    iface = card["supportedInterfaces"][0]
    modes = ["text/plain", "application/json"]
    published = []
    for dirpath, _, names in os.walk(os.path.join(ROOT, "public")):
        for name in names:
            if name in ("agent-card.json", "agent.json", "server.json", "did.json"):
                published.append(os.path.join(dirpath, name))
    check("agent card template is unpublished",
          card["name"] == "Aien" and card["version"] == "0.1.0"
          and iface["protocolBinding"] == "JSONRPC" and iface["protocolVersion"] == "1.0"
          and iface["url"] == "https://www.aienos.com/aien/api/a2a"
          and card["defaultInputModes"] == modes and card["defaultOutputModes"] == modes
          and card["capabilities"]["streaming"] is False
          and card["capabilities"]["pushNotifications"] is False
          and [s["id"] for s in card["skills"]] == ["introduce"]
          and not published
          and not os.path.exists(os.path.join(ROOT, "public", ".well-known", "agent-card.json")),
          "published %s" % published)


def main():
    test_valid_send()
    test_consent()
    test_malformed()
    test_unsupported()
    test_nothing_stored()
    test_version()
    test_a2a_content_type()
    test_docs_and_card()
    failed = [n for n, ok in results if not ok]
    print("%d/%d passed" % (len(results) - len(failed), len(results)))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
