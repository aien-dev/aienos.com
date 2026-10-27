#!/usr/bin/env python3
"""Test-only fake of /community/api for browser checks of the generated /community/ client.

Serves a built site directory (the site-builder `dist`) and answers every route the client calls
with canned data in the shapes of the community backend contract. Not deployed and not a backend:
the real API is scripts/community/. The WebAuthn stub is injected into /community/ pages served by
this fake only, so a headless browser can walk the passkey ceremonies without an authenticator.

Usage: python3 fake_api.py DIST_DIR [PORT]   (signed out until a passkey sign-in; ?as=member, ?as=probation
or ?as=revoked on the page URL signs in directly as that account).
"""
import base64
import json
import html
import sys
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit

DIST = sys.argv[1] if len(sys.argv) > 1 else "dist"
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 18799
API = "/community/api"
NOW = "2026-10-27T12:00:00Z"

# Minimal module: (func (export "main") (result i32) i32.const 42)
WASM = bytes.fromhex("0061736d01000000" "010501600001 7f".replace(" ", "") + "03020100" "070801046d61696e0000" "0a06010400412a0b")

STUB = b"""'use strict';
(() => {
  const b = (s) => new TextEncoder().encode(s).buffer;
  const cred = (kind) => ({ id: 'fakecred', rawId: b('fakecred'), type: 'public-key',
    response: kind === 'create'
      ? { clientDataJSON: b('{}'), attestationObject: b('att'), getTransports: () => ['internal'] }
      : { clientDataJSON: b('{}'), authenticatorData: b('auth'), signature: b('sig'), userHandle: b('user') } });
  const fake = { create: async () => cred('create'), get: async () => cred('get') };
  Object.defineProperty(navigator, 'credentials', { value: fake, configurable: true });
  window.PublicKeyCredential = window.PublicKeyCredential || function PublicKeyCredential() {};
})();
"""


def user(handle, name, **extra):
    u = {"id": handle, "handle": handle, "display_name": name, "avatar": None, "is_bot": False,
         "bot_owner": None, "role": "user", "label": None}
    u.update(extra)
    return u


DRAKE = user("drake", "Drake Stapleton", role="admin", github_login="mdrakestapleton")
AIEN = user("aien", "Aien", label="Site assistant")
ADA = user("ada-l", "Ada L")
BOT = user("bench_bot", "Bench bot", is_bot=True, bot_owner="ada-l")
ACCOUNTS = {
    "member": dict(user("you", "You Builder"), tier="member", bio="I write kernels.", projects=["omega/hello"],
                   bots=[{"handle": "you_bot", "display_name": "My bot"}], suspended=False, vouch_revoked=False,
                   invites_frozen=False, github_login=None),
    "probation": dict(user("newcomer", "New Comer"), tier="probation", bio="", projects=[], bots=[],
                      suspended=False, vouch_revoked=False, invites_frozen=False, github_login=None),
    "revoked": dict(user("revoked", "Read Only"), tier="member", bio="", projects=[], bots=[],
                    suspended=False, vouch_revoked=True, invites_frozen=False, github_login=None),
}
CODE_POST = "Hello from Omega. Press Run.\n\n```om\nfn main to int\nret 42\nend\n```"
CODE_HTML = ('<p>Hello from Omega. Press Run.</p><pre class="cm-code" data-lang="om"><code>fn main to int\n'
             'ret 42\nend</code></pre>')


def post(pid, author, text, html, **extra):
    p = {"id": pid, "author": author, "text": text, "html": html, "created": NOW, "reply_to": None,
         "reply_to_author": None, "quote": None, "likes": 3, "reposts": 1, "replies": 2, "liked": False,
         "reposted": False, "tags": ["omega"], "project": None, "hidden": False, "code_langs": []}
    p.update(extra)
    return p


