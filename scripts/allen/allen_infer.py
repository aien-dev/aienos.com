#!/usr/bin/env python3
"""allen_infer: the /aien/ speed test (gateway routes infer, infer-verify) and the same benchmark as a CLI.

Gateway routes (imported by allen_chat.py; CONTRACT in aien-dev/aienos.com scripts/allen/README.md
"Speed test"):

    GET  /allen/api/infer-info    presets, limits and current load (no model call)
    POST /allen/api/infer         one run, streamed as server-sent events with gateway-measured timings
    POST /allen/api/infer-verify  checks the HMAC on a shared result (no storage)

A run is a plain chat completion on the shared model: no Aien persona, temperature 0, and a
one-line system message `Speed test run <8 hex>.` first, so the engine can never continue a
cached row and prefill is always measured from scratch. Modes: single (one stream), concurrent
(2-4 streams at once, aggregate tokens/s) and sweep (separate requests at prompt sizes doubling
from 512 tokens, so a sweep can stop between steps).

Protection for the shared server (127.0.0.1:18091, 8 batch rows, also serving Aien's chat):
- tests together hold at most SLOTS (4) rows, and start only while the server reports
  in_flight + rows <= SERVER_ROOM (6), so two rows stay free for chat;
- a heavy test (concurrent, any sweep, or a prompt estimated over HEAVY_TOKENS) runs alone and
  starts only while no chat turn runs or waits in this gateway; a chat arriving mid-test makes it
  yield (sweep: before the next step; concurrent: its streams are closed at once). The reason: a
  prefill on this engine slows every generating row 6-9x (AienPage LATENCY.md), so row
  reservation alone does not protect chat;
- one run per visitor at a time, RUNS_PER_MIN runs and HEAVY_PER_10MIN heavy runs per visitor,
  a FIFO queue of QUEUE_DEPTH with visible position, caps on prompt size and max_tokens, and
  timeouts scaled to the work.

Numbers are measured at the gateway as chunks arrive; nothing is smoothed. Each result carries
host load1 and the server's in_flight at that moment, and an HMAC so a shared link can be checked.

CLI (the "run it on your own GPU" command; same presets and the same measure() code):

    python3 scripts/allen/allen_infer.py bench http://127.0.0.1:8000 --model <name> [--preset all]
        [--streams 4] [--sweep 8192] [--api-key KEY]

Python standard library only. Host dependency and .om successor: DEPENDENCIES.md (allen-gw.om).
"""

import argparse
import base64
import collections
import hashlib
import hmac
import http.client
import json
import math
import os
import queue
import re
import secrets
import socket
import sys
import threading
import time
from urllib.parse import urlsplit

MODEL = "qwen3.6-35b-a3b-fp8"
SLOTS = 4                 # batch rows all tests together may hold
SERVER_ROOM = 6           # start only while server in_flight + rows <= this (8 rows, 2 kept for chat)
HEAVY_TOKENS = 4096       # an estimated prompt above this is heavy
MAX_PROMPT_BYTES = 48 * 1024
MAX_PROMPT_TOKENS = 32768
MAX_TOKENS = 4096
MAX_TOKENS_CONCURRENT = 1024
MAX_STREAMS = 4
SWEEP_MAXES = (4096, 8192, 16384, 32768)
SWEEP_OUT = 8             # output tokens per sweep step
RUNS_PER_MIN = 6
HEAVY_PER_10MIN = 3
QUEUE_DEPTH = 8
QUEUE_WAIT = 180.0        # seconds a run may wait in line
CHUNK_MAX = 1 << 20

PRESETS = collections.OrderedDict()
PRESETS["short"] = {"max_tokens": 64, "prompt": "In two sentences, explain why the sky is blue."}
PRESETS["medium"] = {"max_tokens": 256, "prompt": (
    "Explain how a CPU cache hierarchy works: what L1, L2 and L3 caches are, why they exist, "
    "what a cache line is, and what happens on a cache miss. Use plain language and short paragraphs.")}


