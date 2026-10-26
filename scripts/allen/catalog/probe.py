#!/usr/bin/env python3
"""Measure prompt size/TTFT and run the catalog eval against the live model (sequential).

  probe.py measure SYSTEM_FILE [CATALOG_FILE]     one request, prints JSON
  probe.py eval SYSTEM_FILE CATALOG_FILE OUT_JSON  15 questions, one at a time
"""

import http.client
import json
import re
import sys
import time

HOST, PORT, MODEL = "127.0.0.1", 18091, "qwen3.6-35b-a3b-fp8"
OPEN = "Hello Aien."
GREETING = "Hello, I am Allen, Drake's AI. Are you a human or an agent?"  # site-allen allen.js greeting()


def health():
    c = http.client.HTTPConnection(HOST, PORT, timeout=10)
    c.request("GET", "/health")
    h = json.loads(c.getresponse().read())
    c.close()
    return h


def wait_quiet():
    for _ in range(60):
        h = health()
        if h.get("status") == "ok" and h.get("in_flight", 99) <= 2:
            return h
        time.sleep(5)
    sys.exit("model busy (in_flight > 2) for 5 minutes; not sending")


def system_text(sysfile, catfile):
    s = open(sysfile, encoding="utf-8").read()
    if catfile:  # same join as allen_chat.App.system_text
        s = s.rstrip("\n") + "\n\n" + open(catfile, encoding="utf-8").read()
    return s


def chat(system, user, max_tokens, temperature=0.3, history=()):
    msgs = [{"role": "system", "content": system}] + list(history) + [{"role": "user", "content": user}]
    body = json.dumps({"model": MODEL, "stream": True, "stream_options": {"include_usage": True},
                       "max_tokens": max_tokens, "temperature": temperature,
                       "messages": msgs}).encode()
    c = http.client.HTTPConnection(HOST, PORT, timeout=300)
    t0 = time.monotonic()
    c.request("POST", "/v1/chat/completions", body, {"Content-Type": "application/json"})
    r = c.getresponse()
    if r.status != 200:
        raise SystemExit("HTTP %d: %s" % (r.status, r.read()[:300]))
    ttft, out, usage = None, [], None
    for raw in r:
        line = raw.decode("utf-8", "replace").strip()
        if not line.startswith("data:"):
            continue
        data = line[5:].strip()
        if data == "[DONE]":
            break
        ev = json.loads(data)
        if ev.get("usage"):
            usage = ev["usage"]
        for ch in ev.get("choices") or []:
            piece = (ch.get("delta") or {}).get("content")
            if piece:
                if ttft is None:
                    ttft = time.monotonic() - t0
                out.append(piece)
    total = time.monotonic() - t0
    c.close()
    return {"ttft_s": round(ttft or -1, 3), "total_s": round(total, 3), "usage": usage, "text": "".join(out)}


QUESTIONS = [
    ("features", "What can I do on this site? What features does Drake have?"),
    ("repos", "What repositories exist? Give me the list of public projects."),
    ("omega-what", "What is Omega for? What are its use cases?"),
    ("exists-translate", "Is there something to translate my website without a cloud API?"),
    ("exists-boot", "Do you have anything for verified booting on ARM?"),
    ("exists-gpu", "Is there a way to run CUDA programs without NVIDIA's toolkit?"),
    ("k8s", "Do you have a Kubernetes operator?"),
    ("no-mobile", "Is there an iOS app for this?"),
    ("oss-k8s", "What does the open-source community offer for running containers at scale?"),
    ("oss-vector", "I need a vector database. What open-source options exist?"),
    ("omega-web", "Can I use Omega to build a web app?"),
    ("omega-gpu", "Can I use Omega to program a GPU?"),
    ("omega-http", "Can I write an HTTP server in Omega?"),
    ("where-evidence", "Where can I see the evidence and benchmark results?"),
    ("agents", "I am building a coding agent. Where should it start reading?"),
]


def main():
    if len(sys.argv) >= 3 and sys.argv[1] == "measure":
        sysfile = sys.argv[2]
        cat = sys.argv[3] if len(sys.argv) > 3 else None
        h = wait_quiet()
        res = chat(system_text(sysfile, cat), "Hi Allen, in one sentence: what is Omega?", 40)
        res["in_flight_before"] = h.get("in_flight")
        res["catalog"] = cat
        print(json.dumps(res))
        return
    if len(sys.argv) >= 5 and sys.argv[1] == "opening":
        # steps: "warm" = [system, user OPEN] max_tokens 1; anything else is a visitor
        # question after the fixed opening exchange [user OPEN, assistant GREETING].
        system = system_text(sys.argv[2], sys.argv[3])
        opening = [{"role": "user", "content": OPEN}, {"role": "assistant", "content": GREETING}]
        for step in sys.argv[4:]:
            h = wait_quiet()
            if step == "warm":
                res = chat(system, OPEN, 1)
            else:
                res = chat(system, step, 40, history=opening)
            u = res["usage"] or {}
            print(json.dumps({"step": step, "in_flight_before": h.get("in_flight"), "ttft_s": res["ttft_s"],
                              "total_s": res["total_s"], "prompt_tokens": u.get("prompt_tokens"),
                              "text": res["text"][:120]}))
        return
    if len(sys.argv) == 5 and sys.argv[1] == "eval":
        system = system_text(sys.argv[2], sys.argv[3])
        results = []
        for key, q in QUESTIONS:
            h = wait_quiet()
            res = chat(system, q, 500)
            res.update(key=key, question=q, in_flight_before=h.get("in_flight"))
            results.append(res)
            print("%s ttft=%.2fs prompt=%s" % (key, res["ttft_s"], (res["usage"] or {}).get("prompt_tokens")),
                  file=sys.stderr)
        json.dump(results, open(sys.argv[4], "w"), indent=1)
        return
    sys.exit(__doc__)


if __name__ == "__main__":
    main()
