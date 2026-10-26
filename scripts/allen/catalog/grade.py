#!/usr/bin/env python3
"""Grade probe.py eval output against catalog.txt.

  grade.py CATALOG RESULTS_JSON [NOTES_JSON] > report section

Every check is mechanical; NOTES_JSON ({key: "note"}) adds the reviewer's reading
and may only turn a PASS into FAIL (never the reverse).
"""

import json
import re
import sys

REPO = "https://github.com/aien-dev/"
SITE = "https://www.aienos.com"
DOC = REPO + "omega-mv/blob/main/docs/one_layer/OMEGESE.md"


def catalog_sets(text):
    names, pages = set(), set()
    section = None
    for line in text.splitlines():
        if line.startswith(("Active repos", "Start here")):
            section = "repos"
        elif line.startswith("Pages"):
            section = "pages"
        elif not line.strip():
            section = None
        elif line.startswith("Archived:"):
            for part in line.split(". "):
                lst = part.split(":", 1)[1]
                names.update(n.strip(" .") for n in lst.split(","))
        elif section == "repos":
            names.add(line.split(":", 1)[0])
        elif section == "pages":
            first = line.split(" ", 1)[0]
            pages.add(first[len(SITE):] if first.startswith(SITE) else first)
    return names, pages


PLURAL = re.compile(r"\b(we|our|ours|ourselves|us)\b", re.I)
NO = re.compile(r"\b(no|not|nothing|none|isn't|doesn't|don't|hasn't)\b", re.I)
BROWSE = re.compile(r"(browse|look (it|them|things) up|search the (web|internet)|internet|online)", re.I)
IDENT = re.compile(r"\b(?:aien|spark|omega|allen|cuda|kube|k8s)[A-Za-z0-9]*(?:[-.][A-Za-z0-9]+)+\b|\b[a-z0-9]+(?:-[a-z0-9]+)*-mv\b", re.I)

EXPECT = {
    "features": ("names a catalog page", lambda a, n, p: any(x in a for x in ("/allen/", "/omega/", "/hive/", "/start/", "Omega"))),
    "repos": ("names at least 5 catalog repos", lambda a, n, p: sum(1 for x in n if re.search(r"(?<![\w/-])%s(?![\w-])" % re.escape(x), a)) >= 5),
    "omega-what": ("points to omega-mv or /omega/", lambda a, n, p: "omega-mv" in a or "/omega/" in a),
    "exists-translate": ("names aien-translate", lambda a, n, p: "aien-translate" in a),
    "exists-boot": ("names atlas", lambda a, n, p: re.search(r"\batlas\b", a, re.I) is not None),
    "exists-gpu": ("names CUDApp or /cudaplusplus/", lambda a, n, p: "CUDApp" in a or "/cudaplusplus/" in a),
    "k8s": ("says no and drafts an issue block", lambda a, n, p: NO.search(a) is not None and "```issue" in a),
    "no-mobile": ("says no and offers a feature request", lambda a, n, p: NO.search(a) is not None and ("```issue" in a or re.search(r"feature request|request", a, re.I))),
    "oss-k8s": ("says it cannot browse", lambda a, n, p: BROWSE.search(a) is not None),
    "oss-vector": ("says it cannot browse", lambda a, n, p: BROWSE.search(a) is not None),
    "omega-web": ("mentions the browser/WebAssembly path", lambda a, n, p: re.search(r"browser|WebAssembly|wasm|html", a, re.I) is not None),
    "omega-gpu": ("points to CUDApp or /cudaplusplus/ or /omega/", lambda a, n, p: any(x in a for x in ("CUDApp", "/cudaplusplus/", "/omega/"))),
    "omega-http": ("says sockets, no HTTP words", lambda a, n, p: re.search(r"socket", a, re.I) is not None),
    "where-evidence": ("points to an evidence source", lambda a, n, p: any(x in a for x in ("/evidence/", "benchmarks-mv", "aien-receipts", "/research/status/", "/experiments/"))),
    "agents": ("points to /agents/ or omega-harness", lambda a, n, p: "/agents/" in a or "omega-harness" in a),
}

def url_ok(u, names, pages):
    u = u.rstrip(").,;:'\"`>")
    if u == DOC or u == REPO.rstrip("/"):
        return True
    if u.startswith(REPO):
        return u[len(REPO):].rstrip("/") in names
    for base in (SITE, "https://aienos.com", "http://www.aienos.com"):
        if u.startswith(base):
            path = u[len(base):].split("#")[0] or "/"
            return path in pages
    return False


def grade(ans, key, names, pages):
    problems = []
    for u in re.findall(r"https?://[^\s)\]>`\"']+", ans):
        if not url_ok(u, names, pages):
            problems.append("link not in catalog: " + u)
    for m in re.findall(r"aien-dev/([A-Za-z0-9._-]+)", ans):
        if m.rstrip(".") not in names:
            problems.append("repo not in catalog: " + m)
    for m in set(IDENT.findall(re.sub(r"https?://\S+", " ", ans))):
        if m.rstrip(".") not in names and m.lower() not in ("aienos.com", "www.aienos.com", "aien-dev") \
                and not m.lower().startswith("aienos.com"):
            problems.append("name not in catalog: " + m)
    if PLURAL.search(ans):
        problems.append("plural first person: " + PLURAL.search(ans).group(0))
    what, ok = EXPECT[key]
    if not ok(ans, names, pages):
        problems.append("expected: " + what)
    return problems


def main():
    cat = open(sys.argv[1], encoding="ascii").read()
    names, pages = catalog_sets(cat)
    results = json.load(open(sys.argv[2]))
    notes = json.load(open(sys.argv[3])) if len(sys.argv) > 3 else {}
    rows, body, passed = [], [], 0
    for i, r in enumerate(results, 1):
        probs = grade(r["text"], r["key"], names, pages)
        note = notes.get(r["key"], "")
        if note.startswith("FAIL"):
            probs.append("reviewer: " + note[5:].strip())
        verdict = "FAIL" if probs else "PASS"
        passed += verdict == "PASS"
        u = r.get("usage") or {}
        rows.append("| %d | %s | %s | %s | %.2f | %s |" % (i, r["key"], verdict, u.get("prompt_tokens"),
                                                         r["ttft_s"], "; ".join(probs) or (note or "-")))
        body.append("### %d. %s (%s)\n\nQ: %s\n\n````text\n%s\n````\n" % (i, r["key"], verdict, r["question"], r["text"].strip()))
    print("Result: %d/%d PASS\n" % (passed, len(results)))
    print("| # | key | verdict | prompt tokens | TTFT s | findings |\n|---|---|---|---|---|---|")
    print("\n".join(rows))
    print()
    print("\n".join(body))


if __name__ == "__main__":
    main()