def _long_prompt():
    rows = []
    for i in range(1, 41):
        rows.append("Record %03d: station %s measured %d.%d degrees at %02d:%02d, humidity %d percent, wind %d km/h from bearing %03d."
                    % (i, "ABCDEFGHJK"[i % 10] + "ABCDEFGHJK"[(i * 7) % 10], (i * 37) % 40, (i * 3) % 10,
                       (i * 5) % 24, (i * 11) % 60, 30 + (i * 13) % 60, (i * 17) % 50, (i * 29) % 360))
    return ("Below is a log of weather records.\n\n" + "\n".join(rows) +
            "\n\nWrite a detailed report on this log: the temperature range, which stations were warmest and "
            "coldest, how humidity and wind varied through the day, and anything unusual. Be thorough.")


PRESETS["long"] = {"max_tokens": 1024, "prompt": _long_prompt()}

FILLER = "Line %05d: the quick brown fox jumps over the lazy dog near the river bank.\n"
FILLER_TOKENS = 23  # per filler line, measured on the live server: 25 lines -> 615 prompt tokens,
FILLER_BASE = 40    # 204 lines -> 4732 (nonce line, chat template and question are the 40); pages show the real count
SWEEP_ASK = "\nHow many lines are above? Reply with one number."


def sweep_sizes(top):
    out, n = [], 512
    while n <= top:
        out.append(n)
        n *= 2
    return out


