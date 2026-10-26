#!/usr/bin/python3
"""Tests for omega_projects_pusher.py. Every rejection has an accepted control next to it.

Offline tests use a recording fake GitHub that still goes through the pusher's request
guard. StubEndToEnd runs the pusher program against a local HTTP stand-in for the GitHub
API (127.0.0.1) with a fake fine-grained token: one issue and one PR per item, dedupe,
queue-only without a token, no redirects followed. Nothing reaches GitHub.
Live tests (LIVE=1) use the operator's own gh login, never the pusher's credential: they
read the ruleset and Actions settings, try a direct push to main (must fail with GH013)
and push a submissions/ branch (must succeed, then deleted).

  nice -n 19 python3 test_pusher.py            # offline + stub
  LIVE=1 nice -n 19 python3 test_pusher.py     # plus live repository settings
"""

import base64
import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
import urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))
GW = os.environ.get("ALLEN_GW", "/tmp/allenchat/gw")
sys.path.insert(0, HERE)
sys.path.insert(0, GW)
import allen_spool  # noqa: E402
import omega_projects_pusher as P  # noqa: E402

P.spool = allen_spool
REPO = "aien-dev/omega-projects"


class FakeGitHub:
    """Records calls; answers like GitHub. Requests still pass P._check_request."""

    def __init__(self, fail=None):
        self.calls, self.n, self.fail, self.refs, self.pulls = [], 0, fail, {}, {}

    def call(self, method, path, body=None, ok=(200, 201)):
        P._check_request(method, path, body)
        self.calls.append((method, path, body))
        if self.fail:
            raise self.fail
        rest = path[len(P.REPO_PREFIX):]
        self.n += 1
        if method == "GET" and rest.startswith("labels/"):
            return {"name": rest}
        if method == "GET" and rest == "git/ref/heads/main":
            return {"object": {"sha": "a" * 40}}
        if method == "GET" and rest.startswith("git/ref/heads/submissions/"):
            raise P.HTTPRefused(404, "Not Found")
        if method == "GET" and rest.startswith("git/commits/"):
            return {"tree": {"sha": "b" * 40}}
        if method == "GET" and rest.startswith("pulls"):
            return []
        if method == "GET" and rest.startswith("issues"):
            return []
        if method == "POST" and rest == "issues":
            return {"html_url": "https://github.com/%s/issues/%d" % (REPO, self.n), "number": self.n}
        if method == "POST" and rest == "pulls":
            return {"html_url": "https://github.com/%s/pull/%d" % (REPO, self.n), "number": self.n}
        if method == "POST":
            return {"sha": "%040x" % self.n}
        raise AssertionError("unexpected %s %s" % (method, path))

    def posts(self, rest):
        return [c for c in self.calls if c[0] == "POST" and c[1] == P.REPO_PREFIX + rest]


def issue(**kw):
    it = {"v": 1, "kind": "issue", "template": "feature", "title": "Add a sine word to Omega",
          "body": "It would help to have sin and cos available in IR mode.", "area": "omega-mv"}
    it.update(kw)
    return it


def project(files=None, **kw):
    it = {"v": 1, "kind": "project", "slug": "calc", "title": "Calculator",
          "description": "A small calculator compiled from Omega.",
          "files": files or [{"name": "index.html", "content": "<!doctype html><p>calc</p>"},
                             {"name": "calc.om", "content": "fn add(a: i64, b: i64) -> i64 { a + b }\n"}]}
    it.update(kw)
    return it


