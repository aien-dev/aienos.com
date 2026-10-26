#!/usr/bin/python3
"""Tests for omega_projects_pusher.py. Every rejection has an accepted control next to it.

Offline tests record every call and forward it through the real client and the request
guard to an in-process stand-in for the GitHub API (github_stub.py, 127.0.0.1) that keeps
a small git object store, so saves, publishes, declines and the index are checked against
real trees and refs. StubEndToEnd runs the pusher program as a subprocess against the stub
with a fake fine-grained token. Nothing reaches GitHub.
Live tests (LIVE=1) use the operator's own gh login, never the pusher's credential: they
read the ruleset and Actions settings, try a direct push to main (must fail with GH013)
and push a u/ branch (must succeed, then deleted).

  nice -n 19 python3 test_pusher.py            # offline + stub
  LIVE=1 nice -n 19 python3 test_pusher.py     # plus live repository settings
"""

import base64
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
GW = os.environ.get("ALLEN_GW", "/tmp/allenchat/gw")
sys.path.insert(0, HERE)
sys.path.insert(0, GW)
import allen_build  # noqa: E402
import allen_spool  # noqa: E402
import omega_projects_pusher as P  # noqa: E402
from github_stub import FAKE_PAT, Stub  # noqa: E402

P.spool = allen_spool
P.build = allen_build
REPO = "aien-dev/omega-projects"
ADA = "ada-0a1b2c3d"
BOB = "anon-0123456789"


class FakeGitHub:
    """Records calls, then sends them through the real client (and so the request guard) to an
    in-process stub. `fail` is raised instead for the first `fail_count` calls (-1: always)."""

    def __init__(self, fail=None, fail_count=-1):
        self.calls, self.fail, self.fail_count = [], fail, fail_count
        self.stub = Stub().start()
        self.client = P.GitHub(FAKE_PAT, api=self.stub.base)

    def call(self, method, path, body=None):
        P._check_request(method, path, body)
        self.calls.append((method, path, body))
        if self.fail and self.fail_count != 0:
            self.fail_count -= 1
            raise self.fail
        return self.client.call(method, path, body)

    def posts(self, rest, method="POST"):
        return [c for c in self.calls if c[0] == method and c[1] == P.REPO_PREFIX + rest]

    def close(self):
        self.stub.shutdown()
        self.stub.server_close()


def issue(**kw):
    it = {"v": 1, "kind": "issue", "template": "feature", "title": "Add a sine word to Omega",
          "body": "It would help to have sin and cos available in IR mode.", "area": "omega-mv"}
    it.update(kw)
    return it


README = "# Calculator\n\nAdds and multiplies. It helps students check homework.\n"


def project(files=None, kind="publish", **kw):
    """A save or publish item; README.md is added when the files lack one."""
    files = files or [{"name": "index.html", "content": "<!doctype html><p>calc</p>"},
                      {"name": "calc.om", "content": "fn add(a: i64, b: i64) -> i64 { a + b }\n"}]
    if not any(f["name"] == "README.md" for f in files):
        files = files + [{"name": "README.md", "content": README}]
    it = {"v": 1, "kind": kind, "handle": ADA, "project": "calc", "title": "Calculator",
          "description": "A small calculator compiled from Omega.", "files": files}
    it.update(kw)
    return it


