#!/usr/bin/python3
"""omega-projects-pusher: files visitor-confirmed Allen spool items on aien-dev/omega-projects.

The public gateway (allen-chat) holds no credential; it only drops validated items into
spool-in. This program, a separate system user (omega-pusher), is the only holder of
GitHub write ability. For every spool-in/<id>.json it re-validates the item, checks the
id, scrubs emails and phone numbers, enforces daily caps, dedupes, then acts once: an
issue; a save (one commit on u/<handle>); a publish (one commit on p/<handle>/<project>
and one pull request to main, reused while open); a decline from Drake's console (comment
and close that pull request); or a refresh. Files land only under
projects/<handle>/<project>/ (allen_build.py). It never writes main and never merges.
After build items it rewrites spool-status/index.json (open publish requests, gallery).
The result goes to spool-status/<id>.json (atomic, 0640); the spool file is then deleted.

Credential: a fine-grained token (github_pat_...) scoped to aien-dev/omega-projects, given
by systemd as the encrypted credential "omega-projects-token". Without it, or with any
other kind of token (a gh login is gho_/ghp_), nothing is filed: valid items stay in the
spool with status queued, reason "awaiting activation".

Python stdlib only. allen_spool.py and allen_build.py are imported from this file's
directory (installed copy) or from --spool-module-dir (tests).
"""

import argparse
import base64
import datetime
import fcntl
import json
import os
import re
import socket
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
BACKOFF = (10.0, 20.0, 40.0, 80.0)  # seconds before attempts 2..5; attempts = len + 1
REQUEST_TIMEOUT = 10
RUN_BUDGET = 100.0  # seconds per run; the unit has TimeoutStartSec=120
MAX_ITEM_BYTES = 512 * 1024
FOOTER = "Filed by Allen from aienos.com/allen/ after the visitor confirmed this exact text."
TITLE_PREFIX = {"feature": "[feature] ", "question": "[question] ", "bug": "[bug] "}
# Daily caps: kind -> (state key, cap). Decline and refresh come only from Drake's console.
CAPS = {"issue": ("issues", 40), "publish": ("prs", 20), "save": ("saves", 200)}
ID_FILE = re.compile(r"^([0-9a-f]{20})\.json$")

spool = None  # allen_spool module, bound in main()
build = None  # allen_build module, bound in main()


class Refused(Exception):
    """The content broke a rule: status rejected, never retried."""


class GitHubError(Exception):
    """GitHub did not take a request. `cause` is the honest text shown to the visitor.
    Retried with backoff; after the last attempt the item's status becomes failed."""

    def __init__(self, cause, status=None):
        super().__init__(cause)
        self.cause = cause
        self.status = status


class HTTPRefused(GitHubError):
    def __init__(self, status, msg=""):
        if status == 401:
            cause = "GitHub refused the credential (HTTP 401)"
        elif status == 429 or (status == 403 and "rate limit" in msg.lower()):
            cause = "GitHub rate limit (HTTP %d)" % status
        elif status >= 500:
            cause = "GitHub server error (HTTP %d)" % status
        else:
            cause = "GitHub refused the request (HTTP %d)" % status
        super().__init__(cause, status)


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
            with self._open.open(req, timeout=REQUEST_TIMEOUT) as r:
                status, raw = r.status, r.read()
        except urllib.error.HTTPError as e:
            status, raw = e.code, e.read()
        except (TimeoutError, socket.timeout):
            raise GitHubError("GitHub did not answer within %d s" % REQUEST_TIMEOUT)
        except urllib.error.URLError as e:
            if isinstance(e.reason, (TimeoutError, socket.timeout)):
                raise GitHubError("GitHub did not answer within %d s" % REQUEST_TIMEOUT)
            raise GitHubError("GitHub could not be reached")
        except OSError:
            raise GitHubError("GitHub could not be reached")
        if status in (200, 201):
            try:
                return json.loads(raw) if raw else None
            except ValueError:
                raise GitHubError("GitHub sent an unreadable answer")
        msg = ""
        try:
            msg = str(json.loads(raw).get("message", ""))[:200]
        except (ValueError, AttributeError):
            pass
        raise HTTPRefused(status, msg)


