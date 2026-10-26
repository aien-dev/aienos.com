"""The build loop's shared rules: handles, project items and the published index.

Used by allen-chat (writer, validates and checks ownership) and omega-projects-pusher
(reader, validates again before anything reaches GitHub). Stdlib only.

Homes. Every builder has a home projects/<handle>/<project>/ in aien-dev/omega-projects.
  remembered visitor: handle = <name>-<8 hex>, the hex being HMAC(server secret, "handle", vid)
                      where vid is the gateway's HMAC of the visitor key; the name is chosen
                      by the visitor (never an email: lowercase letters, digits, hyphens).
  anonymous visitor:  handle = anon-<10 hex>, the hex being HMAC(server secret, "claim", code)
                      for a one-time claim code shown to the visitor once.
Only the gateway holds the secret, so only the holder of the visitor key or the claim code
can name a handle in a save or publish item; the pusher builds every path from the
validated handle and project name, so nothing outside projects/<handle>/<project>/ is written.

Items, version 1 (spool files, content-addressed exactly like allen_spool items):
  {"v":1, "kind":"save"|"publish", "handle":h, "project":p, "title":str,
   "description":str, "files":[{"name":str, "content":str}, ...]}
     save:    one commit on branch u/<handle> (created from main when missing).
     publish: one commit on branch p/<handle>/<project> from main and one pull request to
              main, reused while it is open. Drake approves and merges on GitHub.
  {"v":1, "kind":"decline", "pr":int, "reason":str}   console only: close a publish PR.
  {"v":1, "kind":"refresh", "nonce":hex}               console only: rewrite the index.
Index (spool-status/index.json, written by the pusher):
  {"v":1, "updated":iso, "open":[{number,url,title,handle,project}],
   "gallery":[{handle,project,title,description,url,page}]}
"""

import hashlib
import hmac
import json
import os
import re
import secrets

import allen_spool as spool

NAME = re.compile(r"^[a-z](?:[a-z0-9]|-(?=[a-z0-9])){1,23}$")
HANDLE = re.compile(r"^(?:[a-z](?:[a-z0-9]|-(?=[a-z0-9])){1,23}-[0-9a-f]{8}|anon-[0-9a-f]{10})$")
PROJECT = spool.SLUG
CLAIM = re.compile(r"^[0-9a-f]{24}$")
# Allowlisted text files. No paths, no dotfiles, no binaries; meta.json is written by the pusher.
FILENAME = re.compile(r"^(?:index\.html|README\.md|[a-z0-9][a-z0-9_-]{0,39}\.(?:om|js|css|md|txt|patch))$")
WEB = (".html", ".js", ".css")
PROSE = (".md", ".txt")
MAX_FILE = 64 * 1024
MAX_TOTAL = 256 * 1024
MAX_FILES = 12
REPO_URL = "https://github.com/aien-dev/omega-projects"
PAGES_URL = "https://aien-dev.github.io/omega-projects"
INDEX = "index.json"


Invalid = spool.Invalid  # one exception for both modules: _text raises it too


def handle_for_vid(secret, vid, name):
    if not NAME.match(name or ""):
        raise Invalid("a name is 2 to 24 lowercase letters, digits or inner hyphens, starting with a letter")
    tag = hmac.new(secret, b"allen-handle\0" + vid.encode(), hashlib.sha256).hexdigest()[:8]
    return "%s-%s" % (name, tag)


def handle_for_claim(secret, claim):
    if not CLAIM.match(claim or ""):
        raise Invalid("a claim code is 24 hex characters")
    return "anon-" + hmac.new(secret, b"allen-claim\0" + claim.encode(), hashlib.sha256).hexdigest()[:10]


def new_claim():
    return secrets.token_hex(12)


def owns(secret, handle, vid=None, claim=None):
    """True when the caller (vid from the visitor key, or a claim code) owns the handle."""
    if not secret or not isinstance(handle, str) or not HANDLE.match(handle):
        return False
    try:
        if handle.startswith("anon-"):
            return claim is not None and hmac.compare_digest(handle, handle_for_claim(secret, claim))
        if vid is None:
            return False
        name = handle.rsplit("-", 1)[0]
        return hmac.compare_digest(handle, handle_for_vid(secret, vid, name))
    except Invalid:
        return False