def save(files=None, **kw):
    return project(files, kind="save", **kw)


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
        a.backoff = (0.0, 0.0, 0.0, 0.0)
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
        self.gh.close()
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
        self.assertRegex(st["url"], r"^https://github\.com/aien-dev/omega-projects/(issues/\d+|pull/\d+|commit/[0-9a-f]{40})$")
        return st

    # paths: handle and project are names, never paths
    def test_path_traversal_rejected(self):
        for name in ("../x", "a/b", ".github"):
            self.assertRejected(project(project=name), "project name")
        for handle in ("../ada-0a1b2c3d", "ada", "ada-0a1b2c3d/x", "Ada-0a1b2c3d", "anon-xyz"):
            self.assertRejected(project(handle=handle), "unknown handle")
        for fname in ("../x.om", ".github/workflows/x.js", "a/b.om", "x.exe", "meta.json", ".env"):
            self.assertRejected(project([{"name": fname, "content": "x"}]), "file names allowed")
        self.assertEqual(self.gh.calls, [])

    def test_path_control_accepted(self):
        self.assertFiled(project(project="calc"))
        trees = self.gh.posts("git/trees")
        self.assertEqual(len(trees), 1)
        paths = sorted(t["path"] for t in trees[0][2]["tree"])
        d = "projects/%s/calc/" % ADA
        self.assertEqual(paths, [d + "README.md", d + "calc.om", d + "index.html", d + "meta.json"])

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
        self.assertEqual(body["labels"], ["from-aien", "area:omega-mv"])

    def test_scrub_control_plain_text_unchanged(self):
        self.assertFiled(issue())
        body = self.gh.posts("issues")[0][2]
        self.assertTrue(body["body"].startswith(issue()["body"]))
        self.assertNotIn("removed]", body["body"])

    def test_project_prose_scrubbed_code_untouched(self):
        code = "// the answer\nfn k() -> i64 { 1234567890 }\n"
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
        self.assertEqual(set(meta), {"v", "handle", "project", "title", "description", "saved", "item",
                                     "built_with", "files"})

    def test_email_in_code_rejected(self):
        self.assertRejected(project([{"name": "calc.om", "content": "// by bob@example.com\nfn k() -> i64 { 1 }\n"}]),
                            "email address")
        self.assertRejected(project([{"name": "index.html", "content": "<p>calc</p>"},
                                     {"name": "app.js", "content": "fetch('/x')"}]), "elsewhere")
        no_readme = dict(project(), files=[{"name": "index.html", "content": "<p>calc</p>"}])
        self.assertRejected(no_readme, "README.md")
        self.assertEqual(self.gh.calls, [])

    # unknown areas
    def test_area_unknown(self):
        self.assertFiled(issue(area="not-a-repo"))
        self.assertEqual(self.gh.posts("issues")[0][2]["labels"], ["from-aien", "area:unknown"])

    def test_no_areas_file_means_unknown(self):
        os.unlink(self.env.areas)
        self.assertFiled(issue(area="omega-mv"))
        self.assertEqual(self.gh.posts("issues")[0][2]["labels"], ["from-aien", "area:unknown"])

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
        self.assertEqual([r[2]["ref"] for r in refs], ["refs/heads/p/%s/calc" % ADA])
        pr = self.gh.posts("pulls")[0][2]
        self.assertEqual((pr["base"], pr["head"]), ("main", "p/%s/calc" % ADA))
        self.assertIn(P.FOOTER, pr["body"])
        self.assertEqual(self.gh.stub.refs["refs/heads/main"], "c" * 40)
        self.assertFalse([c for c in self.gh.calls if "heads/main" in c[1] and c[0] != "GET"])

    # the build loop
    def test_save_twice_two_commits_on_user_branch(self):
        s = self.gh.stub
        first = self.assertFiled(save())["url"]
        files2 = [{"name": "index.html", "content": "<!doctype html><p>calc v2</p>"},
                  {"name": "calc.om", "content": "fn mul(a: i64, b: i64) -> i64 { a * b }\n"}]
        second = self.assertFiled(save(files2, title="Calculator two"))["url"]
        head = s.refs["refs/heads/u/" + ADA]
        self.assertEqual(second.rsplit("/", 1)[1], head)
        self.assertEqual(s.commits[head]["parents"], [first.rsplit("/", 1)[1]])  # two commits, in order
        self.assertEqual(s.commits[first.rsplit("/", 1)[1]]["parents"], ["c" * 40])  # branched from main
        files = s.files_at("u/" + ADA)
        self.assertIn("calc v2", files["projects/%s/calc/index.html" % ADA])
        self.assertEqual(s.refs["refs/heads/main"], "c" * 40)
        self.assertEqual(s.pulls, [])  # saving never opens a pull request
        # a file dropped from the project is removed from the home, nothing else changes
        self.assertFiled(save([{"name": "index.html", "content": "<p>only html</p>"}]))
        files = s.files_at("u/" + ADA)
        self.assertNotIn("projects/%s/calc/calc.om" % ADA, files)
        self.assertIn("projects/README.md", files)

    def test_two_users_two_homes(self):
        s = self.gh.stub
        self.assertFiled(save())
        self.assertFiled(save(handle=BOB))
        ada = {p for p in s.files_at("u/" + ADA) if p.startswith("projects/") and p != "projects/README.md"}
        bob = {p for p in s.files_at("u/" + BOB) if p.startswith("projects/") and p != "projects/README.md"}
        self.assertTrue(ada and all(p.startswith("projects/%s/" % ADA) for p in ada), ada)
        self.assertTrue(bob and all(p.startswith("projects/%s/" % BOB) for p in bob), bob)
        for c in self.gh.posts("git/trees"):
            handles = {e["path"].split("/")[1] for e in c[2]["tree"]}
            self.assertEqual(len(handles), 1)

    def test_republish_reuses_open_pr_then_decline_closes(self):
        s = self.gh.stub
        url = self.assertFiled(project())["url"]
        url2 = self.assertFiled(project(title="Calculator v2"))["url"]
        self.assertEqual(url, url2)
        self.assertEqual(len(s.pulls), 1)
        head = s.refs["refs/heads/p/%s/calc" % ADA]
        self.assertEqual(len(s.commits[head]["parents"]), 1)  # main did not move: plain child
        idx = allen_build.read_index(self.env.status)
        self.assertEqual([o["number"] for o in idx["open"]], [s.pulls[0]["number"]])
        n = s.pulls[0]["number"]
        self.assertFiled({"v": 1, "kind": "decline", "pr": n, "reason": "Needs a README that says who it helps."})
        self.assertEqual(s.pulls[0]["state"], "closed")
        self.assertIn("Declined by Drake", s.comments[0]["body"])
        self.assertEqual(allen_build.read_index(self.env.status)["open"], [])

    def test_decline_refuses_other_pulls(self):
        s = self.gh.stub
        s.pulls.append({"number": 7, "state": "open", "head": {"ref": "feature/x"}, "base": {"ref": "main"},
                        "html_url": "https://github.com/%s/pull/7" % REPO})
        self.assertRejected({"v": 1, "kind": "decline", "pr": 7, "reason": ""}, "not a project publish")
        self.assertEqual(s.pulls[0]["state"], "open")
        self.assertFalse(self.gh.posts("pulls/7", method="PATCH"))

    def test_gallery_lists_merged_projects(self):
        s = self.gh.stub
        self.assertFiled(project())
        s.refs["refs/heads/main"] = s.refs["refs/heads/p/%s/calc" % ADA]  # Drake merged it
        s.pulls[0]["state"] = "closed"
        iid = self.env.drop({"v": 1, "kind": "refresh", "nonce": "0123456789abcdef"})
        self.env.run(self.gh)
        self.assertEqual(self.env.status_of(iid)["state"], "filed")
        idx = allen_build.read_index(self.env.status)
        self.assertEqual(idx["open"], [])
        self.assertEqual([(g["handle"], g["project"], g["title"]) for g in idx["gallery"]], [(ADA, "calc", "Calculator")])
        self.assertEqual(os.stat(os.path.join(self.env.status, "index.json")).st_mode & 0o777, 0o640)

    # caps
    def test_daily_caps(self):
        st = P.State(self.env.state)
        st.take("issue")
        st.d["issues"] = P.CAPS["issue"][1]
        st.d["prs"] = P.CAPS["publish"][1] - 1
        st.save()
        iid = self.env.drop(issue())
        self.env.run(self.gh)
        st = self.env.status_of(iid)
        self.assertEqual((st["state"], st["url"]), ("failed", None))
        self.assertIn("limit for issues", st["reason"])
        self.assertFiled(project())          # control: one publish left
        iid = self.env.drop(project(project="calc2"))
        self.env.run(self.gh)
        self.assertEqual(self.env.status_of(iid)["state"], "failed")
        self.assertEqual(json.load(open(os.path.join(self.env.state, "state.json")))["prs"], P.CAPS["publish"][1])

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
    def test_github_errors_retry_then_failed(self):
        cases = [(P.HTTPRefused(401, "Bad credentials"), "GitHub refused the credential (HTTP 401)"),
                 (P.HTTPRefused(403, "Resource not accessible"), "GitHub refused the request (HTTP 403)"),
                 (P.HTTPRefused(403, "API rate limit exceeded"), "GitHub rate limit (HTTP 403)"),
                 (P.HTTPRefused(502), "GitHub server error (HTTP 502)"),
                 (P.GitHubError("GitHub did not answer within 10 s"), "GitHub did not answer within 10 s"),
                 (RuntimeError("boom"), "an internal error (RuntimeError)")]
        for k, (err, cause) in enumerate(cases):
            gh = FakeGitHub(fail=err)
            iid = self.env.drop(issue(title="Retry case number %d here" % k))
            for n in range(1, 5):
                run = self.env.run(gh)
                st = self.env.status_of(iid)
                self.assertEqual(st["state"], "queued", cause)
                self.assertEqual(st["reason"], "retrying: %s (attempt %d of 5)" % (cause, n))
                self.assertTrue(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))
                self.assertIsNotNone(run.next_due)
            self.env.run(gh)
            st = self.env.status_of(iid)
            self.assertEqual(st, {"id": iid, "state": "failed", "url": None,
                                  "reason": "%s after 5 attempts" % cause})
            self.assertFalse(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))
        state = json.load(open(os.path.join(self.env.state, "state.json")))
        self.assertEqual((state["issues"], state["pending"], state["attempts"]), (0, {}, {}))  # slots returned

    def test_retry_then_filed_once(self):
        gh = FakeGitHub(fail=P.HTTPRefused(502), fail_count=2)
        iid = self.env.drop(issue())
        self.env.run(gh)
        self.env.run(gh)
        self.assertEqual(self.env.status_of(iid)["reason"], "retrying: GitHub server error (HTTP 502) (attempt 2 of 5)")
        self.env.run(gh)
        self.assertEqual(self.env.status_of(iid)["state"], "filed")
        self.assertEqual(len(gh.posts("issues")), 1)
        self.assertEqual(json.load(open(os.path.join(self.env.state, "state.json")))["issues"], 1)

    def test_backoff_waits(self):
        gh = FakeGitHub(fail=P.HTTPRefused(502))
        iid = self.env.drop(issue())
        a = self.env.args()
        a.backoff = (100.0, 100.0, 100.0, 100.0)
        P.process(a, gh)
        n = len(gh.calls)
        run = P.process(a, gh)  # not due yet: no request, item kept
        self.assertEqual(len(gh.calls), n)
        self.assertGreater(run.next_due, P.time.time() + 90)
        self.assertTrue(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))

    def test_guard_refusal_fails_item(self):
        gh = FakeGitHub(fail=P.GuardRefused("refusing"))
        iid = self.env.drop(issue())
        self.env.run(gh)
        self.assertEqual(self.env.status_of(iid)["state"], "failed")

    # request guard
    def test_guard(self):
        bad = [("POST", "/repos/aien-dev/aienos.com/issues", {}),
               ("POST", "/repos/aien-dev/omega-projects/git/refs", {"ref": "refs/heads/main", "sha": "x"}),
               ("PATCH", "/repos/aien-dev/omega-projects/git/refs/heads/main", {}),
               ("PUT", "/repos/aien-dev/omega-projects/pulls/1/merge", {}),
               ("DELETE", "/repos/aien-dev/omega-projects/git/refs/heads/main", None),
               ("POST", "/repos/aien-dev/omega-projects/pulls", {"base": "dev", "head": "p/%s/calc" % ADA}),
               ("POST", "/repos/aien-dev/omega-projects/pulls", {"base": "main", "head": "submissions/x"}),
               ("POST", "/repos/aien-dev/omega-projects/git/refs", {"ref": "refs/heads/submissions/x", "sha": "x"}),
               ("PATCH", "/repos/aien-dev/omega-projects/git/refs/heads/u/" + ADA, {"sha": "x", "force": True}),
               ("PATCH", "/repos/aien-dev/omega-projects/git/refs/heads/u/" + ADA, {"sha": "x"}),
               ("PATCH", "/repos/aien-dev/omega-projects/pulls/1", {"state": "open"}),
               ("PATCH", "/repos/aien-dev/omega-projects/pulls/1", {"state": "closed", "base": "dev"}),
               ("POST", "/repos/aien-dev/omega-projects/git/trees",
                {"base_tree": "d" * 40, "tree": [{"path": ".github/workflows/x.yml", "sha": "x"}]}),
               ("POST", "/repos/aien-dev/omega-projects/git/trees",
                {"base_tree": "d" * 40, "tree": [{"path": "projects/%s/calc/a/b.om" % ADA, "sha": "x"}]}),
               ("POST", "/repos/aien-dev/omega-projects/git/trees",
                {"base_tree": "d" * 40, "tree": [{"path": "index.html", "sha": "x"}]}),
               ("GET", "/repos/aien-dev/omega-projects/../../user", None),
               ("GET", "/user", None)]
        for m, p, b in bad:
            with self.assertRaises(PermissionError, msg="%s %s %s" % (m, p, b)):
                P._check_request(m, p, b)
        # controls
        for ref in ("refs/heads/u/" + ADA, "refs/heads/p/%s/calc" % ADA):
            P._check_request("POST", "/repos/aien-dev/omega-projects/git/refs", {"ref": ref, "sha": "x"})
        P._check_request("PATCH", "/repos/aien-dev/omega-projects/git/refs/heads/u/" + ADA, {"sha": "x", "force": False})
        P._check_request("PATCH", "/repos/aien-dev/omega-projects/pulls/1", {"state": "closed"})
        P._check_request("POST", "/repos/aien-dev/omega-projects/git/trees",
                         {"base_tree": "d" * 40, "tree": [{"path": "projects/%s/calc/index.html" % ADA, "sha": "x"}]})
        P._check_request("POST", "/repos/aien-dev/omega-projects/pulls", {"base": "main", "head": "p/%s/calc" % ADA})
        P._check_request("POST", "/repos/aien-dev/omega-projects/issues", {})

    # activation
    def test_not_activated_queues_and_keeps(self):
        iid = self.env.drop(issue())
        bad = self.env.drop(project(project="../x"))
        self.assertEqual(self.env.run(None).waiting, 1)
        st = self.env.status_of(iid)
        self.assertEqual((st["state"], st["reason"], st["url"]), ("queued", P.AWAITING, None))
        self.assertTrue(os.path.exists(os.path.join(self.env.spool_in, iid + ".json")))
        self.assertEqual(os.stat(os.path.join(self.env.status, iid + ".json")).st_mode & 0o777, 0o640)
        self.assertEqual(self.env.status_of(bad)["state"], "rejected")  # validation still runs
        # control: once activated the same item is filed and leaves the spool
        self.assertEqual(self.env.run(self.gh).waiting, 0)
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