class GuardRefused(PermissionError):
    """The request guard refused a call outside the allowed shape."""


SAFE_PATH = re.compile(r"^[A-Za-z0-9/_.:%?&=,+-]*$")


BUILD_REF = r"(?:u/[a-z0-9-]+|p/[a-z0-9-]+/[a-z0-9-]+)"
TREE_PATH = re.compile(r"^projects/[a-z0-9-]+/[a-z0-9-]+/[A-Za-z0-9_.-]+$")


def _check_request(method, path, body):
    """Refuse anything outside the one repository and the handful of calls the pusher needs.
    No call can write main: refs are created or moved only under u/ and p/, never forced, and
    trees only carry files under projects/<handle>/<project>/."""
    if not path.startswith(REPO_PREFIX) or ".." in path or "//" in path or not SAFE_PATH.match(path):
        raise GuardRefused("refusing request outside %s: %s" % (REPO, path))
    rest = path[len(REPO_PREFIX):].split("?", 1)[0]
    allowed = {
        "GET": (r"git/ref/heads/main", r"git/ref/heads/" + BUILD_REF, r"git/commits/[0-9a-f]{40}",
                r"git/trees/[0-9a-f]{40}", r"git/blobs/[0-9a-f]{40}", r"pulls", r"pulls/[0-9]+", r"issues",
                r"labels/[A-Za-z0-9:._%-]+"),
        "POST": (r"issues", r"issues/[0-9]+/labels", r"issues/[0-9]+/comments", r"pulls", r"labels",
                 r"git/blobs", r"git/trees", r"git/commits", r"git/refs"),
        "PATCH": (r"git/refs/heads/" + BUILD_REF, r"pulls/[0-9]+"),
    }.get(method, ())
    if not any(re.fullmatch(p, rest) for p in allowed):
        raise GuardRefused("refusing %s %s" % (method, rest))
    body = body or {}
    if method == "POST" and rest == "git/refs":
        if not re.fullmatch(r"refs/heads/" + BUILD_REF, body.get("ref", "")):
            raise GuardRefused("refusing to create ref %r" % body.get("ref"))
    if method == "PATCH" and rest.startswith("git/refs/") and body.get("force") is not False:
        raise GuardRefused("refusing a forced ref update")
    if method == "PATCH" and rest.startswith("pulls/") and body != {"state": "closed"}:
        raise GuardRefused("refusing pull request change %r" % sorted(body))
    if method == "POST" and rest == "git/trees":
        if not all(isinstance(e, dict) and TREE_PATH.match(str(e.get("path", ""))) for e in body.get("tree", [])):
            raise GuardRefused("refusing a tree entry outside projects/<handle>/<project>/")
    if method == "POST" and rest == "pulls":
        if body.get("base") != "main" or not re.fullmatch(r"p/[a-z0-9-]+/[a-z0-9-]+", str(body.get("head", ""))):
            raise GuardRefused("refusing pull request shape")


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
        for k in CAPS.values():
            self.d.setdefault(k[0], 0)

    def save(self):
        atomic_write(self.file, self.d)

    def take(self, kind, now=None):
        """One slot of today's cap for kind; decline and refresh have none."""
        if kind not in CAPS:
            return True
        day = (now or datetime.datetime.now(datetime.timezone.utc)).strftime("%Y-%m-%d")
        if self.d["day"] != day:
            self.d.update({"day": day}, **{k: 0 for k, _ in CAPS.values()})
        key, cap = CAPS[kind]
        if self.d[key] >= cap:
            return False
        self.d[key] += 1
        self.save()
        return True

    def give(self, kind):
        """Return a slot taken by an item that was never filed."""
        if kind in CAPS and self.d[CAPS[kind][0]] > 0:
            self.d[CAPS[kind][0]] -= 1


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
        if isinstance(raw, dict) and raw.get("kind") == "issue":
            item = spool.validate(raw, areas=areas)
            item["title"] = spool.scrub(item["title"])
            item["body"] = spool.scrub(item["body"])
            return item
        item = build.validate(raw)
    except spool.Invalid as e:
        raise Refused(str(e))
    # Prose is scrubbed; code files carrying an email were refused by validate, and Drake
    # reviews every publish request before anything goes live.
    return build.scrubbed(item) if item["kind"] in ("save", "publish") else item


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
                    {"name": name, "color": "c5def5",
                     "description": ("Concerns aien-dev/" + name[5:]) if name.startswith("area:") else "Allen"})
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


