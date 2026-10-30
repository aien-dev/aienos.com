#!/usr/bin/env python3
"""Build Allen's public catalog: catalog.txt, catalog-detail.jsonl and areas.txt.

Only unauthenticated sources are read, so a private repository cannot appear:
  - https://api.github.com/orgs/aien-dev/repos?type=public (no Authorization header;
    one API request per run, so hourly runs stay far below the 60/hour anonymous limit)
  - git ls-remote / a shallow treeless git fetch of https://github.com/aien-dev/<repo>
    with every credential source disabled (no helper, no global/system config, no
    prompt, no SSH agent): head sha, HEAD commit date and subject, top-level names
  - git ls-remote https://github.com/aien-dev/<repo>.wiki.git (wiki present or not)
  - https://raw.githubusercontent.com/aien-dev/<repo>/<head sha>/README.md (STATUS.md)
  - aienos.com om/site/pages.json, else crates/site-builder/src/seo.rs, at its head sha

catalog.txt is the model's cached prompt prefix: only what the model reads, so a push
does not change it. catalog-detail.jsonl holds per-repo detail and every head sha, for
retrieval (retrieve.py) into the last user turn. Status, how-to and docs are copied
from the repo's own text or omitted ("not stated"); nothing is reworded.

Output is ASCII, LF, sorted, with no clock time. Any failed fetch exits nonzero and
leaves the old files untouched; files are written through temporaries and renamed,
and an unchanged file is not rewritten (its mtime is the gateway's cache key).

Usage: catalog.py OUTDIR
"""

import concurrent.futures
import datetime
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
REGISTRY_PATH = "om/site/pages.json"  # the Omega site's page registry; preferred once present
SITE = "https://www.aienos.com"
UA = "aienos-allen-catalog/1"
TIMEOUT = 30

PURPOSE_MAX = 110  # characters of a repo's one-line purpose
PAGE_MAX = 65      # characters of a page title plus its description
NESTED_MAX = 200   # description of a nested page in catalog-detail.jsonl
# Entry points Allen sends visitors and agents to keep more of their seo.rs text.
ENTRY_PAGES = {"/agents/": 180, "/start/": 180}
STATUS_MAX = 140  # a repo's own status/evidence statement
HOWTO_MAX = 120   # first line of its install/usage section
SUBJECT_MAX = 90  # HEAD commit subject
PARSER = "p3"     # bump when what at_sha() derives changes; names the cache entries
CACHE = None      # OUTDIR/.cache: per-commit derived data (immutable, so safe to reuse)

# Drake's front door: listed first under "Start here" once it is public with a README.
START = "sovereign-intelligence"
START_MAX = 200
# Described from their own README's first sentence rather than the GitHub description.
FROM_README = (START, "AienOS-nv")

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
AIEN_WORD = re.compile(r"\bAIEN\b")


def aien_case(s):
    """Drake's spelling of the project name in descriptive text (2026-10-26): Aien, never AIEN.
    Only descriptions pass through here; page titles, status lines and commit subjects stay verbatim."""
    return AIEN_WORD.sub("Aien", s)


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



HOWTO_HEAD = re.compile(r"^#{1,4}\s*(install|installation|usage|quick ?start|getting started|how to use|"
                        r"use it|run it|running|build and run|try it)\b", re.I)
STATUS_LINE = re.compile(r"^((?:status|evidence)\b[^:.]{0,40}):\s*(\S.*)$", re.I)
STATUS_HEAD = re.compile(r"^#{1,4}\s*((?:status|evidence)\b.{0,40})$", re.I)
BADGE = re.compile(r"!\[\s*((?:status|evidence)\b[^\]]{0,60})\]", re.I)


def _lines(text):
    """README lines with fence state: yields (line stripped, inside a code fence)."""
    fence = False
    for raw in text.splitlines():
        s = raw.strip()
        if s.startswith(("```", "~~~")):
            fence = not fence
            yield s, None
            continue
        yield s, fence