class Env:
    def __init__(self, areas=("omega-mv",)):
        self.root = tempfile.mkdtemp(prefix="pusher-test-")
        self.spool_in = os.path.join(self.root, "in")
        self.status = os.path.join(self.root, "status")
        self.state = os.path.join(self.root, "state")
        for d in (self.spool_in, self.status, self.state):
            os.makedirs(d, mode=0o700)
        self.areas = os.path.join(self.root, "areas.txt")
        with open(self.areas, "w") as f:
            f.write("\n".join(areas) + "\n")

    def args(self, test_label=False):
        class A:
            pass
        a = A()
        a.spool_in, a.spool_status, a.state_dir, a.areas = self.spool_in, self.status, self.state, self.areas
        a.test_label = test_label
        return a

    def drop(self, item, iid=None, raw=None):
        iid = iid or allen_spool.item_id(item)
        with open(os.path.join(self.spool_in, iid + ".json"), "wb") as f:
            f.write(raw if raw is not None else json.dumps(item, ensure_ascii=False, sort_keys=True).encode())
        return iid

    def status_of(self, iid):
        with open(os.path.join(self.status, iid + ".json")) as f:
            return json.load(f)

    def run(self, gh, test_label=False):
        return P.process(self.args(test_label), gh)

    def cleanup(self):
        shutil.rmtree(self.root)


