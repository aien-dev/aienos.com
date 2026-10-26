#!/usr/bin/env python3
"""Build Allen's public catalog: catalog.txt and areas.txt.

Only unauthenticated sources are read, so a private repository cannot appear:
  - https://api.github.com/orgs/aien-dev/repos?type=public (no Authorization header)
  - git ls-remote https://github.com/aien-dev/<repo> HEAD with every credential
    source disabled (no helper, no global/system config, no prompt, no SSH agent)
  - https://raw.githubusercontent.com/aien-dev/<repo>/<head sha>/README.md
  - aienos.com crates/site-builder/src/seo.rs at its head sha

Output is ASCII, LF, sorted, with no clock time: it changes only when an input
changes. Any failed fetch exits nonzero and leaves the old files untouched;
both files are written to temporaries and renamed into place.

Usage: catalog.py OUTDIR
"""

import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import unicodedata
import urllib.error
import urllib.request

ORG = "aien-dev"
API = "https://api.github.com/orgs/%s/repos?type=public&per_page=100" % ORG
RAW = "https://raw.githubusercontent.com/%s/%s/%s/%s"
SITE_REPO = "aienos.com"
SEO_PATH = "crates/site-builder/src/seo.rs"
SITE = "https://www.aienos.com"
UA = "aienos-allen-catalog/1"
TIMEOUT = 30

PURPOSE_MAX = 110  # characters of a repo's one-line purpose
PAGE_MAX = 65      # characters of a page title plus its description
# Entry points Allen sends visitors and agents to keep more of their seo.rs text.
ENTRY_PAGES = {"/agents/": 180, "/start/": 180}
SHA_LEN = 7        # head sha digits shown per repo (the inputs digest covers the full shas)

# Fallback for /allen/ until its PageInfo reaches aienos.com main; the text is the
# PageInfo on the feat/allen-chat branch, verbatim.
ALLEN_PAGE = ("/allen/", "Talk to Allen",
              "Chat with Allen, Drake's AI, on the live model on a DGX Spark: speed measured "
              "from the stream, questions about Omega, and small HTML or Omega programs that "
              "run sandboxed in your browser. Conversations are not stored.")

OMEGA_BRIEF = (
    "Omega in brief: Drake's language for verified programs written by AI, GPU to CPU (repo omega-mv; "
    "Omegese is the program language, omc the compiler). The words and rules are above; full reference "
    "docs/one_layer/OMEGESE.md in omega-mv; overview page /omega/; GPU half: CUDApp and /cudaplusplus/."
)

REACH = ("Reach Drake: X https://x.com/mdrakestapleton, email drake@aienos.com, "
         "or talk to Allen at " + SITE + "/allen/.")

PLURAL = re.compile(r"\b(we|our|ours|ourselves|us)\b", re.I)


class FetchError(Exception):
    pass


def get(url, allow_404=False):
    """GET with no credentials. Returns (body bytes, headers) or (None, None) on an allowed 404."""
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "*/*"})
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as r:
            return r.read(), r.headers
    except urllib.error.HTTPError as e:
        if allow_404 and e.code == 404:
            return None, None
        raise FetchError("%s: HTTP %d" % (url, e.code))
    except (urllib.error.URLError, OSError) as e:
        raise FetchError("%s: %s" % (url, e))


def list_repos():
    repos, url = [], API
    while url:
        body, headers = get(url)
        try:
            page = json.loads(body)
        except ValueError:
            raise FetchError("%s: not JSON" % url)
        if not isinstance(page, list):
            raise FetchError("%s: not a list" % url)
        repos.extend(page)
        url = None
        for part in (headers.get("Link") or "").split(","):
            m = re.match(r'\s*<(https://api\.github\.com/[^>]+)>;\s*rel="next"', part)
            if m:
                url = m.group(1)
    if not repos:
        raise FetchError("no public repositories listed")
    for r in repos:
        if r.get("private") or r.get("visibility", "public") != "public":
            raise FetchError("non-public repository in an unauthenticated listing: refusing")
    return repos


def head_sha(name):
    env = {k: v for k, v in os.environ.items() if not k.startswith(("GIT_", "SSH_"))}
    env.update(GIT_TERMINAL_PROMPT="0", GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
               GIT_ASKPASS="", SSH_ASKPASS="")
    cmd = ["git", "-c", "credential.helper=", "ls-remote",
           "https://github.com/%s/%s" % (ORG, name), "HEAD"]
    try:
        p = subprocess.run(cmd, env=env, stdin=subprocess.DEVNULL, capture_output=True,
                           text=True, timeout=TIMEOUT)
    except (OSError, subprocess.TimeoutExpired) as e:
        raise FetchError("ls-remote %s: %s" % (name, e))
    out = p.stdout.strip()
    if p.returncode == 0 and not out:
        return None  # an empty repository has no HEAD yet
    m = re.match(r"([0-9a-f]{40})\tHEAD$", out)
    if p.returncode != 0 or not m:
        raise FetchError("ls-remote %s: exit %d" % (name, p.returncode))
    return m.group(1)


