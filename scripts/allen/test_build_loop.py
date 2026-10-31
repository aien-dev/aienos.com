#!/usr/bin/env python3
"""Build-loop tests for allen-chat + omega-projects-pusher, each guard with a control.

A throwaway gateway on loopback (never the production unit, never the model: no chat is
sent) writes save/publish items into a temporary spool; the real pusher program files them
against github_stub.py (127.0.0.1, fake token, in-memory git store). Checked: homes and
ownership (two visitors cannot write each other's folders; anonymous claim codes), file
rules, dedupe, the status route, two saves = two commits on u/<handle>, publish = one PR
to main and main untouched, Drake's console list with decline, refresh, and the gallery.

Usage: test_build_loop.py --omc PATH --irwasm PATH --system-prompt PATH [--pusher-dir DIR]
"""

import argparse
import hashlib
import hmac
import json
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import test_gateway as T  # noqa: E402
from test_gateway import Console, Gateway, TOKEN, check, cortex_stub  # noqa: E402

SECRET = "b" * 40
KEY_A, KEY_B = "a1" * 16, "b2" * 16
README = "# Calculator\n\nAdds, subtracts, multiplies and divides. It helps anyone who needs quick sums.\n"


def calc(handle, html="<!doctype html><p>calc</p>", **kw):
    it = {"handle": handle, "project": "calculator", "title": "Calculator",
          "description": "A small calculator built with Allen.",
          "files": [{"name": "index.html", "content": html}, {"name": "README.md", "content": README}]}
    it.update(kw)
    return it