class Offline(unittest.TestCase):
    def setUp(self):
        self.env = Env()
        self.gh = FakeGitHub()

    def tearDown(self):
        self.env.cleanup()

    def outcome(self, item, **kw):
        iid = self.env.drop(item, **kw)
        self.env.run(self.gh)
        self.assertFalse(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))
        return self.env.status_of(iid)

    def assertRejected(self, item, needle, **kw):
        st = self.outcome(item, **kw)
        self.assertEqual(st["state"], "rejected", st)
        self.assertIn(needle, st["reason"])
        return st

    def assertFiled(self, item, **kw):
        st = self.outcome(item, **kw)
        self.assertEqual(st["state"], "filed", st)
        self.assertRegex(st["url"], r"^https://github\.com/aien-dev/omega-projects/(issues|pull)/\d+$")
        return st

    # slug traversal
    def test_slug_traversal_rejected(self):
        for slug in ("../x", "a/b"):
            self.assertRejected(project(slug=slug), "slug")
        self.assertEqual(self.gh.calls, [])

    def test_slug_control_accepted(self):
        self.assertFiled(project(slug="calc"))
        trees = self.gh.posts("git/trees")
        self.assertEqual(len(trees), 1)
        paths = sorted(t["path"] for t in trees[0][2]["tree"])
        self.assertEqual(paths, ["projects/calc/calc.om", "projects/calc/index.html", "projects/calc/meta.json"])

    # size
    def test_oversized_rejected(self):
        big = "<!doctype html><p>" + "x" * (65 * 1024) + "</p>"
        self.assertRejected(project([{"name": "index.html", "content": big}]), "larger than 64 KiB")
        self.assertEqual(self.gh.calls, [])

    def test_small_accepted(self):
        small = "<!doctype html><p>" + "x" * 1000 + "</p>"
        self.assertFiled(project([{"name": "index.html", "content": small}]))

    # binary
    def test_binary_rejected(self):
        for bad in ("a\x00b", "a\ufffeb", "a\uffffb", "a\ufdd0b", "a\ufffdb"):
            self.assertRejected(project([{"name": "index.html", "content": "<p>%s</p>" % bad}]),
                                "binary")
        raw = b'{"v":1,"kind":"issue","template":"bug","title":"\xff\xfe broken title","body":"x","area":"a"}'
        self.assertRejected(None, "UTF-8", iid="0" * 20, raw=raw)
        self.assertEqual(self.gh.calls, [])

    def test_text_accepted(self):
        self.assertFiled(project([{"name": "index.html", "content": "<p>caf\u00e9 \u2014 \u03c0 \U0001f600</p>"}]))

    # secrets
    def test_secret_rejected(self):
        self.assertRejected(issue(body="my token is ghp_" + "A1b2C3d4E5" * 3 + "abcdef please"), "secret")
        self.assertRejected(issue(body="-----BEGIN PRIVATE KEY-----\nMIIEvQ\n-----END PRIVATE KEY-----"),
                            "secret")
        self.assertRejected(issue(body="see key -----BEGIN OPENSSH PRIVATE KEY----- here"), "secret")
        self.assertEqual(self.gh.calls, [])

    def test_plain_body_accepted(self):
        self.assertFiled(issue(body="The ghp prefix is how GitHub tokens start; private keys too."))

    # external loads in index.html
    def test_external_script_rejected(self):
        for html in ('<script src="https://cdn.example/x.js"></script>',
                     '<script src="//cdn.example/x.js"></script>',
                     '<img src="https://t.example/p.gif">', '<script>fetch("https://x.example")</script>',
                     '<link rel="stylesheet" href="https://x.example/a.css">'):
            self.assertRejected(project([{"name": "index.html", "content": html}]), "elsewhere")
        self.assertEqual(self.gh.calls, [])

    def test_inline_script_accepted(self):
        self.assertFiled(project([{"name": "index.html",
                                   "content": "<script>document.body.textContent = 1 + 2;</script>"}]))

    # scrub
    def test_emails_phones_scrubbed(self):
        it = issue(title="Help from ann@example.org please",
                   body="Write to ann.lee+x@example.co.uk or call +1 (555) 123-4567 about this.")
        self.assertFiled(it)
        body = self.gh.posts("issues")[0][2]
        text = body["title"] + body["body"]
        for leak in ("ann@example.org", "ann.lee+x@example.co.uk", "555", "4567"):
            self.assertNotIn(leak, text)
        self.assertIn("[email removed]", text)
        self.assertIn("[phone removed]", text)
        self.assertTrue(body["title"].startswith("[feature] "))
        self.assertIn(P.FOOTER, body["body"])
        self.assertEqual(body["labels"], ["from-allen", "area:omega-mv"])

    def test_scrub_control_plain_text_unchanged(self):
        self.assertFiled(issue())
        body = self.gh.posts("issues")[0][2]
        self.assertTrue(body["body"].startswith(issue()["body"]))
        self.assertNotIn("removed]", body["body"])

    def test_project_prose_scrubbed_code_untouched(self):
        code = "// by bob@example.com\nfn k() -> i64 { 1234567890 }\n"
        files = [{"name": "README.md", "content": "Contact bob@example.com or 555-123-4567."},
                 {"name": "calc.om", "content": code}]
        self.assertFiled(project(files, description="Made by bob@example.com, call +44 20 7946 0958."))
        blobs = [base64.b64decode(c[2]["content"]).decode() for c in self.gh.posts("git/blobs")]
        self.assertIn(code, blobs)  # code is published byte-for-byte; Drake reviews it
        prose = "\n".join(b for b in blobs if b != code)
        self.assertNotIn("bob@example.com", prose)
        self.assertNotIn("7946", prose)
        self.assertNotIn("0958", prose)
        self.assertNotIn("555-123", prose)
        meta = json.loads([b for b in blobs if b.startswith("{")][0])
        self.assertEqual(set(meta), {"slug", "title", "description", "submitted", "item"})

    # unknown areas
    def test_area_unknown(self):
        self.assertFiled(issue(area="not-a-repo"))
        self.assertEqual(self.gh.posts("issues")[0][2]["labels"], ["from-allen", "area:unknown"])

    def test_no_areas_file_means_unknown(self):
        os.unlink(self.env.areas)
        self.assertFiled(issue(area="omega-mv"))
        self.assertEqual(self.gh.posts("issues")[0][2]["labels"], ["from-allen", "area:unknown"])

    # id
    def test_id_mismatch_rejected(self):
        self.assertRejected(issue(), "id does not match", iid="f" * 20)
        self.assertEqual(self.gh.calls, [])

    # one issue / one PR per item, dedupe
    def test_exactly_one_issue_and_dedupe(self):
        it = issue()
        st = self.assertFiled(it)
        self.assertEqual(len(self.gh.posts("issues")), 1)
        n = len(self.gh.calls)
        self.env.drop(it)
        self.env.run(self.gh)
        self.assertEqual(len(self.gh.calls), n)
        self.assertEqual(self.env.status_of(allen_spool.item_id(it))["url"], st["url"])

    def test_dedupe_from_ledger_when_status_lost(self):
        it = issue()
        self.assertFiled(it)
        os.unlink(os.path.join(self.env.status, allen_spool.item_id(it) + ".json"))
        n = len(self.gh.calls)
        self.env.drop(it)
        self.env.run(self.gh)
        self.assertEqual(len(self.gh.calls), n)

    def test_exactly_one_pr_never_main(self):
        self.assertFiled(project())
        self.assertEqual(len(self.gh.posts("pulls")), 1)
        refs = self.gh.posts("git/refs")
        self.assertEqual(len(refs), 1)
        self.assertRegex(refs[0][2]["ref"], r"^refs/heads/submissions/calc-[0-9a-f]{8}$")
        pr = self.gh.posts("pulls")[0][2]
        self.assertEqual(pr["base"], "main")
        self.assertIn(P.FOOTER, pr["body"])
        self.assertFalse([c for c in self.gh.calls if c[0] not in ("GET", "POST")])

    # caps
    def test_daily_caps(self):
        st = P.State(self.env.state)
        st.take("issue")
        st.d["issues"] = P.MAX_ISSUES_PER_DAY
        st.d["prs"] = P.MAX_PRS_PER_DAY - 1
        st.save()
        self.assertRejected(issue(), "limit")
        self.assertFiled(project())          # control: one PR left
        self.assertRejected(project(slug="calc2"), "limit")
        self.assertEqual(json.load(open(os.path.join(self.env.state, "state.json")))["prs"], P.MAX_PRS_PER_DAY)

    # tmp files and odd entries
    def test_tmp_skipped_and_symlink_refused(self):
        tmp = os.path.join(self.env.spool_in, ".tmp-abc")
        open(tmp, "w").write("{}")
        target = os.path.join(self.env.root, "secret")
        open(target, "w").write("{}")
        os.symlink(target, os.path.join(self.env.spool_in, "1" * 20 + ".json"))
        self.env.run(self.gh)
        self.assertTrue(os.path.exists(tmp))
        self.assertEqual(self.env.status_of("1" * 20)["state"], "rejected")
        self.assertTrue(os.path.exists(target))

    # transient errors
    def test_transient_kept_then_rejected(self):
        gh = FakeGitHub(fail=P.Transient("GitHub HTTP 502"))
        iid = self.env.drop(issue())
        for i in range(P.MAX_ATTEMPTS - 1):
            self.assertEqual(self.env.run(gh), (1, 0))
            self.assertTrue(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))
        self.assertEqual(self.env.run(gh), (0, 0))
        self.assertEqual(self.env.status_of(iid)["state"], "rejected")

    # request guard
    def test_guard(self):
        bad = [("POST", "/repos/aien-dev/aienos.com/issues", {}),
               ("POST", "/repos/aien-dev/omega-projects/git/refs", {"ref": "refs/heads/main", "sha": "x"}),
               ("PATCH", "/repos/aien-dev/omega-projects/git/refs/heads/main", {}),
               ("PUT", "/repos/aien-dev/omega-projects/pulls/1/merge", {}),
               ("DELETE", "/repos/aien-dev/omega-projects/git/refs/heads/main", None),
               ("POST", "/repos/aien-dev/omega-projects/pulls", {"base": "dev", "head": "submissions/x"}),
               ("GET", "/repos/aien-dev/omega-projects/../../user", None),
               ("GET", "/user", None)]
        for m, p, b in bad:
            with self.assertRaises(PermissionError, msg="%s %s" % (m, p)):
                P._check_request(m, p, b)
        # control
        P._check_request("POST", "/repos/aien-dev/omega-projects/git/refs",
                         {"ref": "refs/heads/submissions/calc-12345678", "sha": "x"})
        P._check_request("POST", "/repos/aien-dev/omega-projects/issues", {})

    # activation
    def test_not_activated_queues_and_keeps(self):
        iid = self.env.drop(issue())
        bad = self.env.drop(project(slug="../x"))
        self.assertEqual(self.env.run(None), (0, 1))
        st = self.env.status_of(iid)
        self.assertEqual((st["state"], st["reason"], st["url"]), ("queued", P.AWAITING, None))
        self.assertTrue(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))
        self.assertEqual(os.stat(os.path.join(self.env.status, iid + ".json")).st_mode & 0o777, 0o640)
        self.assertEqual(self.env.status_of(bad)["state"], "rejected")  # validation still runs
        # control: once activated the same item is filed and leaves the spool
        self.assertEqual(self.env.run(self.gh), (0, 0))
        self.assertEqual(self.env.status_of(iid)["state"], "filed")
        self.assertEqual(len(self.gh.posts("issues")), 1)

    def test_token_kinds(self):
        d = self.env.root
        for name, value in (("gho", "gho_" + "a" * 36), ("ghp", "ghp_" + "a" * 36), ("empty", "")):
            path = os.path.join(d, name)
            with open(path, "w") as f:
                f.write(value)
            self.assertIsNone(P.load_token(path), name)
        self.assertIsNone(P.load_token(os.path.join(d, "missing")))
        self.assertIsNone(P.load_token(None))
        good = os.path.join(d, "pat")
        with open(good, "w") as f:
            f.write("github_pat_" + "A1" * 30 + "\n")
        self.assertEqual(P.load_token(good), "github_pat_" + "A1" * 30)  # control
        with self.assertRaises(ValueError):
            P.GitHub("gho_" + "a" * 36)
        for base in ("https://evil.example", "http://10.0.0.1:80", "http://127.0.0.1.evil.example:1"):
            with self.assertRaises(ValueError):
                P.GitHub("github_pat_" + "A1" * 30, api=base)
        P.GitHub("github_pat_" + "A1" * 30, api="http://127.0.0.1:9")  # control


