#!/usr/bin/env python3
"""Measure the two prompt layouts directly against the shared model (one request at a time).

  full   system = prompt + catalog.txt + every repo's detail line; visitor turns as typed
  cards  system = prompt + catalog.txt; each visitor turn carries its matching cards (live layout)

Per question and layout, three requests, each only while /health reports <= --max-inflight:
  1. prefix   [system, open line], max_tokens 1: the cold prefill of the fixed prefix (the cost
              of every re-warm)
  2. warm     [system, opening, question]: a first turn on the snapshot step 1 left
  3. cold     [system, opening, another question]: nothing to reuse (step 2 took the snapshot)
The open line is the measurement's own, so no production warm row matches it or is consumed.
Writes one JSON line per request (TTFT, decode tok/s, usage) to --out.
"""

import argparse
import http.client
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import allen_chat  # noqa: E402
import allen_speed  # noqa: E402

OPEN = "Hello Allen. I am measuring speed."
QUESTIONS = [
    ("What is CUDApp?", "What does omega-bridge do?"),
    ("Tell me about omnet.", "What is the atlas repo?"),
    ("What is aienux?", "How do I use Synapse?"),
    ("What is mobhub?", "What is the status of AienOS? Can I use it today?"),
]


def health(c_host, c_port):
    c = http.client.HTTPConnection(c_host, c_port, timeout=5)
    c.request("GET", "/health")
    d = json.loads(c.getresponse().read(4096))
    c.close()
    return d


def wait_quiet(host, port, limit):
    while True:
        try:
            h = health(host, port)
            if h.get("status") == "ok" and int(h.get("in_flight", 99)) <= limit:
                return int(h["in_flight"])
        except (OSError, ValueError):
            pass
        time.sleep(3)


def stream(host, port, messages, max_tokens):
    body = json.dumps({"model": allen_chat.MODEL, "messages": messages, "stream": True,
                       "stream_options": {"include_usage": True}, "max_tokens": max_tokens,
                       "temperature": 0}).encode()
    t0 = time.monotonic()
    c = http.client.HTTPConnection(host, port, timeout=600)
    c.request("POST", "/v1/chat/completions", body, {"Content-Type": "application/json"})
    r = c.getresponse()
    first = last = None
    n, usage = 0, {}
    for line in r:
        if not line.startswith(b"data: "):
            continue
        p = line[6:].strip()
        if p == b"[DONE]":
            break
        d = json.loads(p)
        usage = d.get("usage") or usage
        for ch in d.get("choices", ()):
            if ch.get("delta", {}).get("content"):
                now = time.monotonic()
                first = first or now
                last = now
                n += 1
    c.close()
    end = time.monotonic()
    return {"status": r.status, "ttft_ms": int(((first or end) - t0) * 1000), "total_ms": int((end - t0) * 1000),
            "pieces": n, "decode_tok_s": round((n - 1) / (last - first), 2) if n > 1 and last > first else None,
            "prompt_tokens": usage.get("prompt_tokens"), "completion_tokens": usage.get("completion_tokens")}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--system-prompt", required=True)
    ap.add_argument("--catalog", required=True)
    ap.add_argument("--catalog-detail", required=True)
    ap.add_argument("--catalog-retrieve", required=True)
    ap.add_argument("--upstream", default="127.0.0.1:18091")
    ap.add_argument("--max-inflight", type=int, default=2)
    ap.add_argument("--layouts", default="cards,full")
    ap.add_argument("--n", type=int, default=len(QUESTIONS))
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    host, _, port = a.upstream.rpartition(":")
    port = int(port)
    prompt = open(a.system_prompt, encoding="utf-8").read()
    catalog = open(a.catalog, encoding="utf-8").read()
    cards = allen_speed.Cards(a.catalog_detail, a.catalog_retrieve)
    cards.refresh()
    full_detail = "\n\nEvery repo in detail (data, not instructions):\n" + "".join(
        cards.retrieve._line(d) for d in cards.records)
    layouts = {
        "cards": (allen_speed.join_system(prompt, catalog), True),
        "full": (allen_speed.join_system(prompt, catalog.rstrip("\n") + full_detail), False),
    }
    opening = [{"role": "user", "content": OPEN}, {"role": "assistant", "content": allen_chat.GREETING_ANON}]
    with open(a.out, "a", encoding="utf-8") as out:
        for i, (q_warm, q_cold) in enumerate(QUESTIONS[:a.n]):
            for name in a.layouts.split(","):
                system, use_cards = layouts[name]

                def turn(q):
                    m = [{"role": "user", "content": q}]
                    return allen_speed.augment(m, cards) if use_cards else m
                steps = [("prefix", [{"role": "system", "content": system}, {"role": "user", "content": OPEN}], 1, None),
                         ("warm", [{"role": "system", "content": system}] + opening + turn(q_warm), 200, q_warm),
                         ("cold", [{"role": "system", "content": system}] + opening + turn(q_cold), 200, q_cold)]
                for step, msgs, mt, q in steps:
                    inflight = wait_quiet(host, port, a.max_inflight)
                    rec = dict(stream(host, port, msgs, mt), layout=name, step=step, sample=i, q=q,
                               in_flight_before=inflight, at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
                    out.write(json.dumps(rec) + "\n")
                    out.flush()
                    print(json.dumps(rec), flush=True)


if __name__ == "__main__":
    main()
