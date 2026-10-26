#!/usr/bin/env python3
"""Generate the answer cache (answers/answers.json) offline through the shared model.

Each question is sent as a visitor's exact anonymous first turn (allen_speed.generation_messages:
system prompt + catalog, the anonymous opening, the question with its stable cards) at
temperature 0, one request at a time and only while the server's /health reports at most
--max-inflight requests. Entries already in --out whose key still matches are kept as they are.
The gateway serves an entry only while its prefix_sha and grounding_sha match the live files,
and regenerates stale entries itself (allen_chat.py App.regen).

    python3 gen_answers.py --out answers/answers.json \\
        --catalog /var/lib/allen-catalog/catalog.txt \\
        --catalog-detail /var/lib/allen-catalog/catalog-detail.jsonl \\
        --catalog-retrieve /opt/allen-catalog/retrieve.py
"""

import argparse
import http.client
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import allen_chat  # noqa: E402  (its constants, so the opening matches byte for byte)
import allen_speed  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
MODEL = allen_chat.MODEL


def health(host, port, headers):
    try:
        c = http.client.HTTPConnection(host, port, timeout=5)
        c.request("GET", "/health", headers=headers)
        r = c.getresponse()
        d = json.loads(r.read(4096))
        c.close()
        return d if r.status == 200 and d.get("status") == "ok" else None
    except (OSError, ValueError, http.client.HTTPException):
        return None


def write(path, entries):
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump({"format": 1, "entries": sorted(entries.values(), key=lambda e: e["id"])}, f,
                  ensure_ascii=False, indent=1, sort_keys=True)
        f.write("\n")
    os.replace(tmp, path)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--questions", default=os.path.join(HERE, "answers", "questions.json"))
    ap.add_argument("--out", default=os.path.join(HERE, "answers", "answers.json"))
    ap.add_argument("--system-prompt", default=os.path.join(HERE, "system-prompt.txt"))
    ap.add_argument("--catalog", required=True)
    ap.add_argument("--catalog-detail")
    ap.add_argument("--catalog-retrieve")
    ap.add_argument("--learn-dir")
    ap.add_argument("--upstream", default="127.0.0.1:18091")
    ap.add_argument("--max-inflight", type=int, default=2)
    ap.add_argument("--max-tokens", type=int, default=1024)
    ap.add_argument("--only", nargs="*", help="question ids to (re)generate even if still valid")
    ap.add_argument("--log", help="JSON lines: one timing record per request")
    ap.add_argument("--tools", default="", help="the gateway's --tools registry, when production sets one")
    a = ap.parse_args()

    with open(a.system_prompt, encoding="utf-8") as f:
        prompt = f.read()
    tools = allen_chat.load_tools(a.tools) if a.tools else None
    specs = list(tools.TOOLS) if tools else []
    if tools:  # the same composition as App.system_text()
        prompt = allen_speed.join_system(prompt, tools.SYSTEM_PROMPT.strip() + "\n")
    with open(a.catalog, encoding="utf-8") as f:
        text = allen_speed.join_system(prompt, f.read())
    psha = allen_speed.prefix_key(allen_speed.sha256_bytes(text.encode()), specs)
    cards = allen_speed.Cards(a.catalog_detail, a.catalog_retrieve, a.learn_dir)
    cards.refresh()
    opening = [{"role": "user", "content": allen_chat.OPEN_USER_ANON},
               {"role": "assistant", "content": allen_chat.GREETING_ANON}]
    with open(a.questions, encoding="utf-8") as f:
        questions = json.load(f)["questions"]
    have = {}
    if os.path.exists(a.out):
        with open(a.out, encoding="utf-8") as f:
            have = {e["id"]: e for e in json.load(f)["entries"]}
    store = allen_speed.Answers()
    host, _, port = a.upstream.rpartition(":")
    port = int(port)
    token = os.environ.get("ALLEN_UPSTREAM_TOKEN", "").strip()
    headers = {"Authorization": "Bearer " + token} if token else {}
    log = open(a.log, "a", encoding="utf-8") if a.log else None
    out = {}
    for q in questions:
        old = have.get(q["id"])
        if old and not (a.only and q["id"] in a.only) and old["q"] == q["q"] \
                and store.valid(old, psha, cards):
            out[q["id"]] = dict(old, alt=q.get("alt") or [])
            continue
        if a.only and q["id"] not in a.only:
            if old:
                out[q["id"]] = old
            continue
        while True:
            h = health(host, port, headers)
            if h and int(h.get("in_flight", 0)) <= a.max_inflight:
                break
            time.sleep(3)
        req = {"model": MODEL, "max_tokens": a.max_tokens, "temperature": 0, "stream": False,
               "messages": allen_speed.generation_messages(text, opening, q["q"], cards)}
        if specs:
            req.update(tools=specs, tool_choice="none")
        body = json.dumps(req).encode()
        t0 = time.monotonic()
        c = http.client.HTTPConnection(host, port, timeout=600)
        c.request("POST", "/v1/chat/completions", body, dict(headers, **{"Content-Type": "application/json"}))
        r = c.getresponse()
        res = json.loads(r.read(1 << 20))
        c.close()
        ms = int((time.monotonic() - t0) * 1000)
        ch = res["choices"][0]
        answer = (ch["message"]["content"] or "").strip()
        rec = {"id": q["id"], "status": r.status, "ms": ms, "in_flight_before": int(h.get("in_flight", 0)),
               "finish": ch.get("finish_reason"), "usage": res.get("usage")}
        print(json.dumps(rec), flush=True)
        if log:
            log.write(json.dumps(rec) + "\n")
            log.flush()
        if r.status != 200 or ch.get("finish_reason") != "stop" or not answer:
            print("skip %s: not a complete answer" % q["id"], file=sys.stderr)
            continue
        out[q["id"]] = allen_speed.make_entry(q, answer, psha, cards, MODEL,
                                              time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
        write(a.out, out)
    write(a.out, out)
    print("entries %d, prefix_sha %s" % (len(out), psha))


if __name__ == "__main__":
    main()