def _sentence(s):
    m = re.search(r"(.+?[.!?])(\s+[A-Z(\"']|\s*$)", s)
    return m.group(1) if m else s


def stated_status(text):
    """The repo's own status/evidence statement, verbatim (first one found), else ""."""
    lines = list(_lines(text))
    for i, (s, fence) in enumerate(lines):
        if fence is not False:
            continue
        m = STATUS_HEAD.match(s)
        if m:
            para = []
            for t, f in lines[i + 1:]:
                if f is not False or t.startswith(("#", "|")) or (not t and para):
                    break
                if t:
                    para.append(t.lstrip("-*+ "))
            if para:
                label = ascii_text(md_plain(m.group(1)).strip())
                body = _sentence(ascii_text(md_plain(" ".join(para))))
                if not body.lower().startswith(label.split()[0].lower()):
                    body = label + ": " + body
                return clip(body, STATUS_MAX)
            continue
        m = STATUS_LINE.match(ascii_text(md_plain(s)))
        if m and not s.startswith(("|", ">")):
            return clip(m.group(1).strip() + ": " + _sentence(m.group(2)), STATUS_MAX)
    m = BADGE.search(text)
    return clip(ascii_text(m.group(1)), STATUS_MAX) if m else ""


def how_to(text):
    """First line of the README's install/usage section: a command or a sentence, else ""."""
    lines = list(_lines(text))
    for i, (s, fence) in enumerate(lines):
        if fence is not False or not HOWTO_HEAD.match(s):
            continue
        lead = ""
        for t, f in lines[i + 1:]:
            if f is False and t.startswith("#"):
                break
            if not t or f is None:
                continue
            if f:
                if t.startswith("#"):
                    continue  # a shell comment
                return clip((lead + " " if lead else "run: ") + ascii_text(t), HOWTO_MAX)
            if lead:
                return clip(lead, HOWTO_MAX)
            plain = ascii_text(md_plain(t.lstrip("-*+ ")))
            if plain.endswith(":"):
                lead = plain  # an intro to the command below it
            elif plain:
                return clip(_sentence(plain), HOWTO_MAX)
        return clip(lead, HOWTO_MAX)
    return ""


def git(args, cwd=None):
    env = {k: v for k, v in os.environ.items() if not k.startswith(("GIT_", "SSH_"))}
    env.update(GIT_TERMINAL_PROMPT="0", GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
               GIT_ASKPASS="", SSH_ASKPASS="")
    try:
        return subprocess.run(["git", "-c", "credential.helper="] + args, cwd=cwd, env=env,
                              stdin=subprocess.DEVNULL, capture_output=True, text=True,
                              timeout=TIMEOUT * 4)
    except (OSError, subprocess.TimeoutExpired) as e:
        raise FetchError("git %s: %s" % (args[0], e))


def wiki_exists(name):
    """A public wiki answers ls-remote; a missing one asks for credentials (which are disabled)."""
    p = git(["ls-remote", "https://github.com/%s/%s.wiki.git" % (ORG, name), "HEAD"])
    if p.returncode == 0:
        return bool(p.stdout.strip())
    if "could not read Username" in p.stderr or "not found" in p.stderr.lower():
        return False
    raise FetchError("ls-remote %s.wiki: exit %d" % (name, p.returncode))


