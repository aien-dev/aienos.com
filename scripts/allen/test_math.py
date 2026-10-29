"""In-process checks for allen_math (POST /allen/api/math) against a stub science service."""
import json
import os
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import allen_math  # noqa: E402

REPLY = {"events": [{"event": "pane.open", "data": {"v": 1, "id": "p_1", "kind": "math", "title": "t"}},
                    {"event": "pane.update", "data": {"v": 1, "id": "p_1", "revision": 1, "content": {"text": "1/3"},
                                                      "provenance": {"status": "computed"}}}],
         "result": "aien-math exact output (not a proof of correctness): 1/3"}
seen = []
mode = {"reply": REPLY}


class Science(BaseHTTPRequestHandler):
    def do_POST(self):
        seen.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
        body = json.dumps(mode["reply"]).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


class Buckets:
    def __init__(self, capacity, refill_s):
        self.left = capacity

    def take(self, key):
        self.left -= 1
        return 0 if self.left >= 0 else 5

    def sweep(self):
        pass


class Stats:
    def add(self, _):
        pass


class App:
    stats = Stats()

    def client_key(self, _):
        return "k"


class H:
    def __init__(self, app, req):
        self.app, self.req, self.out = app, req, None

    def read_json(self):
        return self.req

    def refuse(self, code, err, msg, retry=None):
        self.out = (code, {"error": err, "message": msg})

    def send_json(self, code, obj, extra=None):
        self.out = (code, obj)


def main():
    srv = ThreadingHTTPServer(("127.0.0.1", 0), Science)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    allen_math.SCIENCE = ("127.0.0.1", srv.server_address[1])
    app = App()
    allen_math.init(app, Buckets)
    fails = 0

    def check(name, ok):
        nonlocal fails
        print(("PASS " if ok else "FAIL ") + name)
        fails += not ok

    h = H(app, {"expression": " Integrate[x^2,{x,0,1}] ", "title": "Area"})
    allen_math.post(h)
    check("computes through math_compute", h.out == (200, REPLY) and seen[-1] == {
        "tool": "math_compute", "arguments": {"title": "Area", "expression": "Integrate[x^2,{x,0,1}]"}})
    for bad in ({"expression": ""}, {"expression": 3}, {"expression": "1", "x": 1}, ["1"], {"expression": "a\x00"}):
        h = H(app, bad)
        allen_math.post(h)
        check("refuses %r" % (bad,), h.out[0] == 400)
    h = H(app, {"expression": "1+" * 3000})
    allen_math.post(h)
    check("refuses an over-long expression", h.out[0] == 413)
    mode["reply"] = {"events": [{"event": "shell", "data": {}}], "result": ""}
    h = H(app, {"expression": "1+1"})
    allen_math.post(h)
    check("refuses an unapproved event from the service", h.out[0] == 503)
    mode["reply"] = REPLY
    for _ in range(allen_math.PER_MINUTE):
        allen_math.post(H(app, {"expression": "1+1"}))
    h = H(app, {"expression": "1+1"})
    allen_math.post(h)
    check("rate limits per visitor", h.out[0] == 429)
    srv.shutdown()
    allen_math.SCIENCE = ("127.0.0.1", 9)
    app2 = App()
    allen_math.init(app2, Buckets)
    h = H(app2, {"expression": "1+1"})
    allen_math.post(h)
    check("a missing service is 503, not a crash", h.out[0] == 503)
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