def ascii_text(s):
    for a, b in (("\u2018", "'"), ("\u2019", "'"), ("\u201c", '"'), ("\u201d", '"'),
                 ("\u2013", "-"), ("\u2014", "-"), ("\u2026", "..."), ("\u00a0", " "),
                 ("\u2192", "->"), ("\u00d7", "x")):
        s = s.replace(a, b)
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode("ascii")
    return re.sub(r"\s+", " ", s).strip()


def clip(s, limit):
    """Truncate (never reword): cut before any plural first person clause, then keep whole
    sentences that fit, else cut at the last clause boundary, else at a word boundary."""
    m = PLURAL.search(s)
    if m:
        head = s[:m.start()]
        cut = max(head.rfind(", "), head.rfind("; "), head.rfind(": "), head.rfind(". "))
        s = (head[:cut] if cut > 0 else "").rstrip(" ,;:") + "..."
        if len(s) < 24:
            return ""
    if len(s) <= limit:
        return s
    end = s[:limit + 1].rfind(". ")
    if end >= limit // 2:
        return s[:end + 1]
    head = s[:limit - 2]
    cut = max(head.rfind(", "), head.rfind("; "), head.rfind(" ("))
    if cut >= limit // 2:
        return head[:cut].rstrip(" ,;:(.") + "..."
    return s[:limit - 3].rsplit(" ", 1)[0].rstrip(" ,;:(.") + "..."


def md_plain(line):
    line = re.sub(r"!\[[^\]]*\]\([^)]*\)", "", line)            # images
    line = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", line)        # links -> text
    line = re.sub(r"\[([^\]]+)\]\[[^\]]*\]", r"\1", line)       # reference links
    line = re.sub(r"<[^>]+>", "", line)                         # html tags
    line = re.sub(r"(\*\*|__|`)", "", line)                     # bold, code
    line = re.sub(r"(?<!\w)[*_](\S[^*_]*\S|\S)[*_](?!\w)", r"\1", line)  # italics
    return line


def first_sentence(readme):
    """First prose sentence of a README, verbatim apart from markdown removal."""
    text = readme.decode("utf-8", "replace")
    para, fence = [], False
    for raw in text.splitlines():
        s = raw.strip()
        if s.startswith(("```", "~~~")):
            fence = not fence
            continue
        if fence:
            continue
        if not s:
            if para:
                break
            continue
        if para:
            para.append(s)
            continue
        if s.startswith(("#", "<", "![", "[!", "[![", "|", ">", "-", "*", "+", "=", "---")) \
                or re.match(r"\d+[.)] ", s) or re.match(r"\[[^\]]+\]:", s):
            continue
        plain = ascii_text(md_plain(s))
        if len(plain) < 20 or not re.match(r"[A-Za-z0-9\"']", plain):
            continue
        para.append(s)
    if not para:
        return ""
    body = ascii_text(md_plain(" ".join(para)))
    m = re.search(r"(.+?[.!?])(\s+[A-Z(\"']|\s*$)", body)
    return m.group(1) if m else body



def repo_record(r):
    name = r["name"]
    sha = head_sha(name)
    readme = None
    if sha:
        readme, _ = get(RAW % (ORG, name, sha, "README.md"), allow_404=True)
    sentence = first_sentence(readme) if readme else ""
    desc = ascii_text(r.get("description") or "")
    lic = (r.get("license") or {}).get("spdx_id") or "none"
    if lic == "NOASSERTION":
        lic = "other"
    return {"name": name, "sha": sha, "license": lic, "pushed": (r.get("pushed_at") or "")[:7],
            "purpose": clip(desc or sentence, PURPOSE_MAX),
            "readme": hashlib.sha256(readme or b"").hexdigest()}


def site_pages(sha):
    body, _ = get(RAW % (ORG, SITE_REPO, sha, SEO_PATH))
    src = body.decode("utf-8")
    start = src.find("pub const PAGES: &[PageInfo] = &[")
    end = src.find("\n];", start)
    if start < 0 or end < 0:
        raise FetchError("seo.rs: PAGES table not found")
    block = src[start:end]
    q = r'"((?:[^"\\]|\\.)*)"'
    pages = re.findall(r"PageInfo \{ path: %s, name: %s, what: %s," % (q, q, q), block)
    if not pages or len(pages) != block.count("PageInfo {"):
        raise FetchError("seo.rs: %d PageInfo entries parsed, %d present"
                         % (len(pages), block.count("PageInfo {")))
    unesc = lambda s: re.sub(r"\\(.)", r"\1", s)
    out = {unesc(p): (unesc(n), unesc(w)) for p, n, w in pages}
    out.setdefault(ALLEN_PAGE[0], ALLEN_PAGE[1:])
    return out, hashlib.sha256(body).hexdigest()