def _ref(gh, branch):
    """The branch head sha, or None when the branch does not exist."""
    try:
        return gh.call("GET", REPO_PREFIX + "git/ref/heads/" + branch)["object"]["sha"]
    except HTTPRefused as e:
        if e.status != 404:
            raise
        return None


def _dir_blobs(gh, tree_sha, prefix):
    """{path: blob sha} of every file under prefix/ in a tree (recursive listing)."""
    t = gh.call("GET", REPO_PREFIX + "git/trees/%s?recursive=1" % tree_sha)
    if t.get("truncated"):
        raise GitHubError("the repository tree is too large to list")
    return {e["path"]: e["sha"] for e in t.get("tree", [])
            if e.get("type") == "blob" and e["path"].startswith(prefix + "/")}


def _project_tree(gh, item, iid, today, base_tree):
    """A new tree: base_tree with projects/<handle>/<project>/ replaced by exactly the item's files
    plus meta.json. Every path is built from the validated handle and project name."""
    d = build.project_dir(item)
    files = [(f["name"], f["content"]) for f in item["files"]]
    files.append(("meta.json", json.dumps(build.meta(item, iid, today), ensure_ascii=False, indent=2,
                                          sort_keys=True) + "\n"))
    entries, keep = [], set()
    for name, content in files:
        blob = gh.call("POST", REPO_PREFIX + "git/blobs",
                       {"content": base64.b64encode(content.encode()).decode(), "encoding": "base64"})
        keep.add(d + "/" + name)
        entries.append({"path": d + "/" + name, "mode": "100644", "type": "blob", "sha": blob["sha"]})
    for path in sorted(set(_dir_blobs(gh, base_tree, d)) - keep):
        entries.append({"path": path, "mode": "100644", "type": "blob", "sha": None})  # removed file
    for e in entries:
        if not e["path"].startswith(d + "/") or "/" in e["path"][len(d) + 1:]:
            raise GuardRefused("path outside the project home: %r" % e["path"])
    return gh.call("POST", REPO_PREFIX + "git/trees", {"base_tree": base_tree, "tree": entries})["sha"]


def _commit_to(gh, branch, item, iid, today, verb, merge_main):
    """One commit on branch with the item's project. Returns the branch head sha afterwards.
    A head whose message names this item means an earlier attempt already committed it."""
    head = _ref(gh, branch)
    main = gh.call("GET", REPO_PREFIX + "git/ref/heads/main")["object"]["sha"]
    head_commit = None
    if head:
        head_commit = gh.call("GET", REPO_PREFIX + "git/commits/" + head)
        if ("Allen item " + iid) in (head_commit.get("message") or ""):
            return head
    base_commit = main if (merge_main or head is None) else head
    base_tree = gh.call("GET", REPO_PREFIX + "git/commits/" + base_commit)["tree"]["sha"]
    tree = _project_tree(gh, item, iid, today, base_tree)
    if head and head_commit["tree"]["sha"] == tree:
        return head  # nothing changed
    parents = [head] if head else [main]
    if merge_main and head:
        # A publish branch merges main in when main moved since its last commit (recorded as
        # "Main <sha>"), so the pull request always shows only the project.
        seen = re.search(r"^Main ([0-9a-f]{40})$", head_commit.get("message") or "", re.M)
        if not seen or seen.group(1) != main:
            parents = [head, main]
    c = gh.call("POST", REPO_PREFIX + "git/commits",
                {"message": "%s %s/%s from Allen\n\nAllen item %s\nMain %s" % (
                    verb, item["handle"], item["project"], iid, main),
                 "tree": tree, "parents": parents})
    if head:
        gh.call("PATCH", REPO_PREFIX + "git/refs/heads/" + branch, {"sha": c["sha"], "force": False})
    else:
        gh.call("POST", REPO_PREFIX + "git/refs", {"ref": "refs/heads/" + branch, "sha": c["sha"]})
    return c["sha"]


