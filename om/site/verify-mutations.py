#!/usr/bin/env python3
"""Negative-control runner for the Omega site verifier (verification only, never in production).

Each row of om/site/verify-mutations.jsonl names the original oracle rule it exercises (`rule`,
the B/V/C/H/E/F/G ids of crates/site-builder plus O/D/L/Q/N1/R0 for the gates the oracle grew),
the mutation (`ops`) and what the Omega verifier must say (`expect`): a rule label that must appear
as `verify: FAIL <label> ` with exit 1, `EXIT70` for a fail-closed runtime refusal, `PASS` for a
control that must stay accepted, or `HASH` for the artifact-hash gate this runner itself performs
with the pinned external `sha256sum` (an oracle, not part of the site build).

Usage (from the repository root, after a known-good dist exists):
  python3 om/site/verify-mutations.py --verifier build/site-verify --dist dist \
      [--oracle target/release/site-builder] [--only ID,...] [--keep]

Before any mutation the runner proves its setup: the unmodified dist, every source data file and
`--sources` must pass, and the artifact hashes must match. A mutation whose anchor text is absent is
INVALID (the control did not run), never a pass. Exit 0 only when every row behaved as expected.
"""
import argparse, hashlib, html, json, os, re, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROWS = os.path.join(HERE, "verify-mutations.jsonl")
QUOTES = "data/quotes.json"
ENGINEER = "public/data/engineer.json"
SOURCES = ["public/data/now.tsv", "facts.json", "public/data/agent-repos.json",
           "crates/site-builder/data/machines.json", "crates/site-builder/data/omega.json",
           "crates/site-builder/data/cudaplusplus.json", "crates/site-builder/data/research_status.json",
           "crates/site-builder/data/status.json", "public/data/project-progress.json", QUOTES, ENGINEER]
# Artifact hashes (rule B8): (sidecar, dist copy). The sidecar names the source file it pins.
HASHES = [("om/hive-page/hive.js.sha256", "js/hive.js"),
          ("om/hive-page/turing-look.js.sha256", "js/turing-look.js"),
          ("public/wasm/hive-fresh.wasm.sha256", "wasm/hive-fresh.wasm"),
          ("om/hive-page/main.html.sha256", None)]
BOX_CLASSES = ("mx-card mx-row glow-box lineage-card lineage-role status-row live-row receipt rs-card rs-step "
               "rs-impl-item hub-card narr-card blog-card now-card now-queue-row now-tl now-usage om-part om-layer "
               "om-receipt om-gap om-bar-row om-beyond cp-hop cp-card cp-row dash-card").split()
DESTS = ["/classic/", "/omega/", "/machine/", "/discoveries/", "/learn/", "/allen/"]


class Invalid(Exception):
    pass


def rd(p):
    with open(p, "rb") as f:
        return f.read()


def wr(p, b):
    os.makedirs(os.path.dirname(p) or ".", exist_ok=True)
    with open(p, "wb") as f:
        f.write(b)


def text(p):
    return rd(p).decode("utf-8")


def jpath(obj, path, create=False):
    keys = [k for k in path.split(".")] if path else []
    for k in keys[:-1]:
        obj = obj[int(k)] if isinstance(obj, list) else obj[k]
    return obj, (keys[-1] if keys else None)


def jkey(parent, k):
    return int(k) if isinstance(parent, list) else k


def first_tag(s, pat):
    m = re.search(pat, s)
    if not m:
        raise Invalid(f"anchor {pat!r} absent")
    return m


def sha256sum(path):
    out = subprocess.run(["sha256sum", path], capture_output=True, check=True, text=True).stdout
    return out.split()[0]


