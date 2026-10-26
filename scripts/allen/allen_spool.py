"""Validation and spool format shared by allen-chat (writer) and omega-projects-pusher (reader).

The gateway holds no GitHub credential. It validates a visitor-confirmed item
and drops it into a spool directory it can only write into; the pusher, a
separate unit and the only holder of GitHub write ability, validates the same
item again before filing anything. An item is one JSON file named <id>.json,
where id is derived from the content, so identical submissions collapse into
one item (dedupe). Status comes back as status/<id>.json written by the pusher.

Item, version 1:
  {"v":1, "kind":"issue", "template":"feature"|"question"|"bug",
   "title":str, "body":str, "area":str}
  {"v":1, "kind":"project", "slug":str, "title":str, "description":str,
   "files":[{"name":str, "content":str}, ...]}
Status: {"id":..., "state":"queued"|"filed"|"rejected"|"failed", "url":str|null, "reason":str|null}
  queued = not filed yet; filed = url set; rejected = content broke a rule (fixable);
  failed = GitHub did not take it after N attempts or the daily cap was hit (offer the relay).
"""

import hashlib
import json
import os
import re
import secrets

TEMPLATES = ("feature", "question", "bug")
SLUG = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])?$")
FILENAME = re.compile(r"^(?:index\.html|README\.md|[a-z0-9][a-z0-9_-]{0,39}\.om)$")
AREA = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$")
ID = re.compile(r"^[0-9a-f]{20}$")
MAX_FILE = 64 * 1024
MAX_TOTAL = 256 * 1024
MAX_FILES = 8

SECRET_SHAPES = [
    re.compile(p) for p in (
        r"gh[pousr]_[A-Za-z0-9]{20,}", r"github_pat_[A-Za-z0-9_]{20,}",
        r"\bsk-[A-Za-z0-9_-]{20,}", r"\bsk-ant-[A-Za-z0-9_-]{10,}",
        r"\bAKIA[0-9A-Z]{16}\b", r"\bAIza[0-9A-Za-z_-]{30,}",
        r"xox[abprs]-[A-Za-z0-9-]{10,}", r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
        r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.",
        r"(?i)\b(api[_-]?key|secret|passw(or)?d|access[_-]?token|auth[_-]?token)\b\s*[:=]\s*['\"]?[^\s'\"]{12,}",
    )
]
# Anything that would make a published page load or send something elsewhere.
EXTERNAL = [
    re.compile(p, re.I) for p in (
        r"<script\b[^>]*\bsrc\s*=", r"<link\b", r"<iframe\b", r"<object\b", r"<embed\b", r"<base\b",
        r"<meta\b[^>]*http-equiv", r"\b(src|href|action|formaction|poster|data)\s*=\s*['\"]?\s*(https?:)?//",
        r"url\(\s*['\"]?\s*(https?:)?//", r"@import\b", r"\bfetch\s*\(", r"XMLHttpRequest", r"WebSocket",
        r"EventSource", r"sendBeacon", r"\bimport\s*\(", r"\bimportScripts\b", r"\bwindow\.open\b",
    )
]
EMAIL = re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")
PHONE = re.compile(r"(?<![\w.])\+?\d[\d ().-]{7,}\d(?!\w|\.\d)")


class Invalid(ValueError):
    pass


def _text(value, name, lo, hi, line=False):
    if not isinstance(value, str):
        raise Invalid("%s must be text" % name)
    if not lo <= len(value) <= hi:
        raise Invalid("%s must be %d to %d characters" % (name, lo, hi))
    bad = "\n\r" if line else ""
    for ch in value:
        o = ord(ch)
        if ((o < 32 and ch not in "\n\t\r") or o == 127 or ch in bad or 0xD800 <= o <= 0xDFFF
                or 0xFDD0 <= o <= 0xFDEF or (o & 0xFFFE) == 0xFFFE or o == 0xFFFD):
            raise Invalid("%s contains a control or binary character" % name)
    for pat in SECRET_SHAPES:
        if pat.search(value):
            raise Invalid("%s looks like it contains a secret (key, token or password); remove it" % name)
    return value


def scrub(text):
    """Remove email addresses and phone numbers before anything is published."""
    return PHONE.sub("[phone removed]", EMAIL.sub("[email removed]", text))