def js(r):
    raw = r.read()
    try:
        return json.loads(raw)
    except ValueError:
        return {"raw": raw[:200].decode("utf-8", "replace")}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--omc", required=True)
    ap.add_argument("--irwasm", required=True)
    ap.add_argument("--system-prompt", required=True)
    ap.add_argument("--pusher-dir", default="/tmp/allenchat/pusher")
    a = ap.parse_args()
    sys.path.insert(0, a.pusher_dir)
    from github_stub import FAKE_PAT, Stub

    cx = cortex_stub()
    stub = Stub().start()
    g = Gateway(a, "--memory-url", "http://127.0.0.1:%d" % cx.server_address[1], console=True,
                env={"CORTEX_VISITORS_TOKEN": "t" * 40, "ALLEN_VISITOR_SECRET": SECRET,
                     "ALLEN_CONSOLE_TOKEN_SHA256": hashlib.sha256(TOKEN.encode()).hexdigest()})
    state = os.path.join(g.dirs["console"], "pusher-state")
    tokfile = os.path.join(g.dirs["console"], "token")
    with open(tokfile, "w") as f:
        f.write(FAKE_PAT)
    A = {"X-Allen-Key": KEY_A}
    B = {"X-Allen-Key": KEY_B}

    def pusher():
        r = subprocess.run([sys.executable, os.path.join(a.pusher_dir, "omega_projects_pusher.py"),
                            "--spool-in", g.dirs["in"], "--spool-status", g.dirs["status"], "--state-dir", state,
                            "--areas", "/nonexistent", "--spool-module-dir", HERE, "--api-base", stub.base,
                            "--token-file", tokfile, "--backoff", "0.1", "--idle-delay", "0"],
                           capture_output=True, text=True, timeout=60, env={"PATH": "/usr/bin:/bin"})
        return r.returncode, r.stderr

    try:
        # Homes.
        c, r = g.post("/allen/api/projects/handle", {"name": "ada"}, headers=A)
        ha = js(r)
        c, r = g.post("/allen/api/projects/handle", {"name": "ada"}, headers=A, ip="203.0.113.2")
        ha2 = js(r)
        c, r = g.post("/allen/api/projects/handle", {"name": "ada"}, headers=B, ip="203.0.113.3")
        hb = js(r)
        check("remembered handle is name + 8 hex, stable per key", r.status == 200 and ha["handle"] == ha2["handle"]
              and ha["handle"].startswith("ada-") and len(ha["handle"]) == 12, "%s %s" % (ha, ha2))
        check("same name, other key: other handle", hb["handle"] != ha["handle"], "%s %s" % (ha["handle"], hb["handle"]))
        check("handle remembered in the visitor Cortex", ha.get("remembered") is True, "%s" % ha)
        c, r = g.get("/allen/api/projects/me", headers=A)
        me = js(r)
        check("projects/me recalls the handle from Cortex", me.get("handle") == ha["handle"], "%s" % me)
        c, r = g.get("/allen/api/projects/me", headers={"X-Allen-Key": "c3" * 16})
        check("projects/me control: unknown key has no handle", js(r).get("handle") is None, "")
        c, r = g.post("/allen/api/projects/handle", {"name": "bob@example.com"}, headers=A, ip="203.0.113.4")
        check("an email is never a name", r.status == 422, "%d %s" % (r.status, js(r)))
        c, r = g.post("/allen/api/projects/handle", {}, ip="203.0.113.5")
        an = js(r)
        check("anonymous: one-time handle with a claim code", r.status == 200 and an["handle"].startswith("anon-")
              and len(an.get("claim", "")) == 24, "%s" % {k: an.get(k) for k in ("handle", "anonymous")})
        claim = {"X-Allen-Claim": an["claim"]}
        c, r = g.get("/allen/api/projects/me", headers=claim)
        check("claim code gives back the anonymous handle", js(r).get("handle") == an["handle"], "")

        # Ownership: two visitors cannot write each other's folders.
        A_h, B_h, N_h = ha["handle"], hb["handle"], an["handle"]
        cases = [("key B saves into A's home", calc(A_h), B, 403),
                 ("no key saves into A's home", calc(A_h), {}, 403),
                 ("anonymous mode with A's key saves into A's home", calc(A_h),
                  dict(A, **{"X-Allen-Anonymous": "1"}), 403),
                 ("key A saves into an anon home without its claim", calc(N_h), A, 403),
                 ("wrong claim saves into the anon home", calc(N_h), {"X-Allen-Claim": "0" * 24}, 403),
                 ("A forges a handle with A's name and another tag", calc("ada-00000000"), A, 403),
                 ("A's handle with traversal in the project", calc(A_h, project="../x"), A, 422),
                 ("A publishes a workflow file", calc(A_h, files=[{"name": "README.md", "content": README},
                                                                  {"name": "ci.yml", "content": "on: push"}]), A, 422),
                 ("A publishes a page that loads a remote script",
                  calc(A_h, html='<script src="https://evil.example/x.js"></script>'), A, 422)]
        for i, (name, body, hdr, want) in enumerate(cases):
            c, r = g.post("/allen/api/projects/save", body, headers=hdr, ip="198.51.100.%d" % (10 + i))
            j = js(r)
            check(name + " -> refused", r.status == want, "%d %s" % (r.status, j.get("message", j)))
        check("refusals wrote nothing to the spool", g.files("in") == [], "%s" % g.files("in"))

        # Controls, then two saves = two commits.
        c, r = g.post("/allen/api/projects/save", calc(A_h), headers=A, ip="198.51.100.40")
        s1 = js(r)
        check("A saves into A's home (control)", r.status == 200 and s1["branch"] == "u/" + A_h, "%s" % s1)
        c, r = g.post("/allen/api/projects/save", calc(A_h), headers=A, ip="198.51.100.40")
        check("the same save again is a duplicate", js(r).get("duplicate") is True, "")
        c, r = g.post("/allen/api/projects/save", calc(N_h), headers=claim, ip="198.51.100.41")
        s3 = js(r)
        check("anon saves into its home with its claim (control)", r.status == 200, "%s" % s3)
        rc, err = pusher()
        st1 = json.loads(g.get("/allen/api/status/" + s1["id"])[1].read())
        check("pusher files the save; status shows the commit", rc == 0 and st1["state"] == "filed"
              and "/commit/" in (st1.get("url") or ""), "%s %s" % (st1, err[-300:]))
        c, r = g.post("/allen/api/projects/save", calc(A_h, html="<!doctype html><p>calc 2</p>"), headers=A,
                      ip="198.51.100.40")
        s2 = js(r)
        pusher()
        st2 = json.loads(g.get("/allen/api/status/" + s2["id"])[1].read())
        head = stub.refs["refs/heads/u/" + A_h]
        c1 = st1["url"].rsplit("/", 1)[1]
        check("second save is a second commit on u/<handle>", st2["url"].endswith(head)
              and stub.commits[head]["parents"] == [c1], "%s parents=%s" % (head, stub.commits[head]["parents"]))
        files_a = stub.files_at("u/" + A_h)
        files_n = stub.files_at("u/" + N_h)
        check("each branch holds only its own home", all(p.startswith(("projects/%s/" % A_h, "README.md",
              "projects/README.md")) for p in files_a) and all(p.startswith(("projects/%s/" % N_h, "README.md",
              "projects/README.md")) for p in files_n), "%s | %s" % (sorted(files_a), sorted(files_n)))
        meta = json.loads(files_a["projects/%s/calculator/meta.json" % A_h])
        check("meta.json carries no personal data", set(meta) == {"v", "handle", "project", "title", "description",
              "saved", "item", "built_with", "files"} and KEY_A not in json.dumps(meta), "%s" % sorted(meta))

        # Publish: one PR to main, main untouched.
        c, r = g.post("/allen/api/projects/publish", calc(A_h, html="<!doctype html><p>calc 2</p>"), headers=A,
                      ip="198.51.100.40")
        p1 = js(r)
        c, r = g.post("/allen/api/projects/publish", calc(A_h), headers=B, ip="198.51.100.42")
        check("key B cannot publish A's project", r.status == 403, "%d" % r.status)
        pusher()
        stp = json.loads(g.get("/allen/api/status/" + p1["id"])[1].read())
        check("publish opens one pull request to main", stp["state"] == "filed" and "/pull/" in stp["url"]
              and len(stub.pulls) == 1 and stub.pulls[0]["base"] == {"ref": "main"}
              and stub.pulls[0]["head"] == {"ref": "p/%s/calculator" % A_h}, "%s" % stp)
        check("main untouched by save and publish", stub.refs["refs/heads/main"] == "c" * 40,
              stub.refs["refs/heads/main"])

        # Drake's console: the list, decline, refresh.
        cons = Console(g)
        cons.login()
        st, _, page = cons.req("GET", "/")
        n = stub.pulls[0]["number"]
        check("console lists the publish request with approve and decline", st == 200 and "#%d" % n in page
              and "Review and approve on GitHub" in page and "/projects/decline" in page, "%d" % st)
        st, _, _ = cons.req("POST", "/projects/decline", {"pr": str(n), "reason": "x"})
        check("decline without CSRF is refused", st == 403, "%d" % st)
        st, _, _ = cons.post("/projects/decline", pr=str(n), reason="Please say who it helps.")
        pusher()
        check("decline closes the pull request with a comment", st == 303 and stub.pulls[0]["state"] == "closed"
              and "Declined by Drake" in stub.comments[0]["body"], "%d %s" % (st, stub.pulls[0]["state"]))
        _, _, page = cons.req("GET", "/")
        check("declined request leaves the list", "#%d" % n not in page, "")

        # Gallery: Drake merges a re-publish; refresh shows it.
        c, r = g.post("/allen/api/projects/publish", calc(A_h, title="Calculator for everyone"), headers=A,
                      ip="198.51.100.43")
        pusher()
        check("re-publish after decline opens a new request", len(stub.pulls) == 2
              and stub.pulls[1]["state"] == "open", "%d" % len(stub.pulls))
        stub.refs["refs/heads/main"] = stub.refs["refs/heads/p/%s/calculator" % A_h]  # Drake merged
        stub.pulls[1]["state"] = "closed"
        c, r = g.get("/allen/api/gallery")
        check("gallery empty before refresh (control)", js(r)["gallery"] == [], "")
        cons.post("/projects/refresh")
        pusher()
        c, r = g.get("/allen/api/gallery")
        gal = js(r)["gallery"]
        check("gallery lists the merged project", [(x["handle"], x["title"]) for x in gal]
              == [(A_h, "Calculator for everyone")], "%s" % gal)
    finally:
        g.close()
        stub.shutdown()
        cx.shutdown()
    bad = [n for n, ok in T.results if not ok]
    print("\n%d/%d passed%s" % (len(T.results) - len(bad), len(T.results), (": FAILED " + ", ".join(bad)) if bad else ""))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