def _files(files):
    if not isinstance(files, list) or not 1 <= len(files) <= MAX_FILES:
        raise Invalid("a project has 1 to %d files" % MAX_FILES)
    out, names, total = [], set(), 0
    for f in files:
        if not isinstance(f, dict) or set(f) != {"name", "content"}:
            raise Invalid("each file has a name and a content")
        name = f["name"]
        if not isinstance(name, str) or not FILENAME.match(name) or name in names:
            raise Invalid("file names allowed: index.html, README.md, name.om, name.js, name.css, "
                          "name.md, name.txt, name.patch (lowercase, no folders)")
        names.add(name)
        content = f["content"]
        if not isinstance(content, str):
            raise Invalid("file content must be text")
        size = len(content.encode("utf-8"))
        if size > MAX_FILE:
            raise Invalid("%s is larger than %d KiB" % (name, MAX_FILE // 1024))
        total += size
        spool._text(content, name, 1, MAX_FILE)  # control characters and secret shapes
        if name.endswith(WEB):
            for pat in spool.EXTERNAL:
                if pat.search(content):
                    raise Invalid("%s may not load or send anything from elsewhere (%s)" % (name, pat.pattern[:40]))
        if not name.endswith(PROSE) and spool.EMAIL.search(content):
            raise Invalid("%s contains an email address; remove it" % name)
        out.append({"name": name, "content": content})
    if total > MAX_TOTAL:
        raise Invalid("the project is larger than %d KiB" % (MAX_TOTAL // 1024))
    if "README.md" not in names:
        raise Invalid("a project needs README.md: what it is and who it helps")
    if not any(n == "index.html" or n.endswith(".om") for n in names):
        raise Invalid("a project needs index.html or an .om file")
    out.sort(key=lambda f: f["name"])
    return out


def validate(item):
    """Return the canonical build item or raise Invalid."""
    if not isinstance(item, dict) or item.get("v") != 1:
        raise Invalid("unknown item version")
    kind = item.get("kind")
    if kind in ("save", "publish"):
        if set(item) != {"v", "kind", "handle", "project", "title", "description", "files"}:
            raise Invalid("%s fields are handle, project, title, description and files" % kind)
        if not isinstance(item["handle"], str) or not HANDLE.match(item["handle"]):
            raise Invalid("unknown handle")
        if not isinstance(item["project"], str) or not PROJECT.match(item["project"]):
            raise Invalid("a project name is 1 to 40 lowercase letters, digits or inner hyphens")
        return {"v": 1, "kind": kind, "handle": item["handle"], "project": item["project"],
                "title": spool._text(item["title"], "title", 3, 80, line=True).strip(),
                "description": spool._text(item["description"], "description", 10, 2000),
                "files": _files(item["files"])}
    if kind == "decline":
        if set(item) != {"v", "kind", "pr", "reason"} or not isinstance(item["pr"], int) \
                or isinstance(item["pr"], bool) or not 1 <= item["pr"] < 10 ** 9:
            raise Invalid("decline fields are pr and reason")
        return {"v": 1, "kind": "decline", "pr": item["pr"],
                "reason": spool._text(item["reason"], "reason", 0, 1000)}
    if kind == "refresh":
        if set(item) != {"v", "kind", "nonce"} or not isinstance(item["nonce"], str) \
                or not re.fullmatch(r"[0-9a-f]{16}", item["nonce"]):
            raise Invalid("refresh fields are nonce")
        return dict(item)
    raise Invalid("kind must be save, publish, decline or refresh")


def scrubbed(item):
    """Emails and phone numbers out of prose; code files are refused instead (see _files)."""
    item = dict(item, title=spool.scrub(item["title"]), description=spool.scrub(item["description"]))
    item["files"] = [dict(f, content=spool.scrub(f["content"])) if f["name"].endswith(PROSE) else dict(f)
                     for f in item["files"]]
    return item


def meta(item, iid, day):
    """meta.json: what the project is, never who the person is."""
    return {"v": 1, "handle": item["handle"], "project": item["project"], "title": item["title"],
            "description": item["description"], "saved": day, "item": iid, "built_with": "Allen",
            "files": [f["name"] for f in item["files"]]}


def project_dir(item):
    return "projects/%s/%s" % (item["handle"], item["project"])


def read_index(status_dir):
    """The pusher's index, or an empty one."""
    try:
        with open(os.path.join(status_dir, INDEX), encoding="utf-8") as f:
            d = json.load(f)
        if isinstance(d, dict) and isinstance(d.get("open"), list) and isinstance(d.get("gallery"), list):
            return d
    except (OSError, ValueError):
        pass
    return {"v": 1, "updated": None, "open": [], "gallery": []}
