"""POST /allen/api/math: one expression to the Omega math engine, answered as glass pane events.

The /aien/ page sends {"expression": "...", "title": "..."}; this module forwards it as the
math_compute tool to the math-only allen-science instance (127.0.0.1:18114, started with
--tools math_compute, scripts/allen/science/allen-science-math-fg). That instance refuses every
other science tool, so the public route reaches math and nothing else. It runs the pure-Omega
aien-math engine in its sandbox and answers with at most three v1 pane
events (pane.open, pane.update, pane.close) plus a one-line result. No model call, no network
beyond loopback, nothing stored or logged. The page renders the events with AllenPanes, which
checks every event again against the pane contract before showing it.
"""
import http.client
import json
import socket

SCIENCE = ("127.0.0.1", 18114)
MAX_EXPRESSION = 4096
MAX_TITLE = 200
MAX_REPLY = 262144
PANE_EVENTS = frozenset(("pane.open", "pane.update", "pane.close"))
PER_MINUTE = 12


def init(app, buckets):
    app.math_rate = buckets(PER_MINUTE, 60 / PER_MINUTE)


def _bad(text):
    return not isinstance(text, str) or "\x00" in text


def _compute(expression, title):
    body = json.dumps({"tool": "math_compute", "arguments": {"title": title, "expression": expression}},
                      separators=(",", ":"), ensure_ascii=False).encode()
    conn = http.client.HTTPConnection(*SCIENCE, timeout=20)
    try:
        conn.request("POST", "/compute", body, {"Content-Type": "application/json", "Connection": "close"})
        resp = conn.getresponse()
        raw = resp.read(MAX_REPLY + 1)
    finally:
        conn.close()
    if resp.status != 200 or len(raw) > MAX_REPLY:
        return None
    out = json.loads(raw)
    if type(out) is not dict or set(out) != {"events", "result"} or type(out["events"]) is not list:
        return None
    if len(out["events"]) > 3 or not isinstance(out["result"], str) or len(out["result"]) > 8000:
        return None
    for item in out["events"]:
        if type(item) is not dict or set(item) != {"event", "data"} or item["event"] not in PANE_EVENTS \
                or type(item["data"]) is not dict:
            return None
    return out


def post(h):
    app = h.app
    app.stats.add("math_requests")
    req = h.read_json()
    if req is None:
        return
    if not isinstance(req, dict) or set(req) - {"expression", "title"}:
        return h.refuse(400, "bad_request", "Send {expression, title}.")
    expression = req.get("expression")
    title = req.get("title", "Math")
    if _bad(expression) or not expression.strip() or _bad(title):
        return h.refuse(400, "bad_request", "Write an expression, for example Integrate[x^2,{x,0,1}].")
    if len(expression) > MAX_EXPRESSION or len(title) > MAX_TITLE:
        return h.refuse(413, "too_long", "An expression holds at most %d characters." % MAX_EXPRESSION)
    retry = app.math_rate.take(app.client_key(h))
    if retry:
        return h.refuse(429, "rate", "That is %d expressions this minute. Try again in %d s." % (PER_MINUTE, retry),
                        retry)
    try:
        out = _compute(expression.strip(), title.strip() or "Math")
    except (OSError, socket.timeout, ValueError, http.client.HTTPException):
        out = None
    if out is None:
        app.stats.add("math_unavailable")
        return h.refuse(503, "unavailable", "The math engine did not answer. Try again in a minute.", 60)
    h.send_json(200, out)