def save_project(gh, item, iid, today):
    """Save: one commit on u/<handle>; the URL is that commit's page."""
    sha = _commit_to(gh, "u/" + item["handle"], item, iid, today, "Save", merge_main=False)
    return "https://github.com/%s/commit/%s" % (REPO, sha)


def find_open_pull(gh, branch):
    found = gh.call("GET", REPO_PREFIX + "pulls?state=open&per_page=10&head=" +
                    urllib.parse.quote("aien-dev:" + branch, safe=""))
    return found[0]["html_url"] if found else None


def publish_project(gh, item, iid, today, extra_labels, label_cache):
    """Publish: p/<handle>/<project> carries the project on top of main; one open pull request
    to main per branch, reused on republish. Drake approves and merges; the pusher never does."""
    branch = "p/%s/%s" % (item["handle"], item["project"])
    _commit_to(gh, branch, item, iid, today, "Publish", merge_main=True)
    existing = find_open_pull(gh, branch)
    if existing:
        return existing
    d = build.project_dir(item)
    names = "\n".join("- `%s/%s`" % (d, f["name"]) for f in item["files"])
    body = ("%s\n\n%s\n\nBuilt with Allen by `%s`. Files:\n%s\n- `%s/meta.json`\n\n"
            "Drake reviews this project before it goes live; merging publishes it at %s/%s/\n\n---\n%s\n%s\n") % (
        item["title"], item["description"].rstrip(), item["handle"], names, d, build.PAGES_URL, d, FOOTER,
        marker(iid))
    pr = gh.call("POST", REPO_PREFIX + "pulls",
                 {"title": "Project: " + item["title"], "head": branch, "base": "main", "body": body,
                  "maintainer_can_modify": True})
    labels = ["from-allen", "built-with-allen"] + list(extra_labels)
    for name in labels:
        ensure_label(gh, name, label_cache)
    gh.call("POST", REPO_PREFIX + "issues/%d/labels" % pr["number"], {"labels": labels})
    return pr["html_url"]


def decline_pull(gh, item):
    """Drake declined in his console: comment and close. Only open publish pull requests."""
    pr = gh.call("GET", REPO_PREFIX + "pulls/%d" % item["pr"])
    head = (pr.get("head") or {}).get("ref", "")
    if not re.fullmatch(r"p/[a-z0-9-]+/[a-z0-9-]+", head) or (pr.get("base") or {}).get("ref") != "main":
        raise Refused("pull request %d is not a project publish request" % item["pr"])
    if pr.get("state") == "open":
        reason = spool.scrub(item["reason"]).strip()
        gh.call("POST", REPO_PREFIX + "issues/%d/comments" % item["pr"],
                {"body": "Declined by Drake%s\n\n---\n%s" % ((": " + reason) if reason else ".", FOOTER)})
        gh.call("PATCH", REPO_PREFIX + "pulls/%d" % item["pr"], {"state": "closed"})
    return pr["html_url"]