def head_commit(name, sha):
    """(commit time UTC date, subject, top-level names) of `sha`, by a shallow treeless fetch."""
    with tempfile.TemporaryDirectory(prefix="catalog-git-") as d:
        url = "https://github.com/%s/%s" % (ORG, name)
        for args in (["init", "-q", "--bare", d], ["-C", d, "remote", "add", "origin", url],
                     ["-C", d, "config", "remote.origin.promisor", "true"],
                     ["-C", d, "config", "remote.origin.partialclonefilter", "tree:0"],
                     ["-C", d, "fetch", "-q", "--depth", "1", "--filter=tree:0", "origin", sha]):
            p = git(args)
            if p.returncode != 0:
                raise FetchError("git %s for %s: exit %d" % (args[2] if args[0] == "-C" else args[0],
                                                             name, p.returncode))
        log = git(["-C", d, "log", "-1", "--format=%ct%x00%s", sha])
        tree = git(["-C", d, "ls-tree", "--name-only", sha])
        if log.returncode != 0 or tree.returncode != 0 or "\x00" not in log.stdout:
            raise FetchError("git log/ls-tree %s: failed" % name)
    ts, subject = log.stdout.rstrip("\n").split("\x00", 1)
    day = datetime.datetime.fromtimestamp(int(ts), datetime.timezone.utc).strftime("%Y-%m-%d")
    return day, subject, sorted(tree.stdout.split())


def at_sha(name, sha):
    """Everything derived from one commit (immutable, so cached by sha and parser version)."""
    path = os.path.join(CACHE, "%s-%s.json" % (PARSER, sha)) if CACHE else None
    if path and os.path.exists(path):
        with open(path, encoding="ascii") as f:
            return json.load(f)
    readme, _ = get(RAW % (ORG, name, sha, "README.md"), allow_404=True)
    day, subject, top = head_commit(name, sha)
    text = readme.decode("utf-8", "replace") if readme else ""
    status = stated_status(text)
    if not status and "STATUS.md" in top:
        body, _ = get(RAW % (ORG, name, sha, "STATUS.md"), allow_404=True)
        status = stated_status(body.decode("utf-8", "replace")) if body else ""
    info = {"readme": hashlib.sha256(readme or b"").hexdigest() if readme else "",
            "sentence": first_sentence(readme) if readme else "",
            "status": status, "how_to": how_to(text),
            "commit_date": day, "subject": clip(ascii_text(subject), SUBJECT_MAX),
            "docs_dir": "docs" in top, "status_md": "STATUS.md" in top}
    if path:
        tmp = path + ".tmp"
        with open(tmp, "w", encoding="ascii") as f:
            json.dump(info, f, sort_keys=True)
        os.replace(tmp, path)
    return info


def repo_record(r):
    name = r["name"]
    sha = head_sha(name)
    info = at_sha(name, sha) if sha else {}
    sentence = info.get("sentence", "")
    desc = ascii_text(r.get("description") or "")
    lic = (r.get("license") or {}).get("spdx_id") or "none"
    if lic == "NOASSERTION":
        lic = "other"
    if name in FROM_README:
        purpose = clip(sentence, START_MAX if name == START else PURPOSE_MAX) or clip(desc, PURPOSE_MAX)
    else:
        purpose = clip(desc or sentence, PURPOSE_MAX)
    purpose = aien_case(purpose)
    url = "https://github.com/%s/%s" % (ORG, name)
    branch = r.get("default_branch") or "main"
    docs = []
    if info.get("readme"):
        docs.append(url + "#readme")
    if info.get("docs_dir"):
        docs.append("%s/tree/%s/docs" % (url, branch))
    if info.get("status_md"):
        docs.append("%s/blob/%s/STATUS.md" % (url, branch))
    if r.get("has_wiki") and wiki_exists(name):
        docs.append(url + "/wiki")
    detail = {"name": name, "url": url, "purpose": purpose, "license": lic,
              "state": "archived" if r.get("archived") else "active",
              "last_push": (r.get("pushed_at") or "")[:10], "head": sha or "empty",
              "evidence": info.get("status") or "not stated", "docs": docs}
    if info.get("how_to"):
        detail["how_to"] = info["how_to"]
    if sha:
        detail["latest"] = {"date": info["commit_date"], "subject": info["subject"]}
    return {"name": name, "sha": sha, "license": lic, "pushed": (r.get("pushed_at") or "")[:7],
            "purpose": purpose, "from_readme": bool(sentence) and name in FROM_README,
            "archived": bool(r.get("archived")), "detail": detail}


