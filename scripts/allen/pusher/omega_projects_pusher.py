#!/usr/bin/python3
"""omega-projects-pusher: files visitor-confirmed Allen spool items on aien-dev/omega-projects.

The public gateway (allen-chat) holds no credential; it only drops validated items into
spool-in. This program, a separate system user (omega-pusher), is the only holder of
GitHub write ability. For every spool-in/<id>.json it re-validates the item, checks the
id, scrubs emails and phone numbers, enforces daily caps, dedupes, then files exactly one
issue or exactly one pull request from a submissions/ branch. It never touches main and
never merges. The result goes to spool-status/<id>.json (atomic, 0640); the spool file is
then deleted.

Credential: a fine-grained token (github_pat_...) scoped to aien-dev/omega-projects, given
by systemd as the encrypted credential "omega-projects-token". Without it, or with any
other kind of token (a gh login is gho_/ghp_), nothing is filed: valid items stay in the
spool with status queued, reason "awaiting activation".

Python stdlib only. allen_spool.py is imported from this file's directory (installed
copy) or from --spool-module-dir (tests).
"""

import argparse
import base64
import datetime
import fcntl
import json
import os
import re
import stat
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

REPO = "aien-dev/omega-projects"
API = "https://api.github.com"
# Tests may point the client at a local stub; nothing else is accepted.
STUB_API = re.compile(r"^http://127\.0\.0\.1:[0-9]{1,5}$")
FINE_GRAINED = re.compile(r"^github_pat_[A-Za-z0-9_]{20,250}$")
AWAITING = "awaiting activation"
REPO_PREFIX = "/repos/" + REPO + "/"
MAX_ISSUES_PER_DAY = 40
MAX_PRS_PER_DAY = 20
MAX_ATTEMPTS = 5
MAX_ITEM_BYTES = 512 * 1024
FOOTER = "Filed by Allen from aienos.com/allen/ after the visitor confirmed this exact text."
TITLE_PREFIX = {"feature": "[feature] ", "question": "[question] ", "bug": "[bug] "}
ID_FILE = re.compile(r"^([0-9a-f]{20})\.json$")

spool = None  # allen_spool module, bound in main()


class Refused(Exception):
    """Permanent: the item will never be filed (status rejected)."""


class Transient(Exception):
    """Retry later: GitHub unreachable, rate limited or 5xx."""


def log(msg):
    print(msg, file=sys.stderr, flush=True)


# ---------------------------------------------------------------- GitHub client

