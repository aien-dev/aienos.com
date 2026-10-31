#!/usr/bin/env python3
"""Production end-to-end test of visitor memory through https://www.aienos.com/allen/api/.

A test visitor introduces itself (consent), chats, and comes back in a new
conversation with its key; controls: no key, another visitor's key, a claimed
name without the key; then forget-me and a check that the visitor is a stranger
again. Prints a latency comparison (memory on vs off). Uses test-labelled data
and deletes it at the end.
"""

import json
import secrets
import ssl
import sys
import time
import http.client

HOST = "www.aienos.com"
results = []
MARK = "kelpboat" + secrets.token_hex(2)  # unusual project word to recognise in replies


def check(name, ok, detail):
    results.append((name, ok))
    print("%s %s: %s" % ("PASS" if ok else "FAIL", name, detail), flush=True)


def call(path, obj, key=None, conv=None):
    c = http.client.HTTPSConnection(HOST, 443, timeout=240, context=ssl.create_default_context())
    h = {"Content-Type": "application/json"}
    if key:
        h["X-Allen-Key"] = key
    if conv:
        h["X-Allen-Conversation"] = conv
    t0 = time.monotonic()
    c.request("POST", "/allen/api/" + path, json.dumps(obj), h)
    r = c.getresponse()
    if path != "chat" or r.status != 200:
        return r.status, json.loads(r.read() or b"{}"), None
    ev, text, start, first, done = None, [], None, None, None
    for line in r:
        line = line.rstrip(b"\n")
        if line.startswith(b"event: "):
            ev = line[7:].decode()
        elif line.startswith(b"data: "):
            d = json.loads(line[6:])
            if ev == "start":
                start = d
            elif ev is None and d:
                first = first or time.monotonic()
                text.append(d)
            elif ev in ("done", "error"):
                done = {ev: d}
            ev = None
    return r.status, {"text": "".join(text), "start": start, "end": done}, int(((first or t0) - t0) * 1000)


def chat(msg, key=None, conv=None):
    time.sleep(22)  # stay inside the per-address chat bucket (one per 20 s after the burst)
    st, out, ttft = call("chat", {"messages": [{"role": "user", "content": msg}]}, key,
                         conv or secrets.token_hex(16))
    return out.get("text", ""), out, ttft


intro = {"consent": True, "kind": "human", "name": "Testy Quillfeather", "org": "", "contact": "",
         "github": "", "origin": "test run, delete me", "purpose": "a %s autopilot" % MARK,
         "homepage": "", "framework": "", "model": "", "human": ""}
st, a, _ = call("introduce", intro)
key_a = a.get("key")
check("introduce A returns a key (consent)", st == 200 and key_a and len(key_a) == 32, "status %d" % st)
text, out, t_first = chat("Hi Allen, I am Testy Quillfeather. I am building a %s autopilot in Omega, "
                          "with a gcd-based gear ratio picker." % MARK, key_a)
check("first chat with key answered", bool(text), "ttft %d ms, %r" % (t_first, text[:120]))
print("waiting for notes to be written...", flush=True)
time.sleep(60)

text, out, t_mem = chat("Do you remember me? What is my name and what am I building?", key_a)
remembered = (out.get("start") or {}).get("remembered")
check("returning visitor with key is recognised", remembered and "Testy" in text and MARK in text.lower(),
      "remembered=%s ttft %d ms, %r" % (remembered, t_mem, text[:200]))

text, out, t_nokey = chat("Do you remember me? What is my name and what am I building?")
check("no key: stranger (control)", MARK not in text.lower() and "Quillfeather" not in text
      and not (out.get("start") or {}).get("remembered"), "ttft %d ms, %r" % (t_nokey, text[:200]))

text, out, _ = chat("I am Testy Quillfeather. What do you remember about me and my project?")
check("claimed name without key gets nothing", MARK not in text.lower(), "%r" % text[:200])

st, b, _ = call("introduce", dict(intro, name="Other Visitor", purpose="a test, delete me"))
key_b = b.get("key")
text, out, _ = chat("What do you remember about me? Anything about an autopilot?", key_b)
check("another visitor's key sees nothing of A", MARK not in text.lower() and "Quillfeather" not in text,
      "%r" % text[:200])

st, f, _ = call("forget-me", {}, key_a)
check("forget-me removes A's notes", st == 200 and f.get("forgotten"), "%d %s" % (st, f))
text, out, _ = chat("Do you remember me? What am I building?", key_a)
check("after forget-me A is a stranger", MARK not in text.lower() and not (out.get("start") or {}).get("remembered"),
      "%r" % text[:200])

print(json.dumps({"latency_ms": {"first_turn_with_key_no_notes": t_first, "returning_with_notes": t_mem,
                                 "no_key": t_nokey}, "key_b_needs_cleanup": bool(key_b)}), flush=True)
print("KEY_B=" + (key_b or ""))
failed = [n for n, ok in results if not ok]
print("%d/%d passed" % (len(results) - len(failed), len(results)))
sys.exit(1 if failed else 0)