def validate(item, areas=None):
    """Return the canonical item or raise Invalid. `areas`: known public repo names."""
    if not isinstance(item, dict) or item.get("v") != 1:
        raise Invalid("unknown item version")
    kind = item.get("kind")
    if kind == "issue":
        if set(item) != {"v", "kind", "template", "title", "body", "area"}:
            raise Invalid("issue fields are template, title, body and area")
        if item["template"] not in TEMPLATES:
            raise Invalid("template must be feature, question or bug")
        area = item["area"]
        if not isinstance(area, str) or not AREA.match(area):
            raise Invalid("area must be a repository name")
        if areas is not None and area not in areas and area != "unknown":
            area = "unknown"
        return {"v": 1, "kind": "issue", "template": item["template"],
                "title": _text(item["title"], "title", 8, 120, line=True).strip(),
                "body": _text(item["body"], "body", 20, 6000), "area": area}
    if kind == "project":
        if set(item) != {"v", "kind", "slug", "title", "description", "files"}:
            raise Invalid("project fields are slug, title, description and files")
        slug = item["slug"]
        if not isinstance(slug, str) or not SLUG.match(slug):
            raise Invalid("slug must be 1 to 40 lowercase letters, digits or inner hyphens")
        files = item["files"]
        if not isinstance(files, list) or not 1 <= len(files) <= MAX_FILES:
            raise Invalid("a project has 1 to %d files" % MAX_FILES)
        out, names, total = [], set(), 0
        for f in files:
            if not isinstance(f, dict) or set(f) != {"name", "content"}:
                raise Invalid("each file has a name and a content")
            name = f["name"]
            if not isinstance(name, str) or not FILENAME.match(name) or name in names:
                raise Invalid("file names allowed: index.html, README.md, name.om (lowercase, no paths)")
            names.add(name)
            content = f["content"]
            if not isinstance(content, str):
                raise Invalid("file content must be text")
            size = len(content.encode("utf-8"))
            if size > MAX_FILE:
                raise Invalid("%s is larger than %d KiB" % (name, MAX_FILE // 1024))
            total += size
            _text(content, name, 1, MAX_FILE)
            if name.endswith(".html"):
                for pat in EXTERNAL:
                    if pat.search(content):
                        raise Invalid("index.html may not load or send anything from elsewhere (%s)"
                                      % pat.pattern[:40])
            out.append({"name": name, "content": content})
        if total > MAX_TOTAL:
            raise Invalid("the project is larger than %d KiB" % (MAX_TOTAL // 1024))
        if not any(f["name"] in ("index.html",) or f["name"].endswith(".om") for f in out):
            raise Invalid("a project needs index.html or an .om file")
        out.sort(key=lambda f: f["name"])
        return {"v": 1, "kind": "project", "slug": slug,
                "title": _text(item["title"], "title", 3, 80, line=True).strip(),
                "description": _text(item["description"], "description", 10, 2000), "files": out}
    raise Invalid("kind must be issue or project")


def item_id(item):
    canon = json.dumps(item, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return hashlib.sha256(canon).hexdigest()[:20]


def write_item(spool_in, item):
    """Atomically place a validated item; returns (id, created)."""
    iid = item_id(item)
    final = os.path.join(spool_in, iid + ".json")
    if os.path.exists(final):
        return iid, False
    tmp = os.path.join(spool_in, ".tmp-%s-%s" % (iid, secrets.token_hex(4)))
    # 0640: the spool directory's setgid group lets the pusher read it; the writer cannot list the dir.
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o640)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(item, f, ensure_ascii=False, sort_keys=True)
        f.flush()
        os.fsync(f.fileno())
    os.rename(tmp, final)
    return iid, True


def spool_file(directory, iid):
    """directory/iid.json for a well-formed id, or None; the real path must stay in directory."""
    if not ID.match(iid or ""):
        return None
    root = os.path.realpath(directory)
    path = os.path.realpath(os.path.join(root, iid + ".json"))
    return path if path.startswith(root + os.sep) else None


def read_status(spool_status, iid):
    path = spool_file(spool_status, iid)
    if path is None:
        return None
    try:
        with open(path, encoding="utf-8") as f:
            st = json.load(f)
    except (OSError, ValueError):
        return None
    url = st.get("url")
    if url is not None and not (isinstance(url, str) and re.match(
            r"^https://github\.com/aien-dev/omega-projects/(issues|pull)/\d+$", url)):
        url = None
    return {"id": iid, "state": st.get("state") if st.get("state") in ("queued", "filed", "rejected", "failed")
            else "queued", "url": url, "reason": st.get("reason") if isinstance(st.get("reason"), str) else None}


# Reaching Drake: relay messages (gateway writes, allen-relay mailer reads) and live text.
RELAY_MESSAGE = 4000
RELAY_FIELD = 200
TRANSCRIPT_MSGS = 24
TRANSCRIPT_CHARS = 16000
RELAY_ID = re.compile(r"^r_[0-9a-f]{16}$")
CONV = re.compile(r"^[0-9a-f]{16}$")
RECEIVED = re.compile(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")


def validate_transcript(t):
    """A conversation the visitor chose to include: [] or up to 24 user/assistant messages."""
    if t is None:
        return []
    if not isinstance(t, list):
        raise Invalid("transcript must be a list of messages")
    if len(t) > TRANSCRIPT_MSGS:
        raise Invalid("transcript is limited to %d messages" % TRANSCRIPT_MSGS)
    out, total = [], 0
    for m in t:
        if not isinstance(m, dict) or set(m) != {"role", "content"} or m.get("role") not in ("user", "assistant"):
            raise Invalid("each transcript message has a role (user or assistant) and a content")
        c = _text(m["content"], "transcript message", 0, RELAY_MESSAGE)
        total += len(c)
        out.append({"role": m["role"], "content": c})
    if total > TRANSCRIPT_CHARS:
        raise Invalid("transcript is limited to %d characters" % TRANSCRIPT_CHARS)
    return out


def validate_relay(req):
    """{"message", "name", "contact", "transcript"} -> canonical relay fields or Invalid."""
    if not isinstance(req, dict):
        raise Invalid("send a JSON object")
    extra = set(req) - {"message", "name", "contact", "transcript"}
    if extra:
        raise Invalid("unknown fields: " + ", ".join(sorted(extra))[:200])
    message = _text(req.get("message"), "message", 1, RELAY_MESSAGE).strip()
    if not message:
        raise Invalid("message is empty")
    name = req.get("name") or ""
    contact = req.get("contact") or ""
    return {"message": message,
            "name": _text(name, "name", 0, RELAY_FIELD, line=True).strip(),
            "contact": _text(contact, "contact", 0, RELAY_FIELD, line=True).strip(),
            "transcript": validate_transcript(req.get("transcript"))}


def validate_relay_record(rec):
    """A relay drop file as the gateway writes it; the mailer re-checks every field."""
    if not isinstance(rec, dict) or rec.get("v") != 1 or rec.get("type") != "relay":
        raise Invalid("not a version 1 relay record")
    if set(rec) != {"v", "type", "id", "received", "conv", "message", "name", "contact", "transcript"}:
        raise Invalid("relay record fields do not match")
    if not isinstance(rec["id"], str) or not RELAY_ID.match(rec["id"]):
        raise Invalid("bad relay id")
    if not isinstance(rec["received"], str) or not RECEIVED.match(rec["received"]):
        raise Invalid("bad received time")
    if not isinstance(rec["conv"], str) or not (rec["conv"] == "" or CONV.match(rec["conv"])):
        raise Invalid("bad conversation id")
    item = validate_relay({k: rec[k] for k in ("message", "name", "contact", "transcript")})
    return dict(item, v=1, type="relay", id=rec["id"], received=rec["received"], conv=rec["conv"])


def validate_say(req):
    """{"text"} the visitor types while Drake is there."""
    if not isinstance(req, dict) or set(req) != {"text"}:
        raise Invalid("say takes only text")
    text = _text(req["text"], "text", 1, RELAY_MESSAGE).strip()
    if not text:
        raise Invalid("text is empty")
    return text


def validate_summary(req):
    """{"summary", "consent"} for POST project-summary: the visitor's own short project summary,
    text only (no code or file fields), no secrets, emails or phone numbers."""
    if not isinstance(req, dict) or set(req) - {"summary", "consent"}:
        raise Invalid("project-summary takes only summary and consent")
    s = req.get("summary")
    if not isinstance(s, str):
        raise Invalid("summary must be text")
    s = s.strip()
    if not s or len(s.encode("utf-8")) > 1000:
        raise Invalid("summary must be 1 to 1000 bytes")
    _text(s, "summary", 1, 1000)
    if EMAIL.search(s) or PHONE.search(s):
        raise Invalid("summary must not contain an email address or a phone number")
    return s