def build():
    repos = list_repos()
    keep = sorted((r for r in repos if not r.get("fork") and not r.get("archived")),
                  key=lambda r: r["name"].lower())
    archived = sorted((r["name"] for r in repos if r.get("archived") and not r.get("fork")), key=str.lower)
    forks = sorted((r["name"] for r in repos if r.get("fork")), key=str.lower)
    with concurrent.futures.ThreadPoolExecutor(8) as pool:
        records = list(pool.map(repo_record, keep))
        site_sha = pool.submit(head_sha, SITE_REPO).result()
    if not site_sha:
        raise FetchError("%s has no HEAD" % SITE_REPO)
    pages, seo_digest = site_pages(site_sha)

    inputs = hashlib.sha256()
    for rec in records:
        inputs.update(json.dumps(rec, sort_keys=True).encode())
    inputs.update(json.dumps([archived, forks, seo_digest], sort_keys=True).encode())

    months = [rec["pushed"] for rec in records]
    usual = max(sorted(set(months)), key=months.count)  # most common last-push month, latest on a tie

    L = []
    L.append("Drake's public repos and site pages; nothing else of his is public. Name them and cite "
             "only these URLs; if nothing fits, say Drake has nothing public for it.")
    L.append("Sources: github.com/%s public repos, unauthenticated; %s seo.rs @%s; inputs %s."
             % (ORG, SITE_REPO, site_sha[:SHA_LEN], inputs.hexdigest()[:16]))
    L.append("")
    L.append("Active repos (URL: https://github.com/%s/ then the name; last push %s unless noted). "
             "name: purpose (license, head):" % (ORG, usual))
    for rec in records:
        meta = [rec["license"]]
        if rec["pushed"] != usual:
            meta.append("last push " + rec["pushed"])
        meta.append(rec["sha"][:SHA_LEN] if rec["sha"] else "empty")
        L.append("%s: %s (%s)" % (rec["name"], rec["purpose"] or "no description", ", ".join(meta)))
    L.append("Archived: %s. Forks of outside projects: %s." % (", ".join(archived), ", ".join(forks)))
    L.append("")
    L.append("Pages on Drake's site (URL, title: what). These are pages, not repos:")
    for path in sorted(pages):
        name, what = pages[path]
        name = ascii_text(name)
        room = (ENTRY_PAGES.get(path) or PAGE_MAX) - len(name)
        what = clip(ascii_text(re.sub(r"\s*\([^)]*\)", "", what)), room) if room >= 30 else ""
        L.append("%s%s %s%s" % (SITE, path, name, ": " + what if what else ""))
    L.append("")
    L.append(OMEGA_BRIEF)
    L.append(REACH)
    catalog = "\n".join(L) + "\n"
    catalog.encode("ascii")  # raises if anything non-ASCII slipped through
    areas = "".join(r["name"] + "\n" for r in records)
    return catalog, areas


def write_atomic(outdir, files):
    tmps = []
    try:
        for name, text in files:
            fd, tmp = tempfile.mkstemp(prefix="." + name + ".", dir=outdir)
            tmps.append((tmp, name))
            with os.fdopen(fd, "w", encoding="ascii", newline="\n") as f:
                f.write(text)
                f.flush()
                os.fsync(f.fileno())
            os.chmod(tmp, 0o644)
        for tmp, name in tmps:
            os.replace(tmp, os.path.join(outdir, name))
        tmps = []
    finally:
        for tmp, _ in tmps:
            try:
                os.unlink(tmp)
            except OSError:
                pass


def same(outdir, name, text):
    try:
        with open(os.path.join(outdir, name), "rb") as f:
            return f.read() == text.encode("ascii")
    except FileNotFoundError:
        return False


def main():
    if len(sys.argv) != 2 or not os.path.isdir(sys.argv[1]):
        sys.exit("usage: catalog.py OUTDIR")
    try:
        catalog, areas = build()
    except (FetchError, UnicodeError, KeyError, TypeError, ValueError) as e:
        print("catalog: not written: %s" % e, file=sys.stderr)
        sys.exit(1)
    outdir = sys.argv[1]
    changed = [(n, t) for n, t in (("areas.txt", areas), ("catalog.txt", catalog)) if not same(outdir, n, t)]
    if not changed:
        # an untouched file keeps its mtime, so the gateway keeps its warm prefix
        print("catalog: unchanged, %d bytes, %d areas" % (len(catalog), areas.count("\n")))
        return
    write_atomic(outdir, changed)
    print("catalog: wrote %s, %d bytes, %d areas"
          % (" ".join(n for n, _ in changed), len(catalog), areas.count("\n")))


if __name__ == "__main__":
    main()