def write_index(gh, status_dir):
    """spool-status/index.json: open publish requests (Drake's console) and merged projects
    (the Built with Allen gallery), read from main's meta.json files."""
    open_prs = []
    for pr in gh.call("GET", REPO_PREFIX + "pulls?state=open&per_page=100&base=main") or []:
        m = re.fullmatch(r"p/([a-z0-9-]+)/([a-z0-9-]+)", (pr.get("head") or {}).get("ref", ""))
        if m:
            open_prs.append({"number": pr["number"], "url": pr["html_url"], "title": str(pr.get("title", ""))[:120],
                             "handle": m.group(1), "project": m.group(2)})
    main = gh.call("GET", REPO_PREFIX + "git/ref/heads/main")["object"]["sha"]
    tree = gh.call("GET", REPO_PREFIX + "git/commits/" + main)["tree"]["sha"]
    gallery = []
    metas = sorted(p for p in _dir_blobs(gh, tree, "projects").items()
                   if re.fullmatch(r"projects/[a-z0-9-]+/[a-z0-9-]+/meta\.json", p[0]))
    for path, sha in metas[:200]:
        try:
            blob = gh.call("GET", REPO_PREFIX + "git/blobs/" + sha)
            m = json.loads(base64.b64decode(blob["content"]).decode("utf-8"))
            handle, project = path.split("/")[1:3]
            gallery.append({"handle": handle, "project": project, "title": str(m.get("title", ""))[:80],
                            "description": str(m.get("description", ""))[:400],
                            "url": "%s/tree/main/projects/%s/%s" % (build.REPO_URL, handle, project),
                            "page": "%s/projects/%s/%s/" % (build.PAGES_URL, handle, project)})
        except (ValueError, KeyError, TypeError, AttributeError):
            log("index: skipping unreadable %s" % path)
    atomic_write(os.path.join(status_dir, build.INDEX),
                 {"v": 1, "updated": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                  "open": open_prs, "gallery": gallery}, mode=0o640)
    log("index: %d open, %d in the gallery" % (len(open_prs), len(gallery)))


# ---------------------------------------------------------------- driver

class Run:
    """Result of one pass over the spool."""

    def __init__(self):
        self.next_due = None   # epoch seconds of the earliest retry not yet due
        self.waiting = 0       # items kept because the pusher is not activated
        self.more = False      # stopped early for time; the next run continues
        self.index = False     # a build item ran: rewrite spool-status/index.json

    def due(self, when):
        self.next_due = when if self.next_due is None else min(self.next_due, when)


def process(args, gh, deadline=None):
    """One pass over spool-in. `deadline` is a time.monotonic() value; no new item starts
    within 30 s of it, so a run never ends mid-item."""
    spool_in, status_dir = args.spool_in, args.spool_status
    backoff = tuple(getattr(args, "backoff", BACKOFF))
    attempts_max = len(backoff) + 1
    st = State(args.state_dir)
    areas = load_areas(args.areas)
    label_cache = set()
    extra = ["test"] if args.test_label else []
    today = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    run = Run()
    for name in sorted(os.listdir(spool_in)):
        if name.startswith(".tmp-"):
            continue
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
        except Refused as e:
            _finish(st, status_dir, path, iid, "rejected", str(e))
            continue
        kind = item["kind"]
        if gh is None:
            # Not activated: keep the item, tell the visitor it is queued.
            if not (prior and prior.get("state") == "queued" and prior.get("reason") == AWAITING):
                write_status(status_dir, iid, "queued", reason=AWAITING)
                log("%s: queued (%s)" % (iid, AWAITING))
            run.waiting += 1
            continue
        rec = st.d["attempts"].get(iid)
        if rec and rec["next"] > time.time():
            run.due(rec["next"])
            continue
        if deadline is not None and time.monotonic() > deadline - 30:
            run.more = True
            log("run time used; remaining items go to the next run")
            break
        try:
            url = None
            if st.d["pending"].get(iid) == "issue":
                url = find_issue(gh, iid)  # an earlier attempt may have created it
            if url is None:
                if iid not in st.d["pending"]:
                    if not st.take(kind):
                        _finish(st, status_dir, path, iid, "failed",
                                "today's limit for %s on GitHub is reached"
                                % {"issue": "issues", "save": "saves"}.get(kind, "publish requests"))
                        continue
                    st.d["pending"][iid] = kind
                    st.save()
                if kind == "issue":
                    url = file_issue(gh, item, iid, extra, label_cache)
                elif kind == "save":
                    url = save_project(gh, item, iid, today)
                elif kind == "publish":
                    url = publish_project(gh, item, iid, today, extra, label_cache)
                elif kind == "decline":
                    url = decline_pull(gh, item)
                else:  # refresh: the index below is the whole action
                    url = None
            run.index = run.index or kind != "issue"
            st.d["filed"][iid] = url
            st.d["pending"].pop(iid, None)
            st.d["attempts"].pop(iid, None)
            st.save()
            write_status(status_dir, iid, "filed", url=url)
            _remove(path)
            log("%s: filed %s" % (iid, url))
            continue
        except Refused as e:
            _finish(st, status_dir, path, iid, "rejected", str(e))
            continue
        except GitHubError as e:
            cause = e.cause
        except GuardRefused as e:
            # The request guard stopped a call the code should never make: not retried.
            log("%s: guard: %s" % (iid, e))
            _finish(st, status_dir, path, iid, "failed", "an internal safety check stopped the request")
            continue
        except Exception as e:  # noqa: BLE001 - an unknown error must not lose the item
            cause = "an internal error (%s)" % type(e).__name__
            log("%s: %s: %s" % (iid, type(e).__name__, str(e)[:200]))
        n = (rec["n"] if rec else 0) + 1
        if n >= attempts_max:
            _finish(st, status_dir, path, iid, "failed", "%s after %d attempts" % (cause, n))
            continue
        when = time.time() + backoff[n - 1]
        st.d["attempts"][iid] = {"n": n, "next": when, "cause": cause}
        st.save()
        write_status(status_dir, iid, "queued",
                     reason="retrying: %s (attempt %d of %d)" % (cause, n, attempts_max))
        log("%s: attempt %d of %d failed: %s; next in %.0f s" % (iid, n, attempts_max, cause, backoff[n - 1]))
        run.due(when)
    if gh is not None and run.index:
        try:
            write_index(gh, status_dir)
        except (GitHubError, GuardRefused, OSError, KeyError, TypeError) as e:
            log("index: not rewritten: %s" % e)
    return run