def apply(op, root, files):
    name, args = op[0], op[1:]

    def P(a):
        return files.get(a) or os.path.join(root, a)

    if name in ("sub", "suball"):
        p, old, new = P(args[0]), args[1], args[2]
        s = text(p)
        if old not in s:
            raise Invalid(f"{args[0]}: {old!r} absent")
        wr(p, (s.replace(old, new, 1) if name == "sub" else s.replace(old, new)).encode())
    elif name == "ins":
        p, before, t = P(args[0]), args[1], args[2]
        s = text(p)
        if before not in s:
            raise Invalid(f"{args[0]}: {before!r} absent")
        wr(p, s.replace(before, t + before, 1).encode())
    elif name == "app":
        p = P(args[0])
        if not os.path.exists(p):
            raise Invalid(f"{args[0]} absent")
        wr(p, rd(p) + args[1].encode())
    elif name == "put":
        wr(P(args[0]), args[1].encode())
    elif name == "del":
        p = P(args[0])
        if not os.path.lexists(p):
            raise Invalid(f"{args[0]} absent")
        os.remove(p)
    elif name == "trunc":
        p = P(args[0])
        wr(p, rd(p)[: -int(args[1])])
    elif name == "flipbyte":
        p = P(args[0])
        b = bytearray(rd(p))
        if not b:
            raise Invalid(f"{args[0]} empty")
        b[-1] ^= 1
        wr(p, bytes(b))
    elif name == "symlink":
        target, link = P(args[0]), P(args[1])
        if not os.path.exists(target):
            raise Invalid(f"{args[0]} absent")
        os.symlink(os.path.relpath(target, os.path.dirname(link)), link)
    elif name in ("jset", "jdel", "jcopy", "jtrunc", "jappend"):
        p = P(args[0])
        doc = json.loads(text(p))
        try:
            if name == "jset":
                par, k = jpath(doc, args[1])
                par[jkey(par, k)] = args[2]
            elif name == "jdel":
                par, k = jpath(doc, args[1])
                del par[jkey(par, k)]
            elif name == "jcopy":
                sp, sk = jpath(doc, args[1])
                dp, dk = jpath(doc, args[2])
                dp[jkey(dp, dk)] = sp[jkey(sp, sk)]
            elif name == "jtrunc":
                par, k = jpath(doc, args[1])
                lst = par[jkey(par, k)]
                if len(lst) <= args[2]:
                    raise Invalid(f"{args[1]} already has {len(lst)} items")
                del lst[args[2]:]
            else:
                par, k = jpath(doc, args[1])
                par[jkey(par, k)].append(args[2])
        except (KeyError, IndexError, TypeError) as e:
            raise Invalid(f"{args[0]}: path {args[1]}: {e!r}")
        wr(p, (json.dumps(doc, ensure_ascii=False, indent=1) + "\n").encode())
    elif name == "machines_above_ceiling":
        p = P(args[0])
        doc = json.loads(text(p))
        ceil = {c["id"]: c["tok_s"] for c in doc["ceilings"]}
        for sp in doc["speeds"]:
            if sp.get("ceiling") in ceil:
                sp["tok_s"] = ceil[sp["ceiling"]] * 2
                break
        else:
            raise Invalid("no speed with a ceiling")
        wr(p, (json.dumps(doc, ensure_ascii=False, indent=1) + "\n").encode())
    elif name == "engineer_second_home":
        p = P(args[0])
        doc = json.loads(text(p))
        others = [x for x in doc["pages"] if x.get("kind") != "home"]
        if not others:
            raise Invalid("no non-home engineer page")
        others[0]["kind"] = "home"
        wr(p, (json.dumps(doc, ensure_ascii=False, indent=1) + "\n").encode())
    elif name in ("unbox", "boxlink_off", "boxlink_drop"):
        p = P(args[0])
        s = text(p)
        if name == "unbox":
            def strip_cls(m):
                toks = [t for t in m.group(1).split() if t not in BOX_CLASSES]
                return 'class="' + " ".join(toks) + '"'
            t = re.sub(r'class="([^"]*)"', strip_cls, s)
            if t == s:
                raise Invalid("no box classes")
        else:
            m = first_tag(s, r'<a [^>]*class="[^"]*\bbox-link\b[^"]*"[^>]*>')
            tag = m.group(0)
            if name == "boxlink_off":
                new = re.sub(r'href="[^"]*"', 'href="/now/"', tag, count=1)
            else:
                new = re.sub(r'\bbox-link\b', "box-lnk", tag, count=1)
            t = s[: m.start()] + new + s[m.end():]
        wr(p, t.encode())
    elif name == "ldgraph_trunc":
        p = P(args[0])
        s = text(p)
        o = '<script type="application/ld+json">'
        a = s.index(o) + len(o)
        b = s.index("</script>", a)
        doc = json.loads(s[a:b])
        if len(doc.get("@graph", [])) <= args[1]:
            raise Invalid("graph already small")
        doc["@graph"] = doc["@graph"][: args[1]]
        wr(p, (s[:a] + json.dumps(doc, ensure_ascii=False) + s[b:]).encode())
    elif name in ("quote_html", "quote_raw"):
        p = P(args[0])
        q = {x["id"]: x["quote"] for x in json.loads(text(files["$QUOTES"]))["quotes"]}[args[1]]
        s = text(p)
        if name == "quote_html":
            if "</main>" not in s:
                raise Invalid("no </main>")
            block = f'<blockquote data-quote-id="{args[1]}">{html.escape(q, quote=True)}</blockquote>'
            s = s.replace("</main>", block + "</main>", 1)
        else:
            s = s + "\n" + q + "\n"
        wr(p, s.encode())
    elif name == "hide_destinations":
        p = P(args[0])
        s = text(p)
        t = s
        for h in DESTS:
            t = t.replace(f'<a href="{h}"', f'<a aria-hidden="true" href="{h}"')
        if t == s:
            raise Invalid("no destination anchors in the <a href= form")
        wr(p, t.encode())
    elif name == "x_after_main":
        p = P(args[0])
        s = text(p)
        cut = s.find("<main")
        if cut < 0:
            raise Invalid("no <main")
        head = s[:cut].replace('href="https://x.com/mdrakestapleton"', 'href="https://x.com/mdrakestapleton/moved"')
        if head == s[:cut]:
            raise Invalid("no X link before <main")
        wr(p, (head + s[cut:]).encode())
    elif name == "swap_ids":
        p, a, b = P(args[0]), args[1], args[2]
        s = text(p)
        if a not in s or b not in s:
            raise Invalid("ids absent")
        wr(p, s.replace(a, "\0").replace(b, a).replace("\0", b).encode())
    elif name == "move_before_footer_into_footer":
        p, needle = P(args[0]), args[1]
        s = text(p)
        cut = s.find("<footer")
        if cut < 0 or needle not in s[:cut]:
            raise Invalid("needle not before the footer")
        body = s[:cut].replace(needle, needle.replace("href", "data-moved"))
        rest = s[cut:]
        if needle not in rest:
            rest = rest.replace("</footer>", f"<a {needle}>moved</a></footer>", 1)
        wr(p, (body + rest).encode())
    else:
        raise Invalid(f"unknown op {name}")