FAKE_PAT = "github_pat_" + "T3st" * 20


class Stub(http.server.ThreadingHTTPServer):
    """A small stateful stand-in for the GitHub REST calls the pusher makes."""

    def __init__(self):
        super().__init__(("127.0.0.1", 0), StubHandler)
        self.log, self.issues, self.refs, self.pulls, self.labels = [], [], {}, [], {"from-allen", "test"}
        self.objects, self.redirect = 0, None
        self.refs["refs/heads/main"] = "c" * 40
        threading.Thread(target=self.serve_forever, daemon=True).start()

    @property
    def base(self):
        return "http://127.0.0.1:%d" % self.server_address[1]


class StubHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def reply(self, code, obj=None, headers=()):
        raw = json.dumps(obj).encode() if obj is not None else b""
        self.send_response(code)
        for k, v in headers:
            self.send_header(k, v)
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def handle_one(self, method):
        s = self.server
        n = int(self.headers.get("Content-Length") or 0)
        body = json.loads(self.rfile.read(n)) if n else None
        s.log.append((method, self.path, self.headers.get("Authorization"), body))
        if s.redirect:
            return self.reply(302, headers=[("Location", s.redirect)])
        if self.headers.get("Authorization") != "Bearer " + FAKE_PAT:
            return self.reply(401, {"message": "Bad credentials"})
        pre = "/repos/%s/" % REPO
        if not self.path.startswith(pre):
            return self.reply(404, {"message": "Not Found"})
        rest = self.path[len(pre):]
        if method == "GET" and rest.startswith("labels/"):
            name = urllib.parse.unquote(rest[7:])
            return self.reply(200, {"name": name}) if name in s.labels else self.reply(404, {"message": "Not Found"})
        if method == "POST" and rest == "labels":
            s.labels.add(body["name"])
            return self.reply(201, {"name": body["name"]})
        if method == "GET" and rest.startswith("git/ref/heads/"):
            ref = "refs/heads/" + rest[len("git/ref/heads/"):]
            if ref in s.refs:
                return self.reply(200, {"ref": ref, "object": {"sha": s.refs[ref]}})
            return self.reply(404, {"message": "Not Found"})
        if method == "GET" and rest.startswith("git/commits/"):
            return self.reply(200, {"sha": rest[12:], "tree": {"sha": "d" * 40}})
        if method == "POST" and rest in ("git/blobs", "git/trees", "git/commits"):
            s.objects += 1
            return self.reply(201, {"sha": "%040x" % s.objects})
        if method == "POST" and rest == "git/refs":
            if body["ref"] in s.refs:
                return self.reply(422, {"message": "Reference already exists"})
            s.refs[body["ref"]] = body["sha"]
            return self.reply(201, {"ref": body["ref"]})
        if method == "GET" and rest.startswith("pulls"):
            q = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
            head = q.get("head", [""])[0].split(":", 1)[-1]
            return self.reply(200, [p for p in s.pulls if p["head"] == head])
        if method == "POST" and rest == "pulls":
            num = len(s.issues) + len(s.pulls) + 1
            pr = dict(body, number=num, html_url="https://github.com/%s/pull/%d" % (REPO, num))
            s.pulls.append(pr)
            return self.reply(201, pr)
        if method == "POST" and rest == "issues":
            num = len(s.issues) + len(s.pulls) + 1
            it = dict(body, number=num, html_url="https://github.com/%s/issues/%d" % (REPO, num))
            s.issues.append(it)
            return self.reply(201, it)
        if method == "GET" and rest.startswith("issues"):
            return self.reply(200, s.issues)
        if method == "POST" and rest.endswith("/labels") and rest.startswith("issues/"):
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


