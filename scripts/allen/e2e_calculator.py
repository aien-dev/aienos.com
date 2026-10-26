#!/usr/bin/env python3
"""End-to-end build loop against the real aien-dev/omega-projects (test-labelled).

A throwaway loopback gateway (this directory's code, a test visitor secret, no model calls)
compiles calc.om to WebAssembly through /allen/api/omega, gives the test visitor a home,
takes two saves and one publish; the real pusher program files each one with the
fine-grained OMEGA_PROJECTS_TOKEN from the atlas vault (never printed). Expected: two
commits on u/<handle>, one pull request to main left open for Drake, main untouched.

Usage: e2e_calculator.py --omc PATH --irwasm PATH --system-prompt PATH --calc FILE --out DIR
"""

import argparse
import base64
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from test_gateway import Gateway  # noqa: E402

KEY = "e2" * 16  # the test visitor's Allen key


def page(wasm_b64):
    return """<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Calculator</title>
<style>body{font:18px/1.5 system-ui,sans-serif;max-width:28rem;margin:2rem auto;padding:0 1rem}
input,select,button{font:inherit;padding:.3rem}</style>
<h1>Calculator</h1>
<p>Whole numbers, computed by Omega compiled to WebAssembly.</p>
<p><input id=a type=number value=12> <select id=op><option value=plus>+</option><option value=minus>-</option>
<option value=times>&times;</option><option value=quot>&divide;</option></select> <input id=b type=number value=3>
<button id=go>=</button> <output id=out></output></p>
<script>
const bytes = Uint8Array.from(atob("%s"), c => c.charCodeAt(0));
WebAssembly.instantiate(bytes, {}).then(({ instance }) => {
  const run = () => {
    const f = instance.exports[document.getElementById('op').value];
    const a = BigInt(document.getElementById('a').value || 0), b = BigInt(document.getElementById('b').value || 0);
    document.getElementById('out').textContent = String(f(a, b));
  };
  document.getElementById('go').addEventListener('click', run);
  run();
});
</script>
</html>
""" % wasm_b64


README = """# Calculator

A four-function calculator for whole numbers. The arithmetic is written in Omega (`calc.om`),
compiled to WebAssembly by Allen's Omega compiler, and run by `index.html` in the browser with
nothing loaded from elsewhere.

Who it helps: anyone who wants a tiny, readable example of an Omega program running on a web
page, and students checking quick sums. Division by zero returns 0.

Built with Allen at https://www.aienos.com/allen/ (end-to-end test of the build loop).
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--omc", required=True)
    ap.add_argument("--irwasm", required=True)
    ap.add_argument("--system-prompt", required=True)
    ap.add_argument("--calc", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--pusher-dir", default="/tmp/allenchat/pusher")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    log = open(os.path.join(a.out, "e2e-calculator.log"), "a")

    def say(*parts):
        line = " ".join(str(p) for p in parts)
        print(line, flush=True)
        log.write(line + "\n")
        log.flush()

    g = Gateway(a, env={"ALLEN_VISITOR_SECRET": "e2e-test-visitor-secret-" + "0" * 20})
    state = os.path.join(a.out, "pusher-state")
    os.makedirs(state, mode=0o700, exist_ok=True)
    H = {"X-Allen-Key": KEY}

    def post(path, body, ip):
        c, r = g.post(path, body, headers=H, ip=ip)
        return r.status, json.loads(r.read())

    def pusher():
        cmd = ("exec python3 %s --spool-in %s --spool-status %s --state-dir %s --areas /nonexistent "
               "--spool-module-dir %s --token-file <(printf %%s \"$OMEGA_PROJECTS_TOKEN\") --test-label "
               "--idle-delay 0" % (os.path.join(a.pusher_dir, "omega_projects_pusher.py"), g.dirs["in"],
                                   g.dirs["status"], state, HERE))
        r = subprocess.run(["atlas-vault", "run", "OMEGA_PROJECTS_TOKEN", "--", "bash", "-c", cmd],
                           capture_output=True, text=True, timeout=180)
        say("pusher rc=%d" % r.returncode, r.stderr.strip().replace("\n", " | ")[-800:])

    def status(iid):
        c, r = g.get("/allen/api/status/" + iid)
        return json.loads(r.read())

    try:
        src = open(a.calc).read()
        st, j = post("/allen/api/omega", {"source": src}, "198.51.100.60")
        wasm = j.get("wasm") or j.get("wasm_b64") or ""
        say("compile calc.om:", st, "wasm bytes:", len(base64.b64decode(wasm)) if wasm else 0,
            "keys:", sorted(j))
        if st != 200 or not wasm:
            return 1
        st, h = post("/allen/api/projects/handle", {"name": "calc-tester"}, "198.51.100.61")
        handle = h["handle"]
        say("handle:", st, handle)
        base = {"handle": handle, "project": "calculator", "title": "Calculator",
                "description": "A four-function calculator: Omega arithmetic compiled to WebAssembly."}
        v1 = dict(base, files=[{"name": "calc.om", "content": src}, {"name": "README.md", "content": README}])
        st, s1 = post("/allen/api/projects/save", v1, "198.51.100.62")
        say("save 1:", st, s1)
        pusher()
        r1 = status(s1["id"])
        say("save 1 status:", r1)
        v2 = dict(base, files=v1["files"] + [{"name": "index.html", "content": page(wasm)}])
        st, s2 = post("/allen/api/projects/save", v2, "198.51.100.63")
        say("save 2:", st, s2)
        pusher()
        r2 = status(s2["id"])
        say("save 2 status:", r2)
        st, p = post("/allen/api/projects/publish", v2, "198.51.100.64")
        say("publish:", st, p)
        pusher()
        rp = status(p["id"])
        say("publish status:", rp)
        with open(os.path.join(a.out, "e2e-calculator.json"), "w") as f:
            json.dump({"handle": handle, "branch": "u/" + handle, "save1": r1, "save2": r2, "publish": rp,
                       "view": s2.get("view")}, f, indent=2)
        ok = r1["state"] == r2["state"] == rp["state"] == "filed"
        say("RESULT", "PASS" if ok else "FAIL")
        return 0 if ok else 1
    finally:
        g.close()


if __name__ == "__main__":
    sys.exit(main())