def run(cmd, cwd=None):
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, timeout=1800)
    return r.returncode, r.stderr.decode("utf-8", "replace")


def hash_gate(repo, dist):
    bad = []
    for sidecar, copy in HASHES:
        want, name = text(os.path.join(repo, sidecar)).split()[:2]
        src = os.path.normpath(os.path.join(repo, os.path.dirname(sidecar), name))
        if sha256sum(src) != want:
            bad.append(f"{src} != {sidecar}")
        if copy and dist and sha256sum(os.path.join(dist, copy)) != want:
            bad.append(f"dist/{copy} != {sidecar}")
    return bad


def verdict(row, code, err):
    e = row["expect"]
    if e == "PASS":
        return code == 0
    if e == "EXIT70":
        return code == 70
    return code == 1 and f"verify: FAIL {e} " in err


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--verifier", required=True)
    ap.add_argument("--dist", required=True)
    ap.add_argument("--repo", default=".")
    ap.add_argument("--oracle")
    ap.add_argument("--only")
    ap.add_argument("--keep", action="store_true")
    a = ap.parse_args()
    repo = os.path.abspath(a.repo)
    bin_ = os.path.abspath(a.verifier)
    good = os.path.abspath(a.dist)
    rows = [json.loads(l) for l in open(ROWS, encoding="utf-8") if l.strip()]
    if a.only:
        keep = set(a.only.split(","))
        rows = [r for r in rows if r["id"] in keep]
    quotes, engineer = os.path.join(repo, QUOTES), os.path.join(repo, ENGINEER)
    setup = []
    code, err = run([bin_, good, quotes, engineer])
    if code != 0:
        setup.append(f"unmodified dist refused ({code}):\n{err[-4000:]}")
    code, err = run([bin_, "--sources"], cwd=repo)
    if code != 0:
        setup.append(f"--sources refused ({code}):\n{err[-4000:]}")
    setup += hash_gate(repo, good)
    if a.oracle:
        code, err = run([os.path.abspath(a.oracle), "verify", good], cwd=repo)
        if code != 0:
            setup.append(f"Rust oracle refused the unmodified dist ({code})")
    if setup:
        print("MUTATIONS INVALID setup:\n" + "\n".join(setup))
        return 2
    counts = {"PASS": 0, "FAIL": 0, "INVALID": 0}
    for row in rows:
        tmp = tempfile.mkdtemp(prefix="svm-")
        try:
            files = {"$QUOTES": os.path.join(tmp, "quotes.json"), "$ENGINEER": os.path.join(tmp, "engineer.json")}
            shutil.copy(quotes, files["$QUOTES"])
            shutil.copy(engineer, files["$ENGINEER"])
            if row["mode"] == "dist":
                root = os.path.join(tmp, "dist")
                shutil.copytree(good, root, symlinks=True)
            elif row["mode"] == "data":
                root = tmp
                files["$F"] = os.path.join(tmp, os.path.basename(row["src"]))
                shutil.copy(os.path.join(repo, row["src"]), files["$F"])
            else:
                root = os.path.join(tmp, "repo")
                for s in SOURCES:
                    if os.path.exists(os.path.join(repo, s)):
                        os.makedirs(os.path.dirname(os.path.join(root, s)) or root, exist_ok=True)
                        shutil.copy(os.path.join(repo, s), os.path.join(root, s))
                files["$F"] = os.path.join(root, row["src"])
            try:
                for op in row["ops"]:
                    apply(op, root, files)
            except (Invalid, OSError, ValueError, KeyError) as e:
                counts["INVALID"] += 1
                print(f"INVALID\t{row['id']}\t{row['rule']}\t{e}")
                continue
            if row["expect"] == "HASH":
                ok = bool(hash_gate(repo, root))
                code, err = (1 if ok else 0), ""
            elif row["mode"] == "dist":
                code, err = run([bin_, root, files["$QUOTES"], files["$ENGINEER"]])
                ok = verdict(row, code, err)
            elif row["mode"] == "data":
                code, err = run([bin_, "--data", row["kind"], files["$F"]])
                ok = verdict(row, code, err)
            else:
                code, err = run([bin_, "--sources"], cwd=root)
                ok = verdict(row, code, err)
            oracle = "n/a"
            if a.oracle and row["mode"] == "dist" and row.get("oracle") == "yes":
                ocode, _ = run([os.path.abspath(a.oracle), "verify", root], cwd=repo)
                oracle = "agrees" if (ocode == 0) == (row["expect"] == "PASS") else "DISAGREES"
                ok = ok and oracle == "agrees"
            counts["PASS" if ok else "FAIL"] += 1
            first = next((l for l in err.splitlines() if l.startswith("verify: FAIL")), "")
            print(f"{'PASS' if ok else 'FAIL'}\t{row['id']}\t{row['rule']}\texit={code}\toracle={oracle}\t{first[:200]}")
        finally:
            if not a.keep:
                shutil.rmtree(tmp, ignore_errors=True)
    print(f"MUTATIONS pass={counts['PASS']} fail={counts['FAIL']} invalid={counts['INVALID']} total={len(rows)}")
    return 0 if counts["FAIL"] == 0 and counts["INVALID"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