class StubEndToEnd(unittest.TestCase):
    """Runs the pusher program as a subprocess against the local stub (no GitHub)."""

    def setUp(self):
        self.env = Env()
        self.stub = Stub()
        self.token = os.path.join(self.env.root, "omega-projects-token")
        with open(self.token, "w") as f:
            f.write(FAKE_PAT + "\n")

    def tearDown(self):
        self.stub.shutdown()
        self.stub.server_close()
        self.env.cleanup()

    def pusher(self, *extra):
        cmd = [sys.executable, os.path.join(HERE, "omega_projects_pusher.py"),
               "--spool-in", self.env.spool_in, "--spool-status", self.env.status,
               "--state-dir", self.env.state, "--areas", self.env.areas, "--spool-module-dir", GW,
               "--api-base", self.stub.base, "--retry-delay", "0", "--idle-delay", "0"] + list(extra)
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=60, env={"PATH": "/usr/bin:/bin"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn(FAKE_PAT, r.stdout + r.stderr)
        return r.stderr

    def test_one_issue_one_pr_then_dedupe(self):
        it = issue(body="Please add sin. Reach me at a.b@example.org or +1 555 010 9999.")
        pr = project(slug="calc", files=[{"name": "index.html", "content": "<p>calc</p>"},
                                         {"name": "README.md", "content": "Calculator by a.b@example.org."}])
        ids = [self.env.drop(it), self.env.drop(pr)]
        self.pusher("--token-file", self.token)
        self.assertEqual(len(self.stub.issues), 1)
        self.assertEqual(len(self.stub.pulls), 1)
        self.assertEqual(sorted(self.stub.refs), ["refs/heads/main", "refs/heads/submissions/calc-" + ids[1][:8]])
        self.assertEqual(self.stub.refs["refs/heads/main"], "c" * 40)  # main untouched
        iss = self.stub.issues[0]
        self.assertEqual(iss["labels"], ["from-allen", "area:omega-mv"])
        self.assertTrue(iss["title"].startswith("[feature] "))
        self.assertNotIn("a.b@example.org", iss["body"])
        self.assertNotIn("9999", iss["body"])
        self.assertIn(P.FOOTER, iss["body"])
        self.assertEqual(self.stub.pulls[0]["base"], "main")
        self.assertEqual({m for m, *_ in self.stub.log}, {"GET", "POST"})
        self.assertTrue(all(a == "Bearer " + FAKE_PAT for _, _, a, _ in self.stub.log))
        for i in ids:
            self.assertEqual(self.env.status_of(i)["state"], "filed")
            self.assertEqual(os.stat(os.path.join(self.env.status, i + ".json")).st_mode & 0o777, 0o640)
        self.assertEqual(os.listdir(self.env.spool_in), [])
        # dedupe: the same items again file nothing and make no request at all
        n = len(self.stub.log)
        self.env.drop(it)
        self.env.drop(pr)
        self.pusher("--token-file", self.token)
        self.assertEqual(len(self.stub.log), n)
        self.assertEqual((len(self.stub.issues), len(self.stub.pulls)), (1, 1))
        # dedupe without the local ledger: the GitHub-side lookup finds the existing PR
        os.unlink(os.path.join(self.env.state, "state.json"))
        for i in ids:
            os.unlink(os.path.join(self.env.status, i + ".json"))
        self.env.drop(pr)
        self.pusher("--token-file", self.token)
        self.assertEqual(len(self.stub.pulls), 1)
        self.assertEqual(self.env.status_of(ids[1])["url"], self.stub.pulls[0]["html_url"])

    def test_no_token_queues_without_any_request(self):
        iid = self.env.drop(issue())
        self.pusher()  # no --token-file, no CREDENTIALS_DIRECTORY
        bad = os.path.join(self.env.root, "gho")
        with open(bad, "w") as f:
            f.write("gho_" + "a" * 36)
        self.pusher("--token-file", bad)
        self.assertEqual(self.stub.log, [])
        self.assertEqual(self.env.status_of(iid)["reason"], P.AWAITING)
        self.assertTrue(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))
        self.pusher("--token-file", self.token)  # control
        self.assertEqual(len(self.stub.issues), 1)

    def test_redirect_not_followed(self):
        other = Stub()
        try:
            self.stub.redirect = other.base + "/repos/%s/issues" % REPO
            client = P.GitHub(FAKE_PAT, api=self.stub.base)
            with self.assertRaises(P.HTTPRefused):
                client.call("POST", P.REPO_PREFIX + "issues", {"title": "x"})
            self.assertEqual(other.log, [])  # the token never reached the redirect target
            self.assertEqual(len(self.stub.log), 1)
        finally:
            other.shutdown()
            other.server_close()