def registry_pages(body):
    """om/site/pages.json: [{path, title, description, area}, ...] (the Omega site's registry)."""
    try:
        entries = json.loads(body)
    except ValueError:
        raise FetchError("%s: not JSON" % REGISTRY_PATH)
    if not isinstance(entries, list) or not entries:
        raise FetchError("%s: not a non-empty array" % REGISTRY_PATH)
    out = {}
    for e in entries:
        if not isinstance(e, dict) or not all(isinstance(e.get(k), str) for k in ("path", "title", "description")):
            raise FetchError("%s: entry without string path, title and description" % REGISTRY_PATH)
        if not re.match(r"^/([a-z0-9._-]+/)*$", e["path"]) or e["path"] in out:
            raise FetchError("%s: bad or repeated path %r" % (REGISTRY_PATH, e["path"]))
        out[e["path"]] = (e["title"], e["description"])
    return out


def seo_pages(body):
    """crates/site-builder/src/seo.rs PAGES table (the Rust site builder)."""
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
    return {unesc(p): (unesc(n), unesc(w)) for p, n, w in pages}


def site_pages(sha):
    """The page list at aienos.com `sha`: the Omega registry once it exists, else seo.rs.
    Returns (pages, source label, digest of the file read)."""
    body, _ = get(RAW % (ORG, SITE_REPO, sha, REGISTRY_PATH), allow_404=True)
    if body is not None:
        out, label = registry_pages(body), "pages.json"
    else:
        body, _ = get(RAW % (ORG, SITE_REPO, sha, SEO_PATH))
        out, label = seo_pages(body), "seo.rs"
    out.setdefault(ALLEN_PAGE[0], ALLEN_PAGE[1:])
    return out, label, hashlib.sha256(body).hexdigest()