class StubEndToEnd(unittest.TestCase):
    """Runs the pusher program as a subprocess against the local stub (no GitHub)."""

    def setUp(self):
        self.env = Env()
        self.stub = Stub(hold=12.0).start()
        self.token = os.path.join(self.env.root, "omega-projects-token")
        with open(self.token, "w") as f:
            f.write(FAKE_PAT + "\n")

    def tearDown(self):
        self.stub.shutdown()
        self.stub.server_close()
        self.env.cleanup()

    def pusher(self, *extra, timeout=60):
        cmd = [sys.executable, os.path.join(HERE, "omega_projects_pusher.py"),
               "--spool-in", self.env.spool_in, "--spool-status", self.env.status,
               "--state-dir", self.env.state, "--areas", self.env.areas, "--spool-module-dir", GW,
               "--api-base", self.stub.base, "--backoff", "0.2,0.2,0.2,0.2", "--idle-delay", "0"] + list(extra)
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env={"PATH": "/usr/bin:/bin"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn(FAKE_PAT, r.stdout + r.stderr)
        return r.stderr

    def api_log(self):
        return [e for e in self.stub.log if e["path"].startswith("/repos/")]

    def test_one_issue_one_pr_then_dedupe(self):
        it = issue(body="Please add sin. Reach me at a.b@example.org or +1 555 010 9999.")
        pr = project(files=[{"name": "index.html", "content": "<p>calc</p>"},
                            {"name": "README.md", "content": "Calculator by a.b@example.org."}])
        ids = [self.env.drop(it), self.env.drop(pr)]
        self.pusher("--token-file", self.token)
        self.assertEqual(len(self.stub.issues), 1)
        self.assertEqual(len(self.stub.pulls), 1)
        self.assertEqual(sorted(self.stub.refs), ["refs/heads/main", "refs/heads/p/%s/calc" % ADA])
        self.assertEqual(self.stub.refs["refs/heads/main"], "c" * 40)  # main untouched
        iss = self.stub.issues[0]
        self.assertEqual(iss["labels"], ["from-aien", "area:omega-mv"])
        self.assertTrue(iss["title"].startswith("[feature] "))
        self.assertNotIn("a.b@example.org", iss["body"])
        self.assertNotIn("9999", iss["body"])
        self.assertIn(P.FOOTER, iss["body"])
        self.assertEqual(self.stub.pulls[0]["base"], {"ref": "main"})
        self.assertEqual({e["method"] for e in self.stub.log}, {"GET", "POST"})
        readme = self.stub.files_at("p/%s/calc" % ADA)["projects/%s/calc/README.md" % ADA]
        self.assertNotIn("a.b@example.org", readme)
        self.assertTrue(all(e["auth_ok"] for e in self.stub.log))
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
        other = Stub().start()
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

    def test_api_failures_end_failed_in_one_run(self):
        for mode, cause in (("401", "GitHub refused the credential (HTTP 401)"),
                            ("403", "GitHub refused the request (HTTP 403)"),
                            ("500", "GitHub server error (HTTP 500)"),
                            ("502", "GitHub server error (HTTP 502)")):
            self.stub.set_mode(mode, -1)
            before = len(self.api_log())
            iid = self.env.drop(issue(title="Failure mode %s probe item" % mode))
            err = self.pusher("--token-file", self.token)
            st = self.env.status_of(iid)
            self.assertEqual(st, {"id": iid, "state": "failed", "url": None,
                                  "reason": "%s after 5 attempts" % cause}, err)
            self.assertEqual(len(self.api_log()) - before, 5)  # one request per attempt, then stop
            self.assertEqual(os.listdir(self.env.spool_in), [])
        self.assertEqual(self.stub.issues, [])
        # control: the same kind of item with the API healthy is filed
        self.stub.set_mode("ok")
        iid = self.env.drop(issue(title="Healthy API control item"))
        self.pusher("--token-file", self.token)
        self.assertEqual(self.env.status_of(iid)["state"], "filed")
        self.assertEqual(len(self.stub.issues), 1)

    def test_timeout_then_filed_once(self):
        self.stub.set_mode("timeout", 1)
        iid = self.env.drop(issue())
        err = self.pusher("--token-file", self.token)
        self.assertIn("GitHub did not answer within 10 s; next in", err)
        self.assertEqual(self.env.status_of(iid)["state"], "filed")
        self.assertEqual(len(self.stub.issues), 1)

    def test_server_error_then_filed_once(self):
        self.stub.set_mode("502", 2)
        iid = self.env.drop(issue())
        err = self.pusher("--token-file", self.token)
        self.assertIn("attempt 2 of 5 failed: GitHub server error (HTTP 502)", err)
        self.assertEqual(self.env.status_of(iid)["state"], "filed")
        self.assertEqual(len(self.stub.issues), 1)


@unittest.skipUnless(os.environ.get("LIVE") == "1", "LIVE=1 checks aien-dev/omega-projects settings")
class Live(unittest.TestCase):
    """Operator setup checks with the normal session credential, never the pusher's."""

    def test_live_ruleset_settings(self):
        r = subprocess.run(["gh", "api", "repos/%s/rules/branches/main" % REPO], capture_output=True, text=True)
        rules = {x["type"]: x.get("parameters") for x in json.loads(r.stdout)}
        self.assertEqual(set(rules), {"deletion", "non_fast_forward", "pull_request"})
        self.assertEqual(rules["pull_request"]["required_approving_review_count"], 1)
        self.assertTrue(rules["pull_request"]["require_code_owner_review"])
        # Actions run only the project checks: no actions usable, read-only token, cannot approve.
        r = subprocess.run(["gh", "api", "repos/%s/actions/permissions" % REPO], capture_output=True, text=True)
        self.assertEqual({k: json.loads(r.stdout)[k] for k in ("enabled", "allowed_actions")},
                         {"enabled": True, "allowed_actions": "local_only"})
        r = subprocess.run(["gh", "api", "repos/%s/actions/permissions/workflow" % REPO], capture_output=True, text=True)
        self.assertEqual(json.loads(r.stdout), {"default_workflow_permissions": "read",
                                                "can_approve_pull_request_reviews": False})

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
            br = "u/push-control-%d" % os.getpid()
            r2 = run("git", "push", "origin", "HEAD:refs/heads/" + br)
            print("LIVE push %s rc=%d" % (br, r2.returncode), file=sys.stderr)
            self.assertEqual(r2.returncode, 0, r2.stderr[-300:])
            r3 = run("git", "push", "origin", "--delete", br)
            self.assertEqual(r3.returncode, 0, r3.stderr[-300:])
        finally:
            shutil.rmtree(d)


if __name__ == "__main__":
    unittest.main(verbosity=2)