def _finish(st, status_dir, path, iid, state, reason):
    """rejected: the content broke a rule. failed: GitHub did not take it."""
    kind = st.d["pending"].pop(iid, None)
    if state == "failed" and kind:
        st.give(kind)
    st.d["attempts"].pop(iid, None)
    st.save()
    write_status(status_dir, iid, state, reason=reason)
    _remove(path)
    log("%s: %s: %s" % (iid, state, reason))


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


def _seconds_list(text):
    vals = tuple(float(x) for x in text.split(","))
    if not vals or any(v < 0 for v in vals):
        raise argparse.ArgumentTypeError("comma-separated non-negative seconds")
    return vals


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
    p.add_argument("--backoff", type=_seconds_list, default=BACKOFF,
                   help="seconds before each retry; attempts = entries + 1 (default 10,20,40,80)")
    p.add_argument("--run-budget", type=float, default=RUN_BUDGET,
                   help="seconds one run may take, retries included (unit TimeoutStartSec is 120)")
    p.add_argument("--idle-delay", type=float, default=90.0,
                   help="seconds to wait before exiting when items wait for activation")
    args = p.parse_args(argv)
    global spool, build
    sys.path.insert(0, args.spool_module_dir)
    import allen_spool
    import allen_build
    spool, build = allen_spool, allen_build
    os.makedirs(args.state_dir, mode=0o700, exist_ok=True)
    lock = open(os.path.join(args.state_dir, "lock"), "w")
    fcntl.flock(lock, fcntl.LOCK_EX)
    token = load_token(args.token_file)
    gh = GitHub(token, api=args.api_base) if token else None
    if gh is None:
        log("not activated: no omega-projects-token credential; items stay queued")
    deadline = time.monotonic() + args.run_budget
    # The path unit starts a new run as soon as this one exits while spool-in holds
    # items, so every exit with items left waits first: until a retry is due (looping
    # inside this run while time is left) or for idle-delay when not activated.
    while True:
        run = process(args, gh, deadline)
        left = deadline - time.monotonic()
        if run.more:
            break
        if run.waiting:
            time.sleep(max(0.0, min(args.idle_delay, left)))
            break
        if run.next_due is None:
            break
        delay = max(0.0, run.next_due - time.time())
        if delay > left:
            time.sleep(max(0.0, left))
            break
        time.sleep(delay)
    return 0


if __name__ == "__main__":
    sys.exit(main())
