#!/usr/bin/python3
"""github_stub: a local stand-in for the GitHub REST calls omega-projects-pusher makes.

Test tool only. It listens on 127.0.0.1, accepts one fake fine-grained token, and keeps
issues, pull requests, refs and labels in memory. Issue and PR numbers start at 900001
so a stub URL can never be mistaken for a real omega-projects issue.

Failure injection, for retry tests:
  POST /_stub/mode {"mode": "ok|401|403|429|500|502|timeout", "count": N}
    the next N API requests (or all of them when N is -1) answer with that failure;
    "timeout" holds the request for --hold seconds, longer than the pusher's 10 s.
  GET /_stub/state -> {"issues", "pulls", "refs", "log", "mode"}
The /_stub/ paths are outside /repos/aien-dev/omega-projects/, so the pusher's request
guard can never reach them.

  python3 github_stub.py --port 18190 [--token github_pat_...] [--log FILE]
"""

import argparse
import http.server
import re
import json
import sys
import threading
import time
import urllib.parse

REPO = "aien-dev/omega-projects"
FAKE_PAT = "github_pat_" + "T3st" * 20
FIRST_NUMBER = 900001


class Stub(http.server.ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, port=0, token=FAKE_PAT, hold=12.0, logfile=None):
        super().__init__(("127.0.0.1", port), StubHandler)
        self.token, self.hold, self.logfile = token, hold, logfile
        self.lock = threading.Lock()
        self.log, self.issues, self.pulls, self.comments = [], [], [], []
        # A small git object store: blobs {sha: base64}, trees {sha: {path: blob sha}} (flat,
        # recursive listings derive the directories), commits {sha: {tree, parents, message}}.
        self.blobs = {"b" * 40: "IyBvbWVnYS1wcm9qZWN0cwo="}
        self.trees = {"d" * 40: {"README.md": "b" * 40, "projects/README.md": "b" * 40}}
        self.commits = {"c" * 40: {"tree": "d" * 40, "parents": [], "message": "Start"}}
        self.refs = {"refs/heads/main": "c" * 40}
        self.labels = {"from-aien", "test", "area:unknown"}
        self.objects, self.redirect = 0, None
        self.mode, self.count = "ok", 0

    def new_sha(self):
        self.objects += 1
        return "%040x" % self.objects

    def files_at(self, ref):
        """{path: decoded text} at a branch head, for tests."""
        import base64
        tree = self.trees[self.commits[self.refs["refs/heads/" + ref]]["tree"]]
        return {p: base64.b64decode(self.blobs[s]).decode() for p, s in tree.items()}
        self.mode, self.count = "ok", 0

    @property
    def base(self):
        return "http://127.0.0.1:%d" % self.server_address[1]

    def start(self):
        threading.Thread(target=self.serve_forever, daemon=True).start()
        return self

    def set_mode(self, mode, count=-1):
        with self.lock:
            self.mode, self.count = mode, count

    def next_number(self):
        return FIRST_NUMBER + len(self.issues) + len(self.pulls)

    def record(self, entry):
        self.log.append(entry)
        if self.logfile:
            with open(self.logfile, "a", encoding="utf-8") as f:
                f.write(json.dumps(entry, sort_keys=True) + "\n")


class StubHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def reply(self, code, obj=None, headers=()):
        # Record the request before the client can see the answer, so a client that has
        # its reply always finds the request (with its status) in /_stub/state.
        entry = getattr(self, "entry", None)
        if entry is not None:
            self.entry = None
            entry["status"] = code
            self.server.record(entry)
        raw = json.dumps(obj).encode() if obj is not None else b""
        try:
            self.send_response(code)
            for k, v in headers:
                self.send_header(k, v)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(raw)))
            self.end_headers()
            self.wfile.write(raw)
        except (BrokenPipeError, ConnectionResetError):
            pass
        return code

    def control(self, method, body):
        s = self.server
        if method == "POST" and self.path == "/_stub/mode":
            s.set_mode(str(body.get("mode", "ok")), int(body.get("count", -1)))
            return self.reply(200, {"mode": s.mode, "count": s.count})
        if method == "GET" and self.path == "/_stub/state":
            return self.reply(200, {"issues": s.issues, "pulls": s.pulls, "refs": s.refs,
                                    "log": s.log, "mode": s.mode, "count": s.count})
        return self.reply(404, {"message": "Not Found"})

    def injected(self):
        """The failure to answer with, if one is armed."""
        s = self.server
        with s.lock:
            if s.mode == "ok" or s.count == 0:
                return None
            if s.count > 0:
                s.count -= 1
            return s.mode

    def handle_one(self, method):
        s = self.server
        n = int(self.headers.get("Content-Length") or 0)
        body = json.loads(self.rfile.read(n)) if n else None
        if self.path.startswith("/_stub/"):
            return self.control(method, body or {})
        entry = {"t": round(time.time(), 3), "method": method, "path": self.path,
                 "auth_ok": self.headers.get("Authorization") == "Bearer " + s.token}
        if method == "POST" and isinstance(body, dict) and "title" in body:
            entry["title"] = body["title"]
        self.entry = entry
        status = self.route(method, body, entry)
        if self.entry is not None:  # route answered without reply()
            self.entry = None
            entry["status"] = status
            s.record(entry)

    def route(self, method, body, entry):
        s = self.server
        if s.redirect:
            return self.reply(302, headers=[("Location", s.redirect)])
        if not entry["auth_ok"]:
            return self.reply(401, {"message": "Bad credentials"})
        fail = self.injected()
        if fail:
            entry["injected"] = fail
            if fail == "timeout":
                time.sleep(s.hold)
                return self.reply(504, {"message": "held"})
            code = int(fail)
            msg = {401: "Bad credentials", 403: "Resource not accessible by personal access token",
                   429: "API rate limit exceeded"}.get(code, "Server Error")
            return self.reply(code, {"message": msg})
        pre = "/repos/%s/" % REPO
        if not self.path.startswith(pre):
            return self.reply(404, {"message": "Not Found"})
        rest = self.path[len(pre):]
        with s.lock:
            if method == "GET" and rest.startswith("labels/"):
                name = urllib.parse.unquote(rest[7:])
                if name in s.labels:
                    return self.reply(200, {"name": name})
                return self.reply(404, {"message": "Not Found"})
            if method == "POST" and rest == "labels":
                s.labels.add(body["name"])
                return self.reply(201, {"name": body["name"]})
            if method == "GET" and rest.startswith("git/ref/heads/"):
                ref = "refs/heads/" + rest[len("git/ref/heads/"):]
                if ref in s.refs:
                    return self.reply(200, {"ref": ref, "object": {"sha": s.refs[ref]}})
                return self.reply(404, {"message": "Not Found"})
            if method == "GET" and rest.startswith("git/commits/"):
                c = s.commits.get(rest[12:])
                if not c:
                    return self.reply(404, {"message": "Not Found"})
                return self.reply(200, {"sha": rest[12:], "tree": {"sha": c["tree"]}, "message": c["message"],
                                        "parents": [{"sha": p} for p in c["parents"]]})
            if method == "GET" and rest.startswith("git/trees/"):
                t = s.trees.get(rest[10:].split("?", 1)[0])
                if t is None:
                    return self.reply(404, {"message": "Not Found"})
                dirs = sorted({p.rsplit("/", 1)[0] for p in t if "/" in p})
                tree = [{"path": d, "type": "tree", "sha": "e" * 40} for d in dirs]
                tree += [{"path": p, "type": "blob", "mode": "100644", "sha": b} for p, b in sorted(t.items())]
                return self.reply(200, {"sha": rest[10:], "tree": tree, "truncated": False})
            if method == "GET" and rest.startswith("git/blobs/"):
                b = s.blobs.get(rest[10:])
                if b is None:
                    return self.reply(404, {"message": "Not Found"})
                return self.reply(200, {"sha": rest[10:], "content": b, "encoding": "base64"})
            if method == "POST" and rest == "git/blobs":
                sha = s.new_sha()
                s.blobs[sha] = body["content"]
                return self.reply(201, {"sha": sha})
            if method == "POST" and rest == "git/trees":
                t = dict(s.trees.get(body.get("base_tree"), {}))
                for e in body["tree"]:
                    if e["sha"] is None:
                        t.pop(e["path"], None)
                    else:
                        t[e["path"]] = e["sha"]
                for sha, old in s.trees.items():  # same content, same sha, as in git
                    if old == t:
                        return self.reply(201, {"sha": sha})
                sha = s.new_sha()
                s.trees[sha] = t
                return self.reply(201, {"sha": sha})
            if method == "POST" and rest == "git/commits":
                sha = s.new_sha()
                s.commits[sha] = {"tree": body["tree"], "parents": body["parents"], "message": body["message"]}
                return self.reply(201, {"sha": sha})
            if method == "POST" and rest == "git/refs":
                if body["ref"] in s.refs:
                    return self.reply(422, {"message": "Reference already exists"})
                s.refs[body["ref"]] = body["sha"]
                return self.reply(201, {"ref": body["ref"]})
            if method == "PATCH" and rest.startswith("git/refs/"):
                ref = rest[len("git/"):]
                if ref not in s.refs:
                    return self.reply(422, {"message": "Reference does not exist"})
                if s.refs[ref] not in s.commits.get(body["sha"], {}).get("parents", []):
                    return self.reply(422, {"message": "Update is not a fast forward"})
                s.refs[ref] = body["sha"]
                return self.reply(200, {"ref": ref, "object": {"sha": body["sha"]}})
            if method == "GET" and re.fullmatch(r"pulls/[0-9]+", rest):
                for p in s.pulls:
                    if p["number"] == int(rest[6:]):
                        return self.reply(200, p)
                return self.reply(404, {"message": "Not Found"})
            if method == "PATCH" and re.fullmatch(r"pulls/[0-9]+", rest):
                for p in s.pulls:
                    if p["number"] == int(rest[6:]):
                        p.update(body)
                        return self.reply(200, p)
                return self.reply(404, {"message": "Not Found"})
            if method == "GET" and rest.startswith("pulls"):
                q = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
                head = q.get("head", [""])[0].split(":", 1)[-1]
                state = q.get("state", ["open"])[0]
                return self.reply(200, [p for p in s.pulls if (not head or p["head"]["ref"] == head)
                                        and state in ("all", p["state"])])
            if method == "POST" and rest == "pulls":
                num = s.next_number()
                pr = dict(body, number=num, state="open", head={"ref": body["head"]}, base={"ref": body["base"]},
                          html_url="https://github.com/%s/pull/%d" % (REPO, num))
                s.pulls.append(pr)
                return self.reply(201, pr)
            if method == "POST" and re.fullmatch(r"issues/[0-9]+/comments", rest):
                s.comments.append(dict(body, number=int(rest.split("/")[1])))
                return self.reply(201, {"id": len(s.comments)})
            if method == "POST" and rest == "issues":
                num = s.next_number()
                it = dict(body, number=num, html_url="https://github.com/%s/issues/%d" % (REPO, num))
                s.issues.append(it)
                return self.reply(201, it)
            if method == "GET" and rest.startswith("issues"):
                return self.reply(200, s.issues)
            if method == "POST" and rest.startswith("issues/") and rest.endswith("/labels"):
                return self.reply(200, [{"name": x} for x in body["labels"]])
        return self.reply(404, {"message": "Not Found"})

    def do_GET(self):
        self.handle_one("GET")

    def do_POST(self):
        self.handle_one("POST")

    def do_PATCH(self):
        self.handle_one("PATCH")

    def do_PUT(self):
        self.handle_one("PUT")

    def do_DELETE(self):
        self.handle_one("DELETE")


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--port", type=int, required=True)
    p.add_argument("--token", default=FAKE_PAT)
    p.add_argument("--hold", type=float, default=12.0)
    p.add_argument("--log")
    a = p.parse_args()
    s = Stub(a.port, a.token, a.hold, a.log)
    print("github_stub listening on %s" % s.base, flush=True)
    try:
        s.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