@unittest.skipUnless(os.environ.get("LIVE") == "1", "LIVE=1 checks aien-dev/omega-projects settings")
class Live(unittest.TestCase):
    """Operator setup checks with the normal session credential, never the pusher's."""

    def test_live_ruleset_settings(self):
        r = subprocess.run(["gh", "api", "repos/%s/rules/branches/main" % REPO], capture_output=True, text=True)
        rules = {x["type"]: x.get("parameters") for x in json.loads(r.stdout)}
        self.assertEqual(set(rules), {"deletion", "non_fast_forward", "pull_request"})
        self.assertEqual(rules["pull_request"]["required_approving_review_count"], 1)
        self.assertTrue(rules["pull_request"]["require_code_owner_review"])
        r = subprocess.run(["gh", "api", "repos/%s/actions/permissions" % REPO], capture_output=True, text=True)
        self.assertFalse(json.loads(r.stdout)["enabled"])

    def test_live_direct_push_to_main_rejected(self):
        d = tempfile.mkdtemp(prefix="pusher-push-")
        try:
            run = lambda *a: subprocess.run(a, cwd=d, capture_output=True, text=True)  # noqa: E731
            self.assertEqual(run("git", "clone", "-q", "--depth", "1",
                                 "git@github.com:%s.git" % REPO, ".").returncode, 0)
            open(os.path.join(d, "CONTROL"), "w").write("x\n")
            run("git", "add", "CONTROL")
            self.assertEqual(run("git", "-c", "user.name=control", "-c", "user.email=control@example.invalid",
                                 "commit", "-qm", "control").returncode, 0)
            r = run("git", "push", "origin", "HEAD:refs/heads/main")
            print("\nLIVE push main rc=%d: %s" % (r.returncode, " | ".join(
                ln for ln in r.stderr.splitlines() if "GH013" in ln or "rule" in ln.lower())[:400]), file=sys.stderr)
            self.assertNotEqual(r.returncode, 0)
            self.assertIn("GH013", r.stderr)
            br = "submissions/push-control-%d" % os.getpid()
            r2 = run("git", "push", "origin", "HEAD:refs/heads/" + br)
            print("LIVE push %s rc=%d" % (br, r2.returncode), file=sys.stderr)
            self.assertEqual(r2.returncode, 0, r2.stderr[-300:])
            r3 = run("git", "push", "origin", "--delete", br)
            self.assertEqual(r3.returncode, 0, r3.stderr[-300:])
        finally:
            shutil.rmtree(d)


if __name__ == "__main__":
    unittest.main(verbosity=2)