POSTS = [
    post("p3", DRAKE, CODE_POST, CODE_HTML, code_langs=["om"], project="omega/hello"),
    post("p2", BOT, "Nightly bench: vecadd 1M elements, 0 mismatches.",
         "<p>Nightly bench: vecadd 1M elements, 0 mismatches.</p>", likes=0, reposts=0, replies=0),
    post("p1", ADA, "First post. Reading the Omega guide tonight.",
         '<p>First post. Reading the <a class="cm-tag" href="/community/?tag=omega">#omega</a> guide tonight.</p>',
         quote=None),
]


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *a, **k):
        super().__init__(*a, directory=DIST, **k)

    def log_message(self, fmt, *args):
        sys.stderr.write("fake_api " + (fmt % args) + "\n")

    def who(self):
        for part in (self.headers.get("Cookie") or "").split(";"):
            k, _, v = part.strip().partition("=")
            if k == "cm_s" and v in ACCOUNTS:
                return ACCOUNTS[v]
        return None

    def send_json(self, obj, status=200, cookie=None):
        body = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Cache-Control", "no-store")
        if cookie:
            self.send_header("Set-Cookie", cookie)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        url = urlsplit(self.path)
        q = parse_qs(url.query)
        if url.path == "/__test/webauthn-stub.js":
            self.send_response(200)
            self.send_header("Content-Type", "text/javascript")
            self.send_header("Content-Length", str(len(STUB)))
            self.end_headers()
            self.wfile.write(STUB)
            return
        if url.path in ("/community/", "/community/index.html"):
            page = open(DIST + "/community/index.html", "rb").read()
            page = page.replace(b'<script defer src="/js/community.js">',
                                b'<script src="/__test/webauthn-stub.js"></script><script defer src="/js/community.js">', 1)
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            if q.get("as", [""])[0] in ACCOUNTS:
                self.send_header("Set-Cookie", "cm_s=" + q["as"][0] + "; Path=/community/")
            self.send_header("Content-Length", str(len(page)))
            self.end_headers()
            self.wfile.write(page)
            return
        if not url.path.startswith(API + "/"):
            return super().do_GET()
        path = url.path[len(API):]
        me = self.who()
        if path == "/me":
            return self.send_json({"user": me, "csrf": "fake-csrf" if me else "", "unread": 2 if me else 0,
                                   "signin": {"passkey": True, "github": False},
                                   "can_invite": bool(me and me["tier"] == "member" and not me["vouch_revoked"]),
                                   "invites_left": 3 if me else 0})
        if path == "/live":
            body = b"retry: 60000\n\nevent: new\ndata: {\"latest\": \"p4\"}\n\n"
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if path == "/tags":
            return self.send_json({"tags": [{"tag": "omega", "count": 12}, {"tag": "cudapp", "count": 4}]})
        if path in ("/timeline",) or path.startswith("/tags/"):
            return self.send_json({"posts": POSTS, "next": None})
        if path.startswith("/posts/"):
            p = next((x for x in POSTS if x["id"] == path.split("/")[2]), POSTS[0])
            return self.send_json({"post": p, "ancestors": [], "replies": [POSTS[2]], "next": None})
        if path.startswith("/users/"):
            parts = path.split("/")
            if len(parts) > 3:
                key = parts[3]
                if key == "posts":
                    return self.send_json({"posts": POSTS[:1], "next": None})
                return self.send_json({"users": [ADA, AIEN], "next": None})
            h = parts[2]
            base = {"drake": DRAKE, "ada-l": ADA, "aien": AIEN, "bench_bot": BOT}.get(h)
            if base is None and me and me["handle"] == h:
                base = me
            if base is None:
                return self.send_json({"error": "not_found", "message": "No account @" + h + "."}, 404)
            u = dict(base, bio="Builder of things.", created="2026-10-01T00:00:00Z", followers=10, following=3,
                     posts=4, invitee_count=2, inviter=None if h in ("drake", "aien") else "drake",
                     followed=False, muted=False, blocked=False, suspended=False, vouch_revoked=False,
                     invites_frozen=False, can_revoke=bool(me) and h == "ada-l", descendants=1, projects=["omega/hello"])
            return self.send_json({"user": u})
        if path == "/search":
            return self.send_json({"posts": POSTS[:1], "users": [ADA], "next": None})
        if path == "/notifications":
            return self.send_json({"notifications": [
                {"id": "n1", "kind": "invite_used", "actor": ADA, "at": NOW, "read": False, "post": None},
                {"id": "n2", "kind": "reply", "actor": ADA, "at": NOW, "read": True, "post": POSTS[2]}], "next": None})
        if path == "/modlog":
            return self.send_json({"entries": [{"id": "m1", "at": NOW, "action": "revoke_vouch", "target_kind": "user",
                                                "target": "an account", "reason_public": "Spam ring.", "by": "drake"}], "next": None})
        if path == "/mod/queue":
            return self.send_json({"reports": [], "next": None})
        if path == "/me/passkeys":
            return self.send_json({"passkeys": [{"id": "k1", "name": "Laptop", "created": NOW, "last_used": NOW}]})
        if path == "/me/invites":
            return self.send_json({"invites": [
                {"id": "i1", "note": "For Grace", "status": "open", "used_by": None, "expires": "2026-11-10T12:00:00Z"},
                {"id": "i2", "note": "Ada", "status": "used", "used_by": "ada-l", "expires": "2026-11-01T12:00:00Z"}]})
        if path == "/me/export":
            return self.send_json({"user": me})
        return self.send_json({"error": "not_found", "message": "Not in the fake."}, 404)

    def do_POST(self):
        url = urlsplit(self.path)
        length = int(self.headers.get("Content-Length") or 0)
        body = json.loads(self.rfile.read(length) or b"{}")
        if url.path == "/aien/api/omega":
            return self.send_json({"ok": True, "wasm": base64.b64encode(WASM).decode(), "compile_ms": 41,
                                   "functions": [{"name": "main", "params": [], "result": "i32"}]})
        if not url.path.startswith(API + "/"):
            return self.send_json({"error": "not_found"}, 404)
        path = url.path[len(API):]
        me = self.who()
        challenge = base64.urlsafe_b64encode(b"0123456789abcdef").rstrip(b"=").decode()
        if path.endswith("/options"):
            opts = {"challenge": challenge, "rpId": "localhost", "timeout": 60000}
            if "register" in path or "add" in path:
                opts.update({"rp": {"id": "localhost", "name": "aienos.com Community"},
                             "user": {"id": challenge, "name": body.get("handle", "you"), "displayName": body.get("display_name", "You")},
                             "pubKeyCredParams": [{"type": "public-key", "alg": -7}]})
            return self.send_json(opts)
        if path in ("/auth/passkey/login/verify", "/auth/passkey/register/verify"):
            if not isinstance(body.get("credential"), dict) or "rawId" not in body["credential"]:
                return self.send_json({"error": "bad_request", "message": "No credential."}, 400)
            return self.send_json({"user": ACCOUNTS["member"], "csrf": "fake-csrf"},
                                  cookie="cm_s=member; Path=/community/")
        if path == "/auth/logout":
            return self.send_json({"ok": True}, cookie="cm_s=; Path=/community/; Max-Age=0")
        if self.headers.get("X-CM-CSRF") != "fake-csrf":
            return self.send_json({"error": "csrf", "message": "Sign in first."}, 401)
        if path == "/invites":
            return self.send_json({"id": "i3", "code": "ABCDEFGHJKMNPQRSTVWX", "url": "https://evil.example/",
                                   "expires": "2026-11-10T12:00:00Z"})
        if path == "/posts":
            p = post("p9", me, body.get("text", ""), "<p>" + html.escape(body.get("text", "")) + "</p>")
            return self.send_json({"post": p})
        return self.send_json({"ok": True})

if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