class _NoRedirect(urllib.request.HTTPRedirectHandler):
    """A redirect would carry the token to another URL; treat it as an error instead."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class GitHub:
    """Talks to api.github.com for aien-dev/omega-projects only."""

    def __init__(self, token, api=API):
        if not FINE_GRAINED.match(token or ""):
            raise ValueError("not a fine-grained token")
        if api != API and not STUB_API.match(api):
            raise ValueError("refusing API base %r" % api)
        self._token = token
        self._api = api
        self._open = urllib.request.build_opener(_NoRedirect())

    def call(self, method, path, body=None):
        _check_request(method, path, body)
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(self._api + path, data=data, method=method)
        req.add_header("Authorization", "Bearer " + self._token)
        req.add_header("Accept", "application/vnd.github+json")
        req.add_header("X-GitHub-Api-Version", "2022-11-28")
        req.add_header("User-Agent", "omega-projects-pusher")
        if data is not None:
            req.add_header("Content-Type", "application/json")
        try:
            with self._open.open(req, timeout=30) as r:
                status, raw = r.status, r.read()
        except urllib.error.HTTPError as e:
            status, raw = e.code, e.read()
        except (urllib.error.URLError, OSError, TimeoutError) as e:
            raise Transient("GitHub unreachable (%s)" % type(e).__name__)
        if status in (200, 201):
            return json.loads(raw) if raw else None
        msg = ""
        try:
            msg = str(json.loads(raw).get("message", ""))[:200]
        except ValueError:
            pass
        if status == 429 or status >= 500 or (status == 403 and "rate limit" in msg.lower()):
            raise Transient("GitHub HTTP %d" % status)
        raise HTTPRefused(status, msg)


class HTTPRefused(Exception):
    def __init__(self, status, msg):
        super().__init__("GitHub HTTP %d: %s" % (status, msg))
        self.status = status


SAFE_PATH = re.compile(r"^[A-Za-z0-9/_.:%?&=,+-]*$")


def _check_request(method, path, body):
    """Refuse anything outside the one repository and the handful of calls the pusher needs."""
    if not path.startswith(REPO_PREFIX) or ".." in path or "//" in path or not SAFE_PATH.match(path):
        raise PermissionError("refusing request outside %s: %s" % (REPO, path))
    rest = path[len(REPO_PREFIX):].split("?", 1)[0]
    allowed = {
        "GET": (r"git/ref/heads/main", r"git/ref/heads/submissions/[a-z0-9-]+", r"git/commits/[0-9a-f]{40}",
                r"pulls", r"issues", r"labels/[A-Za-z0-9:._%-]+"),
        "POST": (r"issues", r"issues/[0-9]+/labels", r"pulls", r"labels", r"git/blobs", r"git/trees",
                 r"git/commits", r"git/refs"),
    }.get(method, ())
    if not any(re.fullmatch(p, rest) for p in allowed):
        raise PermissionError("refusing %s %s" % (method, rest))
    if method == "POST" and rest == "git/refs":
        ref = (body or {}).get("ref", "")
        if not re.fullmatch(r"refs/heads/submissions/[a-z0-9-]+", ref):
            raise PermissionError("refusing to create ref %r" % ref)
    if method == "POST" and rest == "pulls":
        if (body or {}).get("base") != "main" or not str((body or {}).get("head", "")).startswith("submissions/"):
            raise PermissionError("refusing pull request shape")


# ---------------------------------------------------------------- state

class State:
    """Persisted counters, ledger of filed ids, in-flight markers and attempt counts."""

    def __init__(self, path):
        self.path = path
        self.file = os.path.join(path, "state.json")
        try:
            with open(self.file, encoding="utf-8") as f:
                self.d = json.load(f)
        except FileNotFoundError:
            self.d = {}
        for k in ("filed", "pending", "attempts"):
            self.d.setdefault(k, {})
        self.d.setdefault("day", "")
        self.d.setdefault("issues", 0)
        self.d.setdefault("prs", 0)

    def save(self):
        atomic_write(self.file, self.d)

    def take(self, kind, now=None):
        day = (now or datetime.datetime.now(datetime.timezone.utc)).strftime("%Y-%m-%d")
        if self.d["day"] != day:
            self.d.update(day=day, issues=0, prs=0)
        key, cap = ("issues", MAX_ISSUES_PER_DAY) if kind == "issue" else ("prs", MAX_PRS_PER_DAY)
        if self.d[key] >= cap:
            return False
        self.d[key] += 1
        self.save()
        return True


def atomic_write(path, obj, mode=0o600):
    tmp = "%s.tmp-%d" % (path, os.getpid())
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    os.fchmod(fd, mode)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, sort_keys=True)
        f.flush()
        os.fsync(f.fileno())
    os.rename(tmp, path)


def write_status(status_dir, iid, state, url=None, reason=None):
    # 0640: the gateway reads status through the allen-spool group.
    atomic_write(os.path.join(status_dir, iid + ".json"),
                 {"id": iid, "state": state, "url": url, "reason": reason}, mode=0o640)


def read_status_file(status_dir, iid):
    try:
        with open(os.path.join(status_dir, iid + ".json"), encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


# ---------------------------------------------------------------- item checks

def load_areas(path):
    """Known public repository names. Without the file every area becomes 'unknown'."""
    try:
        with open(path, encoding="utf-8") as f:
            return {ln.strip() for ln in f if ln.strip()}
    except FileNotFoundError:
        return set()


def read_item(path):
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except OSError:
        raise Refused("the item is not a regular file")
    with os.fdopen(fd, "rb") as f:
        info = os.fstat(f.fileno())
        if not stat.S_ISREG(info.st_mode):
            raise Refused("the item is not a regular file")
        if info.st_size > MAX_ITEM_BYTES:
            raise Refused("the item is too large")
        raw = f.read(MAX_ITEM_BYTES + 1)
    if len(raw) > MAX_ITEM_BYTES:
        raise Refused("the item is too large")
    try:
        return json.loads(raw.decode("utf-8", errors="strict"))
    except (UnicodeDecodeError, ValueError):
        raise Refused("the item is not valid UTF-8 JSON")


def check_item(raw, iid, areas):
    """Return the canonical, scrubbed item or raise Refused."""
    if spool.item_id(raw) != iid:
        raise Refused("the item id does not match its content")
    try:
        item = spool.validate(raw, areas=areas)
    except spool.Invalid as e:
        raise Refused(str(e))
    if item["kind"] == "issue":
        item["title"] = spool.scrub(item["title"])
        item["body"] = spool.scrub(item["body"])
    else:
        item["title"] = spool.scrub(item["title"])
        item["description"] = spool.scrub(item["description"])
        for f in item["files"]:
            # Prose only. index.html and .om are code: any substitution could corrupt them,
            # and Drake reviews the pull request before anything goes live.
            if f["name"] == "README.md":
                f["content"] = spool.scrub(f["content"])
    return item


# ---------------------------------------------------------------- filing

def marker(iid):
    return "<!-- allen-item: %s -->" % iid


def ensure_label(gh, name, cache):
    if name in cache:
        return
    try:
        gh.call("GET", REPO_PREFIX + "labels/" + urllib.parse.quote(name, safe=""))
    except HTTPRefused as e:
        if e.status != 404:
            raise
        try:
            gh.call("POST", REPO_PREFIX + "labels",
                    {"name": name, "color": "c5def5", "description": "Concerns aien-dev/" + name[5:]})
        except HTTPRefused as e2:
            if e2.status != 422:  # 422: created meanwhile
                raise
    cache.add(name)


def find_issue(gh, iid):
    """Recover an issue filed before a crash: search recent from-allen issues for the id marker."""
    found = gh.call("GET", REPO_PREFIX + "issues?labels=from-allen&state=all&per_page=100&sort=created&direction=desc")
    for it in found or []:
        if marker(iid) in (it.get("body") or "") and "pull_request" not in it:
            return it["html_url"]
    return None


def file_issue(gh, item, iid, extra_labels, label_cache):
    area_label = "area:" + item["area"]
    labels = ["from-allen", area_label] + list(extra_labels)
    for name in labels:
        ensure_label(gh, name, label_cache)
    body = "%s\n\n---\n%s\n%s\n" % (item["body"].rstrip(), FOOTER, marker(iid))
    r = gh.call("POST", REPO_PREFIX + "issues",
                {"title": TITLE_PREFIX[item["template"]] + item["title"], "body": body, "labels": labels})
    return r["html_url"]


def branch_for(item, iid):
    return "submissions/%s-%s" % (item["slug"], iid[:8])


def find_pull(gh, branch):
    found = gh.call("GET", REPO_PREFIX + "pulls?state=all&per_page=10&head=" +
                    urllib.parse.quote("aien-dev:" + branch, safe=""))
    return found[0]["html_url"] if found else None


def file_pull(gh, item, iid, extra_labels, label_cache, today):
    branch = branch_for(item, iid)
    existing = find_pull(gh, branch)
    if existing:
        return existing
    try:
        head = gh.call("GET", REPO_PREFIX + "git/ref/heads/" + branch)["object"]["sha"]
    except HTTPRefused as e:
        if e.status != 404:
            raise
        head = None
    if head is None:
        main = gh.call("GET", REPO_PREFIX + "git/ref/heads/main")["object"]["sha"]
        meta = {"slug": item["slug"], "title": item["title"], "description": item["description"],
                "submitted": today, "item": iid}
        files = [(f["name"], f["content"]) for f in item["files"]]
        files.append(("meta.json", json.dumps(meta, ensure_ascii=False, indent=2, sort_keys=True) + "\n"))
        tree = []
        for name, content in files:
            blob = gh.call("POST", REPO_PREFIX + "git/blobs",
                           {"content": base64.b64encode(content.encode()).decode(), "encoding": "base64"})
            tree.append({"path": "projects/%s/%s" % (item["slug"], name), "mode": "100644",
                         "type": "blob", "sha": blob["sha"]})
        main_tree = gh.call("GET", REPO_PREFIX + "git/commits/" + main)["tree"]["sha"]
        t = gh.call("POST", REPO_PREFIX + "git/trees", {"base_tree": main_tree, "tree": tree})
        c = gh.call("POST", REPO_PREFIX + "git/commits",
                    {"message": "Submit project %s from Allen\n\nAllen item %s" % (item["slug"], iid),
                     "tree": t["sha"], "parents": [main]})
        gh.call("POST", REPO_PREFIX + "git/refs", {"ref": "refs/heads/" + branch, "sha": c["sha"]})
    names = "\n".join("- `projects/%s/%s`" % (item["slug"], f["name"]) for f in item["files"])
    body = ("%s\n\n%s\n\nFiles:\n%s\n- `projects/%s/meta.json`\n\n"
            "Drake reviews this project before it goes live; merging publishes it at "
            "https://aien-dev.github.io/omega-projects/projects/%s/\n\n---\n%s\n%s\n") % (
        item["title"], item["description"].rstrip(), names, item["slug"], item["slug"], FOOTER, marker(iid))
    pr = gh.call("POST", REPO_PREFIX + "pulls",
                 {"title": "Project: " + item["title"], "head": branch, "base": "main", "body": body,
                  "maintainer_can_modify": True})
    labels = ["from-allen"] + list(extra_labels)
    for name in labels:
        ensure_label(gh, name, label_cache)
    gh.call("POST", REPO_PREFIX + "issues/%d/labels" % pr["number"], {"labels": labels})
    return pr["html_url"]


# ---------------------------------------------------------------- driver

def process(args, gh):
    spool_in, status_dir = args.spool_in, args.spool_status
    st = State(args.state_dir)
    areas = load_areas(args.areas)
    label_cache = set()
    extra = ["test"] if args.test_label else []
    today = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    transient = waiting = 0
    started = time.monotonic()
    for name in sorted(os.listdir(spool_in)):
        if name.startswith(".tmp-"):
            continue
        if time.monotonic() - started > getattr(args, "budget", 75.0):
            # TimeoutStartSec=120 would kill a long run mid-item; the rest go next run.
            log("time budget used; remaining items go to the next run")
            break
        path = os.path.join(spool_in, name)
        m = ID_FILE.match(name)
        if not m:
            log("removing unexpected spool entry %r" % name)
            _remove(path)
            continue
        iid = m.group(1)
        prior = read_status_file(status_dir, iid)
        if iid in st.d["filed"] or (prior and prior.get("state") == "filed"):
            url = st.d["filed"].get(iid) or prior.get("url")
            write_status(status_dir, iid, "filed", url=url)
            _remove(path)
            log("%s: already filed (%s); nothing filed" % (iid, url))
            continue
        try:
            item = check_item(read_item(path), iid, areas)
            kind = item["kind"]
            if gh is None:
                # Not activated: keep the item, tell the visitor it is queued.
                if not (prior and prior.get("state") == "queued" and prior.get("reason") == AWAITING):
                    write_status(status_dir, iid, "queued", reason=AWAITING)
                    log("%s: queued (%s)" % (iid, AWAITING))
                waiting += 1
                continue
            url = None
            if st.d["pending"].get(iid) == "issue":
                url = find_issue(gh, iid)
            if url is None:
                if not st.take(kind):
                    raise Refused("Allen has reached today's limit for %s; please try again tomorrow"
                                  % ("issues" if kind == "issue" else "project submissions"))
                st.d["pending"][iid] = kind
                st.save()
                if kind == "issue":
                    url = file_issue(gh, item, iid, extra, label_cache)
                else:
                    url = file_pull(gh, item, iid, extra, label_cache, today)
            st.d["filed"][iid] = url
            st.d["pending"].pop(iid, None)
            st.d["attempts"].pop(iid, None)
            st.save()
            write_status(status_dir, iid, "filed", url=url)
            _remove(path)
            log("%s: filed %s" % (iid, url))
        except Refused as e:
            _reject(st, status_dir, path, iid, str(e))
        except HTTPRefused as e:
            _reject(st, status_dir, path, iid, "GitHub refused the item (HTTP %d)" % e.status)
            log("%s: %s" % (iid, e))
        except PermissionError as e:
            # The request guard refused a call: a bug, never retried.
            _reject(st, status_dir, path, iid, "the pusher refused its own request")
            log("%s: guard: %s" % (iid, e))
        except (Transient, Exception) as e:
            if not isinstance(e, Transient):
                e = "%s: %s" % (type(e).__name__, str(e)[:200])
            n = st.d["attempts"].get(iid, 0) + 1
            st.d["attempts"][iid] = n
            st.save()
            log("%s: attempt %d failed: %s" % (iid, n, e))
            if n >= MAX_ATTEMPTS:
                _reject(st, status_dir, path, iid, "GitHub could not be reached; please submit again later")
            else:
                transient += 1
    return transient, waiting


def _reject(st, status_dir, path, iid, reason):
    st.d["pending"].pop(iid, None)
    st.d["attempts"].pop(iid, None)
    st.save()
    write_status(status_dir, iid, "rejected", reason=reason)
    _remove(path)
    log("%s: rejected: %s" % (iid, reason))


def _remove(path):
    try:
        os.unlink(path)
    except FileNotFoundError:
        pass


def load_token(path):
    """The fine-grained token, or None. Never falls back to any other credential."""
    if not path:
        return None
    try:
        with open(path, encoding="ascii") as f:
            token = f.read().strip()
    except (OSError, UnicodeDecodeError):
        return None
    if not FINE_GRAINED.match(token):
        log("credential is not a fine-grained github_pat_ token; refusing it")
        return None
    return token


def main(argv=None):
    creds = os.environ.get("CREDENTIALS_DIRECTORY")
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--spool-in", default="/var/lib/allen-chat/spool-in")
    p.add_argument("--spool-status", default="/var/lib/allen-chat/spool-status")
    p.add_argument("--state-dir", default="/var/lib/omega-pusher")
    p.add_argument("--areas", default="/var/lib/allen-catalog/areas.txt")
    p.add_argument("--token-file", default=os.path.join(creds, "omega-projects-token") if creds else None)
    p.add_argument("--api-base", default=API, help="tests only: http://127.0.0.1:PORT stub")
    p.add_argument("--spool-module-dir", default=os.path.dirname(os.path.abspath(__file__)))
    p.add_argument("--test-label", action="store_true", help="add the 'test' label (tests only)")
    p.add_argument("--retry-delay", type=float, default=30.0,
                   help="seconds to wait before exiting when items remain after transient errors")
    p.add_argument("--idle-delay", type=float, default=100.0,
                   help="seconds to wait before exiting when items wait for activation")
    args = p.parse_args(argv)
    global spool
    sys.path.insert(0, args.spool_module_dir)
    import allen_spool
    spool = allen_spool
    os.makedirs(args.state_dir, mode=0o700, exist_ok=True)
    lock = open(os.path.join(args.state_dir, "lock"), "w")
    fcntl.flock(lock, fcntl.LOCK_EX)
    token = load_token(args.token_file)
    gh = GitHub(token, api=args.api_base) if token else None
    if gh is None:
        log("not activated: no omega-projects-token credential; items stay queued")
    transient, waiting = process(args, gh)
    # The path unit re-triggers while spool-in holds items; pace the next run.
    if waiting:
        time.sleep(args.idle_delay)
    elif transient:
        time.sleep(args.retry_delay)
    return 0


if __name__ == "__main__":
    sys.exit(main())