def sweep_prompt(target):
    return "".join(FILLER % i for i in range(1, max(1, (target - FILLER_BASE) // FILLER_TOKENS) + 1)) + SWEEP_ASK


WORD = re.compile(r"[A-Za-z]+|[0-9]|[^\sA-Za-z0-9]")


def est_tokens(text):
    """A conservative token estimate without the tokenizer: ASCII letter runs count one per 4
    letters, every digit, punctuation mark and non-ASCII character counts one."""
    n = 0
    for m in WORD.finditer(text):
        w = m.group(0)
        n += (len(w) + 3) // 4 if w[0].isascii() and w[0].isalpha() else 1
    return n


def load1():
    try:
        with open("/proc/loadavg") as f:
            return float(f.read().split()[0])
    except (OSError, ValueError, IndexError):
        return None


def measure(host, port, path, headers, model, messages, max_tokens, on_tok, cancel, ttft_s, total_s,
            hold=None):
    """One streamed chat completion, timed as chunks arrive. on_tok(t_monotonic, text) per content
    chunk. cancel: threading.Event; set it (and call hold[0].close()) to stop. Returns a dict with
    monotonic times: sent, first, last, end; prompt_tokens, completion_tokens, chunks, finish, error."""
    body = json.dumps({"model": model, "messages": messages, "stream": True,
                       "stream_options": {"include_usage": True}, "max_tokens": max_tokens,
                       "temperature": 0}).encode()
    out = {"sent": None, "first": None, "last": None, "end": None, "prompt_tokens": None,
           "completion_tokens": None, "chunks": 0, "finish": None, "error": None}
    c = http.client.HTTPConnection(host, port, timeout=ttft_s)
    if hold is not None:
        hold.append(c)
    try:
        out["sent"] = time.monotonic()
        c.request("POST", path, body, dict(headers, **{"Content-Type": "application/json"}))
        r = c.getresponse()
        if r.status != 200:
            r.read(65536)
            out["error"] = "busy" if r.status == 503 else "upstream_%d" % r.status
            return out
        deadline = out["sent"] + total_s
        while True:
            if cancel.is_set():
                out["error"] = "cancelled"
                return out
            if time.monotonic() > deadline:
                out["error"] = "timeout"
                return out
            line = r.readline(CHUNK_MAX)
            if not line:
                break
            if not line.startswith(b"data: "):
                continue
            payload = line[6:].strip()
            if payload == b"[DONE]":
                break
            chunk = json.loads(payload)
            u = chunk.get("usage")
            if u:
                out["prompt_tokens"] = u.get("prompt_tokens")
                out["completion_tokens"] = u.get("completion_tokens")
            for ch in chunk.get("choices", ()):
                if ch.get("finish_reason"):
                    out["finish"] = ch["finish_reason"]
                text = (ch.get("delta") or {}).get("content")
                if text:
                    now = time.monotonic()
                    if out["first"] is None:
                        out["first"] = now
                        if c.sock is not None:
                            c.sock.settimeout(30)
                    out["last"] = now
                    out["chunks"] += 1
                    on_tok(now, text)
    except (OSError, ValueError, http.client.HTTPException) as e:
        out["error"] = "cancelled" if cancel.is_set() else ("timeout" if isinstance(e, (socket.timeout, TimeoutError))
                                                             else "upstream")
    finally:
        out["end"] = time.monotonic()
        c.close()
    return out


def rate(n, seconds):
    return round(n / seconds, 1) if n is not None and seconds and seconds > 0 else None


def stats(m, t0):
    """The stream summary the page shows, in ms relative to t0 (the run's start)."""
    ms = lambda t: int(round((t - t0) * 1000)) if t is not None else None  # noqa: E731
    comp = m["completion_tokens"] if m["completion_tokens"] is not None else m["chunks"]
    ttft = (m["first"] - m["sent"]) if m["first"] is not None else None
    dec = (m["last"] - m["first"]) if m["first"] is not None and m["last"] is not None else None
    return {"prompt_tokens": m["prompt_tokens"], "completion_tokens": comp,
            "ttft_ms": int(round(ttft * 1000)) if ttft is not None else None,
            "prefill_tps": rate(m["prompt_tokens"], ttft),
            "decode_tps": rate(comp - 1, dec) if comp and comp > 1 else None,
            "total_ms": int(round((m["end"] - m["sent"]) * 1000)) if m["sent"] else None,
            "start_ms": ms(m["sent"]), "first_ms": ms(m["first"]), "last_ms": ms(m["last"]),
            "finish": m["finish"] or m["error"] or "stop", "error": m["error"]}


def run_messages(prompt, nonce):
    return [{"role": "system", "content": "Speed test run %s." % nonce}, {"role": "user", "content": prompt}]


# ---------------------------------------------------------------- gateway state

class Window:
    def __init__(self, n, span):
        self.n, self.span, self.lock, self.hits = n, span, threading.Lock(), {}

    def take(self, key):
        """0 when admitted, else seconds until the oldest hit leaves the window."""
        now = time.monotonic()
        with self.lock:
            q = [t for t in self.hits.get(key, ()) if now - t < self.span]
            if len(q) >= self.n:
                self.hits[key] = q
                return int(self.span - (now - q[0])) + 1
            q.append(now)
            self.hits[key] = q
            return 0

    def sweep(self):
        now = time.monotonic()
        with self.lock:
            for k in [k for k, q in self.hits.items() if not q or now - q[-1] > self.span]:
                del self.hits[k]


class Lab:
    """FIFO admission for speed tests. A run needs `rows` rows; heavy runs go alone and only
    while no chat turn runs or waits."""

    def __init__(self, app):
        self.app = app
        self.cond = threading.Condition()
        self.used = 0
        self.heavy = False
        self.waiting = []
        self.ticket = 0
        self.active = set()  # visitor keys with a run queued or running
        self.runs = Window(RUNS_PER_MIN, 60)
        self.heavies = Window(HEAVY_PER_10MIN, 600)
        secret = os.environ.get("ALLEN_VISITOR_SECRET", "").strip().encode()
        self.key = hmac.new(secret, b"aien-infer-sig-v1", hashlib.sha256).digest() if len(secret) >= 32 \
            else secrets.token_bytes(32)

    def chat_busy(self):
        g = self.app.gate
        with g.cond:
            return g.running > 0 or bool(g.waiting)

    def server(self, fresh=False):
        up = self.app.up
        if fresh:
            up.health_at = 0.0  # admission decides on a read taken now, not the 0.5 s cache
        up.admit()
        return up.health_ok, up.health_inflight

    def enter(self, vkey):
        with self.cond:
            if vkey in self.active:
                return None, "one"
            if len(self.waiting) >= QUEUE_DEPTH:
                return None, "full"
            self.ticket += 1
            self.waiting.append(self.ticket)
            self.active.add(vkey)
            return self.ticket, None

    def why_waiting(self, t, rows, heavy):
        """None when the run may start now (and it is then started), else the reason."""
        with self.cond:
            if not self.waiting or self.waiting[0] != t:
                return "line"
        ok, inflight = self.server(fresh=True)  # only the head of the line reads /health
        with self.cond:
            if not self.waiting or self.waiting[0] != t:
                return "line"
            if self.heavy or self.used + rows > SLOTS:
                return "slots"
            if not ok or inflight + rows > SERVER_ROOM:
                return "server"
            if heavy and (self.used or self.chat_busy()):
                return "chat" if self.chat_busy() else "slots"
            self.waiting.pop(0)
            self.used += rows
            self.heavy = heavy
            self.cond.notify_all()
            return None

    def position(self, t):
        with self.cond:
            return self.waiting.index(t) + 1 if t in self.waiting else 0

    def leave(self, t, vkey, rows=0, started=False):
        with self.cond:
            if t in self.waiting:
                self.waiting.remove(t)
            if started:
                self.used -= rows
                self.heavy = False
            self.active.discard(vkey)
            self.cond.notify_all()

    def wait(self, s):
        with self.cond:
            self.cond.wait(s)

    def sign(self, result):
        return hmac.new(self.key, canon(result), hashlib.sha256).hexdigest()


def canon(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def init(app):
    app.lab = Lab(app)


def limits():
    return {"slots": SLOTS, "max_tokens": MAX_TOKENS, "max_tokens_concurrent": MAX_TOKENS_CONCURRENT,
            "max_streams": MAX_STREAMS, "max_prompt_bytes": MAX_PROMPT_BYTES,
            "max_prompt_tokens": MAX_PROMPT_TOKENS, "heavy_tokens": HEAVY_TOKENS,
            "sweep_max": list(SWEEP_MAXES), "sweep_out": SWEEP_OUT, "runs_per_min": RUNS_PER_MIN,
            "heavy_per_10min": HEAVY_PER_10MIN, "queue_depth": QUEUE_DEPTH}


def get(h):
    lab = h.app.lab
    ok, inflight = lab.server()
    with lab.cond:
        st = {"used": lab.used, "heavy": lab.heavy, "queue": len(lab.waiting)}
    h.send_json(200, {"model": MODEL, "presets": PRESETS, "limits": limits(), "tests_in_flight": st["used"],
                      "heavy_running": st["heavy"], "queue": st["queue"], "chat_active": lab.chat_busy(),
                      "server_ok": ok, "server_in_flight": inflight, "load1": load1()})


def verify(h):
    req = h.read_json()
    if req is None:
        return
    res, sig = (req.get("result"), req.get("sig")) if isinstance(req, dict) else (None, None)
    if not isinstance(res, dict) or not isinstance(sig, str) or not re.match(r"^[0-9a-f]{64}$", sig):
        return h.refuse(400, "bad_request", "Send {result, sig}.")
    h.send_json(200, {"ok": hmac.compare_digest(h.app.lab.sign(res), sig)})


def parse(req):
    """(plan, error message). plan: mode, prompt(s), max_tokens, streams, sizes, heavy, preset."""
    if not isinstance(req, dict):
        return None, "Send a JSON object."
    mode = req.get("mode", "single")
    if mode not in ("single", "concurrent", "sweep"):
        return None, "mode is single, concurrent or sweep."
    plan = {"mode": mode, "preset": None, "streams": 1, "sizes": None}
    if mode == "sweep":
        top = req.get("sweep_max", 8192)
        if top not in SWEEP_MAXES:
            return None, "sweep_max is one of %s." % ", ".join(map(str, SWEEP_MAXES))
        plan.update(sizes=sweep_sizes(top), max_tokens=SWEEP_OUT, requested=SWEEP_OUT, heavy=True, prompt=None)
        return plan, None
    preset = req.get("preset")
    if preset is not None:
        if preset not in PRESETS:
            return None, "preset is short, medium or long."
        prompt, mt = PRESETS[preset]["prompt"], PRESETS[preset]["max_tokens"]
        asked = mt
        plan["preset"] = preset
    else:
        prompt = req.get("prompt")
        if not isinstance(prompt, str) or not prompt.strip():
            return None, "Write a prompt or pick a preset."
        if len(prompt.encode()) > MAX_PROMPT_BYTES or est_tokens(prompt) > MAX_PROMPT_TOKENS:
            return None, "too_long"
        asked = req.get("max_tokens", 256)
        if not isinstance(asked, int) or isinstance(asked, bool) or asked < 1:
            return None, "max_tokens is a whole number from 1."
        mt = asked
    cap = MAX_TOKENS if mode == "single" else MAX_TOKENS_CONCURRENT
    if mode == "concurrent":
        n = req.get("streams", 4)
        if not isinstance(n, int) or isinstance(n, bool) or not 2 <= n <= MAX_STREAMS:
            return None, "streams is 2 to %d." % MAX_STREAMS
        plan["streams"] = n
    plan.update(prompt=prompt, max_tokens=min(mt, cap), requested=asked,
                heavy=mode == "concurrent" or est_tokens(prompt) > HEAVY_TOKENS)
    return plan, None


def run(h):
    app = h.app
    lab = app.lab
    app.stats.add("infer_requests")
    req = h.read_json()
    if req is None:
        return
    plan, problem = parse(req)
    if problem == "too_long":
        return h.refuse(413, "too_long", "That prompt is over the limit (48 KiB, about 32k tokens).")
    if problem:
        return h.refuse(400, "bad_request", problem)
    vkey = app.client_key(h)
    retry = lab.runs.take(vkey)
    if retry:
        return h.refuse(429, "rate", "You have run %d tests this minute. Try again in %d s." % (RUNS_PER_MIN, retry), retry)
    if plan["heavy"]:
        retry = lab.heavies.take(vkey)
        if retry:
            return h.refuse(429, "rate", "Heavy tests (parallel, sweep, long prompts): %d per 10 minutes each. "
                            "Try again in %d s." % (HEAVY_PER_10MIN, retry), retry)
    ticket, why = lab.enter(vkey)
    if ticket is None:
        if why == "one":
            return h.refuse(409, "one_at_a_time", "You already have a test running. Wait for it to finish.", 5)
        return h.refuse(503, "busy", "The test line is full. Try again in 30 s.", 30)
    rows = plan["streams"]
    started = False
    h.sse_headers("infer")
    try:
        t0 = time.monotonic()
        last_pos, last_beat, last_why = -1, 0.0, None
        while True:
            why = lab.why_waiting(ticket, rows, plan["heavy"])
            if why is None:
                started = True
                break
            now = time.monotonic()
            if now - t0 > QUEUE_WAIT:
                app.stats.add("infer_queue_timeout")
                h.sse("error", {"code": "busy", "message": "The server stayed busy for %d s. Try again in a minute."
                                % QUEUE_WAIT, "retry_after": 60})
                return
            pos = lab.position(ticket)
            if pos != last_pos or why != last_why or now - last_beat > 3:
                with lab.cond:
                    used = lab.used
                h.sse("queue", {"position": pos, "reason": why, "tests_in_flight": used, "slots": SLOTS,
                                "waited_ms": int((now - t0) * 1000)})
                last_pos, last_why, last_beat = pos, why, now
            lab.wait(0.25)
        execute(h, plan, int((time.monotonic() - t0) * 1000))
    except (BrokenPipeError, ConnectionResetError, socket.timeout, TimeoutError):
        app.stats.add("client_gone")
    except Exception as e:  # noqa: BLE001  a gateway bug ends the stream with an error, never silently
        app.stats.add("infer_internal_error")
        print("allen-chat: internal error in infer: %s" % type(e).__name__, flush=True)
        try:
            h.sse("error", {"code": "upstream", "message": "The test failed inside the gateway. Try again."})
        except OSError:
            pass
    finally:
        lab.leave(ticket, vkey, rows, started)


def execute(h, plan, queued_ms):
    app = h.app
    lab = app.lab
    up = app.up
    run_id = secrets.token_hex(4)
    t0 = time.monotonic()
    ok, inflight = lab.server()
    with lab.cond:
        used = lab.used
    start = {"run": run_id, "mode": plan["mode"], "preset": plan["preset"], "model": MODEL,
             "max_tokens": plan["max_tokens"], "max_tokens_requested": plan["requested"],
             "streams": plan["streams"], "queued_ms": queued_ms, "server_in_flight": inflight,
             "tests_in_flight": used, "load1": load1(), "heavy": plan["heavy"]}
    if plan["sizes"]:
        start["steps"] = plan["sizes"]
    h.sse("start", start)
    events = queue.Queue()
    cancel = threading.Event()
    holds = []
    outs = []
    yielded = False
    gone = False

    def worker(i, prompt, nonce, ttft_s, total_s, target=None):
        hold = []
        holds.append(hold)
        m = measure(up.host, up.port, "/v1/chat/completions", up.headers, MODEL, run_messages(prompt, nonce),
                    plan["max_tokens"], lambda t, s: events.put(("tok", i, t, s)), cancel, ttft_s, total_s, hold)
        events.put(("end", i, m, target))

    def pump(n):
        """Forward events until n streams ended. Watches for a chat arriving (heavy runs yield)."""
        nonlocal yielded, gone
        left = n
        beat = time.monotonic()
        while left:
            try:
                ev = events.get(timeout=0.1)
            except queue.Empty:
                ev = None
            now = time.monotonic()
            if ev is None and not gone and now - beat > 5:
                beat = now  # keeps the proxy and the page informed during a long prefill
                try:
                    h.sse("beat", {"t": int(round((now - t0) * 1000))})
                except OSError:
                    gone = True
                    cancel.set()
            elif ev is not None:
                beat = now
            if plan["mode"] == "concurrent" and not yielded and lab.chat_busy():
                yielded = True
                cancel.set()
                for hold in holds:
                    for c in hold:
                        try:
                            if c.sock is not None:
                                c.sock.shutdown(socket.SHUT_RDWR)
                        except OSError:
                            pass
            if ev is None:
                continue
            if ev[0] == "tok":
                if not gone:
                    try:
                        h.sse("tok", {"i": ev[1], "t": int(round((ev[2] - t0) * 1000)), "s": ev[3]})
                    except OSError:
                        gone = True
                        cancel.set()
                continue
            left -= 1
            s = stats(ev[2], t0)
            s["i"] = ev[1]
            if ev[3] is not None:
                s["target_tokens"] = ev[3]
            _, s["server_in_flight"] = lab.server()
            s["load1"] = load1()
            outs.append(s)
            if not gone:
                try:
                    h.sse("stream", s)
                except OSError:
                    gone = True
                    cancel.set()

    if plan["mode"] == "sweep":
        for i, size in enumerate(plan["sizes"]):
            if i and lab.chat_busy():
                yielded = True
                break
            if gone or (outs and outs[-1]["error"]):
                break
            t = threading.Thread(target=worker, daemon=True,
                                 args=(i, sweep_prompt(size), secrets.token_hex(4), 60 + size / 50, 120 + size / 50, size))
            t.start()
            pump(1)
    else:
        est = est_tokens(plan["prompt"])
        for i in range(plan["streams"]):
            threading.Thread(target=worker, daemon=True,
                             args=(i, plan["prompt"], secrets.token_hex(4), 60 + est / 50,
                                   120 + est / 50 + plan["max_tokens"] / 4)).start()
        pump(plan["streams"])
    if gone:
        app.stats.add("client_gone")
        return
    outs.sort(key=lambda s: s["i"])
    _, inflight_end = lab.server()
    result = {"v": 1, "run": run_id, "mode": plan["mode"], "preset": plan["preset"], "model": MODEL,
              "utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "max_tokens": plan["max_tokens"],
              "streams": plan["streams"], "server_in_flight": start["server_in_flight"],
              "server_in_flight_end": inflight_end, "tests_in_flight": start["tests_in_flight"],
              "load1": start["load1"], "load1_end": load1(), "yielded": yielded,
              "streams_out": outs, "wall_ms": int(round((time.monotonic() - t0) * 1000)), "aggregate_tps": None}
    if plan["mode"] == "concurrent":
        firsts = [s["start_ms"] for s in outs if s["start_ms"] is not None]
        lasts = [s["last_ms"] for s in outs if s["last_ms"] is not None]
        total = sum(s["completion_tokens"] or 0 for s in outs)
        if firsts and lasts and max(lasts) > min(firsts):
            result["aggregate_tps"] = round(total * 1000.0 / (max(lasts) - min(firsts)), 1)
    if yielded:
        app.stats.add("infer_yielded")
        h.sse("yielded", {"reason": "chat", "completed_steps": sum(1 for s in outs if not s["error"])})
    app.stats.add("infer_completed")
    h.sse("done", {"result": result, "sig": lab.sign(result)})


# ---------------------------------------------------------------- CLI: the same benchmark on your GPU

def bench(argv):
    ap = argparse.ArgumentParser(prog="allen_infer.py bench",
                                 description="The aienos.com/aien/ speed test against your own OpenAI-compatible server.")
    ap.add_argument("url", help="server base URL, for example http://127.0.0.1:8000 (POSTs <url>/v1/chat/completions)")
    ap.add_argument("--model", required=True)
    ap.add_argument("--preset", default="all", choices=["all"] + list(PRESETS))
    ap.add_argument("--streams", type=int, default=0, help="also run the parallel test with this many streams (2-8)")
    ap.add_argument("--sweep", type=int, default=0, help="also run the context sweep up to this many prompt tokens")
    ap.add_argument("--api-key", default=os.environ.get("OPENAI_API_KEY", ""))
    ap.add_argument("--json", action="store_true", help="print one JSON line per run instead of a table")
    a = ap.parse_args(argv)
    u = urlsplit(a.url.rstrip("/"))
    if u.scheme != "http" or not u.hostname:
        sys.exit("url must be http://host:port (put a TLS server behind a local proxy)")
    path = (u.path or "") + "/v1/chat/completions"
    headers = {"Authorization": "Bearer " + a.api_key} if a.api_key else {}
    port = u.port or 80

    def one(prompt, mt, n=1):
        t0 = time.monotonic()
        res, ths, cancel = [None] * n, [], threading.Event()
        for i in range(n):
            def go(i=i):
                res[i] = measure(u.hostname, port, path, headers, a.model, run_messages(prompt, secrets.token_hex(4)),
                                 mt, lambda t, s: None, cancel, 600, 3600)
            ths.append(threading.Thread(target=go))
            ths[-1].start()
        for t in ths:
            t.join()
        return [stats(m, t0) for m in res]

    def show(label, ss, agg=None):
        if a.json:
            print(json.dumps({"run": label, "load1": load1(), "streams": ss, "aggregate_tps": agg}))
            return
        for s in ss:
            print("%-14s in %6s out %5s  ttft %7s ms  prefill %8s tok/s  decode %7s tok/s  total %7s ms  %s"
                  % (label, s["prompt_tokens"], s["completion_tokens"], s["ttft_ms"], s["prefill_tps"],
                     s["decode_tps"], s["total_ms"], s["finish"]))
        if agg is not None:
            print("%-14s aggregate %s tok/s over %d streams" % (label, agg, len(ss)))

    print("load1 %s, model %s at %s" % (load1(), a.model, a.url))
    for name in (PRESETS if a.preset == "all" else [a.preset]):
        show(name, one(PRESETS[name]["prompt"], PRESETS[name]["max_tokens"]))
    if a.streams:
        n = max(2, min(8, a.streams))
        ss = one(PRESETS["medium"]["prompt"], PRESETS["medium"]["max_tokens"], n)
        lasts, starts = [s["last_ms"] for s in ss if s["last_ms"]], [s["start_ms"] for s in ss if s["start_ms"] is not None]
        agg = round(sum(s["completion_tokens"] or 0 for s in ss) * 1000.0 / (max(lasts) - min(starts)), 1) \
            if lasts and starts else None
        show("parallel-%d" % n, ss, agg)
    if a.sweep:
        for size in sweep_sizes(a.sweep):
            show("sweep-%d" % size, one(sweep_prompt(size), SWEEP_OUT))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bench":
        bench(sys.argv[2:])
    else:
        sys.exit("usage: allen_infer.py bench URL --model NAME [--preset all|short|medium|long] [--streams N] [--sweep TOKENS]")