def build():
    repos = list_repos()
    own = sorted((r for r in repos if not r.get("fork")), key=lambda r: r["name"].lower())
    forks = sorted((r["name"] for r in repos if r.get("fork")), key=str.lower)
    with concurrent.futures.ThreadPoolExecutor(8) as pool:
        everything = list(pool.map(repo_record, own))
        site_sha = pool.submit(head_sha, SITE_REPO).result()
    if not site_sha:
        raise FetchError("%s has no HEAD" % SITE_REPO)
    pages, page_src, page_digest = site_pages(site_sha)
    records = [rec for rec in everything if not rec["archived"]]
    archived = [rec["name"] for rec in everything if rec["archived"]]

    start = next((rec for rec in records if rec["name"] == START), None)
    records = [rec for rec in records if rec["name"] != START]
    if not (start and start["from_readme"]):
        # not public, empty, or no README yet: nothing is said about it
        print("catalog: %s %s; no Start here entry" % (START, "has no README yet" if start else "is not public"),
              file=sys.stderr)
        start = None

    months = [rec["pushed"] for rec in records]
    usual = max(sorted(set(months)), key=months.count)  # most common last-push month, latest on a tie

    # catalog.txt is the cached prompt prefix: it holds only what the model reads, so a push
    # (new head sha) does not rewrite it. Heads and per-repo detail live in catalog-detail.jsonl.
    L = []
    L.append("Drake's public repos and site pages; nothing else of his is public. Name them and cite "
             "only these URLs; if nothing fits, say Drake has nothing public for it.")
    L.append("Sources: github.com/%s public repos and %s %s, read unauthenticated; heads in catalog-detail.jsonl."
             % (ORG, SITE_REPO, page_src))
    L.append("")
    if start:
        L.append("Start here:")
        L.append("%s: %s (%s) https://github.com/%s/%s" % (start["name"], start["purpose"], start["license"],
                                                          ORG, start["name"]))
        L.append("")
    L.append("Active repos (URL: https://github.com/%s/ then the name; last push %s unless noted). "
             "name: purpose (license):" % (ORG, usual))
    for rec in records:
        meta = [rec["license"]]
        if rec["pushed"] != usual:
            meta.append("last push " + rec["pushed"])
        L.append("%s: %s (%s)" % (rec["name"], rec["purpose"] or "no description", ", ".join(meta)))
    L.append("Archived: %s. Forks of outside projects: %s." % (", ".join(archived), ", ".join(forks)))
    L.append("")
    L.append("Pages on Drake's site (URL, title: what). These are pages, not repos:")
    nested_pages = []
    for path in sorted(pages):
        name, what = pages[path]
        name = ascii_text(name)
        what = ascii_text(re.sub(r"\s*\([^)]*\)", "", what))
        if path.count("/") > 2 and path not in ENTRY_PAGES:
            # nested pages (/engineer/works/x/, /research/x/) go to retrieval: the prefix stays compact
            # state/last_push/evidence/docs keep the repo-record shape, so a reader that predates
            # page records (a gateway with an older retrieve.py loaded) cannot fail on them
            nested_pages.append({"name": path, "kind": "page", "url": SITE + path, "title": name,
                                 "purpose": aien_case(clip(what, NESTED_MAX)), "state": "page",
                                 "last_push": "-", "evidence": "not stated", "docs": []})
            continue
        room = (ENTRY_PAGES.get(path) or PAGE_MAX) - len(name)
        what = aien_case(clip(what, room)) if room >= 30 else ""
        L.append("%s%s %s%s" % (SITE, path, name, ": " + what if what else ""))
    parents = sorted({"/" + p["name"].split("/")[1] + "/" for p in nested_pages})
    if parents:
        L.append("More pages under %s are looked up per question; cite only URLs given to you."
                 % ", ".join(SITE + p for p in parents))
    L.append("")
    L.append(OMEGA_BRIEF)
    L.append(REACH)
    catalog = "\n".join(L) + "\n"
    catalog.encode("ascii")  # raises if anything non-ASCII slipped through
    areas = "".join(rec["name"] + "\n" for rec in everything if not rec["archived"])

    head = {"name": "_sources", "org": ORG, "site_repo": SITE_REPO, "site_head": site_sha,
            "page_source": page_src, "page_source_sha256": page_digest, "forks": forks}
    detail = "".join(json.dumps(d, sort_keys=True, ensure_ascii=True) + "\n"
                     for d in [head] + [rec["detail"] for rec in everything] + nested_pages)
    return catalog, areas, detail


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
    global CACHE
    if len(sys.argv) != 2 or not os.path.isdir(sys.argv[1]):
        sys.exit("usage: catalog.py OUTDIR")
    outdir = sys.argv[1]
    CACHE = os.path.join(outdir, ".cache")
    os.makedirs(CACHE, mode=0o755, exist_ok=True)
    try:
        catalog, areas, detail = build()
    except (FetchError, UnicodeError, KeyError, TypeError, ValueError) as e:
        print("catalog: not written: %s" % e, file=sys.stderr)
        sys.exit(1)
    files = (("areas.txt", areas), ("catalog-detail.jsonl", detail), ("catalog.txt", catalog))
    changed = [(n, t) for n, t in files if not same(outdir, n, t)]
    if changed:
        # an untouched file keeps its mtime, so the gateway keeps its warm prefix
        write_atomic(outdir, changed)
    heads = {json.loads(l).get("head") for l in detail.splitlines()}
    for f in os.listdir(CACHE):  # keep only the entries for current heads and parser
        if not (f.startswith(PARSER + "-") and f[len(PARSER) + 1:-5] in heads):
            os.unlink(os.path.join(CACHE, f))
    print("catalog: %s; catalog.txt %d bytes, %d areas, %d detail records"
          % ("wrote " + " ".join(n for n, _ in changed) if changed else "unchanged",
             len(catalog), areas.count("\n"), detail.count("\n") - 1))


if __name__ == "__main__":
    main()
